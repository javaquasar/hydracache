//! Synchronous value replication and the Raft-certified GET prefix.

use std::fmt;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use hydracache::{
    CacheError, CacheResult, PartitionId, PartitionValuePrefix, ReplicatedSlot, ReplicationConfig,
};
use hydracache_client_protocol::{
    ClientErrorCode, ClientErrorEnvelope, ClientRequest, ClientResponse, Namespace, StructuredKey,
};
use hydracache_client_transport_axum::{
    ClientBackendFuture, ClientBackendOutcome, ClientDataBackend, VerifiedClientRequest,
};
use hydracache_cluster_raft::{
    RaftLogStore, RaftMessageSink, RaftMetadataRuntime, RaftPartitionSnapshot,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;
use tokio::time::{sleep, timeout_at, Instant};

use crate::grid_host::send_raft_messages_with_diagnostics;
use crate::imap_owner_read::{
    request_read_index, unavailable, OwnerReadLane, OwnerReadProgress, OwnerReadRoute,
    OwnerReadValues, RaftOwnerReadBackend,
};
use crate::imap_read_barrier::ReadInvocation;
use crate::imap_replicated_read::{ReplicaReadScope, D2_PARTITION_COUNT};
use crate::imap_topology::{PartitionMember, PartitionPlacement, PartitionTopology};
use crate::imap_value_journal::{
    next_record, ApplyResult, Mutation, PartitionJournal, ValueJournal,
};

const WIRE_VERSION: u16 = 1;
const MAX_VALUE_BYTES: usize = 128 * 1024;
const MAX_KEY_HEX_BYTES: usize = 64 * 1024;
const MAX_ID_BYTES: usize = 128;
const MAX_WRITES: usize = 32;
const WRITE_WAIT: Duration = Duration::from_secs(2);

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
        })
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
        if request.version != WIRE_VERSION
            || request.target != self.local
            || source != mutation.owner.node.as_str()
            || epoch != record.epoch.value()
            || record.epoch != snapshot.epoch
            || placement.primary != mutation.owner
            || placement.backups != mutation.backups
            || !placement.backups.contains(&self.local)
            || expected_version != Some(record.version)
            || slot_version != record.version
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
            version: WIRE_VERSION,
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
        let (_, storage_key) = self
            .replica
            .scope
            .route_key(self.scope.0, self.scope.1, key)
            .ok_or_else(unavailable)?;
        let (_, meta, required) = self.proof.as_ref().ok_or_else(unavailable)?;
        self.replica.lookups.fetch_add(1, Ordering::Relaxed);
        let value = self
            .replica
            .journal
            .read(self.invocation.partition, &storage_key, meta, *required)
            .map_err(|_| unavailable())?;
        Ok(ClientResponse::Value { value })
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
        if matches!(request.envelope().request, ClientRequest::Get { .. }) {
            self.read.execute(request)
        } else {
            Box::pin(self.write(request))
        }
    }
}

impl<S: RaftLogStore> ValueBackend<S> {
    async fn write(&self, request: VerifiedClientRequest<'_>) -> ClientBackendOutcome {
        let envelope = request.envelope();
        let (ns, key, value, result) = match &envelope.request {
            ClientRequest::Put {
                ns,
                key,
                value,
                ttl_ms: None,
                dimensions,
            } if dimensions.is_empty() => (ns, key, Some(value.as_slice()), ClientResponse::Stored),
            ClientRequest::Invalidate { ns, key } => (ns, key, None, ClientResponse::Invalidated),
            _ => {
                return Err(write_error(
                    false,
                    "W10c supports persistent PUT and tombstones",
                ))
            }
        };
        let (route, key) = self
            .replica
            .scope
            .route_key(request.identity().tenant(), ns, key)
            .ok_or_else(|| write_error(false, "value scope unavailable"))?;
        if key.len() > MAX_KEY_HEX_BYTES
            || value.is_some_and(|bytes| {
                bytes.len() > MAX_VALUE_BYTES.min(request.limits().max_value_bytes)
            })
            || envelope
                .idempotency_key
                .as_ref()
                .is_some_and(|id| id.is_empty() || id.len() > MAX_ID_BYTES)
        {
            return Err(ClientErrorEnvelope::new(
                ClientErrorCode::TooLarge,
                false,
                "mutation exceeds W10c limits",
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
        let _write = self
            .replica
            .writes
            .try_acquire()
            .map_err(|_| before_apply())?;
        let lane = self
            .replica
            .lane(route.partition)
            .ok_or_else(before_apply)?;
        let _partition = lane.writer.try_acquire().map_err(|_| before_apply())?;
        let proof = request_read_index(&self.raft, &self.raft_transport, deadline).await?;
        let mutation = {
            let _guard = lane.storage.try_lock().map_err(|_| before_apply())?;
            proof
                .with_snapshot(|snapshot, _| {
                    let placement = self
                        .replica
                        .placement(&snapshot, route.partition)
                        .ok_or_else(before_apply)?;
                    if placement.primary != self.replica.local || Instant::now() >= deadline {
                        return Err(before_apply());
                    }
                    let prior = snapshot.value_prefixes.get(&route.partition).copied();
                    let mutation = Mutation {
                        owner: placement.primary,
                        backups: placement.backups,
                        previous: prior.map_or([0; 32], |prefix| prefix.digest),
                        key,
                        identity: envelope.idempotency_key.clone(),
                        record: next_record(
                            route.partition,
                            snapshot.epoch,
                            prior,
                            value.map(<[u8]>::to_vec),
                        )
                        .ok_or_else(before_apply)?,
                    };
                    match self
                        .replica
                        .journal
                        .apply(prior, mutation.clone(), true)
                        .map_err(|_| ambiguous())?
                    {
                        ApplyResult::Flushed => Ok(Some(mutation)),
                        ApplyResult::Replay => Ok(None),
                        ApplyResult::Busy => Err(before_apply()),
                        ApplyResult::Conflict => Err(ClientErrorEnvelope::new(
                            ClientErrorCode::Conflict,
                            false,
                            "mutation identity conflict",
                        )),
                    }
                })
                .map_err(|_| before_apply())??
        };
        let Some(mutation) = mutation else {
            return Ok(result);
        };
        let prefix = mutation.certificate();
        timeout_at(deadline, async {
            for target in &mutation.backups {
                let transport = self.value_transport.as_ref().ok_or_else(ambiguous)?;
                let ack = transport
                    .apply(ReplicaApply {
                        version: WIRE_VERSION,
                        target: target.clone(),
                        mutation: mutation.clone(),
                    })
                    .await
                    .map_err(|_| ambiguous())?;
                if ack
                    != (ReplicaApplyAck {
                        version: WIRE_VERSION,
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
                    return Ok(result);
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
