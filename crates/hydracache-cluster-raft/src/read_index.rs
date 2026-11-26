//! Request-scoped safe ReadIndex for the applied metadata state machine.

use std::collections::BTreeMap;
use std::num::NonZeroU64;

use hydracache::{CacheError, ClusterEpoch};
use raft::{ReadOnlyOption, ReadState};
use tokio::sync::oneshot;
use tokio::time::{Duration, Instant};

use super::{
    to_cache_error, MessageType, RaftLogStore, RaftMessage, RaftMetadataRuntime,
    RaftPartitionSnapshot, RaftRuntimeState, RaftWireMessage, StateRole,
};

const MAX_PENDING_READS: usize = 256;
const FORWARDED_READ_RETENTION: Duration = Duration::from_secs(1);
type ReadContext = [u8; 16];

#[derive(Clone, Copy, PartialEq, Eq)]
struct Authority {
    term: u64,
    leader: u64,
    epoch: ClusterEpoch,
    configuration_index: u64,
}

struct PendingRead {
    authority: Authority,
    index: Option<NonZeroU64>,
    completed: oneshot::Sender<NonZeroU64>,
}

#[derive(Default)]
pub(super) struct ReadIndexes {
    pending: BTreeMap<ReadContext, PendingRead>,
    forwarded: BTreeMap<ReadContext, Instant>,
    pub(super) configuration_index: u64,
}

/// Why a safe Raft read could not be issued or used.
#[derive(Debug)]
pub enum RaftReadIndexError {
    /// Leadership, membership, or the applied view cannot authorize the read.
    Unavailable,
    /// The local runtime or leader has reached the pending-read bound.
    Busy,
    /// Failure to initialize or drive Raft.
    Runtime(CacheError),
}

/// One pending invocation. Dropping it releases the local Raft reservation.
pub struct RaftReadIndexRequest<'a, S: RaftLogStore> {
    runtime: &'a RaftMetadataRuntime<S>,
    context: ReadContext,
    authority: Authority,
    completed: oneshot::Receiver<NonZeroU64>,
}

/// A one-use ReadIndex result from this runtime, after local metadata application.
pub struct RaftReadIndexProof<'a, S: RaftLogStore> {
    runtime: &'a RaftMetadataRuntime<S>,
    authority: Authority,
    index: NonZeroU64,
}

impl<S: RaftLogStore> RaftMetadataRuntime<S> {
    /// Issue a fresh safe ReadIndex and return messages to deliver through the
    /// normal Raft transport. The caller must bound the lifetime of the request.
    pub fn begin_read_index(
        &self,
    ) -> Result<(RaftReadIndexRequest<'_, S>, Vec<RaftWireMessage>), RaftReadIndexError> {
        let mut context = ReadContext::default();
        getrandom::fill(&mut context)
            .map_err(|error| RaftReadIndexError::Runtime(CacheError::Backend(error.to_string())))?;
        let mut state = self.raft.lock().expect("raft metadata state poisoned");
        let authority = state
            .read_authority()
            .ok_or(RaftReadIndexError::Unavailable)?;
        if state.reads.pending.len() >= MAX_PENDING_READS
            || state.raw_node.raft.read_only.pending_read_count() >= MAX_PENDING_READS
            || state.reads.pending.contains_key(&context)
            || state
                .raw_node
                .raft
                .read_only
                .pending_read_index
                .contains_key(context.as_slice())
        {
            return Err(RaftReadIndexError::Busy);
        }

        let (sender, completed) = oneshot::channel();
        state.reads.pending.insert(
            context,
            PendingRead {
                authority,
                index: None,
                completed: sender,
            },
        );
        state.raw_node.read_index(context.to_vec());
        let outbound = state.drain_ready().map_err(RaftReadIndexError::Runtime)?;
        Ok((
            RaftReadIndexRequest {
                runtime: self,
                context,
                authority,
                completed,
            },
            outbound,
        ))
    }
}

impl<'a, S: RaftLogStore> RaftReadIndexRequest<'a, S> {
    /// Wait for the safe quorum result and its local applied metadata prefix.
    pub async fn complete(mut self) -> Result<RaftReadIndexProof<'a, S>, RaftReadIndexError> {
        let index = (&mut self.completed)
            .await
            .map_err(|_| RaftReadIndexError::Unavailable)?;
        Ok(RaftReadIndexProof {
            runtime: self.runtime,
            authority: self.authority,
            index,
        })
    }
}

impl<S: RaftLogStore> Drop for RaftReadIndexRequest<'_, S> {
    fn drop(&mut self) {
        let mut state = self
            .runtime
            .raft
            .lock()
            .expect("raft metadata state poisoned");
        state.cancel_read_index(&self.context);
    }
}

impl<S: RaftLogStore> RaftReadIndexProof<'_, S> {
    /// Check the authority again and consume the proof under the Raft apply lock.
    /// The caller holds the value lane and supplies a short synchronous callback
    /// that does not reenter this runtime.
    pub fn with_snapshot<T>(
        self,
        action: impl FnOnce(RaftPartitionSnapshot, NonZeroU64) -> T,
    ) -> Result<T, RaftReadIndexError> {
        let state = self
            .runtime
            .raft
            .lock()
            .expect("raft metadata state poisoned");
        if state.read_authority() != Some(self.authority) || state.applied_index < self.index.get()
        {
            return Err(RaftReadIndexError::Unavailable);
        }
        let snapshot = state
            .partition_snapshot()
            .ok_or(RaftReadIndexError::Unavailable)?;
        Ok(action(snapshot, self.index))
    }
}

impl<S: RaftLogStore> RaftRuntimeState<S> {
    pub(super) fn partition_snapshot(&self) -> Option<RaftPartitionSnapshot> {
        (!self.partition_view_fenced).then(|| RaftPartitionSnapshot {
            term: self.raw_node.raft.term,
            epoch: self.cluster.epoch(),
            commit_index: self.raw_node.raft.hard_state().commit,
            applied_index: self.applied_index,
            members: self.cluster.members(),
            value_prefixes: self.value_prefixes.clone(),
            value_promotions: self.value_promotions.clone(),
            value_repairs: self.value_repairs.clone(),
        })
    }

    fn read_authority(&self) -> Option<Authority> {
        let raft = &self.raw_node.raft;
        if self.partition_view_fenced
            || raft.leader_id == 0
            || raft.read_only.option != ReadOnlyOption::Safe
            || (raft.state == StateRole::Leader && !raft.promotable())
            || !matches!(raft.state, StateRole::Leader | StateRole::Follower)
        {
            return None;
        }
        Some(Authority {
            term: raft.term,
            leader: raft.leader_id,
            epoch: self.cluster.epoch(),
            configuration_index: self.reads.configuration_index,
        })
    }

    pub(super) fn step_with_read_index_validation(
        &mut self,
        message: RaftMessage,
    ) -> hydracache::CacheResult<bool> {
        let kind = message.get_msg_type();
        if !matches!(
            kind,
            MessageType::MsgReadIndex | MessageType::MsgReadIndexResp
        ) {
            self.raw_node.step(message).map_err(to_cache_error)?;
            return Ok(true);
        }
        let [entry] = message.entries.as_slice() else {
            return Ok(false);
        };
        let Ok(context) = <ReadContext>::try_from(&entry.data[..]) else {
            return Ok(false);
        };
        let Some(authority) = self.read_authority() else {
            return Ok(false);
        };
        self.expire_forwarded_read_indexes();
        let raft = &self.raw_node.raft;
        let accepted = match kind {
            MessageType::MsgReadIndex => {
                raft.state == StateRole::Leader
                    && message.from != raft.id
                    && raft.prs().get(message.from).is_some()
                    && message.term == 0
                    && raft.read_only.pending_read_count() < MAX_PENDING_READS
                    && !raft
                        .read_only
                        .pending_read_index
                        .contains_key(context.as_slice())
            }
            MessageType::MsgReadIndexResp => {
                raft.state == StateRole::Follower
                    && message.from == authority.leader
                    && message.term == authority.term
                    && message.index > 0
                    && self.reads.pending.get(&context).is_some_and(|pending| {
                        pending.authority == authority && pending.index.is_none()
                    })
            }
            _ => unreachable!("non-read message handled above"),
        };
        if accepted {
            self.raw_node.step(message).map_err(to_cache_error)?;
            if kind == MessageType::MsgReadIndex
                && self
                    .raw_node
                    .raft
                    .read_only
                    .pending_read_index
                    .contains_key(context.as_slice())
            {
                self.reads
                    .forwarded
                    .insert(context, Instant::now() + FORWARDED_READ_RETENTION);
            }
        }
        Ok(accepted)
    }

    pub(super) fn expire_forwarded_read_indexes(&mut self) {
        if self.reads.forwarded.is_empty() {
            return;
        }
        let now = Instant::now();
        let reads = &mut self.raw_node.raft.read_only;
        self.reads.forwarded.retain(|context, deadline| {
            if *deadline <= now
                && reads
                    .pending_read_index
                    .remove(context.as_slice())
                    .is_some()
            {
                reads.read_index_queue.retain(|queued| queued != context);
                return false;
            }
            reads.pending_read_index.contains_key(context.as_slice())
        });
    }

    pub(super) fn record_read_states(&mut self, states: Vec<ReadState>) {
        for state in states {
            if let (Ok(context), Some(index)) = (
                <&ReadContext>::try_from(state.request_ctx.as_slice()),
                NonZeroU64::new(state.index),
            ) {
                if let Some(pending) = self.reads.pending.get_mut(context) {
                    pending.index.get_or_insert(index);
                }
            }
        }
    }

    pub(super) fn finish_read_indexes(&mut self) {
        if self.reads.pending.is_empty() {
            return;
        }
        let authority = self.read_authority();
        let applied = self.applied_index;
        let finished: Vec<_> = self
            .reads
            .pending
            .iter()
            .filter_map(|(context, pending)| {
                if authority != Some(pending.authority) {
                    return Some((*context, None));
                }
                pending
                    .index
                    .filter(|index| index.get() <= applied)
                    .map(|index| (*context, Some(index)))
            })
            .collect();
        for (context, index) in finished {
            if let Some(index) = index {
                if let Some(pending) = self.reads.pending.remove(&context) {
                    let _ = pending.completed.send(index);
                }
            } else {
                self.cancel_read_index(&context);
            }
        }
    }

    fn cancel_read_index(&mut self, context: &ReadContext) {
        self.reads.pending.remove(context);
        let reads = &mut self.raw_node.raft.read_only;
        if reads
            .pending_read_index
            .remove(context.as_slice())
            .is_some()
        {
            reads.read_index_queue.retain(|queued| queued != context);
        }
    }
}
