//! Exact additive original preparation only, never watchdog or target admission.
#[path = "performance_contract_074_original_pending_watchdog.rs"]
mod pending;
const BASE: &str = "38f8ecb2";
const OWNER: &str = r####"
#[path = "diagnostic_original_watchdog_context.rs"]
pub(super) mod watchdog_context;
"####;
const API: &str = r####"
#[path = "diagnostic_original_watchdog_context_test_api.rs"]
mod watchdog;
"####;
const FRAME: &str = r####"
#[path = "diagnostic_original_watchdog_context_frame_tests.rs"]
mod watchdog_context;
"####;
const REGISTRY: &str = r####"
# BEGIN ORIGINAL WATCHDOG CONTEXT GATE

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-watchdog-context-controls"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_watchdog_context_frame_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_watchdog_context_frame_controls"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "manual nonroot original context frame; no guardian spawn or delivery"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","original_watchdog_context_frame_controls","--","--ignored","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

# END ORIGINAL WATCHDOG CONTEXT GATE
"####;
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let current = pending::normalize(path, raw);
    let raw = current.as_str();
    let addition = match path {
        "tools/long-run-supervisor-074/src/diagnostic_original_manager_message.rs" => OWNER,
        "tools/long-run-supervisor-074/src/diagnostic_original_manager_message_test_api.rs" => API,
        "tools/long-run-supervisor-074/src/diagnostic_original_manager_message_frame_tests.rs" => {
            FRAME
        }
        "docs/testing/gated-test-registry.toml" => REGISTRY,
        _ => return raw.to_owned(),
    };
    assert_eq!(
        raw.matches(addition).count(),
        1,
        "original context registration drift: {path}"
    );
    raw.replacen(addition, "", 1)
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
fn original_watchdog_context_preserves_every_old_rust_and_wire_body() {
    let paths = git(&[
        "ls-tree",
        "-r",
        "--name-only",
        BASE,
        "--",
        "tools/long-run-supervisor-074/src",
    ]);
    let paths: Vec<_> = paths.lines().filter(|p| p.ends_with(".rs")).collect();
    assert_eq!(paths.len(), 372);
    for path in paths
        .into_iter()
        .chain(["docs/COMPAT.md", "docs/testing/gated-test-registry.toml"])
    {
        assert_eq!(
            normalize(path, &read(path)),
            git(&["show", &format!("{BASE}:{path}")]),
            "{path}"
        );
    }
}
#[test]
fn original_watchdog_context_adapter_refuses_old_predicate_or_registration_mutations() {
    for (path, addition) in [
        (
            "tools/long-run-supervisor-074/src/diagnostic_original_manager_message.rs",
            OWNER,
        ),
        (
            "tools/long-run-supervisor-074/src/diagnostic_original_manager_message_test_api.rs",
            API,
        ),
        (
            "tools/long-run-supervisor-074/src/diagnostic_original_manager_message_frame_tests.rs",
            FRAME,
        ),
        ("docs/testing/gated-test-registry.toml", REGISTRY),
    ] {
        let raw = read(path);
        assert!(
            std::panic::catch_unwind(|| normalize(path, &raw.replacen(addition, "", 1))).is_err()
        );
        assert!(std::panic::catch_unwind(|| normalize(path, &format!("{raw}{addition}"))).is_err());
        assert_ne!(
            normalize(
                path,
                &raw.replacen(addition, &format!("{addition}/* changed */"), 1)
            ),
            git(&["show", &format!("{BASE}:{path}")])
        );
    }
}
#[test]
fn original_watchdog_context_retains_original_owner_and_one_full_round_without_effects() {
    let code = read("tools/long-run-supervisor-074/src/diagnostic_original_watchdog_context.rs");
    assert_eq!(code.matches("super::round(").count(), 1);
    for token in [
        "&'held mut OriginalManagerMessage",
        "let opening = capture(frame",
        "let closing = capture(frame",
        "old.source != opening",
        "opening != closing",
        "frame.original.refuse();",
        "self.refuse();",
    ] {
        assert!(code.contains(token), "{token}");
    }
    for token in [
        "try_clone",
        "timerfd_",
        "sendmsg",
        "StartTransientUnit",
        "GuardPair",
        "File::",
        "Deserialize",
        "Serialize",
        "Duration::",
        "pub fn",
    ] {
        assert!(!code.contains(token), "{token}");
    }
    let api =
        read("tools/long-run-supervisor-074/src/diagnostic_original_watchdog_context_test_api.rs");
    assert!(!api.contains("OriginalWatchdogContext {"));
    let contract: toml::Value = toml::from_str(&read(
        "docs/testing/performance/0.74/original-watchdog-context-contract.toml",
    ))
    .unwrap();
    for key in ["same_original_owner_borrow", "derived_inputs_only"] {
        assert_eq!(contract[key].as_bool(), Some(true), "{key}");
    }
    for key in [
        "new_fd_owners",
        "new_timer_arm",
        "watchdog_spawn",
        "original_send",
        "production_enrolled",
        "whole_cgroup_bound",
        "two_phase_artifacts_bound",
        "all_writers_ceased",
        "completion_allowed",
        "server_operation_allowed",
        "qualification_allowed",
    ] {
        assert_eq!(contract[key].as_bool(), Some(false), "{key}");
    }
    assert_eq!(contract["full_round_calls"].as_integer(), Some(1));
    assert_eq!(
        contract["healthy_authentication_calls"].as_integer(),
        Some(34)
    );
    assert_eq!(contract["healthy_channel_checks"].as_integer(), Some(2));
    assert_eq!(contract["frame_scenarios"].as_integer(), Some(14));
}
