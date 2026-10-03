use serde_json::Value as JsonValue;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use toml::Value as TomlValue;

const ROOT: &str = "docs/testing/performance/0.74";
const IDENTITIES: &str = "baseline-identities.toml";
const MATRIX: &str = "scenario-matrix.toml";
const STATISTICS: &str = "statistics.toml";
const REGISTRY: &str = "proposal-registry.toml";
const HOST: &str = "host-profile.toml";
const LOCAL_HARNESS: &str = "local-harness.toml";
const RELEASE: &str = "0.74";
const FROZEN_C73: &str = "16d2e98b6cc9e22d9ccf95eb26fe28bbbcf80f2b";
const C73_TREE: &str = "92336607f21a68f563e65dc0fccccd8efaa14f7b";
const TAG_OBJECT_073: &str = "38b99244ac11371a6a4149c4d95ece0cdf99520f";
const TAG_COMMIT_073: &str = "d1db9937e61295341ac95f289bace275641b1650";
const TAG_TREE_073: &str = "5bea41dd43278a9405648939666de5024d2164cc";
const ARCHIVE_COMMIT_073: &str = "570a5bcb6959ecc7f01f8c80d0fc32b719832ad9";

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let options = Options::parse(args)?;
    let mut problems = check_at_root(&options.root, options.receipt.as_deref())?;
    if options.require_ship {
        problems.push(
            "0.74 ship admission is closed while candidate identity and release qualification are incomplete"
                .to_owned(),
        );
    }
    if problems.is_empty() {
        println!("performance-contract-check 0.74: OK (local, non-promotable)");
        Ok(())
    } else {
        for problem in &problems {
            eprintln!("performance-contract-check 0.74: {problem}");
        }
        Err(format!(
            "performance-contract-check 0.74 found {} problem(s)",
            problems.len()
        )
        .into())
    }
}

pub fn check_at_root(root: &Path, receipt: Option<&Path>) -> Result<Vec<String>, Box<dyn Error>> {
    let evidence = root.join(ROOT);
    let identities = read_toml(&evidence.join(IDENTITIES))?;
    let matrix = read_toml(&evidence.join(MATRIX))?;
    let statistics = read_toml(&evidence.join(STATISTICS))?;
    let registry = read_toml(&evidence.join(REGISTRY))?;
    let host = read_toml(&evidence.join(HOST))?;
    let local_harness = read_toml(&evidence.join(LOCAL_HARNESS))?;
    let mut problems = Vec::new();
    problems.extend(check_identities(&identities));
    problems.extend(check_matrix(&matrix));
    problems.extend(check_statistics(&statistics));
    problems.extend(check_registry(&registry));
    problems.extend(check_host(&host));
    problems.extend(check_local_harness(&local_harness));
    if let Some(path) = receipt {
        let path = if path.is_absolute() {
            path.to_owned()
        } else {
            root.join(path)
        };
        let value: JsonValue = serde_json::from_slice(&fs::read(path)?)?;
        problems.extend(check_receipt(&value));
    }
    Ok(problems)
}

pub fn check_identities(value: &TomlValue) -> Vec<String> {
    let mut problems = common(value, "baseline-identities");
    expect_str(
        value,
        "contract_id",
        "baseline-identities-074-v1",
        &mut problems,
    );
    expect_str(value, "state", "predecessor-published-local", &mut problems);
    expect_bool(value, "claim_eligible", false, &mut problems);
    let predecessor = table(value, "predecessor_candidate", &mut problems);
    expect_str(predecessor, "id", "B73", &mut problems);
    expect_str(predecessor, "source_sha", FROZEN_C73, &mut problems);
    expect_str(predecessor, "tree_oid", C73_TREE, &mut problems);
    expect_bool(predecessor, "product_identity_frozen", true, &mut problems);
    expect_str(predecessor, "publication_state", "published", &mut problems);
    expect_str(
        predecessor,
        "annotated_tag_object_sha",
        TAG_OBJECT_073,
        &mut problems,
    );
    expect_str(
        predecessor,
        "annotated_tag_commit_sha",
        TAG_COMMIT_073,
        &mut problems,
    );
    expect_str(
        predecessor,
        "annotated_tag_tree_oid",
        TAG_TREE_073,
        &mut problems,
    );
    expect_bool(predecessor, "annotated_tag_present", true, &mut problems);
    expect_bool(
        predecessor,
        "product_runtime_equivalent_to_tag",
        true,
        &mut problems,
    );
    expect_i64(
        predecessor,
        "confirmation_run",
        36_839_197_349,
        &mut problems,
    );
    expect_bool(predecessor, "confirmation_passed", true, &mut problems);
    expect_str(
        predecessor,
        "release_archive_commit",
        ARCHIVE_COMMIT_073,
        &mut problems,
    );
    expect_bool(predecessor, "release_archive_verified", true, &mut problems);
    expect_bool(predecessor, "binaries_verified", true, &mut problems);
    expect_bool(
        predecessor,
        "supported_target_receipts_verified",
        true,
        &mut problems,
    );
    let closure = table(value, "predecessor_closure", &mut problems);
    for field in [
        "tag_and_product_relationship_verified",
        "release_archive_and_hashes_verified",
        "toolchain_feature_allocator_tls_and_target_receipts_verified",
        "independent_confirmation_passed",
    ] {
        expect_bool(closure, field, true, &mut problems);
    }
    let instrumented = table(value, "instrumented_baseline", &mut problems);
    expect_str(instrumented, "id", "I74", &mut problems);
    expect_str(instrumented, "root_sha", FROZEN_C73, &mut problems);
    expect_bool(
        instrumented,
        "candidate_measurement_allowed",
        false,
        &mut problems,
    );
    let redis = table(value, "redis_reference", &mut problems);
    expect_str(
        redis,
        "benchmark_version",
        "redis-benchmark 7.2.5",
        &mut problems,
    );
    expect_str(redis, "server_version", "7.2.5", &mut problems);
    expect_bool(redis, "same_box_required", true, &mut problems);
    problems
}

pub fn check_matrix(value: &TomlValue) -> Vec<String> {
    let mut problems = common(value, "scenario-matrix");
    expect_str(
        value,
        "matrix_id",
        "resp-native-throughput-074-v1",
        &mut problems,
    );
    expect_i64(value, "seed", 740074, &mut problems);
    for field in [
        "counterbalanced_order",
        "independent_processes",
        "complete_outcome_accounting",
        "final_state_digest_required",
    ] {
        expect_bool(value, field, true, &mut problems);
    }
    expect_bool(value, "candidate_measurement_allowed", false, &mut problems);
    let dimensions = table(value, "dimensions", &mut problems);
    expect_array(
        dimensions,
        "resp_dialects",
        &["resp2", "resp3"],
        &mut problems,
    );
    expect_array(
        dimensions,
        "commands",
        &["get", "set", "mget", "mset", "del", "exists"],
        &mut problems,
    );
    expect_integer_array(
        dimensions,
        "pipeline_depths",
        &[1, 10, 50, 100],
        &mut problems,
    );
    expect_integer_array(dimensions, "concurrency", &[1, 8, 32, 128], &mut problems);
    expect_integer_array(
        dimensions,
        "value_bytes",
        &[16, 256, 4096, 1_048_576],
        &mut problems,
    );
    expect_integer_array(
        dimensions,
        "hit_ratio_percent",
        &[0, 50, 95, 100],
        &mut problems,
    );
    expect_array(
        dimensions,
        "security",
        &["plaintext", "mtls"],
        &mut problems,
    );
    let surfaces = array_of_tables(value, "surfaces", &mut problems);
    let actual = surfaces
        .iter()
        .filter_map(|surface| string(surface, "id"))
        .collect::<BTreeSet<_>>();
    let required = [
        "resp-api",
        "native-api-hc1",
        "native-api-hc2",
        "client-surface-state",
        "embedded-hydracache-raw",
        "embedded-hydracache-typed",
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    if actual != required {
        problems.push(format!(
            "scenario-matrix surfaces must remain separate and complete: {actual:?}"
        ));
    }
    if surfaces
        .iter()
        .any(|surface| boolean(surface, "pooled") != Some(false))
    {
        problems.push("scenario-matrix forbids pooled surface results".to_owned());
    }
    let tiers = array_of_tables(value, "tiers", &mut problems);
    let tier_map = tiers
        .iter()
        .filter_map(|tier| string(tier, "id").map(|id| (id, *tier)))
        .collect::<BTreeMap<_, _>>();
    for id in ["local-quick", "local-attribution", "release-qualification"] {
        if !tier_map.contains_key(id) {
            problems.push(format!("scenario-matrix is missing tier {id}"));
        }
    }
    if let Some(release) = tier_map.get("release-qualification") {
        if boolean(release, "expensive") != Some(true)
            || boolean(release, "explicit_authorization_required") != Some(true)
        {
            problems.push(
                "release-qualification must stay expensive and explicitly authorized".to_owned(),
            );
        }
    }
    let guard = table(value, "trace_guard", &mut problems);
    let equal = string_array(guard, "fields_equal_between_roles");
    for required in [
        "trace_sha256",
        "payload_corpus_sha256",
        "key_corpus_sha256",
        "seed",
        "measurement_seconds",
        "concurrency",
        "pipeline_depth",
        "security",
        "persistence",
        "semantic_contract",
    ] {
        if !equal.contains(required) {
            problems.push(format!("trace guard is missing equality field {required}"));
        }
    }
    problems
}

pub fn check_statistics(value: &TomlValue) -> Vec<String> {
    let mut problems = common(value, "statistics");
    expect_str(
        value,
        "contract_id",
        "performance-statistics-074-v1",
        &mut problems,
    );
    expect_i64(value, "minimum_claim_pairs", 5, &mut problems);
    expect_bool(value, "candidate_may_amend", false, &mut problems);
    expect_bool(value, "silent_retry_allowed", false, &mut problems);
    expect_bool(value, "best_sample_selection_allowed", false, &mut problems);
    let metrics = string_array(value, "required_metrics");
    for metric in [
        "goodput_operations_per_second",
        "latency_p50_microseconds",
        "latency_p95_microseconds",
        "latency_p99_microseconds",
        "cpu_seconds_per_operation",
        "gross_allocated_bytes_per_operation",
        "copied_bytes_per_operation",
        "write_calls",
        "flush_calls",
        "store_lock_wait_nanoseconds",
        "store_lock_hold_nanoseconds",
        "rss_bytes",
        "retained_bytes",
    ] {
        if !metrics.contains(metric) {
            problems.push(format!("statistics is missing required metric {metric}"));
        }
    }
    let native = table(value, "native_non_regression", &mut problems);
    expect_f64(native, "minimum_goodput_ratio", 0.98, &mut problems);
    expect_f64(
        native,
        "maximum_cpu_per_operation_ratio",
        1.03,
        &mut problems,
    );
    expect_f64(native, "maximum_p99_ratio", 1.03, &mut problems);
    expect_bool(native, "surfaces_must_remain_separate", true, &mut problems);
    problems
}

pub fn check_registry(value: &TomlValue) -> Vec<String> {
    let mut problems = common(value, "proposal-registry");
    let product_mutation_allowed = match boolean(value, "product_mutation_allowed") {
        Some(value) => value,
        None => {
            problems.push("product_mutation_allowed must be a boolean".to_owned());
            false
        }
    };
    let work = array_of_tables(value, "work_items", &mut problems);
    let ids = work
        .iter()
        .filter_map(|item| string(item, "id"))
        .collect::<BTreeSet<_>>();
    let required = (0..=9)
        .map(|index| format!("W{index}"))
        .collect::<BTreeSet<_>>();
    if ids
        .iter()
        .map(|id| (*id).to_owned())
        .collect::<BTreeSet<_>>()
        != required
    {
        problems.push("proposal-registry must contain W0 through W9 exactly once".to_owned());
    }
    let w1_attributed = work.iter().any(|item| {
        string(item, "id") == Some("W1")
            && matches!(
                string(item, "decision"),
                Some(
                    "attributed-local-open-gates"
                        | "attributed-through-tokio-socket-boundary-open-kernel-gate"
                        | "complete"
                )
            )
    });
    if product_mutation_allowed && !w1_attributed {
        problems.push("product mutation requires locally attributed W1".to_owned());
    }
    for item in &work {
        if string(item, "id").is_some_and(|id| !matches!(id, "W0" | "W1")) {
            let decision = string(item, "decision").unwrap_or_default();
            if decision != "not-authorized" && !w1_attributed {
                problems.push("W2-W9 must remain not-authorized before W1 attribution".to_owned());
            }
            if decision.starts_with("authorized-") {
                if !product_mutation_allowed {
                    problems.push("authorized product work requires mutation admission".to_owned());
                }
                if string(item, "evidence").is_none() {
                    problems.push("authorized product work requires evidence".to_owned());
                }
            }
        }
    }
    let native = array_of_tables(value, "native_investigations", &mut problems);
    let native_ids = native
        .iter()
        .filter_map(|item| string(item, "id"))
        .collect::<BTreeSet<_>>();
    let expected = (1..=10)
        .map(|index| format!("N{index}"))
        .collect::<BTreeSet<_>>();
    if native_ids
        .iter()
        .map(|id| (*id).to_owned())
        .collect::<BTreeSet<_>>()
        != expected
    {
        problems.push("proposal-registry must contain N1 through N10 exactly once".to_owned());
    }
    problems
}

pub fn check_host(value: &TomlValue) -> Vec<String> {
    let mut problems = common(value, "host-profile");
    expect_str(value, "state", "local-unadmitted", &mut problems);
    expect_bool(value, "claim_eligible", false, &mut problems);
    let release = table(value, "release_qualification", &mut problems);
    for field in [
        "explicit_authorization_required",
        "dedicated_host_required",
        "cpu_placement_required",
        "irq_policy_required",
        "power_state_required",
        "toolchain_match_required",
        "redis_identity_match_required",
    ] {
        expect_bool(release, field, true, &mut problems);
    }
    problems
}

pub fn check_local_harness(value: &TomlValue) -> Vec<String> {
    let mut problems = common(value, "local-harness");
    expect_str(value, "contract_id", "local-pairing-074-v1", &mut problems);
    expect_str(value, "state", "preregistered-local", &mut problems);
    expect_bool(value, "promotable", false, &mut problems);
    expect_i64(value, "pairs", 5, &mut problems);
    expect_str(value, "order", "abba-counterbalanced-v1", &mut problems);
    expect_i64(value, "same_binary_pairs", 5, &mut problems);
    for field in [
        "warmup_required",
        "independent_processes",
        "cpu_affinity_required",
        "priority_required",
    ] {
        expect_bool(value, field, true, &mut problems);
    }
    expect_i64(value, "quiet_sample_milliseconds", 500, &mut problems);
    expect_i64(value, "maximum_failed_attempts", 0, &mut problems);
    expect_f64(value, "maximum_background_cpu_percent", 20.0, &mut problems);
    let noise = table(value, "noise", &mut problems);
    expect_f64(noise, "percentile", 0.95, &mut problems);
    expect_f64(noise, "minimum_goodput_effect", 0.02, &mut problems);
    expect_f64(
        noise,
        "minimum_cpu_per_operation_effect",
        0.03,
        &mut problems,
    );
    expect_f64(noise, "minimum_p99_effect", 0.03, &mut problems);
    expect_f64(noise, "noise_multiplier", 2.0, &mut problems);
    expect_str(
        noise,
        "classification",
        "inconclusive-below-aa-derived-mde",
        &mut problems,
    );
    let receipt = table(value, "receipt", &mut problems);
    let identities = string_array(receipt, "required_identity_fields");
    for field in [
        "release",
        "surface",
        "operation",
        "operations",
        "warmup_operations",
        "concurrency",
        "payload_bytes",
        "key_space",
        "seed",
        "workload_sha256",
    ] {
        if !identities.contains(field) {
            problems.push(format!("local harness is missing identity field {field}"));
        }
    }
    let metrics = string_array(receipt, "required_metrics");
    for metric in [
        "goodput_operations_per_second",
        "cpu_nanoseconds_per_operation",
        "latency.p99_us",
    ] {
        if !metrics.contains(metric) {
            problems.push(format!("local harness is missing metric {metric}"));
        }
    }
    problems
}

pub fn check_receipt(value: &JsonValue) -> Vec<String> {
    let mut problems = Vec::new();
    if value.get("release").and_then(JsonValue::as_str) != Some(RELEASE) {
        problems.push("receipt release must be 0.74".to_owned());
    }
    if value
        .get("candidate_derived_thresholds")
        .and_then(JsonValue::as_bool)
        != Some(false)
    {
        problems.push("candidate-derived thresholds are forbidden".to_owned());
    }
    if value
        .get("final_sample_present")
        .and_then(JsonValue::as_bool)
        != Some(true)
    {
        problems.push("receipt is missing the final sample".to_owned());
    }
    let declared = value
        .get("declared_block_order")
        .and_then(JsonValue::as_str);
    let blocks = value
        .get("block_order")
        .and_then(JsonValue::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(JsonValue::as_str)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let expected = match declared {
        Some("baseline_candidate") => vec!["baseline", "candidate"],
        Some("candidate_baseline") => vec!["candidate", "baseline"],
        _ => {
            problems.push("receipt has an invalid declared block order".to_owned());
            Vec::new()
        }
    };
    if blocks != expected {
        problems.push("receipt blocks are missing or reordered".to_owned());
    }
    let baseline = value.get("baseline").unwrap_or(&JsonValue::Null);
    let candidate = value.get("candidate").unwrap_or(&JsonValue::Null);
    for (name, role) in [("baseline", baseline), ("candidate", candidate)] {
        let binaries = role.get("binary_sha256s").and_then(JsonValue::as_array);
        if binaries.is_some_and(|items| items.len() != 1) {
            problems.push(format!("{name} mixes more than one binary"));
        }
        for field in ["binary_sha256", "source_sha"] {
            if role.get(field).and_then(JsonValue::as_str).is_none() {
                problems.push(format!("{name} is missing {field}"));
            }
        }
        let outcomes = role.get("outcomes").unwrap_or(&JsonValue::Null);
        for field in [
            "completed",
            "errors",
            "timeouts",
            "rejections",
            "late",
            "incomplete",
            "final_cardinality",
            "final_state_sha256",
        ] {
            if outcomes.get(field).is_none() {
                problems.push(format!("{name} outcomes are missing {field}"));
            }
        }
        if let Some(metrics) = role.get("metrics").and_then(JsonValue::as_object) {
            for (metric, number) in metrics {
                if number.as_f64().is_none_or(|value| !value.is_finite()) {
                    problems.push(format!("{name} metric {metric} is not finite"));
                }
            }
        } else {
            problems.push(format!("{name} is missing metrics"));
        }
    }
    let baseline_trace = baseline.get("trace").unwrap_or(&JsonValue::Null);
    let candidate_trace = candidate.get("trace").unwrap_or(&JsonValue::Null);
    for field in [
        "trace_sha256",
        "payload_corpus_sha256",
        "key_corpus_sha256",
        "seed",
        "warmup_seconds",
        "measurement_seconds",
        "operation_count",
        "offered_schedule_sha256",
        "concurrency",
        "pipeline_depth",
        "security",
        "persistence",
        "semantic_contract",
    ] {
        if baseline_trace.get(field).is_none() || candidate_trace.get(field).is_none() {
            problems.push(format!("receipt trace is missing guarded field {field}"));
        } else if baseline_trace.get(field) != candidate_trace.get(field) {
            problems.push(format!("baseline/candidate trace mismatch for {field}"));
        }
    }
    if let (Some(left), Some(right)) = (
        baseline.get("redis_tool_sha256"),
        candidate.get("redis_tool_sha256"),
    ) {
        if left != right {
            problems.push("mismatched Redis tool identities".to_owned());
        }
    }
    if let Some(results) = value.get("surface_results").and_then(JsonValue::as_array) {
        let mut surfaces = BTreeSet::new();
        for result in results {
            if let Some(surface) = result.get("surface").and_then(JsonValue::as_str) {
                if surface == "native" || !surfaces.insert(surface) {
                    problems
                        .push("pooled or duplicate native surface result is forbidden".to_owned());
                }
                if result.get("trace_sha256") != baseline_trace.get("trace_sha256") {
                    problems.push(format!("surface {surface} does not use the matched trace"));
                }
                if surface != "resp-api" {
                    let baseline_goodput = json_f64(result, "baseline_goodput");
                    let candidate_goodput = json_f64(result, "candidate_goodput");
                    if matches!((baseline_goodput, candidate_goodput), (Some(base), Some(candidate)) if candidate / base < 0.98)
                    {
                        problems.push(format!(
                            "native surface {surface} regresses goodput beyond 2%"
                        ));
                    }
                }
            }
        }
        if value
            .get("integrated_native_guard")
            .and_then(JsonValue::as_bool)
            == Some(true)
        {
            for required in [
                "native-api-hc1",
                "native-api-hc2",
                "client-surface-state",
                "embedded-hydracache-raw",
                "embedded-hydracache-typed",
            ] {
                if !surfaces.contains(required) {
                    problems.push(format!("integrated native guard omits {required}"));
                }
            }
        }
    }
    problems
}

fn common(value: &TomlValue, name: &str) -> Vec<String> {
    let mut problems = Vec::new();
    expect_i64(value, "schema_version", 1, &mut problems);
    expect_str(value, "release", RELEASE, &mut problems);
    if value.as_table().is_none() {
        problems.push(format!("{name} root must be a table"));
    }
    problems
}

fn read_toml(path: &Path) -> Result<TomlValue, Box<dyn Error>> {
    Ok(toml::from_str(&fs::read_to_string(path)?)?)
}

fn table<'a>(value: &'a TomlValue, field: &str, problems: &mut Vec<String>) -> &'a TomlValue {
    if let Some(value) = value.get(field).filter(|value| value.is_table()) {
        value
    } else {
        problems.push(format!("missing table {field}"));
        value
    }
}

fn array_of_tables<'a>(
    value: &'a TomlValue,
    field: &str,
    problems: &mut Vec<String>,
) -> Vec<&'a TomlValue> {
    match value.get(field).and_then(TomlValue::as_array) {
        Some(items) if items.iter().all(TomlValue::is_table) => items.iter().collect(),
        _ => {
            problems.push(format!("missing array of tables {field}"));
            Vec::new()
        }
    }
}

fn expect_str(value: &TomlValue, field: &str, expected: &str, problems: &mut Vec<String>) {
    if string(value, field) != Some(expected) {
        problems.push(format!("{field} must be {expected:?}"));
    }
}

fn expect_bool(value: &TomlValue, field: &str, expected: bool, problems: &mut Vec<String>) {
    if boolean(value, field) != Some(expected) {
        problems.push(format!("{field} must be {expected}"));
    }
}

fn expect_i64(value: &TomlValue, field: &str, expected: i64, problems: &mut Vec<String>) {
    if value.get(field).and_then(TomlValue::as_integer) != Some(expected) {
        problems.push(format!("{field} must be {expected}"));
    }
}

fn expect_f64(value: &TomlValue, field: &str, expected: f64, problems: &mut Vec<String>) {
    let actual = value.get(field).and_then(|value| {
        value
            .as_float()
            .or_else(|| value.as_integer().map(|n| n as f64))
    });
    if actual != Some(expected) {
        problems.push(format!("{field} must be {expected}"));
    }
}

fn expect_array(value: &TomlValue, field: &str, expected: &[&str], problems: &mut Vec<String>) {
    let actual = string_array(value, field);
    if actual != expected.iter().copied().collect::<BTreeSet<_>>() {
        problems.push(format!("{field} does not match the frozen values"));
    }
}

fn expect_integer_array(
    value: &TomlValue,
    field: &str,
    expected: &[i64],
    problems: &mut Vec<String>,
) {
    let actual = value.get(field).and_then(TomlValue::as_array).map(|items| {
        items
            .iter()
            .filter_map(TomlValue::as_integer)
            .collect::<Vec<_>>()
    });
    if actual.as_deref() != Some(expected) {
        problems.push(format!("{field} does not match the frozen values"));
    }
}

fn string<'a>(value: &'a TomlValue, field: &str) -> Option<&'a str> {
    value.get(field).and_then(TomlValue::as_str)
}

fn boolean(value: &TomlValue, field: &str) -> Option<bool> {
    value.get(field).and_then(TomlValue::as_bool)
}

fn string_array<'a>(value: &'a TomlValue, field: &str) -> BTreeSet<&'a str> {
    value
        .get(field)
        .and_then(TomlValue::as_array)
        .into_iter()
        .flatten()
        .filter_map(TomlValue::as_str)
        .collect()
}

fn json_f64(value: &JsonValue, field: &str) -> Option<f64> {
    value.get(field).and_then(JsonValue::as_f64)
}

struct Options {
    root: PathBuf,
    receipt: Option<PathBuf>,
    require_ship: bool,
}

impl Options {
    fn parse(args: Vec<String>) -> Result<Self, Box<dyn Error>> {
        let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut receipt = None;
        let mut require_ship = false;
        let mut release = None;
        let mut args = args.into_iter();
        while let Some(argument) = args.next() {
            match argument.as_str() {
                "--release" => release = args.next(),
                "--root" => root = PathBuf::from(args.next().ok_or("--root requires a path")?),
                "--receipt" => {
                    receipt = Some(PathBuf::from(
                        args.next().ok_or("--receipt requires a path")?,
                    ))
                }
                "--require-ship" => require_ship = true,
                _ => {
                    return Err(
                        format!("unsupported performance-contract-check option {argument}").into(),
                    )
                }
            }
        }
        if release.as_deref() != Some(RELEASE) {
            return Err("0.74 checker requires --release 0.74".into());
        }
        Ok(Self {
            root,
            receipt,
            require_ship,
        })
    }
}
