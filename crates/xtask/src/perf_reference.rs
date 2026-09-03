//! Deterministic 0.67.1 reference-contract proposal, review, activation, and
//! frozen-candidate gates.
//!
//! The commands in this module deliberately separate four authorities:
//! measurement produces five immutable samples, automation proposes contracts,
//! a distinct reviewer approves or rejects the exact proposal bytes, and the
//! release candidate consumes the already-reviewed contract.  No phase widens
//! a workload SLO, drops a sample, or derives a baseline from the candidate.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::canary_sweep::{self, CanaryOutcome, CanaryReceipt};
use crate::perf_bootstrap::{
    collect_full_reference_evidence, validate_sample_receipt, BootstrapSampleReceipt,
    BootstrapSampleSetReceipt,
};
use crate::perf_budget::{
    self, AnchorMetric, BaselineChangeApproval, BaselineChangeControl, BaselineChangeProposal,
    BaselineMember, BaselineReportReceipt, BootstrapStatus, BudgetContract, BudgetRuleStatus,
    CandidateReport, ChangeControlStatus, ContractBundle, MemberMetric, ProfileContract,
    ReleaseAnchor, RollingBaselineManifest, CLEAN_GIT_STATUS_SHA256,
};
use crate::perf_qualification::{observe_context, trusted_performance_context_problems};

pub const RELEASE: &str = "0.67.1";
pub const PROFILE: &str = "reference-v1";
pub const PROPOSAL_DIR: &str = "target/test-evidence/0.67.1/reference-proposal";
pub const REVIEWED_DIR: &str = "target/test-evidence/0.67.1/reference-reviewed";
pub const REVIEW_DECISION_PATH: &str = "target/test-evidence/0.67.1/reference-review-decision.json";
pub const REVIEW_RECEIPT_PATH: &str = "target/test-evidence/0.67.1/baseline-review.json";
pub const ACTIVATION_RECEIPT_PATH: &str = "target/test-evidence/0.67.1/reference-activation.json";
pub const FROZEN_RECEIPT_PATH: &str = "target/test-evidence/0.67.1/frozen-candidate.json";
pub const COMMITTED_REVIEW_PATH: &str = "docs/testing/perf-reviews/0.67.1/reference-v1.json";
pub const COMMITTED_ANCHOR_PATH: &str = "docs/testing/perf-anchors/0.67.1/reference-v1.json";
pub const COMMITTED_BUDGET_PATH: &str = "docs/testing/perf-budgets/0.67.1/reference-v1.toml";
pub const COMMITTED_BASELINE_PATH: &str = "docs/testing/perf-baselines/0.67.1/reference-v1.toml";
pub const COMMITTED_PROFILE_PATH: &str = "docs/testing/perf-profiles/reference-v1.toml";
const TEMPLATE_BUDGET_PATH: &str = "docs/testing/perf-budgets/0.67/reference-v1.toml";
const TEMPLATE_BASELINE_PATH: &str = "docs/testing/perf-baselines/0.67/reference-v1.toml";
const SAMPLE_SET_PATH: &str = "target/test-evidence/0.67.1/bootstrap-sample-set.json";
const MAX_INPUT_BYTES: u64 = 64 * 1024 * 1024;
const ANCHOR_TOLERANCE: f64 = 0.10;
const ROLLING_TOLERANCE: f64 = 0.10;
const REPORT_SPREAD_CEILING: f64 = 0.05;

#[derive(Debug, Clone)]
pub struct ReferenceSampleInput {
    pub receipt: BootstrapSampleReceipt,
    pub receipt_sha256: String,
    pub reports: Vec<CandidateReport>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnchorDocument {
    pub schema_version: u32,
    pub release: String,
    pub profile: String,
    pub sample_set_sha256: String,
    pub anchor: ReleaseAnchor,
    pub anchor_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileDigest {
    pub id: String,
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposalReceipt {
    pub schema_version: u32,
    pub release: String,
    pub profile: String,
    pub producer: String,
    pub proposed_at: String,
    pub rationale: String,
    pub sample_set_sha256: String,
    pub sample_receipt_sha256: Vec<String>,
    pub runner_provisioning_sha256: String,
    pub full_dress_admission_sha256: String,
    pub previous_baseline_sha256: String,
    pub proposed_payload_sha256: String,
    pub files: Vec<FileDigest>,
    pub receipt_sha256: String,
}

impl ProposalReceipt {
    pub fn seal(&mut self) {
        self.receipt_sha256.clear();
        self.receipt_sha256 = perf_budget::digest_json(self);
    }

    pub fn is_valid(&self) -> bool {
        let mut payload = self.clone();
        payload.receipt_sha256.clear();
        self.receipt_sha256 == perf_budget::digest_json(&payload)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewDecisionKind {
    Approve,
    Reject,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewDecision {
    pub schema_version: u32,
    pub release: String,
    pub profile: String,
    pub proposal_file_sha256: String,
    pub decision: ReviewDecisionKind,
    pub reviewer: String,
    pub reviewed_at: String,
    pub review_reference: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewReceipt {
    pub schema_version: u32,
    pub release: String,
    pub profile: String,
    pub proposal_receipt_sha256: String,
    pub sample_set_sha256: String,
    pub sample_receipt_sha256: Vec<String>,
    pub runner_provisioning_sha256: String,
    pub full_dress_admission_sha256: String,
    pub producer: String,
    pub reviewer: String,
    pub reviewed_at: String,
    pub review_reference: String,
    pub decision: ReviewDecisionKind,
    pub reason: String,
    pub reviewed_files: Vec<FileDigest>,
    pub passed: bool,
    pub ship_evidence_eligible: bool,
    pub receipt_sha256: String,
}

impl ReviewReceipt {
    pub fn seal(&mut self) {
        self.receipt_sha256.clear();
        self.receipt_sha256 = perf_budget::digest_json(self);
    }

    pub fn is_valid(&self) -> bool {
        let mut payload = self.clone();
        payload.receipt_sha256.clear();
        self.receipt_sha256 == perf_budget::digest_json(&payload)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActivationReceipt {
    pub schema_version: u32,
    pub release: String,
    pub profile: String,
    pub source_commit: String,
    pub runner_fingerprint: String,
    pub review_receipt_sha256: String,
    pub files: Vec<FileDigest>,
    pub td_0013_resolved: bool,
    pub release_notes_present: bool,
    pub passed: bool,
    pub ship_evidence_eligible: bool,
    pub receipt_sha256: String,
}

impl ActivationReceipt {
    pub fn seal(&mut self) {
        self.receipt_sha256.clear();
        self.receipt_sha256 = perf_budget::digest_json(self);
    }

    pub fn is_valid(&self) -> bool {
        let mut payload = self.clone();
        payload.receipt_sha256.clear();
        self.receipt_sha256 == perf_budget::digest_json(&payload)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrozenCandidateReceipt {
    pub schema_version: u32,
    pub release: String,
    pub profile: String,
    pub source_commit: String,
    pub github_run_id: String,
    pub runner_fingerprint: String,
    pub activation_sha256: String,
    pub budget_verdict_sha256: String,
    pub reference_evidence_sha256: Vec<FileDigest>,
    pub canary_receipt_sha256: Vec<FileDigest>,
    pub passed: bool,
    pub ship_evidence_eligible: bool,
    pub receipt_sha256: String,
}

impl FrozenCandidateReceipt {
    pub fn seal(&mut self) {
        self.receipt_sha256.clear();
        self.receipt_sha256 = perf_budget::digest_json(self);
    }
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let options = Options::parse(args)?;
    match options.phase.as_str() {
        "propose" => propose(&options),
        "review" => review(&options),
        "reviewed" => validate_committed_review(&options),
        "activate" => activate(&options),
        "frozen-candidate" => frozen_candidate(&options),
        other => Err(format!("unsupported perf-reference phase {other:?}").into()),
    }
}

fn validate_committed_review(options: &Options) -> Result<(), Box<dyn Error>> {
    let root = &options.root;
    let review_path = root.join(COMMITTED_REVIEW_PATH);
    let review_bytes = read_bounded(&review_path)?;
    let review: ReviewReceipt = serde_json::from_slice(&review_bytes)?;
    let review_problems = review_provenance_problems(&review);
    if !review_problems.is_empty() {
        return Err(format!(
            "committed independent review receipt is invalid: {review_problems:?}"
        )
        .into());
    }
    verify_committed_files(root, &review.reviewed_files, &committed_files())?;
    let bundle = perf_budget::load_bundle(root, RELEASE, PROFILE)?;
    let problems = perf_budget::validate_contract_bundle(&bundle);
    if !problems.is_empty() {
        return Err(format!("committed reviewed contract is invalid: {problems:?}").into());
    }
    write_new_bytes(&root.join(REVIEW_RECEIPT_PATH), &review_bytes)?;
    println!("perf-reference reviewed: OK");
    Ok(())
}

#[derive(Clone, Copy, Debug)]
pub struct ProposalMetadata<'a> {
    pub sample_set_sha256: &'a str,
    pub producer: &'a str,
    pub proposed_at: &'a str,
    pub rationale: &'a str,
    pub previous_baseline_sha256: &'a str,
}

pub fn derive_contracts(
    mut profile: ProfileContract,
    mut budget: BudgetContract,
    sample_set: &BootstrapSampleSetReceipt,
    samples: &[ReferenceSampleInput],
    metadata: ProposalMetadata<'_>,
) -> Result<
    (
        ProfileContract,
        BudgetContract,
        RollingBaselineManifest,
        AnchorDocument,
    ),
    Box<dyn Error>,
> {
    validate_proposal_metadata(
        metadata.producer,
        metadata.proposed_at,
        metadata.rationale,
        metadata.previous_baseline_sha256,
    )?;
    validate_sample_inputs(sample_set, samples)?;
    if sample_set.prebuild_contract_digest != profile.prebuild.digest {
        return Err("bootstrap sample set changed the frozen prebuild contract".into());
    }
    profile.bootstrap_status = BootstrapStatus::Bootstrapped;
    profile.runner.allowed_fingerprints = vec![sample_set.runner_fingerprint.clone()];
    budget.bootstrap_status = BootstrapStatus::Bootstrapped;
    for rule in &mut budget.budgets {
        rule.status = BudgetRuleStatus::Active;
        rule.anchor_tolerance_ratio = Some(ANCHOR_TOLERANCE);
        rule.rolling_tolerance_ratio = Some(ROLLING_TOLERANCE);
        rule.maximum_spread_ratio = Some(REPORT_SPREAD_CEILING);
    }
    let profile_sha256 = perf_budget::digest_json(&profile);
    let budget_sha256 = perf_budget::digest_json(&budget);
    let mut members = samples
        .iter()
        .map(|sample| {
            member_from_sample(&profile, &budget, &profile_sha256, &budget_sha256, sample)
        })
        .collect::<Result<Vec<_>, _>>()?;
    members.sort_by_key(|member| {
        sample_set
            .samples
            .iter()
            .position(|sample| sample.github_run_id == member.run_id)
            .unwrap_or(usize::MAX)
    });
    let rolling_metrics = perf_budget::rolling_summaries(&budget.budgets, &members)?;
    let anchor_metrics = rolling_metrics
        .iter()
        .map(|metric| AnchorMetric {
            budget_id: metric.budget_id.clone(),
            value: metric.median,
            unit: metric.unit.clone(),
        })
        .collect::<Vec<_>>();
    let frozen_at = samples
        .iter()
        .map(|sample| sample.receipt.observed_at.as_str())
        .max()
        .ok_or("reference proposal has no sample timestamp")?
        .to_owned();
    let anchor = ReleaseAnchor {
        status: BootstrapStatus::Bootstrapped,
        frozen_at,
        contract_commit: sample_set.source_commit.clone(),
        source_run_ids: members.iter().map(|member| member.run_id.clone()).collect(),
        source_members: members.clone(),
        metrics: anchor_metrics,
    };
    let mut baseline = RollingBaselineManifest {
        schema_version: 1,
        release: perf_budget::RELEASE.to_owned(),
        profile: PROFILE.to_owned(),
        bootstrap_status: BootstrapStatus::Bootstrapped,
        profile_sha256,
        budget_sha256,
        selection_reason: "most-recent-eligible-successful-main-medians".to_owned(),
        policy: perf_budget::RollingPolicy {
            branch: "main".to_owned(),
            minimum_members: 5,
            maximum_members: 10,
            maximum_age_days: 30,
        },
        anchor: anchor.clone(),
        candidate_members: members.clone(),
        members,
        rolling_metrics,
        change_control: BaselineChangeControl {
            status: ChangeControlStatus::PendingBootstrap,
            proposal: None,
            approval: None,
        },
        receipt_sha256: String::new(),
    };
    let payload_sha256 = perf_budget::baseline_payload_digest(&baseline);
    baseline.change_control.proposal = Some(BaselineChangeProposal {
        proposal_id: format!("reference-v1-bootstrap-{}", &payload_sha256[..16]),
        proposed_at: metadata.proposed_at.to_owned(),
        proposer: metadata.producer.to_owned(),
        rationale: metadata.rationale.to_owned(),
        previous_manifest_sha256: metadata.previous_baseline_sha256.to_owned(),
        proposed_payload_sha256: payload_sha256,
    });
    perf_budget::seal_baseline_manifest(&mut baseline);
    let mut anchor_document = AnchorDocument {
        schema_version: 1,
        release: RELEASE.to_owned(),
        profile: PROFILE.to_owned(),
        sample_set_sha256: metadata.sample_set_sha256.to_owned(),
        anchor,
        anchor_sha256: String::new(),
    };
    anchor_document.anchor_sha256 = perf_budget::digest_json(&anchor_document.anchor);
    Ok((profile, budget, baseline, anchor_document))
}

fn member_from_sample(
    profile: &ProfileContract,
    budget: &BudgetContract,
    profile_sha256: &str,
    budget_sha256: &str,
    sample: &ReferenceSampleInput,
) -> Result<BaselineMember, Box<dyn Error>> {
    let report_by_id = sample
        .reports
        .iter()
        .map(|report| (report.id.as_str(), report))
        .collect::<BTreeMap<_, _>>();
    if report_by_id.len() != budget.reports.len()
        || budget
            .reports
            .iter()
            .any(|expected| !report_by_id.contains_key(expected.id.as_str()))
    {
        return Err(format!(
            "sample {} has a partial report set",
            sample.receipt.sample_index
        )
        .into());
    }
    let first = sample
        .reports
        .first()
        .ok_or("sample has no normalized reports")?;
    let invalid_reports = sample
        .reports
        .iter()
        .filter_map(|report| {
            let identity_matches = report.source_commit == sample.receipt.source_commit
                && report.runner_fingerprint == sample.receipt.runner_fingerprint
                && report.prebuild_contract_digest == sample.receipt.prebuild_contract_digest;
            (!identity_matches
                || !report.stable
                || report.maximum_spread_ratio > REPORT_SPREAD_CEILING)
                .then(|| {
                    format!(
                        "{}(identity_matches={},stable={},spread={})",
                        report.id, identity_matches, report.stable, report.maximum_spread_ratio,
                    )
                })
        })
        .collect::<Vec<_>>();
    if !invalid_reports.is_empty() {
        return Err(format!(
            "sample {} has invalid reports [{}] (spread ceiling={})",
            sample.receipt.sample_index,
            invalid_reports.join(", "),
            REPORT_SPREAD_CEILING,
        )
        .into());
    }
    let reports = sample
        .reports
        .iter()
        .map(|report| {
            let metrics = budget
                .budgets
                .iter()
                .filter(|rule| rule.report == report.id)
                .map(|rule| {
                    let metric = report
                        .metrics
                        .get(&rule.metric)
                        .ok_or_else(|| format!("report {} misses {}", report.id, rule.metric))?;
                    Ok(metric.clone())
                })
                .collect::<Result<Vec<_>, String>>()?;
            let mut receipt = BaselineReportReceipt {
                report_id: report.id.clone(),
                report_sha256: report.report_sha256.clone(),
                scenario_digest: report.scenario_digest.clone(),
                workload_digest: report.workload_digest.clone(),
                slo_digest: report.slo_digest.clone(),
                methodology_digest: report.methodology_digest.clone(),
                cargo_lock_sha256: report.cargo_lock_sha256.clone(),
                prebuild_manifest_sha256: report.prebuild_manifest_sha256.clone(),
                binary_sha256: report.binary_sha256.clone(),
                binary_set_digest: report.binary_set_digest.clone(),
                stable: report.stable,
                maximum_spread_ratio: report.maximum_spread_ratio,
                metrics,
                receipt_sha256: String::new(),
            };
            perf_budget::seal_baseline_report(&mut receipt);
            Ok(receipt)
        })
        .collect::<Result<Vec<_>, String>>()?;
    let metrics = budget
        .budgets
        .iter()
        .map(|rule| {
            let report = report_by_id
                .get(rule.report.as_str())
                .ok_or_else(|| format!("missing report {}", rule.report))?;
            let metric = report
                .metrics
                .get(&rule.metric)
                .ok_or_else(|| format!("missing metric {}", rule.metric))?;
            Ok(MemberMetric {
                budget_id: rule.id.clone(),
                value: metric.value,
                unit: metric.unit.clone(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let mut member = BaselineMember {
        run_id: sample.receipt.github_run_id.clone(),
        branch: "main".to_owned(),
        source_commit: sample.receipt.source_commit.clone(),
        observed_at: sample.receipt.observed_at.clone(),
        successful: true,
        quarantined: false,
        calibration_passed: true,
        spread_stable: true,
        gate_exit_code: 0,
        git_status_porcelain_sha256: CLEAN_GIT_STATUS_SHA256.to_owned(),
        quarantine_reason: None,
        runner_contract: profile.runner.clone(),
        runner_contract_digest: perf_budget::digest_json(&profile.runner),
        observed_runner: sample.receipt.observed_runner.clone(),
        runner_fingerprint: sample.receipt.runner_fingerprint.clone(),
        toolchain_identity: first.toolchain_identity.clone(),
        prebuild_contract_digest: sample.receipt.prebuild_contract_digest.clone(),
        profile_sha256: profile_sha256.to_owned(),
        budget_sha256: budget_sha256.to_owned(),
        reports,
        metrics,
        receipt_sha256: String::new(),
    };
    perf_budget::seal_baseline_member(&mut member);
    Ok(member)
}

fn validate_proposal_metadata(
    producer: &str,
    proposed_at: &str,
    rationale: &str,
    previous_baseline_sha256: &str,
) -> Result<(), Box<dyn Error>> {
    if producer.trim().is_empty()
        || rationale.trim().is_empty()
        || OffsetDateTime::parse(proposed_at, &Rfc3339).is_err()
        || !is_sha256(previous_baseline_sha256)
    {
        return Err("proposal metadata is incomplete".into());
    }
    Ok(())
}

fn validate_sample_inputs(
    sample_set: &BootstrapSampleSetReceipt,
    samples: &[ReferenceSampleInput],
) -> Result<(), Box<dyn Error>> {
    if sample_set.schema_version != 2
        || sample_set.release != RELEASE
        || sample_set.profile != PROFILE
        || !sample_set.passed
        || !sample_set.bootstrap_eligible
        || sample_set.ship_evidence_eligible
        || samples.len() != 5
        || sample_set.samples.len() != 5
    {
        return Err("sample set is not exact five-run bootstrap evidence".into());
    }
    let mut run_ids = BTreeSet::new();
    let mut receipt_sha256 = BTreeSet::new();
    for (offset, member) in sample_set.samples.iter().enumerate() {
        let expected_index = offset as u32 + 1;
        if member.sample_index != expected_index
            || !run_ids.insert(member.github_run_id.as_str())
            || !receipt_sha256.insert(member.receipt_sha256.as_str())
        {
            return Err("sample set is reordered or contains duplicate provenance".into());
        }
        let sample = samples
            .iter()
            .find(|sample| sample.receipt.sample_index == member.sample_index)
            .ok_or("sample-set member has no input sample")?;
        validate_sample_receipt(&sample.receipt)?;
        if sample.receipt_sha256 != member.receipt_sha256
            || sample.receipt.github_run_id != member.github_run_id
            || sample.receipt.source_commit != sample_set.source_commit
            || sample.receipt.runner_fingerprint != sample_set.runner_fingerprint
            || sample.receipt.runner_provisioning_sha256 != sample_set.runner_provisioning_sha256
            || sample.receipt.prebuild_contract_digest != sample_set.prebuild_contract_digest
            || sample.receipt.scenario_contract_set_digest
                != sample_set.scenario_contract_set_digest
            || sample.receipt.admission_sha256 != sample_set.full_dress_admission_sha256
        {
            return Err("sample-set/member provenance does not match".into());
        }
        if expected_index == 1 {
            if sample.receipt.predecessor_github_run_id.is_some()
                || sample.receipt.predecessor_receipt_sha256.is_some()
            {
                return Err("bootstrap sample 1 is not the chain root".into());
            }
        } else {
            let previous = &sample_set.samples[offset - 1];
            if sample.receipt.predecessor_github_run_id.as_deref()
                != Some(previous.github_run_id.as_str())
                || sample.receipt.predecessor_receipt_sha256.as_deref()
                    != Some(previous.receipt_sha256.as_str())
            {
                return Err("bootstrap sample chain is broken or reordered".into());
            }
        }
    }
    Ok(())
}

fn propose(options: &Options) -> Result<(), Box<dyn Error>> {
    let sample_set_path = options
        .sample_set
        .as_deref()
        .unwrap_or_else(|| Path::new(SAMPLE_SET_PATH));
    let samples_dir = options
        .samples_dir
        .as_deref()
        .ok_or("--samples-dir is required")?;
    let output_dir = options
        .output_dir
        .as_deref()
        .unwrap_or_else(|| Path::new(PROPOSAL_DIR));
    ensure_new_directory(output_dir)?;
    let sample_set_bytes = read_bounded(sample_set_path)?;
    let sample_set: BootstrapSampleSetReceipt = serde_json::from_slice(&sample_set_bytes)?;
    let mut profile: ProfileContract = read_toml(&options.root.join(COMMITTED_PROFILE_PATH))?;
    let budget_template_path = options.root.join(TEMPLATE_BUDGET_PATH);
    let baseline_template_path = options.root.join(TEMPLATE_BASELINE_PATH);
    let budget: BudgetContract = read_toml(&budget_template_path)?;
    let previous_baseline = read_bounded(&baseline_template_path)?;
    let samples = load_samples(samples_dir, &sample_set, &budget)?;
    let proposed_at = options
        .proposed_at
        .as_deref()
        .ok_or("--proposed-at is required")?;
    let producer = options
        .producer
        .as_deref()
        .ok_or("--producer is required")?;
    let rationale = options
        .rationale
        .as_deref()
        .ok_or("--rationale is required")?;
    profile.runner.allowed_fingerprints.clear();
    let (profile, budget, baseline, anchor) = derive_contracts(
        profile,
        budget,
        &sample_set,
        &samples,
        ProposalMetadata {
            sample_set_sha256: &sha256(&sample_set_bytes),
            producer,
            proposed_at,
            rationale,
            previous_baseline_sha256: &sha256(&previous_baseline),
        },
    )?;
    let files = write_contract_files(output_dir, &profile, &budget, &baseline, &anchor)?;
    let mut receipt = ProposalReceipt {
        schema_version: 1,
        release: RELEASE.to_owned(),
        profile: PROFILE.to_owned(),
        producer: producer.to_owned(),
        proposed_at: proposed_at.to_owned(),
        rationale: rationale.to_owned(),
        sample_set_sha256: sha256(&sample_set_bytes),
        sample_receipt_sha256: samples
            .iter()
            .map(|sample| sample.receipt_sha256.clone())
            .collect(),
        runner_provisioning_sha256: sample_set.runner_provisioning_sha256,
        full_dress_admission_sha256: sample_set.full_dress_admission_sha256,
        previous_baseline_sha256: sha256(&previous_baseline),
        proposed_payload_sha256: perf_budget::baseline_payload_digest(&baseline),
        files,
        receipt_sha256: String::new(),
    };
    receipt.seal();
    write_new_json(&output_dir.join("proposal.json"), &receipt)?;
    println!("perf-reference propose: {}", output_dir.display());
    Ok(())
}

fn review(options: &Options) -> Result<(), Box<dyn Error>> {
    let proposal_dir = options
        .proposal_dir
        .as_deref()
        .unwrap_or_else(|| Path::new(PROPOSAL_DIR));
    let decision_path = options
        .decision
        .as_deref()
        .unwrap_or_else(|| Path::new(REVIEW_DECISION_PATH));
    let output_dir = options
        .output_dir
        .as_deref()
        .unwrap_or_else(|| Path::new(REVIEWED_DIR));
    let proposal_bytes = read_bounded(&proposal_dir.join("proposal.json"))?;
    let proposal: ProposalReceipt = serde_json::from_slice(&proposal_bytes)?;
    let proposal_problems = proposal_provenance_problems(&proposal);
    if !proposal_problems.is_empty() {
        return Err(format!("proposal receipt is invalid: {proposal_problems:?}").into());
    }
    verify_files(proposal_dir, &proposal.files)?;
    let decision: ReviewDecision = read_json(decision_path)?;
    validate_review_decision(&proposal, &proposal_bytes, &decision)?;
    let receipt_path = options.root.join(REVIEW_RECEIPT_PATH);
    let (reviewed_files, passed) = match decision.decision {
        ReviewDecisionKind::Reject => (Vec::new(), false),
        ReviewDecisionKind::Approve => {
            ensure_new_directory(output_dir)?;
            let profile: ProfileContract = read_toml(&proposal_dir.join("profile.toml"))?;
            let budget: BudgetContract = read_toml(&proposal_dir.join("budget.toml"))?;
            let mut baseline: RollingBaselineManifest =
                read_toml(&proposal_dir.join("baseline.toml"))?;
            let anchor: AnchorDocument = read_json(&proposal_dir.join("anchor.json"))?;
            let proposal_control = baseline
                .change_control
                .proposal
                .clone()
                .ok_or("proposal baseline has no proposal control")?;
            if proposal_control.proposer != proposal.producer
                || proposal_control.proposed_payload_sha256 != proposal.proposed_payload_sha256
            {
                return Err("proposal change control does not bind proposal receipt".into());
            }
            baseline.change_control.status = ChangeControlStatus::Approved;
            baseline.change_control.approval = Some(BaselineChangeApproval {
                proposal_sha256: perf_budget::digest_json(&proposal_control),
                approved_payload_sha256: proposal.proposed_payload_sha256.clone(),
                approved_at: decision.reviewed_at.clone(),
                approver: decision.reviewer.clone(),
                review_reference: decision.review_reference.clone(),
            });
            perf_budget::seal_baseline_manifest(&mut baseline);
            let bundle = ContractBundle {
                profile: profile.clone(),
                profile_sha256: perf_budget::digest_json(&profile),
                budget: budget.clone(),
                budget_sha256: perf_budget::digest_json(&budget),
                baseline: baseline.clone(),
                baseline_sha256: perf_budget::digest_json(&baseline),
            };
            let problems = perf_budget::validate_contract_bundle(&bundle);
            if !problems.is_empty() {
                return Err(format!("reviewed contract is invalid: {problems:?}").into());
            }
            (
                write_contract_files(output_dir, &profile, &budget, &baseline, &anchor)?,
                true,
            )
        }
    };
    let mut receipt = ReviewReceipt {
        schema_version: 1,
        release: RELEASE.to_owned(),
        profile: PROFILE.to_owned(),
        proposal_receipt_sha256: sha256(&proposal_bytes),
        sample_set_sha256: proposal.sample_set_sha256,
        sample_receipt_sha256: proposal.sample_receipt_sha256,
        runner_provisioning_sha256: proposal.runner_provisioning_sha256,
        full_dress_admission_sha256: proposal.full_dress_admission_sha256,
        producer: proposal.producer,
        reviewer: decision.reviewer,
        reviewed_at: decision.reviewed_at,
        review_reference: decision.review_reference,
        decision: decision.decision,
        reason: decision.reason,
        reviewed_files,
        passed,
        ship_evidence_eligible: false,
        receipt_sha256: String::new(),
    };
    receipt.seal();
    write_new_json(&receipt_path, &receipt)?;
    if passed {
        println!("perf-reference review: approved");
        Ok(())
    } else {
        Err("reference proposal was independently rejected; TD-0013 remains open".into())
    }
}

fn activate(options: &Options) -> Result<(), Box<dyn Error>> {
    let root = &options.root;
    let review: ReviewReceipt = read_json(&root.join(COMMITTED_REVIEW_PATH))?;
    if !review.is_valid()
        || !review.passed
        || review.decision != ReviewDecisionKind::Approve
        || review.ship_evidence_eligible
    {
        return Err("committed baseline review is absent, rejected, or invalid".into());
    }
    let committed = committed_files();
    verify_committed_files(root, &review.reviewed_files, &committed)?;
    let bundle = perf_budget::load_bundle(root, RELEASE, PROFILE)?;
    let current_commit = git_head(root)?;
    let problems = activation_bundle_problems(&bundle, &review, &current_commit);
    if !problems.is_empty() {
        return Err(format!("activated reference contract is invalid: {problems:?}").into());
    }
    let td = fs::read_to_string(root.join(
        "docs/technical-debt/TD-0013-dedicated-performance-runner-and-baseline-bootstrap.md",
    ))?;
    let debt_index = fs::read_to_string(root.join("docs/technical-debt/README.md"))?;
    let td_resolved = td.contains("Resolved")
        && debt_index
            .split("## Resolved Items")
            .nth(1)
            .is_some_and(|resolved| resolved.contains("TD-0013"));
    let release_notes_present = root.join("docs/releases/0.67.1.md").is_file();
    if !td_resolved || !release_notes_present {
        return Err("activation requires TD-0013 resolution and 0.67.1 release notes".into());
    }
    let mut receipt = ActivationReceipt {
        schema_version: 1,
        release: RELEASE.to_owned(),
        profile: PROFILE.to_owned(),
        source_commit: current_commit,
        runner_fingerprint: bundle.profile.runner.allowed_fingerprints[0].clone(),
        review_receipt_sha256: sha256(&read_bounded(&root.join(COMMITTED_REVIEW_PATH))?),
        files: committed
            .iter()
            .map(|(id, path)| file_digest(root, id, path))
            .collect::<Result<Vec<_>, _>>()?,
        td_0013_resolved: true,
        release_notes_present: true,
        passed: true,
        ship_evidence_eligible: true,
        receipt_sha256: String::new(),
    };
    receipt.seal();
    write_new_json(&root.join(ACTIVATION_RECEIPT_PATH), &receipt)?;
    println!("perf-reference activate: OK");
    Ok(())
}

fn frozen_candidate(options: &Options) -> Result<(), Box<dyn Error>> {
    let root = &options.root;
    let context = observe_context(root)?;
    let problems = trusted_performance_context_problems(&context, "frozen-candidate");
    if !problems.is_empty() {
        return Err(format!("frozen candidate context rejected: {problems:?}").into());
    }
    let activation_bytes = read_bounded(&root.join(ACTIVATION_RECEIPT_PATH))?;
    let activation: ActivationReceipt = serde_json::from_slice(&activation_bytes)?;
    let review_bytes = read_bounded(&root.join(COMMITTED_REVIEW_PATH))?;
    let review: ReviewReceipt = serde_json::from_slice(&review_bytes)?;
    let review_problems = review_provenance_problems(&review);
    if !review_problems.is_empty() {
        return Err(format!("frozen candidate review rejected: {review_problems:?}").into());
    }
    let committed = committed_files();
    verify_committed_files(root, &review.reviewed_files, &committed)?;
    let bundle = perf_budget::load_bundle(root, RELEASE, PROFILE)?;
    let bundle_problems = activation_bundle_problems(&bundle, &review, &context.git_head);
    if !bundle_problems.is_empty() {
        return Err(
            format!("frozen candidate activation contract rejected: {bundle_problems:?}").into(),
        );
    }
    let activation_files = committed
        .iter()
        .map(|(id, path)| file_digest(root, id, path))
        .collect::<Result<Vec<_>, _>>()?;
    let activation_problems = activation_receipt_problems(
        &activation,
        &context.git_head,
        &sha256(&review_bytes),
        &bundle.profile.runner.allowed_fingerprints[0],
        &activation_files,
    );
    if !activation_problems.is_empty() {
        return Err(format!(
            "frozen candidate does not consume an exact activation receipt: {activation_problems:?}"
        )
        .into());
    }
    let evidence = collect_full_reference_evidence(root, &context)?;
    if evidence.source_commit != context.git_head
        || evidence.runner_fingerprint != activation.runner_fingerprint
    {
        return Err("frozen candidate evidence differs from activated identity".into());
    }
    let verdict_path = root.join(perf_budget::VERDICT_PATH_0671);
    let verdict_bytes = read_bounded(&verdict_path)?;
    let verdict: serde_json::Value = serde_json::from_slice(&verdict_bytes)?;
    let verdict_payload_sha256 = perf_budget::digest_json(&verdict["payload"]);
    if verdict
        .pointer("/payload/release")
        .and_then(|value| value.as_str())
        != Some(RELEASE)
        || verdict
            .pointer("/payload/profile")
            .and_then(|value| value.as_str())
            != Some(PROFILE)
        || verdict
            .pointer("/payload/candidate_commit")
            .and_then(|value| value.as_str())
            != Some(context.git_head.as_str())
        || verdict
            .pointer("/payload/status")
            .and_then(|value| value.as_str())
            != Some("passed")
        || verdict
            .pointer("/payload/problems")
            .and_then(|value| value.as_array())
            .is_none_or(|v| !v.is_empty())
        || verdict
            .get("receipt_sha256")
            .and_then(|value| value.as_str())
            != Some(verdict_payload_sha256.as_str())
    {
        return Err("frozen candidate budget verdict is not exact and green".into());
    }
    let canaries = validate_canary_receipts(root, &context.git_head)?;
    let mut receipt = FrozenCandidateReceipt {
        schema_version: 1,
        release: RELEASE.to_owned(),
        profile: PROFILE.to_owned(),
        source_commit: context.git_head,
        github_run_id: context.github_run_id,
        runner_fingerprint: evidence.runner_fingerprint,
        activation_sha256: sha256(&activation_bytes),
        budget_verdict_sha256: sha256(&verdict_bytes),
        reference_evidence_sha256: evidence
            .evidence_files
            .iter()
            .map(|artifact| FileDigest {
                id: artifact.path.clone(),
                path: artifact.path.clone(),
                sha256: artifact.sha256.clone(),
            })
            .collect(),
        canary_receipt_sha256: canaries,
        passed: true,
        ship_evidence_eligible: true,
        receipt_sha256: String::new(),
    };
    receipt.seal();
    write_new_json(&root.join(FROZEN_RECEIPT_PATH), &receipt)?;
    println!("perf-reference frozen-candidate: OK");
    Ok(())
}

fn validate_canary_receipts(
    root: &Path,
    source_commit: &str,
) -> Result<Vec<FileDigest>, Box<dyn Error>> {
    let registry = crate::canary_check::load_registry_for_release(root, RELEASE)?;
    let mut files = Vec::new();
    for entry in &registry.entries {
        for relative in &entry.artifacts {
            let path = root.join(relative);
            let receipt: CanaryReceipt = read_json(&path)?;
            let problems =
                canary_sweep::receipt_problems(root, &registry, entry, &receipt, source_commit);
            if !problems.is_empty() || receipt.outcome != CanaryOutcome::ExpectedRed {
                return Err(format!(
                    "canary {} is not exact expected-red proof: {problems:?}",
                    entry.w_item
                )
                .into());
            }
            files.push(file_digest(root, &entry.w_item, relative)?);
        }
    }
    Ok(files)
}

fn validate_review_decision(
    proposal: &ProposalReceipt,
    proposal_bytes: &[u8],
    decision: &ReviewDecision,
) -> Result<(), Box<dyn Error>> {
    let problems = review_decision_problems(proposal, proposal_bytes, decision);
    if !problems.is_empty() {
        return Err(format!(
            "review decision is incomplete, stale, or not independent: {problems:?}"
        )
        .into());
    }
    Ok(())
}

fn proposal_provenance_problems(proposal: &ProposalReceipt) -> Vec<String> {
    let sample_sha256 = proposal
        .sample_receipt_sha256
        .iter()
        .collect::<BTreeSet<_>>();
    let mut problems = Vec::new();
    if !proposal.is_valid()
        || proposal.schema_version != 1
        || proposal.release != RELEASE
        || proposal.profile != PROFILE
        || proposal.producer.trim().is_empty()
        || proposal.rationale.trim().is_empty()
        || OffsetDateTime::parse(&proposal.proposed_at, &Rfc3339).is_err()
        || !is_sha256(&proposal.sample_set_sha256)
        || proposal.sample_receipt_sha256.len() != 5
        || sample_sha256.len() != 5
        || proposal
            .sample_receipt_sha256
            .iter()
            .any(|value| !is_sha256(value))
        || !is_sha256(&proposal.runner_provisioning_sha256)
        || !is_sha256(&proposal.full_dress_admission_sha256)
        || !is_sha256(&proposal.previous_baseline_sha256)
        || !is_sha256(&proposal.proposed_payload_sha256)
    {
        problems.push("proposal lacks exact sealed five-sample provenance".to_owned());
    }
    problems
}

pub fn review_decision_problems(
    proposal: &ProposalReceipt,
    proposal_bytes: &[u8],
    decision: &ReviewDecision,
) -> Vec<String> {
    let mut problems = Vec::new();
    if decision.schema_version != 1
        || decision.release != RELEASE
        || decision.profile != PROFILE
        || decision.proposal_file_sha256 != sha256(proposal_bytes)
    {
        problems.push("review decision does not bind the exact proposal file".to_owned());
    }
    if decision.reviewer.trim().is_empty()
        || decision.reviewer == proposal.producer
        || decision.review_reference.trim().is_empty()
        || decision.reason.trim().is_empty()
    {
        problems.push("review decision lacks an independent reviewer or rationale".to_owned());
    }
    let proposed = OffsetDateTime::parse(&proposal.proposed_at, &Rfc3339);
    let reviewed = OffsetDateTime::parse(&decision.reviewed_at, &Rfc3339);
    if proposed
        .ok()
        .zip(reviewed.ok())
        .is_none_or(|(proposed, reviewed)| reviewed < proposed)
    {
        problems.push("review timestamp predates or cannot parse against proposal".to_owned());
    }
    problems
}

pub fn activation_bundle_problems(
    bundle: &ContractBundle,
    review: &ReviewReceipt,
    candidate_commit: &str,
) -> Vec<String> {
    let mut problems = perf_budget::validate_contract_bundle(bundle);
    if !review.is_valid()
        || !review.passed
        || review.decision != ReviewDecisionKind::Approve
        || review.ship_evidence_eligible
    {
        problems.push("activation lacks exact non-ship independent approval".to_owned());
    }
    problems.extend(review_provenance_problems(review));
    if bundle.profile.bootstrap_status != BootstrapStatus::Bootstrapped
        || bundle.profile.runner.allowed_fingerprints.len() != 1
    {
        problems.push("activation requires one exact bootstrapped fingerprint".to_owned());
    }
    if bundle.baseline.anchor.contract_commit == candidate_commit {
        problems.push("activation candidate cannot baseline itself".to_owned());
    }
    problems.sort();
    problems.dedup();
    problems
}

fn review_provenance_problems(review: &ReviewReceipt) -> Vec<String> {
    let mut problems = Vec::new();
    if review.schema_version != 1
        || review.release != RELEASE
        || review.profile != PROFILE
        || review.producer.trim().is_empty()
        || review.reviewer.trim().is_empty()
        || review.producer == review.reviewer
        || review.review_reference.trim().is_empty()
        || review.reason.trim().is_empty()
        || OffsetDateTime::parse(&review.reviewed_at, &Rfc3339).is_err()
        || !is_sha256(&review.proposal_receipt_sha256)
        || !is_sha256(&review.sample_set_sha256)
        || !is_sha256(&review.runner_provisioning_sha256)
        || !is_sha256(&review.full_dress_admission_sha256)
        || review.sample_receipt_sha256.len() != 5
        || review
            .sample_receipt_sha256
            .iter()
            .any(|value| !is_sha256(value))
        || review
            .sample_receipt_sha256
            .iter()
            .collect::<BTreeSet<_>>()
            .len()
            != 5
    {
        problems.push("activation review lacks exact five-sample transitive provenance".to_owned());
    }
    problems
}

pub fn activation_receipt_problems(
    receipt: &ActivationReceipt,
    source_commit: &str,
    review_receipt_sha256: &str,
    runner_fingerprint: &str,
    files: &[FileDigest],
) -> Vec<String> {
    let mut problems = Vec::new();
    if !receipt.is_valid()
        || receipt.schema_version != 1
        || receipt.release != RELEASE
        || receipt.profile != PROFILE
        || !receipt.passed
        || !receipt.ship_evidence_eligible
        || !receipt.td_0013_resolved
        || !receipt.release_notes_present
    {
        problems.push("frozen candidate activation receipt is invalid or incomplete".to_owned());
    }
    if receipt.source_commit != source_commit
        || receipt.review_receipt_sha256 != review_receipt_sha256
        || receipt.runner_fingerprint != runner_fingerprint
        || receipt.files != files
    {
        problems.push("frozen candidate activation identity or committed files drifted".to_owned());
    }
    problems
}

fn load_samples(
    samples_dir: &Path,
    sample_set: &BootstrapSampleSetReceipt,
    budget: &BudgetContract,
) -> Result<Vec<ReferenceSampleInput>, Box<dyn Error>> {
    let mut inputs = Vec::new();
    for member in &sample_set.samples {
        let root = samples_dir.join(format!("sample-{}", member.sample_index));
        let receipt_path = root.join("bootstrap-sample.json");
        let receipt_bytes = read_bounded(&receipt_path)?;
        let receipt: BootstrapSampleReceipt = serde_json::from_slice(&receipt_bytes)?;
        validate_sample_receipt(&receipt)?;
        for artifact in &receipt.evidence_files {
            let relative = Path::new(&artifact.path);
            if !safe_evidence_path(relative) {
                return Err(format!("unsafe sample artifact path {}", artifact.path).into());
            }
            let bytes = read_bounded(&root.join(relative))?;
            if sha256(&bytes) != artifact.sha256 {
                return Err(format!("sample artifact digest mismatch: {}", artifact.path).into());
            }
        }
        let reports = perf_budget::load_archived_candidate_reports(&root, budget)?;
        inputs.push(ReferenceSampleInput {
            receipt,
            receipt_sha256: sha256(&receipt_bytes),
            reports,
        });
    }
    Ok(inputs)
}

fn write_contract_files(
    directory: &Path,
    profile: &ProfileContract,
    budget: &BudgetContract,
    baseline: &RollingBaselineManifest,
    anchor: &AnchorDocument,
) -> Result<Vec<FileDigest>, Box<dyn Error>> {
    let values = [
        (
            "profile",
            "profile.toml",
            toml::to_string_pretty(profile)?.into_bytes(),
        ),
        (
            "budget",
            "budget.toml",
            toml::to_string_pretty(budget)?.into_bytes(),
        ),
        (
            "baseline",
            "baseline.toml",
            toml::to_string_pretty(baseline)?.into_bytes(),
        ),
        ("anchor", "anchor.json", serde_json::to_vec_pretty(anchor)?),
    ];
    let mut files = Vec::new();
    for (id, name, mut bytes) in values {
        bytes.push(b'\n');
        write_new_bytes(&directory.join(name), &bytes)?;
        files.push(FileDigest {
            id: id.to_owned(),
            path: name.to_owned(),
            sha256: sha256(&bytes),
        });
    }
    Ok(files)
}

fn verify_files(root: &Path, files: &[FileDigest]) -> Result<(), Box<dyn Error>> {
    if files.len() != 4
        || files
            .iter()
            .map(|file| file.id.as_str())
            .collect::<BTreeSet<_>>()
            .len()
            != 4
    {
        return Err("proposal/review file set must contain exactly four unique contracts".into());
    }
    for file in files {
        if Path::new(&file.path)
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
            || sha256(&read_bounded(&root.join(&file.path))?) != file.sha256
        {
            return Err(format!("reviewed file {} has drifted", file.id).into());
        }
    }
    Ok(())
}

fn committed_files() -> Vec<(&'static str, &'static str)> {
    vec![
        ("profile", COMMITTED_PROFILE_PATH),
        ("budget", COMMITTED_BUDGET_PATH),
        ("baseline", COMMITTED_BASELINE_PATH),
        ("anchor", COMMITTED_ANCHOR_PATH),
    ]
}

fn verify_committed_files(
    root: &Path,
    reviewed: &[FileDigest],
    committed: &[(&str, &str)],
) -> Result<(), Box<dyn Error>> {
    if reviewed.len() != committed.len() {
        return Err("review receipt has a partial contract file set".into());
    }
    for (id, path) in committed {
        let expected = reviewed
            .iter()
            .find(|file| file.id == *id)
            .ok_or("reviewed file is absent")?;
        if sha256(&read_bounded(&root.join(path))?) != expected.sha256 {
            return Err(format!("committed {id} differs from independently reviewed bytes").into());
        }
    }
    Ok(())
}

fn file_digest(root: &Path, id: &str, path: &str) -> Result<FileDigest, Box<dyn Error>> {
    Ok(FileDigest {
        id: id.to_owned(),
        path: path.to_owned(),
        sha256: sha256(&read_bounded(&root.join(path))?),
    })
}

fn safe_evidence_path(path: &Path) -> bool {
    path.starts_with("target/test-evidence/0.67")
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn ensure_new_directory(path: &Path) -> Result<(), Box<dyn Error>> {
    if path.exists() {
        return Err(format!(
            "refusing to overwrite existing directory {}",
            path.display()
        )
        .into());
    }
    fs::create_dir_all(path)?;
    Ok(())
}

fn read_bounded(path: &Path) -> Result<Vec<u8>, Box<dyn Error>> {
    let metadata = fs::metadata(path)?;
    if metadata.len() == 0 || metadata.len() > MAX_INPUT_BYTES {
        return Err(format!("{} is empty or oversized", path.display()).into());
    }
    Ok(fs::read(path)?)
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, Box<dyn Error>> {
    Ok(serde_json::from_slice(&read_bounded(path)?)?)
}

fn read_toml<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, Box<dyn Error>> {
    Ok(toml::from_str(std::str::from_utf8(&read_bounded(path)?)?)?)
}

fn write_new_json(path: &Path, value: &impl Serialize) -> Result<(), Box<dyn Error>> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    write_new_bytes(path, &bytes)
}

fn write_new_bytes(path: &Path, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create_new(true).write(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn git_head(root: &Path) -> Result<String, Box<dyn Error>> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        return Err("git rev-parse HEAD failed".into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

#[derive(Debug)]
struct Options {
    root: PathBuf,
    release: String,
    profile: String,
    phase: String,
    sample_set: Option<PathBuf>,
    samples_dir: Option<PathBuf>,
    proposal_dir: Option<PathBuf>,
    decision: Option<PathBuf>,
    output_dir: Option<PathBuf>,
    producer: Option<String>,
    proposed_at: Option<String>,
    rationale: Option<String>,
}

impl Options {
    fn parse(args: Vec<String>) -> Result<Self, Box<dyn Error>> {
        let mut options = Self {
            root: PathBuf::from("."),
            release: String::new(),
            profile: String::new(),
            phase: String::new(),
            sample_set: None,
            samples_dir: None,
            proposal_dir: None,
            decision: None,
            output_dir: None,
            producer: None,
            proposed_at: None,
            rationale: None,
        };
        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            let value = |iter: &mut std::vec::IntoIter<String>, flag: &str| {
                iter.next()
                    .ok_or_else(|| format!("{flag} requires a value"))
            };
            match arg.as_str() {
                "--root" => options.root = PathBuf::from(value(&mut iter, "--root")?),
                "--release" => options.release = value(&mut iter, "--release")?,
                "--profile" => options.profile = value(&mut iter, "--profile")?,
                "--phase" => options.phase = value(&mut iter, "--phase")?,
                "--sample-set" => {
                    options.sample_set = Some(PathBuf::from(value(&mut iter, "--sample-set")?))
                }
                "--samples-dir" => {
                    options.samples_dir = Some(PathBuf::from(value(&mut iter, "--samples-dir")?))
                }
                "--proposal-dir" => {
                    options.proposal_dir = Some(PathBuf::from(value(&mut iter, "--proposal-dir")?))
                }
                "--decision" => {
                    options.decision = Some(PathBuf::from(value(&mut iter, "--decision")?))
                }
                "--output-dir" => {
                    options.output_dir = Some(PathBuf::from(value(&mut iter, "--output-dir")?))
                }
                "--producer" => options.producer = Some(value(&mut iter, "--producer")?),
                "--proposed-at" => options.proposed_at = Some(value(&mut iter, "--proposed-at")?),
                "--rationale" => options.rationale = Some(value(&mut iter, "--rationale")?),
                other => return Err(format!("unsupported perf-reference argument {other}").into()),
            }
        }
        if options.release != RELEASE || options.profile != PROFILE || options.phase.is_empty() {
            return Err(
                "perf-reference requires --release 0.67.1 --profile reference-v1 --phase ..."
                    .into(),
            );
        }
        Ok(options)
    }
}
