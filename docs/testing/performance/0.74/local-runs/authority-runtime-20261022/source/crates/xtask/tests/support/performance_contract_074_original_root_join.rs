//! Exact forward adapter for original root frame joins.
#[path = "performance_contract_074_root_retirement.rs"]
mod retirement;
const BASE: &str = "52d805da";
const ADDITIONS: &[(&str, &str)] = &[
    (
        "tools/long-run-supervisor-074/src/diagnostic_root_creator_tests.rs",
        "                super::original::queued_ack_for_fixture(guard.peer.as_raw_fd(), &pair.frame.enroll);\n",
    ),
    (
        "tools/long-run-supervisor-074/src/diagnostic_root_guardian.rs",
        r####"
#[path = "diagnostic_root_guardian_original_kernel.rs"]
pub(crate) mod original;
"####,
    ),
    (
        "tools/long-run-supervisor-074/src/diagnostic_cgroup_root.rs",
        r####"
pub(crate) use guardian::original::{BoundGuardians, RootSource};
"####,
    ),
    (
        "tools/long-run-supervisor-074/src/diagnostic_root_enrollment.rs",
        r####"
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(crate) use kernel::{BoundGuardians, RootSource};
"####,
    ),
    (
        "tools/long-run-supervisor-074/src/diagnostic_original_watchdog_context.rs",
        r####"
#[path = "diagnostic_original_root_guardians.rs"]
pub(in super::super) mod root_guardians;
"####,
    ),
    (
        "tools/long-run-supervisor-074/src/diagnostic_original_watchdog_context_tests.rs",
        r####"
#[path = "diagnostic_original_root_guardian_tests.rs"]
mod root_guardians;
"####,
    ),
    (
        "tools/long-run-supervisor-074/src/diagnostic_original_pending_watchdog_test_api.rs",
        r####"
#[path = "diagnostic_original_root_guardian_test_api.rs"]
mod root_guardians;
"####,
    ),
    (
        "tools/long-run-supervisor-074/src/diagnostic_original_pending_watchdog_frame_tests.rs",
        r####"
#[path = "diagnostic_original_root_guardian_frame_tests.rs"]
mod root_guardians;
"####,
    ),
    (
        "tools/long-run-supervisor-074/src/diagnostic_root_guardian_fault_tests.rs",
        r####"
#[path = "diagnostic_root_guardian_original_kernel_fault_tests.rs"]
mod original_adapter;
"####,
    ),
    (
        "docs/COMPAT.md",
        r####"
## Original frame binding for local root guardians

The internal HCRG0742 context digest uses the domain
hc074-original-root-context-v1 and a serde JSON tuple of actual owner IDs,
original state, origins, bounds, ten descriptor roles, collector/custodian
pidfd roles, fixed manager/client names, original serial, addressed writer
aliases and pinned root device/inode. The attempt digest uses
hc074-original-root-attempt-v1 and the original diagnostic identity.
These are opaque same-build bindings, not credentials or a new trust reader.
No persisted migration or cross-build replay is admitted.

The exclusive original context supplies its own fence and existing timer;
callers cannot provide a replacement deadline, context digest or owner PID.
Preparation and enrollment retain the unchanged complete validation rounds.
The crate-private pinned root verifies kernel objects but does not enroll a
production host or authorize a held unit. Local Armed still grants no execution,
completion or fence release. Existing v1 predicates and all source deadlines
remain unchanged. Root kernel controls and nonroot original frame controls are
separate gates, not proof of an end-to-end original privileged manager channel.
"####,
    ),
    (
        "docs/testing/gated-test-registry.toml",
        r####"
# BEGIN ORIGINAL ROOT FRAME GATE

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-root-frame-controls"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_root_guardian_frame_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_root_guardian_frame_controls"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "manual nonroot original frame with simulated root kernel; no original send"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","original_root_guardian_frame_controls","--","--ignored","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

# END ORIGINAL ROOT FRAME GATE

# BEGIN ROOT JOIN KERNEL GATE

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.root-join-kernel-controls"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_root_guardian_original_kernel_fault_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "root_guardian_original_kernel_controls"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "manual local root kernel adapter; no original manager channel or product"
tier = "manual"
required_features = []
required_env = ["HC074_ROOT_JOIN_FIXTURE"]
required_tools = ["cargo"]
timeout_seconds = 180
owner_release = "0.74.0"
ship_mandatory = false
artifacts = []

[gate.ci]
workflow = ".github/workflows/ci.yml"
job = "gated-proof-registry"
step = "Run registered gated proofs"

[gate.command]
program = "cargo"
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","root_guardian_original_kernel_controls","--","--ignored","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

# END ROOT JOIN KERNEL GATE
"####,
    ),
];
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let current = retirement::normalize(path, raw);
    let raw = current.as_str();
    let Some((_, extra)) = ADDITIONS.iter().find(|(p, _)| *p == path) else {
        return raw.to_owned();
    };
    assert_eq!(
        raw.matches(extra).count(),
        1,
        "original root join addition: {path}"
    );
    raw.replacen(extra, "", 1)
}
fn root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
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
fn original_root_join_preserves_every_previous_source_and_carrier() {
    let listing = git(&[
        "ls-tree",
        "-r",
        "--name-only",
        BASE,
        "--",
        "tools/long-run-supervisor-074/src",
    ]);
    let files: Vec<_> = listing.lines().filter(|p| p.ends_with(".rs")).collect();
    assert_eq!(files.len(), 394);
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
fn original_root_join_requires_full_rounds_original_roles_and_closed_execution() {
    let source = read("tools/long-run-supervisor-074/src/diagnostic_original_root_guardians.rs");
    for item in [
        "original_descriptors(",
        "source.bounds[3]",
        "source.owner",
        "source.origins",
        "source.descriptors",
        "source.guardians",
        "source.serial.get()",
        "binding.addressed",
        "owner.full_round()?",
        "controls.enroll_once()?",
        "owner.refuse()",
        "controls.refuse()",
        "held: Box::new(self)",
        "prepare(&mut self, root)",
    ] {
        assert!(source.contains(item), "{item}");
    }
    for item in [
        "timerfd_settime",
        "timerfd_create",
        "LOCK_UN",
        "StartTransientUnit",
        "pub fn ",
        "pub(crate) fn descriptors",
    ] {
        assert!(!source.contains(item), "{item}");
    }
    let contract: toml::Value = toml::from_str(&read(
        "docs/testing/performance/0.74/original-root-join-contract.toml",
    ))
    .unwrap();
    for key in [
        "root_pin_is_host_admission",
        "end_to_end_privileged_original_frame_tested",
        "original_armed_execution_capability",
        "whole_writer_cessation_proved",
        "completion_allowed",
        "fence_release_allowed",
        "server_operation_allowed",
        "product_allowed",
        "qualification_allowed",
    ] {
        assert_eq!(contract[key].as_bool(), Some(false), "{key}");
    }
}
#[test]
fn original_root_join_exact_adapter_refuses_missing_duplicate_and_body_mutation() {
    for (path, extra) in ADDITIONS {
        let raw = read(path);
        assert!(std::panic::catch_unwind(|| normalize(path, &raw.replacen(extra, "", 1))).is_err());
        assert!(std::panic::catch_unwind(|| normalize(path, &format!("{raw}{extra}"))).is_err());
        assert_ne!(
            normalize(path, &format!("{raw}/* changed old body */")),
            git(&["show", &format!("{BASE}:{path}")])
        );
    }
}
