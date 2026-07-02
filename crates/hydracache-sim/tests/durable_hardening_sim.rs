use std::collections::BTreeMap;
use std::time::Duration;

use hydracache::{
    recover_cluster_checkpoint, CheckpointCoordinator, ChecksummedReplicatedValueRecord,
    ClusterEpoch, DurabilitySnapshotManifest, EffectiveReplicationMap,
    InMemoryReplicatedValueStore, NamespacePersistenceRule, NodeCheckpointManifest, PartitionId,
    PersistencePolicy, PersistenceRegionPlacement, RecoveryNamespace, RecoveryPolicy, Replicas,
    ReplicatedValueRecord, ReplicatedValueStore, Scrubber, WriteWatermark,
    REPLICATED_VALUE_RECORD_CHECKSUM_FORMAT_VERSION,
};
use hydracache_sim::run_checkpoint_rescale;

#[derive(Debug, Clone, PartialEq, Eq)]
struct CheckpointRestartReport {
    seed: u64,
    checkpoint_id: String,
    recovered_records: u64,
    stale_fenced: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CorruptionReport {
    seed: u64,
    checksum_corruption_detected: bool,
    torn_future_format_detected: bool,
    corrupt_record_not_served: bool,
    repaired_from_peer: bool,
}

#[test]
fn durable_hardening_sim_checkpoint_survives_crash_restart_deterministically() {
    let first = checkpoint_survives_crash_restart(55);
    let second = checkpoint_survives_crash_restart(55);

    assert_eq!(first, second, "seed {} replay diverged", first.seed);
    assert_eq!(first.recovered_records, 4);
    assert_eq!(first.stale_fenced, 0);
}

#[test]
fn durable_hardening_sim_torn_write_and_corruption_are_detected_not_served() {
    let first = detect_torn_write_and_corruption(5_501);
    let second = detect_torn_write_and_corruption(5_501);

    assert_eq!(first, second, "seed {} replay diverged", first.seed);
    assert!(first.checksum_corruption_detected, "{first:?}");
    assert!(first.torn_future_format_detected, "{first:?}");
    assert!(first.corrupt_record_not_served, "{first:?}");
    assert!(first.repaired_from_peer, "{first:?}");
}

#[test]
fn durable_hardening_sim_reshard_with_checkpoint_loses_no_committed_write() {
    for seed in [55, 551, 5_501] {
        let first = run_checkpoint_rescale(seed);
        let second = run_checkpoint_rescale(seed);

        assert_eq!(first, second, "seed {seed} replay diverged");
        assert!(first.passed(), "{first:?}");
        assert!(first.committed_after >= first.committed_before);
    }
}

fn checkpoint_survives_crash_restart(seed: u64) -> CheckpointRestartReport {
    let partition = PartitionId::new((seed % 17) as u32 + 1);
    let epoch = ClusterEpoch::new(55);
    let mut store = InMemoryReplicatedValueStore::default();
    for version in 1..=4 {
        store
            .upsert(
                format!("cache:{seed}:{version}"),
                ReplicatedValueRecord::value(
                    partition,
                    version,
                    epoch,
                    format!("sealed:{seed}:{version}").into_bytes(),
                ),
            )
            .expect("store accepts committed write");
    }

    let barrier = WriteWatermark::new(partition, 4, epoch);
    let mut coordinator = CheckpointCoordinator::new();
    let checkpoint = coordinator
        .coordinate(
            format!("checkpoint-{seed}"),
            epoch,
            [barrier],
            [NodeCheckpointManifest::new(
                "node-a",
                vec![DurabilitySnapshotManifest::new(
                    "default",
                    barrier,
                    Duration::from_millis(seed % 100),
                    Duration::from_secs(1),
                )],
            )],
            Duration::from_secs(1),
        )
        .expect("checkpoint covers committed writes");

    let reopened = InMemoryReplicatedValueStore::reopen_from_snapshot(u64::MAX, store.snapshot());
    let report = recover_cluster_checkpoint(
        &checkpoint,
        &reopened,
        &policy(),
        &"eu".into(),
        epoch,
        &RecoveryPolicy::full_recovery_only(),
        [
            RecoveryNamespace::new("default", placement(), replication_map())
                .with_key_prefix(format!("cache:{seed}:")),
        ],
    )
    .expect("checkpoint recovery");

    CheckpointRestartReport {
        seed,
        checkpoint_id: checkpoint.checkpoint_id,
        recovered_records: report.recovered_record_total,
        stale_fenced: report.stale_fenced_total,
    }
}

fn detect_torn_write_and_corruption(seed: u64) -> CorruptionReport {
    let partition = PartitionId::new((seed % 13) as u32 + 1);
    let epoch = ClusterEpoch::new(55);
    let record = ReplicatedValueRecord::value(partition, 1, epoch, seed.to_le_bytes().to_vec());
    let sealed = ChecksummedReplicatedValueRecord::seal(record.clone());
    let corrupt = ChecksummedReplicatedValueRecord::from_parts(
        REPLICATED_VALUE_RECORD_CHECKSUM_FORMAT_VERSION,
        record.clone(),
        sealed.checksum ^ 0x55,
    );
    let torn_future_format = ChecksummedReplicatedValueRecord::from_parts(
        REPLICATED_VALUE_RECORD_CHECKSUM_FORMAT_VERSION + 1,
        record.clone(),
        record.artifact_checksum(),
    );

    let checksum_corruption_detected = corrupt.verify().is_err();
    let torn_future_format_detected = torn_future_format.verify().is_err();

    let mut corrupt_map = BTreeMap::new();
    corrupt_map.insert("cache:corrupt".to_owned(), corrupt);
    let corrupt_record_not_served = Scrubber::verified_get(&corrupt_map, "cache:corrupt").is_err();

    let mut primary = corrupt_map;
    let mut peer = BTreeMap::new();
    peer.insert("cache:corrupt".to_owned(), sealed);
    let scrub_report = Scrubber::default().scrub_replicated_values(&mut primary, &[peer]);
    let repaired_from_peer = scrub_report.repaired == 1
        && primary
            .get("cache:corrupt")
            .and_then(|candidate| candidate.verified_record("cache:corrupt").ok())
            .is_some();

    CorruptionReport {
        seed,
        checksum_corruption_detected,
        torn_future_format_detected,
        corrupt_record_not_served,
        repaired_from_peer,
    }
}

fn policy() -> PersistencePolicy {
    PersistencePolicy::try_new([NamespacePersistenceRule::persistent("default").unwrap()]).unwrap()
}

fn placement() -> PersistenceRegionPlacement {
    PersistenceRegionPlacement::home_region_only("eu")
}

fn replication_map() -> EffectiveReplicationMap {
    EffectiveReplicationMap::new(Replicas::new("node-a", Vec::new()))
}
