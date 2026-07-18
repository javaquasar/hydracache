#![allow(dead_code)]

#[path = "../src/perf_budget.rs"]
mod perf_budget;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use perf_budget::{
    AnchorMetric, BaselineChangeApproval, BaselineChangeProposal, BaselineMember,
    BaselineReportReceipt, BinaryDigest, BootstrapStatus, BudgetRuleStatus, CandidateReport,
    ChangeControlStatus, ContractBundle, MacroReportReceipt, MemberMetric, ObservedRunner,
    ReportMetric, VerdictStatus,
};
use serde::{Deserialize, Serialize};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

fn now() -> OffsetDateTime {
    OffsetDateTime::parse("2026-07-18T12:00:00Z", &Rfc3339).unwrap()
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn temp_root(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "hydracache-perf-budget-{label}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

fn sha(label: &str) -> String {
    perf_budget::sha256(label.as_bytes())
}

fn reseal_macro_receipt(envelope: &mut serde_json::Value) {
    let mut receipt: MacroReportReceipt =
        serde_json::from_value(envelope["budget_receipt"].clone()).unwrap();
    receipt.seal().unwrap();
    envelope["budget_receipt"] = serde_json::to_value(receipt).unwrap();
}

fn reseal_macro_source_and_receipt(envelope: &mut serde_json::Value) {
    envelope["budget_receipt"]["source_report_sha256"] =
        serde_json::json!(perf_budget::digest_json(&envelope["report"]));
    reseal_macro_receipt(envelope);
}

#[derive(Clone, Serialize)]
struct FixtureTimeline {
    commit_latency_nanos: u64,
    recovery_latency_nanos: u64,
    receipt_sha256: String,
}

fn fixture_timeline() -> FixtureTimeline {
    let mut timeline = FixtureTimeline {
        commit_latency_nanos: 1_000_000,
        recovery_latency_nanos: 1_500_000,
        receipt_sha256: String::new(),
    };
    timeline.receipt_sha256 = perf_budget::digest_json(&timeline);
    timeline
}

#[derive(Clone, Serialize, Deserialize)]
struct FixtureControlProvenance {
    predecessor_artifact_sha256: String,
    predecessor_receipt_sha256: String,
    predecessor_node_count: u8,
    execution_capability_receipt_sha256: String,
    final_cleanup_receipt_sha256: String,
    scenario_sha256: String,
    receipt_sha256: String,
}

#[derive(Clone, Serialize)]
struct FixtureArchivedLog {
    canonical_path: PathBuf,
    sha256: String,
    bytes: u64,
}

#[derive(Clone, Serialize)]
struct FixtureArchivedNode {
    node_id: String,
    pid: u32,
    kill_requested: bool,
    wait_completed: bool,
    process_no_longer_running: bool,
    exit_status: String,
    stdout_log: FixtureArchivedLog,
    stderr_log: FixtureArchivedLog,
    server_binary_path_after: PathBuf,
    server_binary_sha256_after: String,
    node_config_path_after: PathBuf,
    node_config_sha256_after: String,
}

#[derive(Clone, Serialize)]
struct FixtureFinalCleanup {
    nodes: Vec<FixtureArchivedNode>,
    receipt_sha256: String,
}

fn fixture_file_receipt() -> (PathBuf, String, u64) {
    let path = fs::canonicalize(repo_root().join("Cargo.toml")).unwrap();
    let bytes = fs::read(&path).unwrap();
    let len = u64::try_from(bytes.len()).unwrap();
    (path, perf_budget::sha256(&bytes), len)
}

fn fixture_final_cleanup() -> FixtureFinalCleanup {
    let (path, sha256, bytes) = fixture_file_receipt();
    let nodes = (1..=3)
        .map(|index| FixtureArchivedNode {
            node_id: format!("node-{index}"),
            pid: 9_100 + index,
            kill_requested: true,
            wait_completed: true,
            process_no_longer_running: true,
            exit_status: "fixture-exit".to_owned(),
            stdout_log: FixtureArchivedLog {
                canonical_path: path.clone(),
                sha256: sha256.clone(),
                bytes,
            },
            stderr_log: FixtureArchivedLog {
                canonical_path: path.clone(),
                sha256: sha256.clone(),
                bytes,
            },
            server_binary_path_after: path.clone(),
            server_binary_sha256_after: sha256.clone(),
            node_config_path_after: path.clone(),
            node_config_sha256_after: sha256.clone(),
        })
        .collect();
    let mut cleanup = FixtureFinalCleanup {
        nodes,
        receipt_sha256: String::new(),
    };
    cleanup.receipt_sha256 = perf_budget::digest_json(&cleanup);
    cleanup
}

fn fixture_control_provenance(
    scenario_sha256: &str,
    cleanup_sha256: &str,
) -> FixtureControlProvenance {
    let mut provenance = FixtureControlProvenance {
        predecessor_artifact_sha256: sha("W5A predecessor artifact"),
        predecessor_receipt_sha256: sha("W5A predecessor receipt"),
        predecessor_node_count: 3,
        execution_capability_receipt_sha256: sha("W5A execution capability"),
        final_cleanup_receipt_sha256: cleanup_sha256.to_owned(),
        scenario_sha256: scenario_sha256.to_owned(),
        receipt_sha256: String::new(),
    };
    provenance.receipt_sha256 = perf_budget::digest_json(&provenance);
    provenance
}

fn observed_runner(bundle: &ContractBundle, fingerprint: &str) -> ObservedRunner {
    ObservedRunner {
        runner_class: bundle.profile.runner.required_runner_class.clone(),
        fingerprint: fingerprint.to_owned(),
        cpu_model: "fixture-cpu".to_owned(),
        logical_cores: bundle.profile.runner.minimum_logical_cores,
        ram_bytes: 16 * 1024 * 1024 * 1024,
        os: "linux".to_owned(),
        kernel: "fixture-kernel".to_owned(),
        cpu_affinity: bundle.profile.runner.required_cpu_affinity.clone(),
        cgroup_cpu_quota: bundle.profile.runner.required_cgroup_cpu_quota.clone(),
        governor: "performance".to_owned(),
        turbo: "disabled".to_owned(),
        shared_hardware: false,
        calibration_score: 0.01,
    }
}

fn raw_brownout_window(successes: u64) -> serde_json::Value {
    serde_json::json!({
        "offered": 100,
        "started": 100,
        "completed": 100,
        "successes": successes,
        "errors": 100 - successes,
        "timeouts": 0,
        "rejections": 0,
        "backlog_high_water": 1,
        "backlog_drained": true,
        "drain_ms": 1,
        "elapsed_ms": 100,
        "offered_rate_per_second": 1000.0,
        "achieved_rate_per_second": 1000.0,
        "latency": {"samples": 100, "overflow_count": 0}
    })
}

fn candidate_reports(bundle: &ContractBundle, value: f64) -> Vec<CandidateReport> {
    let runner_digest = perf_budget::digest_json(&bundle.profile.runner);
    bundle
        .budget
        .reports
        .iter()
        .map(|expected| {
            let binary_sha256 = vec![
                BinaryDigest {
                    id: "hydracache-loadgen".to_owned(),
                    sha256: sha("candidate-binary-hydracache-loadgen"),
                },
                BinaryDigest {
                    id: "hydracache-server".to_owned(),
                    sha256: sha("candidate-binary-hydracache-server"),
                },
            ];
            let binary_set_digest = perf_budget::digest_json(&binary_sha256);
            let metrics = bundle
                .budget
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
                .collect();
            CandidateReport {
                id: expected.id.clone(),
                path: expected.path.clone(),
                report_id: expected.report_id.clone(),
                report_sha256: sha(&format!("candidate-report-{}", expected.id)),
                claim_scope: expected.claim_scope.clone(),
                run_mode: perf_budget::EvidenceRunMode::ReferenceEvidence,
                runner_profile: bundle.profile.name.clone(),
                runner_contract_digest: runner_digest.clone(),
                runner_class: bundle.profile.runner.required_runner_class.clone(),
                runner_fingerprint: "reference-runner-fingerprint-v1".to_owned(),
                source_commit: "c".repeat(40),
                cargo_lock_sha256: sha("candidate-cargo-lock"),
                toolchain_identity: bundle.profile.prebuild.toolchain_identity.clone(),
                prebuild_contract_digest: bundle.profile.prebuild.digest.clone(),
                prebuild_manifest_sha256: sha("candidate-prebuild-manifest"),
                binary_sha256,
                binary_set_digest,
                scenario_digest: sha(&format!("scenario-{}", expected.id)),
                workload_digest: sha(&format!("workload-{}", expected.id)),
                slo_digest: sha(&format!("slo-{}", expected.id)),
                methodology_digest: sha(&format!("methodology-{}", expected.id)),
                stable: true,
                maximum_spread_ratio: 0.01,
                metrics,
            }
        })
        .collect()
}

fn member(
    bundle: &ContractBundle,
    reports: &[CandidateReport],
    index: usize,
    metric_value: f64,
) -> BaselineMember {
    let mut member = BaselineMember {
        run_id: format!("main-run-{index}"),
        branch: "main".to_owned(),
        source_commit: format!("{:040x}", index + 1),
        observed_at: format!("2026-07-{:02}T12:00:00Z", 17 - index),
        successful: true,
        quarantined: false,
        calibration_passed: true,
        spread_stable: true,
        gate_exit_code: 0,
        git_status_porcelain_sha256: perf_budget::CLEAN_GIT_STATUS_SHA256.to_owned(),
        quarantine_reason: None,
        runner_contract: bundle.profile.runner.clone(),
        runner_contract_digest: perf_budget::digest_json(&bundle.profile.runner),
        observed_runner: observed_runner(bundle, "reference-runner-fingerprint-v1"),
        runner_fingerprint: "reference-runner-fingerprint-v1".to_owned(),
        toolchain_identity: bundle.profile.prebuild.toolchain_identity.clone(),
        prebuild_contract_digest: bundle.profile.prebuild.digest.clone(),
        profile_sha256: bundle.profile_sha256.clone(),
        budget_sha256: bundle.budget_sha256.clone(),
        reports: reports
            .iter()
            .map(|report| {
                let binary_sha256 = vec![
                    BinaryDigest {
                        id: "hydracache-loadgen".to_owned(),
                        sha256: sha(&format!("baseline-binary-{index}-loadgen")),
                    },
                    BinaryDigest {
                        id: "hydracache-server".to_owned(),
                        sha256: sha(&format!("baseline-binary-{index}-server")),
                    },
                ];
                let metrics = bundle
                    .budget
                    .budgets
                    .iter()
                    .filter(|rule| rule.report == report.id)
                    .map(|rule| ReportMetric {
                        id: rule.metric.clone(),
                        value: metric_value,
                        unit: rule.unit.clone(),
                    })
                    .collect();
                BaselineReportReceipt {
                    report_id: report.id.clone(),
                    report_sha256: sha(&format!("baseline-{index}-{}", report.id)),
                    scenario_digest: report.scenario_digest.clone(),
                    workload_digest: report.workload_digest.clone(),
                    slo_digest: report.slo_digest.clone(),
                    methodology_digest: report.methodology_digest.clone(),
                    cargo_lock_sha256: sha(&format!("cargo-lock-{index}")),
                    prebuild_manifest_sha256: sha(&format!("prebuild-{index}")),
                    binary_set_digest: perf_budget::digest_json(&binary_sha256),
                    binary_sha256,
                    stable: true,
                    maximum_spread_ratio: 0.01,
                    metrics,
                    receipt_sha256: String::new(),
                }
            })
            .collect(),
        metrics: bundle
            .budget
            .budgets
            .iter()
            .map(|rule| MemberMetric {
                budget_id: rule.id.clone(),
                value: metric_value,
                unit: rule.unit.clone(),
            })
            .collect(),
        receipt_sha256: String::new(),
    };
    perf_budget::seal_baseline_member(&mut member);
    member
}

fn approve_baseline_change(bundle: &mut ContractBundle) {
    for member in &mut bundle.baseline.anchor.source_members {
        perf_budget::seal_baseline_member(member);
    }
    for member in &mut bundle.baseline.candidate_members {
        perf_budget::seal_baseline_member(member);
    }
    for member in &mut bundle.baseline.members {
        perf_budget::seal_baseline_member(member);
    }
    let payload_sha256 = perf_budget::baseline_payload_digest(&bundle.baseline);
    let proposal = BaselineChangeProposal {
        proposal_id: "W7-bootstrap-proposal-1".to_owned(),
        proposed_at: "2026-07-18T01:00:00Z".to_owned(),
        proposer: "performance-owner".to_owned(),
        rationale: "review exact first-contract anchor and rolling window".to_owned(),
        previous_manifest_sha256: sha("unbootstrapped-baseline-manifest"),
        proposed_payload_sha256: payload_sha256.clone(),
    };
    bundle.baseline.change_control.status = ChangeControlStatus::Approved;
    bundle.baseline.change_control.approval = Some(BaselineChangeApproval {
        proposal_sha256: perf_budget::digest_json(&proposal),
        approved_payload_sha256: payload_sha256,
        approved_at: "2026-07-18T02:00:00Z".to_owned(),
        approver: "independent-reviewer".to_owned(),
        review_reference: "review/W7-bootstrap-1".to_owned(),
    });
    bundle.baseline.change_control.proposal = Some(proposal);
}

fn bind_reviewed_contract_digests(bundle: &mut ContractBundle) {
    bundle.profile_sha256 = perf_budget::digest_json(&bundle.profile);
    bundle.budget_sha256 = perf_budget::digest_json(&bundle.budget);
    bundle.baseline.profile_sha256 = bundle.profile_sha256.clone();
    bundle.baseline.budget_sha256 = bundle.budget_sha256.clone();
}

fn bootstrapped_fixture() -> (ContractBundle, Vec<CandidateReport>) {
    let mut bundle =
        perf_budget::load_bundle(&repo_root(), perf_budget::RELEASE, "reference-v1").unwrap();
    bundle.profile.bootstrap_status = BootstrapStatus::Bootstrapped;
    bundle.profile.runner.allowed_fingerprints = vec!["reference-runner-fingerprint-v1".to_owned()];
    bundle.budget.bootstrap_status = BootstrapStatus::Bootstrapped;
    for rule in &mut bundle.budget.budgets {
        rule.status = BudgetRuleStatus::Active;
        rule.anchor_tolerance_ratio = Some(0.10);
        rule.rolling_tolerance_ratio = Some(0.10);
        rule.maximum_spread_ratio = Some(0.05);
    }
    bind_reviewed_contract_digests(&mut bundle);
    let reports = candidate_reports(&bundle, 100.0);
    bundle.baseline.bootstrap_status = BootstrapStatus::Bootstrapped;
    bundle.baseline.anchor.status = BootstrapStatus::Bootstrapped;
    bundle.baseline.anchor.frozen_at = "2026-07-18T00:00:00Z".to_owned();
    bundle.baseline.anchor.contract_commit = "a".repeat(40);
    bundle.baseline.anchor.metrics = bundle
        .budget
        .budgets
        .iter()
        .map(|rule| AnchorMetric {
            budget_id: rule.id.clone(),
            value: 100.0,
            unit: rule.unit.clone(),
        })
        .collect();
    bundle.baseline.members = (0..5)
        .map(|index| member(&bundle, &reports, index, 100.0))
        .collect();
    bundle.baseline.candidate_members = bundle.baseline.members.clone();
    bundle.baseline.anchor.source_members = bundle.baseline.members.clone();
    bundle.baseline.anchor.source_run_ids = bundle
        .baseline
        .anchor
        .source_members
        .iter()
        .map(|member| member.run_id.clone())
        .collect();
    bundle.baseline.rolling_metrics =
        perf_budget::rolling_summaries(&bundle.budget.budgets, &bundle.baseline.members).unwrap();
    approve_baseline_change(&mut bundle);
    perf_budget::seal_baseline_manifest(&mut bundle.baseline);
    (bundle, reports)
}

#[test]
fn committed_w7_contract_is_explicitly_unbootstrapped_and_fail_closed() {
    let reference =
        perf_budget::load_bundle(&repo_root(), perf_budget::RELEASE, "reference-v1").unwrap();
    let reference_problems = perf_budget::validate_contract_bundle(&reference);
    assert!(reference_problems.is_empty(), "{reference_problems:#?}");
    assert_eq!(
        reference.profile.bootstrap_status,
        BootstrapStatus::Unbootstrapped
    );
    assert!(reference.profile.runner.allowed_fingerprints.is_empty());
    assert!(reference.baseline.members.is_empty());
    assert!(reference.baseline.anchor.metrics.is_empty());
    let verdict = perf_budget::evaluate(&reference, &[], now());
    assert_eq!(verdict.payload.status, VerdictStatus::Failed);
    assert!(verdict
        .payload
        .problems
        .iter()
        .any(|problem| problem.contains("explicitly unbootstrapped")));

    let shared = perf_budget::load_bundle(&repo_root(), perf_budget::RELEASE, "ci-shared").unwrap();
    assert!(perf_budget::validate_contract_bundle(&shared).is_empty());
    assert!(!shared.profile.noise.absolute_numbers_are_ship_evidence);
}

#[test]
fn rolling_baseline_contract_has_a_versioned_fail_closed_schema() {
    let schema_path = repo_root().join("docs/testing/schemas/perf-rolling-baseline.schema.json");
    let schema: serde_json::Value =
        serde_json::from_slice(&std::fs::read(schema_path).unwrap()).unwrap();
    assert_eq!(
        schema["$schema"],
        "https://json-schema.org/draft/2020-12/schema"
    );
    assert_eq!(schema["properties"]["release"]["const"], "0.67.0");
    assert_eq!(
        schema["$defs"]["policy"]["properties"]["minimum_members"]["const"],
        5
    );
    assert_eq!(
        schema["$defs"]["policy"]["properties"]["maximum_members"]["const"],
        10
    );
    assert_eq!(
        schema["$defs"]["policy"]["properties"]["maximum_age_days"]["const"],
        30
    );
    let serialized = serde_json::to_string(&schema).unwrap();
    assert!(serialized.contains("unbootstrapped"));
    assert!(serialized.contains("receipt_sha256"));
    assert!(serialized.contains("change_control"));
    assert!(serialized.contains("independent") || serialized.contains("approver"));
}

#[test]
fn exact_w7_profiles_cannot_be_weakened_by_relabeling() {
    let mut reference =
        perf_budget::load_bundle(&repo_root(), perf_budget::RELEASE, "reference-v1").unwrap();
    reference.profile.runner.minimum_logical_cores = 1;
    reference.profile.runner.require_dedicated = false;
    reference.profile.noise.maximum_report_spread_ratio = 0.50;
    let problems = perf_budget::validate_contract_bundle(&reference);
    assert!(problems
        .iter()
        .any(|problem| problem.contains("dedicated enforcing profile")));

    let mut shared =
        perf_budget::load_bundle(&repo_root(), perf_budget::RELEASE, "ci-shared").unwrap();
    shared.profile.noise.absolute_numbers_are_ship_evidence = true;
    shared.profile.noise.comparison_class = "absolute-floor".to_owned();
    let problems = perf_budget::validate_contract_bundle(&shared);
    assert!(problems
        .iter()
        .any(|problem| problem.contains("non-enforcing rolling tripwire")));
}

#[test]
fn ci_shared_is_a_rolling_only_non_enforcing_tripwire() {
    let mut bundle =
        perf_budget::load_bundle(&repo_root(), perf_budget::RELEASE, "ci-shared").unwrap();
    bundle.profile.bootstrap_status = BootstrapStatus::Bootstrapped;
    bundle.profile.runner.allowed_fingerprints =
        std::iter::once("reference-runner-fingerprint-v1".to_owned())
            .chain((0..5).map(|index| format!("github-hosted-runner-{index}")))
            .collect();
    bundle.budget.bootstrap_status = BootstrapStatus::Bootstrapped;
    for rule in &mut bundle.budget.budgets {
        rule.status = BudgetRuleStatus::Active;
        rule.anchor_tolerance_ratio = None;
        rule.rolling_tolerance_ratio = Some(0.25);
        rule.maximum_spread_ratio = Some(0.30);
    }
    bind_reviewed_contract_digests(&mut bundle);
    let mut reports = candidate_reports(&bundle, 100.0);
    for report in &mut reports {
        report.run_mode = perf_budget::EvidenceRunMode::CiTripwire;
        report.stable = true;
    }
    bundle.baseline.bootstrap_status = BootstrapStatus::Bootstrapped;
    assert!(bundle.baseline.anchor.metrics.is_empty());
    bundle.baseline.members = (0..5)
        .map(|index| member(&bundle, &reports, index, 100.0))
        .collect();
    for (index, member) in bundle.baseline.members.iter_mut().enumerate() {
        let fingerprint = format!("github-hosted-runner-{index}");
        member.runner_fingerprint = fingerprint.clone();
        member.observed_runner.fingerprint = fingerprint;
    }
    bundle.baseline.candidate_members = bundle.baseline.members.clone();
    bundle.baseline.rolling_metrics =
        perf_budget::rolling_summaries(&bundle.budget.budgets, &bundle.baseline.members).unwrap();
    approve_baseline_change(&mut bundle);
    perf_budget::seal_baseline_manifest(&mut bundle.baseline);
    let problems = perf_budget::validate_contract_bundle(&bundle);
    assert!(problems.is_empty(), "{problems:#?}");

    let verdict = perf_budget::evaluate(&bundle, &reports, now());
    assert_eq!(verdict.payload.status, VerdictStatus::TripwirePassed);
    assert!(verdict
        .payload
        .checks
        .iter()
        .all(|check| check.anchor.is_none()));
    assert!(!bundle.profile.noise.absolute_numbers_are_ship_evidence);
}

#[test]
fn perf_budget_check_fails_on_floor_breach_and_on_unstable_spread() {
    let (bundle, mut reports) = bootstrapped_fixture();
    let floor = bundle
        .budget
        .budgets
        .iter()
        .find(|rule| rule.direction == perf_budget::BudgetDirection::Floor)
        .unwrap();
    let report = reports
        .iter_mut()
        .find(|report| report.id == floor.report)
        .unwrap();
    report.metrics.get_mut(&floor.metric).unwrap().value = 80.0;
    let breached = perf_budget::evaluate(&bundle, &reports, now());
    assert_eq!(breached.payload.status, VerdictStatus::Failed);
    assert!(breached
        .payload
        .problems
        .iter()
        .any(|problem| problem.contains("breached release anchor")));

    let mut reports = candidate_reports(&bundle, 100.0);
    reports[0].maximum_spread_ratio = 0.50;
    let unstable = perf_budget::evaluate(&bundle, &reports, now());
    assert_eq!(unstable.payload.status, VerdictStatus::Failed);
    assert!(unstable
        .payload
        .problems
        .iter()
        .any(|problem| problem.contains("spread ceiling")));
}

#[test]
fn perf_budget_rejects_missing_extra_or_mixed_report_set() {
    let (bundle, reports) = bootstrapped_fixture();
    let missing = perf_budget::evaluate(&bundle, &reports[1..], now());
    assert!(missing
        .payload
        .problems
        .iter()
        .any(|problem| problem.contains("missing, extra, or duplicated")));

    let mut extra = reports.clone();
    let mut duplicate = reports[0].clone();
    duplicate.id = "unexpected-macro-report".to_owned();
    duplicate.path = "target/test-evidence/0.67/overload-extra.json".to_owned();
    extra.push(duplicate);
    let extra = perf_budget::evaluate(&bundle, &extra, now());
    assert_eq!(extra.payload.status, VerdictStatus::Failed);

    let mut mixed = reports.clone();
    mixed[0].source_commit = "d".repeat(40);
    let mixed = perf_budget::evaluate(&bundle, &mixed, now());
    assert!(mixed
        .payload
        .problems
        .iter()
        .any(|problem| problem.contains("mixes commit/profile/fingerprint")));
}

#[test]
fn macro_receipt_revalidates_runner_facts_instead_of_trusting_a_profile_label() {
    let (bundle, _) = bootstrapped_fixture();
    let expected = bundle
        .budget
        .reports
        .iter()
        .find(|report| report.id == "brownout-control-plane")
        .unwrap();
    let observed = observed_runner(&bundle, "reference-runner-fingerprint-v1");
    let (_, fixture_server_sha256, _) = fixture_file_receipt();
    let binary_sha256 = vec![
        BinaryDigest {
            id: "hydracache-loadgen".to_owned(),
            sha256: sha("macro-loadgen-binary"),
        },
        BinaryDigest {
            id: "hydracache-server".to_owned(),
            sha256: fixture_server_sha256,
        },
    ];
    let receipt = MacroReportReceipt {
        schema_version: 1,
        release: perf_budget::RELEASE.to_owned(),
        report_id: expected.report_id.clone(),
        source_report_sha256: String::new(),
        claim_scope: expected.claim_scope.clone(),
        run_mode: perf_budget::EvidenceRunMode::ReferenceEvidence,
        runner_profile: bundle.profile.name.clone(),
        runner_contract: bundle.profile.runner.clone(),
        runner_contract_digest: perf_budget::digest_json(&bundle.profile.runner),
        observed_runner: observed,
        runner_fingerprint: "reference-runner-fingerprint-v1".to_owned(),
        source_commit: "c".repeat(40),
        cargo_lock_sha256: sha("macro-cargo-lock"),
        toolchain_identity: bundle.profile.prebuild.toolchain_identity.clone(),
        prebuild_contract_digest: bundle.profile.prebuild.digest.clone(),
        prebuild_manifest_sha256: sha("macro-prebuild"),
        binary_set_digest: perf_budget::digest_json(&binary_sha256),
        binary_sha256,
        scenario_digest: sha("macro-scenario"),
        workload_digest: sha("macro-workload"),
        slo_digest: sha("macro-slo"),
        methodology_digest: sha("macro-methodology"),
        stable: true,
        maximum_spread_ratio: 0.0,
        metrics: bundle
            .budget
            .budgets
            .iter()
            .filter(|rule| rule.report == expected.id)
            .map(|rule| ReportMetric {
                id: rule.metric.clone(),
                value: if rule.metric.ends_with("recovery_milliseconds") {
                    2.0
                } else {
                    500_000.0
                },
                unit: rule.unit.clone(),
            })
            .collect(),
        receipt_sha256: String::new(),
    };
    let events = [
        "leader_failover",
        "member_add",
        "member_drain",
        "node_kill_rejoin",
    ]
    .into_iter()
    .map(|action| {
        serde_json::json!({
            "action": action,
            "transition_recovery_millis": 2,
            "raw": {
                "timeline": fixture_timeline(),
                "disruption_window": raw_brownout_window(50)
            }
        })
    })
    .collect::<Vec<_>>();
    let scenario_sha256 = sha("brownout-control-plane-scenario");
    let cleanup = fixture_final_cleanup();
    let provenance = fixture_control_provenance(&scenario_sha256, &cleanup.receipt_sha256);
    let report = serde_json::json!({
        "schema_version": 1,
        "scenario_id": "brownout-control-plane-v1",
        "scenario_sha256": scenario_sha256.clone(),
        "evidence_class": "w5a-control-plane-metadata-brownout",
        "run_mode": "reference",
        "predecessor": {
            "evidence_class": "w4a-real-daemon-control-plane",
            "artifact_sha256": provenance.predecessor_artifact_sha256.clone(),
            "reference_receipt_sha256": provenance.predecessor_receipt_sha256.clone(),
            "knee_rate_per_second": 1_000,
            "offered_rate_per_second": 600,
            "rate_fraction_millionths": 600_000
        },
        "predecessor_node_count": 3,
        "reference_provenance": provenance,
        "final_cleanup": cleanup,
        "events": events,
        "generic_client_write_invariant": false,
        "distributed_value_invariant": false,
        "live_reshard_measured": false,
        "aggregate_goodput": false
    });
    let mut receipt = receipt;
    receipt.source_report_sha256 = perf_budget::digest_json(&report);
    receipt.seal().unwrap();
    assert!(receipt.receipt_is_valid());
    let bytes = serde_json::to_vec(&serde_json::json!({
        "report": report,
        "budget_receipt": receipt,
    }))
    .unwrap();
    let normalized =
        perf_budget::normalize_report(expected, perf_budget::Enforcement::Ship, &bytes).unwrap();
    assert_eq!(
        normalized.runner_fingerprint,
        "reference-runner-fingerprint-v1"
    );
    let sealed: hydracache_loadgen::budget_receipt::MacroReportEnvelope<serde_json::Value> =
        serde_json::from_slice(&bytes).unwrap();
    sealed.validate_seal().unwrap();
    assert!(!sealed.to_pretty_json().unwrap().is_empty());
    assert!(perf_budget::normalize_report(
        expected,
        perf_budget::Enforcement::NonEnforcingTripwire,
        &bytes,
    )
    .unwrap_err()
    .to_string()
    .contains("identity is mismatched"));

    let mut forged_topology: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    forged_topology["report"]["predecessor_node_count"] = serde_json::json!(5);
    let mut forged_provenance: FixtureControlProvenance =
        serde_json::from_value(forged_topology["report"]["reference_provenance"].clone()).unwrap();
    forged_provenance.predecessor_node_count = 5;
    forged_provenance.receipt_sha256.clear();
    forged_provenance.receipt_sha256 = perf_budget::digest_json(&forged_provenance);
    forged_topology["report"]["reference_provenance"] =
        serde_json::to_value(forged_provenance).unwrap();
    reseal_macro_source_and_receipt(&mut forged_topology);
    assert!(perf_budget::normalize_report(
        expected,
        perf_budget::Enforcement::Ship,
        &serde_json::to_vec(&forged_topology).unwrap(),
    )
    .unwrap_err()
    .to_string()
    .contains("provenance receipt"));

    let mut tampered_seal: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    tampered_seal["budget_receipt"]["stable"] = serde_json::json!(false);
    let tampered_seal = serde_json::to_vec(&tampered_seal).unwrap();
    assert!(perf_budget::normalize_report(
        expected,
        perf_budget::Enforcement::Ship,
        &tampered_seal
    )
    .unwrap_err()
    .to_string()
    .contains("canonical seal does not recompute"));

    let mut unknown_envelope: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    unknown_envelope["self_asserted_pass"] = serde_json::json!(true);
    let unknown_envelope = serde_json::to_vec(&unknown_envelope).unwrap();
    assert!(perf_budget::normalize_report(
        expected,
        perf_budget::Enforcement::Ship,
        &unknown_envelope,
    )
    .unwrap_err()
    .to_string()
    .contains("missing or unknown fields"));

    let mut unknown_field: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    unknown_field["report"]["self_asserted_pass"] = serde_json::json!(true);
    unknown_field["budget_receipt"]["source_report_sha256"] =
        serde_json::json!(perf_budget::digest_json(&unknown_field["report"]));
    reseal_macro_receipt(&mut unknown_field);
    let unknown_field = serde_json::to_vec(&unknown_field).unwrap();
    assert!(perf_budget::normalize_report(
        expected,
        perf_budget::Enforcement::Ship,
        &unknown_field,
    )
    .unwrap_err()
    .to_string()
    .contains("missing or unknown fields"));

    let mut forged_stability: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    forged_stability["budget_receipt"]["stable"] = serde_json::json!(false);
    reseal_macro_receipt(&mut forged_stability);
    let forged_stability = serde_json::to_vec(&forged_stability).unwrap();
    assert!(perf_budget::normalize_report(
        expected,
        perf_budget::Enforcement::Ship,
        &forged_stability,
    )
    .unwrap_err()
    .to_string()
    .contains("stability do not recompute"));

    let mut forged_metric: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    forged_metric["budget_receipt"]["metric"][0]["value"] = serde_json::json!(999.0);
    reseal_macro_receipt(&mut forged_metric);
    let forged_metric = serde_json::to_vec(&forged_metric).unwrap();
    assert!(perf_budget::normalize_report(
        expected,
        perf_budget::Enforcement::Ship,
        &forged_metric,
    )
    .unwrap_err()
    .to_string()
    .contains("do not recompute"));

    let mut forged: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    forged["budget_receipt"]["observed_runner"]["shared_hardware"] = serde_json::json!(true);
    reseal_macro_receipt(&mut forged);
    let bytes = serde_json::to_vec(&forged).unwrap();
    assert!(
        perf_budget::normalize_report(expected, perf_budget::Enforcement::Ship, &bytes)
            .unwrap_err()
            .to_string()
            .contains("does not revalidate")
    );
}

#[test]
fn baseline_eligibility_is_derived_from_receipts_not_caller_booleans() {
    let (mut exit_code, _) = bootstrapped_fixture();
    exit_code.baseline.members[0].gate_exit_code = 1;
    exit_code.baseline.members[0].successful = true;
    exit_code.baseline.candidate_members = exit_code.baseline.members.clone();
    approve_baseline_change(&mut exit_code);
    perf_budget::seal_baseline_manifest(&mut exit_code.baseline);
    assert!(perf_budget::validate_contract_bundle(&exit_code)
        .iter()
        .any(|problem| problem.contains("outcome/clean-checkout/runner/spread")));

    let (mut dirty, _) = bootstrapped_fixture();
    dirty.baseline.members[0].git_status_porcelain_sha256 = sha("M src/lib.rs");
    dirty.baseline.candidate_members = dirty.baseline.members.clone();
    approve_baseline_change(&mut dirty);
    perf_budget::seal_baseline_manifest(&mut dirty.baseline);
    assert!(perf_budget::validate_contract_bundle(&dirty)
        .iter()
        .any(|problem| problem.contains("clean-checkout")));

    let (mut noisy, _) = bootstrapped_fixture();
    noisy.baseline.members[0].reports[0].maximum_spread_ratio = 0.20;
    noisy.baseline.members[0].reports[0].stable = true;
    noisy.baseline.candidate_members = noisy.baseline.members.clone();
    approve_baseline_change(&mut noisy);
    perf_budget::seal_baseline_manifest(&mut noisy.baseline);
    assert!(perf_budget::validate_contract_bundle(&noisy)
        .iter()
        .any(|problem| problem.contains("runner/spread")));
}

#[test]
fn baseline_change_requires_payload_bound_independent_approval() {
    let (mut bundle, _) = bootstrapped_fixture();
    bundle.baseline.rolling_metrics[0].median += 1.0;
    perf_budget::seal_baseline_manifest(&mut bundle.baseline);
    assert!(perf_budget::validate_contract_bundle(&bundle)
        .iter()
        .any(|problem| problem.contains("proposal/independent approval")));

    let (mut same_person, _) = bootstrapped_fixture();
    let proposer = same_person
        .baseline
        .change_control
        .proposal
        .as_ref()
        .unwrap()
        .proposer
        .clone();
    same_person
        .baseline
        .change_control
        .approval
        .as_mut()
        .unwrap()
        .approver = proposer;
    perf_budget::seal_baseline_manifest(&mut same_person.baseline);
    assert!(perf_budget::validate_contract_bundle(&same_person)
        .iter()
        .any(|problem| problem.contains("proposal/independent approval")));
}

#[test]
fn perf_budget_change_requires_reviewed_budget_file_edit() {
    let (mut bundle, _) = bootstrapped_fixture();
    bundle.budget.budgets[0].anchor_tolerance_ratio = Some(0.99);
    bundle.budget_sha256 = sha("silently-edited-budget-file");
    let problems = perf_budget::validate_contract_bundle(&bundle);
    assert!(problems
        .iter()
        .any(|problem| problem.contains("reviewed profile/budget file digests")));

    bundle.baseline.budget_sha256 = bundle.budget_sha256.clone();
    let problems = perf_budget::validate_contract_bundle(&bundle);
    assert!(problems
        .iter()
        .any(|problem| problem.contains("manifest receipt digest")));
}

#[test]
fn rolling_baseline_uses_only_eligible_same_fingerprint_main_reports() {
    let (bundle, reports) = bootstrapped_fixture();
    let mut pool = bundle.baseline.members.clone();

    let mut foreign = member(&bundle, &reports, 5, 100.0);
    foreign.run_id = "newer-foreign-fingerprint".to_owned();
    foreign.observed_at = "2026-07-18T11:59:00Z".to_owned();
    foreign.runner_fingerprint = "other-runner".to_owned();
    perf_budget::seal_baseline_member(&mut foreign);
    pool.push(foreign);

    let mut failed = member(&bundle, &reports, 6, 100.0);
    failed.run_id = "newer-failed-main".to_owned();
    failed.observed_at = "2026-07-18T11:58:00Z".to_owned();
    failed.successful = false;
    perf_budget::seal_baseline_member(&mut failed);
    pool.push(failed);

    let mut quarantined = member(&bundle, &reports, 7, 100.0);
    quarantined.run_id = "newer-quarantined-main".to_owned();
    quarantined.observed_at = "2026-07-18T11:57:00Z".to_owned();
    quarantined.quarantined = true;
    perf_budget::seal_baseline_member(&mut quarantined);
    pool.push(quarantined);

    let mut feature_branch = member(&bundle, &reports, 8, 100.0);
    feature_branch.run_id = "newer-feature-branch".to_owned();
    feature_branch.observed_at = "2026-07-18T11:56:00Z".to_owned();
    feature_branch.branch = "feat/perf".to_owned();
    perf_budget::seal_baseline_member(&mut feature_branch);
    pool.push(feature_branch);

    let selected = perf_budget::select_eligible_members(
        &pool,
        &bundle.baseline,
        &reports,
        now(),
        &reports[0].source_commit,
    );
    assert_eq!(selected.len(), 5);
    assert!(selected
        .iter()
        .all(|member| member.run_id.starts_with("main-run-")));

    let twelve = (0..12)
        .map(|index| member(&bundle, &reports, index, 100.0))
        .collect::<Vec<_>>();
    let selected = perf_budget::select_eligible_members(
        &twelve,
        &bundle.baseline,
        &reports,
        now(),
        &reports[0].source_commit,
    );
    assert_eq!(selected.len(), 10);
    assert!(selected
        .iter()
        .all(|member| { member.run_id != "main-run-10" && member.run_id != "main-run-11" }));
}

#[test]
fn rolling_baseline_rejects_mixed_stale_insufficient_or_unstable_window() {
    let (mut mixed, reports) = bootstrapped_fixture();
    mixed.baseline.members[0].reports[0].methodology_digest = sha("other-methodology");
    mixed.baseline.candidate_members = mixed.baseline.members.clone();
    perf_budget::seal_baseline_manifest(&mut mixed.baseline);
    let verdict = perf_budget::evaluate(&mixed, &reports, now());
    assert!(verdict
        .payload
        .problems
        .iter()
        .any(|problem| problem.contains("mixed/stale/unstable")));

    let (mut stale, reports) = bootstrapped_fixture();
    stale.baseline.members[0].observed_at = "2026-05-01T00:00:00Z".to_owned();
    stale.baseline.candidate_members = stale.baseline.members.clone();
    perf_budget::seal_baseline_manifest(&mut stale.baseline);
    let verdict = perf_budget::evaluate(&stale, &reports, now());
    assert_eq!(verdict.payload.status, VerdictStatus::Failed);
    assert!(verdict
        .payload
        .problems
        .iter()
        .any(|problem| problem.contains("mixed/stale/unstable")));

    let (mut unstable, reports) = bootstrapped_fixture();
    unstable.baseline.members[0].spread_stable = false;
    unstable.baseline.candidate_members = unstable.baseline.members.clone();
    perf_budget::seal_baseline_manifest(&mut unstable.baseline);
    let verdict = perf_budget::evaluate(&unstable, &reports, now());
    assert!(verdict
        .payload
        .problems
        .iter()
        .any(|problem| problem.contains("fewer than five")));

    let (mut insufficient, reports) = bootstrapped_fixture();
    insufficient.baseline.members.pop();
    insufficient.baseline.candidate_members = insufficient.baseline.members.clone();
    insufficient.baseline.rolling_metrics = perf_budget::rolling_summaries(
        &insufficient.budget.budgets,
        &insufficient.baseline.members,
    )
    .unwrap();
    perf_budget::seal_baseline_manifest(&mut insufficient.baseline);
    let structural = perf_budget::validate_contract_bundle(&insufficient);
    assert!(structural
        .iter()
        .any(|problem| problem.contains("window is insufficient")));
    let verdict = perf_budget::evaluate(&insufficient, &reports, now());
    assert_eq!(verdict.payload.status, VerdictStatus::Failed);
}

#[test]
fn candidate_cannot_baseline_itself() {
    let (mut bundle, reports) = bootstrapped_fixture();
    bundle.baseline.members[0].source_commit = reports[0].source_commit.clone();
    bundle.baseline.candidate_members = bundle.baseline.members.clone();
    perf_budget::seal_baseline_manifest(&mut bundle.baseline);
    let verdict = perf_budget::evaluate(&bundle, &reports, now());
    assert_eq!(verdict.payload.status, VerdictStatus::Failed);
    assert!(verdict
        .payload
        .problems
        .iter()
        .any(|problem| problem.contains("fewer than five")));
}

#[test]
fn release_anchor_prevents_slow_rolling_ratcheting() {
    let (mut bundle, mut reports) = bootstrapped_fixture();
    for member in &mut bundle.baseline.members {
        for metric in &mut member.metrics {
            if let Some(rule) = bundle
                .budget
                .budgets
                .iter()
                .find(|rule| rule.id == metric.budget_id)
                .filter(|rule| rule.direction == perf_budget::BudgetDirection::Floor)
            {
                metric.value = 90.0;
                member
                    .reports
                    .iter_mut()
                    .find(|report| report.report_id == rule.report)
                    .and_then(|report| {
                        report
                            .metrics
                            .iter_mut()
                            .find(|report_metric| report_metric.id == rule.metric)
                    })
                    .unwrap()
                    .value = 90.0;
            }
        }
        perf_budget::seal_baseline_member(member);
    }
    bundle.baseline.candidate_members = bundle.baseline.members.clone();
    bundle.baseline.rolling_metrics =
        perf_budget::rolling_summaries(&bundle.budget.budgets, &bundle.baseline.members).unwrap();
    approve_baseline_change(&mut bundle);
    perf_budget::seal_baseline_manifest(&mut bundle.baseline);
    for report in &mut reports {
        for metric in report.metrics.values_mut() {
            if bundle.budget.budgets.iter().any(|rule| {
                rule.report == report.id
                    && rule.metric == metric.id
                    && rule.direction == perf_budget::BudgetDirection::Floor
            }) {
                metric.value = 85.0;
            }
        }
    }
    let verdict = perf_budget::evaluate(&bundle, &reports, now());
    assert_eq!(verdict.payload.status, VerdictStatus::Failed);
    assert!(verdict
        .payload
        .checks
        .iter()
        .any(|check| { check.candidate == 85.0 && check.rolling_median == 90.0 && !check.passed }));
}

#[test]
fn baseline_manifest_and_budget_verdict_are_receipt_digest_bound() {
    let (mut bundle, reports) = bootstrapped_fixture();
    let problems = perf_budget::validate_contract_bundle(&bundle);
    assert!(problems.is_empty(), "{problems:#?}");
    let verdict = perf_budget::evaluate(&bundle, &reports, now());
    assert_eq!(verdict.payload.status, VerdictStatus::Passed);
    assert!(verdict.receipt_is_valid());
    assert_eq!(verdict.payload.baseline_members.len(), 5);
    assert!(verdict
        .payload
        .baseline_members
        .iter()
        .all(|member| member.eligible));

    let mut forged_verdict = verdict.clone();
    forged_verdict
        .payload
        .problems
        .push("post-hash mutation".to_owned());
    assert!(!forged_verdict.receipt_is_valid());

    bundle.baseline.members[0].reports[0].report_sha256 = sha("forged-report");
    let problems = perf_budget::validate_contract_bundle(&bundle);
    assert!(problems.iter().any(
        |problem| problem.contains("baseline report") && problem.contains("does not recompute")
    ));
}

#[test]
fn budget_verdict_is_create_new_atomic_and_rejects_stale_final_or_temp() {
    let (bundle, reports) = bootstrapped_fixture();
    let verdict = perf_budget::evaluate(&bundle, &reports, now());
    assert!(verdict.receipt_is_valid());

    let root = temp_root("verdict-final");
    perf_budget::write_verdict(&root, &verdict).expect("first verdict");
    let final_path = root.join(perf_budget::VERDICT_PATH);
    let original = fs::read(&final_path).unwrap();
    assert!(perf_budget::write_verdict(&root, &verdict).is_err());
    assert_eq!(fs::read(&final_path).unwrap(), original);
    fs::remove_dir_all(&root).unwrap();

    let root = temp_root("verdict-temp");
    let final_path = root.join(perf_budget::VERDICT_PATH);
    fs::create_dir_all(final_path.parent().unwrap()).unwrap();
    let temp_path = final_path.with_extension("json.tmp");
    fs::write(&temp_path, b"recoverable-stale-temp").unwrap();
    assert!(perf_budget::write_verdict(&root, &verdict).is_err());
    assert!(!final_path.exists());
    assert_eq!(fs::read(&temp_path).unwrap(), b"recoverable-stale-temp");
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn every_capacity_bearing_surface_has_a_reference_v1_anchor() {
    let (mut bundle, _) = bootstrapped_fixture();
    let problems = perf_budget::validate_contract_bundle(&bundle);
    assert!(problems.is_empty(), "{problems:#?}");
    let capacity_report = bundle
        .budget
        .reports
        .iter()
        .find(|report| report.capacity_bearing)
        .unwrap();
    let budget_id = bundle
        .budget
        .budgets
        .iter()
        .find(|rule| rule.report == capacity_report.id)
        .unwrap()
        .id
        .clone();
    bundle
        .baseline
        .anchor
        .metrics
        .retain(|metric| metric.budget_id != budget_id);
    perf_budget::seal_baseline_manifest(&mut bundle.baseline);
    let problems = perf_budget::validate_contract_bundle(&bundle);
    assert!(problems.iter().any(|problem| {
        problem.contains("capacity-bearing surface")
            || problem.contains("lacks a valid reference-v1 anchor")
    }));
}

#[test]
fn canary_perf_budget_accepts_a_silent_rebaseline_or_candidate_self_baseline() {
    let (mut bundle, reports) = bootstrapped_fixture();
    bundle.baseline.members[0].source_commit = reports[0].source_commit.clone();
    bundle.baseline.candidate_members = bundle.baseline.members.clone();
    perf_budget::seal_baseline_manifest(&mut bundle.baseline);
    let self_baseline = perf_budget::evaluate(&bundle, &reports, now());
    assert_eq!(self_baseline.payload.status, VerdictStatus::Failed);

    let (mut silent, _) = bootstrapped_fixture();
    silent.budget_sha256 = sha("unreviewed-budget-rebaseline");
    let silent_problems = perf_budget::validate_contract_bundle(&silent);
    assert!(!silent_problems.is_empty());

    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W7") {
        panic!("HC-CANARY-RED:W7 silent rebaseline or candidate self-baseline was accepted");
    }
}
