use std::collections::BTreeMap;

use hydracache::{
    anti_entropy_diff, ActiveActiveAcknowledgement, ActiveActiveConfig, ActiveActiveState,
    ClusterEpoch, CrdtMetadataGcGate, GeoBatch, HigherVersionWins, IdempotencyKey, PartitionDigest,
    PartitionId, RegionId, RegionLink, VersionSummary,
};

fn write(key: &str, value: &[u8], wall: u64) -> hydracache::GeoWrite {
    let config = ActiveActiveConfig::active_active(
        "home",
        ActiveActiveAcknowledgement::BoundedStalenessAccepted,
    )
    .expect("ack");
    let mut state = ActiveActiveState::new(
        config,
        "eu",
        "node-eu",
        64,
        ClusterEpoch::new(3),
        vec![RegionId::from("us")],
    );
    state
        .accept_local_write(key, value.to_vec(), wall)
        .expect("write");
    state.drain_pending().pop().expect("pending")
}

#[test]
fn region_link_batch_is_compressed_and_deduped() {
    let write = write("user:42", b"ada", 10);
    let batch = GeoBatch::new(
        "us",
        vec![write],
        vec![IdempotencyKey::from("idem:user:42:1")],
    )
    .expect("batch");
    assert!(batch.is_compressed());

    let mut link = RegionLink::new("us", 1, 2, 8);
    let mut records = BTreeMap::new();
    let first = link.apply_batch(&batch, &mut records, &HigherVersionWins);
    let replay = link.apply_batch(&batch, &mut records, &HigherVersionWins);

    assert_eq!(first.applied, 1);
    assert_eq!(replay.applied, 0);
    assert_eq!(replay.deduped, 1);
    assert_eq!(records.len(), 1);
}

#[test]
fn region_link_wan_backpressure_bounds_inflight() {
    let write = write("user:42", b"ada", 10);
    let batch = GeoBatch::new("us", vec![write], vec![IdempotencyKey::from("idem")]).unwrap();
    let mut link = RegionLink::new("us", 1, 2, 4);

    assert!(link.try_send(&batch));
    assert!(link.try_send(&batch));
    assert!(!link.try_send(&batch));
    assert_eq!(link.window().in_flight(), 2);
    assert_eq!(link.lag(), 1);

    link.on_ack(false);
    assert_eq!(link.window().max_in_flight(), 1);
    assert!(link.lag() >= 1);
}

#[test]
fn region_link_anti_entropy_ships_only_the_diff() {
    let mut local = VersionSummary::new();
    local.insert("a", 3, ClusterEpoch::new(1));
    local.insert("b", 4, ClusterEpoch::new(1));
    local.insert("c", 5, ClusterEpoch::new(2));
    let mut remote = VersionSummary::new();
    remote.insert("a", 3, ClusterEpoch::new(1));
    remote.insert("b", 2, ClusterEpoch::new(1));

    let diff = anti_entropy_diff(
        &PartitionDigest::new(PartitionId::new(7), local),
        &PartitionDigest::new(PartitionId::new(7), remote),
    );

    assert_eq!(diff, vec!["b".to_owned(), "c".to_owned()]);
}

#[test]
fn region_link_crdt_metadata_gc_gated_on_all_region_confirmation() {
    let mut gate = CrdtMetadataGcGate::new(vec![
        RegionId::from("eu"),
        RegionId::from("us"),
        RegionId::from("ap"),
    ]);

    gate.confirm(RegionId::from("eu"));
    gate.confirm(RegionId::from("us"));
    assert!(!gate.can_collect());

    gate.confirm(RegionId::from("ap"));
    assert!(gate.can_collect());
}

#[test]
#[ignore = "chaos gate: run with -- --ignored for cross-region partition-heal convergence"]
fn region_link_cross_region_converges_after_partition_heal() {
    let write = write("user:42", b"ada", 10);
    let batch = GeoBatch::new("us", vec![write], vec![IdempotencyKey::from("idem")]).unwrap();
    let mut link = RegionLink::new("us", 1, 1, 2);
    let mut records = BTreeMap::new();

    assert!(link.try_send(&batch));
    link.on_ack(true);
    let report = link.apply_batch(&batch, &mut records, &HigherVersionWins);

    assert_eq!(report.applied, 1);
    assert_eq!(records["user:42"].version, 1);
}
