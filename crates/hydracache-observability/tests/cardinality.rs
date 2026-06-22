use hydracache::{
    cluster_grid_metric_descriptors, ClusterGridDiagnostics, ClusterNodeId, PartitionId,
};

#[test]
fn cardinality_exported_metrics_have_only_bounded_labels() {
    let forbidden = ["partition_id", "key", "replica_index"];

    for metric in cluster_grid_metric_descriptors() {
        for label in metric.labels {
            assert!(
                !forbidden.contains(label),
                "metric {} exports forbidden high-cardinality label {label}",
                metric.name
            );
        }
    }
}

#[test]
fn cardinality_under_replication_is_a_single_gauge_not_per_key() {
    let under_replication = cluster_grid_metric_descriptors()
        .iter()
        .filter(|metric| metric.name == "hydracache_under_replicated_keys")
        .collect::<Vec<_>>();

    assert_eq!(under_replication.len(), 1);
    assert!(under_replication[0].labels.is_empty());
}

#[test]
fn cardinality_per_partition_detail_only_in_snapshot() {
    let mut diagnostics = ClusterGridDiagnostics::default();
    diagnostics.partition_replica_versions.insert(
        PartitionId::new(7),
        vec![(ClusterNodeId::from("member-a"), 42)],
    );

    assert!(diagnostics
        .partition_replica_versions
        .contains_key(&PartitionId::new(7)));
    assert!(cluster_grid_metric_descriptors()
        .iter()
        .all(|metric| !metric.labels.contains(&"partition_id")));
}
