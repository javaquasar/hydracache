use hydracache::{
    choose_hlc_tiebreak, ActiveActiveAcknowledgement, ActiveActiveConfig, ActiveActiveState,
    ClusterEpoch, ClusterNodeId, GeoWrite, HigherVersionWins, HybridLogicalClock, PartitionId,
    RegionId,
};

fn active_region(name: &str, peers: Vec<RegionId>) -> ActiveActiveState {
    let config = ActiveActiveConfig::active_active(
        "home",
        ActiveActiveAcknowledgement::BoundedStalenessAccepted,
    )
    .expect("acknowledged");
    ActiveActiveState::new(
        config,
        name,
        format!("node-{name}"),
        64,
        ClusterEpoch::new(7),
        peers,
    )
}

#[test]
fn active_active_local_write_acks_without_crossing_wan() {
    let mut eu = active_region("eu", vec![RegionId::from("home"), RegionId::from("us")]);

    let ack = eu
        .accept_local_write("user:42", b"ada".to_vec(), 100)
        .expect("local write");

    assert!(!ack.crossed_wan);
    assert_eq!(ack.watermark.epoch, ClusterEpoch::new(7));
    assert_eq!(
        ack.replication_targets,
        vec![RegionId::from("home"), RegionId::from("us")]
    );
    assert_eq!(eu.record("user:42").unwrap().version, ack.watermark.version);
}

#[test]
fn active_active_regions_converge_after_propagation() {
    let mut eu = active_region("eu", vec![RegionId::from("us")]);
    let mut us = active_region("us", vec![RegionId::from("eu")]);
    let policy = HigherVersionWins;

    eu.accept_local_write("user:42", b"eu".to_vec(), 10)
        .expect("eu write");
    us.accept_local_write("user:42", b"us".to_vec(), 20)
        .expect("us write");

    for write in eu.drain_pending() {
        us.reconcile_remote(write, &policy);
    }
    for write in us.drain_pending() {
        eu.reconcile_remote(write, &policy);
    }

    assert_eq!(eu.record("user:42"), us.record("user:42"));
    assert_eq!(eu.record("user:42").unwrap().version, 1);
    assert_eq!(eu.record("user:42").unwrap().epoch, ClusterEpoch::new(7));
}

#[test]
fn active_active_intra_region_ryow_unchanged() {
    let mut eu = active_region("eu", vec![RegionId::from("us")]);

    let ack = eu
        .accept_local_write("orders:7", b"visible".to_vec(), 5)
        .expect("write");

    assert!(eu.intra_region_read_your_writes_holds(ack.watermark, 1));
}

#[test]
fn active_active_requires_explicit_ack() {
    let refused = ActiveActiveConfig::active_active("home", ActiveActiveAcknowledgement::Missing);

    assert!(refused.is_err());

    let accepted = ActiveActiveConfig::active_active(
        "home",
        ActiveActiveAcknowledgement::BoundedStalenessAccepted,
    )
    .expect("accepted");

    assert!(accepted.active_active_ready());
}

#[test]
fn active_active_hlc_tiebreak_is_not_wall_clock_authority() {
    let left = GeoWrite {
        key: "k".to_owned(),
        partition: PartitionId::new(1),
        version: 3,
        epoch: ClusterEpoch::new(9),
        hlc: HybridLogicalClock::new(1_000, 0),
        origin_region: RegionId::from("eu"),
        origin_node: ClusterNodeId::from("node-eu"),
        value: b"left".to_vec(),
    };
    let right = GeoWrite {
        key: "k".to_owned(),
        partition: PartitionId::new(1),
        version: 4,
        epoch: ClusterEpoch::new(10),
        hlc: HybridLogicalClock::new(1, 0),
        origin_region: RegionId::from("us"),
        origin_node: ClusterNodeId::from("node-us"),
        value: b"right".to_vec(),
    };

    let winner = choose_hlc_tiebreak(&left, &right);
    assert_eq!(winner.epoch, ClusterEpoch::new(10));
    assert_eq!(winner.version, 4);

    let tie_left = GeoWrite {
        version: 4,
        epoch: ClusterEpoch::new(10),
        ..left
    };
    let tie_winner = choose_hlc_tiebreak(&tie_left, &right);
    assert_eq!(tie_winner.hlc, HybridLogicalClock::new(1_000, 0));
}

#[test]
fn active_active_home_region_only_rejects_remote_local_write() {
    let config = ActiveActiveConfig::home_region_only("home");
    let mut remote = ActiveActiveState::new(
        config,
        "eu",
        "node-eu",
        64,
        ClusterEpoch::new(1),
        vec![RegionId::from("home")],
    );

    assert!(remote
        .accept_local_write("user:42", b"x".to_vec(), 1)
        .is_err());
}

#[test]
fn active_active_record_merge_still_prefers_epoch_version_over_hlc() {
    let left = GeoWrite {
        key: "k".to_owned(),
        partition: PartitionId::new(1),
        version: 10,
        epoch: ClusterEpoch::new(5),
        hlc: HybridLogicalClock::new(1, 0),
        origin_region: RegionId::from("eu"),
        origin_node: ClusterNodeId::from("node-eu"),
        value: b"left".to_vec(),
    };
    let right = GeoWrite {
        version: 9,
        epoch: ClusterEpoch::new(5),
        hlc: HybridLogicalClock::new(9_999, 0),
        origin_region: RegionId::from("us"),
        origin_node: ClusterNodeId::from("node-us"),
        value: b"right".to_vec(),
        ..left.clone()
    };

    assert_eq!(choose_hlc_tiebreak(&left, &right).version, 10);
}
