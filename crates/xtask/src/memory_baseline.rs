use serde::Serialize;
use serde_json::{json, Value as JsonValue};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const IDENTITIES: &str = "docs/testing/memory/0.71/baseline-identities.toml";
const SCENARIO: &str = "docs/testing/perf-scenarios/0.71/memory-efficiency-v1.toml";
const HISTORICAL_REQUIREMENTS: &str = "docs/testing/memory/0.71/historical-input-requirements.toml";
const HISTORICAL_RECEIPT: &str = "target/memory-evidence/0.71/historical-input-receipt.json";
const EXPECTED_ARCHIVE_COMMIT: &str = "dbc2f82f7f303528b3cca7842818730c82232b9c";
const B0_SHA: &str = "75719b0bf5de2250cf4eb16a30073dd7429538e3";
const B1_SHA: &str = "795f9493bcbb7a56aa229c59e4a717f60c654cdb";

#[derive(Debug)]
struct Options {
    root: PathBuf,
    release: String,
    require_d0: bool,
    output: Option<PathBuf>,
}

impl Options {
    fn parse(args: Vec<String>) -> Result<Self, Box<dyn Error>> {
        let root = std::env::current_dir()?;
        let mut release = None;
        let mut require_d0 = false;
        let mut output = None;
        let mut args = args.into_iter();
        while let Some(flag) = args.next() {
            match flag.as_str() {
                "--release" => release = args.next(),
                "--require-d0" => require_d0 = true,
                "--output" => output = args.next().map(PathBuf::from),
                _ => return Err(format!("unknown memory baseline option: {flag}").into()),
            }
        }
        let release = release.ok_or("--release is required")?;
        if release != "0.71" {
            return Err("memory baseline contract exists only for release 0.71".into());
        }
        Ok(Self {
            root,
            release,
            require_d0,
            output,
        })
    }
}

pub fn run_check(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let options = Options::parse(args)?;
    let problems = check_contract(&options.root, options.require_d0)?;
    if problems.is_empty() {
        println!(
            "memory-baseline-check: OK (release {}, structural baseline contract)",
            options.release
        );
        Ok(())
    } else {
        Err(format!("memory-baseline-check failed:\n- {}", problems.join("\n- ")).into())
    }
}

pub fn run_status(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let options = Options::parse(args)?;
    let output = options
        .output
        .unwrap_or_else(|| PathBuf::from("target/memory-evidence/0.71/baseline-status.json"));
    let structural_problems = check_contract(&options.root, false)?;
    if !structural_problems.is_empty() {
        return Err(format!(
            "cannot write baseline status with invalid contracts:\n- {}",
            structural_problems.join("\n- ")
        )
        .into());
    }
    if options.require_d0 {
        let d0_problems = check_contract(&options.root, true)?;
        if !d0_problems.is_empty() {
            return Err(format!(
                "D0 baseline is not admissible:\n- {}",
                d0_problems.join("\n- ")
            )
            .into());
        }
    }
    let dirty_paths = git_lines(&options.root, &["status", "--short"])?;
    let source_sha = git_text(&options.root, &["rev-parse", "HEAD"])?;
    let blockers = d0_blockers(&options.root, &dirty_paths)?;
    let status = BaselineStatus {
        schema_version: 1,
        release: options.release,
        source_sha,
        b0_sha: B0_SHA.to_owned(),
        b1_sha: B1_SHA.to_owned(),
        scenario_sha256: sha256_file(&options.root.join(SCENARIO))?,
        platform: std::env::consts::OS.to_owned(),
        diagnostic_only: true,
        ship_evidence_eligible: false,
        d0_ready: blockers.is_empty(),
        dirty_paths,
        blockers,
    };
    let status_value = serde_json::to_value(&status)?;
    let mut schema_problems = Vec::new();
    validate_json_schema(
        &status_value,
        include_str!("../../../docs/testing/memory/0.71/memory-baseline-status-v1.schema.json"),
        "memory baseline status",
        &mut schema_problems,
    );
    if !schema_problems.is_empty() {
        return Err(format!(
            "refusing to write an invalid baseline status:\n- {}",
            schema_problems.join("\n- ")
        )
        .into());
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output, serde_json::to_vec_pretty(&status_value)?)?;
    println!(
        "memory-baseline-status: wrote non-promotable status to {}",
        output.display()
    );
    Ok(())
}

pub fn run_report_check(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let mut release = None;
    let mut report = None;
    let mut allow_diagnostic_source = false;
    let mut arguments = args.into_iter();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--release" => release = arguments.next(),
            "--report" => report = arguments.next().map(PathBuf::from),
            "--allow-diagnostic-source" => allow_diagnostic_source = true,
            _ => {
                return Err(
                    format!("unknown memory-baseline-report-check argument: {argument}").into(),
                )
            }
        }
    }
    if release.as_deref() != Some("0.71") {
        return Err("memory-baseline-report-check requires --release 0.71".into());
    }
    let report = report.ok_or("memory-baseline-report-check requires --report")?;
    let value: JsonValue = serde_json::from_slice(&fs::read(&report)?)?;
    let problems = if allow_diagnostic_source {
        validate_diagnostic_baseline_report(&value)
    } else {
        validate_baseline_report(&value)
    };
    if !problems.is_empty() {
        return Err(format!(
            "memory baseline report failed:\n- {}",
            problems.join("\n- ")
        )
        .into());
    }
    println!("memory-baseline-report-check: OK ({})", report.display());
    Ok(())
}

#[derive(Debug, Serialize)]
struct BaselineStatus {
    schema_version: u32,
    release: String,
    source_sha: String,
    b0_sha: String,
    b1_sha: String,
    scenario_sha256: String,
    platform: String,
    diagnostic_only: bool,
    ship_evidence_eligible: bool,
    d0_ready: bool,
    dirty_paths: Vec<String>,
    blockers: Vec<String>,
}

pub fn check_contract(root: &Path, require_d0: bool) -> Result<Vec<String>, Box<dyn Error>> {
    let identities = load_toml(&root.join(IDENTITIES))?;
    let scenario = load_toml(&root.join(SCENARIO))?;
    let historical = load_toml(&root.join(HISTORICAL_REQUIREMENTS))?;
    let mut problems = Vec::new();
    check_identities(root, &identities, &mut problems)?;
    check_scenario(&scenario, &mut problems);
    check_historical_requirements(root, &historical, &mut problems)?;
    if require_d0 {
        let dirty = git_lines(root, &["status", "--short"])?;
        problems.extend(d0_blockers(root, &dirty)?);
        let receipt_path = root.join(HISTORICAL_RECEIPT);
        if receipt_path.is_file() {
            let receipt: JsonValue = serde_json::from_slice(&fs::read(receipt_path)?)?;
            problems.extend(validate_historical_receipt(&receipt));
        }
    }
    Ok(problems)
}

fn check_identities(
    root: &Path,
    identities: &toml::Value,
    problems: &mut Vec<String>,
) -> Result<(), Box<dyn Error>> {
    if integer_at(identities, &["schema_version"]) != Some(1)
        || string_at(identities, &["release"]) != Some("0.71")
    {
        problems.push("baseline identities require schema 1 and release 0.71".to_owned());
    }
    let b0 = string_at(identities, &["b0_release", "source_sha"]).unwrap_or_default();
    let b1 = string_at(identities, &["b1_instrumented", "source_sha"]).unwrap_or_default();
    if b0 == b1 || !is_sha(b0) || !is_sha(b1) {
        problems.push("B0 and B1 must be distinct exact commit SHAs".to_owned());
    }
    let tag_commit = git_text(root, &["rev-list", "-n", "1", "v0.70.0"])
        .unwrap_or_else(|_| "unavailable".to_owned());
    if tag_commit != b0 {
        problems.push(format!(
            "v0.70.0 resolves to {tag_commit}, expected B0 {b0}"
        ));
    }
    if !git_success(root, &["cat-file", "-e", &format!("{b1}^{{commit}}")]) {
        problems.push("B1 commit is absent from the object database".to_owned());
    }
    if !git_success(root, &["merge-base", "--is-ancestor", b1, "HEAD"]) {
        problems.push("B1 is not an ancestor of the baseline contract commit".to_owned());
    }
    let scenario_path = root.join(SCENARIO);
    let expected_digest = string_at(identities, &["scenario", "sha256"]).unwrap_or_default();
    if sha256_file(&scenario_path)? != expected_digest {
        problems.push("scenario digest differs from the frozen baseline identity".to_owned());
    }
    let scenario_inputs = identities
        .get("scenario")
        .and_then(|scenario| scenario.get("input"))
        .and_then(toml::Value::as_array)
        .cloned()
        .unwrap_or_default();
    if scenario_inputs.len() != 11 {
        problems.push("scenario cohort must freeze exactly eleven prerequisite inputs".to_owned());
    }
    for input in scenario_inputs {
        let Some(path) = input.get("path").and_then(toml::Value::as_str) else {
            problems.push("scenario input omits its path".to_owned());
            continue;
        };
        let expected = input
            .get("sha256")
            .and_then(toml::Value::as_str)
            .unwrap_or_default();
        match sha256_file(&root.join(path)) {
            Ok(actual) if actual == expected => {}
            Ok(_) => problems.push(format!("scenario input digest changed: {path}")),
            Err(error) => problems.push(format!("scenario input is unavailable: {path}: {error}")),
        }
    }
    if bool_at(identities, &["cohort_pooling"]) != Some(false)
        || bool_at(identities, &["candidate_derived_thresholds"]) != Some(false)
    {
        problems
            .push("baseline identities permit cohort pooling or candidate thresholds".to_owned());
    }
    Ok(())
}

fn check_scenario(value: &toml::Value, problems: &mut Vec<String>) {
    if string_at(value, &["scenario_id"]) != Some("memory-efficiency-v1") {
        problems.push("unexpected memory scenario id".to_owned());
    }
    let phases = string_array(value.get("phases"));
    let expected = [
        "cold",
        "fill",
        "steady",
        "expire_or_delete",
        "reset",
        "refill",
        "post_idle",
        "shutdown",
    ];
    if phases != expected {
        problems.push("memory scenario phases are missing or reordered".to_owned());
    }
    let cases = value
        .get("case")
        .and_then(toml::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let ids = cases
        .iter()
        .filter_map(|case| case.get("id").and_then(toml::Value::as_str))
        .collect::<Vec<_>>();
    let expected_ids = (0..=10)
        .map(|index| format!("M{index}-"))
        .collect::<Vec<_>>();
    if ids.len() != 11
        || ids
            .iter()
            .zip(expected_ids)
            .any(|(actual, prefix)| !actual.starts_with(&prefix))
    {
        problems.push("scenario must contain exactly M0 through M10 in order".to_owned());
    }
    let ttl = cases
        .iter()
        .find(|case| case.get("id").and_then(toml::Value::as_str) == Some("M3-ttl"));
    if ttl
        .and_then(|case| case.get("final_checkpoint"))
        .and_then(toml::Value::as_str)
        != Some("post_idle")
    {
        problems.push("M3-ttl omits the corrected post-idle final checkpoint".to_owned());
    }
    if bool_at(value, &["logical", "unique_keys_independent_from_requests"]) != Some(true) {
        problems.push("unique-key cardinality is not independently verified".to_owned());
    }
    if bool_at(value, &["allocator", "rss_substitution_forbidden"]) != Some(true) {
        problems.push("scenario permits RSS substitution for allocator fields".to_owned());
    }
}

pub fn validate_scenario_text(text: &str) -> Vec<String> {
    match toml::from_str(text) {
        Ok(value) => {
            let mut problems = Vec::new();
            check_scenario(&value, &mut problems);
            problems
        }
        Err(error) => vec![format!("memory scenario is invalid TOML: {error}")],
    }
}

fn check_historical_requirements(
    root: &Path,
    value: &toml::Value,
    problems: &mut Vec<String>,
) -> Result<(), Box<dyn Error>> {
    if string_at(value, &["commit"]) != Some(EXPECTED_ARCHIVE_COMMIT) {
        problems.push("historical archive commit is not the immutable plan commit".to_owned());
    }
    if !git_success(
        root,
        &[
            "cat-file",
            "-e",
            &format!("{EXPECTED_ARCHIVE_COMMIT}^{{commit}}"),
        ],
    ) {
        problems.push("historical archive commit is absent locally".to_owned());
    }
    let tag_commit = git_text(
        root,
        &["rev-list", "-n", "1", "explore-0.67-telemetry-20260803"],
    )
    .unwrap_or_default();
    if tag_commit != EXPECTED_ARCHIVE_COMMIT {
        problems.push("historical archive tag resolves to a different commit".to_owned());
    }
    if bool_at(value, &["missing_mirror_blocks_d0"]) != Some(true)
        || bool_at(value, &["ordinary_clone_is_protected_mirror"]) != Some(false)
    {
        problems.push("historical mirror boundary is fail-open".to_owned());
    }
    Ok(())
}

fn d0_blockers(root: &Path, dirty: &[String]) -> Result<Vec<String>, Box<dyn Error>> {
    let mut blockers = Vec::new();
    let identities = load_toml(&root.join(IDENTITIES))?;
    if bool_at(&identities, &["d0", "ready"]) != Some(true) {
        blockers.push("D0 transition is not approved in baseline-identities.toml".to_owned());
    }
    if !dirty.is_empty() {
        blockers.push("worktree is dirty; a baseline process identity cannot be frozen".to_owned());
    }
    if std::env::consts::OS != "linux" {
        blockers.push("dedicated-host D0 evidence requires Linux".to_owned());
    }
    for (path, label) in [
        (
            "target/test-evidence/0.67.1/reference-activation.json",
            "completed 0.67.1 bootstrap activation",
        ),
        (
            "target/memory-evidence/0.71/host-preflight.json",
            "admitted S7 host receipt",
        ),
        (HISTORICAL_RECEIPT, "protected historical mirror receipt"),
        (
            "target/memory-evidence/0.71/instrumentation-overhead.json",
            "S5 instrumentation overhead receipt",
        ),
    ] {
        if !root.join(path).is_file() {
            blockers.push(format!("missing {label}: {path}"));
        }
    }
    Ok(blockers)
}

pub fn validate_worktree_identity(expected: &str, actual: &str, dirty: &[String]) -> Vec<String> {
    let mut problems = Vec::new();
    if expected != actual {
        problems.push("baseline source SHA differs from B1".to_owned());
    }
    if !dirty.is_empty() {
        problems.push("dirty worktree cannot identify an immutable baseline".to_owned());
    }
    problems
}

pub fn validate_fresh_processes(processes: &[(u64, bool)]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut problems = Vec::new();
    for (pid, fresh) in processes {
        if !fresh || !seen.insert(*pid) {
            problems.push(format!("baseline process {pid} is reused or not fresh"));
        }
    }
    problems
}

pub fn validate_historical_receipt(receipt: &JsonValue) -> Vec<String> {
    let mut problems = Vec::new();
    validate_json_schema(
        receipt,
        include_str!("../../../docs/testing/memory/0.71/historical-input-receipt-v1.schema.json"),
        "historical input receipt",
        &mut problems,
    );
    if receipt.get("commit").and_then(JsonValue::as_str) != Some(EXPECTED_ARCHIVE_COMMIT) {
        problems.push("historical receipt commit mismatch".to_owned());
    }
    if receipt.get("checkout_clean").and_then(JsonValue::as_bool) != Some(true) {
        problems.push("historical archive checkout is dirty".to_owned());
    }
    let mirror = receipt.get("mirror").unwrap_or(&JsonValue::Null);
    for field in [
        "provider",
        "object_id",
        "archive_sha256",
        "verified_at",
        "retention_deadline",
    ] {
        if mirror.get(field).and_then(JsonValue::as_str).is_none() {
            problems.push(format!("historical mirror omits {field}"));
        }
    }
    if mirror
        .get("byte_length")
        .and_then(JsonValue::as_u64)
        .is_none()
    {
        problems.push("historical mirror omits byte_length".to_owned());
    }
    if mirror.get("manifest_sha256") != mirror.get("restored_manifest_sha256") {
        problems.push("restored historical mirror manifest mismatch".to_owned());
    }
    let files = receipt
        .get("files")
        .and_then(JsonValue::as_array)
        .cloned()
        .unwrap_or_default();
    if files.is_empty() {
        problems.push("historical receipt has no raw file manifest".to_owned());
    }
    for file in files {
        if file.get("path").and_then(JsonValue::as_str).is_none()
            || file.get("bytes").and_then(JsonValue::as_u64).is_none()
            || !file
                .get("sha256")
                .and_then(JsonValue::as_str)
                .is_some_and(|digest| digest.starts_with("sha256:") && digest.len() == 71)
        {
            problems.push("historical raw file manifest is incomplete".to_owned());
        }
    }
    problems
}

pub fn validate_baseline_report(report: &JsonValue) -> Vec<String> {
    let mut problems = Vec::new();
    validate_json_schema(
        report,
        include_str!("../../../docs/testing/memory/0.71/memory-baseline-report-v1.schema.json"),
        "memory baseline report",
        &mut problems,
    );
    let cohort = report.get("cohort").and_then(JsonValue::as_str);
    let source_sha = report.get("source_sha").and_then(JsonValue::as_str);
    let expected_sha = match cohort {
        Some("B0-release") => Some(B0_SHA),
        Some("B1-instrumented") => Some(B1_SHA),
        _ => None,
    };
    if expected_sha.is_some_and(|expected| source_sha != Some(expected)) {
        problems.push("baseline report source SHA does not match its frozen cohort".to_owned());
    }
    let unique_keys = report.get("unique_keys").and_then(JsonValue::as_u64);
    let independently_observed = report
        .pointer("/unique_key_verification/observed")
        .and_then(JsonValue::as_u64);
    if unique_keys != independently_observed {
        problems.push("unique-key count was not independently verified".to_owned());
    }
    if report.get("diagnostic_only").and_then(JsonValue::as_bool) == Some(true)
        && report
            .get("ship_evidence_eligible")
            .and_then(JsonValue::as_bool)
            != Some(false)
    {
        problems.push("diagnostic report cannot be promoted to ship evidence".to_owned());
    }
    let build_allocator = report
        .pointer("/build/allocator")
        .and_then(JsonValue::as_str);
    let provider_allocator = report
        .pointer("/allocator/name")
        .and_then(JsonValue::as_str);
    if build_allocator != provider_allocator {
        problems.push("allocator provider does not match the frozen build identity".to_owned());
    }
    if let Some(checkpoints) = report.get("checkpoints").and_then(JsonValue::as_array) {
        let expected = [
            "cold",
            "fill",
            "steady",
            "expire_or_delete",
            "reset",
            "refill",
            "post_idle",
            "shutdown",
        ];
        let mut previous_ns = 0;
        for (index, checkpoint) in checkpoints.iter().enumerate() {
            if checkpoint.get("phase").and_then(JsonValue::as_str) != expected.get(index).copied()
                || checkpoint.get("sequence").and_then(JsonValue::as_u64)
                    != Some((index + 1) as u64)
            {
                problems.push("baseline checkpoints are missing or reordered".to_owned());
                break;
            }
            let monotonic_ns = checkpoint
                .get("monotonic_ns")
                .and_then(JsonValue::as_u64)
                .unwrap_or_default();
            if monotonic_ns <= previous_ns {
                problems.push("baseline checkpoint timestamps are not monotonic".to_owned());
                break;
            }
            previous_ns = monotonic_ns;
        }
    }
    problems
}

pub fn validate_diagnostic_baseline_report(report: &JsonValue) -> Vec<String> {
    let mut problems = validate_baseline_report(report);
    if report.get("diagnostic_only").and_then(JsonValue::as_bool) == Some(true)
        && report
            .get("ship_evidence_eligible")
            .and_then(JsonValue::as_bool)
            == Some(false)
    {
        problems.retain(|problem| {
            problem != "baseline report source SHA does not match its frozen cohort"
        });
    } else {
        problems.push(
            "diagnostic source override requires diagnostic_only=true and ship_evidence_eligible=false"
                .to_owned(),
        );
    }
    problems
}

pub fn diagnostic_fixture_report() -> JsonValue {
    let phases = [
        "cold",
        "fill",
        "steady",
        "expire_or_delete",
        "reset",
        "refill",
        "post_idle",
        "shutdown",
    ];
    let checkpoints = phases
        .into_iter()
        .enumerate()
        .map(|(index, phase)| {
            json!({
                "phase": phase,
                "sequence": index + 1,
                "monotonic_ns": (index + 1) * 1000,
                "logical": {
                    "entries": 10_000, "key_bytes": 100_000, "value_bytes": 2_560_000,
                    "tag_records": 0, "tag_bytes": 0, "generation_records": 0,
                    "generation_bytes": 0, "event_records": 0, "event_bytes": 0,
                    "idempotency_records": 0, "idempotency_bytes": 0,
                    "audit_records": 0, "audit_bytes": 0, "pending": 0,
                    "subscriptions": 0, "sessions": 1
                },
                "process": {
                    "vm_rss_bytes": 1, "vm_hwm_bytes": 1, "smaps_rss_bytes": 1,
                    "smaps_pss_bytes": 1, "smaps_anon_bytes": 1, "smaps_file_bytes": 0,
                    "threads": 1, "fds": 1
                },
                "cgroup": {
                    "memory_current_bytes": availability(1), "memory_peak_bytes": availability(1),
                    "anon_bytes": availability(1), "file_bytes": availability(0),
                    "slab_bytes": availability(0)
                },
                "allocator": {
                    "allocated_bytes": availability(1), "active_bytes": availability(1),
                    "resident_bytes": unavailable("system allocator has no portable resident counter"),
                    "retained_bytes": unavailable("system allocator has no portable retained counter"),
                    "mapped_bytes": unavailable("system allocator has no portable mapped counter")
                },
                "performance": {
                    "rps": 1.0, "p50_ns": 1, "p95_ns": 1, "p99_ns": 1,
                    "max_ns": 1, "errors": 0, "timeouts": 0, "retries": 0,
                    "cpu_seconds": 0.1, "context_switches": 1
                }
            })
        })
        .collect::<Vec<_>>();
    json!({
        "schema_version": 1,
        "release": "0.71",
        "cohort": "B1-instrumented",
        "source_sha": B1_SHA,
        "binary_sha256": format!("sha256:{}", "1".repeat(64)),
        "scenario_digest": format!("sha256:{}", "2".repeat(64)),
        "host_fingerprint": "diagnostic-fixture",
        "build": {
            "profile": "release", "features": [], "allocator": "system",
            "image_digest": null, "kernel": "fixture", "service_profile": "canonical",
            "affinity": "0", "cgroup_limit": 1
        },
        "allocator": {"name": "system", "provider": "system", "provider_version": "fixture"},
        "exact_command": ["hydracache-loadgen", "memory-efficiency"],
        "unique_keys": 10_000,
        "unique_key_verification": {"method": "owner_snapshot", "observed": 10_000},
        "request_count": 20_000,
        "diagnostic_only": true,
        "ship_evidence_eligible": false,
        "checkpoints": checkpoints
    })
}

fn availability(value: u64) -> JsonValue {
    json!({"value": value, "unavailable_reason": null})
}

fn unavailable(reason: &str) -> JsonValue {
    json!({"value": null, "unavailable_reason": reason})
}

fn validate_json_schema(
    value: &JsonValue,
    schema_text: &str,
    label: &str,
    problems: &mut Vec<String>,
) {
    let schema = match serde_json::from_str(schema_text) {
        Ok(schema) => schema,
        Err(error) => {
            problems.push(format!("invalid {label} schema: {error}"));
            return;
        }
    };
    let validator = match jsonschema::validator_for(&schema) {
        Ok(validator) => validator,
        Err(error) => {
            problems.push(format!("cannot compile {label} schema: {error}"));
            return;
        }
    };
    for error in validator.iter_errors(value) {
        problems.push(format!("{label} schema violation: {error}"));
    }
}

fn load_toml(path: &Path) -> Result<toml::Value, Box<dyn Error>> {
    Ok(toml::from_str(&fs::read_to_string(path)?)?)
}

fn sha256_file(path: &Path) -> Result<String, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    Ok(Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn is_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn string_at<'a>(value: &'a toml::Value, path: &[&str]) -> Option<&'a str> {
    let mut current = value;
    for component in path {
        current = current.get(*component)?;
    }
    current.as_str()
}

fn integer_at(value: &toml::Value, path: &[&str]) -> Option<i64> {
    let mut current = value;
    for component in path {
        current = current.get(*component)?;
    }
    current.as_integer()
}

fn bool_at(value: &toml::Value, path: &[&str]) -> Option<bool> {
    let mut current = value;
    for component in path {
        current = current.get(*component)?;
    }
    current.as_bool()
}

fn string_array(value: Option<&toml::Value>) -> Vec<&str> {
    value
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(toml::Value::as_str)
        .collect()
}

fn git_text(root: &Path, args: &[&str]) -> Result<String, Box<dyn Error>> {
    let output = Command::new("git").args(args).current_dir(root).output()?;
    if !output.status.success() {
        return Err(format!("git {} failed", args.join(" ")).into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn git_lines(root: &Path, args: &[&str]) -> Result<Vec<String>, Box<dyn Error>> {
    Ok(git_text(root, args)?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(str::to_owned)
        .collect())
}

fn git_success(root: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .args(args)
        .current_dir(root)
        .status()
        .is_ok_and(|status| status.success())
}
