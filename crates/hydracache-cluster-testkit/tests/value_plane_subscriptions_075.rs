use hydracache_cluster_testkit::value_plane_subscriptions_075::{
    FilteredEvent, FilteredEventKind, FilteredSubscriptionHub, SubscriptionDelivery,
    SubscriptionError, SubscriptionFilter,
};
use std::collections::BTreeSet;

fn filter(prefix: &[u8], kinds: &[FilteredEventKind]) -> SubscriptionFilter {
    SubscriptionFilter {
        key_prefix: prefix.to_vec(),
        kinds: kinds.iter().copied().collect::<BTreeSet<_>>(),
    }
}

fn event(partition: u32, generation: u64, watermark: u64, key: &[u8]) -> FilteredEvent {
    FilteredEvent {
        partition,
        generation,
        watermark,
        key: key.to_vec(),
        kind: FilteredEventKind::Added,
    }
}

#[test]
fn subscribers_filter_independently_and_unsubscribe_releases_backlog() {
    let mut hub = FilteredSubscriptionHub::new(2, 4).unwrap();
    hub.subscribe(1, filter(b"a", &[]), [(0, 1, 0), (1, 1, 0)])
        .unwrap();
    hub.subscribe(2, filter(b"b", &[FilteredEventKind::Added]), [(0, 1, 0)])
        .unwrap();
    assert_eq!(hub.publish(event(0, 1, 1, b"alpha")).unwrap(), 1);
    assert_eq!(hub.publish(event(0, 1, 2, b"beta")).unwrap(), 1);
    assert_eq!(hub.drain(1).unwrap().len(), 1);
    assert_eq!(hub.unsubscribe(2).unwrap(), 1);
    assert_eq!(hub.drain(2), Err(SubscriptionError::UnknownSubscription));
    assert!(!hub.global_order_guaranteed());
}

#[test]
fn migration_and_overflow_emit_one_gap_until_repair() {
    let mut hub = FilteredSubscriptionHub::new(1, 1).unwrap();
    hub.subscribe(1, filter(b"", &[]), [(0, 1, 0)]).unwrap();
    hub.publish(event(0, 1, 1, b"a")).unwrap();
    hub.publish(event(0, 2, 4, b"b")).unwrap();
    let deliveries = hub.drain(1).unwrap();
    assert_eq!(deliveries.len(), 1);
    assert!(matches!(deliveries[0], SubscriptionDelivery::Gap { .. }));
    assert_eq!(
        hub.repair(1, 0, 1, 4),
        Err(SubscriptionError::InvalidRepair)
    );
    hub.repair(1, 0, 2, 4).unwrap();
    assert_eq!(hub.publish(event(0, 2, 5, b"c")).unwrap(), 1);
}

#[test]
fn subscription_bounds_duplicates_and_stale_generations_fail_loud() {
    assert!(FilteredSubscriptionHub::new(0, 1).is_err());
    let mut hub = FilteredSubscriptionHub::new(1, 2).unwrap();
    hub.subscribe(1, filter(b"", &[]), [(0, 2, 0)]).unwrap();
    assert_eq!(
        hub.subscribe(1, filter(b"", &[]), [(0, 2, 0)]),
        Err(SubscriptionError::DuplicateSubscription)
    );
    assert_eq!(
        hub.publish(event(0, 1, 1, b"a")),
        Err(SubscriptionError::StaleGeneration)
    );
}
