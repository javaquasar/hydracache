use std::collections::{BTreeMap, BTreeSet};

use hydracache::{
    AckRequirement, ConsistencyLevel, ConsistencyReadiness, EffectiveReplicationMap, Replicas,
    ReplicationConfig,
};
use hydracache::{ClusterNodeId, RegionId};

fn node(id: &str) -> ClusterNodeId {
    ClusterNodeId::new(id)
}

fn region(id: &str) -> RegionId {
    RegionId::new(id)
}

fn map() -> EffectiveReplicationMap {
    EffectiveReplicationMap::new(Replicas::new(
        node("a"),
        vec![node("b"), node("c"), node("d"), node("e")],
    ))
}

fn topology() -> BTreeMap<ClusterNodeId, RegionId> {
    BTreeMap::from([
        (node("a"), region("eu")),
        (node("b"), region("eu")),
        (node("c"), region("eu")),
        (node("d"), region("us")),
        (node("e"), region("us")),
    ])
}

fn live(ids: &[&str]) -> BTreeSet<ClusterNodeId> {
    ids.iter().map(|id| node(id)).collect()
}

fn requirement(level: ConsistencyLevel) -> AckRequirement {
    level.required_acks(&map(), &topology(), &region("eu"))
}

#[test]
fn consistency_levels_required_acks_match_replica_math() {
    assert_eq!(requirement(ConsistencyLevel::One).required_total, 1);
    assert_eq!(requirement(ConsistencyLevel::Quorum).required_total, 3);
    assert_eq!(requirement(ConsistencyLevel::All).required_total, 5);
}

#[test]
fn consistency_levels_local_quorum_counts_only_local_region() {
    let requirement = requirement(ConsistencyLevel::LocalQuorum);

    assert_eq!(requirement.required_total, 2);
    assert_eq!(
        requirement.required_per_region.get(&region("eu")).copied(),
        Some(2)
    );
    assert!(requirement.is_satisfiable(&live(&["a", "b"])));
    assert!(!requirement.is_satisfiable(&live(&["a", "d", "e"])));
}

#[test]
fn consistency_levels_each_quorum_requires_every_region() {
    let requirement = requirement(ConsistencyLevel::EachQuorum);

    assert_eq!(
        requirement.required_per_region,
        BTreeMap::from([(region("eu"), 2), (region("us"), 2)])
    );
    assert!(requirement.is_satisfiable(&live(&["a", "b", "d", "e"])));

    let error = ConsistencyLevel::EachQuorum
        .validate(&map(), &topology(), &region("eu"), &live(&["a", "b", "d"]))
        .expect_err("one region short must fail loud");
    assert_eq!(error.level, ConsistencyLevel::EachQuorum);
    assert!(error.reason.contains("us needs 2, has 1"));
}

#[test]
fn consistency_levels_quorum_read_after_quorum_write_is_read_your_writes() {
    let readiness = ConsistencyReadiness::evaluate(
        ConsistencyLevel::Quorum,
        ConsistencyLevel::Quorum,
        &map(),
        &topology(),
        &region("eu"),
    );

    assert!(readiness.read_your_writes_overlap);

    let weak = ConsistencyReadiness::evaluate(
        ConsistencyLevel::One,
        ConsistencyLevel::One,
        &map(),
        &topology(),
        &region("eu"),
    );
    assert!(!weak.read_your_writes_overlap);
}

#[test]
fn consistency_levels_unsatisfiable_level_fails_not_degrades() {
    let error = ConsistencyLevel::All
        .validate(
            &map(),
            &topology(),
            &region("eu"),
            &live(&["a", "b", "c", "d"]),
        )
        .expect_err("All cannot silently degrade to Quorum");

    assert_eq!(error.level, ConsistencyLevel::All);
    assert!(error.reason.contains("requires 5 live acknowledgements"));
}

#[test]
fn consistency_levels_deployment_default_is_preserved_as_level() {
    let config = ReplicationConfig {
        replication_factor: 3,
        read_quorum: 2,
        write_quorum: 3,
        sync_backups: 0,
        async_backups: 0,
        max_replicated_entry_bytes: 0,
        replicate_values: false,
    };

    assert_eq!(
        ConsistencyLevel::default_read(config),
        ConsistencyLevel::Quorum
    );
    assert_eq!(
        ConsistencyLevel::default_write(config),
        ConsistencyLevel::All
    );
}
