use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::sync::Arc;

use hydracache::{cluster_grid_metric_descriptors, ClusterEpoch, RegionId, TenantMetricsSnapshot};
use hydracache_observability::{
    consumer_alert_metric_names, consumer_metric_names, AuditEnvelope, AuditEvent, AuditOutcome,
    AuditRecorder, AuditRedactionPolicy, InMemoryAuditSink, TenantStatus,
    CONSUMER_AUDIT_EVENT_SCHEMA_VERSION, TENANT_STATUS_SCHEMA_VERSION,
};

fn metrics() -> TenantMetricsSnapshot {
    let mut metrics = TenantMetricsSnapshot::default();
    metrics.tenant_bytes.insert("tenant-a".to_owned(), 42);
    metrics.tenant_entries.insert("tenant-a".to_owned(), 2);
    metrics
        .tenant_admission_rejected_total
        .insert("tenant-a".to_owned(), 1);
    metrics.tenant_namespace_bytes.insert(
        "tenant-a".to_owned(),
        BTreeMap::from([("users".to_owned(), 42)]),
    );
    metrics.tenant_namespace_entries.insert(
        "tenant-a".to_owned(),
        BTreeMap::from([("users".to_owned(), 2)]),
    );
    metrics.tenant_namespace_quota_bytes.insert(
        "tenant-a".to_owned(),
        BTreeMap::from([("users".to_owned(), 100)]),
    );
    metrics.tenant_namespace_quota_entries.insert(
        "tenant-a".to_owned(),
        BTreeMap::from([("users".to_owned(), 8)]),
    );
    metrics
        .tenant_request_count
        .insert("tenant-a".to_owned(), 3);
    metrics
        .tenant_rate_limit_per_window
        .insert("tenant-a".to_owned(), 10);
    metrics
        .tenant_fair_share_count
        .insert("tenant-a".to_owned(), 3);
    metrics
        .tenant_fair_share_per_window
        .insert("tenant-a".to_owned(), 10);
    metrics
        .tenant_subscriptions
        .insert("tenant-a".to_owned(), 1);
    metrics
        .tenant_max_subscriptions
        .insert("tenant-a".to_owned(), 4);

    metrics.tenant_bytes.insert("tenant-b".to_owned(), 900);
    metrics.tenant_namespace_bytes.insert(
        "tenant-b".to_owned(),
        BTreeMap::from([("orders".to_owned(), 900)]),
    );
    metrics
}

fn referenced_metrics(path: &Path) -> BTreeSet<String> {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter_map(|line| line.trim().strip_prefix("expr:"))
        .filter_map(|expr| {
            expr.trim()
                .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
                .find(|token| token.starts_with("hydracache_"))
                .map(ToOwned::to_owned)
        })
        .collect()
}

mod consumer_observability {
    use super::*;

    #[test]
    fn client_status_is_scoped_to_caller_tenant() {
        let status = TenantStatus::from_metrics("tenant-a", &metrics(), 1, 2);

        assert_eq!(status.schema_version, TENANT_STATUS_SCHEMA_VERSION);
        assert_eq!(status.tenant, "tenant-a");
        assert_eq!(status.namespaces.len(), 1);
        assert_eq!(status.namespaces[0].namespace, "users");
        assert_eq!(status.namespaces[0].bytes, 42);
        assert_eq!(status.namespaces[0].max_bytes, 100);
        assert_eq!(status.rate_limit.request_count, 3);
        assert_eq!(status.rate_limit.rate_limit_per_window, 10);
        assert_eq!(status.near_cache.active_subscriptions, 1);
        let json = serde_json::to_value(&status).unwrap();
        assert!(!json.to_string().contains("tenant-b"));
        assert!(!json.to_string().contains("orders"));
    }

    #[test]
    fn governance_events_are_audited_append_only() {
        let sink = Arc::new(InMemoryAuditSink::new());
        let mut recorder = AuditRecorder::new(Arc::clone(&sink));

        recorder
            .record(&AuditEvent::AuthFailure {
                tenant: Some("tenant-a".to_owned()),
                route: "/client/v1/data".to_owned(),
                request_id: Some("r1".to_owned()),
            })
            .unwrap();
        recorder
            .record(&AuditEvent::QuotaRejected {
                tenant: "tenant-a".to_owned(),
                namespace: "users".to_owned(),
                request_id: Some("r2".to_owned()),
            })
            .unwrap();
        recorder
            .record(&AuditEvent::residency_refused(
                "users",
                "user:42",
                Some(RegionId::from("eu")),
                Some(RegionId::from("us")),
                ClusterEpoch::new(7),
                AuditRedactionPolicy::hash_keys(),
            ))
            .unwrap();

        let mut snapshot = sink.events();
        assert_eq!(snapshot.len(), 3);
        snapshot.clear();
        assert_eq!(sink.events().len(), 3);
        assert_eq!(recorder.health().audit_recorded_total, 3);
    }

    #[test]
    fn mandatory_audit_sink_failure_fails_closed_for_governance_event() {
        let sink = Arc::new(InMemoryAuditSink::unavailable());
        let mut recorder = AuditRecorder::new(sink);

        let error = recorder
            .record(&AuditEvent::PolicyChanged {
                namespace: "users".to_owned(),
                policy_epoch: ClusterEpoch::new(9),
                summary: "eu-only".to_owned(),
            })
            .expect_err("mandatory event must fail closed");
        assert!(error.to_string().contains("unavailable"));
        assert_eq!(recorder.health().audit_mandatory_fail_closed_total, 1);

        let outcome = recorder
            .record(&AuditEvent::Advisory {
                name: "sampled_status".to_owned(),
                detail: "dropped safely".to_owned(),
            })
            .unwrap();
        assert_eq!(outcome, AuditOutcome::DroppedAdvisory);
        assert_eq!(recorder.health().audit_advisory_dropped_total, 1);
    }

    #[test]
    fn audit_payloads_are_redacted_and_never_include_values() {
        let event = AuditEvent::residency_refused(
            "users",
            "user:42:secret-profile",
            Some(RegionId::from("eu")),
            Some(RegionId::from("us")),
            ClusterEpoch::new(7),
            AuditRedactionPolicy::hash_keys(),
        );

        let envelope = AuditEnvelope::new(event);
        let json = serde_json::to_string(&envelope).unwrap();

        assert_eq!(envelope.schema_version, CONSUMER_AUDIT_EVENT_SCHEMA_VERSION);
        assert!(json.contains("hash"));
        assert!(!json.contains("user:42"));
        assert!(!json.contains("secret-profile"));
        assert!(!json.contains("value"));
    }

    #[test]
    fn consumer_metrics_honor_cardinality_rule() {
        let forbidden = ["key", "request_id", "session_id", "partition_id"];
        let consumer_metrics = consumer_metric_names()
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let descriptors = cluster_grid_metric_descriptors()
            .iter()
            .filter(|metric| consumer_metrics.contains(metric.name))
            .collect::<Vec<_>>();

        assert_eq!(descriptors.len(), consumer_metrics.len());
        for descriptor in descriptors {
            for label in descriptor.labels {
                assert!(
                    !forbidden.contains(label),
                    "consumer metric {} exports forbidden label {label}",
                    descriptor.name
                );
            }
        }
    }

    #[test]
    fn consumer_alert_rules_reference_existing_metrics() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let alerts = root.join("docs/cluster/dashboards/consumer/prometheus-alerts.yml");
        let registered = cluster_grid_metric_descriptors()
            .iter()
            .map(|metric| metric.name)
            .collect::<BTreeSet<_>>();
        let expected = consumer_alert_metric_names()
            .iter()
            .map(|metric| (*metric).to_owned())
            .collect::<BTreeSet<_>>();
        let referenced = referenced_metrics(&alerts);

        assert_eq!(referenced, expected);
        for metric in referenced {
            assert!(
                registered.contains(metric.as_str()),
                "consumer alert references unregistered metric {metric}"
            );
        }
    }
}
