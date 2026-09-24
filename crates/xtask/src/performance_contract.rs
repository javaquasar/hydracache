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
const LOCAL_OVERHEAD_LISTENER_NOOP: &str =
    "docs/testing/performance/0.73/local-overhead-listener-noop-5e101e69.toml";
const NOTIFICATION_FEASIBILITY: &str =
    "docs/testing/performance/0.73/notification-feasibility-bf9f1382.toml";
const NOTIFICATION_OBSERVER_REQUIREMENTS: &str =
    "docs/testing/performance/0.73/notification-observer-requirements.toml";
const NOTIFICATION_OBSERVER_PROTOTYPE: &str =
    "docs/testing/performance/0.73/notification-observer-prototype-9a2ca114.toml";
const MOKA_OBSERVER_SPIKE: &str = "docs/testing/performance/0.73/moka-observer-spike-5d560170.toml";
const MOKA_OBSERVER_DIRECT: &str =
    "docs/testing/performance/0.73/moka-observer-direct-779849b6.toml";
const NOTIFICATION_OBSERVER_D2_REVIEW: &str =
    "docs/testing/performance/0.73/notification-observer-d2-review.toml";
const MOKA_OBSERVER_UPSTREAM_DRAFT: &str =
    "docs/testing/performance/0.73/moka-post-removal-observer-upstream-draft.md";
const SINGLE_MAINTAINER_REVIEW_POLICY: &str =
    "docs/testing/performance/0.73/single-maintainer-review-policy.toml";
const MOKA_FORK_DECISION: &str = "docs/testing/performance/0.73/moka-fork-decision-352e53fa.toml";
const NOTIFICATION_OBSERVER_PRODUCT: &str =
    "docs/testing/performance/0.73/notification-observer-product-73fc38a1.toml";
const PROPOSAL_REGISTRY: &str = "docs/testing/performance/0.73/proposal-registry.toml";
const STATISTICS: &str = "docs/testing/performance/0.73/statistics.toml";
const HOST_PROFILE: &str = "docs/testing/performance/0.73/host-profile.toml";
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
    let local_listener_noop: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(LOCAL_OVERHEAD_LISTENER_NOOP),
    )?)?;
    let notification_feasibility: TomlValue =
        toml::from_str(&fs::read_to_string(root.join(NOTIFICATION_FEASIBILITY))?)?;
    let notification_observer_requirements: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(NOTIFICATION_OBSERVER_REQUIREMENTS),
    )?)?;
    let notification_observer_prototype: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(NOTIFICATION_OBSERVER_PROTOTYPE),
    )?)?;
    let moka_observer_spike: TomlValue =
        toml::from_str(&fs::read_to_string(root.join(MOKA_OBSERVER_SPIKE))?)?;
    let moka_observer_direct: TomlValue =
        toml::from_str(&fs::read_to_string(root.join(MOKA_OBSERVER_DIRECT))?)?;
    let notification_observer_d2_review: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(NOTIFICATION_OBSERVER_D2_REVIEW),
    )?)?;
    let single_maintainer_review_policy: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(SINGLE_MAINTAINER_REVIEW_POLICY),
    )?)?;
    let moka_fork_decision: TomlValue =
        toml::from_str(&fs::read_to_string(root.join(MOKA_FORK_DECISION))?)?;
    let notification_observer_product: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(NOTIFICATION_OBSERVER_PRODUCT),
    )?)?;
    let proposal_registry: TomlValue =
        toml::from_str(&fs::read_to_string(root.join(PROPOSAL_REGISTRY))?)?;
    let statistics: TomlValue = toml::from_str(&fs::read_to_string(root.join(STATISTICS))?)?;
    let host_profile: TomlValue = toml::from_str(&fs::read_to_string(root.join(HOST_PROFILE))?)?;
    let mut problems = check_contract(&contract, release);
    if !root.join(MOKA_OBSERVER_UPSTREAM_DRAFT).is_file() {
        problems.push("notification observer dependency review draft is missing".to_owned());
    }
    problems.extend(check_baseline_identities(root, &identities, release)?);
    problems.extend(check_post_tag_delta(root, &delta, release)?);
    problems.extend(check_scenario_matrix(&matrix, release));
    problems.extend(check_instrumentation_overhead(&overhead, release));
    problems.extend(check_local_overhead_screening(&local_overhead, release));
    problems.extend(check_local_overhead_isolation(&local_isolation, release));
    problems.extend(check_local_overhead_listener_noop(
        &local_listener_noop,
        release,
    ));
    problems.extend(check_notification_feasibility(
        &notification_feasibility,
        release,
    ));
    problems.extend(check_notification_observer_requirements(
        &notification_observer_requirements,
        release,
    ));
    problems.extend(check_notification_observer_prototype(
        &notification_observer_prototype,
        release,
    ));
    problems.extend(check_moka_observer_spike(&moka_observer_spike, release));
    problems.extend(check_moka_observer_direct(&moka_observer_direct, release));
    problems.extend(check_notification_observer_d2_review(
        &notification_observer_d2_review,
        release,
    ));
    problems.extend(check_single_maintainer_review_policy(
        &single_maintainer_review_policy,
        release,
    ));
    problems.extend(check_moka_fork_decision(&moka_fork_decision, release));
    problems.extend(check_notification_observer_product(
        &notification_observer_product,
        release,
    ));
    problems.extend(check_proposal_registry(&proposal_registry, release));
    problems.extend(check_statistics(&statistics, release));
    problems.extend(check_host_profile(&host_profile, release));
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
        ("local_overhead_listener_noop", LOCAL_OVERHEAD_LISTENER_NOOP),
        ("notification_feasibility", NOTIFICATION_FEASIBILITY),
        (
            "notification_observer_requirements",
            NOTIFICATION_OBSERVER_REQUIREMENTS,
        ),
        (
            "notification_observer_prototype",
            NOTIFICATION_OBSERVER_PROTOTYPE,
        ),
        ("moka_observer_spike", MOKA_OBSERVER_SPIKE),
        ("moka_observer_direct", MOKA_OBSERVER_DIRECT),
        (
            "notification_observer_d2_review",
            NOTIFICATION_OBSERVER_D2_REVIEW,
        ),
        (
            "single_maintainer_review_policy",
            SINGLE_MAINTAINER_REVIEW_POLICY,
        ),
        ("moka_fork_decision", MOKA_FORK_DECISION),
        (
            "notification_observer_product",
            NOTIFICATION_OBSERVER_PRODUCT,
        ),
        ("proposal_registry", PROPOSAL_REGISTRY),
        ("statistics_contract", STATISTICS),
        ("host_profile", HOST_PROFILE),
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

pub fn check_local_overhead_listener_noop(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_class") != Some("local_screening")
        || text(value, "profile_id") != Some("instrumentation-overhead-listener-noop-073-v1")
    {
        problems.push("local no-op listener screening identity mismatch".to_owned());
    }
    if boolean(value, "diagnostic_only") != Some(true)
        || boolean(value, "counter_correctness_eligible") != Some(false)
        || boolean(value, "promotable") != Some(false)
        || boolean(value, "numerical_claim_eligible") != Some(false)
        || text(value, "thresholds_status") != Some("screening_only_unqualified")
        || text(value, "decision") != Some("backend-notification-cost-attributed-local-diagnostic")
    {
        problems.push(
            "local no-op listener screening must remain diagnostic and non-promotable".to_owned(),
        );
    }
    if text(value, "source_sha").is_none_or(|sha| !full_sha(sha)) {
        problems.push("local no-op listener source_sha is not a full SHA".to_owned());
    }
    for field in [
        "binary_sha256",
        "host_fingerprint_sha256",
        "screening_sha256",
    ] {
        if text(value, field).is_none_or(|digest| !sha256(digest)) {
            problems.push(format!("local no-op listener {field} is not SHA-256"));
        }
    }
    if integer(value, "pair_count").is_none_or(|count| count < 3)
        || boolean(value, "counterbalanced") != Some(true)
        || integer(value, "failed_attempts") != Some(0)
    {
        problems.push(
            "local no-op listener screening lacks three successful counterbalanced pairs"
                .to_owned(),
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
            problems.push(format!("local no-op listener screening omits {required}"));
        }
    }
    for item in &observations {
        for field in ["off", "production", "regression_fraction"] {
            if float(item, field).is_none_or(|number| !number.is_finite()) {
                problems.push(format!(
                    "local no-op listener screening has invalid {field}"
                ));
            }
        }
    }
    for field in ["isolated_factor", "conclusion", "next_evidence"] {
        if text(value, field).is_none_or(str::is_empty) {
            problems.push(format!("local no-op listener screening requires {field}"));
        }
    }
    problems
}

pub fn check_notification_feasibility(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_class") != Some("local_feasibility")
        || text(value, "experiment") != Some("moka-future-vs-sync-noop-listener")
    {
        problems.push("notification feasibility identity mismatch".to_owned());
    }
    if boolean(value, "diagnostic_only") != Some(true)
        || boolean(value, "product_semantics_eligible") != Some(false)
        || boolean(value, "promotable") != Some(false)
        || text(value, "decision") != Some("sync-backend-insufficient")
    {
        problems.push(
            "notification feasibility must remain diagnostic, non-promotable, and insufficient for sync migration"
                .to_owned(),
        );
    }
    if text(value, "source_sha").is_none_or(|sha| !full_sha(sha)) {
        problems.push("notification feasibility source_sha is not a full SHA".to_owned());
    }
    for field in ["binary_sha256", "receipt_sha256"] {
        if text(value, field).is_none_or(|digest| !sha256(digest)) {
            problems.push(format!("notification feasibility {field} is not SHA-256"));
        }
    }
    if integer(value, "operations_per_case").is_none_or(|count| count < 1024)
        || integer(value, "repetitions").is_none_or(|count| count < 3)
        || boolean(value, "counterbalanced") != Some(true)
    {
        problems.push(
            "notification feasibility requires three counterbalanced repetitions of at least 1024 operations"
                .to_owned(),
        );
    }

    let observations = value
        .get("median_observations")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    for (backend, operation) in [
        ("moka-future", "insert"),
        ("moka-future", "remove"),
        ("moka-sync", "insert"),
        ("moka-sync", "remove"),
    ] {
        if !observations.iter().any(|item| {
            text(item, "backend") == Some(backend) && text(item, "operation") == Some(operation)
        }) {
            problems.push(format!(
                "notification feasibility omits {backend} {operation}"
            ));
        }
    }
    for item in &observations {
        for field in [
            "listener_off_bytes_per_operation",
            "listener_noop_bytes_per_operation",
            "incremental_bytes_per_operation",
            "regression_fraction",
        ] {
            if float(item, field).is_none_or(|number| !number.is_finite() || number < 0.0) {
                problems.push(format!("notification feasibility has invalid {field}"));
            }
        }
    }
    for field in ["conclusion", "next_evidence"] {
        if text(value, field).is_none_or(str::is_empty) {
            problems.push(format!("notification feasibility requires {field}"));
        }
    }
    problems
}

pub fn check_notification_observer_requirements(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "contract_id") != Some("notification-observer-requirements-073-v1")
        || text(value, "state") != Some("product-integrated-local-screening-admitted")
        || text(value, "proposal_id") != Some("P73-INSTRUMENTATION-NONBLOCKING-REMOVAL")
        || text(value, "prototype_scope") != Some("completed-lab-prototype")
    {
        problems.push("notification observer requirements identity mismatch".to_owned());
    }
    if boolean(value, "d2_authorized") != Some(true)
        || boolean(value, "product_mutation_allowed") != Some(true)
        || boolean(value, "candidate_measurements_allowed") != Some(false)
        || boolean(value, "local_candidate_screening_allowed") != Some(true)
        || text(value, "review_status") != Some("d2-authorized-pinned-fork")
    {
        problems.push("notification observer requirements must bind D2 to the pinned fork while candidate measurement remains disabled".to_owned());
    }
    for field in [
        "selected_direction",
        "prototype_exit",
        "implementation_commit",
        "implementation_receipt",
        "next_evidence",
    ] {
        if text(value, field).is_none_or(str::is_empty) {
            problems.push(format!(
                "notification observer requirements require {field}"
            ));
        }
    }
    for (field, minimum) in [
        ("owner_split", 3),
        ("callback_constraints", 5),
        ("ordering_invariants", 5),
        ("delivery_invariants", 5),
        ("required_falsifiers", 6),
        ("compatibility_invariants", 4),
    ] {
        if string_array(value.get(field)).len() < minimum {
            problems.push(format!(
                "notification observer requirements have incomplete {field}"
            ));
        }
    }
    problems
}

pub fn check_notification_observer_prototype(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_class") != Some("local_feasibility")
        || text(value, "experiment") != Some("versioned-bounded-post-removal-observer")
    {
        problems.push("notification observer prototype identity mismatch".to_owned());
    }
    if boolean(value, "diagnostic_only") != Some(true)
        || boolean(value, "product_semantics_eligible") != Some(false)
        || boolean(value, "promotable") != Some(false)
        || text(value, "decision") != Some("reference-model-feasible-moka-seam-unproven")
    {
        problems.push(
            "notification observer prototype must remain diagnostic with the Moka seam unproven"
                .to_owned(),
        );
    }
    if text(value, "source_sha").is_none_or(|sha| !full_sha(sha)) {
        problems.push("notification observer prototype source_sha is not a full SHA".to_owned());
    }
    for field in ["binary_sha256", "receipt_sha256"] {
        if text(value, field).is_none_or(|digest| !sha256(digest)) {
            problems.push(format!(
                "notification observer prototype {field} is not SHA-256"
            ));
        }
    }
    if integer(value, "operations_per_case").is_none_or(|count| count < 1024)
        || integer(value, "repetitions").is_none_or(|count| count < 3)
        || boolean(value, "counterbalanced") != Some(true)
    {
        problems.push(
            "notification observer prototype requires three counterbalanced repetitions of at least 1024 operations"
                .to_owned(),
        );
    }
    let observations = value
        .get("median_observations")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    for path in ["atomic-counter-only", "versioned-observer"] {
        if !observations
            .iter()
            .any(|item| text(item, "path") == Some(path))
        {
            problems.push(format!("notification observer prototype omits {path}"));
        }
    }
    for item in &observations {
        for field in ["gross_allocated_bytes_per_operation", "elapsed_ns"] {
            if float(item, field).is_none_or(|number| !number.is_finite() || number < 0.0) {
                problems.push(format!(
                    "notification observer prototype has invalid {field}"
                ));
            }
        }
    }
    for field in ["conclusion", "next_evidence"] {
        if text(value, field).is_none_or(str::is_empty) {
            problems.push(format!("notification observer prototype requires {field}"));
        }
    }
    problems
}

pub fn check_moka_observer_spike(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_class") != Some("local_feasibility")
        || text(value, "experiment") != Some("moka-future-listener-vs-observer-key-lock-ablation")
    {
        problems.push("Moka observer spike identity mismatch".to_owned());
    }
    if boolean(value, "diagnostic_only") != Some(true)
        || boolean(value, "product_semantics_eligible") != Some(false)
        || boolean(value, "promotable") != Some(false)
        || text(value, "decision") != Some("key-lock-owner-confirmed-boxed-removal-cost-remains")
    {
        problems.push(
            "Moka observer spike must remain diagnostic with boxed removal cost unresolved"
                .to_owned(),
        );
    }
    if text(value, "source_sha").is_none_or(|sha| !full_sha(sha)) {
        problems.push("Moka observer spike source_sha is not a full SHA".to_owned());
    }
    for field in ["binary_sha256", "receipt_sha256", "patch_sha256"] {
        if text(value, field).is_none_or(|digest| !sha256(digest)) {
            problems.push(format!("Moka observer spike {field} is not SHA-256"));
        }
    }
    if integer(value, "operations_per_case").is_none_or(|count| count < 1024)
        || integer(value, "repetitions").is_none_or(|count| count < 3)
        || boolean(value, "counterbalanced") != Some(true)
    {
        problems.push(
            "Moka observer spike requires three counterbalanced repetitions of at least 1024 operations"
                .to_owned(),
        );
    }
    let causes: BTreeSet<_> = string_array(value.get("verified_causes"))
        .into_iter()
        .collect();
    for cause in ["explicit", "replaced", "expired", "size"] {
        if !causes.contains(cause) {
            problems.push(format!("Moka observer spike omits {cause} cause"));
        }
    }
    let observations = value
        .get("median_observations")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    for mode in ["off", "listener", "observer"] {
        for operation in ["insert", "remove"] {
            if !observations.iter().any(|item| {
                text(item, "mode") == Some(mode) && text(item, "operation") == Some(operation)
            }) {
                problems.push(format!("Moka observer spike omits {mode} {operation}"));
            }
        }
    }
    for item in &observations {
        for field in ["gross_allocated_bytes_per_operation", "elapsed_ns"] {
            if float(item, field).is_none_or(|number| !number.is_finite() || number < 0.0) {
                problems.push(format!("Moka observer spike has invalid {field}"));
            }
        }
    }
    for field in ["conclusion", "next_evidence"] {
        if text(value, field).is_none_or(str::is_empty) {
            problems.push(format!("Moka observer spike requires {field}"));
        }
    }
    problems
}

pub fn check_moka_observer_direct(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_class") != Some("local_feasibility")
        || text(value, "experiment") != Some("moka-direct-observer-with-versioned-cleanup")
    {
        problems.push("direct Moka observer identity mismatch".to_owned());
    }
    if boolean(value, "diagnostic_only") != Some(true)
        || boolean(value, "product_semantics_eligible") != Some(false)
        || boolean(value, "promotable") != Some(false)
        || boolean(value, "d2_authorized") != Some(false)
        || text(value, "decision") != Some("direct-observer-lab-feasible-d2-still-required")
    {
        problems
            .push("direct Moka observer must remain lab-only until D2 is authorized".to_owned());
    }
    if text(value, "source_sha").is_none_or(|sha| !full_sha(sha)) {
        problems.push("direct Moka observer source_sha is not a full SHA".to_owned());
    }
    for field in ["binary_sha256", "receipt_sha256", "patch_sha256"] {
        if text(value, field).is_none_or(|digest| !sha256(digest)) {
            problems.push(format!("direct Moka observer {field} is not SHA-256"));
        }
    }
    if integer(value, "operations_per_case").is_none_or(|count| count < 1024)
        || integer(value, "repetitions").is_none_or(|count| count < 3)
        || boolean(value, "counterbalanced") != Some(true)
    {
        problems.push(
            "direct Moka observer requires three counterbalanced repetitions of at least 1024 operations"
                .to_owned(),
        );
    }
    let causes: BTreeSet<_> = string_array(value.get("verified_causes"))
        .into_iter()
        .collect();
    for cause in ["explicit", "replaced", "expired", "size"] {
        if !causes.contains(cause) {
            problems.push(format!("direct Moka observer omits {cause} cause"));
        }
    }
    if boolean(value, "versioned_replacement_ordering_verified") != Some(true) {
        problems
            .push("direct Moka observer requires versioned replacement ordering proof".to_owned());
    }
    let observations = value
        .get("median_observations")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    for mode in ["off", "listener", "observer"] {
        for operation in ["insert", "remove"] {
            if !observations.iter().any(|item| {
                text(item, "mode") == Some(mode) && text(item, "operation") == Some(operation)
            }) {
                problems.push(format!("direct Moka observer omits {mode} {operation}"));
            }
        }
    }
    for item in &observations {
        for field in ["gross_allocated_bytes_per_operation", "elapsed_ns"] {
            if float(item, field).is_none_or(|number| !number.is_finite() || number < 0.0) {
                problems.push(format!("direct Moka observer has invalid {field}"));
            }
        }
    }
    for field in ["conclusion", "next_evidence"] {
        if text(value, field).is_none_or(str::is_empty) {
            problems.push(format!("direct Moka observer requires {field}"));
        }
    }
    problems
}

pub fn check_notification_observer_d2_review(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "proposal_id") != Some("P73-INSTRUMENTATION-NONBLOCKING-REMOVAL")
        || text(value, "review_packet_id") != Some("notification-observer-d2-review-v1")
        || text(value, "state") != Some("d2-authorized-pinned-fork")
    {
        problems.push("notification observer D2 review candidate identity mismatch".to_owned());
    }
    if text(value, "reviewer") != Some("project-maintainer-self-review")
        || boolean(value, "reviewer_independent") != Some(false)
        || boolean(value, "d2_authorized") != Some(true)
        || boolean(value, "thresholds_frozen") != Some(true)
        || boolean(value, "candidate_measurements_allowed") != Some(false)
        || boolean(value, "product_mutation_allowed") != Some(true)
        || boolean(value, "candidate_data_used_for_thresholds") != Some(false)
    {
        problems.push(
            "notification observer single-maintainer review must authorize only product integration and keep candidate measurement disabled"
                .to_owned(),
        );
    }
    for (field, expected) in [
        (
            "review_mode",
            "single-maintainer-with-compensating-controls",
        ),
        ("review_policy", SINGLE_MAINTAINER_REVIEW_POLICY),
        (
            "threshold_freeze_commit",
            "1e9fd748f1a234967e9303c7071dd0816fb7ac28",
        ),
        ("reviewed_at", "2026-09-24"),
        (
            "review_decision",
            "thresholds-accepted-pinned-fork-authorized",
        ),
    ] {
        if text(value, field) != Some(expected) {
            problems.push(format!(
                "notification observer single-maintainer review {field} must be {expected}"
            ));
        }
    }
    if text(value, "primary_metric") != Some("fill_allocated_bytes_per_operation")
        || float(value, "minimum_practical_improvement_fraction")
            .is_none_or(|number| (number - 0.15).abs() > f64::EPSILON)
    {
        problems.push("notification observer D2 review primary threshold changed".to_owned());
    }
    for (field, expected) in [
        ("dependency_preference", "upstream-first"),
        ("dependency_fallback", "reviewed-pinned-fork"),
        ("dependency_decision", MOKA_FORK_DECISION),
        ("dependency_review_draft", MOKA_OBSERVER_UPSTREAM_DRAFT),
    ] {
        if text(value, field) != Some(expected) {
            problems.push(format!(
                "notification observer D2 review {field} must be {expected}"
            ));
        }
    }
    for field in [
        "author_role",
        "compatibility_outcome",
        "rollback_class",
        "threshold_derivation",
        "reviewer_action",
        "implementation_scope_adjudication",
        "implementation_receipt",
    ] {
        if text(value, field).is_none_or(str::is_empty) {
            problems.push(format!("notification observer D2 review requires {field}"));
        }
    }
    for (field, minimum) in [
        ("baseline_evidence", 4),
        ("candidate_evidence_excluded_from_threshold_derivation", 3),
        ("authorized_files_if_d2_approved", 6),
        ("authorized_surfaces_if_d2_approved", 4),
        ("required_correctness_falsifiers", 10),
        ("required_dependency_review", 4),
        ("required_d3_measurements", 7),
    ] {
        if string_array(value.get(field)).len() < minimum {
            problems.push(format!(
                "notification observer D2 review has incomplete {field}"
            ));
        }
    }
    let baseline: BTreeSet<_> = string_array(value.get("baseline_evidence"))
        .into_iter()
        .collect();
    let excluded: BTreeSet<_> =
        string_array(value.get("candidate_evidence_excluded_from_threshold_derivation"))
            .into_iter()
            .collect();
    if !baseline.is_disjoint(&excluded) {
        problems.push(
            "notification observer D2 review mixes candidate evidence into threshold derivation"
                .to_owned(),
        );
    }
    let falsifiers = string_array(value.get("required_correctness_falsifiers"));
    for required in ["panic", "reentrancy"] {
        if !falsifiers.iter().any(|item| item.contains(required)) {
            problems.push(format!(
                "notification observer D2 review omits {required} falsifier"
            ));
        }
    }
    let thresholds = value
        .get("threshold_proposal")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    let fill = thresholds
        .iter()
        .find(|item| text(item, "metric") == Some("fill_allocated_bytes_per_operation"));
    if fill.is_none_or(|item| {
        text(item, "role") != Some("primary")
            || text(item, "direction") != Some("lower")
            || float(item, "minimum_relative_improvement")
                .is_none_or(|number| (number - 0.15).abs() > f64::EPSILON)
    }) {
        problems.push("notification observer D2 review has invalid fill threshold".to_owned());
    }
    let allocations = thresholds
        .iter()
        .find(|item| text(item, "metric") == Some("unaffected_allocated_bytes_per_operation"));
    if allocations.is_none_or(|item| {
        float(item, "maximum_relative_regression")
            .is_none_or(|number| (number - 0.03).abs() > f64::EPSILON)
            || integer(item, "absolute_noise_floor_bytes_per_operation") != Some(16)
    }) {
        problems.push("notification observer D2 review has invalid allocation guard".to_owned());
    }
    let rss = thresholds
        .iter()
        .find(|item| text(item, "metric") == Some("post_idle_and_peak_rss_delta_bytes"));
    if rss.is_none_or(|item| {
        float(item, "maximum_relative_regression")
            .is_none_or(|number| (number - 0.05).abs() > f64::EPSILON)
            || integer(item, "absolute_noise_floor_bytes") != Some(1_048_576)
    }) {
        problems.push("notification observer D2 review has invalid RSS guard".to_owned());
    }
    problems
}

pub fn check_single_maintainer_review_policy(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "policy_id") != Some("single-maintainer-review-073-v1")
        || text(value, "state") != Some("active")
        || text(value, "scope") != Some("P73-INSTRUMENTATION-NONBLOCKING-REMOVAL")
        || text(value, "review_mode") != Some("single-maintainer-with-compensating-controls")
    {
        problems.push("single-maintainer review policy identity mismatch".to_owned());
    }
    if boolean(value, "independent_reviewer_required") != Some(false)
        || boolean(value, "independent_review_claim_allowed") != Some(false)
        || boolean(value, "thresholds_accepted") != Some(true)
        || text(value, "dependency_decision") != Some(MOKA_FORK_DECISION)
        || boolean(value, "d2_authorized") != Some(true)
        || boolean(value, "product_mutation_allowed") != Some(true)
    {
        problems.push(
            "single-maintainer policy must bind D2 to the reviewed dependency without claiming independence"
                .to_owned(),
        );
    }
    for (field, expected) in [
        (
            "threshold_freeze_commit",
            "1e9fd748f1a234967e9303c7071dd0816fb7ac28",
        ),
        (
            "dependency_review_draft_commit",
            "fb1e850d536c0ed970f5043f09ad3ce3582704f0",
        ),
    ] {
        if text(value, field) != Some(expected) {
            problems.push(format!(
                "single-maintainer policy {field} must be {expected}"
            ));
        }
    }
    if text(value, "authorization_source").is_none_or(str::is_empty) {
        problems.push("single-maintainer policy requires authorization_source".to_owned());
    }
    for (field, minimum) in [
        ("compensating_controls", 10),
        ("decision_sequence", 6),
        ("forbidden_shortcuts", 5),
    ] {
        if string_array(value.get(field)).len() < minimum {
            problems.push(format!("single-maintainer policy has incomplete {field}"));
        }
    }
    let controls = string_array(value.get("compensating_controls"));
    for required in [
        "threshold",
        "candidate evidence",
        "separate",
        "append-only",
        "dedicated host",
        "independently reviewed",
        "panic",
        "rollback",
    ] {
        if !controls.iter().any(|item| item.contains(required)) {
            problems.push(format!(
                "single-maintainer policy omits {required} compensating control"
            ));
        }
    }
    problems
}

pub fn check_moka_fork_decision(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "decision_id") != Some("moka-fork-352e53fa-073-v1")
        || text(value, "proposal_id") != Some("P73-INSTRUMENTATION-NONBLOCKING-REMOVAL")
        || text(value, "state") != Some("d2-authorized")
        || text(value, "decision") != Some("reviewed-pinned-fork")
        || text(value, "review_mode") != Some("single-maintainer-with-compensating-controls")
        || text(value, "review_policy") != Some(SINGLE_MAINTAINER_REVIEW_POLICY)
    {
        problems.push("Moka fork decision identity mismatch".to_owned());
    }
    if boolean(value, "d2_authorized") != Some(true)
        || boolean(value, "product_mutation_allowed") != Some(true)
        || boolean(value, "candidate_measurements_allowed") != Some(false)
    {
        problems.push(
            "Moka fork decision must authorize product integration but not candidate measurement"
                .to_owned(),
        );
    }

    let missing = TomlValue::Boolean(false);
    let source = value.get("source").unwrap_or(&missing);
    for (field, expected) in [
        ("repository", "https://github.com/javaquasar/moka.git"),
        ("branch", "hydracache/post-removal-observer-0.12.15"),
        ("revision", "352e53faa480c9997272b9c70798dd5b5c15d581"),
        (
            "upstream_revision",
            "616473ee923f4cd1429b3d8eb3be7df3eb9906b1",
        ),
        (
            "prototype_patch",
            "docs/testing/performance/0.73/moka-post-removal-observer-direct-0.12.15.patch",
        ),
    ] {
        if text(source, field) != Some(expected) {
            problems.push(format!(
                "Moka fork decision source {field} must be {expected}"
            ));
        }
    }
    for field in [
        "revision",
        "tree",
        "upstream_revision",
        "upstream_tree",
        "stable_patch_id",
    ] {
        if text(source, field).is_none_or(|digest| !full_sha(digest)) {
            problems.push(format!(
                "Moka fork decision source {field} is not a full digest"
            ));
        }
    }
    if text(source, "prototype_patch_sha256").is_none_or(|digest| !sha256(digest))
        || boolean(source, "remote_revision_verified") != Some(true)
    {
        problems.push("Moka fork decision source integrity is incomplete".to_owned());
    }

    let upstream = value.get("upstream").unwrap_or(&missing);
    if text(upstream, "proposal") != Some(MOKA_OBSERVER_UPSTREAM_DRAFT)
        || text(upstream, "submission_state") != Some("not-submitted")
        || text(upstream, "maintainer_disposition") != Some("not-requested")
        || text(upstream, "decision").is_none_or(str::is_empty)
        || text(upstream, "rationale").is_none_or(str::is_empty)
    {
        problems.push("Moka fork decision must disclose the absent upstream review".to_owned());
    }

    let maintenance = value.get("maintenance").unwrap_or(&missing);
    for field in [
        "repository_owner",
        "integration_owner",
        "upstream_sync_cadence",
        "advisory_cadence",
        "update_policy",
        "rollback",
    ] {
        if text(maintenance, field).is_none_or(str::is_empty) {
            problems.push(format!("Moka fork decision maintenance requires {field}"));
        }
    }

    let compatibility = value.get("compatibility").unwrap_or(&missing);
    if text(compatibility, "license_expression") != Some("(MIT OR Apache-2.0) AND Apache-2.0")
        || text(compatibility, "msrv") != Some("1.71.1")
        || string_array(compatibility.get("selected_features")) != ["future"]
        || boolean(compatibility, "observer_public_api_exposed_by_hydracache") != Some(false)
        || boolean(compatibility, "default_hydracache_behavior_changed") != Some(false)
    {
        problems.push("Moka fork decision compatibility contract changed".to_owned());
    }

    let supply_chain = value.get("supply_chain").unwrap_or(&missing);
    for field in ["cargo_lock_sha256", "sbom_sha256"] {
        if text(supply_chain, field).is_none_or(|digest| !sha256(digest)) {
            problems.push(format!("Moka fork decision {field} is not SHA-256"));
        }
    }
    if text(supply_chain, "cargo_deny_result")
        != Some("advisories ok, bans ok, licenses ok, sources ok")
        || text(supply_chain, "sbom_format") != Some("CycloneDX 1.5 JSON")
    {
        problems.push("Moka fork decision supply-chain gate is incomplete".to_owned());
    }
    if string_array(value.get("validation")).len() < 9
        || string_array(value.get("authorization_limits")).len() < 4
    {
        problems.push("Moka fork decision omits validation or authorization limits".to_owned());
    }
    problems
}

pub fn check_notification_observer_product(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_id") != Some("notification-observer-product-73fc38a1-v1")
        || text(value, "proposal_id") != Some("P73-INSTRUMENTATION-NONBLOCKING-REMOVAL")
        || text(value, "state") != Some("local-correctness-admitted")
        || text(value, "implementation_commit") != Some("73fc38a131d26e78b246fe93d5edd71d33796bbf")
        || text(value, "implementation_parent") != Some("03f893541f6386cbf026c496250e28950c05e0ab")
        || text(value, "dependency_revision") != Some("352e53faa480c9997272b9c70798dd5b5c15d581")
    {
        problems.push("notification observer product admission identity mismatch".to_owned());
    }
    if text(value, "review_mode") != Some("single-maintainer-with-compensating-controls")
        || boolean(value, "promotable") != Some(false)
        || boolean(value, "numerical_claim_eligible") != Some(false)
        || boolean(value, "local_candidate_screening_allowed") != Some(true)
        || boolean(value, "dedicated_candidate_measurement_allowed") != Some(false)
        || boolean(value, "thresholds_changed") != Some(false)
        || boolean(value, "public_api_changed") != Some(false)
        || boolean(value, "instrumentation_off_observer_attached") != Some(false)
        || boolean(value, "rollback_verified") != Some(true)
    {
        problems.push(
            "product admission must open only non-promotable local screening without changing thresholds or public behavior"
                .to_owned(),
        );
    }
    if integer(value, "observer_queue_capacity") != Some(4_096)
        || integer(value, "cache_entry_inline_bytes") != Some(72)
    {
        problems
            .push("product admission observer bounds or frozen entry layout changed".to_owned());
    }
    for field in [
        "scope_variance",
        "shutdown_conclusion",
        "reentrancy_conclusion",
        "next_evidence",
    ] {
        if text(value, field).is_none_or(str::is_empty) {
            problems.push(format!("product admission requires {field}"));
        }
    }
    let expected_files: BTreeSet<_> = [
        "Cargo.lock",
        "Cargo.toml",
        "crates/hydracache/src/builder.rs",
        "crates/hydracache/src/cache.rs",
        "crates/hydracache/src/entry.rs",
        "crates/hydracache/src/lib.rs",
        "crates/hydracache/src/memory_footprint.rs",
        "crates/hydracache/src/removal_observer.rs",
        "crates/hydracache/src/tag_index.rs",
        "crates/hydracache/src/tests/local_cache.rs",
        "crates/hydracache/tests/memory_footprint_071.rs",
        "deny.toml",
    ]
    .into_iter()
    .collect();
    let changed_files: BTreeSet<_> = string_array(value.get("changed_files"))
        .into_iter()
        .collect();
    if changed_files != expected_files {
        problems.push(
            "product admission changed-file ledger does not match implementation commit".to_owned(),
        );
    }
    if string_array(value.get("validation")).len() < 15 {
        problems.push("product admission validation matrix is incomplete".to_owned());
    }
    let falsifiers = value
        .get("falsifier")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    for id in [
        "explicit-replace-invalidate-flush",
        "ttl-expiry",
        "capacity-eviction",
        "delayed-replacement-cleanup",
        "duplicate-delivery",
        "queue-saturation",
        "pending-exact-barrier",
        "cancellation-gap",
        "panic-containment",
        "reentrancy",
        "shutdown",
        "default-compatibility",
        "rollback",
    ] {
        if !falsifiers.iter().any(|item| {
            text(item, "id") == Some(id)
                && text(item, "status") == Some("passed")
                && text(item, "evidence").is_some_and(|evidence| !evidence.is_empty())
        }) {
            problems.push(format!("product admission omits passing {id} falsifier"));
        }
    }
    problems
}

pub fn check_proposal_registry(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "registry_state") != Some("product_integrated_local_screening_admitted")
    {
        problems.push("0.73 proposal registry identity mismatch".to_owned());
    }
    if boolean(value, "candidate_measurements_allowed") != Some(false)
        || boolean(value, "local_candidate_screening_allowed") != Some(true)
        || boolean(value, "product_mutations_allowed") != Some(true)
    {
        problems.push(
            "integrated registry must allow only local non-promotable screening while dedicated candidate measurement remains disabled"
                .to_owned(),
        );
    }
    let proposals = value
        .get("proposals")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    let Some(proposal) = proposals.iter().find(|proposal| {
        text(proposal, "proposal_id") == Some("P73-INSTRUMENTATION-NONBLOCKING-REMOVAL")
    }) else {
        problems.push("proposal registry omits instrumentation redesign".to_owned());
        return problems;
    };
    if text(proposal, "state") != Some("product_integrated_local_screening_admitted")
        || boolean(proposal, "d2_authorized") != Some(true)
        || boolean(proposal, "product_mutation_allowed") != Some(true)
        || boolean(proposal, "candidate_measurements_allowed") != Some(false)
        || boolean(proposal, "local_candidate_screening_allowed") != Some(true)
        || text(proposal, "practical_minimum_effect")
            != Some("fill allocations improve by at least 15%")
        || text(proposal, "threshold_status")
            != Some(
                "frozen by single-maintainer review at commit 1e9fd748f1a234967e9303c7071dd0816fb7ac28",
            )
        || text(proposal, "review_status") != Some("d2-authorized-pinned-fork")
    {
        problems.push(
            "instrumentation redesign must remain bound to the authorized D2 dependency while candidate measurement is disabled"
                .to_owned(),
        );
    }
    for field in [
        "owner",
        "hypothesis",
        "primary_metric",
        "threshold_status",
        "compatibility_outcome",
        "rollback_class",
        "dependency_delta",
        "implementation_commit",
        "implementation_receipt",
        "next_evidence",
    ] {
        if text(proposal, field).is_none_or(str::is_empty) {
            problems.push(format!("instrumentation proposal requires {field}"));
        }
    }
    if text(proposal, "d2_review_candidate") != Some(NOTIFICATION_OBSERVER_D2_REVIEW) {
        problems.push("instrumentation proposal must reference its D2 review candidate".to_owned());
    }
    if text(proposal, "dependency_decision") != Some(MOKA_FORK_DECISION) {
        problems
            .push("instrumentation proposal must reference the pinned fork decision".to_owned());
    }
    if text(proposal, "implementation_receipt") != Some(NOTIFICATION_OBSERVER_PRODUCT)
        || text(proposal, "implementation_commit")
            != Some("73fc38a131d26e78b246fe93d5edd71d33796bbf")
    {
        problems.push(
            "instrumentation proposal must bind the admitted product implementation".to_owned(),
        );
    }
    for (field, minimum) in [
        ("baseline_evidence", 3),
        ("source_findings", 4),
        ("rejected_approaches", 5),
        ("candidate_options_requiring_d2", 2),
        ("required_correctness_tests", 6),
        ("required_regression_guards", 4),
    ] {
        if string_array(proposal.get(field)).len() < minimum {
            problems.push(format!("instrumentation proposal has incomplete {field}"));
        }
    }
    problems
}

pub fn check_statistics(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "contract_id") != Some("performance-statistics-073-v1")
        || text(value, "state") != Some("baseline-only-thresholds-frozen")
    {
        problems.push("0.73 statistics identity/state mismatch".to_owned());
    }
    if boolean(value, "baseline_only_derivation") != Some(true)
        || boolean(value, "candidate_may_amend") != Some(false)
        || boolean(value, "candidate_measurements_allowed") != Some(false)
        || boolean(value, "silent_retry_allowed") != Some(false)
    {
        problems.push("statistics must remain baseline-only before I73".to_owned());
    }
    for (field, expected) in [
        ("process_model", "independently-started"),
        ("pairing_method", "counterbalanced-seeded-v1"),
        ("multiple_comparison", "holm-bonferroni"),
        ("paired_estimator", "hodges-lehmann-v1"),
        ("slope_estimator", "theil-sen-v1"),
        ("bootstrap_method", "moving-block-v1"),
        ("allocation_limit_state", "frozen-single-maintainer-review"),
        ("rss_limit_state", "frozen-single-maintainer-review"),
        (
            "threshold_review_candidate",
            NOTIFICATION_OBSERVER_D2_REVIEW,
        ),
    ] {
        if text(value, field) != Some(expected) {
            problems.push(format!("statistics {field} must be {expected}"));
        }
    }
    if integer(value, "minimum_admitted_pairs").is_none_or(|count| count < 5)
        || integer(value, "bootstrap_iterations").is_none_or(|count| count < 1000)
        || integer(value, "bootstrap_block_samples").is_none_or(|count| count < 2)
        || float(value, "confidence_level").is_none_or(|level| !(0.95..1.0).contains(&level))
    {
        problems.push("statistics weakens sample or confidence requirements".to_owned());
    }
    let budgets = value
        .get("regression_budget")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    for (metric, maximum) in [
        ("goodput_operations_per_second", 0.02),
        ("cpu_seconds_per_operation", 0.03),
        ("p99_latency_seconds", 0.03),
    ] {
        let valid = budgets.iter().any(|budget| {
            text(budget, "metric") == Some(metric)
                && float(budget, "maximum_relative_regression") == Some(maximum)
        });
        if !valid {
            problems.push(format!("statistics weakens or omits {metric} budget"));
        }
    }
    let invalidating = value
        .get("invalidating_condition")
        .and_then(TomlValue::as_array)
        .map_or(0, Vec::len);
    if invalidating < 4 {
        problems.push("statistics requires all four invalidating conditions".to_owned());
    }
    problems
}

pub fn check_host_profile(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "profile_id") != Some("performance-reference-073-v1")
        || text(value, "state") != Some("template-unadmitted")
    {
        problems.push("0.73 host profile identity/state mismatch".to_owned());
    }
    for field in [
        "eligible",
        "candidate_measurements_allowed",
        "identity_reuse_from_071_allowed",
        "local_or_shared_runner_promotable",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!("unadmitted host profile {field} must be false"));
        }
    }
    for field in [
        "dedicated_bare_metal_required",
        "serialized_lease_required",
        "pre_calibration_required",
        "post_calibration_required",
        "completed_bootstrap_admission_required",
    ] {
        if boolean(value, field) != Some(true) {
            problems.push(format!("host profile {field} must be true"));
        }
    }
    if float(value, "calibration_max_relative_spread")
        .is_none_or(|spread| spread <= 0.0 || spread > 0.10)
    {
        problems.push("host profile calibration spread must be in (0, 0.10]".to_owned());
    }
    for (field, minimum) in [
        ("immutable_probes", 8),
        ("mutable_probes", 8),
        ("required_tools", 6),
        ("companion_platforms", 2),
        ("admission_blockers", 5),
    ] {
        if string_array(value.get(field)).len() < minimum {
            problems.push(format!("host profile has incomplete {field}"));
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
    if text(value, "allocation_regression_limit_state") != Some("frozen-single-maintainer-review")
        || text(value, "rss_delta_limit_state") != Some("frozen-single-maintainer-review")
    {
        problems.push(
            "allocation/RSS limits must remain bound to the single-maintainer threshold review"
                .to_owned(),
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
