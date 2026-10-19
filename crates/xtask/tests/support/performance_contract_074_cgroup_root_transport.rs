//! Exact forward adapter; old enrollment/guardian predicates remain immutable.
const BASE: &str = "f183134c";
const MODULE: &str = "\n#[path = \"diagnostic_cgroup_root_transport.rs\"]\nmod transport;\n";
const COMPAT: &str = r####"
## Local cgroup root descriptor transport

The private ordinary-process reader of hc074-cgroup-root-enrollment-v2 accepts
only the existing 104-byte Enroll packet and exactly directory/kill/events
SCM_RIGHTS on an unnamed UNIX seqpacket pair. SO_PASSCRED is required before
send; exactly one SCM_CREDENTIALS must match the actual pair creator PID and
real UID/GID. No numeric caller credentials or old guardian-v1 fallback exist.
All received rights require atomic MSG_CMSG_CLOEXEC, exact access modes and
matching retained/named root-owned cgroup2 objects. Unknown ancillary data,
truncation, extra/missing/substituted rights, binding or owner drift refuse.
Every installed rejected right is closed. Both sides consume their single
operation before its syscall; uncertainty or absent ACK never permits retry.

This reader retains descriptors but emits no Armed or other acknowledgement.
It is not a post-clone-safe guardian, original-owner constructor or runtime
admission API. Replies, guardian credentials, original fence/timer, full-frame
join and durable completion remain separate gates. Reader compatibility is
only v2 as above, with no migration, reconnect or durable replay slot. Existing
packet claims, v1 direct-child guardian and original projections are unchanged.
"####;
const GATE: &str = r####"
# BEGIN CGROUP ROOT TRANSPORT GATE

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.cgroup-root-transport-controls"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_cgroup_root_transport_fault_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "cgroup_root_transport_role_matrix"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "manual isolated local root cgroup2 descriptor transfer; no service or product"
tier = "manual"
required_features = []
required_env = ["HC074_ROOT_TRANSPORT_FIXTURE"]
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","cgroup_root_transport_role_matrix","--","--ignored","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

# END CGROUP ROOT TRANSPORT GATE
"####;
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let extra = match path {
        "tools/long-run-supervisor-074/src/diagnostic_cgroup_root.rs" => MODULE,
        "docs/COMPAT.md" => COMPAT,
        "docs/testing/gated-test-registry.toml" => GATE,
        _ => return raw.to_owned(),
    };
    assert_eq!(raw.matches(extra).count(), 1, "transport addition: {path}");
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
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(root())
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().replace('\r', "")
}
#[test]
fn cgroup_root_transport_preserves_all_prior_sources() {
    let listing = git(&[
        "ls-tree",
        "-r",
        "--name-only",
        BASE,
        "--",
        "tools/long-run-supervisor-074/src",
    ]);
    let paths: Vec<_> = listing.lines().filter(|p| p.ends_with(".rs")).collect();
    assert_eq!(paths.len(), 386);
    for path in paths.into_iter().chain([
        "docs/COMPAT.md",
        "docs/testing/gated-test-registry.toml",
        "tools/long-run-supervisor-074/tests/prearmed_watchdog.py",
        "tools/long-run-supervisor-074/tests/redundant_watchdog.py",
    ]) {
        assert!(
            normalize(path, &read(path)) == git(&["show", &format!("{BASE}:{path}")]),
            "prior body drift: {path}"
        );
    }
}
#[test]
fn cgroup_root_transport_requires_kernel_credentials_and_no_armed_emitter() {
    let code = read("tools/long-run-supervisor-074/src/diagnostic_cgroup_root_transport.rs");
    for required in [
        "SO_PASSCRED",
        "SCM_CREDENTIALS",
        "SCM_RIGHTS",
        "MSG_CMSG_CLOEXEC",
        "MSG_TRUNC",
        "MSG_NOSIGNAL",
        "SOCK_SEQPACKET",
        "self.consumed = true",
        "cgroup.kill",
        "cgroup.events",
        "O_ACCMODE",
        "O_PATH",
        "libc::getpid()",
    ] {
        assert!(code.contains(required), "{required}");
    }
    for prohibited in [
        "Kind::Armed",
        "LOCK_UN",
        "timerfd_",
        "StartTransientUnit",
        "pub fn",
        "submit_kill(",
    ] {
        assert!(!code.contains(prohibited), "{prohibited}");
    }
    let contract: toml::Value = toml::from_str(&read(
        "docs/testing/performance/0.74/cgroup-root-transport-contract.toml",
    ))
    .unwrap();
    for key in [
        "authenticated_guardian_reply",
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
        assert_eq!(contract[key].as_bool(), Some(false), "{key}");
    }
}
#[test]
fn cgroup_root_transport_adapter_refuses_missing_duplicate_and_old_body_drift() {
    for (path, extra) in [
        (
            "tools/long-run-supervisor-074/src/diagnostic_cgroup_root.rs",
            MODULE,
        ),
        ("docs/COMPAT.md", COMPAT),
        ("docs/testing/gated-test-registry.toml", GATE),
    ] {
        let raw = read(path);
        assert!(std::panic::catch_unwind(|| normalize(path, &raw.replacen(extra, "", 1))).is_err());
        assert!(std::panic::catch_unwind(|| normalize(path, &format!("{raw}{extra}"))).is_err());
        assert_ne!(
            normalize(path, &format!("{raw}/* drift */")),
            git(&["show", &format!("{BASE}:{path}")])
        );
    }
}
