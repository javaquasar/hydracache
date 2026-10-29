//! Bounded local IMap CI lanes and complete-matrix receipt validation.

use crate::imap_qualification_075::{digest, hex, read_relative, MAX_ARTIFACT_BYTES};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::process::Command;

pub const RUST_VERSIONS: [&str; 2] = ["1.88.0", "1.94.0"];
pub const JAVA_VERSIONS: [u32; 2] = [17, 21];
const WORKFLOW: &str = ".github/workflows/imap-075.yml";
const BUILD_BUDGET_COMMAND: &str = r#"echo "HYDRACACHE_IMAP075_BUILD_DEADLINE_MS=$(( ($(date +%s) + 43 * 60) * 1000 ))" >> "$GITHUB_ENV""#;
const COMMANDS: [&str; 11] = [
    "imap-distributed-correctness",
    "imap-capability-audit",
    "imap-migration-dry-run",
    "imap-comparison-check",
    "imap-release-admission",
    "imap-tooling-tests",
    "imap-semantic-audit",
    "imap-cluster-chaos",
    "imap-extended-concurrent",
    "imap-extended-evidence",
    "imap-pre074-preparation",
];

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lane {
    pub rust: String,
    pub java: u32,
    pub shard: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CiReceipt {
    pub release: String,
    pub receipt_version: u32,
    pub scope: String,
    pub mode: String,
    pub source_sha: String,
    pub workflow_sha256: String,
    pub lane: Lane,
    pub seeds: Vec<u64>,
    pub steps: usize,
    pub rust_fingerprint: String,
    pub java_fingerprint: String,
    pub commands: Vec<String>,
    pub completed: bool,
    pub passed: bool,
    pub production_enabled: bool,
}

pub fn expected_lanes(mode: &str) -> Result<Vec<Lane>, Box<dyn Error>> {
    let shards = match mode {
        "pr" => 1,
        "nightly" => 4,
        _ => return Err("CI mode must be pr or nightly".into()),
    };
    Ok(RUST_VERSIONS
        .into_iter()
        .flat_map(|rust| {
            JAVA_VERSIONS.into_iter().flat_map(move |java| {
                (0..shards).map(move |shard| Lane {
                    rust: rust.into(),
                    java,
                    shard,
                })
            })
        })
        .collect())
}
pub fn seeds_for(shard: u32) -> Result<Vec<u64>, Box<dyn Error>> {
    if shard >= 4 {
        return Err("CI shard exceeds its bound".into());
    }
    Ok(vec![117 + 2 * u64::from(shard), 118 + 2 * u64::from(shard)])
}

pub fn run_check(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let options = parse(&args, false)?;
    let root = crate::doc_check::find_repo_root()?;
    check_workflow(&root)?;
    if let Some(dir) = options.receipts {
        check_receipts(&root, &dir, &options.mode)?;
    }
    println!(
        "imap-ci-check 0.75: OK (bounded {} lanes; local proof scope)",
        options.mode
    );
    Ok(())
}
pub fn run_lane(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let options = parse(&args, true)?;
    let root = crate::doc_check::find_repo_root()?;
    let lane = options.lane.ok_or("CI lane identity required")?;
    if !expected_lanes(&options.mode)?.contains(&lane) {
        return Err("lane is outside the declared matrix".into());
    }
    check_workflow(&root)?;
    let before = source_state(&root)?;
    let rust = fingerprint("rustc", &["--version"])?;
    let java = fingerprint("java", &["--version"])?;
    if !rust.starts_with(&format!("rustc {} ", lane.rust)) || !java_major(&java, lane.java) {
        return Err("actual Rust/JVM versions do not match the requested CI lane".into());
    }
    crate::imap_distributed_correctness::run(vec!["--release".into(), "0.75".into()])?;
    crate::imap_capability_audit::run(vec!["--release".into(), "0.75".into()])?;
    crate::imap_migration_dry_run::run(vec!["--release".into(), "0.75".into()])?;
    crate::imap_qualification_075::comparison_run(vec!["--release".into(), "0.75".into()])?;
    crate::imap_qualification_075::release_run(vec!["--release".into(), "0.75".into()])?;
    crate::imap_semantic_audit075::run(vec!["--release".into(), "0.75".into()])?;
    crate::imap_build_process_075::BuildController::new(&root)?.run(
        crate::imap_build_process_075::BuildStage::ToolingTests,
        tooling_command(&root, cfg!(windows)),
    )?;
    let output = std::path::Path::new(options.output.as_deref().ok_or("--output is required")?);
    let preparation_dir = output
        .parent()
        .ok_or("CI output requires a parent")?
        .join(format!(
            "pre074-{}-{}-{}-{before}",
            lane.rust, lane.java, lane.shard
        ));
    let preparation_receipt = crate::imap_pre074_075::produce(&root, &preparation_dir)?;
    crate::imap_pre074_075::verify(&root, &preparation_receipt)?;
    let seeds = seeds_for(lane.shard)?;
    for seed in &seeds {
        crate::imap_api_differential::check_actual_at_root(&root, *seed, 128)?;
        crate::imap_extended_concurrent_075::check_actual(&root, i64::try_from(*seed)?, 15)?;
        let output = std::path::Path::new(options.output.as_deref().ok_or("--output is required")?);
        let evidence_dir = output
            .parent()
            .ok_or("CI output requires a parent")?
            .join(format!(
                "extended-{}-{}-{}-{}-{}",
                lane.rust, lane.java, lane.shard, seed, before
            ));
        let receipt =
            crate::imap_extended_receipt_075::produce(&root, &evidence_dir, i64::try_from(*seed)?)?;
        crate::imap_extended_receipt_075::verify(&root, &receipt)?;
        println!(
            "{}",
            crate::imap_cluster_campaign_075::check_at_root(&root, *seed, 3)?
        );
    }
    if source_state(&root)? != before {
        return Err("source changed during CI lane execution".into());
    }
    let receipt = CiReceipt {
        release: "0.75".into(),
        receipt_version: 2,
        scope: "local_ci".into(),
        mode: options.mode,
        source_sha: before,
        workflow_sha256: digest(&read_relative(&root, WORKFLOW, MAX_ARTIFACT_BYTES)?),
        lane,
        seeds,
        steps: 128,
        rust_fingerprint: rust,
        java_fingerprint: java,
        commands: COMMANDS.into_iter().map(str::to_owned).collect(),
        completed: true,
        passed: true,
        production_enabled: false,
    };
    write_receipt(
        &root,
        &options.output.ok_or("--output is required")?,
        &receipt,
    )?;
    println!("imap-ci-run 0.75: OK (exact-source local receipt; not ship admission)");
    Ok(())
}

/// Build tooling separately from the executing producer. Linux can unlink/replace a live
/// executable too, which would invalidate the producer's exact tooling identity.
#[doc(hidden)]
pub fn tooling_command(root: &Path, _windows: bool) -> Command {
    let mut command = Command::new("cargo");
    command
        .args([
            "test",
            "-p",
            "xtask",
            "--lib",
            "--test",
            "imap_contract_075",
            "--test",
            "imap_capability_audit_075",
            "--test",
            "imap_migration_dry_run_075",
            "--test",
            "imap_qualification_075",
            "--test",
            "imap_ci_075",
            "--test",
            "imap_semantic_audit075",
            "--test",
            "imap_cluster_campaign_075",
            "--test",
            "imap_cluster_fuzz_policy075",
            "--test",
            "imap_foundation_generate_075",
            "--test",
            "imap_extended_differential_075",
            "--test",
            "imap_extended_concurrent_075",
            "--test",
            "imap_proof_process_075",
            "--test",
            "imap_build_process_075",
            "--test",
            "imap_extended_receipt_075",
            "--test",
            "imap_daemon_probe_075",
            "--test",
            "imap_storage_probe_075",
            "--test",
            "imap_pre074_075",
            "--test",
            "imap_native_probe_075",
            "--test",
            "imap_native_concurrent_075",
            "--test",
            "imap_native_recovery_075",
            "--locked",
        ])
        .current_dir(root);
    command.env("CARGO_TARGET_DIR", root.join("target/imap-075/tooling"));
    command
}

struct Options {
    mode: String,
    receipts: Option<String>,
    output: Option<String>,
    lane: Option<Lane>,
}
fn parse(args: &[String], run: bool) -> Result<Options, Box<dyn Error>> {
    let mut fields = std::collections::BTreeMap::new();
    if !args.len().is_multiple_of(2) {
        return Err("every CI option requires a value".into());
    }
    for pair in args.chunks_exact(2) {
        if !matches!(
            pair[0].as_str(),
            "--release" | "--mode" | "--receipts" | "--output" | "--rust" | "--java" | "--shard"
        ) || fields.insert(pair[0].as_str(), pair[1].clone()).is_some()
        {
            return Err("unknown or duplicate CI option".into());
        }
    }
    if fields.remove("--release").as_deref() != Some("0.75") {
        return Err("requires --release 0.75".into());
    }
    let mode = fields.remove("--mode").unwrap_or_else(|| "pr".into());
    expected_lanes(&mode)?;
    let receipts = fields.remove("--receipts");
    let output = fields.remove("--output");
    let lane = if run {
        Some(Lane {
            rust: fields.remove("--rust").ok_or("--rust required")?,
            java: fields.remove("--java").ok_or("--java required")?.parse()?,
            shard: fields
                .remove("--shard")
                .ok_or("--shard required")?
                .parse()?,
        })
    } else {
        None
    };
    if !fields.is_empty()
        || (run && (receipts.is_some() || output.is_none()))
        || (!run && output.is_some())
    {
        return Err("options do not apply to this CI command".into());
    }
    Ok(Options {
        mode,
        receipts,
        output,
        lane,
    })
}

pub fn check_workflow(root: &Path) -> Result<(), Box<dyn Error>> {
    let bytes = read_relative(root, WORKFLOW, MAX_ARTIFACT_BYTES)?;
    let value: serde_yaml::Value = serde_yaml::from_slice(&bytes)?;
    let paths = value["on"]["pull_request"]["paths"]
        .as_sequence()
        .ok_or("IMap PR dependency paths required")?
        .iter()
        .filter_map(serde_yaml::Value::as_str)
        .collect::<BTreeSet<_>>();
    for dependency in [
        "crates/**",
        "sdks/java/**",
        "tests/java-imap-benchmark/**",
        "tests/java-imap-hazelcast-075/**",
        "tests/rust-imap-contract-consumer-075/**",
        "docs/testing/imap/**",
        "fuzz/**",
        ".github/workflows/ci.yml",
        WORKFLOW,
        "Cargo.toml",
        "Cargo.lock",
    ] {
        if !paths.contains(dependency) {
            return Err(format!("IMap CI must run for dependency changes: {dependency}").into());
        }
    }
    let jobs = &value["jobs"];
    let actual_jobs = jobs
        .as_mapping()
        .ok_or("IMap jobs mapping required")?
        .keys()
        .filter_map(serde_yaml::Value::as_str)
        .collect::<BTreeSet<_>>();
    if actual_jobs != BTreeSet::from(["plan", "qualification", "complete"])
        || jobs["plan"]["timeout-minutes"].as_u64() != Some(5)
        || jobs["complete"]["timeout-minutes"].as_u64() != Some(30)
    {
        return Err("IMap CI must retain exactly its three bounded jobs".into());
    }
    if !value["on"].is_mapping()
        || jobs["plan"]["outputs"]["matrix"].is_null()
        || jobs["qualification"]["strategy"]["fail-fast"] != serde_yaml::Value::Bool(false)
        || jobs["qualification"]["timeout-minutes"].as_u64() != Some(45)
        || jobs["complete"]["if"].as_str() != Some("always()")
    {
        return Err(
            "IMap workflow must retain bounded matrix and always-run completeness gate".into(),
        );
    }
    let mut run_text = String::new();
    let mut artifact_count = 0;
    for name in ["plan", "qualification", "complete"] {
        let steps = jobs[name]["steps"]
            .as_sequence()
            .ok_or("CI job requires steps")?;
        if name == "qualification" {
            let budgets = steps
                .iter()
                .enumerate()
                .filter(|(_, step)| step["name"].as_str() == Some("Qualification budget"))
                .collect::<Vec<_>>();
            let checkout = steps
                .iter()
                .position(|step| {
                    step["uses"]
                        .as_str()
                        .is_some_and(|v| v.starts_with("actions/checkout@"))
                })
                .ok_or("missing qualification checkout")?;
            if budgets.len() != 1
                || budgets[0].0 >= checkout
                || budgets[0].1["shell"].as_str() != Some("bash")
                || budgets[0].1["run"].as_str().map(str::trim) != Some(BUILD_BUDGET_COMMAND)
            {
                return Err(
                    "qualification must establish its bounded outer build deadline before checkout"
                        .into(),
                );
            }
        }
        if matches!(name, "qualification" | "complete") {
            let checkouts = steps
                .iter()
                .filter(|step| {
                    step["uses"]
                        .as_str()
                        .is_some_and(|action| action.starts_with("actions/checkout@"))
                })
                .collect::<Vec<_>>();
            if checkouts.len() != 1
                || checkouts[0]["with"]["fetch-depth"].as_u64() != Some(0)
                || !checkouts[0]["with"]["filter"].is_null()
                || !checkouts[0]["with"]["sparse-checkout"].is_null()
            {
                return Err(
                    "IMap evidence jobs require one complete, unfiltered history checkout".into(),
                );
            }
        }
        for step in steps {
            if !step["continue-on-error"].is_null()
                && step["continue-on-error"] != serde_yaml::Value::Bool(false)
            {
                return Err("IMap CI cannot ignore failed steps".into());
            }
            if let Some(action) = step["uses"].as_str() {
                let (name, revision) = action.split_once('@').ok_or("action must be pinned")?;
                if !hex(revision, 40) {
                    return Err("IMap CI actions require immutable commit pins".into());
                }
                let expected = match name {
                    "actions/checkout" => "fbc6f3992d24b796d5a048ff273f7fcc4a7b6c09",
                    "actions/setup-java" => "cf277c60eb25467037889841efdb72551f06f6c3",
                    "actions/upload-artifact" => "b7c566a772e6b6bfb58ed0dc250532a479d7789f",
                    "actions/download-artifact" => "37930b1c2abaa49bbe596cd826c3c89aef350131",
                    _ => return Err("IMap CI action is outside its registered set".into()),
                };
                if revision != expected {
                    return Err("IMap CI action differs from its registered immutable pin".into());
                }
                if name == "actions/upload-artifact" {
                    artifact_count += 1;
                    if step["if"].as_str() != Some("always()") {
                        return Err("failure artifacts must upload on failure".into());
                    }
                    let with = &step["with"];
                    let artifact_name =
                        with["name"].as_str().ok_or("IMap artifact name required")?;
                    if artifact_name.starts_with("imap075-diagnostics-")
                        && !with["path"]
                            .as_str()
                            .is_some_and(|path| path.contains("target/imap-075/cluster-failures/"))
                    {
                        return Err("IMap cluster counterexamples must retain original and minimized failure artifacts".into());
                    }
                    if with["retention-days"].as_u64() != Some(14)
                        || ![
                            "matrix.rust",
                            "matrix.java",
                            "matrix.shard",
                            "github.sha",
                            "github.run_id",
                            "github.run_attempt",
                        ]
                        .iter()
                        .all(|part| artifact_name.contains(part))
                    {
                        return Err("IMap artifacts must retain bounded retention and complete lane/source/run identity".into());
                    }
                }
            }
            if let Some(run) = step["run"].as_str() {
                run_text.push_str(run);
                run_text.push('\n');
            }
        }
    }
    let plan = jobs["plan"]["steps"]
        .as_sequence()
        .ok_or("IMap plan steps required")?;
    if !plan.iter().any(|step| step["run"].as_str().is_some_and(|run| run.lines().any(|line| line.trim() == "lanes = [{'rust': r, 'java': j, 'shard': s} for r in ['1.88.0', '1.94.0'] for j in [17, 21] for s in (range(4) if mode == 'nightly' else range(1))]"))) {
        return Err("IMap CI matrix must retain its exact bounded Rust/JVM/shard product".into());
    }
    for required in [
        "1.88.0",
        "1.94.0",
        "17",
        "21",
        "nightly",
        "range(4)",
        "imap-ci-run",
        "imap-ci-check",
        "--receipts",
        "needs.qualification.result",
    ] {
        if !run_text.contains(required) {
            return Err(
                format!("IMap CI is missing required matrix/runner guard: {required}").into(),
            );
        }
    }
    if artifact_count != 2 {
        return Err(
            "IMap CI must separate matrix receipts from bounded failure diagnostics".into(),
        );
    }
    let complete = jobs["complete"]["steps"]
        .as_sequence()
        .ok_or("completeness steps required")?;
    if !complete.iter().any(|step| {
        step["env"]["LANES_RESULT"].as_str() == Some("${{ needs.qualification.result }}")
            && step["run"]
                .as_str()
                .is_some_and(|run| run.contains("test \"$LANES_RESULT\" = success"))
    }) {
        return Err("CI completeness gate must check the actual matrix job verdict".into());
    }
    Ok(())
}

pub fn validate_receipts(
    receipts: &[CiReceipt],
    mode: &str,
    source: &str,
    workflow_digest: &str,
) -> Result<(), Box<dyn Error>> {
    let expected = expected_lanes(mode)?.into_iter().collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    for receipt in receipts {
        if !seen.insert(receipt.lane.clone())
            || receipt.release != "0.75"
            || receipt.receipt_version != 2
            || receipt.scope != "local_ci"
            || receipt.mode != mode
            || receipt.source_sha != source
            || !hex(source, 40)
            || receipt.workflow_sha256 != workflow_digest
            || !hex(workflow_digest, 64)
            || !receipt.passed
            || !receipt.completed
            || receipt.production_enabled
            || receipt.steps != 128
            || receipt.seeds != seeds_for(receipt.lane.shard)?
            || receipt.commands != COMMANDS.into_iter().map(str::to_owned).collect::<Vec<_>>()
            || !receipt
                .rust_fingerprint
                .starts_with(&format!("rustc {} ", receipt.lane.rust))
            || !java_major(&receipt.java_fingerprint, receipt.lane.java)
        {
            return Err("invalid, duplicate, failed, partial or mixed-source CI receipt".into());
        }
    }
    if seen != expected {
        return Err("CI matrix is incomplete or contains an undeclared lane".into());
    }
    Ok(())
}

pub fn check_receipts(root: &Path, directory: &str, mode: &str) -> Result<(), Box<dyn Error>> {
    let relative = Path::new(directory);
    if relative
        .components()
        .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        return Err("receipt directory must stay in workspace".into());
    }
    let base = root.canonicalize()?;
    let dir = base.join(relative).canonicalize()?;
    if !dir.starts_with(&base)
        || fs::symlink_metadata(base.join(relative))?
            .file_type()
            .is_symlink()
    {
        return Err("unsafe CI receipt directory".into());
    }
    let mut references = Vec::new();
    collect_receipts(&base, &dir, 0, &mut references)?;
    let mut receipts = Vec::new();
    for path in references {
        receipts.push(serde_json::from_slice(&read_relative(
            root,
            &path,
            16 * 1024,
        )?)?);
    }
    validate_receipts(
        &receipts,
        mode,
        &source_state(root)?,
        &digest(&read_relative(root, WORKFLOW, MAX_ARTIFACT_BYTES)?),
    )
}
fn collect_receipts(
    base: &Path,
    dir: &Path,
    depth: usize,
    result: &mut Vec<String>,
) -> Result<(), Box<dyn Error>> {
    if depth > 2 {
        return Err("CI artifact nesting exceeds bound".into());
    }
    let entries = fs::read_dir(dir)?
        .take(129)
        .collect::<Result<Vec<_>, _>>()?;
    if entries.len() > 128 {
        return Err("CI artifact directory exceeds entry bound".into());
    }
    for entry in entries {
        if entry.file_type()?.is_symlink() {
            return Err("symlinked CI artifacts refused".into());
        }
        if entry.file_type()?.is_dir() {
            collect_receipts(base, &entry.path(), depth + 1, result)?;
        } else if entry
            .file_name()
            .to_string_lossy()
            .starts_with("ci-receipt-")
            && entry.path().extension().is_some_and(|v| v == "json")
        {
            if result.len() >= 16 {
                return Err("CI receipt count exceeds matrix bound".into());
            }
            result.push(
                entry
                    .path()
                    .strip_prefix(base)?
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
        }
    }
    Ok(())
}
pub fn write_receipt(
    root: &Path,
    relative: &str,
    receipt: &CiReceipt,
) -> Result<(), Box<dyn Error>> {
    let path = Path::new(relative);
    if relative.is_empty()
        || relative.contains('\\')
        || path
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        return Err("unsafe CI output path".into());
    }
    let base = root.canonicalize()?;
    let output = base.join(path);
    let mut parent = base.clone();
    for part in path.parent().ok_or("output parent required")?.components() {
        parent.push(part);
        if parent.exists() {
            if fs::symlink_metadata(&parent)?.file_type().is_symlink()
                || !parent.canonicalize()?.starts_with(&base)
            {
                return Err("CI output leaves workspace".into());
            }
        } else {
            fs::create_dir(&parent)?;
        }
    }
    let bytes = serde_json::to_vec_pretty(receipt)?;
    if bytes.len() > 16 * 1024 {
        return Err("CI receipt exceeds byte bound".into());
    }
    if output.exists() {
        if read_relative(root, relative, 16 * 1024)? != bytes {
            return Err("conflicting CI receipt is never overwritten".into());
        }
        return Ok(());
    }
    OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(output)?
        .write_all(&bytes)?;
    Ok(())
}
pub(crate) fn source_state(root: &Path) -> Result<String, Box<dyn Error>> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()?;
    let source = String::from_utf8(output.stdout)?.trim().to_owned();
    let status = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(root)
        .output()?;
    if !output.status.success()
        || !hex(&source, 40)
        || !status.status.success()
        || !status.stdout.is_empty()
    {
        return Err("CI receipts require exact clean source".into());
    }
    Ok(source)
}
fn fingerprint(program: &str, args: &[&str]) -> Result<String, Box<dyn Error>> {
    let output = Command::new(program).args(args).output()?;
    if !output.status.success() {
        return Err("toolchain fingerprint command failed".into());
    }
    let text = format!(
        "{}{}",
        String::from_utf8(output.stdout)?,
        String::from_utf8(output.stderr)?
    );
    if text.len() > 4096 {
        return Err("toolchain fingerprint exceeds bound".into());
    }
    Ok(text.trim().to_owned())
}
fn java_major(fingerprint: &str, expected: u32) -> bool {
    fingerprint.lines().next().is_some_and(|line| {
        line.starts_with(&format!("openjdk {expected}."))
            || line.starts_with(&format!("java {expected}."))
            || line.starts_with(&format!("openjdk version \"{expected}."))
    })
}
