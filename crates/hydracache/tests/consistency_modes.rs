use std::time::Duration;

use hydracache::{
    CacheOptions, ClusterGeneration, ConsistencyMode, ConsistencyOutcome, ConsistencyToken,
    DegradeReason, HydraCache, WriteBarrierToken,
};

#[derive(Debug)]
struct LoadError;

impl std::fmt::Display for LoadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("load error")
    }
}

impl std::error::Error for LoadError {}

#[tokio::test]
async fn consistency_modes_distinguish_fresh_degraded_timeout_and_fail_closed() {
    let cache = HydraCache::local().build();
    let satisfied = ConsistencyToken::new(0, "users", "local");
    let future = ConsistencyToken::new(99, "users", "remote");

    let fresh = cache
        .get_with_consistency(
            "fresh",
            &satisfied,
            ConsistencyMode::DegradedOk,
            CacheOptions::new(),
            || async { Ok::<_, LoadError>("loaded".to_owned()) },
        )
        .await
        .unwrap();
    assert_eq!(fresh, ConsistencyOutcome::Fresh("loaded".to_owned()));

    cache
        .put("cached", "stale".to_owned(), CacheOptions::new())
        .await
        .unwrap();
    let degraded = cache
        .get_with_consistency(
            "cached",
            &future,
            ConsistencyMode::DegradedOk,
            CacheOptions::new(),
            || async { Ok::<_, LoadError>("unused".to_owned()) },
        )
        .await
        .unwrap();
    assert_eq!(
        degraded,
        ConsistencyOutcome::Degraded {
            value: "stale".to_owned(),
            reason: DegradeReason::Timeout,
        }
    );

    let timed_out = cache
        .get_with_consistency(
            "missing",
            &future,
            ConsistencyMode::DegradedOk,
            CacheOptions::new(),
            || async { Ok::<_, LoadError>("unused".to_owned()) },
        )
        .await
        .unwrap();
    assert_eq!(timed_out, ConsistencyOutcome::TimedOut);

    let failed = cache
        .get_with_consistency(
            "missing",
            &future,
            ConsistencyMode::Eventual,
            CacheOptions::new(),
            || async { Ok::<_, LoadError>("unused".to_owned()) },
        )
        .await
        .unwrap();
    assert_eq!(
        failed,
        ConsistencyOutcome::FailedClosed {
            reason: DegradeReason::Timeout,
        }
    );

    let fail_closed_fresh = cache
        .get_with_consistency(
            "fail-closed-fresh",
            &satisfied,
            ConsistencyMode::FailClosed,
            CacheOptions::new(),
            || async { Ok::<_, LoadError>("fresh".to_owned()) },
        )
        .await
        .unwrap();
    assert_eq!(
        fail_closed_fresh,
        ConsistencyOutcome::Fresh("fresh".to_owned())
    );
}

#[test]
fn applied_barrier_and_unsupported_mode_are_observable() {
    let cache = HydraCache::local().build();
    cache.record_applied_write_barrier(WriteBarrierToken::new(ClusterGeneration::default(), 42));
    assert!(cache.consistency_generation() >= 42);
    assert!(
        HydraCache::<hydracache::PostcardCodec>::unsupported_consistency_mode_error("lease")
            .to_string()
            .contains("consistency mode lease is not implemented")
    );

    let token = ConsistencyToken::new(7, "orders", "member-a");
    assert_eq!(token.generation(), 7);
    assert_eq!(token.namespace(), "orders");
    assert_eq!(token.origin_node(), "member-a");
    let barrier = WriteBarrierToken::new(ClusterGeneration::new(3), 9);
    assert_eq!(barrier.generation(), ClusterGeneration::new(3));
    assert_eq!(barrier.message_id(), 9);
    let _ = Duration::ZERO;
}
