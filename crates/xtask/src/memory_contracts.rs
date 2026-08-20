use serde_json::{json, Map as JsonMap, Value as JsonValue};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use time::OffsetDateTime;
use toml::Value as TomlValue;

const DECISIONS: &str = "docs/testing/memory/0.71/decision-gates.toml";
const STATISTICS: &str = "docs/testing/memory/0.71/memory-statistics-v1.toml";
const ALLOCATORS: &str = "docs/testing/memory/0.71/allocator-capabilities.toml";
const COMPAT: &str = "docs/testing/compat/memory-071.toml";
const RELEASE_POLICY: &str = "docs/testing/memory/0.71/release-policy.toml";
const HOST_PROFILE_DIR: &str = "docs/testing/perf-host-profiles";
const LEGAL_DECISION_STATES: [&str; 6] = ["planned", "D0", "D1", "D2", "D3", "D4"];

pub fn run_decisions(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    run_static(args, "memory-decision-check", DECISIONS, check_decisions)
}

pub fn run_statistics(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    run_static(
        args,
        "memory-statistics-check",
        STATISTICS,
        check_statistics,
    )
}

pub fn run_allocator(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let options = Options::parse(args)?;
    let value = load_toml(&options.root.join(ALLOCATORS))?;
    let mut problems = check_allocators(&value, &options.release);
    problems.extend(check_allocator_source(&options.root));
    finish("allocator-capability-check", &options.release, problems)
}

pub fn run_compat(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let options = Options::parse(args)?;
    let value = load_toml(&options.root.join(COMPAT))?;
    let problems = check_compat(&options.root, &value, &options.release);
    finish("memory-compat-check", &options.release, problems)
}

pub fn run_release_policy(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let options = Options::parse(args)?;
    let value = load_toml(&options.root.join(RELEASE_POLICY))?;
    let problems = check_release_policy(&value, &options.release, options.require_ship);
    finish("memory-release-policy-check", &options.release, problems)
}

pub fn check_static_contracts(
    root: &Path,
    release: &str,
    require_ship: bool,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut problems = Vec::new();
    problems.extend(check_decisions(&load_toml(&root.join(DECISIONS))?, release));
    problems.extend(check_statistics(
        &load_toml(&root.join(STATISTICS))?,
        release,
    ));
    problems.extend(check_allocators(
        &load_toml(&root.join(ALLOCATORS))?,
        release,
    ));
    problems.extend(check_allocator_source(root));
    problems.extend(check_compat(root, &load_toml(&root.join(COMPAT))?, release));
    problems.extend(check_release_policy(
        &load_toml(&root.join(RELEASE_POLICY))?,
        release,
        require_ship,
    ));
    let profile_id = "memory-reference-071-v1";
    let profile: JsonValue = serde_json::from_slice(&fs::read(
        root.join(HOST_PROFILE_DIR)
            .join(format!("{profile_id}.json")),
    )?)?;
    problems.extend(check_host_profile(&profile, release, profile_id));
    Ok(problems)
}

fn run_static(
    args: Vec<String>,
    label: &str,
    relative: &str,
    checker: fn(&TomlValue, &str) -> Vec<String>,
) -> Result<(), Box<dyn Error>> {
    let options = Options::parse(args)?;
    let value = load_toml(&options.root.join(relative))?;
    finish(label, &options.release, checker(&value, &options.release))
}

fn finish(label: &str, release: &str, problems: Vec<String>) -> Result<(), Box<dyn Error>> {
    if problems.is_empty() {
        println!("{label}: OK (release {release})");
        Ok(())
    } else {
        Err(format!("{label} failed:\n- {}", problems.join("\n- ")).into())
    }
}

pub fn check_decisions(root: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = common_header(root, release);
    require_string(root, "statistics_contract", &mut problems);
    require_string(root, "host_profile", &mut problems);
    let required_states = string_array(root.get("required_states"));
    if required_states != ["D0", "D1", "D2", "D3", "D4"] {
        problems.push("required_states must be exactly D0,D1,D2,D3,D4".to_owned());
    }
    let proposals = table_array(root.get("proposal"));
    if proposals.is_empty() {
        problems.push("at least one proposal or foundation item is required".to_owned());
    }
    let mut ids = BTreeSet::new();
    for proposal in proposals {
        let id = string(proposal.get("id")).unwrap_or("<missing>");
        if !ids.insert(id.to_owned()) {
            problems.push(format!("duplicate proposal id {id}"));
        }
        for field in [
            "work_item",
            "hypothesis",
            "minimum_effect",
            "state",
            "classification",
            "reviewer",
            "disposition",
        ] {
            require_table_string(proposal, id, field, &mut problems);
        }
        for field in [
            "controls",
            "alternative_explanations",
            "primary_metrics",
            "allowed_regressions",
            "authorized_files",
            "authorized_surfaces",
        ] {
            if string_array(proposal.get(field)).is_empty() {
                problems.push(format!("proposal {id} requires non-empty {field}"));
            }
        }
        if proposal
            .get("candidate_derived_baseline")
            .and_then(TomlValue::as_bool)
            != Some(false)
        {
            problems.push(format!("proposal {id} uses a candidate-derived baseline"));
        }
        let state = string(proposal.get("state")).unwrap_or("<missing>");
        if !LEGAL_DECISION_STATES.contains(&state) {
            problems.push(format!("proposal {id} has illegal state {state}"));
        }
        let state_index = LEGAL_DECISION_STATES
            .iter()
            .position(|candidate| candidate == &state)
            .unwrap_or_default();
        if state_index >= 1
            && (string_array(proposal.get("baseline_receipts")).is_empty()
                || string_array(proposal.get("baseline_digests")).is_empty())
        {
            problems.push(format!(
                "proposal {id} reached {state} without frozen baselines"
            ));
        }
        if state_index >= 2
            && (string(proposal.get("classification")) == Some("inconclusive")
                || string_array(proposal.get("rejected_alternatives")).is_empty())
        {
            problems.push(format!(
                "proposal {id} reached {state} without classification and rejected alternatives"
            ));
        }
        if state_index >= 3
            && (string_array(proposal.get("owner_ids")).is_empty()
                || (string_array(proposal.get("stack_ids")).is_empty()
                    && string(proposal.get("classification")) != Some("file/page cache")))
        {
            problems.push(format!(
                "proposal {id} reached {state} without owner/stack authorization"
            ));
        }
        if state_index >= 4 && string_array(proposal.get("resulting_receipts")).is_empty() {
            problems.push(format!(
                "proposal {id} reached {state} without candidate receipts"
            ));
        }
        validate_transition_history(proposal, id, state_index, &mut problems);
    }
    problems
}

fn validate_transition_history(
    proposal: &toml::map::Map<String, TomlValue>,
    id: &str,
    state_index: usize,
    problems: &mut Vec<String>,
) {
    let history = table_array(proposal.get("transition"));
    if state_index == 0 && !history.is_empty() {
        problems.push(format!(
            "planned proposal {id} must not contain decision receipts"
        ));
        return;
    }
    if state_index > 0 && history.len() != state_index {
        problems.push(format!(
            "proposal {id} state requires {state_index} ordered transition receipts, found {}",
            history.len()
        ));
    }
    for (index, transition) in history.iter().enumerate() {
        let expected = LEGAL_DECISION_STATES[index + 1];
        if string(transition.get("state")) != Some(expected) {
            problems.push(format!(
                "proposal {id} transition {} must be {expected}",
                index + 1
            ));
        }
        for field in [
            "receipt",
            "source_sha",
            "host_fingerprint",
            "scenario_digest",
            "contract_digest",
            "reviewer",
        ] {
            require_table_string(transition, id, field, problems);
        }
    }
}

pub fn check_statistics(root: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = common_header(root, release);
    for (field, expected) in [
        ("baseline_only_derivation", true),
        ("candidate_may_amend", false),
    ] {
        if root.get(field).and_then(TomlValue::as_bool) != Some(expected) {
            problems.push(format!("{field} must be {expected}"));
        }
    }
    if string_array(root.get("pairing_order")) != ["B1", "C", "C", "B1"] {
        problems.push("pairing_order must be the frozen alternating B1,C,C,B1 order".to_owned());
    }
    for (field, minimum) in [
        ("sample_cadence_seconds", 1),
        ("warmup_seconds", 1),
        ("settle_seconds", 1),
        ("idle_seconds", 1),
        ("minimum_repetitions", 5),
        ("bootstrap_iterations", 1000),
        ("bootstrap_block_samples", 2),
    ] {
        if root
            .get(field)
            .and_then(TomlValue::as_integer)
            .unwrap_or_default()
            < minimum
        {
            problems.push(format!("{field} must be at least {minimum}"));
        }
    }
    let confidence = root
        .get("confidence_level")
        .and_then(TomlValue::as_float)
        .unwrap_or_default();
    if !(0.95..1.0).contains(&confidence) {
        problems.push("confidence_level must be in [0.95, 1.0)".to_owned());
    }
    for (field, expected) in [
        ("multiple_comparison", "holm-bonferroni"),
        ("missing_row_policy", "fail-preflight-no-interpolation"),
        ("process_model", "independently-started"),
        ("bootstrap_method", "moving-block-v1"),
        ("host_eligibility", "dedicated-s7-only"),
    ] {
        if string(root.get(field)) != Some(expected) {
            problems.push(format!("{field} must be {expected}"));
        }
    }
    let metrics = table_array(root.get("metric"));
    let primary = metrics
        .iter()
        .filter(|metric| string(metric.get("kind")) == Some("primary"))
        .count();
    if primary < 2 {
        problems.push("statistics contract requires at least two primary metrics".to_owned());
    }
    for metric in metrics {
        let id = string(metric.get("id")).unwrap_or("<missing>");
        if string(metric.get("kind")) == Some("primary") {
            let absolute = numeric(metric.get("absolute_minimum_effect"));
            let relative = numeric(metric.get("relative_minimum_effect"));
            if absolute <= 0.0 || relative <= 0.0 {
                problems.push(format!(
                    "primary metric {id} needs positive practical effects"
                ));
            }
        }
    }
    let budgets = table_array(root.get("regression_budget"));
    let budget_ids: BTreeSet<_> = budgets
        .iter()
        .filter_map(|budget| string(budget.get("metric")))
        .collect();
    for required in [
        "throughput_ops_per_second",
        "cpu_seconds_per_operation",
        "p99_latency_seconds",
    ] {
        if !budget_ids.contains(required) {
            problems.push(format!("missing unchanged regression budget {required}"));
        }
    }
    problems
}

pub fn check_allocators(root: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = common_header(root, release);
    let features = string_array(root.get("explicit_allocator_features"));
    if features.len() != 3 || features.iter().collect::<BTreeSet<_>>().len() != features.len() {
        problems.push("explicit allocator features must be three unique choices".to_owned());
    }
    if root
        .get("mutual_exclusion_required")
        .and_then(TomlValue::as_bool)
        != Some(true)
    {
        problems.push("allocator features must be mutually exclusive".to_owned());
    }
    let allocators = table_array(root.get("allocator"));
    let ids: BTreeSet<_> = allocators
        .iter()
        .filter_map(|allocator| string(allocator.get("id")))
        .collect();
    if ids != BTreeSet::from(["system", "jemalloc", "mimalloc"]) {
        problems.push("capability matrix must contain system, jemalloc, and mimalloc".to_owned());
    }
    let defaults: Vec<_> = allocators
        .iter()
        .filter(|allocator| allocator.get("default").and_then(TomlValue::as_bool) == Some(true))
        .collect();
    if defaults.len() != 1 || string(defaults[0].get("id")) != Some("system") {
        problems.push("system must remain the sole portable default".to_owned());
    }
    for allocator in allocators {
        let id = string(allocator.get("id")).unwrap_or("<missing>");
        for field in [
            "build_feature",
            "crate_version",
            "native_version",
            "license",
            "statistics_api",
            "profiling_provider",
            "purge_api",
            "profile_adapter",
        ] {
            require_table_string(allocator, id, field, &mut problems);
        }
        if string_array(allocator.get("targets")).is_empty()
            || string_array(allocator.get("fields")).is_empty()
        {
            problems.push(format!("allocator {id} lacks target/field capabilities"));
        }
        for unavailable in string_array(allocator.get("unavailable_fields")) {
            if !unavailable.contains("unavailable(") {
                problems.push(format!(
                    "allocator {id} unavailable field must include unavailable(reason): {unavailable}"
                ));
            }
        }
    }
    problems
}

pub fn check_allocator_source(root: &Path) -> Vec<String> {
    let mut problems = Vec::new();
    let manifest = fs::read_to_string(root.join("crates/hydracache/Cargo.toml"))
        .unwrap_or_else(|error| format!("unavailable: {error}"));
    let source = fs::read_to_string(root.join("crates/hydracache/src/lib.rs"))
        .unwrap_or_else(|error| format!("unavailable: {error}"));
    for marker in [
        "allocator-explicit = []",
        "allocator-system = [\"allocator-explicit\"]",
        "allocator-jemalloc = [\"allocator-explicit\", \"dep:tikv-jemallocator\"]",
        "allocator-mimalloc = [\"allocator-explicit\", \"dep:mimalloc\"]",
        "tikv-jemallocator",
        "mimalloc",
    ] {
        if !manifest.contains(marker) {
            problems.push(format!("allocator manifest wiring is missing {marker:?}"));
        }
    }
    for marker in [
        "allocator-explicit requires exactly one",
        "allocator features are mutually exclusive",
        "HYDRACACHE_JEMALLOC",
        "HYDRACACHE_MIMALLOC",
    ] {
        if !source.contains(marker) {
            problems.push(format!("allocator source wiring is missing {marker:?}"));
        }
    }
    problems
}

pub fn check_compat(root_path: &Path, root: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = common_header(root, release);
    if string(root.get("baseline_tag")) != Some("v0.70.0") {
        problems.push("compat baseline_tag must be v0.70.0".to_owned());
    }
    let expected_commit = string(root.get("baseline_commit")).unwrap_or_default();
    match command_text(root_path, "git", &["rev-list", "-n", "1", "v0.70.0"]) {
        Ok(actual) if actual != expected_commit => problems.push(format!(
            "v0.70.0 resolves to {actual}, matrix freezes {expected_commit}"
        )),
        Err(error) => problems.push(format!("cannot resolve v0.70.0: {error}")),
        _ => {}
    }
    for (field, expected) in [
        ("candidate_ref", "HEAD"),
        ("default_policy", "runtime_only_no_wire_or_durable_change"),
        ("unknown_future_policy", "refuse-before-mutation"),
    ] {
        if string(root.get(field)) != Some(expected) {
            problems.push(format!("{field} must be {expected}"));
        }
    }
    if root.get("backup_required").and_then(TomlValue::as_bool) != Some(true) {
        problems.push("compatibility matrix must require a baseline backup".to_owned());
    }
    let rows = table_array(root.get("row"));
    let ids: BTreeSet<_> = rows
        .iter()
        .filter_map(|row| string(row.get("id")))
        .collect();
    for required in [
        "baseline-create-candidate-read-mutate-restart",
        "candidate-create-candidate-restart",
        "candidate-to-baseline-compatible-rollback",
        "rolling-baseline-candidate-all-role-orders",
        "snapshot-empty-max-record-crash-upgrade",
    ] {
        if !ids.contains(required) {
            problems.push(format!("compatibility matrix is missing row {required}"));
        }
    }
    for row in rows {
        let id = string(row.get("id")).unwrap_or("<missing>");
        for field in [
            "source_binary",
            "target_binary",
            "durable_format",
            "expected",
        ] {
            require_table_string(row, id, field, &mut problems);
        }
        let wires = string_array(row.get("wire_generations"));
        if !wires.contains(&"HC/1") || !wires.contains(&"HC/2") {
            problems.push(format!("compat row {id} must cover HC/1 and HC/2"));
        }
        if string_array(row.get("profiles")).is_empty() {
            problems.push(format!("compat row {id} has no feature/profile coverage"));
        }
    }
    problems
}

pub fn check_release_policy(root: &TomlValue, release: &str, require_ship: bool) -> Vec<String> {
    let mut problems = common_header(root, release);
    for (field, expected) in [
        ("zero_optional_wins_allowed", true),
        ("negative_result_required_when_no_candidate_qualifies", true),
        ("deferred_safety_defects_allowed", false),
    ] {
        if root.get(field).and_then(TomlValue::as_bool) != Some(expected) {
            problems.push(format!("{field} must be {expected}"));
        }
    }
    let foundation: BTreeSet<_> = string_array(root.get("mandatory_foundation"))
        .into_iter()
        .collect();
    for required in [
        "S1-ownership-closure",
        "corrected-baseline-S4-S5-S7",
        "W1-coherent-production-counters",
        "W2a-retained-byte-accounting",
        "confirmed-unbounded-owner-fixes",
        "per-pr-structural-memory-gate",
        "S8-compatibility",
        "S9-ci-reliability",
        "W13-governance",
    ] {
        if !foundation.contains(required) {
            problems.push(format!("mandatory release foundation omits {required}"));
        }
    }
    let optional = table_array(root.get("optional_work"));
    if optional.is_empty() {
        problems.push("release policy must disposition optional work".to_owned());
    }
    for item in optional {
        let id = string(item.get("id")).unwrap_or("<missing>");
        let disposition = string(item.get("disposition")).unwrap_or("<missing>");
        let allowed = [
            "pending-evidence",
            "implemented-and-qualified",
            "measured-no-win",
            "not-applicable",
            "deferred",
        ];
        if !allowed.contains(&disposition) {
            problems.push(format!(
                "optional work {id} has invalid disposition {disposition}"
            ));
        }
        require_table_string(item, id, "reason", &mut problems);
        require_table_string(item, id, "next_evidence", &mut problems);
        if require_ship && disposition == "pending-evidence" {
            problems.push(format!("ship admission refuses pending optional work {id}"));
        }
        if disposition == "deferred"
            && !string(item.get("reason"))
                .is_some_and(|reason| reason.contains("bounded") && reason.contains("correct"))
        {
            problems.push(format!(
                "deferred work {id} must state that its current owner is bounded and correct"
            ));
        }
    }
    problems
}

pub fn run_host_preflight(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let options = Options::parse(args)?;
    let profile_id = options
        .profile
        .as_deref()
        .ok_or("perf-memory-preflight requires --profile")?;
    let profile_path = options
        .root
        .join(HOST_PROFILE_DIR)
        .join(format!("{profile_id}.json"));
    let profile: JsonValue = serde_json::from_slice(&fs::read(&profile_path)?)?;
    let profile_problems = check_host_profile(&profile, &options.release, profile_id);
    if !profile_problems.is_empty() {
        return Err(format!("host profile failed:\n- {}", profile_problems.join("\n- ")).into());
    }

    let started_at = now();
    let source_sha = command_text(&options.root, "git", &["rev-parse", "HEAD"])?;
    let baseline_sha = command_text(&options.root, "git", &["rev-list", "-n", "1", "v0.70.0"])?;
    let platform = observed_platform();
    let protected_environment = std::env::var("HYDRACACHE_MEMORY_PROTECTED_ENV").ok();
    let lease_owner = std::env::var("HYDRACACHE_MEMORY_LEASE_OWNER").ok();
    let lease_end = std::env::var("HYDRACACHE_MEMORY_LEASE_END").ok();
    let dedicated = std::env::var("HYDRACACHE_MEMORY_DEDICATED_BARE_METAL").as_deref() == Ok("1");
    let tools = observed_tools(&["git", "cargo", "rustc", "python3", "perf", "numactl"]);
    let probes = json!({
        "platform": platform,
        "protected_environment": protected_environment,
        "lease_owner": lease_owner,
        "lease_end": lease_end,
        "dedicated_bare_metal": dedicated,
        "tools": tools,
        "logical_cpus": std::thread::available_parallelism().map(|value| value.get()).unwrap_or(0),
        "hardware_model": first_matching_line("/proc/cpuinfo", "model name"),
        "cpu_topology": command_optional("lscpu", &["--json"]),
        "numa_topology": read_optional("/sys/devices/system/node/online"),
        "ram": first_matching_line("/proc/meminfo", "MemTotal"),
        "firmware": read_optional("/sys/class/dmi/id/bios_version"),
        "microcode": first_matching_line("/proc/cpuinfo", "microcode"),
        "page_size": observed_page_size(),
        "kernel": command_optional("uname", &["-srvmo"]),
        "distro": read_optional("/etc/os-release"),
        "cpu_governor": read_optional("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor"),
        "turbo_policy": read_optional("/sys/devices/system/cpu/intel_pstate/no_turbo"),
        "thermal_state": read_optional("/sys/class/thermal/thermal_zone0/temp"),
        "transparent_huge_pages": read_optional("/sys/kernel/mm/transparent_hugepage/enabled"),
        "swap": read_optional("/proc/swaps"),
        "overcommit": read_optional("/proc/sys/vm/overcommit_memory"),
        "ksm": read_optional("/sys/kernel/mm/ksm/run"),
        "cgroup": read_optional("/proc/self/cgroup"),
        "cgroup_memory_limit": read_optional("/sys/fs/cgroup/memory.max"),
        "container_runtime": command_optional("docker", &["version", "--format", "{{.Server.Version}}"]),
        "clock_source": read_optional("/sys/devices/system/clocksource/clocksource0/current_clocksource"),
        "filesystem": command_optional("df", &["-T", "."]),
        "allocator_knobs": {
            "MALLOC_CONF": std::env::var("MALLOC_CONF").ok(),
            "MIMALLOC_OPTIONS": std::env::var("MIMALLOC_OPTIONS").ok()
        },
        "competing_load": read_optional("/proc/loadavg"),
        "available_memory": first_matching_line("/proc/meminfo", "MemAvailable"),
        "major_faults_and_throttling": read_optional("/proc/self/status"),
        "temperature": read_optional("/sys/class/thermal/thermal_zone0/temp")
    });
    let calibration = calibration_samples();
    let calibration_limit = profile
        .get("calibration_max_relative_spread")
        .and_then(JsonValue::as_f64)
        .unwrap_or_default();
    let calibration_green = calibration
        .get("relative_spread")
        .and_then(JsonValue::as_f64)
        .is_some_and(|spread| spread <= calibration_limit);
    let fingerprint = canonical_json_digest(&probes);
    let toolchain = format!(
        "rustc={};cargo={}",
        command_optional("rustc", &["--version"]).unwrap_or_else(|| "unavailable".to_owned()),
        command_optional("cargo", &["--version"]).unwrap_or_else(|| "unavailable".to_owned())
    );
    let eligible = cfg!(target_os = "linux")
        && dedicated
        && protected_environment.as_deref() == Some("memory-reference-071")
        && lease_owner
            .as_deref()
            .is_some_and(|value| !value.is_empty())
        && lease_end.as_deref().is_some_and(|value| !value.is_empty())
        && calibration_green
        && tools
            .values()
            .all(|value| value.as_str() != Some("unavailable"));
    let result = if eligible { "success" } else { "blocked-host" };
    let receipt = json!({
        "schema_version": 1,
        "release": options.release,
        "source_sha": source_sha,
        "tested_sha": source_sha,
        "baseline_sha": baseline_sha,
        "scenario_digest": canonical_json_digest(&profile),
        "host_fingerprint": fingerprint,
        "toolchain": toolchain,
        "started_at": started_at,
        "finished_at": now(),
        "result": result,
        "ship_evidence_eligible": eligible,
        "profile_id": profile_id,
        "protected_environment": protected_environment,
        "lease": {"owner": lease_owner, "end": lease_end},
        "pre_probes": probes,
        "calibration": calibration,
        "calibration_limit": calibration_limit,
        "post_probes_required": true,
        "silent_retry_allowed": false
    });
    let output = options.output.unwrap_or_else(|| {
        options
            .root
            .join("target/memory-evidence/0.71/host-preflight.json")
    });
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output, serde_json::to_vec_pretty(&receipt)?)?;
    println!("perf-memory-preflight: {result} ({})", output.display());
    if eligible {
        Ok(())
    } else {
        Err("blocked-host: dedicated protected host contract is not satisfied".into())
    }
}

pub fn check_host_profile(profile: &JsonValue, release: &str, profile_id: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if profile.get("schema_version").and_then(JsonValue::as_u64) != Some(1)
        || profile.get("release").and_then(JsonValue::as_str) != Some(release)
        || profile.get("profile_id").and_then(JsonValue::as_str) != Some(profile_id)
    {
        problems.push("host profile schema/release/profile identity mismatch".to_owned());
    }
    for field in [
        "protected_environment",
        "completed_bootstrap_admission",
        "ship_evidence_platforms",
        "ineligible_platforms",
        "immutable_probes",
        "mutable_probes",
        "required_tools",
        "cpu_binding",
        "policies",
    ] {
        if profile.get(field).is_none() {
            problems.push(format!("host profile is missing {field}"));
        }
    }
    if profile.get("lease_required").and_then(JsonValue::as_bool) != Some(true) {
        problems.push("host profile must require a serialized lease".to_owned());
    }
    if !profile
        .get("calibration_max_relative_spread")
        .and_then(JsonValue::as_f64)
        .is_some_and(|value| value > 0.0 && value <= 0.10)
    {
        problems.push("host profile needs a reviewed calibration spread in (0, 0.10]".to_owned());
    }
    let mutable = profile
        .get("mutable_probes")
        .and_then(JsonValue::as_array)
        .cloned()
        .unwrap_or_default();
    for required in [
        "cpu_governor",
        "transparent_huge_pages",
        "swap",
        "cgroup_limits",
        "competing_load",
        "temperature",
    ] {
        if !mutable.iter().any(|value| value.as_str() == Some(required)) {
            problems.push(format!("host profile mutable probes omit {required}"));
        }
    }
    problems
}

pub fn canonical_json_digest(value: &JsonValue) -> String {
    let canonical = canonical_json(value);
    let bytes = serde_json::to_vec(&canonical).expect("canonical JSON serialization");
    format!("sha256:{}", hex(&Sha256::digest(bytes)))
}

fn canonical_json(value: &JsonValue) -> JsonValue {
    match value {
        JsonValue::Object(map) => {
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();
            let mut result = JsonMap::new();
            for key in keys {
                result.insert(key.clone(), canonical_json(&map[key]));
            }
            JsonValue::Object(result)
        }
        JsonValue::Array(values) => JsonValue::Array(values.iter().map(canonical_json).collect()),
        other => other.clone(),
    }
}

fn common_header(root: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if root.get("schema_version").and_then(TomlValue::as_integer) != Some(1) {
        problems.push("schema_version must be 1".to_owned());
    }
    if string(root.get("release")) != Some(release) {
        problems.push(format!("release must be {release}"));
    }
    problems
}

fn load_toml(path: &Path) -> Result<TomlValue, Box<dyn Error>> {
    Ok(toml::from_str(&fs::read_to_string(path)?)?)
}

fn table_array(value: Option<&TomlValue>) -> Vec<&toml::map::Map<String, TomlValue>> {
    value
        .and_then(TomlValue::as_array)
        .into_iter()
        .flatten()
        .filter_map(TomlValue::as_table)
        .collect()
}

fn string_array(value: Option<&TomlValue>) -> Vec<&str> {
    value
        .and_then(TomlValue::as_array)
        .into_iter()
        .flatten()
        .filter_map(TomlValue::as_str)
        .collect()
}

fn string(value: Option<&TomlValue>) -> Option<&str> {
    value.and_then(TomlValue::as_str)
}

fn numeric(value: Option<&TomlValue>) -> f64 {
    value
        .and_then(|value| {
            value
                .as_float()
                .or_else(|| value.as_integer().map(|value| value as f64))
        })
        .unwrap_or_default()
}

fn require_string(root: &TomlValue, field: &str, problems: &mut Vec<String>) {
    if string(root.get(field)).is_none_or(|value| value.trim().is_empty()) {
        problems.push(format!("missing non-empty {field}"));
    }
}

fn require_table_string(
    table: &toml::map::Map<String, TomlValue>,
    owner: &str,
    field: &str,
    problems: &mut Vec<String>,
) {
    if string(table.get(field)).is_none_or(|value| value.trim().is_empty()) {
        problems.push(format!("{owner} is missing non-empty {field}"));
    }
}

fn command_text(root: &Path, program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn command_optional(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn observed_tools(programs: &[&str]) -> JsonMap<String, JsonValue> {
    programs
        .iter()
        .map(|program| {
            let version = command_optional(program, &["--version"])
                .unwrap_or_else(|| "unavailable".to_owned());
            ((*program).to_owned(), JsonValue::String(version))
        })
        .collect()
}

fn observed_platform() -> &'static str {
    if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "other"
    }
}

fn observed_page_size() -> Option<String> {
    if cfg!(target_os = "windows") {
        None
    } else {
        command_optional("getconf", &["PAGESIZE"])
    }
}

fn read_optional(path: &str) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().to_owned())
}

fn first_matching_line(path: &str, prefix: &str) -> Option<String> {
    fs::read_to_string(path)
        .ok()?
        .lines()
        .find(|line| line.trim_start().starts_with(prefix))
        .map(str::trim)
        .map(str::to_owned)
}

fn calibration_samples() -> JsonValue {
    let mut durations = Vec::new();
    for sample in 0..5_u64 {
        let started = std::time::Instant::now();
        let mut value = sample.wrapping_add(1);
        for index in 0..2_000_000_u64 {
            value = std::hint::black_box(value.rotate_left(7) ^ index).wrapping_mul(0x9e37_79b9);
        }
        std::hint::black_box(value);
        durations.push(started.elapsed().as_secs_f64());
    }
    let minimum = durations.iter().copied().fold(f64::INFINITY, f64::min);
    let maximum = durations.iter().copied().fold(0.0_f64, f64::max);
    let mean = durations.iter().sum::<f64>() / durations.len() as f64;
    let relative_spread = if mean > 0.0 {
        (maximum - minimum) / mean
    } else {
        f64::INFINITY
    };
    json!({
        "algorithm": "integer-mix-v1",
        "samples_seconds": durations,
        "minimum_seconds": minimum,
        "maximum_seconds": maximum,
        "mean_seconds": mean,
        "relative_spread": relative_spread
    })
}

fn now() -> String {
    OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "unavailable".to_owned())
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

struct Options {
    root: PathBuf,
    release: String,
    profile: Option<String>,
    output: Option<PathBuf>,
    require_ship: bool,
}

impl Options {
    fn parse(args: Vec<String>) -> Result<Self, Box<dyn Error>> {
        let mut root = None;
        let mut release = None;
        let mut profile = None;
        let mut output = None;
        let mut require_ship = false;
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--root" => root = Some(PathBuf::from(take(&args, &mut index, "--root")?)),
                "--release" => release = Some(take(&args, &mut index, "--release")?),
                "--profile" => profile = Some(take(&args, &mut index, "--profile")?),
                "--output" => output = Some(PathBuf::from(take(&args, &mut index, "--output")?)),
                "--require-ship" => require_ship = true,
                other => {
                    return Err(format!("unsupported memory contract argument: {other}").into())
                }
            }
            index += 1;
        }
        Ok(Self {
            root: root.unwrap_or(crate::doc_check::find_repo_root()?),
            release: release.ok_or("memory contract command requires --release")?,
            profile,
            output,
            require_ship,
        })
    }
}

fn take(args: &[String], index: &mut usize, flag: &str) -> Result<String, Box<dyn Error>> {
    *index += 1;
    args.get(*index)
        .cloned()
        .ok_or_else(|| format!("{flag} requires a value").into())
}
