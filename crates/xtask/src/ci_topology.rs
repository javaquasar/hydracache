use serde::Deserialize;
use serde_yaml::{Mapping, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::Path;

const DEFAULT_TOPOLOGY: &str = "docs/testing/memory/0.71/ci-topology.json";
const JOB_CLASSES: [&str; 5] = [
    "core",
    "release-only",
    "scheduled-diagnostic",
    "manual-protected",
    "publish",
];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CiTopology {
    schema_version: u32,
    release: String,
    publication_producer: String,
    workflows: Vec<WorkflowContract>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkflowContract {
    path: String,
    purpose: String,
    triggers: Vec<String>,
    concurrency_markers: Vec<String>,
    #[serde(default)]
    branch_tag_disposition: Option<BranchTagDisposition>,
    classes: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    dependencies: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    timeout_exemptions: BTreeMap<String, String>,
    #[serde(default)]
    artifact_identity_exemptions: BTreeMap<String, ExpiringException>,
    #[serde(default)]
    artifact_budget: Option<ArtifactBudget>,
    #[serde(default)]
    watchdog_jobs: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpiringException {
    reason: String,
    expires_after_release: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BranchTagDisposition {
    policy: String,
    reason: String,
    admission_job: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ArtifactBudget {
    max_retention_days: u64,
    max_uploads_per_job: usize,
    max_bytes_per_artifact: u64,
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let mut root = None;
    let mut release = None;
    let mut topology = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--root" => root = Some(take_value(&args, &mut index, "--root")?.into()),
            "--release" => release = Some(take_value(&args, &mut index, "--release")?),
            "--topology" => topology = Some(take_value(&args, &mut index, "--topology")?.into()),
            other => return Err(format!("unsupported ci-topology-check argument: {other}").into()),
        }
        index += 1;
    }

    let root = root.unwrap_or(crate::doc_check::find_repo_root()?);
    let release = release.ok_or("ci-topology-check requires --release <release>")?;
    let topology = topology.unwrap_or_else(|| root.join(DEFAULT_TOPOLOGY));
    check_with_path(&root, &release, &topology)?;
    println!(
        "CI topology contract for release {release} is valid ({})",
        topology.display()
    );
    Ok(())
}

fn take_value(args: &[String], index: &mut usize, flag: &str) -> Result<String, Box<dyn Error>> {
    *index += 1;
    args.get(*index)
        .cloned()
        .ok_or_else(|| format!("{flag} requires a value").into())
}

pub fn check(root: &Path, release: &str) -> Result<(), Box<dyn Error>> {
    check_with_path(root, release, &root.join(DEFAULT_TOPOLOGY))
}

pub fn check_with_path(root: &Path, release: &str, path: &Path) -> Result<(), Box<dyn Error>> {
    let bytes = fs::read(path)
        .map_err(|error| format!("failed to read topology {}: {error}", path.display()))?;
    let topology: CiTopology = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid topology JSON {}: {error}", path.display()))?;
    let mut errors = Vec::new();

    if topology.schema_version != 1 {
        errors.push(format!(
            "schema_version must be 1, found {}",
            topology.schema_version
        ));
    }
    if topology.release != release {
        errors.push(format!(
            "topology release {} does not match requested release {release}",
            topology.release
        ));
    }

    let actual_paths = discover_workflow_paths(root)?;
    let declared_paths: BTreeSet<_> = topology
        .workflows
        .iter()
        .map(|workflow| workflow.path.clone())
        .collect();
    if actual_paths != declared_paths {
        report_set_delta(
            "workflow inventory",
            &declared_paths,
            &actual_paths,
            &mut errors,
        );
    }

    let mut detected_publishers = Vec::new();
    for contract in &topology.workflows {
        validate_workflow(
            root,
            release,
            contract,
            &mut detected_publishers,
            &mut errors,
        );
    }

    detected_publishers.sort();
    detected_publishers.dedup();
    if detected_publishers != vec![topology.publication_producer.clone()] {
        errors.push(format!(
            "crate publication must have exactly one producer {}; detected {:?}",
            topology.publication_producer, detected_publishers
        ));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("CI topology validation failed:\n- {}", errors.join("\n- ")).into())
    }
}

fn validate_workflow(
    root: &Path,
    release: &str,
    contract: &WorkflowContract,
    detected_publishers: &mut Vec<String>,
    errors: &mut Vec<String>,
) {
    if contract.purpose.trim().is_empty() {
        errors.push(format!("{} has an empty purpose", contract.path));
    }
    let path = root.join(&contract.path);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => {
            errors.push(format!("failed to read {}: {error}", path.display()));
            return;
        }
    };
    let document: Value = match serde_yaml::from_str(&text) {
        Ok(document) => document,
        Err(error) => {
            errors.push(format!("invalid workflow YAML {}: {error}", contract.path));
            return;
        }
    };
    let mapping = match document.as_mapping() {
        Some(mapping) => mapping,
        None => {
            errors.push(format!("{} is not a YAML mapping", contract.path));
            return;
        }
    };

    let actual_triggers = discover_triggers(mapping);
    let declared_triggers: BTreeSet<_> = contract.triggers.iter().cloned().collect();
    if actual_triggers != declared_triggers {
        report_set_delta(
            &format!("{} triggers", contract.path),
            &declared_triggers,
            &actual_triggers,
            errors,
        );
    }
    if contract.concurrency_markers.is_empty() {
        errors.push(format!(
            "{} must declare concurrency identity markers",
            contract.path
        ));
    }
    let concurrency = get(mapping, "concurrency")
        .map(value_text)
        .unwrap_or_default();
    for marker in &contract.concurrency_markers {
        if !concurrency.contains(marker) {
            errors.push(format!(
                "{} concurrency does not contain marker {marker:?}",
                contract.path
            ));
        }
    }

    let jobs = match get(mapping, "jobs").and_then(Value::as_mapping) {
        Some(jobs) => jobs,
        None => {
            errors.push(format!("{} has no jobs mapping", contract.path));
            return;
        }
    };
    let declared_jobs = validate_classes(contract, errors);
    let actual_jobs: BTreeSet<_> = jobs
        .keys()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    if declared_jobs != actual_jobs {
        report_set_delta(
            &format!("{} jobs", contract.path),
            &declared_jobs,
            &actual_jobs,
            errors,
        );
    }

    if actual_triggers.contains("push_branch") && actual_triggers.contains("push_tag") {
        match &contract.branch_tag_disposition {
            Some(disposition)
                if matches!(
                    disposition.policy.as_str(),
                    "reuse-sha-core" | "intentional-rerun"
                ) && !disposition.reason.trim().is_empty()
                    && contract
                        .classes
                        .get("release-only")
                        .is_some_and(|jobs| jobs.contains(&disposition.admission_job)) => {}
            Some(disposition) => errors.push(format!(
                "{} has invalid branch/tag disposition policy {:?} or admission job {:?}",
                contract.path, disposition.policy, disposition.admission_job
            )),
            None => errors.push(format!(
                "{} duplicates branch/tag triggers without an explicit reuse/rerun disposition",
                contract.path
            )),
        }
    } else if contract.branch_tag_disposition.is_some() {
        errors.push(format!(
            "{} declares a branch/tag disposition without both triggers",
            contract.path
        ));
    }

    for (job_id, job_value) in jobs {
        let Some(job_id) = job_id.as_str() else {
            continue;
        };
        let Some(job) = job_value.as_mapping() else {
            errors.push(format!("{}#{job_id} is not a mapping", contract.path));
            continue;
        };
        validate_timeout(contract, job_id, job, errors);
        validate_command_timeouts(contract, job_id, job, errors);
        validate_dependencies(contract, job_id, job, errors);
        validate_artifacts(contract, release, job_id, job, errors);

        let job_text = value_text(job_value);
        if has_cargo_publish(job) {
            detected_publishers.push(format!("{}#{job_id}", contract.path));
        }
        if contract
            .watchdog_jobs
            .iter()
            .any(|candidate| candidate == job_id)
            && !job_text.contains("scripts/ci/run-with-heartbeat.py")
        {
            errors.push(format!(
                "{}#{job_id} is declared as watchdog-protected but does not invoke the watchdog",
                contract.path
            ));
        }
    }

    for watchdog_job in &contract.watchdog_jobs {
        if !actual_jobs.contains(watchdog_job) {
            errors.push(format!(
                "{} watchdog job {watchdog_job} does not exist",
                contract.path
            ));
        }
    }
}

fn validate_classes(contract: &WorkflowContract, errors: &mut Vec<String>) -> BTreeSet<String> {
    let allowed: BTreeSet<_> = JOB_CLASSES
        .iter()
        .map(|class| (*class).to_owned())
        .collect();
    let actual: BTreeSet<_> = contract.classes.keys().cloned().collect();
    if actual != allowed {
        report_set_delta(
            &format!("{} job classes", contract.path),
            &allowed,
            &actual,
            errors,
        );
    }
    let mut jobs = BTreeSet::new();
    for (class, class_jobs) in &contract.classes {
        for job in class_jobs {
            if !jobs.insert(job.clone()) {
                errors.push(format!(
                    "{}#{job} appears in more than one class (including {class})",
                    contract.path
                ));
            }
        }
    }
    jobs
}

fn validate_timeout(
    contract: &WorkflowContract,
    job_id: &str,
    job: &Mapping,
    errors: &mut Vec<String>,
) {
    let timeout = get(job, "timeout-minutes").and_then(Value::as_u64);
    if timeout.is_some_and(|minutes| minutes > 0) {
        return;
    }
    let exemption = contract.timeout_exemptions.get(job_id);
    let reusable = get(job, "uses").and_then(Value::as_str).is_some();
    match (exemption, reusable) {
        (Some(reason), true) if !reason.trim().is_empty() => {}
        (Some(_), false) => errors.push(format!(
            "{}#{job_id} has a timeout exemption but is not a reusable-workflow job",
            contract.path
        )),
        _ => errors.push(format!(
            "{}#{job_id} must declare a positive timeout-minutes",
            contract.path
        )),
    }
}

fn validate_command_timeouts(
    contract: &WorkflowContract,
    job_id: &str,
    job: &Mapping,
    errors: &mut Vec<String>,
) {
    let Some(job_timeout) = get(job, "timeout-minutes").and_then(Value::as_u64) else {
        return;
    };
    let Some(steps) = get(job, "steps").and_then(Value::as_sequence) else {
        return;
    };
    for (index, step) in steps.iter().enumerate() {
        let Some(step) = step.as_mapping() else {
            continue;
        };
        let Some(command) = get(step, "run").and_then(Value::as_str) else {
            continue;
        };
        let name = get(step, "name")
            .and_then(Value::as_str)
            .unwrap_or("unnamed run step");
        let lower = name.to_ascii_lowercase();
        let time_sensitive = [
            "install",
            "browser",
            "playwright",
            "profiler",
            "soak",
            "child process",
            "child-process",
        ]
        .iter()
        .any(|marker| lower.contains(marker));
        if !time_sensitive {
            continue;
        }
        let step_timeout = get(step, "timeout-minutes").and_then(Value::as_u64);
        let command_has_deadline = command.contains("run-with-heartbeat.py")
            || command.lines().any(|line| {
                let line = line.trim();
                line.starts_with("timeout ") || line.contains(" timeout ")
            });
        if !command_has_deadline
            && !step_timeout.is_some_and(|minutes| minutes > 0 && minutes < job_timeout)
        {
            errors.push(format!(
                "{}#{job_id} step {} ({name:?}) needs a positive timeout-minutes smaller than job timeout {job_timeout}",
                contract.path,
                index + 1
            ));
        }
    }
}

fn validate_dependencies(
    contract: &WorkflowContract,
    job_id: &str,
    job: &Mapping,
    errors: &mut Vec<String>,
) {
    let actual = string_list(get(job, "needs"));
    let expected = contract
        .dependencies
        .get(job_id)
        .cloned()
        .unwrap_or_default();
    let actual: BTreeSet<_> = actual.into_iter().collect();
    let expected: BTreeSet<_> = expected.into_iter().collect();
    if actual != expected {
        report_set_delta(
            &format!("{}#{job_id} dependencies", contract.path),
            &expected,
            &actual,
            errors,
        );
    }
}

fn validate_artifacts(
    contract: &WorkflowContract,
    release: &str,
    job_id: &str,
    job: &Mapping,
    errors: &mut Vec<String>,
) {
    let Some(steps) = get(job, "steps").and_then(Value::as_sequence) else {
        return;
    };
    let mut upload_count = 0;
    for step in steps {
        let Some(step) = step.as_mapping() else {
            continue;
        };
        let Some(action) = get(step, "uses").and_then(Value::as_str) else {
            continue;
        };
        if !action.starts_with("actions/upload-artifact@") {
            continue;
        }
        upload_count += 1;
        match &contract.artifact_budget {
            Some(budget) if budget.max_bytes_per_artifact == 0 => errors.push(format!(
                "{} artifact budget must declare a positive max_bytes_per_artifact",
                contract.path
            )),
            Some(budget) => {
                let retention = get(step, "with")
                    .and_then(Value::as_mapping)
                    .and_then(|with| get(with, "retention-days"))
                    .and_then(Value::as_u64)
                    .unwrap_or(90);
                if retention == 0 || retention > budget.max_retention_days {
                    errors.push(format!(
                        "{}#{job_id} artifact retention {} must be 1..={} days (GitHub default is 90)",
                        contract.path, retention, budget.max_retention_days
                    ));
                }
            }
            None => errors.push(format!(
                "{}#{job_id} uploads artifacts without a workflow artifact_budget",
                contract.path
            )),
        }
        let name = get(step, "with")
            .and_then(Value::as_mapping)
            .and_then(|with| get(with, "name"))
            .map(value_text)
            .unwrap_or_default();
        let lower = name.to_ascii_lowercase();
        let complete_identity = lower.contains("github.sha")
            && lower.contains("github.run_id")
            && lower.contains("github.run_attempt");
        if complete_identity {
            continue;
        }
        match contract.artifact_identity_exemptions.get(job_id) {
            Some(exception)
                if !exception.reason.trim().is_empty()
                    && release_is_after(&exception.expires_after_release, release) => {}
            Some(exception) => errors.push(format!(
                "{}#{job_id} has an invalid/expired artifact identity exemption (expires after {})",
                contract.path, exception.expires_after_release
            )),
            None => errors.push(format!(
                "{}#{job_id} upload artifact name {name:?} must contain github.sha, github.run_id, and github.run_attempt",
                contract.path
            )),
        }
    }
    if let Some(budget) = &contract.artifact_budget {
        if upload_count > budget.max_uploads_per_job {
            errors.push(format!(
                "{}#{job_id} uploads {upload_count} artifacts, exceeding per-job budget {}",
                contract.path, budget.max_uploads_per_job
            ));
        }
    }
}

fn has_cargo_publish(job: &Mapping) -> bool {
    get(job, "steps")
        .and_then(Value::as_sequence)
        .into_iter()
        .flatten()
        .filter_map(Value::as_mapping)
        .filter_map(|step| get(step, "run").and_then(Value::as_str))
        .flat_map(str::lines)
        .map(str::trim)
        .any(|line| !line.starts_with('#') && line.starts_with("cargo publish"))
}

fn discover_workflow_paths(root: &Path) -> Result<BTreeSet<String>, Box<dyn Error>> {
    let workflow_dir = root.join(".github/workflows");
    let mut paths = BTreeSet::new();
    for entry in fs::read_dir(&workflow_dir)? {
        let entry = entry?;
        let path = entry.path();
        if matches!(
            path.extension().and_then(|value| value.to_str()),
            Some("yml" | "yaml")
        ) {
            let relative = path
                .strip_prefix(root)?
                .to_string_lossy()
                .replace('\\', "/");
            paths.insert(relative);
        }
    }
    Ok(paths)
}

fn discover_triggers(root: &Mapping) -> BTreeSet<String> {
    let mut result = BTreeSet::new();
    let on = get(root, "on").or_else(|| root.get(Value::Bool(true)));
    let Some(on) = on else {
        return result;
    };
    if let Some(event) = on.as_str() {
        result.insert(trigger_name(event, None));
        return result;
    }
    if let Some(events) = on.as_sequence() {
        for event in events.iter().filter_map(Value::as_str) {
            result.insert(trigger_name(event, None));
        }
        return result;
    }
    let Some(events) = on.as_mapping() else {
        return result;
    };
    for (event, configuration) in events {
        let Some(event) = event.as_str() else {
            continue;
        };
        if event == "push" {
            let configuration = configuration.as_mapping();
            if configuration
                .and_then(|value| get(value, "branches"))
                .is_some()
            {
                result.insert("push_branch".to_owned());
            }
            if configuration.and_then(|value| get(value, "tags")).is_some() {
                result.insert("push_tag".to_owned());
            }
            if configuration.is_none() {
                result.insert("push".to_owned());
            }
        } else {
            result.insert(trigger_name(event, Some(configuration)));
        }
    }
    result
}

fn trigger_name(event: &str, _configuration: Option<&Value>) -> String {
    event.to_owned()
}

fn string_list(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::String(value)) => vec![value.clone()],
        Some(Value::Sequence(values)) => values
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        _ => Vec::new(),
    }
}

fn get<'a>(mapping: &'a Mapping, key: &str) -> Option<&'a Value> {
    mapping.get(Value::String(key.to_owned()))
}

fn value_text(value: &Value) -> String {
    serde_yaml::to_string(value).unwrap_or_default()
}

fn release_is_after(candidate: &str, current: &str) -> bool {
    release_parts(candidate) > release_parts(current)
}

fn release_parts(release: &str) -> Vec<u64> {
    release
        .split('.')
        .map(|part| part.parse::<u64>().unwrap_or_default())
        .collect()
}

fn report_set_delta(
    label: &str,
    expected: &BTreeSet<String>,
    actual: &BTreeSet<String>,
    errors: &mut Vec<String>,
) {
    let missing: Vec<_> = expected.difference(actual).cloned().collect();
    let unexpected: Vec<_> = actual.difference(expected).cloned().collect();
    errors.push(format!(
        "{label} differs: missing {missing:?}; unexpected {unexpected:?}"
    ));
}

#[cfg(test)]
mod tests {
    use super::release_is_after;

    #[test]
    fn release_expiration_is_numeric() {
        assert!(release_is_after("0.72", "0.71"));
        assert!(release_is_after("0.71.1", "0.71"));
        assert!(!release_is_after("0.71", "0.71"));
        assert!(!release_is_after("0.70", "0.71"));
    }
}
