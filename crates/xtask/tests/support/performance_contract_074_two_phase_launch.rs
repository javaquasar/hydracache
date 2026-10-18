//! Exact additive registration only; all original launch and validation remain.
#[path = "performance_contract_074_prearmed.rs"]
mod prearmed;
const BASE: &str = "db35fdc3";
const LIB: &str = "pub mod diagnostic_two_phase_launch;\n";
const READER: &str = "super::live_walk::normalize(p, &read(p))";
const READER_PATH: &str = "crates/xtask/tests/support/performance_contract_074_start_consent.rs";
const COMPAT: &str = r####"
## Two phase diagnostic launch requirements

The new data-only schemas diagnostic-two-phase-launch-requirements-074-v2,
diagnostic-two-phase-launch-074-v2 and diagnostic-signed-two-phase-launch-074-v2
use the distinct hydracache-diagnostic-two-phase-launch-074-v2 signature domain
with a NUL separator and canonical JSON plus one newline. Body/envelope caps are
16384/32768 bytes; all nested objects deny unknown fields. Unknown versions,
noncanonical/duplicate input, malformed hashes/signatures and a mismatched
requirements digest refuse. Readers accept only this exact version; old v1
consent and collector-unit readers remain unchanged and reject the new envelope.

The outer signature binds the original v1 consent claims and three artifact
roles to expected held-launch semantics. The requirements projection is not a
TransientUnitSpec and does not reinterpret the old executable projection.
Supplied-key inspection proves no enrollment, live peer/time, armed watchdog,
original delivery, execution, whole-cgroup revocation, completion or fence
release. Signed timestamps do not renew the original 300/60/10-second windows.
The private owned-held-barrier-mechanics-074-v0 prototype uses one ephemeral
AF_UNIX SOCK_SEQPACKET pair and exactly 32 expected opaque nonce bytes. It
accepts no rights; receive flags are zero or the echoed MSG_CMSG_CLOEXEC policy
only. Other shapes/flags/nonces refuse. Its reader window is that original
socketpair lifetime, with no adoption, reconnect, downgrade or migration.
A future production barrier needs a distinct reviewed protocol; this prototype
cannot construct an original execution owner.
"####;
const REGISTRY: &str = r####"
# BEGIN TWO PHASE BARRIER GATE

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.two-phase-barrier-owned-kernel"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_held_barrier_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "two_phase_barrier_owned_kernel_fault_matrix"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "owned nonroot barrier fork/pidfd faults; no product, systemd or original authority"
tier = "manual"
required_features = []
required_env = []
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_two_phase_launch::held_barrier::tests::two_phase_barrier_owned_kernel_fault_matrix","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

# END TWO PHASE BARRIER GATE
"####;
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let current = prearmed::normalize(path, raw);
    let raw = current.as_str();
    let (addition, replacement) = match path {
        "tools/long-run-supervisor-074/src/lib.rs" => (LIB, ""),
        "docs/COMPAT.md" => (COMPAT, ""),
        "docs/testing/gated-test-registry.toml" => (REGISTRY, ""),
        READER_PATH => (READER, "read(p)"),
        _ => return raw.to_owned(),
    };
    assert_eq!(
        raw.matches(addition).count(),
        1,
        "two phase registration drift: {path}"
    );
    raw.replacen(addition, replacement, 1)
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
fn two_phase_launch_preserves_every_prior_runtime_test_schema_and_registry() {
    let paths = git(&[
        "ls-tree",
        "-r",
        "--name-only",
        BASE,
        "--",
        "tools/long-run-supervisor-074/src",
    ]);
    let paths: Vec<_> = paths.lines().filter(|p| p.ends_with(".rs")).collect();
    assert_eq!(paths.len(), 362);
    for path in paths.into_iter().chain([
        "docs/COMPAT.md",
        "docs/testing/gated-test-registry.toml",
        READER_PATH,
    ]) {
        assert_eq!(
            normalize(path, &read(path)),
            git(&["show", &format!("{BASE}:{path}")]),
            "{path}"
        );
    }
}
#[test]
fn two_phase_launch_adapter_cannot_hide_predicate_or_registration_mutation() {
    for (path, addition) in [
        ("tools/long-run-supervisor-074/src/lib.rs", LIB),
        ("docs/COMPAT.md", COMPAT),
        ("docs/testing/gated-test-registry.toml", REGISTRY),
        (READER_PATH, READER),
    ] {
        let raw = read(path);
        assert!(
            std::panic::catch_unwind(|| normalize(path, &raw.replacen(addition, "", 1))).is_err()
        );
        assert!(std::panic::catch_unwind(|| normalize(path, &format!("{raw}{addition}"))).is_err());
        let changed = raw.replacen(addition, &format!("{addition}/* mutation */"), 1);
        assert_ne!(
            normalize(path, &changed),
            git(&["show", &format!("{BASE}:{path}")])
        );
    }
    let path = "tools/long-run-supervisor-074/src/diagnostic_original_manager_message.rs";
    let raw = read(path);
    for token in [
        "frame.revalidate().map_err(JoinError::from)?;",
        "channel.check(receiver, pin)?;",
        "payload.check(&input, manager, client)?;",
    ] {
        assert!(raw.contains(token), "{token}");
        let mutated = raw.replacen(token, &format!("{token} /* changed */"), 1);
        assert_ne!(
            normalize(path, &mutated),
            git(&["show", &format!("{BASE}:{path}")])
        );
    }
}
#[test]
fn two_phase_launch_signature_data_has_no_runtime_promotion_or_old_schema_upgrade() {
    let contract: toml::Value = toml::from_str(&read(
        "docs/testing/performance/0.74/two-phase-launch-contract.toml",
    ))
    .unwrap();
    for flag in [
        "data_requirements_implemented",
        "old_signed_projections_unchanged",
        "original_deadlines_unchanged",
        "unknown_outcome_forbids_resend",
    ] {
        assert_eq!(contract[flag].as_bool(), Some(true), "{flag}");
    }
    for flag in [
        "production_constructor_enrolled",
        "authenticated_original_delivery_implemented",
        "watchdog_readiness_proved",
        "all_cgroup_writers_revoked",
        "completion_enabled",
        "server_operation_allowed",
        "qualification_allowed",
    ] {
        assert_eq!(contract[flag].as_bool(), Some(false), "{flag}");
    }
    let raw = read("tools/long-run-supervisor-074/src/diagnostic_two_phase_launch.rs");
    for forbidden in [
        "pub fn start",
        "pub fn release",
        "TransientUnitSpec {",
        "Command::new",
        "File::open",
        "Connection::",
        "CheckedWorkerPolicy",
        "OriginalManagerMessage",
    ] {
        assert!(!raw.contains(forbidden), "{forbidden}");
    }
    assert!(raw.contains("start_consent_signing_message(consent).map_err"));
    assert!(raw.contains("key.verify_strict"));
    assert!(raw.contains("MAX_BODY_BYTES: usize = 16384"));
    assert!(raw.contains("MAX_ENVELOPE_BYTES: usize = 32768"));
    let barrier = read("tools/long-run-supervisor-074/src/diagnostic_held_barrier.rs");
    assert!(barrier.find("self.consumed = true;").unwrap() < barrier.find("libc::send(").unwrap());
    for forbidden in [
        "pub ",
        "Command::",
        "flock",
        "LOCK_UN",
        "StartTransientUnit",
        "Duration::",
        "Instant::",
    ] {
        assert!(!barrier.contains(forbidden), "{forbidden}");
    }
    assert!(barrier.contains("libc::TFD_TIMER_ABSTIME"));
    assert!(barrier.contains("libc::MSG_CMSG_CLOEXEC"));
}
