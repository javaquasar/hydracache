use hydracache::testing::StagingClusterHarness;
use hydracache::ClusterHealthState;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cluster_staging_gate_propagates_invalidation_both_directions() {
    let mut harness = StagingClusterHarness::builder()
        .members(2)
        .clients(2)
        .invalidations(4)
        .build()
        .await;

    harness.drive_bidirectional_invalidations(4).await;
    let outcome = harness.outcome();

    assert_eq!(outcome.report.published, outcome.report.received);
    assert_eq!(outcome.report.received, outcome.report.applied);
    assert_eq!(outcome.report.lagged, 0);
    assert_eq!(outcome.report.invalidation_ops, 4);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cluster_staging_gate_rejects_stale_generation_publish() {
    let mut harness = StagingClusterHarness::builder().build().await;

    harness.drive_leave_rejoin_with_newer_generation().await;
    harness.attempt_stale_generation_publish().await;
    let outcome = harness.outcome();

    assert_eq!(outcome.report.stale_generation_rejected, 2);
    assert_eq!(outcome.report.applied, 0);
    assert_eq!(outcome.report.publish_failures, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cluster_staging_gate_peer_fetch_auth_and_wire_checks() {
    let mut harness = StagingClusterHarness::builder().build().await;

    harness.drive_peer_fetch_auth_matrix().await;
    harness.drive_wire_version_matrix().await;
    let outcome = harness.outcome();

    assert_eq!(outcome.report.peer_fetch_auth_failures, 1);
    assert_eq!(outcome.report.wire_version_rejections, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cluster_staging_gate_owner_load_hydrates_near_cache() {
    let mut harness = StagingClusterHarness::builder().build().await;

    harness.drive_owner_remote_hot_cache_matrix().await;
    let outcome = harness.outcome();

    assert!(outcome.report.owner_load_success >= 1);
    assert!(outcome.report.remote_fetch_success >= 1);
    assert!(outcome.report.hot_cache_hits >= 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cluster_staging_gate_health_summary_is_clean() {
    let outcome = StagingClusterHarness::builder()
        .members(2)
        .clients(2)
        .invalidations(4)
        .build()
        .await
        .run_full_gate()
        .await;

    assert_eq!(outcome.health, ClusterHealthState::Healthy);
    assert!(outcome.report.totals_match_requests());
    assert!(outcome.report.has_clean_invalidation_health());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn load_report_totals_equal_requests() {
    let outcome = StagingClusterHarness::builder()
        .invalidations(3)
        .build()
        .await
        .run_full_gate()
        .await;

    assert!(outcome.report.totals_match_requests());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn load_report_health_counters_are_zero_in_gate() {
    let outcome = StagingClusterHarness::builder()
        .build()
        .await
        .run_full_gate()
        .await;

    assert!(outcome.report.has_clean_invalidation_health());
}

#[test]
fn load_report_json_shape_is_stable() {
    let report = hydracache::ClusterLoadReport {
        nodes: 4,
        requests: 12,
        read_ops: 8,
        invalidation_ops: 4,
        published: 12,
        received: 12,
        applied: 12,
        lagged: 0,
        decode_errors: 0,
        publish_failures: 0,
        receiver_closed: 0,
        stale_generation_rejected: 2,
        peer_fetch_auth_failures: 1,
        wire_version_rejections: 1,
        owner_load_success: 1,
        remote_fetch_success: 1,
        hot_cache_hits: 1,
        elapsed_ms: 0,
    };

    let value = serde_json::to_value(report).unwrap();

    assert_eq!(value["nodes"], 4);
    assert_eq!(value["requests"], 12);
    assert_eq!(value["read_ops"], 8);
    assert_eq!(value["invalidation_ops"], 4);
    assert_eq!(value["published"], 12);
    assert_eq!(value["received"], 12);
    assert_eq!(value["applied"], 12);
    assert_eq!(value["peer_fetch_auth_failures"], 1);
    assert_eq!(value["wire_version_rejections"], 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn gossip_reset_increments_reset_count_and_sets_tombstone_age() {
    let mut harness = StagingClusterHarness::builder().build().await;

    harness.simulate_gossip_reset(25);
    let outcome = harness.outcome();

    assert_eq!(outcome.staging_health.gossip_reset_count, 1);
    assert_eq!(outcome.staging_health.tombstone_age_ms, 25);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn recent_gossip_reset_downgrades_health_to_degraded() {
    let mut harness = StagingClusterHarness::builder().build().await;

    harness.simulate_gossip_reset(25);
    let outcome = harness.outcome();

    assert!(matches!(
        outcome.health,
        ClusterHealthState::Degraded { .. }
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "soak: wall-clock thresholds, run manually"]
async fn cluster_staging_gate_soak_under_sustained_load() {
    let requests: usize = std::env::var("HC_SOAK_REQUESTS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(10_000);
    let concurrency: usize = std::env::var("HC_SOAK_CONCURRENCY")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(32);
    let invalidations = requests.saturating_div(concurrency.max(1)).max(1);

    let outcome = StagingClusterHarness::builder()
        .members(2)
        .clients(2)
        .invalidations(invalidations)
        .build()
        .await
        .run_full_gate()
        .await;

    assert!(outcome.report.has_clean_invalidation_health());
    assert_eq!(outcome.health, ClusterHealthState::Healthy);
}
