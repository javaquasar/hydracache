use std::sync::Arc;
use std::time::Duration;

use hydracache::{
    CacheInvalidation, CacheInvalidationBus, CacheInvalidationMessage, CacheInvalidationReceive,
    InMemoryInvalidationBus, InvalidationRing, PartitionId, ReplayResult,
};

#[tokio::test]
async fn lagged_invalidation_subscriber_fails_conservatively_without_unbounded_queue_growth() {
    const CAPACITY: usize = 4;
    const PUBLISHED: usize = 128;

    let bus = Arc::new(InMemoryInvalidationBus::new(CAPACITY));
    let mut subscriber = bus.subscribe();
    tokio::time::timeout(Duration::from_millis(250), async {
        for index in 0..PUBLISHED {
            bus.publish(CacheInvalidationMessage::new(
                "producer",
                CacheInvalidation::key(format!("key-{index}")),
            ))
            .await
            .unwrap();
        }
    })
    .await
    .expect("bounded publication must not block behind a lagging subscriber");

    let lagged = tokio::time::timeout(Duration::from_millis(250), subscriber.recv())
        .await
        .expect("lag notification must be observable");
    assert!(matches!(
        lagged,
        CacheInvalidationReceive::Lagged(count) if count >= (PUBLISHED - CAPACITY) as u64
    ));

    let mut replay = InvalidationRing::new(PartitionId::new(9), CAPACITY);
    for index in 0..PUBLISHED {
        replay.publish(CacheInvalidation::key(format!("key-{index}")));
    }
    assert_eq!(replay.metrics().invalidation_ring_depth, CAPACITY as u64);
    assert_eq!(
        replay.replay_from(0),
        ReplayResult::FellBehind {
            clear_partition: PartitionId::new(9),
        },
        "a subscriber outside retained history must clear conservatively"
    );
    assert_eq!(
        replay.metrics().invalidation_ring_overrun_total,
        (PUBLISHED - CAPACITY) as u64
    );
}
