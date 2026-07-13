use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use xtask::canary_check::check_canary_registry;

static SCRATCH_COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn every_w_item_has_a_registered_canary_that_references_real_functions() {
    let root = workspace_root();

    let problems = check_canary_registry(&root).unwrap();

    assert!(
        problems.is_empty(),
        "expected canary registry to be valid, got: {problems:?}"
    );
}

#[test]
fn each_canary_makes_its_paired_guard_fail_red() {
    let root = scratch_root();
    write_plan(&root);
    write_function_file(
        &root,
        "crates/demo/tests/proof.rs",
        "fn guard_test() {}\nfn canary_test() {}\n",
    );
    write_registry(
        &root,
        r#"
{
  "version": 1,
  "release": "0.64.0",
  "entries": [
    {
      "w_item": "W1",
      "guard": { "file": "crates/demo/tests/proof.rs", "function": "guard_test" },
      "canary": { "file": "crates/demo/tests/proof.rs", "function": "canary_test" },
      "red_evidence": "fixture canary fails the guard",
      "makes_guard_fail": true
    }
  ]
}
"#,
    );

    let problems = check_canary_registry(&root).unwrap();
    cleanup(&root);

    assert!(
        problems
            .iter()
            .all(|problem| !problem.contains("makes_guard_fail=false")),
        "active canary entry should not be reported inert: {problems:?}"
    );
}

#[test]
fn canary_registry_lists_a_canary_that_does_not_fail_its_guard() {
    let root = scratch_root();
    write_plan(&root);
    write_function_file(
        &root,
        "crates/demo/tests/proof.rs",
        "fn guard_test() {}\nfn canary_test() {}\n",
    );
    write_registry(
        &root,
        r#"
{
  "version": 1,
  "release": "0.64.0",
  "entries": [
    {
      "w_item": "W1",
      "guard": { "file": "crates/demo/tests/proof.rs", "function": "guard_test" },
      "canary": { "file": "crates/demo/tests/proof.rs", "function": "canary_test" },
      "red_evidence": "fixture is inert",
      "makes_guard_fail": false
    }
  ]
}
"#,
    );

    let problems = check_canary_registry(&root).unwrap();
    cleanup(&root);

    assert!(
        problems
            .iter()
            .any(|problem| problem.contains("makes_guard_fail=false")),
        "inert canary registry entry must fail: {problems:?}"
    );
}

fn workspace_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    while !dir.join("docs/plans/releases.toml").is_file() {
        dir = dir
            .parent()
            .expect("workspace root should be above xtask")
            .to_path_buf();
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

fn write_plan(root: &Path) {
    fs::create_dir_all(root.join("docs/plans")).unwrap();
    let mut plan = String::new();
    for item in [
        "W1", "W2", "W3", "W4", "W5", "W7", "W8", "W9", "W10", "W11", "W12", "W13", "W14", "W15",
        "W16", "W17", "W18", "W19", "W20",
    ] {
        plan.push_str(&format!("## {item}. Proof\n\n"));
    }
    fs::write(
        root.join("docs/plans/V0_64_RAFT_SNAPSHOT_AND_AGENTIC_DEBUGGING_TEST_EXPANSION_PLAN.md"),
        plan,
    )
    .unwrap();
}

fn write_function_file(root: &Path, file: &str, text: &str) {
    let path = root.join(file);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn write_registry(root: &Path, text: &str) {
    fs::create_dir_all(root.join("docs/testing")).unwrap();
    fs::write(root.join("docs/testing/canary-registry.json"), text).unwrap();
}

fn cleanup(root: &Path) {
    let _ = fs::remove_dir_all(root);
}
