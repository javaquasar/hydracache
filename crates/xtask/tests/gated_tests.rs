use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use xtask::gated_tests::check_registry;

static SCRATCH_COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn gated_test_registry_covers_every_ignored_cfg_and_env_gated_test() {
    let problems = check_registry(&workspace_root()).unwrap();
    assert!(problems.is_empty(), "registry problems: {problems:#?}");
}

#[test]
fn registry_rejects_missing_command_ci_tier_owner_or_timeout() {
    let root = scratch_root();
    write_workspace(&root);
    write_registry(
        &root,
        r#"
schema_version = 1
release = "0.64.0"

[[gate]]
id = "ignored-demo"
kind = "ignored_test"
source = "demo/tests/gated.rs"
package = "demo"
target = "gated"
test = "ignored_case"
reason = ""
tier = "nightly"
timeout_seconds = 0
owner_release = ""
ship_mandatory = true
ci = { workflow = "", job = "", step = "" }
command = { program = "", args = [], cwd = "", platform = "" }
"#,
    );

    let problems = check_registry(&root).unwrap();
    cleanup(&root);
    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W33") {
        assert!(
            problems.is_empty(),
            "HC-CANARY-RED:W33 incomplete gated-test registration was accepted"
        );
    }
    for field in [
        "reason",
        "owner_release",
        "ci.workflow",
        "ci.job",
        "ci.step",
        "command.program",
        "timeout_seconds",
    ] {
        assert!(
            problems.iter().any(|problem| problem.contains(field)),
            "missing problem for {field}: {problems:#?}"
        );
    }
}

#[test]
fn registry_rejects_stale_entries_that_no_longer_resolve_to_a_test() {
    let root = scratch_root();
    write_workspace(&root);
    write_registry(
        &root,
        &valid_registry().replace("ignored_case", "removed_case"),
    );

    let problems = check_registry(&root).unwrap();
    cleanup(&root);
    assert!(
        problems
            .iter()
            .any(|problem| problem.contains("stale gate")),
        "stale entry must fail: {problems:#?}"
    );
}

#[test]
fn wildcard_registration_does_not_hide_a_new_ignored_test() {
    let root = scratch_root();
    write_workspace(&root);
    write_registry(&root, &valid_registry().replace("ignored_case", "*"));

    let problems = check_registry(&root).unwrap();
    cleanup(&root);
    assert!(
        problems
            .iter()
            .any(|problem| problem.contains("unregistered ignored test")),
        "a wildcard must not auto-register future ignored tests: {problems:#?}"
    );
}

fn valid_registry() -> String {
    r#"
schema_version = 1
release = "0.64.0"

[[gate]]
id = "ignored-demo"
kind = "ignored_test"
source = "demo/tests/gated.rs"
package = "demo"
target = "gated"
test = "ignored_case"
reason = "nightly fixture"
tier = "nightly"
timeout_seconds = 60
owner_release = "0.64.0"
ship_mandatory = true
ci = { workflow = ".github/workflows/ci.yml", job = "gated", step = "Run" }
command = { program = "cargo", args = ["test"], cwd = ".", platform = "any" }

[[gate]]
id = "cfg-demo"
kind = "cfg_test_target"
source = "demo/tests/gated.rs"
package = "demo"
target = "gated"
cfg = 'feature = "gated"'
reason = "feature-gated fixture"
tier = "nightly"
required_features = ["gated"]
timeout_seconds = 60
owner_release = "0.64.0"
ship_mandatory = true
ci = { workflow = ".github/workflows/ci.yml", job = "gated", step = "Run" }
command = { program = "cargo", args = ["test"], cwd = ".", platform = "any" }

[[gate]]
id = "env-demo"
kind = "env_gate"
source = "demo/tests/gated.rs"
package = "demo"
target = "gated"
env = "HYDRACACHE_RUN_DEMO"
reason = "environment-gated fixture"
tier = "nightly"
required_env = ["HYDRACACHE_RUN_DEMO"]
timeout_seconds = 60
owner_release = "0.64.0"
ship_mandatory = true
ci = { workflow = ".github/workflows/ci.yml", job = "gated", step = "Run" }
command = { program = "cargo", args = ["test"], cwd = ".", platform = "any" }
"#
    .to_owned()
}

fn write_workspace(root: &Path) {
    fs::create_dir_all(root.join("demo/src")).unwrap();
    fs::create_dir_all(root.join("demo/tests")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"demo\"]\nresolver = \"2\"\n",
    )
    .unwrap();
    fs::write(
        root.join("demo/Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[features]\ngated = []\n",
    )
    .unwrap();
    fs::write(root.join("demo/src/lib.rs"), "pub fn demo() {}\n").unwrap();
    fs::write(
        root.join("demo/tests/gated.rs"),
        r#"#![cfg(feature = "gated")]

const RUN_ENV: &str = "HYDRACACHE_RUN_DEMO";

#[test]
#[ignore = "nightly"]
fn ignored_case() {
    let _ = std::env::var(RUN_ENV);
}
"#,
    )
    .unwrap();
}

fn write_registry(root: &Path, text: &str) {
    fs::create_dir_all(root.join("docs/testing")).unwrap();
    fs::write(root.join("docs/testing/gated-test-registry.toml"), text).unwrap();
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
        "hydracache_gated_tests_{}_{nanos}_{counter}",
        std::process::id()
    ))
}

fn cleanup(root: &Path) {
    let _ = fs::remove_dir_all(root);
}
