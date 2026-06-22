use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use hydracache::{
    cluster_grid_metric_descriptors, ClusterEpoch, ClusterGridCounters, ClusterGridDiagnostics,
    ClusterNodeId, PartitionId, QuorumPosture,
};
use hydracache_observability::{ClusterStatus, MemberStatus, RepairDebtController, RepairDebtMode};

#[test]
fn operational_surface_cluster_status_is_read_only_and_complete() {
    let mut counters = ClusterGridCounters::default();
    counters.under_replicated_keys = 2;
    counters.tombstone_repair_debt = 1;
    let diagnostics = ClusterGridDiagnostics {
        counters,
        ..ClusterGridDiagnostics::default()
    };
    let members = vec![MemberStatus {
        node_id: ClusterNodeId::from("member-a"),
        reachable: true,
    }];

    let first = ClusterStatus::from_grid_diagnostics(
        ClusterEpoch::new(9),
        Some(ClusterNodeId::from("member-a")),
        members.clone(),
        diagnostics.clone(),
        QuorumPosture::Strong,
    );
    let second = ClusterStatus::from_grid_diagnostics(
        ClusterEpoch::new(9),
        Some(ClusterNodeId::from("member-a")),
        members,
        diagnostics,
        QuorumPosture::Strong,
    );

    assert_eq!(first, second);
    assert_eq!(first.committed_epoch, ClusterEpoch::new(9));
    assert_eq!(first.partitions_under_replicated, 2);
    assert_eq!(first.repair_debt, 1);
    assert!(first.still_not_distributed_transactions);
    let json = serde_json::to_value(&first).unwrap();
    assert!(json.get("members").is_some());
    assert!(json.get("quorum_posture").is_some());
    assert!(json.get("still_not_distributed_transactions").is_some());
}

#[test]
fn operational_surface_repair_debt_threshold_enters_degraded_mode() {
    let controller = RepairDebtController::new(2);
    let mut healthy_counters = ClusterGridCounters::default();
    healthy_counters.tombstone_repair_debt = 1;
    let healthy = ClusterGridDiagnostics {
        counters: healthy_counters,
        ..ClusterGridDiagnostics::default()
    };
    let mut degraded_counters = ClusterGridCounters::default();
    degraded_counters.tombstone_repair_debt = 2;
    let degraded = ClusterGridDiagnostics {
        counters: degraded_counters,
        ..ClusterGridDiagnostics::default()
    };

    assert_eq!(controller.observe(&healthy), RepairDebtMode::Healthy);
    assert_eq!(controller.observe(&degraded), RepairDebtMode::Degraded);
}

#[test]
fn operational_surface_status_honors_cardinality_rule() {
    let mut diagnostics = ClusterGridDiagnostics::default();
    diagnostics.partition_replica_versions.insert(
        PartitionId::new(3),
        vec![(ClusterNodeId::from("member-a"), 11)],
    );
    let status = ClusterStatus::from_grid_diagnostics(
        ClusterEpoch::new(1),
        None,
        Vec::new(),
        diagnostics,
        QuorumPosture::DegradedSessionRyow,
    );

    assert_eq!(status.partitions_under_replicated, 0);
    assert!(cluster_grid_metric_descriptors()
        .iter()
        .all(|metric| !metric.labels.contains(&"partition_id")));
}

#[test]
fn operational_surface_alert_rules_reference_existing_metric_names() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let alerts =
        fs::read_to_string(root.join("docs/cluster/dashboards/prometheus-alerts.yml")).unwrap();
    let registered = cluster_grid_metric_descriptors()
        .iter()
        .map(|metric| metric.name)
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

    assert!(!referenced.is_empty());
    for metric in referenced {
        assert!(
            registered.contains(metric),
            "alert references unregistered metric {metric}"
        );
    }
}
