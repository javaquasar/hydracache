//! Acquisition and aggregation of non-ship 0.67.1 reference bootstrap samples.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use hydracache_loadgen::profile::{
    reference_attestation_problems, RunnerFingerprint, REFERENCE_RUNNER_CLASS,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::perf::{
    sha256_file, verify_published_bundle, MachineAttestationReceipt, RunnerPreflightReport,
    ATTESTATION_V5_RELATIVE_PATH, RUNNER_PREFLIGHT_RELATIVE_PATH,
};
use crate::perf_qualification::{
    observe_context, trusted_performance_context_problems, QualificationContext,
};

pub const BOOTSTRAP_SAMPLE_RELATIVE_PATH: &str =
    "target/test-evidence/0.67.1/bootstrap-sample.json";
pub const BOOTSTRAP_SAMPLE_SET_RELATIVE_PATH: &str =
    "target/test-evidence/0.67.1/bootstrap-sample-set.json";
pub const RUNNER_PROVISIONING_RELATIVE_PATH: &str =
    "target/test-evidence/0.67.1/runner-provisioned.json";

const RELEASE: &str = "0.67.1";
const PROFILE: &str = "reference-v1";
const MAX_ARTIFACT_BYTES: u64 = 64 * 1024 * 1024;
const MEASUREMENT_GATES: [&str; 3] = [
    "env.hydracache-run-067-perf-core",
    "env.hydracache-run-067-perf-resp",
    "env.hydracache-run-067-perf-control-plane",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BootstrapArtifactDigest {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BootstrapSampleReceipt {
    pub schema_version: u32,
    pub release: String,
    pub profile: String,
    pub source_commit: String,
    pub github_run_id: String,
    pub runner_fingerprint: String,
    pub observed_runner: RunnerFingerprint,
    pub runner_provisioning_sha256: String,
    pub prebuild_contract_digest: String,
    pub scenario_contract_set_digest: String,
    pub sample_index: u32,
    pub admission_sha256: String,
    pub predecessor_github_run_id: Option<String>,
    pub predecessor_receipt_sha256: Option<String>,
    pub evidence_files: Vec<BootstrapArtifactDigest>,
    pub passed: bool,
    pub bootstrap_eligible: bool,
    pub ship_evidence_eligible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BootstrapSampleMember {
    pub sample_index: u32,
    pub github_run_id: String,
    pub source_commit: String,
    pub runner_fingerprint: String,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FullReferenceEvidence {
    pub source_commit: String,
    pub github_run_id: String,
    pub runner_fingerprint: String,
    pub observed_runner: RunnerFingerprint,
    pub runner_provisioning_sha256: String,
    pub prebuild_contract_digest: String,
    pub scenario_contract_set_digest: String,
    pub evidence_files: Vec<BootstrapArtifactDigest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BootstrapSampleSetReceipt {
    pub schema_version: u32,
    pub release: String,
    pub profile: String,
    pub source_commit: String,
    pub runner_fingerprint: String,
    pub runner_provisioning_sha256: String,
    pub prebuild_contract_digest: String,
    pub scenario_contract_set_digest: String,
    pub full_dress_admission_sha256: String,
    pub samples: Vec<BootstrapSampleMember>,
    pub passed: bool,
    pub bootstrap_eligible: bool,
    pub ship_evidence_eligible: bool,
}

pub fn bootstrap_context_problems(context: &QualificationContext) -> Vec<String> {
    trusted_performance_context_problems(context, "bootstrap")
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let options = Options::parse(args)?;
    let root = crate::doc_check::find_repo_root()?;
    match options.phase.as_str() {
        "context" => {
            let context = observe_context(&root)?;
            reject_context(&context)?;
            println!("0.67.1 bootstrap acquisition context: OK");
        }
        "authorize" => {
            let context = observe_context(&root)?;
            reject_context(&context)?;
            let admission = options
                .admission
                .as_deref()
                .ok_or("--admission is required for authorize phase")?;
            authorize_sample(
                &root,
                &context,
                options.sample_index,
                admission,
                options.predecessor.as_deref(),
                false,
            )?;
            println!("0.67.1 bootstrap sample admission: OK");
        }
        "sample" => {
            let context = observe_context(&root)?;
            reject_context(&context)?;
            let admission = options
                .admission
                .as_deref()
                .ok_or("--admission is required for sample phase")?;
            let receipt = build_sample(
                &root,
                &context,
                options.sample_index,
                admission,
                options.predecessor.as_deref(),
            )?;
            write_create_new_json(&root.join(BOOTSTRAP_SAMPLE_RELATIVE_PATH), &receipt)?;
            println!(
                "0.67.1 bootstrap sample retained as non-ship evidence: {}",
                root.join(BOOTSTRAP_SAMPLE_RELATIVE_PATH).display()
            );
        }
        "sample-set" => {
            let samples_dir = options
                .samples_dir
                .ok_or("--samples-dir is required for sample-set phase")?;
            let receipt = build_sample_set(&samples_dir)?;
            write_create_new_json(&root.join(BOOTSTRAP_SAMPLE_SET_RELATIVE_PATH), &receipt)?;
            println!(
                "0.67.1 bootstrap sample set validated: {}",
                root.join(BOOTSTRAP_SAMPLE_SET_RELATIVE_PATH).display()
            );
        }
        _ => unreachable!("phase parser is exhaustive"),
    }
    Ok(())
}

fn reject_context(context: &QualificationContext) -> Result<(), Box<dyn Error>> {
    let problems = bootstrap_context_problems(context);
    if problems.is_empty() {
        Ok(())
    } else {
        Err(format!("bootstrap acquisition context rejected: {problems:?}").into())
    }
}

fn build_sample(
    root: &Path,
    context: &QualificationContext,
    sample_index: Option<u32>,
    admission_path: &Path,
    predecessor_path: Option<&Path>,
) -> Result<BootstrapSampleReceipt, Box<dyn Error>> {
    let evidence = collect_full_reference_evidence(root, context)?;
    let authorization = authorize_sample(
        root,
        context,
        sample_index,
        admission_path,
        predecessor_path,
        true,
    )?;

    Ok(BootstrapSampleReceipt {
        schema_version: 2,
        release: RELEASE.to_owned(),
        profile: PROFILE.to_owned(),
        source_commit: evidence.source_commit,
        github_run_id: evidence.github_run_id,
        runner_fingerprint: evidence.runner_fingerprint,
        observed_runner: evidence.observed_runner,
        runner_provisioning_sha256: evidence.runner_provisioning_sha256,
        prebuild_contract_digest: evidence.prebuild_contract_digest,
        scenario_contract_set_digest: evidence.scenario_contract_set_digest,
        sample_index: authorization.sample_index,
        admission_sha256: authorization.admission_sha256,
        predecessor_github_run_id: authorization.predecessor_github_run_id,
        predecessor_receipt_sha256: authorization.predecessor_receipt_sha256,
        evidence_files: evidence.evidence_files,
        passed: true,
        bootstrap_eligible: true,
        ship_evidence_eligible: false,
    })
}

pub fn collect_full_reference_evidence(
    root: &Path,
    context: &QualificationContext,
) -> Result<FullReferenceEvidence, Box<dyn Error>> {
    let attestation: MachineAttestationReceipt =
        read_json(&root.join(ATTESTATION_V5_RELATIVE_PATH))?;
    let preflight: RunnerPreflightReport = read_json(&root.join(RUNNER_PREFLIGHT_RELATIVE_PATH))?;
    let attestation_problems =
        reference_attestation_problems(&attestation.observed_runner.attestation);
    if attestation.schema_version != 4
        || attestation.release != RELEASE
        || attestation.profile != PROFILE
        || !attestation.passed
        || attestation.ship_evidence_eligible
        || attestation.observed_runner.runner_class != REFERENCE_RUNNER_CLASS
        || attestation.observed_runner.shared_hardware
        || !attestation_problems.is_empty()
        || preflight.release != "0.67.0"
        || preflight.profile != PROFILE
        || !preflight.passed
        || preflight.observed_runner.fingerprint != attestation.observed_runner.fingerprint
    {
        return Err(
            format!("bootstrap host evidence is ineligible: {attestation_problems:?}").into(),
        );
    }
    let bundle = verify_published_bundle(root)?;
    if bundle.manifest.source.git_commit != context.git_head
        || bundle.manifest.runner_fingerprint != attestation.observed_runner.fingerprint
    {
        return Err("bootstrap prebuild does not bind checkout and runner fingerprint".into());
    }

    let registry = crate::gated_tests::load_registry(root)?;
    let mut paths = BTreeSet::new();
    for gate_id in MEASUREMENT_GATES {
        let gate = registry
            .gate
            .iter()
            .find(|gate| gate.id == gate_id)
            .ok_or_else(|| format!("missing measurement gate {gate_id}"))?;
        if gate.artifacts.is_empty() {
            return Err(format!("measurement gate {gate_id} has no artifacts").into());
        }
        paths.extend(gate.artifacts.iter().cloned());
    }

    let mut evidence_files = Vec::new();
    let mut scenario_contracts = Vec::new();
    let mut reference_report_count = 0_usize;
    for relative in paths {
        let path = root.join(&relative);
        let metadata = fs::metadata(&path)?;
        if metadata.len() == 0 || metadata.len() > MAX_ARTIFACT_BYTES {
            return Err(format!("bootstrap artifact {relative} is empty or oversized").into());
        }
        let bytes = fs::read(&path)?;
        if path.extension().and_then(|extension| extension.to_str()) == Some("json") {
            let value: serde_json::Value = serde_json::from_slice(&bytes)?;
            if value.get("run_mode").and_then(serde_json::Value::as_str)
                == Some("reference_evidence")
            {
                reference_report_count += 1;
                validate_reference_report(&relative, &value, &attestation.observed_runner)?;
                scenario_contracts.push(report_contract_identity(&relative, &value)?);
            }
        }
        evidence_files.push(BootstrapArtifactDigest {
            path: relative,
            sha256: digest_bytes(&bytes),
        });
    }
    if reference_report_count < MEASUREMENT_GATES.len() {
        return Err("bootstrap sample did not retain all reference report families".into());
    }
    scenario_contracts.sort();
    let scenario_contract_set_digest = digest_bytes(&serde_json::to_vec(&scenario_contracts)?);

    Ok(FullReferenceEvidence {
        source_commit: context.git_head.clone(),
        github_run_id: context.github_run_id.clone(),
        runner_fingerprint: attestation.observed_runner.fingerprint.clone(),
        observed_runner: attestation.observed_runner.clone(),
        runner_provisioning_sha256: sha256_file(&root.join(RUNNER_PROVISIONING_RELATIVE_PATH))?,
        prebuild_contract_digest: attestation
            .observed_runner
            .attestation
            .prebuild_contract_digest
            .clone(),
        scenario_contract_set_digest,
        evidence_files,
    })
}

#[derive(Debug)]
struct SampleAuthorization {
    sample_index: u32,
    admission_sha256: String,
    predecessor_github_run_id: Option<String>,
    predecessor_receipt_sha256: Option<String>,
}

fn authorize_sample(
    root: &Path,
    context: &QualificationContext,
    sample_index: Option<u32>,
    admission_path: &Path,
    predecessor_path: Option<&Path>,
    require_scenario_contract: bool,
) -> Result<SampleAuthorization, Box<dyn Error>> {
    use crate::perf_full_dress::{validate_admission_receipt, FullDressAdmissionReceipt};

    let sample_index = sample_index.ok_or("--sample-index is required")?;
    if !(1..=5).contains(&sample_index) {
        return Err("bootstrap sample index must be in 1..=5".into());
    }
    let admission_bytes = fs::read(admission_path)?;
    let admission: FullDressAdmissionReceipt = serde_json::from_slice(&admission_bytes)?;
    validate_admission_receipt(&admission)?;

    let attestation: MachineAttestationReceipt =
        read_json(&root.join(ATTESTATION_V5_RELATIVE_PATH))?;
    let bundle = verify_published_bundle(root)?;
    if admission.source_commit != context.git_head
        || admission.runner_fingerprint != attestation.observed_runner.fingerprint
        || admission.runner_provisioning_sha256
            != sha256_file(&root.join(RUNNER_PROVISIONING_RELATIVE_PATH))?
        || admission.prebuild_contract_digest
            != attestation
                .observed_runner
                .attestation
                .prebuild_contract_digest
        || bundle.manifest.source.git_commit != admission.source_commit
        || bundle.manifest.runner_fingerprint != admission.runner_fingerprint
    {
        return Err(
            "full-dress admission does not bind the current checkout, host, and prebuild".into(),
        );
    }

    if require_scenario_contract {
        let evidence = collect_full_reference_evidence(root, context)?;
        if admission.scenario_contract_set_digest != evidence.scenario_contract_set_digest {
            return Err("full-dress admission scenario contract differs from this sample".into());
        }
    }

    match (sample_index, predecessor_path) {
        (1, None) => Ok(SampleAuthorization {
            sample_index,
            admission_sha256: digest_bytes(&admission_bytes),
            predecessor_github_run_id: None,
            predecessor_receipt_sha256: None,
        }),
        (1, Some(_)) => Err("bootstrap sample 1 must not have a predecessor".into()),
        (_, None) => Err("bootstrap samples 2..=5 require the accepted predecessor receipt".into()),
        (_, Some(path)) => {
            let predecessor_bytes = fs::read(path)?;
            let predecessor: BootstrapSampleReceipt = serde_json::from_slice(&predecessor_bytes)?;
            validate_sample_receipt(&predecessor)?;
            let admission_sha256 = digest_bytes(&admission_bytes);
            if predecessor.sample_index + 1 != sample_index
                || predecessor.source_commit != context.git_head
                || predecessor.runner_fingerprint != admission.runner_fingerprint
                || predecessor.runner_provisioning_sha256 != admission.runner_provisioning_sha256
                || predecessor.prebuild_contract_digest != admission.prebuild_contract_digest
                || predecessor.scenario_contract_set_digest
                    != admission.scenario_contract_set_digest
                || predecessor.admission_sha256 != admission_sha256
                || !predecessor.passed
                || !predecessor.bootstrap_eligible
                || predecessor.ship_evidence_eligible
            {
                return Err(
                    "bootstrap predecessor is not the immediately prior accepted sample".into(),
                );
            }
            Ok(SampleAuthorization {
                sample_index,
                admission_sha256,
                predecessor_github_run_id: Some(predecessor.github_run_id),
                predecessor_receipt_sha256: Some(digest_bytes(&predecessor_bytes)),
            })
        }
    }
}

fn validate_reference_report(
    relative: &str,
    value: &serde_json::Value,
    runner: &RunnerFingerprint,
) -> Result<(), Box<dyn Error>> {
    if value.get("stable").and_then(serde_json::Value::as_bool) != Some(true)
        || value
            .pointer("/profile_validation/eligible")
            .and_then(serde_json::Value::as_bool)
            != Some(true)
        || value
            .pointer("/observed_runner/fingerprint")
            .and_then(serde_json::Value::as_str)
            != Some(runner.fingerprint.as_str())
        || value
            .get("stability_reasons")
            .and_then(serde_json::Value::as_array)
            .is_none_or(|reasons| !reasons.is_empty())
    {
        return Err(format!(
            "reference report {relative} is unstable, ineligible, or mixed-fingerprint"
        )
        .into());
    }
    Ok(())
}

fn report_contract_identity(
    relative: &str,
    value: &serde_json::Value,
) -> Result<String, Box<dyn Error>> {
    let mut identity = BTreeMap::new();
    identity.insert("path", relative.to_owned());
    for field in [
        "scenario_id",
        "scenario_digest",
        "workload_digest",
        "runner_contract_digest",
    ] {
        identity.insert(
            field,
            value
                .get(field)
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| format!("{relative} is missing {field}"))?
                .to_owned(),
        );
    }
    Ok(digest_bytes(&serde_json::to_vec(&identity)?))
}

pub fn build_sample_set(samples_dir: &Path) -> Result<BootstrapSampleSetReceipt, Box<dyn Error>> {
    let mut paths = fs::read_dir(samples_dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|extension| extension.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    paths.sort();
    if paths.len() != 5 {
        return Err("bootstrap sample set requires exactly five JSON receipts".into());
    }

    let mut samples = Vec::new();
    let mut run_ids = BTreeSet::new();
    let mut fingerprint = None;
    let mut runner_provisioning = None;
    let mut prebuild_contract = None;
    let mut scenario_contract = None;
    let mut source_commit = None;
    let mut admission_sha256 = None;
    let mut indexed = Vec::new();
    for path in paths {
        let bytes = fs::read(&path)?;
        let sample: BootstrapSampleReceipt = serde_json::from_slice(&bytes)?;
        validate_sample_receipt(&sample)?;
        if !sample.passed || !sample.bootstrap_eligible || sample.ship_evidence_eligible {
            return Err(format!("sample {} is not bootstrap-only eligible", path.display()).into());
        }
        require_same(
            &mut fingerprint,
            &sample.runner_fingerprint,
            "runner fingerprint",
        )?;
        require_same(
            &mut runner_provisioning,
            &sample.runner_provisioning_sha256,
            "runner provisioning receipt",
        )?;
        require_same(&mut source_commit, &sample.source_commit, "source commit")?;
        require_same(
            &mut admission_sha256,
            &sample.admission_sha256,
            "full-dress admission",
        )?;
        require_same(
            &mut prebuild_contract,
            &sample.prebuild_contract_digest,
            "prebuild contract",
        )?;
        require_same(
            &mut scenario_contract,
            &sample.scenario_contract_set_digest,
            "scenario contract set",
        )?;
        if !run_ids.insert(sample.github_run_id.clone()) {
            return Err("bootstrap sample set contains a duplicate GitHub run id".into());
        }
        indexed.push((sample, digest_bytes(&bytes)));
    }
    indexed.sort_by_key(|(sample, _)| sample.sample_index);
    for (offset, (sample, receipt_sha256)) in indexed.iter().enumerate() {
        let expected_index = offset as u32 + 1;
        if sample.sample_index != expected_index {
            return Err("bootstrap samples must form the exact index sequence 1..=5".into());
        }
        if expected_index == 1 {
            if sample.predecessor_github_run_id.is_some()
                || sample.predecessor_receipt_sha256.is_some()
            {
                return Err("bootstrap sample 1 must be the chain root".into());
            }
        } else {
            let (previous, previous_digest) = &indexed[offset - 1];
            if sample.predecessor_github_run_id.as_deref() != Some(previous.github_run_id.as_str())
                || sample.predecessor_receipt_sha256.as_deref() != Some(previous_digest.as_str())
            {
                return Err("bootstrap sample chain is broken or reordered".into());
            }
        }
        samples.push(BootstrapSampleMember {
            sample_index: sample.sample_index,
            github_run_id: sample.github_run_id.clone(),
            source_commit: sample.source_commit.clone(),
            runner_fingerprint: sample.runner_fingerprint.clone(),
            receipt_sha256: receipt_sha256.clone(),
        });
    }
    Ok(BootstrapSampleSetReceipt {
        schema_version: 2,
        release: RELEASE.to_owned(),
        profile: PROFILE.to_owned(),
        source_commit: source_commit.expect("five samples establish source commit"),
        runner_fingerprint: fingerprint.expect("five samples establish fingerprint"),
        runner_provisioning_sha256: runner_provisioning
            .expect("five samples establish runner provisioning receipt"),
        prebuild_contract_digest: prebuild_contract.expect("five samples establish prebuild"),
        scenario_contract_set_digest: scenario_contract
            .expect("five samples establish scenario contract"),
        full_dress_admission_sha256: admission_sha256
            .expect("five samples establish full-dress admission"),
        samples,
        passed: true,
        bootstrap_eligible: true,
        ship_evidence_eligible: false,
    })
}

fn validate_sample_receipt(sample: &BootstrapSampleReceipt) -> Result<(), Box<dyn Error>> {
    let attestation_problems = reference_attestation_problems(&sample.observed_runner.attestation);
    let evidence_paths = sample
        .evidence_files
        .iter()
        .map(|artifact| artifact.path.as_str())
        .collect::<BTreeSet<_>>();
    if sample.schema_version != 2
        || sample.release != RELEASE
        || sample.profile != PROFILE
        || !is_git_commit(&sample.source_commit)
        || sample.github_run_id.parse::<u64>().is_err()
        || !is_sha256(&sample.runner_fingerprint)
        || sample.observed_runner.fingerprint != sample.runner_fingerprint
        || sample.observed_runner.runner_class != REFERENCE_RUNNER_CLASS
        || sample.observed_runner.shared_hardware
        || !attestation_problems.is_empty()
        || !is_sha256(&sample.prebuild_contract_digest)
        || !is_sha256(&sample.runner_provisioning_sha256)
        || sample.observed_runner.attestation.prebuild_contract_digest
            != sample.prebuild_contract_digest
        || !is_sha256(&sample.scenario_contract_set_digest)
        || !(1..=5).contains(&sample.sample_index)
        || !is_sha256(&sample.admission_sha256)
        || sample
            .predecessor_github_run_id
            .as_deref()
            .is_some_and(|run_id| run_id.parse::<u64>().is_err())
        || sample
            .predecessor_receipt_sha256
            .as_deref()
            .is_some_and(|digest| !is_sha256(digest))
        || sample.predecessor_github_run_id.is_some() != sample.predecessor_receipt_sha256.is_some()
        || sample.evidence_files.is_empty()
        || evidence_paths.len() != sample.evidence_files.len()
        || sample
            .evidence_files
            .iter()
            .any(|artifact| artifact.path.is_empty() || !is_sha256(&artifact.sha256))
    {
        return Err(format!(
            "bootstrap sample receipt is malformed or ineligible: {attestation_problems:?}"
        )
        .into());
    }
    Ok(())
}

fn is_git_commit(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}
fn require_same(
    expected: &mut Option<String>,
    observed: &str,
    label: &str,
) -> Result<(), Box<dyn Error>> {
    match expected {
        Some(expected) if expected != observed => {
            Err(format!("bootstrap samples mix {label}").into())
        }
        Some(_) => Ok(()),
        None => {
            *expected = Some(observed.to_owned());
            Ok(())
        }
    }
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, Box<dyn Error>> {
    let metadata = fs::metadata(path)?;
    if metadata.len() == 0 || metadata.len() > MAX_ARTIFACT_BYTES {
        return Err(format!("{} is empty or oversized", path.display()).into());
    }
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn write_create_new_json(path: &Path, value: &impl Serialize) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut output = OpenOptions::new().write(true).create_new(true).open(path)?;
    output.write_all(&serde_json::to_vec_pretty(value)?)?;
    output.write_all(b"\n")?;
    output.sync_all()?;
    Ok(())
}

fn digest_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[derive(Debug)]
struct Options {
    phase: String,
    samples_dir: Option<PathBuf>,
    sample_index: Option<u32>,
    admission: Option<PathBuf>,
    predecessor: Option<PathBuf>,
}

impl Options {
    fn parse(args: Vec<String>) -> Result<Self, Box<dyn Error>> {
        let mut release = None;
        let mut profile = None;
        let mut phase = None;
        let mut samples_dir = None;
        let mut sample_index = None;
        let mut admission = None;
        let mut predecessor = None;
        let mut args = args.into_iter();
        while let Some(argument) = args.next() {
            match argument.as_str() {
                "--release" => release = args.next(),
                "--profile" => profile = args.next(),
                "--phase" => phase = args.next(),
                "--samples-dir" => samples_dir = args.next().map(PathBuf::from),
                "--sample-index" => {
                    sample_index = Some(
                        args.next()
                            .ok_or("--sample-index requires a value")?
                            .parse::<u32>()?,
                    )
                }
                "--admission" => admission = args.next().map(PathBuf::from),
                "--predecessor" => predecessor = args.next().map(PathBuf::from),
                other => return Err(format!("unknown perf-bootstrap argument: {other}").into()),
            }
        }
        if release.as_deref() != Some(RELEASE)
            || profile.as_deref() != Some(PROFILE)
            || !matches!(
                phase.as_deref(),
                Some("context" | "authorize" | "sample" | "sample-set")
            )
        {
            return Err(
                "usage: perf-bootstrap --release 0.67.1 --profile reference-v1 --phase <context|authorize|sample|sample-set> [--sample-index 1..5 --admission PATH --predecessor PATH] [--samples-dir PATH]"
                    .into(),
            );
        }
        Ok(Self {
            phase: phase.expect("phase was checked"),
            samples_dir,
            sample_index,
            admission,
            predecessor,
        })
    }
}
