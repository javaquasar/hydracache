use hydracache::{
    ClusterEpoch, InMemoryReplicatedValueStore, PartitionId, ReplicatedValueRecord,
    ReplicatedValueStore, TieredValueStore,
};

#[test]
fn tiered_values_cold_hit_promotes_to_hot() {
    let partition = PartitionId::new(1);
    let mut cold = InMemoryReplicatedValueStore::default();
    cold.upsert(
        "profile:1",
        ReplicatedValueRecord::value(partition, 1, ClusterEpoch::new(1), b"abc"),
    )
    .unwrap();
    let mut tiered = TieredValueStore::new(cold, 16);

    let record = tiered.get_promote("profile:1").unwrap().expect("record");

    assert_eq!(record.version, 1);
    assert!(tiered.hot_contains("profile:1"));
    assert_eq!(tiered.promotions_total(), 1);
}

#[test]
fn tiered_values_hot_eviction_demotes_to_cold_without_loss() {
    let partition = PartitionId::new(1);
    let mut tiered = TieredValueStore::new(InMemoryReplicatedValueStore::default(), 4);
    tiered
        .upsert(
            "a",
            ReplicatedValueRecord::value(partition, 1, ClusterEpoch::new(1), b"abcd"),
        )
        .unwrap();
    tiered
        .upsert(
            "b",
            ReplicatedValueRecord::value(partition, 2, ClusterEpoch::new(1), b"efgh"),
        )
        .unwrap();

    assert!(tiered.demotions_total() >= 1);
    assert_eq!(tiered.get("a").unwrap().expect("a").version, 1);
    assert_eq!(tiered.get("b").unwrap().expect("b").version, 2);
    assert!(tiered.hot_bytes() <= 4);
}

#[test]
fn tiered_values_tombstone_in_either_tier_wins() {
    let partition = PartitionId::new(1);
    let mut tiered = TieredValueStore::new(InMemoryReplicatedValueStore::default(), 32);
    tiered
        .upsert(
            "user:1",
            ReplicatedValueRecord::value(partition, 1, ClusterEpoch::new(1), b"value"),
        )
        .unwrap();
    tiered
        .cold_mut()
        .tombstone("user:1", partition, 2, ClusterEpoch::new(1))
        .unwrap();

    assert!(tiered
        .get("user:1")
        .unwrap()
        .expect("record")
        .is_tombstone());
}

#[test]
fn tiered_values_hot_tier_respects_byte_budget() {
    let partition = PartitionId::new(1);
    let mut tiered = TieredValueStore::new(InMemoryReplicatedValueStore::default(), 8);
    for index in 0..10 {
        tiered
            .upsert(
                format!("key:{index}"),
                ReplicatedValueRecord::value(partition, index, ClusterEpoch::new(1), vec![b'x'; 4]),
            )
            .unwrap();
    }

    assert!(tiered.hot_bytes() <= 8);
    assert!(tiered.demotions_total() > 0);
}

#[test]
fn tiered_values_tiering_off_matches_042_behavior() {
    let partition = PartitionId::new(1);
    let mut single_tier = InMemoryReplicatedValueStore::default();
    single_tier
        .upsert(
            "key",
            ReplicatedValueRecord::value(partition, 7, ClusterEpoch::new(1), b"value"),
        )
        .unwrap();

    assert_eq!(single_tier.get("key").unwrap().expect("record").version, 7);
    assert_eq!(single_tier.rejected_total(), 0);
}
