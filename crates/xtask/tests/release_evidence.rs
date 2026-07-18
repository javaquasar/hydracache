use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use xtask::evidence_run::{ArtifactDigest, EvidenceOutcome, EvidenceReceipt, NormalizedResult};

const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

fn root() -> std::path::PathBuf {
    xtask::doc_check::find_repo_root().unwrap()
}

struct ScratchRepo {
    root: PathBuf,
}

impl ScratchRepo {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "hydracache-release-evidence-template-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    fn path(&self) -> &Path {
        &self.root
    }

    fn write(&self, relative: &str, contents: impl AsRef<[u8]>) {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }
}

impl Drop for ScratchRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn canary_registry(release: &str, source: &str, function: &str) -> String {
    serde_json::to_string_pretty(&serde_json::json!({
        "version": 2,
        "release": release,
        "entries": [{
            "w_item": "W0",
            "guard": { "file": source, "function": function },
            "canary": { "file": source, "function": format!("{function}_red") },
            "guard_command": {
                "program": "cargo",
                "args": ["test", "-p", "fixture", "--", function],
                "cwd": ".",
                "platform": "any"
            },
            "canary_command": {
                "program": "cargo",
                "args": ["test", "-p", "fixture", "--", format!("{function}_red")],
                "cwd": ".",
                "platform": "any"
            },
            "defect_id": format!("{release}-W0"),
            "expected_failure": "seeded defect remains observable",
            "timeout_seconds": 30,
            "tier": "fast",
            "artifacts": ["target/release-evidence/canaries/W0.json"],
            "red_evidence": "target/release-evidence/canaries/W0.json"
        }]
    }))
    .unwrap()
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
fn release_evidence_template_cannot_borrow_equal_w_ids_from_the_0_64_registry() {
    let repo = ScratchRepo::new();
    repo.write(
        "docs/plans/releases.toml",
        r#"
[[release]]
version = "0.66.0"
file = "docs/plans/0.66.md"
work_items = ["W0", "W1"]
"#,
    );
    repo.write(
        "docs/testing/fast-suite-registry.toml",
        r#"
schema_version = 1
release = "0.66.0"
nextest_version = "0.9.137"
aggregate_budget_seconds = 1500
suite = []
"#,
    );
    repo.write(
        "docs/testing/canary-registry.json",
        canary_registry(
            "0.64.0",
            "tests/guards/release_0_64_w0.rs",
            "release_0_64_w0_guard",
        ),
    );
    repo.write(
        "docs/testing/canary-registry-0.66.json",
        canary_registry(
            "0.66.0",
            "tests/guards/release_0_66_w0.rs",
            "release_0_66_w0_guard",
        ),
    );

    xtask::release_evidence::run(vec![
        "--root".to_owned(),
        repo.path().display().to_string(),
        "--release".to_owned(),
        "0.66".to_owned(),
        "--emit-template".to_owned(),
    ])
    .unwrap();

    let manifest = xtask::release_evidence::parse_manifest_text(
        &fs::read_to_string(repo.path().join("docs/testing/release-evidence/0.66.toml")).unwrap(),
    )
    .unwrap();
    let w0 = manifest
        .work_item
        .iter()
        .find(|item| item.id == "W0")
        .unwrap();
    assert_eq!(w0.required_sources, ["tests/guards/release_0_66_w0.rs"]);
    assert_eq!(w0.required_tests.len(), 1);
    assert_eq!(w0.required_tests[0].function, "release_0_66_w0_guard");
    assert_eq!(manifest.dynamic_canary_work_items, vec!["W0".to_owned()]);
    assert!(!w0
        .required_sources
        .iter()
        .any(|source| source.contains("release_0_64")));
}

#[test]
fn release_evidence_template_preserves_dynamic_canary_selection() {
    let repo = ScratchRepo::new();
    repo.write(
        "docs/plans/releases.toml",
        r#"
[[release]]
version = "0.66.0"
file = "docs/plans/0.66.md"
work_items = ["W0"]
"#,
    );
    repo.write(
        "docs/testing/fast-suite-registry.toml",
        r#"
schema_version = 1
release = "0.66.0"
nextest_version = "0.9.137"
aggregate_budget_seconds = 1500
suite = []
"#,
    );
    repo.write(
        "docs/testing/canary-registry-0.66.json",
        canary_registry(
            "0.66.0",
            "tests/guards/release_0_66_w0.rs",
            "release_0_66_w0_guard",
        ),
    );
    repo.write(
        "docs/testing/release-evidence/0.66.toml",
        r#"
schema_version = 1
release = "0.66.0"
plan = "docs/plans/0.66.md"
canary_policy = "dynamic_registry"
dynamic_canary_work_items = ["W0"]

[[work_item]]
id = "W0"
required_sources = ["tests/guards/release_0_66_w0.rs"]
required_tests = []
required_artifacts = []
fast_gate_ids = ["fast.workspace-nextest"]
gated_gate_ids = []
ship_required = true
"#,
    );

    xtask::release_evidence::run(vec![
        "--root".to_owned(),
        repo.path().display().to_string(),
        "--release".to_owned(),
        "0.66".to_owned(),
        "--emit-template".to_owned(),
    ])
    .unwrap();

    let manifest = xtask::release_evidence::parse_manifest_text(
        &fs::read_to_string(repo.path().join("docs/testing/release-evidence/0.66.toml")).unwrap(),
    )
    .unwrap();
    assert_eq!(manifest.dynamic_canary_work_items, vec!["W0".to_owned()]);
    assert_eq!(
        manifest.work_item[0].fast_gate_ids,
        vec!["fast.workspace-nextest".to_owned()]
    );
}

#[test]
fn release_evidence_template_rejects_a_mislabeled_release_registry() {
    let repo = ScratchRepo::new();
    repo.write(
        "docs/plans/releases.toml",
        r#"
[[release]]
version = "0.66.0"
file = "docs/plans/0.66.md"
work_items = ["W0"]
"#,
    );
    repo.write(
        "docs/testing/fast-suite-registry.toml",
        r#"
schema_version = 1
release = "0.66.0"
nextest_version = "0.9.137"
aggregate_budget_seconds = 1500
suite = []
"#,
    );
    repo.write(
        "docs/testing/canary-registry-0.66.json",
        canary_registry(
            "0.64.0",
            "tests/guards/release_0_64_w0.rs",
            "release_0_64_w0_guard",
        ),
    );

    let error = xtask::release_evidence::run(vec![
        "--root".to_owned(),
        repo.path().display().to_string(),
        "--release".to_owned(),
        "0.66".to_owned(),
        "--emit-template".to_owned(),
    ])
    .unwrap_err()
    .to_string();

    assert!(
        error.contains("registry release 0.64.0 does not match template release 0.66.0"),
        "mislabeled registry unexpectedly generated a template: {error}"
    );
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

#[test]
fn final_aggregator_requires_exact_candidate_receipts_and_artifact_hashes() {
    let root = root();
    let registry = xtask::gated_tests::load_registry(&root).unwrap();
    let mut gate = registry
        .gate
        .iter()
        .find(|gate| gate.id == "tool.perf-prebuild-067")
        .unwrap()
        .clone();
    gate.artifacts = vec!["Cargo.toml".to_owned()];
    let bytes = fs::read(root.join("Cargo.toml")).unwrap();
    let mut receipt = base_receipt(&gate, "candidate");
    receipt.release = "0.67.0".to_owned();
    receipt.artifacts = vec![ArtifactDigest {
        path: "Cargo.toml".to_owned(),
        sha256: xtask::perf::sha256_bytes(&bytes),
        bytes: bytes.len() as u64,
    }];
    assert!(
        xtask::release_evidence::receipt_problems(&root, "0.67", "candidate", &gate, &receipt,)
            .is_empty()
    );

    let mut wrong_commit = receipt.clone();
    wrong_commit.source_commit = "stale".to_owned();
    assert!(xtask::release_evidence::receipt_problems(
        &root,
        "0.67",
        "candidate",
        &gate,
        &wrong_commit,
    )
    .iter()
    .any(|problem| problem.contains("wrong source commit")));

    let mut tampered = receipt.clone();
    tampered.artifacts[0].sha256 = "00".repeat(32);
    assert!(xtask::release_evidence::receipt_problems(
        &root,
        "0.67",
        "candidate",
        &gate,
        &tampered,
    )
    .iter()
    .any(|problem| problem.contains("artifact hash mismatch")));

    let mut extra = receipt;
    extra.artifacts.push(ArtifactDigest {
        path: "unexpected.json".to_owned(),
        sha256: EMPTY_SHA256.to_owned(),
        bytes: 0,
    });
    assert!(
        xtask::release_evidence::receipt_problems(&root, "0.67", "candidate", &gate, &extra,)
            .iter()
            .any(|problem| problem.contains("exact declared gate artifact set"))
    );

    let fixture_dir = root
        .join("target/test-evidence/0.67")
        .join(format!("release-evidence-unit-{}", std::process::id()));
    assert!(fixture_dir.starts_with(root.join("target/test-evidence/0.67")));
    let _ = fs::remove_dir_all(&fixture_dir);
    fs::create_dir_all(&fixture_dir).unwrap();
    let raw_path = fixture_dir.join("daemon.stderr.log");
    fs::write(&raw_path, b"original daemon log").unwrap();
    let raw_bytes = fs::read(&raw_path).unwrap();
    let report_path = fixture_dir.join("report.json");
    let report = serde_json::json!({
        "archived_log": {
            "canonical_path": raw_path.canonicalize().unwrap(),
            "bytes": raw_bytes.len(),
            "sha256": xtask::perf::sha256_bytes(&raw_bytes),
        }
    });
    fs::write(&report_path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    let report_relative = report_path
        .strip_prefix(&root)
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/");
    gate.artifacts = vec![report_relative.clone()];
    let report_bytes = fs::read(&report_path).unwrap();
    let mut archived_receipt = base_receipt(&gate, "candidate");
    archived_receipt.release = "0.67.0".to_owned();
    archived_receipt.artifacts = vec![ArtifactDigest {
        path: report_relative,
        sha256: xtask::perf::sha256_bytes(&report_bytes),
        bytes: report_bytes.len() as u64,
    }];
    assert!(xtask::release_evidence::receipt_problems(
        &root,
        "0.67",
        "candidate",
        &gate,
        &archived_receipt,
    )
    .is_empty());

    fs::write(&raw_path, b"tampered daemon log").unwrap();
    assert!(xtask::release_evidence::receipt_problems(
        &root,
        "0.67",
        "candidate",
        &gate,
        &archived_receipt,
    )
    .iter()
    .any(|problem| problem.contains("archived-file receipt hash mismatch")));
    fs::remove_dir_all(&fixture_dir).unwrap();
}
