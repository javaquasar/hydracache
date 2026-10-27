//! Retirement must fail closed before returning the original exclusive borrow.
#[path = "performance_contract_074_gnu_preparation.rs"]
mod preparation;
fn root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
#[test]
fn original_root_retirement_must_poison_on_drop() {
    let source = std::fs::read_to_string(
        root().join("tools/long-run-supervisor-074/src/diagnostic_original_root_guardians.rs"),
    )
    .unwrap();
    assert!(
        source.contains("impl Drop for OriginalRootGuardians"),
        "missing original owner Drop refusal"
    );
    assert!(source.contains("retire(&mut self.original, &mut self.controls)"));
}

const BASE: &str = "f48fd4bf";
const ADDITIONS: &[(&str, &str)] = &[
    (
        "tools/long-run-supervisor-074/src/diagnostic_original_root_guardians.rs",
        r####"
pub(in super::super::super) fn retire(
    owner: &mut impl RootOwner,
    controls: &mut impl RootControls,
) {
    // Returning the exclusive borrow must never reopen this original attempt.
    // Refusal precedes field Drop; active kernel holders still revoke and fence.
    owner.refuse();
    controls.refuse();
}
impl Drop for OriginalRootGuardians<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn drop(&mut self) {
        retire(&mut self.original, &mut self.controls);
    }
}

#[cfg(test)]
#[path = "diagnostic_original_root_retirement_tests.rs"]
mod retirement_tests;
"####,
    ),
    (
        "tools/long-run-supervisor-074/src/diagnostic_original_root_guardian_test_api.rs",
        r####"
impl PendingProbe<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn root_retire_for_test(&mut self, controls: &mut RootProbeControls) {
        root_guardians::retire(self, controls);
    }
}
"####,
    ),
    (
        "tools/long-run-supervisor-074/src/diagnostic_original_root_guardian_frame_tests.rs",
        r####"
#[path = "diagnostic_original_root_retirement_frame_tests.rs"]
mod retirement;
"####,
    ),
    (
        "docs/testing/gated-test-registry.toml",
        r####"
# BEGIN ROOT RETIREMENT GATE

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.root-retirement-controls"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_root_retirement_frame_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_root_retirement_frame_control"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "manual nonroot original owner retirement; no original root constructor or send"
tier = "manual"
required_features = []
required_env = ["HC074_START_MATERIAL_BUNDLE"]
required_tools = ["cargo"]
timeout_seconds = 1200
owner_release = "0.74.0"
ship_mandatory = false
artifacts = []

[gate.ci]
workflow = ".github/workflows/ci.yml"
job = "gated-proof-registry"
step = "Run registered gated proofs"

[gate.command]
program = "cargo"
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","original_root_retirement_frame_control","--","--ignored","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

# END ROOT RETIREMENT GATE
"####,
    ),
];
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let raw = &preparation::normalize(path, raw);
    let Some((_, addition)) = ADDITIONS.iter().find(|(p, _)| *p == path) else {
        return raw.to_owned();
    };
    assert_eq!(
        raw.matches(addition).count(),
        1,
        "retirement addition: {path}"
    );
    raw.replacen(addition, "", 1)
}
fn read(path: &str) -> String {
    crate::runtime_io::read_to_string(root().join(path))
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
fn original_root_retirement_preserves_all_previous_sources_and_protocols() {
    let listing = git(&[
        "ls-tree",
        "-r",
        "--name-only",
        BASE,
        "--",
        "tools/long-run-supervisor-074/src",
    ]);
    let files: Vec<_> = listing.lines().filter(|p| p.ends_with(".rs")).collect();
    assert_eq!(files.len(), 400);
    for path in files.into_iter().chain([
        "docs/COMPAT.md",
        "docs/testing/gated-test-registry.toml",
        "tools/long-run-supervisor-074/tests/prearmed_watchdog.py",
        "tools/long-run-supervisor-074/tests/redundant_watchdog.py",
        "tools/long-run-supervisor-074/tests/root_creator_sigkill.py",
    ]) {
        assert!(
            normalize(path, &read(path)) == git(&["show", &format!("{BASE}:{path}")]),
            "{path}"
        );
    }
}
#[test]
fn original_root_retirement_exact_additions_keep_order_and_closed_capabilities() {
    for (path, extra) in ADDITIONS {
        let raw = read(path);
        assert!(std::panic::catch_unwind(|| normalize(path, &raw.replacen(extra, "", 1))).is_err());
        assert!(std::panic::catch_unwind(|| normalize(path, &format!("{raw}{extra}"))).is_err());
        assert_ne!(
            normalize(path, &format!("{raw}/* changed predicate */")),
            git(&["show", &format!("{BASE}:{path}")])
        );
    }
    let raw = read("tools/long-run-supervisor-074/src/diagnostic_original_root_guardians.rs");
    let retirement = raw
        .split("pub(in super::super::super) fn retire(")
        .nth(1)
        .unwrap();
    assert!(
        retirement.find("owner.refuse();").unwrap()
            < retirement.find("controls.refuse();").unwrap()
    );
    for item in [
        "timerfd",
        "CLOCK_",
        "LOCK_UN",
        "cancel_pending",
        "cleanup_for_fixture",
        "StartTransientUnit",
    ] {
        assert!(!retirement.contains(item), "{item}");
    }
    let contract: toml::Value = toml::from_str(&read(
        "docs/testing/performance/0.74/root-retirement-contract.toml",
    ))
    .unwrap();
    for key in [
        "retirement_renews_or_reads_time",
        "active_guardians_cancelled",
        "borrow_return_reopens_attempt",
        "end_to_end_original_constructor_tested",
        "original_armed_execution_capability",
        "completion_allowed",
        "fence_release_allowed",
        "server_operation_allowed",
        "product_allowed",
        "qualification_allowed",
    ] {
        assert_eq!(contract[key].as_bool(), Some(false), "{key}");
    }
}
