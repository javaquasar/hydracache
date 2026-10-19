//! Local pair composition must not modify any prior runtime or wire body.
#[path = "performance_contract_074_original_watchdog_context.rs"]
mod original_context;
const BASE: &str = "6f1dbfab";
const MAIN: &str = r####"
#[path = "diagnostic_redundant_watchdog.rs"]
mod redundant;
"####;
const FIXTURE: &str = r####"
#[path = "diagnostic_redundant_watchdog_fixture.rs"]
mod redundant;
"####;
const COMPAT: &str = r####"
## Private redundant guardian test carrier

hc074-redundant-owner-test-v1 is an owned nonroot test carrier, not a production
reader. Its fixed JSON contains schema, case, creating owner PID, selected
absolute BOOTTIME deadline, Pending and Armed. Actual kernel credentials must
name that creator. Exactly four CLOEXEC rights carry two guardian pidfds, the
target pidfd and read pipe; one explicit hidden write alias raises that to five.
Only the echoed MSG_CMSG_CLOEXEC flag is accepted; truncation, unknown schema,
extra fields/rights or foreign credentials refuse. D without rights marks one
completed harness command. Reader lifetime is the original private temporary
socket/directory, with no reconnect, adoption, downgrade or production consumer.
Both guardians still use unchanged hc074-prearmed-guardian-v1. This composition
adds no production protocol, original authority, completion or unlock operation.
"####;
const REGISTRY: &str = r####"
# BEGIN REDUNDANT WATCHDOG GATE

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.redundant-watchdog-owner-loss"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_redundant_watchdog_fixture.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "redundant_watchdog_owner_victim"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "manual owned single-guardian-failure experiment; no original admission"
tier = "manual"
required_features = []
required_env = ["HC074_REDUNDANT_TEST_BINARY"]
required_tools = ["python3"]
timeout_seconds = 180
owner_release = "0.74.0"
ship_mandatory = false
artifacts = []

[gate.ci]
workflow = ".github/workflows/ci.yml"
job = "gated-proof-registry"
step = "Run registered gated proofs"

[gate.command]
program = "python3"
args = ["tools/long-run-supervisor-074/tests/redundant_watchdog.py","--binary-env","HC074_REDUNDANT_TEST_BINARY"]
cwd = "."
platform = "linux"

[gate.command.env]

# END REDUNDANT WATCHDOG GATE
"####;
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let current = original_context::normalize(path, raw);
    let raw = current.as_str();
    let addition = match path {
        "tools/long-run-supervisor-074/src/diagnostic_prearmed_watchdog.rs" => MAIN,
        "tools/long-run-supervisor-074/src/diagnostic_prearmed_watchdog_fixture.rs" => FIXTURE,
        "docs/COMPAT.md" => COMPAT,
        "docs/testing/gated-test-registry.toml" => REGISTRY,
        _ => return raw.to_owned(),
    };
    assert_eq!(
        raw.matches(addition).count(),
        1,
        "redundant registration drift: {path}"
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
fn redundant_watchdog_preserves_every_previous_source_and_protocol() {
    let paths = git(&[
        "ls-tree",
        "-r",
        "--name-only",
        BASE,
        "--",
        "tools/long-run-supervisor-074/src",
    ]);
    let paths: Vec<_> = paths.lines().filter(|p| p.ends_with(".rs")).collect();
    assert_eq!(paths.len(), 369);
    for path in paths.into_iter().chain([
        "docs/COMPAT.md",
        "docs/testing/gated-test-registry.toml",
        "tools/long-run-supervisor-074/tests/prearmed_watchdog.py",
        "tools/long-run-supervisor-074/tests/original_custody_owner_loss.py",
    ]) {
        assert_eq!(
            normalize(path, &read(path)),
            git(&["show", &format!("{BASE}:{path}")]),
            "{path}"
        );
    }
}
#[test]
fn redundant_watchdog_adapter_rejects_changed_registration_and_old_body() {
    for (path, addition) in [
        (
            "tools/long-run-supervisor-074/src/diagnostic_prearmed_watchdog.rs",
            MAIN,
        ),
        (
            "tools/long-run-supervisor-074/src/diagnostic_prearmed_watchdog_fixture.rs",
            FIXTURE,
        ),
        ("docs/COMPAT.md", COMPAT),
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
                &raw.replacen(addition, &format!("{addition}/* mutation */"), 1)
            ),
            git(&["show", &format!("{BASE}:{path}")])
        );
    }
    let path = "tools/long-run-supervisor-074/src/diagnostic_prearmed_watchdog.rs";
    let raw = read(path);
    for predicate in [
        "message.valid(ENROLL",
        "live_child(fd, origin.pid)",
        "target = Some(fd)",
    ] {
        assert!(raw.contains(predicate));
        assert_ne!(
            normalize(
                path,
                &raw.replacen(predicate, &format!("/* changed */{predicate}"), 1)
            ),
            git(&["show", &format!("{BASE}:{path}")])
        );
    }
}
#[test]
fn redundant_watchdog_is_same_bound_same_ofd_and_not_original_authority() {
    let code = read("tools/long-run-supervisor-074/src/diagnostic_redundant_watchdog.rs");
    for token in [
        "LOCK_UN",
        "StartTransientUnit",
        "OriginalEnrollmentFrame",
        "pub fn",
        "Command::",
        "Duration::",
        "Instant::",
    ] {
        assert!(!code.contains(token), "{token}");
    }
    for token in [
        "fence.try_clone()?",
        "guardians: [Watchdog; 2]",
        "Origin::current() == self.origin",
        "self.live(true)",
    ] {
        assert!(code.contains(token), "{token}");
    }
    assert_eq!(code.matches("\n            deadline,").count(), 2);
    assert!(
        code.find("self.consumed = true;").unwrap()
            < code.find("guardian.enroll_once(target)").unwrap()
    );
    let contract: toml::Value = toml::from_str(&read(
        "docs/testing/performance/0.74/redundant-watchdog-contract.toml",
    ))
    .unwrap();
    for key in [
        "same_fence_ofd",
        "same_absolute_deadline",
        "both_pending_before_target",
        "both_armed_before_fixture_release",
        "global_enrollment_at_most_once",
        "no_fence_unlock_path",
        "single_guardian_failure_experiment",
    ] {
        assert_eq!(contract[key].as_bool(), Some(true), "{key}");
    }
    for key in [
        "immediate_sibling_failure_detection",
        "common_mode_failure_guarantee",
        "historical_original_owner_authenticated",
        "trusted_original_kernel_context",
        "whole_cgroup_revocation",
        "all_external_writers_revoked",
        "production_constructor_enrolled",
        "server_operation_allowed",
        "qualification_allowed",
    ] {
        assert_eq!(contract[key].as_bool(), Some(false), "{key}");
    }
    assert_eq!(contract["guardians"].as_integer(), Some(2));
    assert_eq!(contract["process_fault_cases"].as_integer(), Some(17));
}
