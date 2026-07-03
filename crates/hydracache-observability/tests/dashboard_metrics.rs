use std::collections::BTreeSet;

use hydracache_observability::registered_metric_names;
use serde_json::Value;

const DASHBOARD: &str =
    include_str!("../../../docs/observability/dashboards/hydracache-overview.json");

#[test]
fn dashboard_only_references_registered_metrics() {
    let dashboard = serde_json::from_str::<Value>(DASHBOARD).expect("dashboard JSON is valid");
    let referenced = dashboard_metric_names(&dashboard);
    assert!(
        !referenced.is_empty(),
        "dashboard must reference at least one Prometheus metric"
    );

    for expected in [
        "hydracache_cache_hit_ratio",
        "hydracache_admission_rejected_total",
        "hydracache_cluster_members",
        "hydracache_under_replicated_keys",
        "hydracache_backup_age_seconds",
    ] {
        assert!(
            referenced.contains(expected),
            "dashboard missing expected panel metric {expected}"
        );
    }

    let registered = registered_metric_names();
    for metric in referenced {
        assert!(
            registered.contains(metric.as_str()),
            "dashboard references '{metric}' which the exporter does not emit"
        );
    }
}

fn dashboard_metric_names(value: &Value) -> BTreeSet<String> {
    let mut metrics = BTreeSet::new();
    collect_expr_metrics(value, &mut metrics);
    metrics
}

fn collect_expr_metrics(value: &Value, metrics: &mut BTreeSet<String>) {
    match value {
        Value::Object(object) => {
            for (key, value) in object {
                if key == "expr" {
                    if let Some(expr) = value.as_str() {
                        metrics.extend(promql_metric_names(expr));
                    }
                } else {
                    collect_expr_metrics(value, metrics);
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_expr_metrics(value, metrics);
            }
        }
        _ => {}
    }
}

fn promql_metric_names(expr: &str) -> BTreeSet<String> {
    let mut metrics = BTreeSet::new();
    let bytes = expr.as_bytes();
    let mut index = 0;
    while let Some(offset) = expr[index..].find("hydracache_") {
        let start = index + offset;
        let mut end = start;
        while end < bytes.len() && is_metric_char(bytes[end] as char) {
            end += 1;
        }
        metrics.insert(expr[start..end].to_owned());
        index = end;
    }
    metrics
}

fn is_metric_char(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_' || character == ':'
}
