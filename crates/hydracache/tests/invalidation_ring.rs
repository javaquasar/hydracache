use hydracache::{
    CacheInvalidation, ClusterGeneration, InvalidationRing, PartitionId, ReplayResult,
};

fn ring(capacity: usize) -> InvalidationRing {
    InvalidationRing::new(PartitionId::new(9), capacity)
}

#[test]
fn invalidation_ring_subscriber_within_retention_replays_exact_range() {
    let mut ring = ring(8);
    ring.publish(CacheInvalidation::key("user:1"));
    ring.publish(CacheInvalidation::key("user:2"));
    ring.publish(CacheInvalidation::tag("users"));

    let replay = ring.replay_from(0);

    let ReplayResult::Range(events) = replay else {
        panic!("subscriber is within retention");
    };
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].sequence, 1);
    assert_eq!(events[0].invalidation.key_value(), Some("user:2"));
    assert_eq!(events[1].invalidation.tag_value(), Some("users"));
    assert_eq!(ring.metrics().invalidation_replayed_total, 2);
}

#[test]
fn invalidation_ring_subscriber_beyond_retention_falls_back_to_clear_partition() {
    let mut ring = ring(2);
    for key in ["user:1", "user:2", "user:3", "user:4"] {
        ring.publish(CacheInvalidation::key(key));
    }

    let replay = ring.replay_from(0);

    assert_eq!(
        replay,
        ReplayResult::FellBehind {
            clear_partition: PartitionId::new(9)
        }
    );
    assert_eq!(ring.metrics().invalidation_fell_behind_total, 1);
}

#[test]
fn invalidation_ring_full_ring_advances_tail_without_blocking_writes() {
    let mut ring = ring(2);

    for index in 0..10 {
        assert_eq!(
            ring.publish(CacheInvalidation::key(format!("user:{index}"))),
            index
        );
    }

    assert_eq!(ring.next_seq(), 10);
    assert_eq!(ring.head_seq(), 8);
    assert_eq!(ring.metrics().invalidation_ring_depth, 2);
    assert_eq!(ring.metrics().invalidation_ring_overrun_total, 8);
}

#[test]
fn invalidation_ring_restart_keeps_recent_window_when_durable() {
    let mut ring = ring(4);
    ring.publish_with_generation(
        CacheInvalidation::key("user:1"),
        Some(ClusterGeneration::new(3)),
    );
    ring.publish(CacheInvalidation::key("user:2"));

    let snapshot = ring.snapshot();
    let mut restored = InvalidationRing::restore(snapshot);
    let ReplayResult::Range(events) = restored.replay_from(0) else {
        panic!("restored ring should replay retained events");
    };

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].invalidation.key_value(), Some("user:2"));
}

#[test]
fn invalidation_ring_remote_client_replays_like_embedded() {
    let mut embedded = ring(4);
    let mut remote = ring(4);
    for key in ["user:1", "user:2", "user:3"] {
        embedded.publish(CacheInvalidation::key(key));
        remote.publish(CacheInvalidation::key(key));
    }

    assert_eq!(embedded.replay_from(1), remote.replay_from(1));
}
