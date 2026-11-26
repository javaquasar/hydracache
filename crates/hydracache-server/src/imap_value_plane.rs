//! Synchronous value replication and the Raft-certified GET prefix.

use std::fmt;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use hydracache::{
    CacheError, CacheResult, ClusterTimeAuthority, PartitionId, PartitionValuePrefix,
    ReplicatedSlot, ReplicationConfig,
};
use hydracache_client_protocol::{
    ClientErrorCode, ClientErrorEnvelope, ClientRequest, ClientResponse, Namespace, StructuredKey,
    TtlState,
};
use hydracache_client_transport_axum::{
    ClientBackendFuture, ClientBackendOutcome, ClientDataBackend, VerifiedClientRequest,
};
use hydracache_cluster_raft::{
    RaftLogStore, RaftMessageSink, RaftMetadataRuntime, RaftPartitionSnapshot,
};
use serde::{Deserialize, Serialize};
use tokio::sync::{watch, Semaphore};
use tokio::time::{sleep, timeout_at, Instant};

use crate::grid_host::send_raft_messages_with_diagnostics;
use crate::imap_owner_read::{
    request_read_index, unavailable, OwnerReadLane, OwnerReadProgress, OwnerReadRoute,
    OwnerReadValues, RaftOwnerReadBackend,
};
use crate::imap_read_barrier::ReadInvocation;
use crate::imap_replicated_read::{ReplicaReadScope, D2_PARTITION_COUNT};
use crate::imap_topology::{PartitionMember, PartitionPlacement, PartitionTopology};
use crate::imap_value_decision::{Bounds, Change, WriteOp};
use crate::imap_value_journal::{
    hash, next_record, ApplyResult, CachedDecision, Mutation, PartitionJournal, StoredEntry,
    ValueJournal,
};

const MAX_VALUE_BYTES: usize = 128 * 1024;
const MAX_KEY_HEX_BYTES: usize = 64 * 1024;
const MAX_ID_BYTES: usize = 128;
const MAX_WRITES: usize = 32;
const WRITE_WAIT: Duration = Duration::from_secs(2);
const EXPIRY_TICK: Duration = Duration::from_millis(75);

/// An explicitly supplied bounded clock. A host without an admitted provider
/// leaves time-dependent operations unavailable.
pub struct ReplicaClock {
    pub authority: Arc<dyn ClusterTimeAuthority>,
    pub maximum_uncertainty_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ReplicaApply {
    version: u16,
    pub target: PartitionMember,
    mutation: Mutation,
}

impl ReplicaApply {
    pub(crate) fn epoch(&self) -> u64 {
        self.mutation.record.epoch.value()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ReplicaApplyAck {
    version: u16,
    replica: PartitionMember,
    prefix: PartitionValuePrefix,
}

#[async_trait::async_trait]
pub(crate) trait ReplicaTransport: Send + Sync {
    async fn apply(&self, request: ReplicaApply) -> CacheResult<ReplicaApplyAck>;
}

struct Lane {
    storage: Mutex<()>,
    writer: Semaphore,
}

/// Per-member durable replica service for one explicitly configured scope.
#[doc(hidden)]
pub struct ValueReplica {
    local: PartitionMember,
    scope: ReplicaReadScope,
    replication: ReplicationConfig,
    journal: ValueJournal,
    lanes: Vec<Lane>,
    writes: Semaphore,
    lookups: AtomicU64,
    applies: AtomicU64,
    clock: Option<ReplicaClock>,
}

impl ValueReplica {
    pub(crate) fn open(
        path: &Path,
        local: PartitionMember,
        scope: ReplicaReadScope,
        replication: ReplicationConfig,
    ) -> CacheResult<Self> {
        Self::with_journal(ValueJournal::open(path)?, local, scope, replication)
    }

    fn with_journal(
        journal: ValueJournal,
        local: PartitionMember,
        scope: ReplicaReadScope,
        replication: ReplicationConfig,
    ) -> CacheResult<Self> {
        replication
            .validate()
            .map_err(|error| CacheError::Backend(error.to_string()))?;
        if replication.sync_backups > 2
            || replication.replication_factor != replication.sync_backups + 1
        {
            return Err(CacheError::Backend(
                "W10c requires all configured backups to be synchronous".into(),
            ));
        }
        let (tenant, namespace, generation) = scope.identity();
        journal.bind_scope(tenant, namespace.as_str(), generation.get())?;
        Ok(Self {
            local,
            scope,
            replication,
            journal,
            lanes: (0..D2_PARTITION_COUNT.get())
                .map(|_| Lane {
                    storage: Mutex::new(()),
                    writer: Semaphore::new(1),
                })
                .collect(),
            writes: Semaphore::new(MAX_WRITES),
            lookups: AtomicU64::new(0),
            applies: AtomicU64::new(0),
            clock: None,
        })
    }

    pub(crate) fn with_clock(mut self, clock: Option<ReplicaClock>) -> Self {
        self.clock = clock;
        self
    }

    fn time(&self) -> Option<Bounds> {
        let clock = self.clock.as_ref()?;
        clock
            .authority
            .observe()?
            .bounds(clock.maximum_uncertainty_ms)
    }

    pub fn store_lookups(&self) -> u64 {
        self.lookups.load(Ordering::Relaxed)
    }

    pub fn replica_applies(&self) -> u64 {
        self.applies.load(Ordering::Relaxed)
    }

    fn lane(&self, partition: PartitionId) -> Option<&Lane> {
        self.lanes.get(partition.value() as usize)
    }

    fn placement(
        &self,
        snapshot: &RaftPartitionSnapshot,
        partition: PartitionId,
    ) -> Option<PartitionPlacement> {
        PartitionTopology::from_snapshot(snapshot.clone(), D2_PARTITION_COUNT, self.replication)
            .and_then(|topology| topology.placement(partition))
            .ok()
    }

    pub(crate) fn apply_replica(
        &self,
        snapshot: &RaftPartitionSnapshot,
        source: &str,
        epoch: u64,
        request: ReplicaApply,
    ) -> CacheResult<ReplicaApplyAck> {
        let mutation = request.mutation;
        let record = &mutation.record;
        let required = snapshot.value_prefixes.get(&record.partition).copied();
        let expected_version = required.map_or(0, |prefix| prefix.version).checked_add(1);
        let slot_version = match &record.state {
            ReplicatedSlot::Value { version, value } if value.len() <= MAX_VALUE_BYTES => *version,
            ReplicatedSlot::Tombstone {
                version,
                gc_eligible_after: None,
            } => *version,
            _ => return Err(replica_rejected()),
        };
        let Some(placement) = self.placement(snapshot, record.partition) else {
            return Err(replica_rejected());
        };
        if request.version != mutation.wire_version()
            || request.target != self.local
            || source != mutation.owner.node.as_str()
            || epoch != record.epoch.value()
            || record.epoch != snapshot.epoch
            || placement.primary != mutation.owner
            || placement.backups != mutation.backups
            || !placement.backups.contains(&self.local)
            || expected_version != Some(record.version)
            || slot_version != record.version
            || (record.is_tombstone() && mutation.expires_not_before_ms.is_some())
            || (mutation.outcome_only
                && (!record.is_tombstone()
                    || mutation.identity.is_none()
                    || mutation.decision.is_none()))
            || mutation.previous != required.map_or([0; 32], |prefix| prefix.digest)
            || mutation.key.len() > MAX_KEY_HEX_BYTES
            || mutation
                .identity
                .as_ref()
                .is_some_and(|id| id.is_empty() || id.len() > MAX_ID_BYTES)
            || !self
                .scope
                .admits_storage_key(&mutation.key, record.partition)
        {
            return Err(replica_rejected());
        }
        let lane = self.lane(record.partition).ok_or_else(replica_rejected)?;
        let _guard = lane.storage.try_lock().map_err(|_| replica_rejected())?;
        let prefix = mutation.certificate();
        if self.journal.apply(required, mutation, false)? != ApplyResult::Flushed {
            return Err(replica_rejected());
        }
        self.applies.fetch_add(1, Ordering::Relaxed);
        Ok(ReplicaApplyAck {
            version: request.version,
            replica: self.local.clone(),
            prefix,
        })
    }
}

fn replica_rejected() -> CacheError {
    CacheError::Backend("W10c replica apply refused".into())
}

pub struct JournalReadLane<'a> {
    replica: &'a ValueReplica,
    _guard: MutexGuard<'a, ()>,
    invocation: OwnerReadRoute,
    scope: (&'a str, &'a Namespace),
    proof: Option<(
        OwnerReadProgress,
        PartitionJournal,
        Option<PartitionValuePrefix>,
    )>,
}

impl OwnerReadLane for JournalReadLane<'_> {
    fn progress(&self) -> OwnerReadProgress {
        self.proof
            .as_ref()
            .expect("W10c lane requires a Raft proof")
            .0
    }

    fn prove_prefix(
        &mut self,
        snapshot: &RaftPartitionSnapshot,
    ) -> Result<OwnerReadProgress, ClientErrorEnvelope> {
        let required = snapshot
            .value_prefixes
            .get(&self.invocation.partition)
            .copied();
        let meta = self
            .replica
            .journal
            .load(self.invocation.partition)
            .map_err(|_| unavailable())?;
        if !meta.covers(required) {
            return Err(unavailable());
        }
        let version = required.map_or(0, |prefix| prefix.version);
        let progress = OwnerReadProgress {
            namespace_generation: self.invocation.namespace_generation,
            epoch: snapshot.epoch,
            applied_prefix: version,
            required_prefix: version,
        };
        self.proof = Some((progress, meta, required));
        Ok(progress)
    }

    fn lookup(&self, key: &StructuredKey) -> ClientBackendOutcome {
        let (live, _) = self.read_live(key)?;
        Ok(ClientResponse::Value {
            value: live
                .as_ref()
                .and_then(StoredEntry::value)
                .map(<[u8]>::to_vec),
        })
    }

    fn lookup_ttl(&self, key: &StructuredKey) -> ClientBackendOutcome {
        let (live, remaining) = self.read_live(key)?;
        Ok(ClientResponse::Ttl {
            state: match (live, remaining) {
                (None, _) => TtlState::Missing,
                (Some(_), Some(ttl_ms)) => TtlState::ExpiresIn { ttl_ms },
                (Some(_), None) => TtlState::Persistent,
            },
        })
    }
}

impl JournalReadLane<'_> {
    fn read_live(
        &self,
        key: &StructuredKey,
    ) -> Result<(Option<StoredEntry>, Option<u64>), ClientErrorEnvelope> {
        let (_, storage_key) = self
            .replica
            .scope
            .route_key(self.scope.0, self.scope.1, key)
            .ok_or_else(unavailable)?;
        let (_, meta, required) = self.proof.as_ref().ok_or_else(unavailable)?;
        self.replica.lookups.fetch_add(1, Ordering::Relaxed);
        let entry = self
            .replica
            .journal
            .read_entry(self.invocation.partition, &storage_key, meta, *required)
            .map_err(|_| unavailable())?
            .filter(|entry| entry.value().is_some());
        let remaining = match entry.as_ref().and_then(|entry| entry.expires_not_before_ms) {
            Some(deadline) => {
                let (_, upper) = self.replica.time().ok_or_else(unavailable)?;
                // A timed read cannot expose a possibly expired record or
                // report absence before the fenced tombstone commits.
                Some(
                    deadline
                        .checked_sub(upper)
                        .filter(|remaining| *remaining > 0)
                        .ok_or_else(unavailable)?,
                )
            }
            None => None,
        };
        Ok((entry, remaining))
    }
}

impl OwnerReadValues for Arc<ValueReplica> {
    type Lane<'a> = JournalReadLane<'a>;

    fn route(
        &self,
        tenant: &str,
        namespace: &Namespace,
        key: &StructuredKey,
    ) -> Option<OwnerReadRoute> {
        self.scope
            .route_key(tenant, namespace, key)
            .map(|(route, _)| route)
    }

    fn try_lock_lane(&self, invocation: &ReadInvocation<'_>) -> Option<Self::Lane<'_>> {
        let (tenant, namespace, generation) = self.scope.identity();
        if tenant != invocation.tenant
            || namespace.as_str() != invocation.namespace
            || generation != invocation.namespace_generation
        {
            return None;
        }
        let _guard = self.lane(invocation.partition)?.storage.try_lock().ok()?;
        Some(JournalReadLane {
            replica: self,
            _guard,
            invocation: OwnerReadRoute {
                namespace_generation: invocation.namespace_generation,
                partition: invocation.partition,
            },
            scope: (tenant, namespace),
            proof: None,
        })
    }
}

pub(crate) struct ValueBackend<S: RaftLogStore> {
    pub(crate) raft: Arc<RaftMetadataRuntime<S>>,
    pub(crate) raft_transport: Arc<dyn RaftMessageSink>,
    pub(crate) value_transport: Option<Arc<dyn ReplicaTransport>>,
    pub(crate) replica: Arc<ValueReplica>,
    pub(crate) read: RaftOwnerReadBackend<S, Arc<ValueReplica>>,
}

impl<S: RaftLogStore> fmt::Debug for ValueBackend<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ValueBackend")
            .field("local", &self.replica.local)
            .finish_non_exhaustive()
    }
}

impl<S: RaftLogStore + 'static> ClientDataBackend for ValueBackend<S> {
    fn execute<'a>(&'a self, request: VerifiedClientRequest<'a>) -> ClientBackendFuture<'a> {
        if matches!(
            request.envelope().request,
            ClientRequest::Get { .. } | ClientRequest::GetTtl { .. }
        ) {
            self.read.execute(request)
        } else {
            Box::pin(self.write(request))
        }
    }
}

struct WriteInput<'a> {
    partition: PartitionId,
    key: String,
    identity: Option<String>,
    fingerprint: [u8; 32],
    operation: WriteOp<'a>,
}

impl<S: RaftLogStore> ValueBackend<S> {
    async fn write(&self, request: VerifiedClientRequest<'_>) -> ClientBackendOutcome {
        let envelope = request.envelope();
        let (ns, key, operation) = WriteOp::request(&envelope.request)
            .ok_or_else(|| write_error(false, "unsupported W10d mutation or consistency"))?;
        let (route, key) = self
            .replica
            .scope
            .route_key(request.identity().tenant(), ns, key)
            .ok_or_else(|| write_error(false, "value scope unavailable"))?;
        if key.len() > MAX_KEY_HEX_BYTES
            || !operation.fits(MAX_VALUE_BYTES.min(request.limits().max_value_bytes))
            || envelope
                .idempotency_key
                .as_ref()
                .is_some_and(|id| id.is_empty() || id.len() > MAX_ID_BYTES)
        {
            return Err(ClientErrorEnvelope::new(
                ClientErrorCode::TooLarge,
                false,
                "mutation exceeds W10d limits",
            ));
        }
        let now = Instant::now();
        let relative = envelope
            .deadline_ms
            .map(Duration::from_millis)
            .unwrap_or(WRITE_WAIT);
        let deadline = request
            .transport_deadline()
            .unwrap_or(now + relative.min(WRITE_WAIT))
            .min(now + WRITE_WAIT);
        self.write_to(
            WriteInput {
                partition: route.partition,
                key,
                identity: envelope.idempotency_key.clone(),
                fingerprint: hash(&envelope.request),
                operation,
            },
            deadline,
        )
        .await
    }

    async fn write_to(&self, input: WriteInput<'_>, deadline: Instant) -> ClientBackendOutcome {
        let _write = self
            .replica
            .writes
            .try_acquire()
            .map_err(|_| before_apply())?;
        let lane = self
            .replica
            .lane(input.partition)
            .ok_or_else(before_apply)?;
        let _partition = lane.writer.try_acquire().map_err(|_| before_apply())?;
        let proof = request_read_index(&self.raft, &self.raft_transport, deadline).await?;
        let (mutation, response) = {
            let _guard = lane.storage.try_lock().map_err(|_| before_apply())?;
            proof
                .with_snapshot(|snapshot, _| {
                    if Instant::now() >= deadline {
                        return Err(before_apply());
                    }
                    self.prepare(input, &snapshot)
                })
                .map_err(|_| before_apply())??
        };
        let Some(mutation) = mutation else {
            return Ok(response);
        };
        let prefix = mutation.certificate();
        let wire_version = mutation.wire_version();
        timeout_at(deadline, async {
            for target in &mutation.backups {
                let transport = self.value_transport.as_ref().ok_or_else(ambiguous)?;
                let ack = transport
                    .apply(ReplicaApply {
                        version: wire_version,
                        target: target.clone(),
                        mutation: mutation.clone(),
                    })
                    .await
                    .map_err(|_| ambiguous())?;
                if ack
                    != (ReplicaApplyAck {
                        version: wire_version,
                        replica: target.clone(),
                        prefix,
                    })
                {
                    return Err(ambiguous());
                }
            }
            self.raft
                .propose_value_prefix(prefix)
                .map_err(|_| ambiguous())?;
            let outbound = self.raft.take_outbound_messages();
            let _ = send_raft_messages_with_diagnostics(&self.raft_transport, outbound, None).await;
            loop {
                let snapshot = self.raft.partition_snapshot().ok_or_else(ambiguous)?;
                if snapshot.value_prefixes.get(&prefix.partition) == Some(&prefix) {
                    return Ok(response);
                }
                if snapshot.epoch != prefix.epoch {
                    return Err(ambiguous());
                }
                sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .map_err(|_| ambiguous())?
    }

    fn prepare(
        &self,
        input: WriteInput<'_>,
        snapshot: &RaftPartitionSnapshot,
    ) -> Result<(Option<Mutation>, ClientResponse), ClientErrorEnvelope> {
        let placement = self
            .replica
            .placement(snapshot, input.partition)
            .ok_or_else(before_apply)?;
        if placement.primary != self.replica.local {
            return Err(before_apply());
        }
        let prior = snapshot.value_prefixes.get(&input.partition).copied();
        let journal = &self.replica.journal;
        let meta = journal.load(input.partition).map_err(|_| before_apply())?;
        if !meta.covers(prior) {
            return Err(before_apply());
        }
        if let Some(retained) = input
            .identity
            .as_deref()
            .and_then(|id| meta.retained(prior, id))
        {
            return if retained.request == input.fingerprint {
                Ok((None, retained.response.clone()))
            } else {
                Err(identity_conflict())
            };
        }
        let time = self.replica.time();
        if let WriteOp::ExpireDue(ticket) = input.operation {
            if !time.is_some_and(|(lower, _)| lower >= ticket.deadline) {
                return Err(before_apply());
            }
        }
        let retry = meta
            .pending_retry(prior, snapshot.epoch)
            .and_then(|pending| {
                let decision = pending.decision.as_ref()?;
                (pending.key == input.key
                    && pending.identity == input.identity
                    && decision.request == input.fingerprint)
                    .then_some((pending.clone(), decision.response.clone()))
            });
        let (mutation, response) = match retry {
            Some(retry) => retry,
            None => {
                let current = if input.operation.needs_current() {
                    journal
                        .read_entry(input.partition, &input.key, &meta, prior)
                        .map_err(|_| before_apply())?
                } else {
                    None
                };
                let version = prior
                    .map_or(0, |prefix| prefix.version)
                    .checked_add(1)
                    .ok_or_else(before_apply)?;
                let decision = input
                    .operation
                    .decide(current.as_ref(), time, version)
                    .ok_or_else(|| {
                        write_error(true, "clock observation cannot authorize this decision")
                    })?;
                let outcome_only = decision.change.is_none();
                if outcome_only && input.identity.is_none() {
                    return Ok((None, decision.response));
                }
                let change = decision.change.unwrap_or_else(Change::tombstone);
                let mutation = Mutation {
                    owner: placement.primary,
                    backups: placement.backups,
                    previous: prior.map_or([0; 32], |prefix| prefix.digest),
                    key: input.key,
                    identity: input.identity,
                    record: next_record(input.partition, snapshot.epoch, prior, change.value)
                        .ok_or_else(before_apply)?,
                    expires_not_before_ms: change.expiry,
                    decision: Some(CachedDecision {
                        request: input.fingerprint,
                        response: decision.response.clone(),
                    }),
                    outcome_only,
                };
                (mutation, decision.response)
            }
        };
        match journal
            .apply(prior, mutation.clone(), true)
            .map_err(|_| ambiguous())?
        {
            ApplyResult::Flushed => Ok((Some(mutation), response)),
            ApplyResult::Replay => Ok((None, response)),
            ApplyResult::Busy => Err(before_apply()),
            ApplyResult::Conflict => Err(identity_conflict()),
        }
    }

    pub(crate) async fn run_expiry(&self, mut shutdown: watch::Receiver<bool>) {
        let mut timer = tokio::time::interval(EXPIRY_TICK);
        timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut cursor = None;
        while !*shutdown.borrow() {
            tokio::select! {
                changed = shutdown.changed() => {
                    if changed.is_err() { break; }
                }
                _ = timer.tick() => {
                    self.expire_batch(&mut cursor).await;
                }
            }
        }
    }

    pub(super) async fn expire_batch(&self, cursor: &mut Option<Vec<u8>>) {
        let Some((lower, _)) = self.replica.time() else {
            return;
        };
        let Ok(candidates) = self.replica.journal.expiry_batch(cursor, lower, 32) else {
            return;
        };
        for candidate in candidates {
            let is_owner = self
                .raft
                .partition_snapshot()
                .and_then(|snapshot| self.replica.placement(&snapshot, candidate.partition))
                .is_some_and(|placement| placement.primary == self.replica.local);
            if !is_owner {
                continue;
            }
            let _ = self
                .write_to(
                    WriteInput {
                        partition: candidate.partition,
                        key: candidate.key.clone(),
                        identity: None,
                        fingerprint: hash(&candidate),
                        operation: WriteOp::ExpireDue(&candidate),
                    },
                    Instant::now() + EXPIRY_TICK,
                )
                .await;
        }
    }
}

fn identity_conflict() -> ClientErrorEnvelope {
    ClientErrorEnvelope::new(
        ClientErrorCode::Conflict,
        false,
        "mutation identity conflict",
    )
}

fn before_apply() -> ClientErrorEnvelope {
    write_error(true, "mutation refused before apply")
}

fn ambiguous() -> ClientErrorEnvelope {
    write_error(false, "mutation applied locally, acknowledgement unknown")
}

fn write_error(retry: bool, detail: &str) -> ClientErrorEnvelope {
    ClientErrorEnvelope::new(ClientErrorCode::BackendUnavailable, retry, detail)
}

#[cfg(test)]
#[path = "imap_value_plane_tests.rs"]
mod tests;
