use serde_json::Value as JsonValue;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use toml::Value as TomlValue;

const CONTRACT: &str = "docs/testing/performance/0.73/local-screening.toml";
const RECEIPT_SCHEMA: &str = "docs/testing/performance/0.73/local-screening-receipt-v1.schema.json";
const CONTEXT_SCHEMA: &str = "docs/testing/performance/0.73/local-screening-context-v1.schema.json";
const EXAMPLE_RECEIPT: &str = "docs/testing/performance/0.73/local-screening-receipt.example.json";
const BASELINE_IDENTITIES: &str = "docs/testing/performance/0.73/baseline-identities.toml";
const POST_TAG_DELTA: &str = "docs/testing/performance/0.73/post-tag-delta.toml";
const SCENARIO_MATRIX: &str = "docs/testing/performance/0.73/scenario-matrix.toml";
const INSTRUMENTATION_OVERHEAD: &str =
    "docs/testing/performance/0.73/instrumentation-overhead.toml";
const LOCAL_OVERHEAD_SCREENING: &str =
    "docs/testing/performance/0.73/local-overhead-screening-307b3500.toml";
const LOCAL_OVERHEAD_ISOLATION: &str =
    "docs/testing/performance/0.73/local-overhead-isolation-5264d96c.toml";
const RELEASE: &str = "0.73";
const PROFILE: &str = "local-screening-073-v1";
const ENVIRONMENT_CLASS: &str = "local_screening";

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let options = Options::parse(args)?;
    let mut problems = check_at_root(&options.root, &options.release, options.receipt.as_deref())?;
    if options.require_ship {
        problems.push(
            "local screening is intentionally non-promotable and cannot satisfy --require-ship"
                .to_owned(),
        );
    }
    finish("performance-contract-check", &options.release, problems)
}

pub fn check_at_root(
    root: &Path,
    release: &str,
    receipt: Option<&Path>,
) -> Result<Vec<String>, Box<dyn Error>> {
    let contract: TomlValue = toml::from_str(&fs::read_to_string(root.join(CONTRACT))?)?;
    let schema: JsonValue = serde_json::from_slice(&fs::read(root.join(RECEIPT_SCHEMA))?)?;
    let example: JsonValue = serde_json::from_slice(&fs::read(root.join(EXAMPLE_RECEIPT))?)?;
    let identities: TomlValue =
        toml::from_str(&fs::read_to_string(root.join(BASELINE_IDENTITIES))?)?;
    let delta: TomlValue = toml::from_str(&fs::read_to_string(root.join(POST_TAG_DELTA))?)?;
    let matrix: TomlValue = toml::from_str(&fs::read_to_string(root.join(SCENARIO_MATRIX))?)?;
    let overhead: TomlValue =
        toml::from_str(&fs::read_to_string(root.join(INSTRUMENTATION_OVERHEAD))?)?;
    let local_overhead: TomlValue =
        toml::from_str(&fs::read_to_string(root.join(LOCAL_OVERHEAD_SCREENING))?)?;
    let local_isolation: TomlValue =
        toml::from_str(&fs::read_to_string(root.join(LOCAL_OVERHEAD_ISOLATION))?)?;
    let mut problems = check_contract(&contract, release);
    problems.extend(check_baseline_identities(root, &identities, release)?);
    problems.extend(check_post_tag_delta(root, &delta, release)?);
    problems.extend(check_scenario_matrix(&matrix, release));
    problems.extend(check_instrumentation_overhead(&overhead, release));
    problems.extend(check_local_overhead_screening(&local_overhead, release));
    problems.extend(check_local_overhead_isolation(&local_isolation, release));
    problems.extend(check_schema(
        &schema,
        &example,
        "checked-in local screening example",
    ));
    problems.extend(check_receipt(&example, &contract));
    if let Some(path) = receipt {
        let path = if path.is_absolute() {
            path.to_owned()
        } else {
            root.join(path)
        };
        let value: JsonValue = serde_json::from_slice(&fs::read(&path)?)?;
        problems.extend(check_schema(
            &schema,
            &value,
            &format!("local screening receipt {}", path.display()),
        ));
        problems.extend(check_receipt(&value, &contract));
    }
    Ok(problems)
}

pub fn check_receipt_at_root(
    root: &Path,
    value: &JsonValue,
) -> Result<Vec<String>, Box<dyn Error>> {
    let contract: TomlValue = toml::from_str(&fs::read_to_string(root.join(CONTRACT))?)?;
    let schema: JsonValue = serde_json::from_slice(&fs::read(root.join(RECEIPT_SCHEMA))?)?;
    let mut problems = check_schema(&schema, value, "generated local screening receipt");
    problems.extend(check_receipt(value, &contract));
    Ok(problems)
}

pub fn check_contract(root: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if release != RELEASE {
        problems.push(format!(
            "unsupported performance contract release {release}"
        ));
        return problems;
    }
    if integer(root, "schema_version") != Some(1) {
        problems.push("local screening schema_version must be 1".to_owned());
    }
    for (field, expected) in [
        ("release", RELEASE),
        ("profile_id", PROFILE),
        ("environment_class", ENVIRONMENT_CLASS),
        ("receipt_schema", RECEIPT_SCHEMA),
        ("context_schema", CONTEXT_SCHEMA),
        ("baseline_identities", BASELINE_IDENTITIES),
        ("post_tag_delta", POST_TAG_DELTA),
        ("scenario_matrix", SCENARIO_MATRIX),
        ("instrumentation_overhead", INSTRUMENTATION_OVERHEAD),
        ("local_overhead_screening", LOCAL_OVERHEAD_SCREENING),
        ("local_overhead_isolation", LOCAL_OVERHEAD_ISOLATION),
    ] {
        if text(root, field) != Some(expected) {
            problems.push(format!("local screening {field} must be {expected}"));
        }
    }
    for field in [
        "promotable",
        "numerical_claims_allowed",
        "candidate_thresholds_allowed",
    ] {
        if boolean(root, field) != Some(false) {
            problems.push(format!("local screening {field} must be false"));
        }
    }
    for field in [
        "require_counterbalanced_order",
        "require_prebuilt_binary",
        "require_binary_sha256",
        "require_scenario_sha256",
        "preserve_failed_attempts",
    ] {
        if boolean(root, field) != Some(true) {
            problems.push(format!("local screening {field} must be true"));
        }
    }
    let required_pairs = integer(root, "minimum_pairs").unwrap_or_default();
    let recommended_pairs = integer(root, "recommended_pairs").unwrap_or_default();
    if required_pairs < 3 || recommended_pairs < required_pairs {
        problems.push(
            "local screening requires at least three pairs and a recommendation no lower than the minimum"
                .to_owned(),
        );
    }
    if text(root, "non_promotion_reason").is_none_or(str::is_empty) {
        problems.push("local screening requires a non_promotion_reason".to_owned());
    }
    if !string_array(root.get("allowed_promotion_targets")).is_empty() {
        problems.push("local screening must not declare promotion targets".to_owned());
    }
    let expected_concurrency = [1_i64, 8, 32, 128];
    if integer_array(root.get("concurrency_lanes")) != expected_concurrency {
        problems.push("local screening concurrency_lanes must be 1,8,32,128".to_owned());
    }
    let load_fractions = float_array(root.get("offered_load_fractions"));
    if load_fractions.len() != 3
        || load_fractions
            .iter()
            .zip([0.25, 0.60, 0.85])
            .any(|(actual, expected)| (actual - expected).abs() > f64::EPSILON)
    {
        problems.push("local screening offered_load_fractions must be 0.25,0.60,0.85".to_owned());
    }
    if float(root, "maximum_goodput_regression") != Some(0.02) {
        problems.push("local screening maximum_goodput_regression must remain 0.02".to_owned());
    }
    if string_array(root.get("allowed_instrumentation_modes")) != ["off", "production", "profile"] {
        problems.push(
            "local screening allowed_instrumentation_modes must be off,production,profile"
                .to_owned(),
        );
    }
    let outcomes: BTreeSet<_> = string_array(root.get("required_outcome_fields"))
        .into_iter()
        .collect();
    let expected: BTreeSet<_> = [
        "attempted",
        "success",
        "rejected",
        "timeout",
        "late",
        "incomplete",
    ]
    .into_iter()
    .collect();
    if outcomes != expected {
        problems.push("local screening outcome accounting is incomplete".to_owned());
    }
    problems
}

pub fn check_local_overhead_screening(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_class") != Some("local_screening")
    {
        problems.push("local overhead screening identity mismatch".to_owned());
    }
    if boolean(value, "promotable") != Some(false)
        || boolean(value, "numerical_claim_eligible") != Some(false)
        || text(value, "thresholds_status") != Some("screening_only_unqualified")
        || text(value, "decision") != Some("blocked-instrumentation-overhead")
    {
        problems.push("local overhead screening must remain a non-promotable blocker".to_owned());
    }
    for field in ["source_sha"] {
        if text(value, field).is_none_or(|sha| !full_sha(sha)) {
            problems.push(format!(
                "local overhead screening {field} is not a full SHA"
            ));
        }
    }
    for field in [
        "binary_sha256",
        "host_fingerprint_sha256",
        "screening_sha256",
    ] {
        if text(value, field).is_none_or(|digest| !sha256(digest)) {
            problems.push(format!("local overhead screening {field} is not SHA-256"));
        }
    }
    if integer(value, "pair_count").is_none_or(|count| count < 3)
        || boolean(value, "counterbalanced") != Some(true)
        || integer(value, "failed_attempts") != Some(0)
    {
        problems.push(
            "local overhead screening lacks three successful counterbalanced pairs".to_owned(),
        );
    }
    let observations = value
        .get("median_observations")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    let metrics: BTreeSet<_> = observations
        .iter()
        .filter_map(|item| text(item, "metric"))
        .collect();
    for required in [
        "elapsed_ns",
        "fill_allocated_bytes_per_operation",
        "steady_allocated_bytes_per_operation",
        "expire_delete_allocated_bytes_per_operation",
        "refill_allocated_bytes_per_operation",
        "post_idle_rss_delta_bytes",
        "peak_rss_delta_bytes",
    ] {
        if !metrics.contains(required) {
            problems.push(format!("local overhead screening omits {required}"));
        }
    }
    for item in &observations {
        for field in ["off", "production", "regression_fraction"] {
            if float(item, field).is_none_or(|number| !number.is_finite()) {
                problems.push(format!("local overhead observation has invalid {field}"));
            }
        }
    }
    if text(value, "suspected_owner").is_none_or(str::is_empty)
        || text(value, "next_evidence").is_none_or(str::is_empty)
    {
        problems.push(
            "local overhead screening requires owner hypothesis and next evidence".to_owned(),
        );
    }
    problems
}

pub fn check_local_overhead_isolation(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_class") != Some("local_screening")
        || text(value, "profile_id") != Some("instrumentation-overhead-counters-only-073-v1")
    {
        problems.push("local overhead isolation identity mismatch".to_owned());
    }
    if boolean(value, "diagnostic_only") != Some(true)
        || boolean(value, "counter_correctness_eligible") != Some(false)
        || boolean(value, "promotable") != Some(false)
        || boolean(value, "numerical_claim_eligible") != Some(false)
        || text(value, "thresholds_status") != Some("screening_only_unqualified")
        || text(value, "decision") != Some("listener-cost-attributed-local-diagnostic")
    {
        problems
            .push("local overhead isolation must remain diagnostic and non-promotable".to_owned());
    }
    if text(value, "source_sha").is_none_or(|sha| !full_sha(sha)) {
        problems.push("local overhead isolation source_sha is not a full SHA".to_owned());
    }
    for field in [
        "binary_sha256",
        "host_fingerprint_sha256",
        "screening_sha256",
    ] {
        if text(value, field).is_none_or(|digest| !sha256(digest)) {
            problems.push(format!("local overhead isolation {field} is not SHA-256"));
        }
    }
    if integer(value, "pair_count").is_none_or(|count| count < 3)
        || boolean(value, "counterbalanced") != Some(true)
        || integer(value, "failed_attempts") != Some(0)
    {
        problems.push(
            "local overhead isolation lacks three successful counterbalanced pairs".to_owned(),
        );
    }
    let observations = value
        .get("median_observations")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    let metrics: BTreeSet<_> = observations
        .iter()
        .filter_map(|item| text(item, "metric"))
        .collect();
    for required in [
        "elapsed_ns",
        "fill_allocated_bytes_per_operation",
        "steady_allocated_bytes_per_operation",
        "expire_delete_allocated_bytes_per_operation",
        "refill_allocated_bytes_per_operation",
        "post_idle_rss_delta_bytes",
        "peak_rss_delta_bytes",
    ] {
        if !metrics.contains(required) {
            problems.push(format!("local overhead isolation omits {required}"));
        }
    }
    for item in &observations {
        for field in ["off", "production", "regression_fraction"] {
            if float(item, field).is_none_or(|number| !number.is_finite()) {
                problems.push(format!("local overhead isolation has invalid {field}"));
            }
        }
    }
    for field in ["isolated_factor", "conclusion", "next_evidence"] {
        if text(value, field).is_none_or(str::is_empty) {
            problems.push(format!("local overhead isolation requires {field}"));
        }
    }
    problems
}

pub fn check_instrumentation_overhead(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1) || text(value, "release") != Some(release) {
        problems.push("instrumentation overhead schema/release mismatch".to_owned());
        return problems;
    }
    if text(value, "contract_id") != Some("instrumentation-overhead-073-v1")
        || text(value, "measurement_profile") != Some("instrumentation-overhead-073-v1")
        || text(value, "resource_phase_schema")
            != Some("docs/testing/performance/0.73/resource-phase-v1.schema.json")
        || text(value, "state") != Some("pilot")
        || boolean(value, "i73_freeze_allowed") != Some(false)
        || boolean(value, "candidate_data_allowed") != Some(false)
    {
        problems.push("instrumentation overhead must remain a candidate-blocking pilot".to_owned());
    }
    if text(value, "comparison") != Some("off_vs_production")
        || text(value, "classification_only_mode") != Some("profile")
    {
        problems.push(
            "instrumentation overhead must compare off/production and isolate profile".to_owned(),
        );
    }
    for field in [
        "same_source_required",
        "same_binary_required",
        "same_toolchain_required",
        "same_host_required",
        "same_scenario_required",
        "same_trace_required",
        "counterbalanced_order_required",
        "independent_processes_required",
        "complete_outcome_accounting_required",
        "raw_series_required",
        "failed_attempts_preserved",
    ] {
        if boolean(value, field) != Some(true) {
            problems.push(format!("instrumentation overhead requires {field}=true"));
        }
    }
    if integer(value, "minimum_screening_pairs").is_none_or(|count| count < 3)
        || integer(value, "minimum_qualification_pairs").is_none_or(|count| count < 5)
        || float(value, "throughput_regression_limit") != Some(0.02)
        || float(value, "cpu_per_request_regression_limit") != Some(0.03)
        || float(value, "p99_regression_limit") != Some(0.03)
        || integer(value, "unexpected_failures_allowed") != Some(0)
    {
        problems.push("instrumentation overhead weakens inherited regression limits".to_owned());
    }
    if text(value, "allocation_regression_limit_state") != Some("unmeasured")
        || text(value, "rss_delta_limit_state") != Some("unmeasured")
    {
        problems.push(
            "allocation/RSS limits must remain unmeasured until baseline-only evidence".to_owned(),
        );
    }
    let metrics: BTreeSet<_> = string_array(value.get("required_metrics"))
        .into_iter()
        .collect();
    for metric in [
        "goodput",
        "cpu_per_request",
        "p99",
        "allocations_per_request",
        "allocated_bytes_per_request",
        "rss_delta_bytes",
    ] {
        if !metrics.contains(metric) {
            problems.push(format!(
                "instrumentation overhead omits required metric {metric}"
            ));
        }
    }
    let workloads: BTreeSet<_> = string_array(value.get("required_workloads"))
        .into_iter()
        .collect();
    for workload in ["cold", "small-hot", "tag-heavy", "hc2-1000", "reset"] {
        if !workloads.contains(workload) {
            problems.push(format!(
                "instrumentation overhead omits workload {workload}"
            ));
        }
    }
    if string_array(value.get("blocking_before_i73")).len() < 5 {
        problems.push("instrumentation overhead does not enumerate all I73 blockers".to_owned());
    }
    problems
}

pub fn check_scenario_matrix(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1) || text(value, "release") != Some(release) {
        problems.push("scenario matrix schema/release mismatch".to_owned());
        return problems;
    }
    if text(value, "matrix_id") != Some("performance-073-v1")
        || text(value, "state") != Some("pilot")
        || boolean(value, "candidate_measurement_allowed") != Some(false)
    {
        problems.push("scenario matrix must remain a candidate-blocking 0.73 pilot".to_owned());
    }
    if text(value, "baseline_identity") != Some("I73")
        || text(value, "baseline_source_sha") != Some("")
        || text(value, "external_baseline_identity") != Some("B72")
    {
        problems.push("scenario matrix must distinguish unfrozen I73 from external B72".to_owned());
    }
    for field in [
        "calibration_state",
        "measurement_windows_state",
        "stable_rates_state",
    ] {
        if text(value, field) != Some("unmeasured") {
            problems.push(format!(
                "scenario matrix {field} must remain unmeasured before I73 freeze"
            ));
        }
    }
    if integer_array(value.get("concurrency_lanes")) != [1, 8, 32, 128]
        || float_array(value.get("offered_load_fractions")) != [0.25, 0.60, 0.85]
    {
        problems.push("scenario matrix changed the frozen concurrency/load lanes".to_owned());
    }
    if float(value, "maximum_goodput_regression") != Some(0.02)
        || integer(value, "unexpected_failures_allowed") != Some(0)
        || integer(value, "minimum_screening_pairs").is_none_or(|value| value < 3)
        || integer(value, "minimum_claim_pairs").is_none_or(|value| value < 5)
    {
        problems.push("scenario matrix weakened comparison admission".to_owned());
    }
    let expected_modes = ["off", "production", "profile"];
    if string_array(value.get("instrumentation_modes")) != expected_modes
        || text(value, "candidate_comparison_instrumentation_mode") != Some("production")
        || boolean(value, "profile_samples_classification_only") != Some(true)
    {
        problems.push("scenario matrix conflates production and profile identities".to_owned());
    }
    for field in [
        "counterbalanced_pair_order_required",
        "raw_series_required",
        "complete_outcome_accounting_required",
    ] {
        if boolean(value, field) != Some(true) {
            problems.push(format!("scenario matrix requires {field}=true"));
        }
    }
    let trace = value.get("trace").unwrap_or(&TomlValue::Boolean(false));
    if text(trace, "format") != Some("length-framed-v1") {
        problems.push("scenario matrix requires the length-framed-v1 request trace".to_owned());
    }
    for field in [
        "byte_identical_between_roles",
        "seed_frozen_before_candidate",
        "pacing_frozen_before_candidate",
        "warmup_frozen_before_candidate",
        "duration_frozen_before_candidate",
        "shutdown_and_drain_frozen_before_candidate",
    ] {
        if boolean(trace, field) != Some(true) {
            problems.push(format!("scenario matrix trace requires {field}=true"));
        }
    }
    let pilot = value.get("pilot").unwrap_or(&TomlValue::Boolean(false));
    if text(pilot, "evidence_class") != Some("local_screening")
        || boolean(pilot, "promotable") != Some(false)
        || boolean(pilot, "numerical_claim_eligible") != Some(false)
    {
        problems
            .push("scenario matrix pilot evidence must remain local and non-promotable".to_owned());
    }
    let mut surfaces = BTreeMap::new();
    for surface in value
        .get("surfaces")
        .and_then(TomlValue::as_array)
        .into_iter()
        .flatten()
    {
        let work_item = text(surface, "work_item").unwrap_or_default();
        if surfaces.insert(work_item, surface).is_some() {
            problems.push(format!("scenario matrix duplicates {work_item}"));
        }
        if boolean(surface, "d1_analysis_required") != Some(true)
            || boolean(surface, "local_screening_required") != Some(true)
            || string_array(surface.get("protocols")).is_empty()
            || string_array(surface.get("persistence_modes")).is_empty()
            || string_array(surface.get("primary_observations")).is_empty()
        {
            problems.push(format!(
                "scenario matrix has incomplete surface {work_item}"
            ));
        }
    }
    let expected_surfaces = [
        ("W2", "shared-store-expiry", "A"),
        ("W3", "tag-index", "B"),
        ("W4", "resp-translation", "B"),
        ("W5", "hc2-connection-state", "A"),
        ("W6", "management-service-overhead", "A"),
        ("W7", "durability-page-cache", "B"),
        ("W8", "allocator", "B"),
        ("W9", "retained-byte-admission", "C"),
    ];
    for (work_item, id, wave) in expected_surfaces {
        match surfaces.get(work_item) {
            Some(surface)
                if text(surface, "id") == Some(id) && text(surface, "wave") == Some(wave) => {}
            Some(_) => problems.push(format!(
                "scenario matrix changes {work_item} identity or wave"
            )),
            None => problems.push(format!("scenario matrix omits mandatory {work_item}")),
        }
    }
    if surfaces.get("W7").is_none_or(|surface| {
        !string_array(surface.get("persistence_modes")).contains(&"all-supported-separate")
    }) || surfaces.get("W9").is_none_or(|surface| {
        !string_array(surface.get("persistence_modes")).contains(&"all-supported-separate")
    }) {
        problems.push("W7/W9 must keep supported persistence modes in separate cells".to_owned());
    }
    let mixed = value
        .get("mixed_runtime")
        .unwrap_or(&TomlValue::Boolean(false));
    let weights = mixed
        .get("weights_percent")
        .unwrap_or(&TomlValue::Boolean(false));
    let expected_weights = [
        ("hc2_key_and_subscription", 35),
        ("resp", 30),
        ("hc1_native_client", 15),
        ("direct_local_client_surface", 10),
        ("tag_invalidation", 5),
        ("ttl_expire_refill", 5),
    ];
    let weight_total: i64 = expected_weights
        .iter()
        .map(|(field, expected)| {
            let actual = integer(weights, field).unwrap_or_default();
            if actual != *expected {
                problems.push(format!("mixed-runtime weight {field} must be {expected}%"));
            }
            actual
        })
        .sum();
    if text(mixed, "id") != Some("mixed-runtime-073-v1")
        || text(mixed, "state") != Some("unmeasured")
        || integer(mixed, "management_reads_per_second") != Some(1)
        || boolean(mixed, "persistence_modes_separate") != Some(true)
        || boolean(mixed, "aggregate_cannot_hide_surface_regression") != Some(true)
        || weight_total != 100
    {
        problems.push(
            "mixed-runtime pilot contract is incomplete or weights do not total 100%".to_owned(),
        );
    }
    problems
}

pub fn check_baseline_identities(
    root: &Path,
    value: &TomlValue,
    release: &str,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1) || text(value, "release") != Some(release) {
        problems.push("baseline identity schema/release mismatch".to_owned());
        return Ok(problems);
    }
    let baseline = value
        .get("published_baseline")
        .unwrap_or(&TomlValue::Boolean(false));
    let branch_root = value
        .get("branch_root")
        .unwrap_or(&TomlValue::Boolean(false));
    let instrumented = value
        .get("instrumented_baseline")
        .unwrap_or(&TomlValue::Boolean(false));
    let tag = text(baseline, "tag").unwrap_or_default();
    let tag_object = text(baseline, "tag_object_sha").unwrap_or_default();
    let peeled = text(baseline, "peeled_commit_sha").unwrap_or_default();
    let root_sha = text(branch_root, "commit_sha").unwrap_or_default();
    for (label, value) in [
        ("tag_object_sha", tag_object),
        ("peeled_commit_sha", peeled),
        ("branch_root commit_sha", root_sha),
    ] {
        if !full_sha(value) {
            problems.push(format!(
                "baseline {label} must be a full lowercase commit SHA"
            ));
        }
    }
    if tag.is_empty() || git(root, &["cat-file", "-t", tag]).ok().as_deref() != Some("tag") {
        problems.push("B72 must resolve through an annotated tag object".to_owned());
    }
    if !tag.is_empty() && git(root, &["rev-parse", tag]).ok().as_deref() != Some(tag_object) {
        problems.push("B72 tag object SHA does not match the repository".to_owned());
    }
    let peeled_ref = format!("{tag}^{{}}");
    if !tag.is_empty() && git(root, &["rev-parse", &peeled_ref]).ok().as_deref() != Some(peeled) {
        problems.push("B72 peeled commit SHA does not match the repository".to_owned());
    }
    if !root_sha.is_empty() && git(root, &["rev-parse", root_sha]).ok().as_deref() != Some(root_sha)
    {
        problems.push("R73 branch root is absent from the repository".to_owned());
    }
    if text(instrumented, "state") != Some("unfrozen")
        || text(instrumented, "source_sha") != Some("")
    {
        problems.push("I73 must remain explicitly unfrozen until W0 admission closes".to_owned());
    }
    Ok(problems)
}

pub fn check_post_tag_delta(
    root: &Path,
    value: &TomlValue,
    release: &str,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1) || text(value, "release") != Some(release) {
        problems.push("post-tag delta schema/release mismatch".to_owned());
        return Ok(problems);
    }
    let from = text(value, "from").unwrap_or_default();
    let to = text(value, "to").unwrap_or_default();
    let allowed: BTreeSet<_> = string_array(value.get("allowed_classifications"))
        .into_iter()
        .collect();
    let mut declared = BTreeMap::new();
    for item in value
        .get("paths")
        .and_then(TomlValue::as_array)
        .into_iter()
        .flatten()
    {
        let path = text(item, "path").unwrap_or_default();
        let status = text(item, "status").unwrap_or_default();
        let classification = text(item, "classification").unwrap_or_default();
        if path.is_empty() || !allowed.contains(classification) {
            problems.push(format!(
                "post-tag delta has invalid path/classification for {path}"
            ));
            continue;
        }
        if declared
            .insert(
                path.to_owned(),
                (status.to_owned(), classification.to_owned()),
            )
            .is_some()
        {
            problems.push(format!("post-tag delta declares {path} more than once"));
        }
    }
    let range = format!("{from}..{to}");
    let observed_text = git(root, &["diff", "--name-status", &range])?;
    let observed: BTreeMap<_, _> = observed_text
        .lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            Some((fields.next()?.to_owned(), fields.next()?.to_owned()))
        })
        .map(|(status, path)| (path, status))
        .collect();
    for (path, status) in &observed {
        match declared.get(path) {
            Some((declared_status, _)) if declared_status == status => {}
            Some((declared_status, _)) => problems.push(format!(
                "post-tag delta status mismatch for {path}: declared {declared_status}, observed {status}"
            )),
            None => problems.push(format!("post-tag delta leaves {path} unclassified")),
        }
    }
    for path in declared.keys() {
        if !observed.contains_key(path) {
            problems.push(format!("post-tag delta declares stale path {path}"));
        }
    }
    let runtime: BTreeSet<_> = string_array(value.get("runtime_paths"))
        .into_iter()
        .collect();
    let instrumentation: BTreeSet<_> = string_array(value.get("instrumentation_paths"))
        .into_iter()
        .collect();
    let declared_runtime: BTreeSet<_> = declared
        .iter()
        .filter(|(_, (_, class))| class == "runtime")
        .map(|(path, _)| path.as_str())
        .collect();
    let declared_instrumentation: BTreeSet<_> = declared
        .iter()
        .filter(|(_, (_, class))| class == "instrumentation")
        .map(|(path, _)| path.as_str())
        .collect();
    if runtime != declared_runtime || instrumentation != declared_instrumentation {
        problems.push(
            "post-tag runtime/instrumentation summaries do not match path classifications"
                .to_owned(),
        );
    }
    Ok(problems)
}

fn git(root: &Path, args: &[&str]) -> Result<String, Box<dyn Error>> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

pub fn check_receipt(value: &JsonValue, contract: &TomlValue) -> Vec<String> {
    let mut problems = Vec::new();
    for (pointer, expected) in [
        ("/release", RELEASE),
        ("/profile_id", PROFILE),
        ("/environment_class", ENVIRONMENT_CLASS),
    ] {
        if value.pointer(pointer).and_then(JsonValue::as_str) != Some(expected) {
            problems.push(format!("receipt {pointer} must be {expected}"));
        }
    }
    if value.get("promotable").and_then(JsonValue::as_bool) != Some(false) {
        problems.push("local receipt must never be promotable".to_owned());
    }
    if value
        .get("numerical_claim_eligible")
        .and_then(JsonValue::as_bool)
        != Some(false)
    {
        problems.push("local receipt must not be numerical-claim eligible".to_owned());
    }
    if value
        .get("non_promotion_reason")
        .and_then(JsonValue::as_str)
        .is_none_or(str::is_empty)
    {
        problems.push("local receipt requires a non_promotion_reason".to_owned());
    }
    if value
        .get("source_sha")
        .and_then(JsonValue::as_str)
        .is_none_or(|value| !full_sha(value))
    {
        problems.push("local receipt source_sha must be a full lowercase commit SHA".to_owned());
    }
    for field in [
        "binary_sha256",
        "scenario_sha256",
        "host_fingerprint_sha256",
        "raw_series_sha256",
    ] {
        if value
            .get(field)
            .and_then(JsonValue::as_str)
            .is_none_or(|value| !sha256(value))
        {
            problems.push(format!("local receipt {field} must be lowercase SHA-256"));
        }
    }
    let modes = string_array(contract.get("allowed_instrumentation_modes"));
    if value
        .get("instrumentation_mode")
        .and_then(JsonValue::as_str)
        .is_none_or(|mode| !modes.contains(&mode))
    {
        problems.push("local receipt has an unsupported instrumentation_mode".to_owned());
    }
    if value.get("pair_index").and_then(JsonValue::as_u64) == Some(0) {
        problems.push("local receipt pair_index must be positive".to_owned());
    }
    let attempted = value
        .pointer("/outcomes/attempted")
        .and_then(JsonValue::as_u64);
    let accounted = ["success", "rejected", "timeout", "late", "incomplete"]
        .into_iter()
        .map(|field| {
            value
                .pointer(&format!("/outcomes/{field}"))
                .and_then(JsonValue::as_u64)
        })
        .try_fold(0_u64, |total, count| {
            count.map(|count| total.saturating_add(count))
        });
    if attempted.is_none() || attempted != accounted {
        problems
            .push("local receipt outcomes must account for every attempted operation".to_owned());
    }
    problems
}

fn check_schema(schema: &JsonValue, value: &JsonValue, label: &str) -> Vec<String> {
    let validator = match jsonschema::validator_for(schema) {
        Ok(validator) => validator,
        Err(error) => return vec![format!("cannot compile local receipt schema: {error}")],
    };
    validator
        .iter_errors(value)
        .map(|error| format!("{label} schema violation: {error}"))
        .collect()
}

fn full_sha(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn text<'a>(root: &'a TomlValue, field: &str) -> Option<&'a str> {
    root.get(field).and_then(TomlValue::as_str)
}

fn integer(root: &TomlValue, field: &str) -> Option<i64> {
    root.get(field).and_then(TomlValue::as_integer)
}

fn float(root: &TomlValue, field: &str) -> Option<f64> {
    root.get(field).and_then(TomlValue::as_float)
}

fn boolean(root: &TomlValue, field: &str) -> Option<bool> {
    root.get(field).and_then(TomlValue::as_bool)
}

fn string_array(value: Option<&TomlValue>) -> Vec<&str> {
    value
        .and_then(TomlValue::as_array)
        .into_iter()
        .flatten()
        .filter_map(TomlValue::as_str)
        .collect()
}

fn integer_array(value: Option<&TomlValue>) -> Vec<i64> {
    value
        .and_then(TomlValue::as_array)
        .into_iter()
        .flatten()
        .filter_map(TomlValue::as_integer)
        .collect()
}

fn float_array(value: Option<&TomlValue>) -> Vec<f64> {
    value
        .and_then(TomlValue::as_array)
        .into_iter()
        .flatten()
        .filter_map(|value| {
            value
                .as_float()
                .or_else(|| value.as_integer().map(|v| v as f64))
        })
        .collect()
}

fn finish(label: &str, release: &str, problems: Vec<String>) -> Result<(), Box<dyn Error>> {
    if problems.is_empty() {
        println!("{label}: OK (release {release}, local screening only)");
        Ok(())
    } else {
        Err(format!("{label} failed:\n- {}", problems.join("\n- ")).into())
    }
}

struct Options {
    root: PathBuf,
    release: String,
    receipt: Option<PathBuf>,
    require_ship: bool,
}

impl Options {
    fn parse(args: Vec<String>) -> Result<Self, Box<dyn Error>> {
        let mut root = crate::doc_check::find_repo_root()?;
        let mut release = None;
        let mut receipt = None;
        let mut require_ship = false;
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--root" => root = PathBuf::from(args.next().ok_or("--root requires a path")?),
                "--release" => release = Some(args.next().ok_or("--release requires a value")?),
                "--receipt" => {
                    receipt = Some(PathBuf::from(
                        args.next().ok_or("--receipt requires a path")?,
                    ))
                }
                "--require-ship" => require_ship = true,
                other => {
                    return Err(
                        format!("unsupported performance contract argument: {other}").into(),
                    )
                }
            }
        }
        Ok(Self {
            root,
            release: release.ok_or("performance-contract-check requires --release")?,
            receipt,
            require_ship,
        })
    }
}
