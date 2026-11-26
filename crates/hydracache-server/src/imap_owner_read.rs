//! Request-scoped Raft authority for the staged IMap GET backend.

use std::fmt;
use std::num::{NonZeroU32, NonZeroU64};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use hydracache::{ClusterEpoch, PartitionId, ReplicationConfig};
use hydracache_client_protocol::{
    ClientErrorCode, ClientErrorEnvelope, ClientRequest, Namespace, StructuredKey,
};
use hydracache_client_transport_axum::{
    ClientBackendFuture, ClientBackendOutcome, ClientDataBackend, VerifiedClientRequest,
};
use hydracache_cluster_raft::{RaftLogStore, RaftMessageSink, RaftMetadataRuntime};
use tokio::time::{timeout_at, Instant};

use crate::grid_host::send_raft_messages_with_diagnostics;
use crate::imap_read_barrier::{OwnerReadBarrier, ReadInvocation};
use crate::imap_topology::{PartitionMember, PartitionTopology};

const MAX_READ_WAIT: Duration = Duration::from_millis(500);

/// Admitted scope from the value executor's namespace catalog and partitioner.
#[derive(Debug, Clone, Copy)]
pub struct OwnerReadRoute {
    pub namespace_generation: NonZeroU64,
    pub partition: PartitionId,
}

/// Value progress for a particular namespace and ownership epoch.
#[derive(Debug, Clone, Copy)]
pub struct OwnerReadProgress {
    pub namespace_generation: NonZeroU64,
    pub epoch: ClusterEpoch,
    pub applied_prefix: u64,
    pub required_prefix: u64,
}

/// A held lane guard covering the final progress check and lookup. Mutations
/// and local ownership transitions must use this lane too. It must not reenter Raft.
pub trait OwnerReadLane {
    fn progress(&self) -> OwnerReadProgress;

    fn lookup(&self, key: &StructuredKey) -> ClientBackendOutcome;
}

/// Local value executor required by the owner backend. Routing admits the
/// authenticated tenant and resolves the current namespace incarnation and
/// partition. Lane acquisition must return immediately and must not read the
/// key or materialize a value.
pub trait OwnerReadValues: Send + Sync + 'static {
    type Lane<'a>: OwnerReadLane
    where
        Self: 'a;

    fn route(
        &self,
        tenant: &str,
        namespace: &Namespace,
        key: &StructuredKey,
    ) -> Option<OwnerReadRoute>;

    fn try_lock_lane(&self, invocation: &ReadInvocation<'_>) -> Option<Self::Lane<'_>>;
}

/// Staged GET-only backend using the normal HC/1 and HC/2 dispatcher. Assembly
/// requires an explicit value executor and never grants public member readiness.
pub struct RaftOwnerReadBackend<S: RaftLogStore, V> {
    raft: Arc<RaftMetadataRuntime<S>>,
    transport: Arc<dyn RaftMessageSink>,
    values: V,
    local: PartitionMember,
    partition_count: NonZeroU32,
    replication: ReplicationConfig,
    next_invocation: AtomicU64,
}

impl<S: RaftLogStore, V> fmt::Debug for RaftOwnerReadBackend<S, V> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RaftOwnerReadBackend")
            .field("local", &self.local)
            .finish_non_exhaustive()
    }
}

impl<S: RaftLogStore, V: OwnerReadValues> RaftOwnerReadBackend<S, V> {
    pub fn new(
        raft: Arc<RaftMetadataRuntime<S>>,
        transport: Arc<dyn RaftMessageSink>,
        values: V,
        local: PartitionMember,
        partition_count: NonZeroU32,
        replication: ReplicationConfig,
    ) -> Self {
        Self {
            raft,
            transport,
            values,
            local,
            partition_count,
            replication,
            next_invocation: AtomicU64::new(1),
        }
    }

    async fn read(&self, request: VerifiedClientRequest<'_>) -> ClientBackendOutcome {
        let envelope = request.envelope();
        let ClientRequest::Get { ns, key } = &envelope.request else {
            return Err(ClientErrorEnvelope::new(
                ClientErrorCode::BackendUnavailable,
                false,
                "the owner-read backend only supports GET",
            ));
        };
        let now = Instant::now();
        let deadline = request
            .transport_deadline()
            .unwrap_or_else(|| {
                let budget = envelope
                    .deadline_ms
                    .map(Duration::from_millis)
                    .unwrap_or(MAX_READ_WAIT);
                now + budget.min(MAX_READ_WAIT)
            })
            .min(now + MAX_READ_WAIT);
        check_deadline(deadline)?;

        let tenant = request.identity().tenant();
        let route = self.values.route(tenant, ns, key).ok_or_else(unavailable)?;
        let invocation_id = self
            .next_invocation
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .ok()
            .and_then(NonZeroU64::new)
            .ok_or_else(unavailable)?;
        let invocation = ReadInvocation {
            invocation_id,
            tenant,
            namespace: ns.as_str(),
            namespace_generation: route.namespace_generation,
            partition: route.partition,
        };

        let snapshot = self.raft.partition_snapshot().ok_or_else(unavailable)?;
        let (term, epoch) = (snapshot.term, snapshot.epoch);
        let placement =
            PartitionTopology::from_snapshot(snapshot, self.partition_count, self.replication)
                .and_then(|topology| topology.placement(invocation.partition))
                .map_err(|_| unavailable())?;
        if placement.primary != self.local {
            return Err(unavailable());
        }
        check_deadline(deadline)?;

        let proof = timeout_at(deadline, async {
            let (pending, outbound) = self.raft.begin_read_index().map_err(|_| unavailable())?;
            let sending = send_raft_messages_with_diagnostics(&self.transport, outbound, None);
            let proof = pending.complete();
            tokio::pin!(proof);
            // A failed peer send can coexist with a successful quorum. Raft
            // is the authority for completion and the deadline bounds both futures.
            let result = tokio::select! {
                result = &mut proof => result,
                _ = sending => proof.await,
            };
            result.map_err(|_| unavailable())
        })
        .await
        .map_err(|_| deadline_exceeded())??;

        check_deadline(deadline)?;
        // Lock order is value lane, then Raft apply lock. No await or Raft
        // reentry is allowed while reading the value under both guards.
        let lane = self
            .values
            .try_lock_lane(&invocation)
            .ok_or_else(unavailable)?;
        proof
            .with_snapshot(|snapshot, index| {
                check_deadline(deadline)?;
                let progress = lane.progress();
                if progress.epoch != epoch
                    || progress.namespace_generation != invocation.namespace_generation
                {
                    return Err(unavailable());
                }
                let topology = PartitionTopology::from_snapshot(
                    snapshot,
                    self.partition_count,
                    self.replication,
                )
                .map_err(|_| unavailable())?;
                OwnerReadBarrier {
                    invocation,
                    term,
                    epoch,
                    owner: placement.primary,
                    quorum_index: index,
                    required_value_prefix: progress.required_prefix,
                }
                .lookup(
                    &invocation,
                    &topology,
                    &self.local,
                    progress.applied_prefix,
                    0,
                    || {
                        check_deadline(deadline)?;
                        lane.lookup(key)
                    },
                )
                .map_err(|_| unavailable())?
            })
            .map_err(|_| unavailable())?
    }
}

impl<S: RaftLogStore + 'static, V: OwnerReadValues> ClientDataBackend
    for RaftOwnerReadBackend<S, V>
{
    fn execute<'a>(&'a self, request: VerifiedClientRequest<'a>) -> ClientBackendFuture<'a> {
        Box::pin(self.read(request))
    }
}

pub(crate) fn unavailable() -> ClientErrorEnvelope {
    ClientErrorEnvelope::new(
        ClientErrorCode::BackendUnavailable,
        true,
        "IMap owner read authority is unavailable",
    )
}

fn check_deadline(deadline: Instant) -> Result<(), ClientErrorEnvelope> {
    if Instant::now() >= deadline {
        return Err(deadline_exceeded());
    }
    Ok(())
}

fn deadline_exceeded() -> ClientErrorEnvelope {
    ClientErrorEnvelope::new(
        ClientErrorCode::DeadlineExceeded,
        true,
        "IMap owner read deadline expired",
    )
}
