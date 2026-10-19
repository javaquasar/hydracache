//! Preserve prior admission while adding a distinct root enrollment protocol.
const BASE: &str = "7fb289c5";
const GATE: &str = r####"
# BEGIN CGROUP ROOT ENROLLMENT GATE

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.cgroup-root-fault-controls"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_cgroup_root_fault_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "cgroup_root_kernel_fault_matrix"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "manual isolated local root cgroup2 fixtures; no systemd or product"
tier = "manual"
required_features = []
required_env = ["HC074_ROOT_FAULT_FIXTURE"]
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","cgroup_root_kernel_fault_matrix","--","--ignored","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

# END CGROUP ROOT ENROLLMENT GATE
"####;
const MODULE: &str = "\n#[path = \"diagnostic_root_enrollment.rs\"]\npub mod root_enrollment;\n";
const COMPAT: &str = r####"

## Attempt cgroup root enrollment packet

hc074-cgroup-root-enrollment-v2 uses exactly 104 bytes: HCRG0742 magic,
one kind byte (Enroll=1, Armed=2, Refused=3, Revoking=4), seven zero reserved
bytes, 32-byte attempt digest, little-endian root device/inode (8 bytes each),
little-endian absolute BOOTTIME deadline (8 bytes), and 32-byte original-context
digest. Digests must be nonzero, root IDs positive and deadline in 1..=i64::MAX.
Unknown version/kind, reserved bytes, invalid fields or any size change refuse.
The reader window is this version only; old guardian-v1 packets cannot upgrade
or downgrade. There is no migration, reconnect or new durable replay slot.

This package defines data-only bytes and private pinned cgroup2 mechanics, not
an authenticated transport, Armed capability or original-start permit. The
future Enroll transport must require exact creator credentials and three
atomic-CLOEXEC rights in directory/kill/events order, validate those roles and
retain the root before Armed. Replies carry no rights and must echo every bound
field. A model sender consumes its one send before the syscall boundary;
missing or malformed ACK cannot reset it. Context/deadline bytes are claims,
not live observations or permission to renew the original 300/60/10 bounds.
Existing v1 guardian, signed v2 launch and original worker projections remain
unchanged. Root kill submission or empty-at-read never authorizes completion,
fence release or server execution, including when external pipe aliases remain.
"####;
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let extra = match path {
        "docs/testing/gated-test-registry.toml" => GATE,
        "tools/long-run-supervisor-074/src/diagnostic_two_phase_launch.rs" => MODULE,
        "docs/COMPAT.md" => COMPAT,
        _ => return raw.to_owned(),
    };
    assert_eq!(
        raw.matches(extra).count(),
        1,
        "root enrollment addition: {path}"
    );
    raw.replacen(extra, "", 1)
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
    let result = std::process::Command::new("git")
        .args(args)
        .current_dir(root())
        .output()
        .unwrap();
    assert!(result.status.success());
    String::from_utf8(result.stdout).unwrap().replace('\r', "")
}
#[test]
fn cgroup_root_enrollment_preserves_old_sources_and_protocols() {
    let paths = git(&[
        "ls-tree",
        "-r",
        "--name-only",
        BASE,
        "--",
        "tools/long-run-supervisor-074/src",
    ]);
    let paths: Vec<_> = paths.lines().filter(|p| p.ends_with(".rs")).collect();
    assert_eq!(paths.len(), 381);
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
fn cgroup_root_enrollment_gate_requires_separate_protocol_and_kernel_checks() {
    let protocol = read("tools/long-run-supervisor-074/src/diagnostic_root_enrollment.rs");
    for item in [
        "HCRG0742",
        "PACKET_BYTES: usize = 104",
        "self.consumed = true",
        "self.refused = true",
    ] {
        assert!(protocol.contains(item), "{item}");
    }
    let kernel = read("tools/long-run-supervisor-074/src/diagnostic_cgroup_root.rs");
    for item in [
        "0x63677270",
        "libc::O_NOFOLLOW",
        "cgroup.kill",
        "cgroup.events",
        "cgroup.type",
        "cgroup.procs",
        "self.submitted = true",
        "write_at(b\"1\", 0)",
    ] {
        assert!(kernel.contains(item), "{item}");
    }
    for item in [
        "LOCK_UN",
        "timerfd_",
        "StartTransientUnit",
        "systemctl",
        "pub fn",
        "remove_dir_all",
    ] {
        assert!(!kernel.contains(item), "{item}");
    }
    let contract: toml::Value = toml::from_str(&read(
        "docs/testing/performance/0.74/cgroup-root-enrollment-contract.toml",
    ))
    .unwrap();
    for name in [
        "authenticated_enrollment_transport",
        "original_owner_constructor",
        "autonomous_root_guardian",
        "armed_capability",
        "whole_writer_cessation_proved",
        "held_unit_projection",
        "production_trust_enrolled",
        "completion_allowed",
        "fence_release_allowed",
        "server_operation_allowed",
        "product_allowed",
        "qualification_allowed",
    ] {
        assert_eq!(contract[name].as_bool(), Some(false), "{name}");
    }
    assert_eq!(contract["packet_bytes"].as_integer(), Some(104));
    assert_eq!(contract["actual_owner_sigkill_cuts"].as_integer(), Some(4));
}
#[test]
fn cgroup_root_enrollment_additive_adapter_rejects_old_body_and_registration_drift() {
    for path in [
        "tools/long-run-supervisor-074/src/diagnostic_two_phase_launch.rs",
        "docs/COMPAT.md",
        "docs/testing/gated-test-registry.toml",
    ] {
        let raw = read(path);
        let extra = match path {
            "docs/COMPAT.md" => COMPAT,
            "docs/testing/gated-test-registry.toml" => GATE,
            _ => MODULE,
        };
        assert!(std::panic::catch_unwind(|| normalize(path, &raw.replacen(extra, "", 1))).is_err());
        assert!(std::panic::catch_unwind(|| normalize(path, &format!("{raw}{extra}"))).is_err());
        assert_ne!(
            normalize(path, &format!("{raw}/* old predicate drift */")),
            git(&["show", &format!("{BASE}:{path}")])
        );
    }
}
