use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use hydracache_loadgen::budget_receipt::{BinaryDigest, ReportMetric};
use hydracache_loadgen::profile::{
    reference_cpu_isolation, MeasurementCore, RunnerAttestationV5, RunnerFingerprint,
    REFERENCE_FINGERPRINT_SCHEMA_VERSION, REFERENCE_HOST_CONTRACT_VERSION,
    REFERENCE_MEASUREMENT_CPUS, REFERENCE_OS_IMAGE, REFERENCE_RUNNER_CLASS,
    REFERENCE_STORAGE_CLASS,
};
use xtask::perf_bootstrap::{
    BootstrapArtifactDigest, BootstrapSampleMember, BootstrapSampleReceipt,
    BootstrapSampleSetReceipt,
};
use xtask::perf_budget::{
    self, BaselineChangeApproval, BootstrapStatus, CandidateReport, ChangeControlStatus,
    ContractBundle, EvidenceRunMode,
};
use xtask::perf_reference::{
    activation_bundle_problems, activation_receipt_problems, derive_contracts,
    review_decision_problems, ActivationReceipt, ProposalMetadata, ProposalReceipt,
    ReferenceSampleInput, ReviewDecision, ReviewDecisionKind, ReviewReceipt,
};

const SOURCE_SHA: &str = "0123456789abcdef0123456789abcdef01234567";
const CANDIDATE_SHA: &str = "fedcba9876543210fedcba9876543210fedcba98";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn sha(label: &str) -> String {
    perf_budget::sha256(label.as_bytes())
}

fn observed_runner(fingerprint: &str, prebuild_digest: &str) -> RunnerFingerprint {
    RunnerFingerprint {
        runner_class: REFERENCE_RUNNER_CLASS.to_owned(),
        fingerprint: fingerprint.to_owned(),
        cpu_model: "AMD EPYC 7232P".to_owned(),
        logical_cores: 8,
        ram_bytes: 64 * 1024 * 1024 * 1024,
        os: "linux".to_owned(),
        kernel: "6.8.0-fixture".to_owned(),
        cpu_affinity: "1-4".to_owned(),
        cgroup_cpu_quota: "unlimited".to_owned(),
        governor: "performance".to_owned(),
        turbo: "disabled".to_owned(),
        shared_hardware: false,
        calibration_score: 0.01,
        attestation: RunnerAttestationV5 {
            schema_version: REFERENCE_FINGERPRINT_SCHEMA_VERSION,
            contract_version: REFERENCE_HOST_CONTRACT_VERSION.to_owned(),
            virtualization: "none".to_owned(),
            physical_cores: 8,
            measurement_cores: REFERENCE_MEASUREMENT_CPUS
                .into_iter()
                .map(|logical_cpu| MeasurementCore {
                    logical_cpu,
                    package_id: 0,
                    core_id: logical_cpu,
                })
                .collect(),
            cpu_isolation: reference_cpu_isolation(),
            host_digest: "d".repeat(64),
            storage_class: REFERENCE_STORAGE_CLASS.to_owned(),
            storage_identity_digest: "e".repeat(64),
            os_image: REFERENCE_OS_IMAGE.to_owned(),
            toolchain_identity: "rustc-1.94.0".to_owned(),
            prebuild_contract_digest: prebuild_digest.to_owned(),
        },
    }
}

fn fixture() -> (
    xtask::perf_budget::ProfileContract,
    xtask::perf_budget::BudgetContract,
    BootstrapSampleSetReceipt,
    Vec<ReferenceSampleInput>,
) {
    let root = repo_root();
    let template = perf_budget::load_bundle(&root, "0.67", "reference-v1").unwrap();
    let mut profile = template.profile;
    let budget = template.budget;
    let fingerprint = "c".repeat(64);
    let prebuild_digest = profile.prebuild.digest.clone();
    profile.bootstrap_status = BootstrapStatus::Bootstrapped;
    profile.runner.allowed_fingerprints = vec![fingerprint.clone()];
    let runner_digest = perf_budget::digest_json(&profile.runner);
    let values = [100.0, 110.0, 120.0, 130.0, 10_000.0];
    let mut samples = Vec::new();
    let mut members = Vec::new();
    let mut predecessor: Option<(String, String)> = None;
    for (offset, value) in values.into_iter().enumerate() {
        let index = offset as u32 + 1;
        let run_id = (700 + index).to_string();
        let receipt = BootstrapSampleReceipt {
            schema_version: 2,
            release: "0.67.1".to_owned(),
            profile: "reference-v1".to_owned(),
            source_commit: SOURCE_SHA.to_owned(),
            github_run_id: run_id.clone(),
            observed_at: format!("2026-08-0{index}T12:00:00Z"),
            runner_fingerprint: fingerprint.clone(),
            observed_runner: observed_runner(&fingerprint, &prebuild_digest),
            runner_provisioning_sha256: "6".repeat(64),
            prebuild_contract_digest: prebuild_digest.clone(),
            scenario_contract_set_digest: "b".repeat(64),
            sample_index: index,
            admission_sha256: "8".repeat(64),
            predecessor_github_run_id: predecessor.as_ref().map(|value| value.0.clone()),
            predecessor_receipt_sha256: predecessor.as_ref().map(|value| value.1.clone()),
            evidence_files: vec![BootstrapArtifactDigest {
                path: "target/test-evidence/0.67/local.json".to_owned(),
                sha256: "f".repeat(64),
            }],
            passed: true,
            bootstrap_eligible: true,
            ship_evidence_eligible: false,
        };
        let receipt_sha256 = perf_budget::sha256(&serde_json::to_vec_pretty(&receipt).unwrap());
        let reports = budget
            .reports
            .iter()
            .map(|expected| {
                let binary_sha256 = vec![
                    BinaryDigest {
                        id: "hydracache-loadgen".to_owned(),
                        sha256: sha(&format!("loadgen-{index}")),
                    },
                    BinaryDigest {
                        id: "hydracache-server".to_owned(),
                        sha256: sha(&format!("server-{index}")),
                    },
                ];
                let metrics = budget
                    .budgets
                    .iter()
                    .filter(|rule| rule.report == expected.id)
                    .map(|rule| {
                        (
                            rule.metric.clone(),
                            ReportMetric {
                                id: rule.metric.clone(),
                                value,
                                unit: rule.unit.clone(),
                            },
                        )
                    })
                    .collect::<BTreeMap<_, _>>();
                CandidateReport {
                    id: expected.id.clone(),
                    path: expected.path.clone(),
                    report_id: expected.report_id.clone(),
                    report_sha256: sha(&format!("report-{index}-{}", expected.id)),
                    claim_scope: expected.claim_scope.clone(),
                    run_mode: EvidenceRunMode::ReferenceEvidence,
                    runner_profile: "reference-v1".to_owned(),
                    runner_contract_digest: runner_digest.clone(),
                    runner_class: REFERENCE_RUNNER_CLASS.to_owned(),
                    runner_fingerprint: fingerprint.clone(),
                    source_commit: SOURCE_SHA.to_owned(),
                    cargo_lock_sha256: sha(&format!("cargo-lock-{index}")),
                    toolchain_identity: "rustc-1.94.0".to_owned(),
                    prebuild_contract_digest: prebuild_digest.clone(),
                    prebuild_manifest_sha256: sha(&format!("prebuild-{index}")),
                    binary_set_digest: perf_budget::digest_json(&binary_sha256),
                    binary_sha256,
                    scenario_digest: sha(&format!("scenario-{}", expected.id)),
                    workload_digest: sha(&format!("workload-{}", expected.id)),
                    slo_digest: sha(&format!("slo-{}", expected.id)),
                    methodology_digest: sha(&format!("method-{}", expected.id)),
                    stable: true,
                    maximum_spread_ratio: 0.01,
                    metrics,
                }
            })
            .collect::<Vec<_>>();
        members.push(BootstrapSampleMember {
            sample_index: index,
            github_run_id: run_id,
            source_commit: SOURCE_SHA.to_owned(),
            runner_fingerprint: fingerprint.clone(),
            receipt_sha256: receipt_sha256.clone(),
        });
        predecessor = Some((receipt.github_run_id.clone(), receipt_sha256.clone()));
        samples.push(ReferenceSampleInput {
            receipt,
            receipt_sha256,
            reports,
        });
    }
    let sample_set = BootstrapSampleSetReceipt {
        schema_version: 2,
        release: "0.67.1".to_owned(),
        profile: "reference-v1".to_owned(),
        source_commit: SOURCE_SHA.to_owned(),
        runner_fingerprint: fingerprint,
        runner_provisioning_sha256: "6".repeat(64),
        prebuild_contract_digest: prebuild_digest,
        scenario_contract_set_digest: "b".repeat(64),
        full_dress_admission_sha256: "8".repeat(64),
        samples: members,
        passed: true,
        bootstrap_eligible: true,
        ship_evidence_eligible: false,
    };
    (profile, budget, sample_set, samples)
}

#[test]
fn frozen_candidate_gate_is_wired_to_full_pipeline() {
    let workflow = std::fs::read_to_string(repo_root().join(".github/workflows/ci.yml")).unwrap();
    let _: serde_yaml::Value = serde_yaml::from_str(&workflow).unwrap();
    let job = workflow
        .split("  release-0671-frozen-candidate:")
        .nth(1)
        .expect("frozen-candidate job must exist")
        .split("  raft-loom:")
        .next()
        .expect("frozen-candidate job must be bounded");
    let ordered = [
        "Checkout trusted frozen main",
        "Prepare tmpfs reference evidence",
        "Import frozen campaign host admission",
        "Revalidate committed five-sample independent review",
        "Validate exact reference activation and TD closure",
        "Import offline runner provisioning proof",
        "Attest and preflight the 0.67.1 host",
        "Prebuild exact frozen performance binaries",
        "Run frozen-candidate real 3/5/7 daemon control-plane evidence",
        "Run frozen-candidate core reference evidence",
        "Run frozen-candidate RESP and Redis reference evidence",
        "Check activated 0.67.1 reference budgets and rolling baseline",
        "Materialize tmpfs reference evidence",
        "Execute complete 0.67.1 expected-red canary sweep",
        "Seal exact frozen-candidate reference receipt",
        "Aggregate exact 0.67.1 ship evidence",
        "Upload immutable frozen-candidate evidence",
    ];
    let positions = ordered
        .iter()
        .map(|step| {
            job.find(step)
                .unwrap_or_else(|| panic!("frozen-candidate job lost {step}"))
        })
        .collect::<Vec<_>>();
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
    for required in [
        "inputs.performance_0671_mode == 'frozen-candidate'",
        "github.ref == 'refs/heads/main'",
        "group: release-067-performance-reference-v1",
        "clean: true",
        "persist-credentials: false",
        "--release 0.67.1 --profile reference-v1",
        "--release 0.67.1 --receipts-dir target/release-evidence/receipts --require-ship",
        "if-no-files-found: error",
    ] {
        assert!(
            job.contains(required),
            "frozen-candidate job lost {required}"
        );
    }
    assert!(!job.contains("perf-budget-check --release 0.67 --profile reference-v1"));
}

#[test]
fn live_budget_checks_precede_destructive_tmpfs_materialization() {
    let workflow = std::fs::read_to_string(repo_root().join(".github/workflows/ci.yml")).unwrap();
    let reference_job = workflow
        .split("  release-067-performance:")
        .nth(1)
        .expect("0.67 reference job must exist")
        .split("  release-0671-performance-qualification:")
        .next()
        .expect("0.67 reference job must be bounded");
    assert!(
        reference_job
            .find("Check 0.67 performance budgets")
            .expect("0.67 budget step must exist")
            < reference_job
                .find("Materialize tmpfs reference evidence")
                .expect("0.67 materialization step must exist"),
        "the live 0.67 budget checker must run before materialization deletes /dev/shm"
    );

    let frozen_job = workflow
        .split("  release-0671-frozen-candidate:")
        .nth(1)
        .expect("frozen-candidate job must exist")
        .split("  raft-loom:")
        .next()
        .expect("frozen-candidate job must be bounded");
    assert!(
        frozen_job
            .find("Check activated 0.67.1 reference budgets and rolling baseline")
            .expect("frozen budget step must exist")
            < frozen_job
                .find("Materialize tmpfs reference evidence")
                .expect("frozen materialization step must exist"),
        "the live frozen budget checker must run before materialization deletes /dev/shm"
    );
}

fn approved_bundle() -> (ContractBundle, ReviewReceipt) {
    let (profile, budget, sample_set, samples) = fixture();
    let (profile, budget, mut baseline, _) = derive_contracts(
        profile,
        budget,
        &sample_set,
        &samples,
        ProposalMetadata {
            sample_set_sha256: &sha("sample-set"),
            producer: "reference-automation",
            proposed_at: "2026-08-06T12:00:00Z",
            rationale: "bootstrap exact five-run reference contract",
            previous_baseline_sha256: &sha("previous-baseline"),
        },
    )
    .unwrap();
    let proposal = baseline.change_control.proposal.clone().unwrap();
    baseline.change_control.status = ChangeControlStatus::Approved;
    baseline.change_control.approval = Some(BaselineChangeApproval {
        proposal_sha256: perf_budget::digest_json(&proposal),
        approved_payload_sha256: proposal.proposed_payload_sha256.clone(),
        approved_at: "2026-08-06T13:00:00Z".to_owned(),
        approver: "independent-reviewer".to_owned(),
        review_reference: "review/0.67.1/reference-v1".to_owned(),
    });
    perf_budget::seal_baseline_manifest(&mut baseline);
    let mut review = ReviewReceipt {
        schema_version: 1,
        release: "0.67.1".to_owned(),
        profile: "reference-v1".to_owned(),
        proposal_receipt_sha256: sha("proposal"),
        sample_set_sha256: sha("sample-set"),
        sample_receipt_sha256: (1..=5)
            .map(|index| sha(&format!("sample-{index}")))
            .collect(),
        runner_provisioning_sha256: "6".repeat(64),
        full_dress_admission_sha256: "8".repeat(64),
        producer: "reference-automation".to_owned(),
        reviewer: "independent-reviewer".to_owned(),
        reviewed_at: "2026-08-06T13:00:00Z".to_owned(),
        review_reference: "review/0.67.1/reference-v1".to_owned(),
        decision: ReviewDecisionKind::Approve,
        reason: "all five samples and scoped claims reviewed".to_owned(),
        reviewed_files: Vec::new(),
        passed: true,
        ship_evidence_eligible: false,
        receipt_sha256: String::new(),
    };
    review.seal();
    let bundle = ContractBundle {
        profile_sha256: perf_budget::digest_json(&profile),
        budget_sha256: perf_budget::digest_json(&budget),
        baseline_sha256: perf_budget::digest_json(&baseline),
        profile,
        budget,
        baseline,
    };
    (bundle, review)
}

#[test]
fn reference_proposal_uses_all_five_samples_and_median_not_fastest() {
    let (profile, budget, sample_set, samples) = fixture();
    let (_, _, baseline, anchor) = derive_contracts(
        profile,
        budget,
        &sample_set,
        &samples,
        ProposalMetadata {
            sample_set_sha256: &sha("sample-set"),
            producer: "reference-automation",
            proposed_at: "2026-08-06T12:00:00Z",
            rationale: "bootstrap exact five-run reference contract",
            previous_baseline_sha256: &sha("previous-baseline"),
        },
    )
    .unwrap();
    assert_eq!(baseline.members.len(), 5);
    assert_eq!(baseline.candidate_members.len(), 5);
    assert_eq!(baseline.anchor.source_members.len(), 5);
    assert!(baseline
        .rolling_metrics
        .iter()
        .all(|metric| metric.median == 120.0));
    assert!(anchor
        .anchor
        .metrics
        .iter()
        .all(|metric| metric.value == 120.0));
    assert!(baseline
        .anchor
        .source_run_ids
        .iter()
        .any(|run| run == "705"));
}

#[test]
fn reference_proposal_rejects_a_broken_five_sample_chain() {
    let (profile, budget, sample_set, mut samples) = fixture();
    samples[3].receipt.predecessor_receipt_sha256 = Some(sha("wrong-predecessor"));
    assert!(derive_contracts(
        profile,
        budget,
        &sample_set,
        &samples,
        ProposalMetadata {
            sample_set_sha256: &sha("sample-set"),
            producer: "reference-automation",
            proposed_at: "2026-08-06T12:00:00Z",
            rationale: "bootstrap exact five-run reference contract",
            previous_baseline_sha256: &sha("previous-baseline"),
        },
    )
    .is_err());
}

#[test]
fn bootstrap_accepts_scenario_eligible_spread_but_activation_stays_at_five_percent() {
    let (profile, budget, sample_set, mut samples) = fixture();
    let overload = samples[0]
        .reports
        .iter_mut()
        .find(|report| report.id == "overload-node-resp")
        .expect("overload report");
    overload.maximum_spread_ratio = 0.25;
    let (_, activated_budget, _, _) = derive_contracts(
        profile.clone(),
        budget.clone(),
        &sample_set,
        &samples,
        ProposalMetadata {
            sample_set_sha256: &sha("sample-set"),
            producer: "reference-automation",
            proposed_at: "2026-08-06T12:00:00Z",
            rationale: "bootstrap exact five-run reference contract",
            previous_baseline_sha256: &sha("previous-baseline"),
        },
    )
    .expect("scenario-eligible bootstrap spread");
    assert!(activated_budget
        .budgets
        .iter()
        .all(|rule| rule.maximum_spread_ratio == Some(0.05)));

    let overload = samples[0]
        .reports
        .iter_mut()
        .find(|report| report.id == "overload-node-resp")
        .expect("overload report");
    overload.maximum_spread_ratio = 0.31;
    assert!(derive_contracts(
        profile,
        budget,
        &sample_set,
        &samples,
        ProposalMetadata {
            sample_set_sha256: &sha("sample-set"),
            producer: "reference-automation",
            proposed_at: "2026-08-06T12:00:00Z",
            rationale: "bootstrap exact five-run reference contract",
            previous_baseline_sha256: &sha("previous-baseline"),
        },
    )
    .is_err());
}

#[test]
fn independent_review_binds_exact_proposal_and_rejects_same_identity() {
    let mut proposal = ProposalReceipt {
        schema_version: 1,
        release: "0.67.1".to_owned(),
        profile: "reference-v1".to_owned(),
        producer: "reference-automation".to_owned(),
        proposed_at: "2026-08-06T12:00:00Z".to_owned(),
        rationale: "exact five samples".to_owned(),
        sample_set_sha256: sha("samples"),
        sample_receipt_sha256: vec![sha("1"), sha("2"), sha("3"), sha("4"), sha("5")],
        runner_provisioning_sha256: "6".repeat(64),
        full_dress_admission_sha256: "8".repeat(64),
        previous_baseline_sha256: sha("previous"),
        proposed_payload_sha256: sha("payload"),
        files: Vec::new(),
        receipt_sha256: String::new(),
    };
    proposal.seal();
    let bytes = serde_json::to_vec_pretty(&proposal).unwrap();
    let mut decision = ReviewDecision {
        schema_version: 1,
        release: "0.67.1".to_owned(),
        profile: "reference-v1".to_owned(),
        proposal_file_sha256: perf_budget::sha256(&bytes),
        decision: ReviewDecisionKind::Approve,
        reviewer: "independent-reviewer".to_owned(),
        reviewed_at: "2026-08-06T13:00:00Z".to_owned(),
        review_reference: "review/123".to_owned(),
        reason: "verified exact files".to_owned(),
    };
    assert!(review_decision_problems(&proposal, &bytes, &decision).is_empty());
    decision.reviewer = proposal.producer.clone();
    assert!(!review_decision_problems(&proposal, &bytes, &decision).is_empty());
    decision.reviewer = "independent-reviewer".to_owned();
    decision.proposal_file_sha256 = sha("mutated");
    assert!(!review_decision_problems(&proposal, &bytes, &decision).is_empty());
}

#[test]
fn activation_is_fail_closed_against_self_baseline_and_unreviewed_drift() {
    let (bundle, mut review) = approved_bundle();
    let problems = activation_bundle_problems(&bundle, &review, CANDIDATE_SHA);
    assert!(problems.is_empty(), "{problems:#?}");

    let mut self_baseline = bundle.clone();
    self_baseline.baseline.anchor.contract_commit = CANDIDATE_SHA.to_owned();
    assert!(
        activation_bundle_problems(&self_baseline, &review, CANDIDATE_SHA)
            .iter()
            .any(|problem| problem.contains("baseline itself"))
    );

    review.reason.push_str(" edited");
    assert!(activation_bundle_problems(&bundle, &review, CANDIDATE_SHA)
        .iter()
        .any(|problem| problem.contains("approval")));
}

#[test]
fn canary_reference_review_accepts_silent_numerical_rebaseline() {
    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() != Ok("W5") {
        return;
    }
    let (mut bundle, review) = approved_bundle();
    bundle.baseline.anchor.metrics[0].value *= 2.0;
    assert!(!activation_bundle_problems(&bundle, &review, CANDIDATE_SHA).is_empty());
    panic!("HC-CANARY-RED:W5");
}

#[test]
fn canary_reference_activation_accepts_candidate_self_baseline() {
    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() != Ok("W6") {
        return;
    }
    let (mut bundle, review) = approved_bundle();
    bundle.baseline.anchor.contract_commit = CANDIDATE_SHA.to_owned();
    assert!(!activation_bundle_problems(&bundle, &review, CANDIDATE_SHA).is_empty());
    panic!("HC-CANARY-RED:W6");
}

#[test]
fn canary_frozen_candidate_accepts_stale_activation_identity() {
    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() != Ok("W7") {
        return;
    }
    let (bundle, review) = approved_bundle();
    let review_sha256 = perf_budget::digest_json(&review);
    let mut activation = ActivationReceipt {
        schema_version: 1,
        release: "0.67.1".to_owned(),
        profile: "reference-v1".to_owned(),
        source_commit: CANDIDATE_SHA.to_owned(),
        runner_fingerprint: bundle.profile.runner.allowed_fingerprints[0].clone(),
        review_receipt_sha256: review_sha256.clone(),
        files: Vec::new(),
        td_0013_resolved: true,
        release_notes_present: true,
        passed: true,
        ship_evidence_eligible: true,
        receipt_sha256: String::new(),
    };
    activation.seal();
    assert!(activation_receipt_problems(
        &activation,
        CANDIDATE_SHA,
        &review_sha256,
        &bundle.profile.runner.allowed_fingerprints[0],
        &[],
    )
    .is_empty());
    activation.review_receipt_sha256 = sha("stale-review");
    activation.seal();
    assert!(!activation_receipt_problems(
        &activation,
        CANDIDATE_SHA,
        &review_sha256,
        &bundle.profile.runner.allowed_fingerprints[0],
        &[],
    )
    .is_empty());
    panic!("HC-CANARY-RED:W7");
}
