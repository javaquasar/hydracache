//! Tests for the release-manifest consistency checker (`xtask doc-check`).

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use xtask::doc_check;

/// Create a throwaway repo root under the system temp dir with the given
/// `releases.toml` body and (optionally) referenced plan files.
fn scratch_root(manifest: &str, plan_files: &[&str]) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("hydracache_doc_check_{nanos}"));
    fs::create_dir_all(root.join("docs/plans")).unwrap();
    fs::write(root.join("docs/plans/releases.toml"), manifest).unwrap();
    for file in plan_files {
        let path = root.join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "# plan\n").unwrap();
    }
    root
}

fn cleanup(root: &Path) {
    let _ = fs::remove_dir_all(root);
}

#[test]
fn consistent_manifest_has_no_problems() {
    let manifest = r#"
[[release]]
version = "0.37.0"
file = "docs/plans/A.md"
status = "shipped"
depends_on = []

[[release]]
version = "0.38.0"
file = "docs/plans/B.md"
status = "planned"
depends_on = ["0.37.0"]

[[release]]
version = "TBD"
file = "docs/plans/DRAFT.md"
status = "draft"
depends_on = ["0.38.0"]
"#;
    let root = scratch_root(
        manifest,
        &["docs/plans/A.md", "docs/plans/B.md", "docs/plans/DRAFT.md"],
    );
    let problems = doc_check::check(&root).unwrap();
    cleanup(&root);
    assert!(
        problems.is_empty(),
        "expected no problems, got: {problems:?}"
    );
}

#[test]
fn detects_duplicate_version_missing_file_bad_status_and_dangling_dep() {
    let manifest = r#"
[[release]]
version = "0.40.0"
file = "docs/plans/A.md"
status = "planned"
depends_on = ["9.9.9"]

[[release]]
version = "0.40.0"
file = "docs/plans/missing.md"
status = "weird"
depends_on = []

[[release]]
version = "TBD"
file = "docs/plans/A.md"
status = "planned"
depends_on = []
"#;
    // Only A.md exists; missing.md does not.
    let root = scratch_root(manifest, &["docs/plans/A.md"]);
    let problems = doc_check::check(&root).unwrap();
    cleanup(&root);

    let joined = problems.join("\n");
    assert!(
        joined.contains("duplicate version '0.40.0'"),
        "missing dup check: {joined}"
    );
    assert!(
        joined.contains("file does not exist"),
        "missing file-existence check: {joined}"
    );
    assert!(
        joined.contains("invalid status 'weird'"),
        "missing status check: {joined}"
    );
    assert!(
        joined.contains("depends_on '9.9.9'"),
        "missing dangling-dep check: {joined}"
    );
    assert!(
        joined.contains("version 'TBD' is only allowed"),
        "missing TBD-on-non-draft check: {joined}"
    );
}

#[test]
fn detects_shipped_043_without_networked_control_plane_sentinel() {
    let manifest = r#"
[[release]]
version = "0.43.0"
file = "docs/plans/V0_43.md"
status = "shipped"
depends_on = []
"#;
    let root = scratch_root(manifest, &["docs/plans/V0_43.md"]);
    let problems = doc_check::check(&root).unwrap();
    cleanup(&root);

    let joined = problems.join("\n");
    assert!(
        joined.contains("shipped 0.43.0 must set networked_control_plane = true"),
        "missing 0.43 sentinel check: {joined}"
    );
}

#[test]
fn detects_shipped_release_with_false_networked_control_plane_sentinel() {
    let manifest = r#"
[[release]]
version = "0.43.0"
file = "docs/plans/V0_43.md"
status = "shipped"
networked_control_plane = false
depends_on = []
"#;
    let root = scratch_root(manifest, &["docs/plans/V0_43.md"]);
    let problems = doc_check::check(&root).unwrap();
    cleanup(&root);

    let joined = problems.join("\n");
    assert!(
        joined.contains("shipped release cannot set networked_control_plane = false"),
        "missing false-sentinel check: {joined}"
    );
}

#[test]
fn detects_dangling_in_prose_plan_links() {
    let manifest = r#"
[[release]]
version = "0.50.0"
file = "docs/plans/V0_50_EXISTING_PLAN.md"
status = "planned"
depends_on = []
"#;
    let root = scratch_root(manifest, &["docs/plans/V0_50_EXISTING_PLAN.md"]);
    fs::write(
        root.join("docs/plans/V0_50_EXISTING_PLAN.md"),
        "See `V0_44_DETERMINISTIC_SIMULATION_TESTING_PLAN.md` and `V0_99_MISSING_PLAN.md`.\n",
    )
    .unwrap();
    fs::write(
        root.join("docs/plans/V0_44_DETERMINISTIC_SIMULATION_TESTING_PLAN.md"),
        "# existing plan\n",
    )
    .unwrap();

    let problems = doc_check::check(&root).unwrap();
    cleanup(&root);

    let joined = problems.join("\n");
    assert!(
        joined.contains("references missing plan 'V0_99_MISSING_PLAN.md'"),
        "missing in-prose plan-link check: {joined}"
    );
    assert!(
        !joined.contains("V0_44_DETERMINISTIC_SIMULATION_TESTING_PLAN.md"),
        "existing in-prose plan link should not fail: {joined}"
    );
}
