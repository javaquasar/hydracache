use hydracache::{
    recover_namespaces, ClusterEpoch, EffectiveReplicationMap, InMemoryReplicatedValueStore,
    NamespacePersistenceRule, NamespacePersistenceSettings, PartitionId, PersistencePolicy,
    PersistenceRegionPlacement, RecoveryNamespace, RecoveryPolicy, RegionId, Replicas,
    ReplicatedValueRecord, ReplicatedValueStore, StorageOp, StorageOpKind,
};

use crate::{SimRng, SimStorage, SimStorageError, StorageFault};

/// Persistence recovery fault class covered by the simulator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PersistenceRecoveryFault {
    /// Fsynced persistent values must survive a full cluster restart.
    WholeClusterCrashRestart,
    /// Uncommitted snapshot bytes must be lost on crash and never served.
    CrashMidSnapshot,
    /// Torn durable writes must fail checksum validation.
    TornDurableWrite,
    /// Corrupt durable bytes must fail checksum validation.
    StorageCorruption,
    /// Durable records from an older control-plane epoch must be fenced.
    StaleEpochOnDisk,
}

/// Deterministic persistence recovery scenario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistenceRecoveryScenario {
    /// Replay seed.
    pub seed: u64,
    /// Faults to exercise.
    pub faults: Vec<PersistenceRecoveryFault>,
}

impl PersistenceRecoveryScenario {
    /// Create a scenario with all persistence recovery faults enabled.
    pub fn all(seed: u64) -> Self {
        Self {
            seed,
            faults: vec![
                PersistenceRecoveryFault::WholeClusterCrashRestart,
                PersistenceRecoveryFault::CrashMidSnapshot,
                PersistenceRecoveryFault::TornDurableWrite,
                PersistenceRecoveryFault::StorageCorruption,
                PersistenceRecoveryFault::StaleEpochOnDisk,
            ],
        }
    }

    /// Create a scenario with selected faults.
    pub fn new(seed: u64, faults: Vec<PersistenceRecoveryFault>) -> Self {
        Self { seed, faults }
    }
}

/// Invariant report returned by persistence recovery validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistenceRecoveryInvariantReport {
    /// Replay seed.
    pub seed: u64,
    /// Faults exercised in order.
    pub faults_exercised: Vec<PersistenceRecoveryFault>,
    /// Whether sync-acked persistent values survived restart.
    pub sync_committed_survived_restart: bool,
    /// Whether RAM-only namespaces stayed empty after restart.
    pub non_persistent_empty_after_restart: bool,
    /// Whether an un-fsynced snapshot write was not visible after crash.
    pub mid_snapshot_uncommitted_not_served: bool,
    /// Whether stale durable records were fenced by authority epoch.
    pub stale_records_fenced: bool,
    /// Whether corrupt durable bytes were rejected.
    pub corrupt_storage_refused: bool,
    /// Whether torn durable writes were rejected.
    pub torn_write_refused: bool,
    /// Deterministic digest over the scenario trace and invariant outcomes.
    pub deterministic_digest: u64,
    /// Deterministic trace useful for reproducing a failure.
    pub trace: Vec<String>,
}

impl PersistenceRecoveryInvariantReport {
    /// Return whether all exercised invariants passed.
    pub fn passed(&self) -> bool {
        self.sync_committed_survived_restart
            && self.non_persistent_empty_after_restart
            && self.mid_snapshot_uncommitted_not_served
            && self.stale_records_fenced
            && self.corrupt_storage_refused
            && self.torn_write_refused
    }
}

/// Run the persistence recovery scenario.
pub fn run_persistence_recovery(
    scenario: PersistenceRecoveryScenario,
) -> PersistenceRecoveryInvariantReport {
    let mut rng = SimRng::from_seed(scenario.seed);
    let mut trace = Vec::new();

    let mut sync_committed_survived_restart = true;
    let mut non_persistent_empty_after_restart = true;
    let mut mid_snapshot_uncommitted_not_served = true;
    let mut stale_records_fenced = true;
    let mut corrupt_storage_refused = true;
    let mut torn_write_refused = true;

    for fault in &scenario.faults {
        match fault {
            PersistenceRecoveryFault::WholeClusterCrashRestart => {
                let suffix = rng.next_index(10_000);
                let persistent_key = format!("cache.jwt.pem/jwt:{suffix}");
                let ephemeral_key = format!("cache.ephemeral/tmp:{suffix}");
                let persistent_record = record(
                    0,
                    10 + suffix as u64,
                    3,
                    format!("jwt-{suffix}").into_bytes(),
                );
                let ephemeral_record = record(
                    0,
                    1 + suffix as u64,
                    3,
                    format!("tmp-{suffix}").into_bytes(),
                );
                let mut store = InMemoryReplicatedValueStore::default();
                store
                    .upsert(persistent_key.clone(), persistent_record.clone())
                    .expect("persistent upsert fits simulator store");
                store
                    .upsert(ephemeral_key.clone(), ephemeral_record)
                    .expect("ephemeral upsert fits simulator store");

                let reopened =
                    InMemoryReplicatedValueStore::reopen_from_snapshot(u64::MAX, store.snapshot());
                let report = recover_namespaces(
                    &reopened,
                    &persistence_policy(),
                    &local_region(),
                    ClusterEpoch::new(3),
                    &RecoveryPolicy::full_recovery_only(),
                    [
                        RecoveryNamespace::new("cache.jwt.pem", placement(), replication_map())
                            .with_key_prefix("cache.jwt.pem/"),
                        RecoveryNamespace::new("cache.ephemeral", placement(), replication_map())
                            .with_key_prefix("cache.ephemeral/"),
                    ],
                )
                .expect("restart recovery should succeed");

                sync_committed_survived_restart =
                    report.record("cache.jwt.pem", &persistent_key) == Some(&persistent_record);
                non_persistent_empty_after_restart = !report
                    .namespace_persistent("cache.ephemeral")
                    && report.record("cache.ephemeral", &ephemeral_key).is_none()
                    && report.non_persistent_skipped_total == 1;
                trace.push(format!(
                    "whole-cluster-crash-restart:key={persistent_key}:ephemeral={ephemeral_key}"
                ));
            }
            PersistenceRecoveryFault::CrashMidSnapshot => {
                let snapshot_key = format!("snapshot:{}", rng.next_index(10_000));
                let mut storage = SimStorage::new();
                storage
                    .apply_checked(write_request(1, &snapshot_key, b"uncommitted-snapshot"))
                    .expect("snapshot write accepted before crash");
                storage.crash();

                mid_snapshot_uncommitted_not_served = storage
                    .read_checked(&snapshot_key)
                    .expect("read after crash succeeds")
                    .is_none();
                trace.push(format!("crash-mid-snapshot:key={snapshot_key}"));
            }
            PersistenceRecoveryFault::TornDurableWrite => {
                let key = format!("durable-record:{}", rng.next_index(10_000));
                let mut storage = SimStorage::new();
                storage.inject_fault("default", StorageFault::TornWrite);
                storage
                    .apply_checked(write_request(2, &key, b"full-durable-record"))
                    .expect("torn write is accepted but damaged");
                storage.fsync();

                torn_write_refused = matches!(
                    storage.read_checked(&key),
                    Err(SimStorageError::ChecksumMismatch { .. })
                );
                trace.push(format!("torn-durable-write:key={key}"));
            }
            PersistenceRecoveryFault::StorageCorruption => {
                let key = format!("corrupt-record:{}", rng.next_index(10_000));
                let mut storage = SimStorage::new();
                storage
                    .apply_checked(write_request(3, &key, b"clean-durable-record"))
                    .expect("clean write succeeds");
                storage.fsync();
                storage.inject_fault("default", StorageFault::Corruption);

                corrupt_storage_refused = matches!(
                    storage.read_checked(&key),
                    Err(SimStorageError::ChecksumMismatch { .. })
                );
                trace.push(format!("storage-corruption:key={key}"));
            }
            PersistenceRecoveryFault::StaleEpochOnDisk => {
                let suffix = rng.next_index(10_000);
                let stale_key = format!("cache.jwt.pem/stale:{suffix}");
                let fresh_key = format!("cache.jwt.pem/fresh:{suffix}");
                let fresh_record = record(
                    0,
                    22 + suffix as u64,
                    3,
                    format!("fresh-{suffix}").into_bytes(),
                );
                let mut store = InMemoryReplicatedValueStore::default();
                store
                    .upsert(stale_key.clone(), record(0, 21 + suffix as u64, 1, b"old"))
                    .expect("stale upsert fits simulator store");
                store
                    .upsert(fresh_key.clone(), fresh_record.clone())
                    .expect("fresh upsert fits simulator store");

                let report = recover_namespaces(
                    &store,
                    &persistence_policy(),
                    &local_region(),
                    ClusterEpoch::new(2),
                    &RecoveryPolicy::full_recovery_only().with_auto_remove_stale_data(true),
                    [
                        RecoveryNamespace::new("cache.jwt.pem", placement(), replication_map())
                            .with_key_prefix("cache.jwt.pem/"),
                    ],
                )
                .expect("stale fencing recovery should succeed");

                stale_records_fenced = report.record("cache.jwt.pem", &stale_key).is_none()
                    && report.record("cache.jwt.pem", &fresh_key) == Some(&fresh_record)
                    && report.stale_fenced_total == 1
                    && report.auto_remove_stale_data;
                trace.push(format!(
                    "stale-epoch-on-disk:stale={stale_key}:fresh={fresh_key}"
                ));
            }
        }
    }

    let deterministic_digest = digest_report(
        scenario.seed,
        &scenario.faults,
        &trace,
        [
            sync_committed_survived_restart,
            non_persistent_empty_after_restart,
            mid_snapshot_uncommitted_not_served,
            stale_records_fenced,
            corrupt_storage_refused,
            torn_write_refused,
        ],
    );

    PersistenceRecoveryInvariantReport {
        seed: scenario.seed,
        faults_exercised: scenario.faults,
        sync_committed_survived_restart,
        non_persistent_empty_after_restart,
        mid_snapshot_uncommitted_not_served,
        stale_records_fenced,
        corrupt_storage_refused,
        torn_write_refused,
        deterministic_digest,
        trace,
    }
}

fn record(
    partition: u32,
    version: u64,
    epoch: u64,
    bytes: impl Into<Vec<u8>>,
) -> ReplicatedValueRecord {
    ReplicatedValueRecord::value(
        PartitionId::new(partition),
        version,
        ClusterEpoch::new(epoch),
        bytes.into(),
    )
}

fn write_request(request_id: u64, key: &str, value: &[u8]) -> StorageOp {
    StorageOp {
        request_id,
        kind: StorageOpKind::Write {
            key: key.to_owned(),
            value: value.to_vec(),
        },
    }
}

fn persistence_policy() -> PersistencePolicy {
    PersistencePolicy::try_new([
        NamespacePersistenceRule::persistent("cache.jwt.pem").expect("valid persistent rule"),
        NamespacePersistenceRule::new("cache.ephemeral", NamespacePersistenceSettings::ram_only())
            .expect("valid ram-only rule"),
    ])
    .expect("valid persistence policy")
}

fn local_region() -> RegionId {
    RegionId::new("eu")
}

fn placement() -> PersistenceRegionPlacement {
    PersistenceRegionPlacement::home_region_only("eu")
}

fn replication_map() -> EffectiveReplicationMap {
    EffectiveReplicationMap::new(Replicas::new("node-a", Vec::new()))
}

fn digest_report(
    seed: u64,
    faults: &[PersistenceRecoveryFault],
    trace: &[String],
    invariants: impl IntoIterator<Item = bool>,
) -> u64 {
    let mut digest = 0xcbf2_9ce4_8422_2325_u64;
    fn update(digest: &mut u64, bytes: &[u8]) {
        for byte in bytes {
            *digest ^= u64::from(*byte);
            *digest = digest.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    update(&mut digest, &seed.to_le_bytes());
    for fault in faults {
        update(&mut digest, &[*fault as u8]);
    }
    for entry in trace {
        update(&mut digest, entry.as_bytes());
        update(&mut digest, &[0]);
    }
    for invariant in invariants {
        update(&mut digest, &[u8::from(invariant)]);
    }
    digest
}
