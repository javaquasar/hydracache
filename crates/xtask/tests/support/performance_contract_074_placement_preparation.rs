//! The new observer wraps, but never rewrites, the old complete fixture.
#[path = "performance_contract_074_placement_stage_cost.rs"]
mod cost;
const BASE: &str = "ddbfe9ed";
const PREFIX: &str = "tools/long-run-supervisor-074/src/";
const FILE: &str = "tools/long-run-supervisor-074/src/diagnostic_owned_placement_live_tests.rs";
const HOOK: &str =
    "\n#[path = \"diagnostic_placement_preparation_trace.rs\"]\nmod preparation_trace;\n";
const WRAPPER: &str = "fn run(case: &str) {\n    preparation_trace::capture(case, || run_original(case));\n}\nfn run_original(case: &str) {";
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let text = cost::normalize(path, raw);
    let raw = text.as_str();
    if path != FILE {
        return raw.to_owned();
    }
    assert_eq!(raw.matches(HOOK).count(), 1, "exact preparation module");
    assert_eq!(raw.matches(WRAPPER).count(), 1, "exact preparation wrapper");
    raw.replacen(HOOK, "", 1)
        .replacen(WRAPPER, "fn run(case: &str) {", 1)
}
fn root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path))
        .unwrap()
        .replace('\r', "")
}
fn git(args: &[&str]) -> String {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(root())
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().replace('\r', "")
}
#[test]
fn placement_preparation_preserves_all_430_sources_wire_registry_and_harness() {
    let files = git(&[
        "ls-tree",
        "-r",
        "--name-only",
        BASE,
        "--",
        "tools/long-run-supervisor-074/src",
    ]);
    assert_eq!(files.lines().count(), 430);
    for path in files.lines().chain([
        "docs/COMPAT.md",
        "docs/testing/gated-test-registry.toml",
        "tools/long-run-supervisor-074/tests/owned_root_creator_sigkill.py",
    ]) {
        assert_eq!(
            normalize(path, &read(path)),
            git(&["show", &format!("{BASE}:{path}")]),
            "{path}"
        );
    }
}
#[test]
fn placement_preparation_adapter_rejects_missing_duplicate_and_old_body_drift() {
    let raw = read(FILE);
    for exact in [HOOK, WRAPPER] {
        for bad in [raw.replacen(exact, "", 1), format!("{raw}{exact}")] {
            assert!(std::panic::catch_unwind(|| normalize(FILE, &bad)).is_err());
        }
    }
    let bad = raw.replacen("original_bounds=true", "original_bounds=false", 1);
    assert_ne!(
        normalize(FILE, &bad),
        git(&["show", &format!("{BASE}:{FILE}")])
    );
}
#[test]
fn placement_preparation_trace_is_test_only_and_never_a_guard_or_clock() {
    let raw = read(&format!(
        "{PREFIX}diagnostic_placement_preparation_trace.rs"
    ));
    let observer = raw.split("#[cfg(test)]").next().unwrap();
    for required in [
        "time::closing::capture",
        "time::capture",
        "catch_unwind(AssertUnwindSafe(call))",
        "resume_unwind(payload)",
        "\"new_clock\":false",
        "\"deadline_changed\":false",
        "HC074-PLACEMENT-PREPARATION",
    ] {
        assert!(observer.contains(required), "{required}");
    }
    for forbidden in [
        "unsafe",
        "Instant",
        "clock_gettime",
        "clock(",
        "sample(",
        "sleep(",
        "Command::",
        "File::",
        "sendmsg",
        "enroll_once",
        "revalidate(",
        "issue_progress_challenge",
    ] {
        assert!(!observer.contains(forbidden), "{forbidden}");
    }
    for required in [
        "calls.get(), 1",
        "Arc::ptr_eq",
        "point.boot_id()",
        "10_000_000_005",
        "backward-boottime",
    ] {
        assert!(raw.contains(required), "{required}");
    }
}
