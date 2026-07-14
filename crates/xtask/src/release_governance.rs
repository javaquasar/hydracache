use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use serde_yaml::{Mapping, Value};

use crate::canary_check;
use crate::doc_check;
use crate::fast_suite;
use crate::feature_leak;
use crate::gated_tests::{self, GateEntry};
use crate::quarantine;

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

    for (id, path) in [
        ("TODO-W32-COMPAT-CHECK", "crates/xtask/src/compat_check.rs"),
        (
            "TODO-W38-RAFT-SPEC-CHECK",
            "crates/xtask/src/raft_spec_check.rs",
        ),
    ] {
        if root.join(path).is_file() {
            report.completed_checks += 1;
        } else {
            report.todos.push(format!("{id}: missing {path}"));
        }
    }
    Ok(report)
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
