use std::time::Duration;

use hydracache::{
    rescale_with_checkpoint, CheckpointCoordinator, ClusterEpoch, DurabilitySnapshotManifest,
    InMemoryReplicatedValueStore, MovePhase, NodeCheckpointManifest, PartitionId, PartitionMove,
    ReplicatedValueRecord, ReplicatedValueStore, RescaleCheckpointPhase, ReshardPlan,
    WriteWatermark,
};

use crate::SimRng;

/// Deterministic checkpoint/rescale simulation result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckpointRescaleReport {
    /// Seed used to replay the scenario.
    pub seed: u64,
    /// Number of records committed before the checkpoint barrier.
    pub committed_before: usize,
    /// Number of records visible on the target after resume.
    pub committed_after: usize,
    /// Whether the cluster checkpoint verified.
    pub checkpoint_valid: bool,
    /// Whether the reshard plan reached the resumed phase after snapshot/reopen.
    pub reshard_resumed: bool,
    /// Whether every committed write survived redistribution.
    pub no_committed_loss: bool,
}

impl CheckpointRescaleReport {
    /// Return whether all checkpoint/rescale invariants held.
    pub fn passed(&self) -> bool {
        self.checkpoint_valid && self.reshard_resumed && self.no_committed_loss
    }
}

/// Run a seeded stop-checkpoint-redistribute-resume model.
pub fn run_checkpoint_rescale(seed: u64) -> CheckpointRescaleReport {
    let mut rng = SimRng::from_seed(seed);
    let partition = PartitionId::new((rng.next_u64() % 31) as u32 + 1);
    let epoch = ClusterEpoch::new(55);
    let committed_before = 8 + rng.next_index(8);
    let mut source = InMemoryReplicatedValueStore::default();
    let mut target = InMemoryReplicatedValueStore::default();

    for version in 1..=committed_before as u64 {
        source
            .upsert(
                key(seed, version),
                ReplicatedValueRecord::value(
                    partition,
                    version,
                    epoch,
                    format!("sealed-{seed}-{version}").into_bytes(),
                ),
            )
            .expect("source accepts committed write");
    }

    let barrier = WriteWatermark::new(partition, committed_before as u64, epoch);
    let mut coordinator = CheckpointCoordinator::new();
    let checkpoint = coordinator
        .coordinate(
            format!("checkpoint-{seed}"),
            epoch,
            [barrier],
            [NodeCheckpointManifest::new(
                "source",
                vec![DurabilitySnapshotManifest::new(
                    "default",
                    barrier,
                    Duration::from_millis(seed % 97),
                    Duration::from_secs(1),
                )],
            )],
            Duration::from_millis(100 + seed % 101),
        )
        .expect("checkpoint should cover the barrier");
    let checkpoint_valid = checkpoint.verify().is_ok();

    let reshard = ReshardPlan::new(
        epoch,
        vec![PartitionMove::new(
            partition,
            "source",
            "target",
            source.total_bytes(),
        )],
        1,
    );
    let mut flow = rescale_with_checkpoint(checkpoint, reshard).expect("rescale checkpoint");
    flow.redistribute();

    let shadowed_version = committed_before as u64 + 1;
    let shadowed = ReplicatedValueRecord::value(
        partition,
        shadowed_version,
        epoch,
        format!("shadowed-{seed}").into_bytes(),
    );
    for target_node in flow
        .reshard
        .write_targets_for_partition(partition)
        .unwrap_or_default()
    {
        if target_node.as_str() == "source" {
            source
                .upsert(key(seed, shadowed_version), shadowed.clone())
                .expect("source shadow write");
        }
        if target_node.as_str() == "target" {
            target
                .upsert(key(seed, shadowed_version), shadowed.clone())
                .expect("target shadow write");
        }
    }

    for (record_key, record) in source.scan_all().expect("source scan") {
        target
            .upsert(record_key, record)
            .expect("target accepts backfilled write");
    }
    flow.reshard
        .record_backfill(partition, source.total_bytes());
    flow.reshard.moves[0].advance();
    flow.reshard.moves[0].advance();

    let resumed = rescale_with_checkpoint(flow.checkpoint.clone(), flow.reshard.snapshot())
        .and_then(|flow| {
            let mut snapshot = flow.snapshot();
            snapshot.phase = RescaleCheckpointPhase::Redistributing;
            hydracache::RescaleWithCheckpointPlan::resume_from(snapshot)
        })
        .expect("rescale resume");
    let reshard_resumed = resumed.phase == RescaleCheckpointPhase::Resumed
        && resumed.reshard.moves[0].phase == MovePhase::Commit;

    let committed_after = target.scan_all().expect("target scan").len();
    let no_committed_loss = (1..=committed_before as u64).all(|version| {
        target
            .get(&key(seed, version))
            .expect("target get")
            .is_some()
    }) && target
        .get(&key(seed, shadowed_version))
        .expect("target get shadowed")
        .map(|record| record.version == shadowed_version)
        .unwrap_or(false);

    CheckpointRescaleReport {
        seed,
        committed_before,
        committed_after,
        checkpoint_valid,
        reshard_resumed,
        no_committed_loss,
    }
}

fn key(seed: u64, version: u64) -> String {
    format!("sim:{seed}:key:{version}")
}
