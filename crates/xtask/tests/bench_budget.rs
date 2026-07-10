use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use xtask::bench_budget::{
    check_budget, load_measurements, parse_budget, BenchMeasurement, BenchMeasurements,
};

#[test]
fn within_budget_passes() {
    let rules = parse_budget(
        r#"
        [hot_path.hit]
        max_regression_pct = 5
        "#,
    )
    .unwrap();
    let baseline = measurements([("hot_path/hit", 100.0)]);
    let current = measurements([("hot_path/hit", 104.0)]);

    let report = check_budget(&rules, &baseline, &current);

    assert!(report.passed());
}

#[test]
fn over_pct_budget_fails() {
    let rules = parse_budget(
        r#"
        [hot_path.hit]
        max_regression_pct = 5
        "#,
    )
    .unwrap();
    let baseline = measurements([("hot_path/hit", 100.0)]);
    let current = measurements([("hot_path/hit", 106.0)]);

    let report = check_budget(&rules, &baseline, &current);

    assert_eq!(report.failures.len(), 1);
    assert!(report.failures[0].message.contains("exceeds baseline"));
}

#[test]
fn missing_baseline_is_explicit_error() {
    let rules = parse_budget(
        r#"
        [hot_path.hit]
        max_regression_pct = 5
        "#,
    )
    .unwrap();
    let baseline = measurements([]);
    let current = measurements([("hot_path/hit", 100.0)]);

    let report = check_budget(&rules, &baseline, &current);

    assert_eq!(report.failures.len(), 1);
    assert_eq!(report.failures[0].id, "hot_path/hit");
    assert_eq!(report.failures[0].message, "missing baseline measurement");
}

#[test]
fn amplification_ratio_computed() {
    let rules = parse_budget(
        r#"
        [outbox_write.write_with_vs_without]
        max_amplification_x = 2.5
        numerator = "outbox_write/write_with_outbox"
        denominator = "outbox_write/write_without_outbox"
        "#,
    )
    .unwrap();
    let baseline = measurements([]);
    let current = measurements([
        ("outbox_write/write_without_outbox", 100.0),
        ("outbox_write/write_with_outbox", 260.0),
    ]);

    let report = check_budget(&rules, &baseline, &current);

    assert_eq!(report.failures.len(), 1);
    assert!(report.failures[0].message.contains("2.60x"));
}

#[test]
fn budget_toml_parser_keeps_section_ids() {
    let rules = parse_budget(
        r#"
        [hot_path.event_publish_no_subscriber]
        max_ns_absolute = 50
        "#,
    )
    .unwrap();

    assert_eq!(rules[0].id, "hot_path/event_publish_no_subscriber");
    assert_eq!(rules[0].spec.max_ns_absolute, Some(50.0));
}

#[test]
fn criterion_estimates_directory_is_loaded() {
    let root = unique_temp_dir("criterion_estimates_directory_is_loaded");
    write_criterion_estimate(&root, "hot_path/hit", "new", 123.0);

    let loaded = load_measurements(&root).unwrap();

    assert_eq!(
        loaded.measurements["hot_path/hit"],
        BenchMeasurement { mean_ns: 123.0 }
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn criterion_base_estimates_directory_is_loaded() {
    let root = unique_temp_dir("criterion_base_estimates_directory_is_loaded");
    write_criterion_estimate(&root, "hot_path/hit", "base", 123.0);

    let loaded = load_measurements(&root).unwrap();

    assert_eq!(
        loaded.measurements["hot_path/hit"],
        BenchMeasurement { mean_ns: 123.0 }
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn criterion_benchmark_json_restores_slash_containing_ids() {
    let root = unique_temp_dir("criterion_benchmark_json_restores_slash_containing_ids");
    write_criterion_estimate_in_dir(&root, "hot_path_hit", "new", 123.0);
    write_criterion_benchmark_id(&root, "hot_path_hit", "new", "hot_path/hit");

    let loaded = load_measurements(&root).unwrap();

    assert_eq!(
        loaded.measurements["hot_path/hit"],
        BenchMeasurement { mean_ns: 123.0 }
    );
    assert!(!loaded.measurements.contains_key("hot_path_hit"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn criterion_new_estimates_override_base() {
    let root = unique_temp_dir("criterion_new_estimates_override_base");
    write_criterion_estimate(&root, "hot_path/hit", "base", 200.0);
    write_criterion_estimate(&root, "hot_path/hit", "new", 123.0);

    let loaded = load_measurements(&root).unwrap();

    assert_eq!(
        loaded.measurements["hot_path/hit"],
        BenchMeasurement { mean_ns: 123.0 }
    );
    let _ = fs::remove_dir_all(root);
}

fn measurements<const N: usize>(items: [(&str, f64); N]) -> BenchMeasurements {
    BenchMeasurements {
        version: 1,
        measurements: BTreeMap::from_iter(
            items
                .into_iter()
                .map(|(id, mean_ns)| (id.to_owned(), BenchMeasurement { mean_ns })),
        ),
    }
}

fn unique_temp_dir(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("hydracache_xtask_{name}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    path
}

fn write_criterion_estimate(root: &Path, id: &str, snapshot: &str, mean_ns: f64) {
    let estimate_dir = id
        .split('/')
        .fold(root.to_path_buf(), |path, component| path.join(component))
        .join(snapshot);
    write_criterion_estimate_file(&estimate_dir, mean_ns);
}

fn write_criterion_estimate_in_dir(
    root: &Path,
    directory_name: &str,
    snapshot: &str,
    mean_ns: f64,
) {
    let estimate_dir = root.join(directory_name).join(snapshot);
    write_criterion_estimate_file(&estimate_dir, mean_ns);
}

fn write_criterion_estimate_file(estimate_dir: &Path, mean_ns: f64) {
    fs::create_dir_all(estimate_dir).unwrap();
    fs::write(
        estimate_dir.join("estimates.json"),
        format!(r#"{{"mean":{{"point_estimate":{mean_ns}}}}}"#),
    )
    .unwrap();
}

fn write_criterion_benchmark_id(root: &Path, directory_name: &str, snapshot: &str, group_id: &str) {
    let estimate_dir = root.join(directory_name).join(snapshot);
    fs::create_dir_all(&estimate_dir).unwrap();
    fs::write(
        estimate_dir.join("benchmark.json"),
        format!(
            r#"{{"group_id":"{group_id}","function_id":null,"value_str":null,"throughput":[]}}"#
        ),
    )
    .unwrap();
}
