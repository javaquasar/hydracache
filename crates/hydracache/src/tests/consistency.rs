use std::fmt;
use std::time::Duration;

use crate::{
    CacheOptions, ConsistencyMode, ConsistencyOutcome, ConsistencyToken, DegradeReason, HydraCache,
};

#[derive(Debug)]
struct LoadError;

impl fmt::Display for LoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("load failed")
    }
}

impl std::error::Error for LoadError {}

#[test]
fn default_mode_is_eventual() {
    assert_eq!(ConsistencyMode::default(), ConsistencyMode::Eventual);
}

#[test]
fn token_carries_generation_namespace_origin() {
    let token = ConsistencyToken::new(7, "tenant-a", "node-a");

    assert_eq!(token.generation(), 7);
    assert_eq!(token.namespace(), "tenant-a");
    assert_eq!(token.origin_node(), "node-a");
}

#[test]
fn degraded_outcome_shape() {
    let outcome = ConsistencyOutcome::Degraded {
        value: 42_u64,
        reason: DegradeReason::Timeout,
    };

    assert!(matches!(
        outcome,
        ConsistencyOutcome::Degraded {
            value: 42,
            reason: DegradeReason::Timeout
        }
    ));
}

#[tokio::test]
async fn local_generation_prevents_stale_overwrite() {
    let cache = HydraCache::local().build();
    cache
        .put(
            "user:1",
            "stale".to_owned(),
            CacheOptions::new().tag("user:1"),
        )
        .await
        .unwrap();

    let token = cache
        .invalidate_after_write("user:1")
        .namespace("profiles")
        .consistency(ConsistencyMode::LocalReadYourWrites)
        .await
        .unwrap();
    let outcome = cache
        .get_with_consistency(
            "user:1",
            &token,
            ConsistencyMode::LocalReadYourWrites,
            CacheOptions::new().tag("user:1"),
            || async { Ok::<_, LoadError>("fresh".to_owned()) },
        )
        .await
        .unwrap();

    assert_eq!(outcome, ConsistencyOutcome::Fresh("fresh".to_owned()));
    assert_eq!(
        cache.get::<String>("user:1").await.unwrap(),
        Some("fresh".to_owned())
    );
}

#[tokio::test]
async fn strict_mode_does_not_return_pre_invalidation_value() {
    let cache = HydraCache::local().build();
    cache
        .put("user:1", "stale".to_owned(), CacheOptions::new())
        .await
        .unwrap();
    let future_token = ConsistencyToken::new(99, "local", "other-node");

    let outcome = cache
        .get_with_consistency(
            "user:1",
            &future_token,
            ConsistencyMode::FailClosed,
            CacheOptions::new(),
            || async { Ok::<_, LoadError>("fresh".to_owned()) },
        )
        .await
        .unwrap();

    assert_eq!(
        outcome,
        ConsistencyOutcome::FailedClosed {
            reason: DegradeReason::Timeout
        }
    );
    assert_eq!(
        cache.get::<String>("user:1").await.unwrap(),
        Some("stale".to_owned())
    );
}

#[tokio::test]
async fn strict_mode_timeout_returns_explicit_error() {
    let cache = HydraCache::local().build();
    let future_token = ConsistencyToken::new(99, "local", "other-node");

    let outcome = cache
        .get_with_consistency(
            "missing",
            &future_token,
            ConsistencyMode::ClusterReadYourWrites {
                timeout: Duration::from_millis(1),
            },
            CacheOptions::new(),
            || async { Ok::<_, LoadError>("fresh".to_owned()) },
        )
        .await
        .unwrap();

    assert_eq!(outcome, ConsistencyOutcome::TimedOut);
    assert_eq!(cache.stats().consistency_wait_timeouts, 1);
}

#[tokio::test]
async fn degraded_mode_returns_stale_value_with_reason() {
    let cache = HydraCache::local().build();
    cache
        .put("user:1", "stale".to_owned(), CacheOptions::new())
        .await
        .unwrap();
    let future_token = ConsistencyToken::new(99, "local", "other-node");

    let outcome = cache
        .get_with_consistency(
            "user:1",
            &future_token,
            ConsistencyMode::DegradedOk,
            CacheOptions::new(),
            || async { Ok::<_, LoadError>("fresh".to_owned()) },
        )
        .await
        .unwrap();

    assert_eq!(
        outcome,
        ConsistencyOutcome::Degraded {
            value: "stale".to_owned(),
            reason: DegradeReason::Timeout,
        }
    );
    assert_eq!(cache.stats().consistency_degraded_reads, 1);
}

#[tokio::test]
async fn quorum_and_leader_fail_closed_until_cluster_supports_them() {
    let cache = HydraCache::local().build();
    let token = ConsistencyToken::new(1, "local", "other-node");

    let quorum = cache
        .get_with_consistency(
            "user:1",
            &token,
            ConsistencyMode::Quorum {
                timeout: Duration::from_millis(1),
            },
            CacheOptions::new(),
            || async { Ok::<_, LoadError>("fresh".to_owned()) },
        )
        .await
        .unwrap();
    let leader = cache
        .get_with_consistency(
            "user:1",
            &token,
            ConsistencyMode::Leader,
            CacheOptions::new(),
            || async { Ok::<_, LoadError>("fresh".to_owned()) },
        )
        .await
        .unwrap();

    assert_eq!(
        quorum,
        ConsistencyOutcome::FailedClosed {
            reason: DegradeReason::UnsupportedMode("quorum")
        }
    );
    assert_eq!(
        leader,
        ConsistencyOutcome::FailedClosed {
            reason: DegradeReason::UnsupportedMode("leader")
        }
    );
    assert_eq!(cache.stats().consistency_fail_closed, 2);
}

#[tokio::test]
async fn wait_success_timeout_degraded_counters_move() {
    let cache = HydraCache::local().build();
    let token = cache
        .invalidate_after_write("users")
        .consistency(ConsistencyMode::LocalReadYourWrites)
        .await
        .unwrap();

    let outcome = cache
        .get_with_consistency(
            "users:list",
            &token,
            ConsistencyMode::LocalReadYourWrites,
            CacheOptions::new().tag("users"),
            || async { Ok::<_, LoadError>(vec![1_u64]) },
        )
        .await
        .unwrap();

    assert_eq!(outcome, ConsistencyOutcome::Fresh(vec![1]));
    assert_eq!(cache.stats().consistency_wait_successes, 1);
}
