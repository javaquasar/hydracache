//! Exact old predicates and private borrowed original Pending custody only.
#[path = "performance_contract_074_cgroup_root_enrollment.rs"]
mod root_enrollment;
const BASE: &str = "f94b4b56";
fn addition(path: &str) -> Option<&'static str> {
    Some(match path {
        "tools/long-run-supervisor-074/src/diagnostic_prearmed_watchdog.rs" => "\n#[path = \"diagnostic_inherited_pending_watchdog.rs\"]\npub(super) mod inherited_pending;\n",
        "tools/long-run-supervisor-074/src/diagnostic_held_barrier.rs" => "\npub(crate) use prearmed::inherited_pending::InheritedPending;\n",
        "tools/long-run-supervisor-074/src/diagnostic_two_phase_launch.rs" => "\n#[cfg(all(target_os = \"linux\", target_arch = \"x86_64\"))]\npub(crate) use held_barrier::InheritedPending;\n",
        "tools/long-run-supervisor-074/src/diagnostic_original_watchdog_context.rs" => "\n#[path = \"diagnostic_original_pending_watchdog.rs\"]\npub(in super::super) mod pending;\n",
        "tools/long-run-supervisor-074/src/diagnostic_original_watchdog_context_test_api.rs" => "\n#[path = \"diagnostic_original_pending_watchdog_test_api.rs\"]\nmod pending;\n",
        "tools/long-run-supervisor-074/src/diagnostic_original_watchdog_context_frame_tests.rs" => "\n#[path = \"diagnostic_original_pending_watchdog_frame_tests.rs\"]\nmod pending;\n",
        _ => return None,
    })
}
fn strip_registry(raw: &str) -> String {
    const REGISTRY: &str = r####"
# BEGIN ORIGINAL PENDING WATCHDOG GATE

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-pending-watchdog-controls"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_pending_watchdog_frame_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_pending_watchdog_frame_controls"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "manual nonroot original Pending custody; no target enrollment or delivery"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","original_pending_watchdog_frame_controls","--","--ignored","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

# END ORIGINAL PENDING WATCHDOG GATE
"####;
    assert_eq!(raw.matches(REGISTRY).count(), 1);
    raw.replacen(REGISTRY, "", 1)
}
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let current = root_enrollment::normalize(path, raw);
    let raw = current.as_str();
    if path == "docs/testing/gated-test-registry.toml" {
        return strip_registry(raw);
    }
    let Some(addition) = addition(path) else {
        return raw.to_owned();
    };
    assert_eq!(
        raw.matches(addition).count(),
        1,
        "Pending registration drift: {path}"
    );
    raw.replacen(addition, "", 1)
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
fn original_pending_watchdog_preserves_all_old_sources_protocols_and_helpers() {
    let paths = git(&[
        "ls-tree",
        "-r",
        "--name-only",
        BASE,
        "--",
        "tools/long-run-supervisor-074/src",
    ]);
    let paths: Vec<_> = paths.lines().filter(|p| p.ends_with(".rs")).collect();
    assert_eq!(paths.len(), 376);
    for path in paths.into_iter().chain([
        "docs/COMPAT.md",
        "docs/testing/gated-test-registry.toml",
        "tools/long-run-supervisor-074/tests/prearmed_watchdog.py",
        "tools/long-run-supervisor-074/tests/redundant_watchdog.py",
    ]) {
        assert_eq!(
            normalize(path, &read(path)),
            git(&["show", &format!("{BASE}:{path}")]),
            "{path}"
        );
    }
}
#[test]
fn original_pending_watchdog_adapter_refuses_removed_duplicate_and_old_body_mutations() {
    for path in [
        "tools/long-run-supervisor-074/src/diagnostic_prearmed_watchdog.rs",
        "tools/long-run-supervisor-074/src/diagnostic_held_barrier.rs",
        "tools/long-run-supervisor-074/src/diagnostic_two_phase_launch.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_watchdog_context.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_watchdog_context_test_api.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_watchdog_context_frame_tests.rs",
    ] {
        let raw = read(path);
        let extra = addition(path).unwrap();
        assert!(std::panic::catch_unwind(|| normalize(path, &raw.replacen(extra, "", 1))).is_err());
        assert!(std::panic::catch_unwind(|| normalize(path, &format!("{raw}{extra}"))).is_err());
        assert_ne!(
            normalize(path, &format!("{raw}/* old predicate changed */")),
            git(&["show", &format!("{BASE}:{path}")])
        );
    }
    let raw = read("docs/testing/gated-test-registry.toml");
    for bad in [
        raw.replace("# BEGIN ORIGINAL PENDING WATCHDOG GATE", "# removed"),
        raw.replace("# END ORIGINAL PENDING WATCHDOG GATE", "# removed"),
    ] {
        assert!(std::panic::catch_unwind(|| strip_registry(&bad)).is_err());
    }
    let command_mutation = raw.replace(
        "--locked\",\"original_pending_watchdog_frame_controls",
        "--locked\",\"changed_command",
    );
    assert!(std::panic::catch_unwind(|| strip_registry(&command_mutation)).is_err());
}
#[test]
fn original_pending_watchdog_has_full_rounds_owned_cleanup_and_no_target_or_new_timer() {
    let engine = read("tools/long-run-supervisor-074/src/diagnostic_inherited_pending_watchdog.rs");
    for forbidden in [
        "timerfd_",
        "LOCK_UN",
        "libc::flock",
        "alarm(",
        "enroll_once(",
        "StartTransientUnit",
        "pub fn",
    ] {
        assert!(!engine.contains(forbidden), "{forbidden}");
    }
    for required in [
        "alarm.try_clone_to_owned()",
        "libc::P_PIDFD",
        "let owned = (pid > 0)",
        "owned.0.pending()?",
        "child.consumed",
        "child.armed",
    ] {
        assert!(engine.contains(required), "{required}");
    }
    assert!(engine.find("let owned = (pid > 0)").unwrap() < engine.rfind("&previous,").unwrap());
    let join = read("tools/long-run-supervisor-074/src/diagnostic_original_pending_watchdog.rs");
    assert_eq!(join.matches("original.full_round()?").count(), 3);
    for forbidden in [
        "sendmsg",
        "try_clone",
        "Deserialize",
        "Serialize",
        "release_barrier",
        "pub fn",
        "timerfd_",
    ] {
        assert!(!join.contains(forbidden), "{forbidden}");
    }
    for required in [
        "original: OriginalWatchdogContext",
        "original.refuse();",
        "Box::new(self)",
        "original_descriptors(&self.original.original.original, &self.binding)",
    ] {
        assert!(join.contains(required), "{required}");
    }
    let probe =
        read("tools/long-run-supervisor-074/src/diagnostic_original_pending_watchdog_test_api.rs");
    assert!(!probe.contains("OriginalPendingWatchdogs {"));
    assert!(!probe.contains("OriginalWatchdogContext {"));
    let contract: toml::Value = toml::from_str(&read(
        "docs/testing/performance/0.74/original-pending-watchdog-contract.toml",
    ))
    .unwrap();
    for key in [
        "same_original_owner_borrow",
        "derived_original_fence_and_timer",
        "same_fence_ofd",
        "same_timer_ofd",
        "pending_protocol_unchanged",
    ] {
        assert_eq!(contract[key].as_bool(), Some(true), "{key}");
    }
    for key in [
        "new_timer_arm",
        "target_enrollment",
        "original_send",
        "barrier_release",
        "responsiveness_proved",
        "common_mode_failure_guarantee",
        "production_enrolled",
        "two_phase_artifacts_bound",
        "whole_cgroup_bound",
        "all_writers_ceased",
        "completion_allowed",
        "server_operation_allowed",
        "qualification_allowed",
    ] {
        assert_eq!(contract[key].as_bool(), Some(false), "{key}");
    }
    for (key, value) in [
        ("guardians", 2),
        ("complete_rounds_around_setup", 2),
        ("healthy_setup_authentication_calls", 68),
        ("healthy_setup_channel_checks", 4),
        ("revalidation_authentication_calls", 34),
        ("revalidation_channel_checks", 2),
        ("frame_scenarios", 10),
    ] {
        assert_eq!(contract[key].as_integer(), Some(value), "{key}");
    }
}
