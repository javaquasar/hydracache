use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use xtask::canary_check::{
    check_canary_registry, check_canary_registry_for_release, CanaryCommand, CanaryEntry,
    CanaryRegistry, CanaryTier, FunctionRef,
};
use xtask::canary_sweep::{
    classify_canary_result, execute_entry, load_receipts_for_release, receipt_problems,
    CanaryOutcome, ProcessResult,
};

static SCRATCH_COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn every_w_item_has_a_registered_canary_that_references_real_functions() {
    let root = workspace_root();
    let problems = check_canary_registry(&root).unwrap();
    assert!(problems.is_empty(), "registry problems: {problems:#?}");

    let registry = xtask::canary_check::load_registry(&root).unwrap();
    let required = xtask::canary_check::required_work_items(&root, "0.64").unwrap();
    assert_eq!(registry.entries.len(), required.len());
}

#[test]
fn each_canary_makes_its_paired_guard_fail_red() {
    let root = workspace_root();
    let registry = xtask::canary_check::load_registry(&root).unwrap();
    for entry in registry.entries {
        assert_ne!(
            serde_json::to_vec(&entry.guard_command).unwrap(),
            serde_json::to_vec(&entry.canary_command).unwrap()
        );
        assert!(!entry.expected_failure.trim().is_empty());
        assert!(!entry.defect_id.trim().is_empty());
        assert!(
            entry
                .canary_command
                .env
                .contains_key("HYDRACACHE_CANARY_DEFECT")
                || matches!(entry.tier, CanaryTier::External)
                || entry.w_item == "W26",
            "{} has no executable defect switch",
            entry.w_item
        );
    }
}

#[test]
fn dynamic_canary_runner_rejects_a_guard_that_stays_green() {
    assert_eq!(
        classify_canary_result(&process(Some(0), "", false), "HC-CANARY-RED:fixture"),
        CanaryOutcome::StayedGreen
    );
}

#[test]
fn dynamic_canary_runner_rejects_timeout_compile_failure_or_unrelated_panic_as_red_evidence() {
    let expected = "HC-CANARY-RED:fixture";
    assert_eq!(
        classify_canary_result(&process(None, "", true), expected),
        CanaryOutcome::Timeout
    );
    assert_eq!(
        classify_canary_result(
            &process(
                Some(101),
                "error[E0599]: missing item\ncould not compile demo",
                false
            ),
            expected
        ),
        CanaryOutcome::CompileFailure
    );
    assert_eq!(
        classify_canary_result(
            &process(Some(101), "panicked at unrelated assertion", false),
            expected
        ),
        CanaryOutcome::WrongFailure
    );
    assert_eq!(
        classify_canary_result(&process(Some(101), expected, false), expected),
        CanaryOutcome::ExpectedRed
    );
}

#[test]
fn dynamic_canary_receipt_is_bound_to_command_defect_and_source_commit() {
    let root = scratch_root();
    fs::create_dir_all(&root).unwrap();
    let executable = std::env::current_exe().unwrap().display().to_string();
    let guard = CanaryCommand {
        program: executable.clone(),
        args: vec![
            "--exact".to_owned(),
            "dynamic_canary_fixture_process".to_owned(),
            "--nocapture".to_owned(),
        ],
        env: BTreeMap::new(),
        cwd: ".".to_owned(),
        platform: "any".to_owned(),
    };
    let mut canary = guard.clone();
    canary
        .env
        .insert("HYDRACACHE_CANARY_FIXTURE".to_owned(), "red".to_owned());
    let entry = fixture_entry(guard, canary);
    let registry = CanaryRegistry {
        version: 2,
        release: "0.64.0".to_owned(),
        entries: vec![entry.clone()],
    };
    let receipt = execute_entry(&root, &registry, &entry).unwrap();
    assert_eq!(receipt.outcome, CanaryOutcome::ExpectedRed);

    let mut tampered = receipt.clone();
    tampered.source_commit = "f".repeat(40);
    tampered.defect_id = "different-defect".to_owned();
    tampered.command_digest = "0".repeat(64);
    let problems = receipt_problems(&root, &registry, &entry, &tampered, &receipt.source_commit);
    assert!(problems
        .iter()
        .any(|problem| problem == "wrong source commit"));
    assert!(problems.iter().any(|problem| problem == "stale defect id"));
    assert!(problems
        .iter()
        .any(|problem| problem == "stale canary command digest"));
    cleanup(&root);
}

#[test]
fn equal_work_item_receipts_from_different_releases_do_not_shadow_each_other() {
    let root = scratch_root();
    fs::create_dir_all(&root).unwrap();
    let executable = std::env::current_exe().unwrap().display().to_string();
    let guard = CanaryCommand {
        program: executable.clone(),
        args: vec![
            "--exact".to_owned(),
            "dynamic_canary_fixture_process".to_owned(),
            "--nocapture".to_owned(),
        ],
        env: BTreeMap::new(),
        cwd: ".".to_owned(),
        platform: "any".to_owned(),
    };
    let mut canary = guard.clone();
    canary
        .env
        .insert("HYDRACACHE_CANARY_FIXTURE".to_owned(), "red".to_owned());
    let entry = fixture_entry(guard, canary);
    let registry_064 = CanaryRegistry {
        version: 2,
        release: "0.64.0".to_owned(),
        entries: vec![entry.clone()],
    };
    let registry_065 = CanaryRegistry {
        version: 2,
        release: "0.65.0".to_owned(),
        entries: vec![entry.clone()],
    };

    execute_entry(&root, &registry_064, &entry).unwrap();
    execute_entry(&root, &registry_065, &entry).unwrap();

    let receipts_064 = load_receipts_for_release(&root, "0.64").unwrap();
    let receipts_065 = load_receipts_for_release(&root, "0.65").unwrap();
    assert!(receipts_064
        .iter()
        .any(|receipt| receipt.release == "0.64.0" && receipt.w_item == "W1"));
    assert!(receipts_065
        .iter()
        .any(|receipt| receipt.release == "0.65.0" && receipt.w_item == "W1"));
    assert!(root
        .join("target/release-evidence/canaries/0.64-W1.json")
        .is_file());
    assert!(root
        .join("target/release-evidence/canaries/0.65-W1.json")
        .is_file());
    cleanup(&root);
}

#[test]
fn canary_registry_lists_a_canary_that_does_not_fail_its_guard() {
    let root = scratch_root();
    write_scratch_contract(&root);
    let command = command_json(BTreeMap::new());
    fs::write(
        root.join("docs/testing/canary-registry.json"),
        format!(
            r#"{{
  "version": 2,
  "release": "0.64.0",
  "entries": [{{
    "w_item": "W1",
    "guard": {{ "file": "crates/demo/tests/proof.rs", "function": "guard_test" }},
    "canary": {{ "file": "crates/demo/tests/proof.rs", "function": "canary_test" }},
    "guard_command": {command},
    "canary_command": {command},
    "defect_id": "inert",
    "expected_failure": "HC-CANARY-RED:fixture",
    "timeout_seconds": 10,
    "tier": "fast",
    "artifacts": ["target/release-evidence/canaries/W1.json"],
    "red_evidence": "fixture is inert"
  }}]
}}"#
        ),
    )
    .unwrap();
    let problems = check_canary_registry(&root).unwrap();
    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W17") {
        assert!(
            problems.is_empty(),
            "HC-CANARY-RED:W17 inert dynamic canary entry was accepted"
        );
    }
    cleanup(&root);
    assert!(problems
        .iter()
        .any(|problem| problem.contains("commands are identical")));
}

#[test]
fn canary_receipt_artifact_cannot_escape_the_evidence_directory() {
    let root = scratch_root();
    write_scratch_contract(&root);
    let command = CanaryCommand {
        program: "cargo".to_owned(),
        args: vec!["test".to_owned()],
        env: BTreeMap::new(),
        cwd: ".".to_owned(),
        platform: "any".to_owned(),
    };
    let mut entry = fixture_entry(command.clone(), command);
    entry.guard = FunctionRef {
        file: "crates/demo/tests/proof.rs".to_owned(),
        function: "guard_test".to_owned(),
    };
    entry.canary = FunctionRef {
        file: "crates/demo/tests/proof.rs".to_owned(),
        function: "canary_test".to_owned(),
    };
    entry.artifacts = vec!["Cargo.toml".to_owned()];
    let registry = CanaryRegistry {
        version: 2,
        release: "0.64.0".to_owned(),
        entries: vec![entry],
    };
    fs::write(
        root.join("docs/testing/canary-registry.json"),
        serde_json::to_vec_pretty(&registry).unwrap(),
    )
    .unwrap();

    let problems = check_canary_registry(&root).unwrap();
    cleanup(&root);
    assert!(problems.iter().any(|problem| problem
        .contains("receipt artifact must be a JSON file under target/release-evidence/canaries")));
}

#[test]
fn requested_release_does_not_borrow_an_older_canary_registry() {
    let root = scratch_root();
    fs::create_dir_all(root.join("docs/plans")).unwrap();
    fs::create_dir_all(root.join("docs/testing")).unwrap();
    fs::write(
        root.join("docs/plans/releases.toml"),
        "[[release]]\nversion = \"0.66.0\"\nfile = \"docs/plans/custom-0.66.md\"\nstatus = \"planned\"\nwork_items = [\"W0\"]\n",
    )
    .unwrap();
    fs::write(
        root.join("docs/testing/canary-registry.json"),
        r#"{"version":2,"release":"0.64.0","entries":[]}"#,
    )
    .unwrap();

    let error = check_canary_registry_for_release(&root, "0.66")
        .unwrap_err()
        .to_string();
    cleanup(&root);

    assert!(
        error.contains("canary-registry-0.66.json"),
        "requested release silently borrowed legacy evidence: {error}"
    );
}

#[test]
fn requested_release_without_work_items_is_rejected_before_canary_evidence() {
    let root = scratch_root();
    fs::create_dir_all(root.join("docs/plans")).unwrap();
    fs::create_dir_all(root.join("docs/testing")).unwrap();
    fs::write(
        root.join("docs/plans/releases.toml"),
        "[[release]]\nversion = \"0.66.0\"\nfile = \"docs/plans/custom-0.66.md\"\nstatus = \"planned\"\n",
    )
    .unwrap();
    fs::write(
        root.join("docs/testing/canary-registry-0.66.json"),
        r#"{"version":2,"release":"0.66.0","entries":[]}"#,
    )
    .unwrap();

    let error = check_canary_registry_for_release(&root, "0.66")
        .unwrap_err()
        .to_string();
    cleanup(&root);

    assert!(
        error.contains("release 0.66 has no work_items in releases.toml"),
        "candidate without an explicit work-item contract was accepted: {error}"
    );
}

#[test]
fn dynamic_canary_fixture_process() {
    if std::env::var("HYDRACACHE_CANARY_FIXTURE").as_deref() == Ok("red") {
        panic!("HC-CANARY-RED:fixture invariant rejected the injected defect");
    }
}

fn process(exit_code: Option<i32>, output: &str, timed_out: bool) -> ProcessResult {
    ProcessResult {
        exit_code,
        stdout: output.to_owned(),
        stderr: String::new(),
        timed_out,
        skipped: false,
    }
}

fn fixture_entry(guard: CanaryCommand, canary: CanaryCommand) -> CanaryEntry {
    CanaryEntry {
        w_item: "W1".to_owned(),
        guard: FunctionRef {
            file: "fixture.rs".to_owned(),
            function: "guard".to_owned(),
        },
        canary: FunctionRef {
            file: "fixture.rs".to_owned(),
            function: "canary".to_owned(),
        },
        guard_command: guard,
        canary_command: canary,
        defect_id: "fixture-defect".to_owned(),
        expected_failure: "HC-CANARY-RED:fixture".to_owned(),
        timeout_seconds: 10,
        tier: CanaryTier::Fast,
        artifacts: vec!["target/release-evidence/canaries/W1.json".to_owned()],
        red_evidence: "fixture".to_owned(),
    }
}

fn command_json(env: BTreeMap<String, String>) -> String {
    serde_json::to_string(&CanaryCommand {
        program: "cargo".to_owned(),
        args: vec!["test".to_owned()],
        env,
        cwd: ".".to_owned(),
        platform: "any".to_owned(),
    })
    .unwrap()
}

fn write_scratch_contract(root: &Path) {
    fs::create_dir_all(root.join("docs/plans")).unwrap();
    fs::create_dir_all(root.join("docs/testing")).unwrap();
    fs::create_dir_all(root.join("crates/demo/tests")).unwrap();
    fs::write(
        root.join("docs/plans/releases.toml"),
        "[[release]]\nversion = \"0.64.0\"\nfile = \"docs/plans/V0_64_RAFT_SNAPSHOT_AND_AGENTIC_DEBUGGING_TEST_EXPANSION_PLAN.md\"\nstatus = \"planned\"\nwork_items = [\"W1\"]\n",
    )
    .unwrap();
    fs::write(
        root.join("docs/plans/V0_64_RAFT_SNAPSHOT_AND_AGENTIC_DEBUGGING_TEST_EXPANSION_PLAN.md"),
        "## W1. Proof\n",
    )
    .unwrap();
    fs::write(
        root.join("crates/demo/tests/proof.rs"),
        "fn guard_test() {}\nfn canary_test() {}\n",
    )
    .unwrap();
}

fn workspace_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    while !dir.join("docs/plans/releases.toml").is_file() {
        dir = dir.parent().unwrap().to_path_buf();
    }
    dir
}

fn scratch_root() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let counter = SCRATCH_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "hydracache_canary_check_{}_{nanos}_{counter}",
        std::process::id()
    ))
}

fn cleanup(root: &Path) {
    let _ = fs::remove_dir_all(root);
}
