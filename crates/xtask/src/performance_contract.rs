use serde_json::Value as JsonValue;
use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use toml::Value as TomlValue;

const CONTRACT: &str = "docs/testing/performance/0.73/local-screening.toml";
const RECEIPT_SCHEMA: &str = "docs/testing/performance/0.73/local-screening-receipt-v1.schema.json";
const EXAMPLE_RECEIPT: &str = "docs/testing/performance/0.73/local-screening-receipt.example.json";
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
    let mut problems = check_contract(&contract, release);
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
