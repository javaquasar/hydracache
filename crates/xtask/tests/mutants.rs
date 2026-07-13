use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use xtask::mutants::check_mutation_baseline;

static SCRATCH_COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn mutants_baseline_has_no_untriaged_survivors_in_snapshot_and_membership_paths() {
    let root = workspace_root();

    check_mutation_baseline(&root).unwrap();
}

#[test]
fn canary_mutants_baseline_hides_a_live_survivor() {
    let root = scratch_root();
    write_required_files(&root);
    fs::create_dir_all(root.join("target/hydracache-mutants")).unwrap();
    fs::write(
        root.join("target/hydracache-mutants/report.txt"),
        "SURVIVED crates/hydracache-cluster-raft/src/log_store.rs: dropped durable snapshot checksum assertion\n",
    )
    .unwrap();

    let error = check_mutation_baseline(&root).unwrap_err();
    cleanup(&root);

    assert!(
        error.contains("untriaged mutation survivor"),
        "unexpected error: {error}"
    );
}

#[test]
fn triaged_survivor_in_baseline_is_accepted() {
    let root = scratch_root();
    write_required_files(&root);
    let survivor =
        "SURVIVED crates/hydracache-cluster-raft/src/log_store.rs: equivalent formatting mutant";
    fs::write(
        root.join("docs/testing/mutation-baseline.md"),
        format!(
            "# Raft Mutation Baseline\n\n## Scope\n\n- crates/hydracache-cluster-raft/src/lib.rs\n- crates/hydracache-cluster-raft/src/log_store.rs\n\n## Allowed Survivors\n\n- {survivor}\n"
        ),
    )
    .unwrap();
    fs::create_dir_all(root.join("target/hydracache-mutants")).unwrap();
    fs::write(
        root.join("target/hydracache-mutants/report.txt"),
        format!("{survivor}\n"),
    )
    .unwrap();

    check_mutation_baseline(&root).unwrap();
    cleanup(&root);
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
        "hydracache_mutants_{}_{nanos}_{counter}",
        std::process::id()
    ))
}

fn write_required_files(root: &Path) {
    fs::create_dir_all(root.join(".cargo")).unwrap();
    fs::create_dir_all(root.join("docs/testing")).unwrap();
    fs::write(
        root.join(".cargo/mutants.toml"),
        r#"
[hydracache]
scope = [
  "crates/hydracache-cluster-raft/src/lib.rs",
  "crates/hydracache-cluster-raft/src/log_store.rs",
]
required_tests = [
  "cargo test -p hydracache-cluster-raft snapshot_immutability --locked",
  "cargo test -p hydracache-cluster-raft --test raft_snapshot_membership --locked",
  "cargo test -p hydracache-cluster-raft --features test-failpoints snapshot_apply --locked -- --test-threads=1",
  "cargo test -p hydracache-cluster-raft --test rejoin_after_compaction --features test-failpoints --locked -- --test-threads=1",
  "cargo test -p hydracache-cluster-raft --test proposal_idempotency --locked",
]
"#,
    )
    .unwrap();
    fs::write(
        root.join("docs/testing/mutation-baseline.md"),
        "# Raft Mutation Baseline\n\n## Scope\n\n- crates/hydracache-cluster-raft/src/lib.rs\n- crates/hydracache-cluster-raft/src/log_store.rs\n\n## Allowed Survivors\n\nNo allowed survivors.\n",
    )
    .unwrap();
}

fn cleanup(root: &Path) {
    let _ = fs::remove_dir_all(root);
}
