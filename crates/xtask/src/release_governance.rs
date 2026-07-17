use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use serde_yaml::{Mapping, Value};

use crate::canary_check;
use crate::compat_check;
use crate::coverage_ratchet;
use crate::doc_check;
use crate::fast_suite;
use crate::feature_leak;
use crate::gated_tests::{self, GateEntry};
use crate::miri_check;
use crate::quarantine;
use crate::raft_spec_check;

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let (root, release) = parse_args(args)?;
    let report = check(&root, &release)?;
    for todo in &report.todos {
        println!("release-governance-check: TODO {todo}");
    }
    if report.problems.is_empty() {
        println!(
            "release-governance-check: OK ({} checks, {} explicit TODOs)",
            report.completed_checks,
            report.todos.len()
        );
        Ok(())
    } else {
        for problem in &report.problems {
            eprintln!("release-governance-check: {problem}");
        }
        Err(format!(
            "release-governance-check found {} problem(s)",
            report.problems.len()
        )
        .into())
    }
}

#[derive(Debug, Default)]
pub struct GovernanceReport {
    pub completed_checks: usize,
    pub todos: Vec<String>,
    pub problems: Vec<String>,
}

pub fn check(root: &Path, release: &str) -> Result<GovernanceReport, Box<dyn Error>> {
    let mut report = GovernanceReport::default();

    report
        .problems
        .extend(prefix("doc-check", doc_check::check(root)?));
    report.completed_checks += 1;

    report.problems.extend(prefix(
        "coverage-ratchet-check",
        coverage_ratchet::validate_contract(root, &coverage_ratchet::load_config(root)?)?,
    ));
    report.completed_checks += 1;

    report
        .problems
        .extend(prefix("compat-check", compat_check::manifest_check(root)?));
    report.completed_checks += 1;

    report
        .problems
        .extend(feature_leak::check(root)?.into_iter().map(|leak| {
            format!(
                "verify-no-test-features: {} contains {} ({})",
                leak.package, leak.marker, leak.reason
            )
        }));
    report.completed_checks += 1;

    report.problems.extend(prefix(
        "canary-check",
        canary_check::check_canary_registry_for_release(root, release)?,
    ));
    report.completed_checks += 1;

    report.problems.extend(prefix(
        "gated-test-check",
        gated_tests::check_registry(root)?,
    ));
    report.completed_checks += 1;

    let quarantine = quarantine::check_at(root, release, time::OffsetDateTime::now_utc())?;
    report
        .problems
        .extend(prefix("quarantine-check", quarantine.problems));
    report.completed_checks += 1;

    let fast = fast_suite::load_registry(root)?;
    report.problems.extend(prefix(
        "fast-suite-check",
        fast_suite::validate_registry(root, &fast, release, None)?,
    ));
    report.completed_checks += 1;

    let gates = gated_tests::load_registry(root)?;
    report
        .problems
        .extend(ci_wiring_problems(root, &gates.gate)?);
    report.completed_checks += 1;

    report.problems.extend(prefix(
        "raft-spec-check",
        raft_spec_check::structural_check(root)?,
    ));
    report.completed_checks += 1;

    if let Err(error) = miri_check::structural_check(root) {
        report.problems.push(format!("miri-check: {error}"));
    }
    report.completed_checks += 1;

    let workflow = fs::read_to_string(root.join(".github/workflows/ci.yml"))?;
    report
        .problems
        .extend(release_execution_wiring_problems(&workflow, release)?);
    report.completed_checks += 1;
    for required in [
        "canary-sweep --release 0.64 --tier fast",
        "dynamic-canary-sweep:",
        "canary-sweep --release 0.64 --tier all",
    ] {
        if !workflow.contains(required) {
            report
                .problems
                .push(format!("canary-sweep CI wiring is missing `{required}`"));
        }
    }
    report.completed_checks += 1;

    let publish_workflow = fs::read_to_string(root.join(".github/workflows/publish-crates.yml"))?;
    report.problems.extend(prefix(
        "publish-workflow-check",
        publish_workflow_problems(&publish_workflow),
    ));
    report.completed_checks += 1;

    let post_publish_workflow =
        fs::read_to_string(root.join(".github/workflows/post-publish.yml"))?;
    let post_publish_fixture =
        fs::read_to_string(root.join("tests/post-publish-consumer/src/lib.rs"))?;
    report.problems.extend(prefix(
        "post-publish-workflow-check",
        post_publish_contract_problems(&post_publish_workflow, &post_publish_fixture),
    ));
    report.completed_checks += 1;
    Ok(report)
}

pub fn publish_workflow_problems(text: &str) -> Vec<String> {
    let mut problems = Vec::new();
    for required in [
        "crates_io_status()",
        "--user-agent",
        "GITHUB_REPOSITORY",
        "status=\"$(crates_io_status \"$package\")\"",
        "429|5??)",
        "if dependency_id in publishable_ids:",
    ] {
        if !text.contains(required) {
            problems.push(format!(
                "crates.io publication probe is missing `{required}`"
            ));
        }
    }
    if text.contains("kind.get(\"kind\") is None") {
        problems.push(
            "publish order filters out packaged dev/build dependencies; cargo publish must see every workspace dependency in the registry first"
                .to_owned(),
        );
    }
    problems
}

pub fn post_publish_contract_problems(workflow: &str, fixture: &str) -> Vec<String> {
    let mut problems = Vec::new();
    for required in [
        "actions/checkout@v5",
        "tests/post-publish-consumer/src/lib.rs",
    ] {
        if !workflow.contains(required) {
            problems.push(format!(
                "post-publish workflow is missing checked consumer fixture wiring `{required}`"
            ));
        }
    }
    for required in [
        "cluster.ownership_diagnostics()",
        "ownership_diagnostics.resolutions",
        "ownership_diagnostics.no_owner",
        "Vec::from(\"encoded-user\").into()",
        ".diesel_one(||",
        ".sea_one(|| async",
    ] {
        if !fixture.contains(required) {
            problems.push(format!(
                "published consumer smoke is missing current API `{required}`"
            ));
        }
    }
    for obsolete in [
        "cluster_diagnostics.ownership_resolutions",
        "cluster_diagnostics.ownership_no_owner",
        ".diesel_first(",
        ".sea_value(",
    ] {
        if fixture.contains(obsolete) {
            problems.push(format!(
                "published consumer smoke still references obsolete API `{obsolete}`"
            ));
        }
    }
    problems
}

pub fn release_execution_wiring_problems(
    text: &str,
    requested_release: &str,
) -> Result<Vec<String>, Box<dyn Error>> {
    let workflow = parse_workflow(text)?;
    let mut problems = Vec::new();
    problems.extend(release_history_checkout_problems(text)?);
    problems.extend(release_scoped_fast_wiring_problems(
        &workflow,
        requested_release,
    ));
    for (job, required_steps) in [
        (
            "rust",
            &[
                "Raft nemesis membership fast",
                "Raft corpus vectors",
                "Snapshot corruption",
                "Raft rejoin after compaction",
                "Raft snapshot resource faults",
                "Snapshot exhaustive grid",
                "Proposal idempotency",
                "Clock skew safety",
            ][..],
        ),
        (
            "raft-corner-case-nightly",
            &[
                "Raft nemesis soak",
                "Snapshot exhaustive grid wide",
                "Rejoin after compaction proof",
                "Snapshot resource faults",
                "Clock skew safety",
                "Upload raft corner-case artifacts",
            ][..],
        ),
        ("dst-nightly-soak", &["Run daemon-process cluster tier"][..]),
    ] {
        let Some(steps) = workflow.jobs.get(job) else {
            problems.push(format!("release execution matrix is missing job {job}"));
            continue;
        };
        for step in required_steps {
            if !steps.contains(*step) {
                problems.push(format!("job {job} is missing required step {step:?}"));
            }
        }
    }
    for job in ["raft-corner-case-nightly", "dst-nightly-soak"] {
        let condition = workflow
            .conditions
            .get(job)
            .map(String::as_str)
            .unwrap_or("");
        if !condition.contains("schedule") || !condition.contains("workflow_dispatch") {
            problems.push(format!(
                "heavy job {job} must be gated by schedule or workflow_dispatch"
            ));
        }
    }
    const REDIS_MULTINODE_EVIDENCE: &str = "cargo run -p xtask --locked -- evidence-run --release 0.65 --gate env.hydracache-run-redis-resp-multinode-e2e";
    match workflow
        .step_runs
        .get("dst-nightly-soak")
        .and_then(|steps| steps.get("Redis RESP multinode debt sentinels"))
        .map(|run| run.trim())
    {
        Some(run) if run == REDIS_MULTINODE_EVIDENCE => {}
        _ => problems.push(format!(
            "job dst-nightly-soak step Redis RESP multinode debt sentinels must run exactly `{REDIS_MULTINODE_EVIDENCE}`"
        )),
    }
    for required in [
        "evidence-run --release 0.64 --gate env.hydracache-run-raft-nemesis-soak",
        "evidence-run --release 0.64 --gate env.hydracache-grid-scope",
        "evidence-run --release 0.64 --gate cfg.hydracache-cluster-raft.rejoin-after-compaction",
        "evidence-run --release 0.64 --gate cfg.hydracache-cluster-raft.snapshot-resource-faults",
        "evidence-run --release 0.64 --gate env.hydracache-run-daemon-process-e2e",
    ] {
        if !text.contains(required) {
            problems.push(format!("release execution matrix is missing `{required}`"));
        }
    }
    problems.extend(fuzz_nightly_wiring_problems(text));
    Ok(problems)
}

fn release_scoped_fast_wiring_problems(
    workflow: &WorkflowShape,
    requested_release: &str,
) -> Vec<String> {
    let release = normalize_release(requested_release);
    let mut problems = Vec::new();
    for (step, command) in [
        (
            "Canary completeness",
            format!("cargo run -p xtask --locked -- canary-check --release {release}"),
        ),
        (
            "Canary completeness",
            format!("cargo run -p xtask --locked -- canary-sweep --release {release} --tier fast"),
        ),
    ] {
        let wired = workflow
            .step_runs
            .get("rust")
            .and_then(|steps| steps.get(step))
            .is_some_and(|run| run.lines().any(|line| line.trim() == command));
        if !wired {
            problems.push(format!(
                "release {release} fast job step {step:?} is missing exact command `{command}`"
            ));
        }
    }
    problems.extend(candidate_receipt_wiring_problems(workflow, release));

    if release == "0.66" {
        const W0_STEP: &str = "Raft compaction control 0.66";
        const W0_COMMANDS: &str = concat!(
            "cargo test -p hydracache-cluster-raft --test compaction_seam --locked\n",
            "cargo run -p xtask --locked -- evidence-run --release \"$HYDRACACHE_CANDIDATE_RELEASE\" --gate fast.raft-sled-snapshot\n",
            "cargo test -p hydracache-server compaction --locked"
        );
        let exact_w0_step = workflow
            .step_runs
            .get("rust")
            .and_then(|steps| steps.get(W0_STEP))
            .is_some_and(|run| run.trim() == W0_COMMANDS);
        if !exact_w0_step {
            problems.push(format!(
                "release 0.66 fast job must contain exact {W0_STEP:?} commands for the default, receipt-bound Sled, and server compaction proofs"
            ));
        }
    }
    problems
}

fn candidate_receipt_wiring_problems(
    workflow: &WorkflowShape,
    requested_release: &str,
) -> Vec<String> {
    const DEFAULT_RELEASE: &str = "0.66";
    const RELEASE_ENV: &str = "HYDRACACHE_CANDIDATE_RELEASE";
    const RELEASE_ENV_BINDING: &str = "${{ inputs.candidate_release || '0.66' }}";
    const FAST_RECEIPT: &str = "cargo run -p xtask --locked -- evidence-run --release \"$HYDRACACHE_CANDIDATE_RELEASE\" --gate fast.workspace-nextest";
    const GOVERNANCE: &str = "cargo run -p xtask --locked -- release-governance-check --release \"$HYDRACACHE_CANDIDATE_RELEASE\"";
    const MANUAL_RECEIPT: &str = r#"cargo run -p xtask --locked -- evidence-run --release "$HYDRACACHE_CANDIDATE_RELEASE" --gate "${{ inputs.gated_gate_id }}""#;

    let mut problems = Vec::new();
    if workflow.candidate_release_default.as_deref() != Some(DEFAULT_RELEASE) {
        problems.push(format!(
            "release {requested_release} candidate receipt wiring requires workflow_dispatch input candidate_release with default {DEFAULT_RELEASE}"
        ));
    }
    if workflow.candidate_release_env.as_deref() != Some(RELEASE_ENV_BINDING) {
        problems.push(format!(
            "release {requested_release} candidate receipt wiring requires global {RELEASE_ENV} to equal `{RELEASE_ENV_BINDING}`"
        ));
    }
    for (job, step, command, proof) in [
        ("rust", "Test", FAST_RECEIPT, "fast workspace receipt"),
        (
            "rust",
            "Release governance",
            GOVERNANCE,
            "candidate governance",
        ),
        (
            "gated-proof-registry",
            "Run registered gated proofs",
            MANUAL_RECEIPT,
            "manually dispatched gate receipt",
        ),
    ] {
        let exact = workflow
            .step_runs
            .get(job)
            .and_then(|steps| steps.get(step))
            .is_some_and(|run| run.trim() == command);
        if !exact {
            problems.push(format!(
                "release {requested_release} {proof} must use exact candidate binding `{command}`"
            ));
        }
    }
    problems
}

fn normalize_release(release: &str) -> &str {
    release.strip_suffix(".0").unwrap_or(release)
}

pub fn release_history_checkout_problems(text: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let root: Value = serde_yaml::from_str(text)?;
    let jobs = mapping_value(root.as_mapping(), "jobs")
        .and_then(Value::as_mapping)
        .ok_or("workflow has no jobs mapping")?;
    let mut problems = Vec::new();

    for job_id in [
        "rust",
        "dynamic-canary-sweep",
        "coverage-ratchet",
        "msrv",
        "gated-proof-registry",
    ] {
        let Some(job) = jobs.get(Value::String(job_id.to_owned())) else {
            problems.push(format!(
                "release compatibility proof is missing required job {job_id}"
            ));
            continue;
        };
        let checkout = mapping_value(job.as_mapping(), "steps")
            .and_then(Value::as_sequence)
            .and_then(|steps| {
                steps.iter().find(|step| {
                    mapping_value(step.as_mapping(), "uses")
                        .and_then(Value::as_str)
                        .is_some_and(|uses| uses.starts_with("actions/checkout@"))
                })
            });
        let full_history = checkout
            .and_then(|step| mapping_value(step.as_mapping(), "with"))
            .and_then(Value::as_mapping)
            .and_then(|with| mapping_value(Some(with), "fetch-depth"))
            .is_some_and(|depth| match depth {
                Value::Number(number) => number.as_u64() == Some(0),
                Value::String(value) => value == "0",
                _ => false,
            });

        if !full_history {
            problems.push(format!(
                "job {job_id} checkout must set with.fetch-depth: 0 so compatibility tag v0.63.0 and its ancestry are available"
            ));
        }
    }

    Ok(problems)
}

fn fuzz_nightly_wiring_problems(text: &str) -> Vec<String> {
    let mut problems = Vec::new();
    for required in [
        "cargo install cargo-fuzz --version 0.13.2 --locked",
        "working-directory: fuzz",
        "cargo +nightly fuzz run fuzz_config_parse -- -max_total_time=60",
        "cargo +nightly fuzz run fuzz_kv_codec -- -max_total_time=60",
        "cargo +nightly fuzz run fuzz_resp_command -- -max_total_time=60",
        "cargo +nightly fuzz run fuzz_snapshot_decode -- -max_total_time=60",
    ] {
        if !text.contains(required) {
            problems.push(format!("fuzz nightly wiring is missing `{required}`"));
        }
    }
    if text.contains("fuzz run fuzz_config_parse --manifest-path")
        || text.contains("fuzz run fuzz_kv_codec --manifest-path")
        || text.contains("fuzz run fuzz_resp_command --manifest-path")
        || text.contains("fuzz run fuzz_snapshot_decode --manifest-path")
    {
        problems.push(
            "fuzz nightly passes --manifest-path after the target; cargo-fuzz parses that position as corpus/libFuzzer arguments"
                .to_owned(),
        );
    }
    problems
}

pub fn ci_wiring_problems(root: &Path, gates: &[GateEntry]) -> Result<Vec<String>, Box<dyn Error>> {
    let mut problems = Vec::new();
    let mut workflows = BTreeMap::<String, WorkflowShape>::new();
    for gate in gates {
        let workflow = if let Some(workflow) = workflows.get(&gate.ci.workflow) {
            workflow
        } else {
            let path = root.join(&gate.ci.workflow);
            let parsed = parse_workflow(
                &fs::read_to_string(&path)
                    .map_err(|error| format!("reading workflow {}: {error}", path.display()))?,
            )?;
            workflows.insert(gate.ci.workflow.clone(), parsed);
            workflows.get(&gate.ci.workflow).expect("inserted workflow")
        };
        let Some(steps) = workflow.jobs.get(&gate.ci.job) else {
            problems.push(format!(
                "gate {} references missing job {} in {}",
                gate.id, gate.ci.job, gate.ci.workflow
            ));
            continue;
        };
        if !steps.contains(&gate.ci.step) {
            problems.push(format!(
                "gate {} references missing step {:?} in job {}",
                gate.id, gate.ci.step, gate.ci.job
            ));
        }
    }
    Ok(problems)
}

#[derive(Debug, Default)]
struct WorkflowShape {
    jobs: BTreeMap<String, BTreeSet<String>>,
    conditions: BTreeMap<String, String>,
    step_runs: BTreeMap<String, BTreeMap<String, String>>,
    candidate_release_default: Option<String>,
    candidate_release_env: Option<String>,
}

fn parse_workflow(text: &str) -> Result<WorkflowShape, Box<dyn Error>> {
    let root: Value = serde_yaml::from_str(text)?;
    let root_mapping = root.as_mapping().ok_or("workflow root is not a mapping")?;
    let jobs = mapping_value(Some(root_mapping), "jobs")
        .and_then(Value::as_mapping)
        .ok_or("workflow has no jobs mapping")?;
    let mut shape = WorkflowShape::default();
    let triggers = mapping_value(Some(root_mapping), "on")
        .or_else(|| root_mapping.get(Value::Bool(true)))
        .and_then(Value::as_mapping);
    shape.candidate_release_default = mapping_value(triggers, "workflow_dispatch")
        .and_then(Value::as_mapping)
        .and_then(|dispatch| mapping_value(Some(dispatch), "inputs"))
        .and_then(Value::as_mapping)
        .and_then(|inputs| mapping_value(Some(inputs), "candidate_release"))
        .and_then(Value::as_mapping)
        .and_then(|candidate| mapping_value(Some(candidate), "default"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    shape.candidate_release_env = mapping_value(Some(root_mapping), "env")
        .and_then(Value::as_mapping)
        .and_then(|env| mapping_value(Some(env), "HYDRACACHE_CANDIDATE_RELEASE"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    for (job_id, job) in jobs {
        let Some(job_id) = job_id.as_str() else {
            continue;
        };
        let mut steps = BTreeSet::new();
        let mut step_runs = BTreeMap::new();
        if let Some(sequence) =
            mapping_value(job.as_mapping(), "steps").and_then(Value::as_sequence)
        {
            for step in sequence {
                if let Some(name) = mapping_value(step.as_mapping(), "name").and_then(Value::as_str)
                {
                    steps.insert(name.to_owned());
                    if let Some(run) =
                        mapping_value(step.as_mapping(), "run").and_then(Value::as_str)
                    {
                        step_runs.insert(name.to_owned(), run.to_owned());
                    }
                }
            }
        }
        shape.jobs.insert(job_id.to_owned(), steps);
        shape.step_runs.insert(job_id.to_owned(), step_runs);
        if let Some(condition) = mapping_value(job.as_mapping(), "if").and_then(Value::as_str) {
            shape
                .conditions
                .insert(job_id.to_owned(), condition.to_owned());
        }
    }
    Ok(shape)
}

fn mapping_value<'a>(mapping: Option<&'a Mapping>, key: &str) -> Option<&'a Value> {
    mapping?.get(Value::String(key.to_owned()))
}

fn prefix(name: &str, problems: Vec<String>) -> impl Iterator<Item = String> + '_ {
    problems
        .into_iter()
        .map(move |problem| format!("{name}: {problem}"))
}

fn parse_args(args: Vec<String>) -> Result<(PathBuf, String), Box<dyn Error>> {
    let mut root = doc_check::find_repo_root()?;
    let mut release = None;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => root = PathBuf::from(args.next().ok_or("--root requires a path")?),
            "--release" => release = Some(args.next().ok_or("--release requires a value")?),
            other => {
                return Err(format!("unknown release-governance-check argument: {other}").into())
            }
        }
    }
    Ok((
        root,
        release.ok_or("release-governance-check requires --release")?,
    ))
}
