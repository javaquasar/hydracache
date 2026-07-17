use std::fs;
use std::process::Command;

use xtask::evidence_run::{ArtifactDigest, EvidenceOutcome, EvidenceReceipt, NormalizedResult};

const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

fn root() -> std::path::PathBuf {
    xtask::doc_check::find_repo_root().unwrap()
}

fn base_receipt(gate: &xtask::gated_tests::GateEntry, source_commit: &str) -> EvidenceReceipt {
    let expected = xtask::evidence_run::expected_digests(&root(), gate).unwrap();
    EvidenceReceipt {
        schema_version: 1,
        release: "0.64.0".to_owned(),
        gate_id: gate.id.clone(),
        source_commit: source_commit.to_owned(),
        dirty_worktree: false,
        command_digest: expected.command,
        registry_digest: expected.registry,
        input_digest: expected.input,
        toolchain: "rustc test".to_owned(),
        container_identity: Default::default(),
        platform: "test".to_owned(),
        started_at: "2026-07-14T00:00:00Z".to_owned(),
        ended_at: "2026-07-14T00:00:01Z".to_owned(),
        duration_ms: 1_000,
        outcome: EvidenceOutcome::Pass,
        exit_code: Some(0),
        normalized_result: NormalizedResult {
            outcome: EvidenceOutcome::Pass,
            exit_code: Some(0),
            stdout_sha256: EMPTY_SHA256.to_owned(),
            stderr_sha256: EMPTY_SHA256.to_owned(),
        },
        stdout: String::new(),
        stderr: String::new(),
        artifacts: vec![],
        missing_artifacts: vec![],
    }
}

#[test]
fn release_evidence_reports_every_manifest_work_item_exactly_once() {
    let report = xtask::release_evidence::build_report(&root(), "0.64", None).unwrap();
    let ids: Vec<_> = report
        .work_items
        .iter()
        .map(|item| item.id.as_str())
        .collect();
    assert_eq!(
        ids.iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        ids.len()
    );
    assert!(ids.contains(&"W5a"));
    assert!(ids.contains(&"W6b"));
    assert!(ids.contains(&"W38"));
    assert!(ids.contains(&"W39"));
}

#[test]
fn release_evidence_never_reuses_canaries_with_equal_ids_from_an_older_release() {
    let root = root();
    let current = xtask::release_evidence::build_report(&root, "0.64", None).unwrap();
    assert!(current.work_items.iter().all(|item| !item
        .reasons
        .iter()
        .any(|reason| reason.contains("not cross-release evidence"))));

    assert!(xtask::release_evidence::dynamic_canary_release_problem(
        xtask::release_evidence::CanaryPolicy::DynamicRegistry,
        "0.64.0",
        "0.65.0"
    )
    .is_some());
}

#[test]
fn explicit_flip_sentinel_policy_can_advance_without_an_unrelated_dynamic_registry() {
    assert!(xtask::release_evidence::dynamic_canary_release_problem(
        xtask::release_evidence::CanaryPolicy::DedicatedFlipSentinels,
        "0.64.0",
        "0.65.0"
    )
    .is_none());

    let root = root();
    let manifest = xtask::release_evidence::parse_manifest_text(
        &fs::read_to_string(root.join("docs/testing/release-evidence/0.65.toml")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        manifest.dynamic_canary_work_items,
        vec!["W1", "W2", "W3", "W4"]
    );
    assert!(matches!(
        manifest.canary_policy,
        xtask::release_evidence::CanaryPolicy::DedicatedFlipSentinelsWithDynamicRegistry
    ));

    let registry = xtask::canary_check::load_registry_for_release(&root, "0.65").unwrap();
    let canary_receipts = xtask::canary_sweep::load_receipts(&root).unwrap();
    let source_commit = String::from_utf8(
        Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&root)
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .to_owned();

    let report = xtask::release_evidence::build_report(&root, "0.65", None).unwrap();
    assert!(!report.work_items.is_empty());
    assert!(report
        .work_items
        .iter()
        .all(|item| item.stage >= xtask::release_evidence::EvidenceStage::Implemented));

    for item in &report.work_items {
        let has_dynamic_reason = item
            .reasons
            .iter()
            .any(|reason| reason.contains("dynamic canary"));
        if matches!(item.id.as_str(), "W1" | "W2" | "W3" | "W4") {
            let entry = registry
                .entries
                .iter()
                .find(|entry| entry.w_item == item.id)
                .expect("dynamic canary registry entry");
            let receipt = canary_receipts
                .iter()
                .find(|receipt| receipt.w_item == item.id);
            let receipt_is_invalid = receipt.is_none_or(|receipt| {
                !xtask::canary_sweep::receipt_problems(
                    &root,
                    &registry,
                    entry,
                    receipt,
                    &source_commit,
                )
                .is_empty()
            });
            assert_eq!(
                has_dynamic_reason, receipt_is_invalid,
                "W1-W4 dynamic-canary reason must match the observed receipt state for {}",
                item.id
            );
        } else {
            assert!(
                !has_dynamic_reason,
                "unrelated work item {} must not be blocked by dynamic canary policy",
                item.id
            );
        }
    }
}

#[test]
fn release_evidence_marks_missing_skipped_stale_or_wrong_commit_receipts_non_green() {
    let root = root();
    let registry = xtask::gated_tests::load_registry(&root).unwrap();
    let gate = &registry.gate[0];
    let mut receipt = base_receipt(gate, "candidate");
    assert!(
        xtask::release_evidence::receipt_problems(&root, "0.64", "candidate", gate, &receipt)
            .is_empty()
    );

    receipt.source_commit = "stale".to_owned();
    receipt.outcome = EvidenceOutcome::Skip;
    receipt.normalized_result.outcome = EvidenceOutcome::Skip;
    let problems =
        xtask::release_evidence::receipt_problems(&root, "0.64", "candidate", gate, &receipt);
    assert!(problems
        .iter()
        .any(|problem| problem.contains("wrong source commit")));
    assert!(problems.iter().any(|problem| problem.contains("Skip")));

    let report = xtask::release_evidence::build_report(&root, "0.64", None).unwrap();
    assert!(report
        .work_items
        .iter()
        .all(|item| item.stage <= xtask::release_evidence::EvidenceStage::Implemented));
}

#[test]
fn release_evidence_rejects_handwritten_green_status_and_tampered_artifact_hash() {
    let text = r#"
schema_version = 1
release = "0.64.0"
plan = "plan.md"

[[work_item]]
id = "W1"
required_sources = ["source.rs"]
required_tests = []
required_artifacts = []
fast_gate_ids = []
gated_gate_ids = []
ship_required = true
status = "green"
"#;
    assert!(xtask::release_evidence::parse_manifest_text(text).is_err());

    let root = root();
    let registry = xtask::gated_tests::load_registry(&root).unwrap();
    let mut gate = registry.gate[0].clone();
    let relative = "Cargo.toml";
    gate.artifacts = vec![relative.to_owned()];
    let mut receipt = base_receipt(&gate, "candidate");
    receipt.artifacts = vec![ArtifactDigest {
        path: relative.to_owned(),
        sha256: "00".repeat(32),
        bytes: 0,
    }];
    let problems =
        xtask::release_evidence::receipt_problems(&root, "0.64", "candidate", &gate, &receipt);
    assert!(problems
        .iter()
        .any(|problem| problem.contains("artifact hash mismatch")));
}

#[test]
fn release_evidence_rejects_dirty_receipts_and_path_traversal() {
    let root = root();
    let registry = xtask::gated_tests::load_registry(&root).unwrap();
    let mut gate = registry.gate[0].clone();
    let mut receipt = base_receipt(&gate, "candidate");
    receipt.dirty_worktree = true;
    assert!(
        xtask::release_evidence::receipt_problems(&root, "0.64", "candidate", &gate, &receipt,)
            .iter()
            .any(|problem| problem.contains("dirty worktree"))
    );

    gate.artifacts = vec!["../outside".to_owned()];
    receipt.dirty_worktree = false;
    receipt.artifacts = vec![ArtifactDigest {
        path: "../outside".to_owned(),
        sha256: EMPTY_SHA256.to_owned(),
        bytes: 0,
    }];
    assert!(
        xtask::release_evidence::receipt_problems(&root, "0.64", "candidate", &gate, &receipt,)
            .iter()
            .any(|problem| problem.contains("unsafe repository path"))
    );
}
