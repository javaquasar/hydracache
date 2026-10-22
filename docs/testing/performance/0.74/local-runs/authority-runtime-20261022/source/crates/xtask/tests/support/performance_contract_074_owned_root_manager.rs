//! Exact additive owned-root adapter, never a predicate or deadline waiver.
#[path = "performance_contract_074_closing_validation.rs"]
mod closing;
const BASE: &str = "28bbba4a";
const PREFIX: &str = "tools/long-run-supervisor-074/src/";
const HOOKS: &[(&str, &str)] = &[
    (
        "diagnostic_original_root_guardians.rs",
        r#"
#[path = "diagnostic_owned_root_manager.rs"]
mod owned;
"#,
    ),
    (
        "diagnostic_root_enrollment.rs",
        r#"
#[cfg(all(test, target_os = "linux"))]
#[path = "diagnostic_privileged_fixture_origin.rs"]
pub(crate) mod privileged_fixture;
"#,
    ),
    (
        "diagnostic_authority_fixed.rs",
        r#"
#[cfg(test)]
#[path = "diagnostic_privileged_authority_fixture.rs"]
pub(crate) mod privileged_fixture;
"#,
    ),
    (
        "diagnostic_intent_fixed_storage.rs",
        r#"
#[cfg(test)]
#[path = "diagnostic_privileged_fence_fixture.rs"]
pub(crate) mod privileged_fixture;
"#,
    ),
    (
        "diagnostic_original_enrollment_frame.rs",
        r#"#[cfg(test)]
#[path = "diagnostic_privileged_enrollment_fixture.rs"]
mod privileged_fixture;
"#,
    ),
    (
        "diagnostic_worker_trust_input_tests.rs",
        r#"#[path = "diagnostic_privileged_worker_fixture.rs"]
pub(crate) mod privileged_fixture;
"#,
    ),
    (
        "diagnostic_controller_trust_input_tests.rs",
        r#"#[path = "diagnostic_privileged_controller_fixture.rs"]
pub(crate) mod privileged_fixture;
"#,
    ),
    (
        "diagnostic_collector_starting_publication_tests.rs",
        r#"
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "diagnostic_privileged_root_frame_tests.rs"]
mod privileged_root;
"#,
    ),
    (
        "diagnostic_root_guardian_original_kernel.rs",
        r#"
#[cfg(test)]
impl BoundGuardians<'_> {
    pub(crate) fn owned_fixture_pins(&self) -> [File; 2] {
        self.pair
            .guards
            .each_ref()
            .map(|guard| guard.pin.try_clone().unwrap())
    }
}
"#,
    ),
];
const GATES: &str = r#"
# BEGIN OWNED ROOT MANAGER GATES

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.privileged-owned-root-screen"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_privileged_root_frame_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "privileged_original_root_manager_writer_screen"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "manual local privileged original manager/root/dummy writers; no product or original send"
tier = "manual"
required_features = []
required_env = ["HC074_OWNED_ROOT_FIXTURE", "HC074_START_MATERIAL_BUNDLE"]
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","privileged_original_root_manager_writer_screen","--","--ignored","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.privileged-owned-root-matrix"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_privileged_root_frame_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "privileged_original_root_manager_writer_matrix"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "manual local privileged original manager/root/dummy writers; no product or original send"
tier = "manual"
required_features = []
required_env = ["HC074_OWNED_ROOT_FIXTURE", "HC074_START_MATERIAL_BUNDLE"]
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","privileged_original_root_manager_writer_matrix","--","--ignored","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]


[[gate]]
id = "tool.hydracache-long-run-supervisor-074.privileged-owned-creator"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_privileged_root_frame_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "privileged_original_root_manager_creator_victim"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "external owning local root original manager creator SIGKILL; no product or send"
tier = "manual"
required_features = []
required_env = ["HC074_OWNED_ROOT_FIXTURE", "HC074_START_MATERIAL_BUNDLE", "HC074_OWNED_CREATOR_BINARY", "HC074_OWNED_CREATOR_OUT"]
required_tools = ["python3"]
timeout_seconds = 1200
owner_release = "0.74.0"
ship_mandatory = false
artifacts = []

[gate.ci]
workflow = ".github/workflows/ci.yml"
job = "gated-proof-registry"
step = "Run registered gated proofs"

[gate.command]
program = "python3"
args = ["-B","tools/long-run-supervisor-074/tests/owned_root_creator_sigkill.py","--binary-env","HC074_OWNED_CREATOR_BINARY","--out-env","HC074_OWNED_CREATOR_OUT"]
cwd = "."
platform = "linux"

[gate.command.env]

# END OWNED ROOT MANAGER GATES
"#;
const COMPAT: &str = r#"
## Local owned original manager creator fixture carrier

The private local-only hc074-owned-root-creator-v1 JSON carrier is bounded below
16384 bytes and uses an unnamed Unix seqpacket pair. The preparing phase carries
no rights and inventories six root-owned fresh temporary scopes, a relay scope,
the original fence path and fresh cgroup device/inode. The ready phase repeats
these fields, adds four original origins/bounds and transfers exactly thirteen
CLOEXEC rights: stdout/stderr readers, two guardian pidfds, collector/custodian
pidfds, the controller relay pidfd, three dummy writer pidfds and three held
stdin writers (preventing natural EOF exit). Only the outside-alias negative
adds a fourteenth stdout writer. Exact creator kernel
credentials, fields, version, phase, rights and truncation are checked; unknown
values refuse. No migration, persisted replay or production reader exists.
This ignored fixture carrier is not original delivery, host enrollment,
completion, unlock or a new guardian protocol.
"#;
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let raw = closing::normalize(path, raw);
    let raw = raw.as_str();
    let mut text = raw.to_owned();
    for (file, hook) in HOOKS {
        if path == format!("{PREFIX}{file}") {
            assert_eq!(
                text.matches(hook).count(),
                1,
                "owned-root exact hook: {path}"
            );
            text = text.replacen(hook, "", 1);
        }
    }
    if path == "docs/testing/gated-test-registry.toml" {
        assert_eq!(text.matches(GATES).count(), 1);
        text = text.replacen(GATES, "", 1);
    }
    if path == "docs/COMPAT.md" {
        assert_eq!(text.matches(COMPAT).count(), 1);
        text = text.replacen(COMPAT, "", 1);
    }
    text
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
    let result = std::process::Command::new("git")
        .args(args)
        .current_dir(root())
        .output()
        .unwrap();
    assert!(result.status.success());
    String::from_utf8(result.stdout).unwrap().replace('\r', "")
}
#[test]
fn owned_root_manager_preserves_every_previous_runtime_and_fixture_body() {
    let files = git(&[
        "ls-tree",
        "-r",
        "--name-only",
        BASE,
        "--",
        "tools/long-run-supervisor-074/src",
    ]);
    assert_eq!(files.lines().count(), 406);
    for path in files.lines().chain([
        "docs/COMPAT.md",
        "docs/testing/gated-test-registry.toml",
        "tools/long-run-supervisor-074/tests/root_creator_sigkill.py",
    ]) {
        assert_eq!(
            normalize(path, &read(path)),
            git(&["show", &format!("{BASE}:{path}")]),
            "{path}"
        );
    }
}
#[test]
fn owned_root_manager_additive_adapter_rejects_missing_duplicate_and_body_drift() {
    for (file, hook) in HOOKS {
        let path = format!("{PREFIX}{file}");
        let current = read(&path);
        for bad in [current.replacen(hook, "", 1), format!("{current}{hook}")] {
            assert!(std::panic::catch_unwind(|| normalize(&path, &bad)).is_err());
        }
        let changed = current.replace("use super::*;", "use super::*; /* body drift */");
        if changed != current {
            assert_ne!(
                normalize(&path, &changed),
                git(&["show", &format!("{BASE}:{path}")])
            );
        }
    }
    let registry = read("docs/testing/gated-test-registry.toml");
    for bad in [
        registry.replacen(GATES, "", 1),
        format!("{registry}{GATES}"),
    ] {
        assert!(std::panic::catch_unwind(|| normalize(
            "docs/testing/gated-test-registry.toml",
            &bad
        ))
        .is_err());
    }
}
#[test]
fn owned_root_manager_consumes_alias_owner_without_new_clock_or_execution_capability() {
    let raw = read(&format!("{PREFIX}diagnostic_owned_root_manager.rs"));
    for needed in [
        "original: OriginalManagerMessage<",
        "owner: OwnedOwner<",
        "controls: BoundGuardians<",
        "watchdog_context_round(Some(binding))",
        "watchdog_context_round(None)",
        "prepare(&mut owner, root)",
        "join(&mut self.owner, &mut self.controls, true)",
        "owner.refuse();",
        "held: Box::new(owner)",
        "retire(&mut self.owner, &mut self.controls)",
    ] {
        assert!(raw.contains(needed), "{needed}");
    }
    assert!(
        raw.find("owner: OwnedOwner<").unwrap() < raw.find("controls: BoundGuardians<").unwrap()
    );
    for forbidden in [
        "timerfd",
        "clock_gettime",
        "LOCK_UN",
        "unsafe",
        "Command::",
        "pub fn",
        "send_message",
        "from_raw_fd",
    ] {
        assert!(!raw.contains(forbidden), "{forbidden}");
    }
    let origin = read(&format!("{PREFIX}diagnostic_privileged_fixture_origin.rs"));
    for needed in [
        "local-only-v1",
        "hc074-owned-root-",
        "geteuid()",
        "getegid()",
        "canonicalize",
    ] {
        assert!(origin.contains(needed));
    }
}
