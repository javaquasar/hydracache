//! Full-workload qualification and two-run admission for 0.67.1 bootstrap.
//!
//! A full-dress run executes the same measurement families as a bootstrap
//! sample, but its receipt is deliberately neither bootstrap nor ship
//! evidence. Two independently dispatched, identical full-dress receipts are
//! required to create the admission consumed by the serialized sample chain.

use std::collections::BTreeSet;
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use hydracache_loadgen::profile::{
    reference_attestation_problems, RunnerFingerprint, REFERENCE_RUNNER_CLASS,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::perf_bootstrap::{
    collect_full_reference_evidence, BootstrapArtifactDigest, FullReferenceEvidence,
};
use crate::perf_qualification::{
    observe_context, trusted_performance_context_problems, QualificationContext,
};

pub const FULL_DRESS_RECEIPT_RELATIVE_PATH: &str =
    "target/test-evidence/0.67.1/full-dress-receipt.json";
pub const FULL_DRESS_ADMISSION_RELATIVE_PATH: &str =
    "target/test-evidence/0.67.1/full-dress-admission.json";

const RELEASE: &str = "0.67.1";
const PROFILE: &str = "reference-v1";
const MAX_JSON_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FullDressReceipt {
    pub schema_version: u32,
    pub release: String,
    pub profile: String,
    pub mode: String,
    pub source_commit: String,
    pub github_run_id: String,
    pub runner_fingerprint: String,
    pub observed_runner: RunnerFingerprint,
    pub runner_provisioning_sha256: String,
    pub prebuild_contract_digest: String,
    pub scenario_contract_set_digest: String,
    pub evidence_files: Vec<BootstrapArtifactDigest>,
    pub passed: bool,
    pub qualification_only: bool,
    pub bootstrap_eligible: bool,
    pub ship_evidence_eligible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FullDressAdmissionMember {
    pub github_run_id: String,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FullDressAdmissionReceipt {
    pub schema_version: u32,
    pub release: String,
    pub profile: String,
    pub source_commit: String,
    pub runner_fingerprint: String,
    pub runner_provisioning_sha256: String,
    pub prebuild_contract_digest: String,
    pub scenario_contract_set_digest: String,
    pub full_dress_runs: Vec<FullDressAdmissionMember>,
    pub passed: bool,
    pub bootstrap_admission_eligible: bool,
    pub bootstrap_eligible: bool,
    pub ship_evidence_eligible: bool,
}

pub fn full_dress_context_problems(context: &QualificationContext) -> Vec<String> {
    trusted_performance_context_problems(context, "full-dress")
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let options = Options::parse(args)?;
    let root = crate::doc_check::find_repo_root()?;
    let context = observe_context(&root)?;
    let problems = full_dress_context_problems(&context);
    if !problems.is_empty() {
        return Err(format!("full-dress context rejected: {problems:?}").into());
    }

    match options.phase.as_str() {
        "context" => println!("0.67.1 full-dress context: OK"),
        "receipt" => {
            let evidence = collect_full_reference_evidence(&root, &context)?;
            let receipt = receipt_from_evidence(evidence);
            let output = root.join(FULL_DRESS_RECEIPT_RELATIVE_PATH);
            write_create_new_json(&output, &receipt)?;
            println!(
                "0.67.1 non-promotable full-dress receipt retained: {}",
                output.display()
            );
        }
        "admission" => {
            let predecessor = options
                .predecessor_receipt
                .as_deref()
                .ok_or("--predecessor-receipt is required for admission phase")?;
            let current = root.join(FULL_DRESS_RECEIPT_RELATIVE_PATH);
            let admission = build_admission(&[predecessor.to_path_buf(), current])?;
            let output = root.join(FULL_DRESS_ADMISSION_RELATIVE_PATH);
            write_create_new_json(&output, &admission)?;
            println!("0.67.1 bootstrap admission retained: {}", output.display());
        }
        _ => unreachable!("phase parser is exhaustive"),
    }
    Ok(())
}

pub fn receipt_from_evidence(evidence: FullReferenceEvidence) -> FullDressReceipt {
    FullDressReceipt {
        schema_version: 1,
        release: RELEASE.to_owned(),
        profile: PROFILE.to_owned(),
        mode: "full-dress-qualification-only".to_owned(),
        source_commit: evidence.source_commit,
        github_run_id: evidence.github_run_id,
        runner_fingerprint: evidence.runner_fingerprint,
        observed_runner: evidence.observed_runner,
        runner_provisioning_sha256: evidence.runner_provisioning_sha256,
        prebuild_contract_digest: evidence.prebuild_contract_digest,
        scenario_contract_set_digest: evidence.scenario_contract_set_digest,
        evidence_files: evidence.evidence_files,
        passed: true,
        qualification_only: true,
        bootstrap_eligible: false,
        ship_evidence_eligible: false,
    }
}

pub fn build_admission(paths: &[PathBuf]) -> Result<FullDressAdmissionReceipt, Box<dyn Error>> {
    if paths.len() != 2 {
        return Err("full-dress admission requires exactly two receipt paths".into());
    }

    let mut receipts = Vec::new();
    let mut run_ids = BTreeSet::new();
    for path in paths {
        let bytes = read_bounded(path)?;
        let receipt: FullDressReceipt = serde_json::from_slice(&bytes)?;
        validate_full_dress_receipt(&receipt)?;
        if !run_ids.insert(receipt.github_run_id.clone()) {
            return Err("full-dress admission requires two distinct GitHub runs".into());
        }
        receipts.push((receipt, digest_bytes(&bytes)));
    }

    let first = &receipts[0].0;
    let second = &receipts[1].0;
    if first.source_commit != second.source_commit
        || first.runner_fingerprint != second.runner_fingerprint
        || first.runner_provisioning_sha256 != second.runner_provisioning_sha256
        || first.prebuild_contract_digest != second.prebuild_contract_digest
        || first.scenario_contract_set_digest != second.scenario_contract_set_digest
    {
        return Err(
            "full-dress receipts mix source, runner provisioning, prebuild, or scenario contracts"
                .into(),
        );
    }
    let source_commit = first.source_commit.clone();
    let runner_fingerprint = first.runner_fingerprint.clone();
    let runner_provisioning_sha256 = first.runner_provisioning_sha256.clone();
    let prebuild_contract_digest = first.prebuild_contract_digest.clone();
    let scenario_contract_set_digest = first.scenario_contract_set_digest.clone();

    let mut members = receipts
        .into_iter()
        .map(|(receipt, receipt_sha256)| FullDressAdmissionMember {
            github_run_id: receipt.github_run_id,
            receipt_sha256,
        })
        .collect::<Vec<_>>();
    members.sort_by(|left, right| left.github_run_id.cmp(&right.github_run_id));

    Ok(FullDressAdmissionReceipt {
        schema_version: 1,
        release: RELEASE.to_owned(),
        profile: PROFILE.to_owned(),
        source_commit,
        runner_fingerprint,
        runner_provisioning_sha256,
        prebuild_contract_digest,
        scenario_contract_set_digest,
        full_dress_runs: members,
        passed: true,
        bootstrap_admission_eligible: true,
        bootstrap_eligible: false,
        ship_evidence_eligible: false,
    })
}

pub fn validate_full_dress_receipt(receipt: &FullDressReceipt) -> Result<(), Box<dyn Error>> {
    let attestation_problems = reference_attestation_problems(&receipt.observed_runner.attestation);
    let evidence_paths = receipt
        .evidence_files
        .iter()
        .map(|artifact| artifact.path.as_str())
        .collect::<BTreeSet<_>>();
    if receipt.schema_version != 1
        || receipt.release != RELEASE
        || receipt.profile != PROFILE
        || receipt.mode != "full-dress-qualification-only"
        || !is_git_commit(&receipt.source_commit)
        || receipt.github_run_id.parse::<u64>().is_err()
        || !is_sha256(&receipt.runner_fingerprint)
        || receipt.observed_runner.fingerprint != receipt.runner_fingerprint
        || receipt.observed_runner.runner_class != REFERENCE_RUNNER_CLASS
        || receipt.observed_runner.shared_hardware
        || !attestation_problems.is_empty()
        || !is_sha256(&receipt.prebuild_contract_digest)
        || !is_sha256(&receipt.runner_provisioning_sha256)
        || receipt.observed_runner.attestation.prebuild_contract_digest
            != receipt.prebuild_contract_digest
        || !is_sha256(&receipt.scenario_contract_set_digest)
        || receipt.evidence_files.is_empty()
        || evidence_paths.len() != receipt.evidence_files.len()
        || receipt
            .evidence_files
            .iter()
            .any(|artifact| artifact.path.is_empty() || !is_sha256(&artifact.sha256))
        || !receipt.passed
        || !receipt.qualification_only
        || receipt.bootstrap_eligible
        || receipt.ship_evidence_eligible
    {
        return Err(format!(
            "full-dress receipt is malformed or promotable: {attestation_problems:?}"
        )
        .into());
    }
    Ok(())
}

pub fn validate_admission_receipt(
    admission: &FullDressAdmissionReceipt,
) -> Result<(), Box<dyn Error>> {
    let run_ids = admission
        .full_dress_runs
        .iter()
        .map(|member| member.github_run_id.as_str())
        .collect::<BTreeSet<_>>();
    if admission.schema_version != 1
        || admission.release != RELEASE
        || admission.profile != PROFILE
        || !is_git_commit(&admission.source_commit)
        || !is_sha256(&admission.runner_fingerprint)
        || !is_sha256(&admission.prebuild_contract_digest)
        || !is_sha256(&admission.runner_provisioning_sha256)
        || !is_sha256(&admission.scenario_contract_set_digest)
        || admission.full_dress_runs.len() != 2
        || run_ids.len() != 2
        || admission.full_dress_runs.iter().any(|member| {
            member.github_run_id.parse::<u64>().is_err() || !is_sha256(&member.receipt_sha256)
        })
        || !admission.passed
        || !admission.bootstrap_admission_eligible
        || admission.bootstrap_eligible
        || admission.ship_evidence_eligible
    {
        return Err("full-dress admission is malformed or ineligible".into());
    }
    Ok(())
}

fn read_bounded(path: &Path) -> Result<Vec<u8>, Box<dyn Error>> {
    let metadata = fs::metadata(path)?;
    if metadata.len() == 0 || metadata.len() > MAX_JSON_BYTES {
        return Err(format!("{} is empty or oversized", path.display()).into());
    }
    Ok(fs::read(path)?)
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

#[derive(Debug)]
struct Options {
    phase: String,
    predecessor_receipt: Option<PathBuf>,
}

impl Options {
    fn parse(args: Vec<String>) -> Result<Self, Box<dyn Error>> {
        let mut release = None;
        let mut profile = None;
        let mut phase = None;
        let mut predecessor_receipt = None;
        let mut args = args.into_iter();
        while let Some(argument) = args.next() {
            match argument.as_str() {
                "--release" => release = args.next(),
                "--profile" => profile = args.next(),
                "--phase" => phase = args.next(),
                "--predecessor-receipt" => predecessor_receipt = args.next().map(PathBuf::from),
                other => return Err(format!("unknown perf-full-dress argument: {other}").into()),
            }
        }
        if release.as_deref() != Some(RELEASE)
            || profile.as_deref() != Some(PROFILE)
            || !matches!(phase.as_deref(), Some("context" | "receipt" | "admission"))
        {
            return Err(
                "usage: perf-full-dress --release 0.67.1 --profile reference-v1 --phase <context|receipt|admission> [--predecessor-receipt PATH]"
                    .into(),
            );
        }
        Ok(Self {
            phase: phase.expect("phase was checked"),
            predecessor_receipt,
        })
    }
}
