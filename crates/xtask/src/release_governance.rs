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
        canary_check::check_canary_registry(root)?,
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
        .extend(release_execution_wiring_problems(&workflow)?);
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
    Ok(report)
}

pub fn release_execution_wiring_problems(text: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let workflow = parse_workflow(text)?;
    let mut problems = Vec::new();
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
}

fn parse_workflow(text: &str) -> Result<WorkflowShape, Box<dyn Error>> {
    let root: Value = serde_yaml::from_str(text)?;
    let jobs = mapping_value(root.as_mapping(), "jobs")
        .and_then(Value::as_mapping)
        .ok_or("workflow has no jobs mapping")?;
    let mut shape = WorkflowShape::default();
    for (job_id, job) in jobs {
        let Some(job_id) = job_id.as_str() else {
            continue;
        };
        let mut steps = BTreeSet::new();
        if let Some(sequence) =
            mapping_value(job.as_mapping(), "steps").and_then(Value::as_sequence)
        {
            for step in sequence {
                if let Some(name) = mapping_value(step.as_mapping(), "name").and_then(Value::as_str)
                {
                    steps.insert(name.to_owned());
                }
            }
        }
        shape.jobs.insert(job_id.to_owned(), steps);
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
