//! Admitted replica snapshots for the IMap GET owner lane.

use std::fmt::Write;
use std::num::{NonZeroU32, NonZeroU64};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use hydracache::{
    fnv1a64_bytes, CanonicalMapKey, CanonicalMapKeyBounds, ClusterEpoch, PartitionId,
    ReplicatedSlot, ReplicatedValueRecord, ReplicatedValueStore, ValueStoreError,
};
use hydracache_client_protocol::{ClientResponse, Namespace, StructuredKey};
use hydracache_client_transport_axum::ClientBackendOutcome;
use thiserror::Error;

use crate::imap_owner_read::{
    unavailable, OwnerReadLane, OwnerReadProgress, OwnerReadRoute, OwnerReadValues,
};
use crate::imap_read_barrier::ReadInvocation;

/// D2's staged count with the candidate FNV-1a partitioner.
pub const D2_PARTITION_COUNT: NonZeroU32 = NonZeroU32::new(271).unwrap();

/// Explicit scope for an isolated, read-only dataset.
pub struct ReplicaReadScope {
    tenant: String,
    namespace: Namespace,
    generation: NonZeroU64,
    decode_key: fn(&StructuredKey) -> Option<Vec<u8>>,
}

impl ReplicaReadScope {
    pub(crate) fn identity(&self) -> (&str, &Namespace, NonZeroU64) {
        (&self.tenant, &self.namespace, self.generation)
    }
    /// Bind one tenant and namespace incarnation to its admitted wire key decoder.
    pub fn new(
        tenant: String,
        namespace: Namespace,
        generation: NonZeroU64,
        decode_key: fn(&StructuredKey) -> Option<Vec<u8>>,
    ) -> Self {
        Self {
            tenant,
            namespace,
            generation,
            decode_key,
        }
    }

    /// Derive the partition and storage key from canonical bytes.
    /// The hex rendering fits the existing string-keyed replica store.
    pub fn locate(&self, raw_key: Vec<u8>) -> Result<(PartitionId, String), ReplicaReadError> {
        let key = CanonicalMapKey::new(
            &self.tenant,
            self.namespace.as_str(),
            self.generation.get(),
            raw_key,
        )
        .encode(CanonicalMapKeyBounds::default())?;
        let partition = staged_partition(&key);
        let mut storage_key = String::with_capacity(key.len() * 2);
        for byte in key {
            write!(storage_key, "{byte:02x}").expect("writing to String cannot fail");
        }
        Ok((partition, storage_key))
    }

    pub(crate) fn route_key(
        &self,
        tenant: &str,
        namespace: &Namespace,
        key: &StructuredKey,
    ) -> Option<(OwnerReadRoute, String)> {
        if tenant != self.tenant || namespace != &self.namespace {
            return None;
        }
        let (partition, storage_key) = self.locate((self.decode_key)(key)?).ok()?;
        Some((
            OwnerReadRoute {
                namespace_generation: self.generation,
                partition,
            },
            storage_key,
        ))
    }

    pub(crate) fn admits_storage_key(&self, storage_key: &str, partition: PartitionId) -> bool {
        let bounds = CanonicalMapKeyBounds::default();
        if storage_key.len() > bounds.max_frame_bytes * 2
            || !storage_key.len().is_multiple_of(2)
            || !storage_key
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return false;
        }
        let raw: Vec<_> = (0..storage_key.len())
            .step_by(2)
            .map(|i| {
                u8::from_str_radix(&storage_key[i..i + 2], 16).expect("validated lowercase hex")
            })
            .collect();
        staged_partition(&raw) == partition
            && CanonicalMapKey::decode(&raw, bounds).is_ok_and(|key| {
                key.tenant == self.tenant
                    && key.namespace == self.namespace.as_str()
                    && key.namespace_generation == self.generation.get()
            })
    }
}

fn staged_partition(bytes: &[u8]) -> PartitionId {
    PartitionId::new((fnv1a64_bytes(bytes) % u64::from(D2_PARTITION_COUNT.get())) as u32)
}

/// A trusted installer supplies its certified prefix separately from the data.
/// A record version alone cannot establish coverage of a partition prefix.
#[derive(Clone, Copy, Debug)]
pub struct ReplicaReadPrefix {
    pub epoch: ClusterEpoch,
    pub applied: u64,
    pub required: u64,
}

struct PublishedReplica<S> {
    prefix: ReplicaReadPrefix,
    store: S,
}

struct ReplicaState<S> {
    scope: ReplicaReadScope,
    partitions: Vec<Mutex<Option<PublishedReplica<S>>>>,
    lookups: AtomicU64,
}

/// Thread-safe owner lanes backed by the existing replicated value-store seam.
pub struct ReplicatedReadValues<S>(Arc<ReplicaState<S>>);

impl<S> Clone for ReplicatedReadValues<S> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

#[derive(Debug, Error)]
pub enum ReplicaReadError {
    #[error(transparent)]
    Key(#[from] hydracache::CanonicalMapKeyError),
    #[error(transparent)]
    Store(#[from] ValueStoreError),
    #[error("the replica snapshot does not match its partition or applied prefix")]
    InvalidSnapshot,
    #[error("the replica snapshot does not advance the published prefix")]
    Regressed,
    #[error("the replica lane is unavailable")]
    LaneUnavailable,
}

impl<S: ReplicatedValueStore> ReplicatedReadValues<S> {
    pub fn new(scope: ReplicaReadScope) -> Self {
        Self(Arc::new(ReplicaState {
            scope,
            partitions: (0..D2_PARTITION_COUNT.get())
                .map(|_| Mutex::new(None))
                .collect(),
            lookups: AtomicU64::new(0),
        }))
    }

    /// Publish a prepared partition image atomically to readers.
    /// The caller owns the completeness proof for the supplied prefix.
    pub fn install(
        &self,
        partition: PartitionId,
        prefix: ReplicaReadPrefix,
        store: S,
    ) -> Result<(), ReplicaReadError> {
        let lane = self
            .0
            .partitions
            .get(partition.value() as usize)
            .ok_or(ReplicaReadError::InvalidSnapshot)?;
        for (storage_key, record) in store.scan_all()? {
            if !self.0.scope.admits_storage_key(&storage_key, partition)
                || !record_matches(&record, partition, prefix)
            {
                return Err(ReplicaReadError::InvalidSnapshot);
            }
        }
        let mut published = lane.lock().map_err(|_| ReplicaReadError::LaneUnavailable)?;
        if published.as_ref().is_some_and(|current| {
            prefix.epoch < current.prefix.epoch
                || prefix.required < current.prefix.required
                || (prefix.epoch == current.prefix.epoch
                    && prefix.applied <= current.prefix.applied)
        }) {
            return Err(ReplicaReadError::Regressed);
        }
        let replaced = published.replace(PublishedReplica { prefix, store });
        drop(published);
        drop(replaced);
        Ok(())
    }

    /// Aggregate probes of the backing store, without exposing keys or tenants.
    pub fn store_lookups(&self) -> u64 {
        self.0.lookups.load(Ordering::Relaxed)
    }
}

pub struct ReplicatedReadLane<'a, S> {
    replica: MutexGuard<'a, Option<PublishedReplica<S>>>,
    scope: &'a ReplicaReadScope,
    partition: PartitionId,
    lookups: &'a AtomicU64,
}

impl<S: ReplicatedValueStore> OwnerReadLane for ReplicatedReadLane<'_, S> {
    fn progress(&self) -> OwnerReadProgress {
        let prefix = self
            .replica
            .as_ref()
            .expect("held published replica")
            .prefix;
        OwnerReadProgress {
            namespace_generation: self.scope.generation,
            epoch: prefix.epoch,
            applied_prefix: prefix.applied,
            required_prefix: prefix.required,
        }
    }

    fn lookup(&self, key: &StructuredKey) -> ClientBackendOutcome {
        let raw = (self.scope.decode_key)(key).ok_or_else(unavailable)?;
        let (partition, storage_key) = self.scope.locate(raw).map_err(|_| unavailable())?;
        if partition != self.partition {
            return Err(unavailable());
        }
        let replica = self.replica.as_ref().expect("held published replica");
        self.lookups.fetch_add(1, Ordering::Relaxed);
        let record = replica.store.get(&storage_key).map_err(|_| unavailable())?;
        let value = match record {
            Some(record) if !record_matches(&record, partition, replica.prefix) => {
                return Err(unavailable())
            }
            Some(ReplicatedValueRecord {
                state: ReplicatedSlot::Value { value, .. },
                ..
            }) => Some(value),
            _ => None,
        };
        Ok(ClientResponse::Value { value })
    }
}

impl<S: ReplicatedValueStore + 'static> OwnerReadValues for ReplicatedReadValues<S> {
    type Lane<'a> = ReplicatedReadLane<'a, S>;

    fn route(
        &self,
        tenant: &str,
        namespace: &Namespace,
        key: &StructuredKey,
    ) -> Option<OwnerReadRoute> {
        self.0
            .scope
            .route_key(tenant, namespace, key)
            .map(|(route, _)| route)
    }

    fn try_lock_lane(&self, invocation: &ReadInvocation<'_>) -> Option<Self::Lane<'_>> {
        let scope = &self.0.scope;
        if invocation.tenant != scope.tenant
            || invocation.namespace != scope.namespace.as_str()
            || invocation.namespace_generation != scope.generation
        {
            return None;
        }
        let replica = self
            .0
            .partitions
            .get(invocation.partition.value() as usize)?
            .try_lock()
            .ok()?;
        replica.as_ref()?;
        Some(ReplicatedReadLane {
            replica,
            scope,
            partition: invocation.partition,
            lookups: &self.0.lookups,
        })
    }
}

fn record_matches(
    record: &ReplicatedValueRecord,
    partition: PartitionId,
    prefix: ReplicaReadPrefix,
) -> bool {
    let slot_version = match &record.state {
        ReplicatedSlot::Value { version, .. } | ReplicatedSlot::Tombstone { version, .. } => {
            *version
        }
    };
    record.partition == partition
        && record.epoch <= prefix.epoch
        && record.version > 0
        && slot_version == record.version
        && record.version <= prefix.applied
}
