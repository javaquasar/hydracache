use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use hydracache::{
    cluster_grid_metric_descriptors, rebuild_expired_sessionless, recover_session_after_failover,
    validate_session_lifecycle, ClusterEpoch, ClusterGridCounters, HybridLogicalClock, PartitionId,
    PartitionKey, SessionFailoverAction, SessionId, SessionRequest, SessionToken,
    SessionTokenError, SessionTtl, SessionWatermark, VersionStamp,
};
use hydracache_observability::{session_alert_metric_names, session_metric_names, SessionStats};

fn stamp(version: u64) -> VersionStamp {
    VersionStamp::new(
        version,
        ClusterEpoch::new(1),
        HybridLogicalClock::new(version, 0),
    )
}

fn key(partition: u32, region: &str) -> PartitionKey {
    PartitionKey::new(PartitionId::new(partition), region)
}

#[test]
fn session_observability_expired_token_is_rejected_and_rebuilds() {
    let secret = b"session-secret";
    let session = SessionId::new("session-a");
    let token = SessionToken::issue(session.clone(), SessionWatermark::new(8), 1, 1_000, secret);
    let error = validate_session_lifecycle(
        &token,
        &session,
        secret,
        1,
        SessionTtl::from_millis(100),
        1_101,
    )
    .expect_err("expired token must be rejected");

    assert_eq!(error, SessionTokenError::Expired);
    assert_eq!(
        rebuild_expired_sessionless(error).expect("expired token downgrades safely"),
        SessionRequest::Sessionless
    );
}

#[test]
fn session_observability_failover_repair_preserves_watermark_guarantees() {
    let mut watermark = SessionWatermark::new(8);
    watermark.observe(key(42, "region-a"), stamp(5));

    let recovery = recover_session_after_failover(&watermark, "region-b", true);

    assert_eq!(recovery.action, SessionFailoverAction::RepairToWatermark);
    assert_eq!(recovery.watermark_entries, 1);
    assert!(recovery.guarantees_preserved);
}

#[test]
#[ignore = "networked failover chaos scenario for promoted region repair"]
fn session_observability_guarantees_survive_region_failover() {}

#[test]
fn session_observability_session_metrics_honor_cardinality_rule() {
    let forbidden = ["session", "session_id", "partition_id", "key"];
    let session_metrics = session_metric_names()
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let descriptors = cluster_grid_metric_descriptors()
        .iter()
        .filter(|metric| session_metrics.contains(metric.name))
        .collect::<Vec<_>>();

    assert_eq!(descriptors.len(), session_metrics.len());
    for descriptor in descriptors {
        for label in descriptor.labels {
            assert!(
                !forbidden.contains(label),
                "metric {} exports forbidden session label {label}",
                descriptor.name
            );
        }
    }
}

#[test]
fn session_observability_session_alert_rules_reference_existing_metrics() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let alerts =
        fs::read_to_string(root.join("docs/cluster/dashboards/sessions/prometheus-alerts.yml"))
            .unwrap();
    let registered = cluster_grid_metric_descriptors()
        .iter()
        .map(|metric| metric.name)
        .collect::<BTreeSet<_>>();
    let expected = session_alert_metric_names()
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();

    let referenced = alerts
        .lines()
        .filter_map(|line| line.trim().strip_prefix("expr:"))
        .filter_map(|expr| {
            expr.trim()
                .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
                .find(|token| token.starts_with("hydracache_"))
        })
        .collect::<BTreeSet<_>>();

    assert_eq!(referenced, expected);
    for metric in referenced {
        assert!(
            registered.contains(metric),
            "session alert references unregistered metric {metric}"
        );
    }
}

#[test]
fn session_observability_session_stats_are_aggregate_only() {
    let mut counters = ClusterGridCounters::default();
    counters.session_active_sessions = 12;
    counters.session_watermark_entries = 48;
    counters.session_watermark_entries_p99 = 7;
    counters.session_worst_staleness_versions = 3;
    counters.session_guarantee_unmet_total = 1;
    counters.session_ryw_escalations_total = 2;
    counters.causal_writes_deferred_total = 4;

    let stats = SessionStats::from_grid_counters(counters, 100);

    assert_eq!(stats.active_sessions, 12);
    assert_eq!(stats.p99_watermark_entries, 7);
    assert_eq!(stats.worst_session_staleness.max_version_lag, 3);
    assert_eq!(stats.guarantee_unmet_rate, 0.01);
    assert!(!stats.is_healthy());
    let json = serde_json::to_value(stats).unwrap();
    assert!(json.get("active_sessions").is_some());
    assert!(json.get("session_id").is_none());
}
