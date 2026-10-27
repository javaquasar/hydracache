//! Freeze old mechanics while introducing a distinct local active v2 pair.
#[path = "performance_contract_074_root_creator.rs"]
mod creator;
const BASE: &str = "037802aa";
const MODULE: &str = "\n#[path = \"diagnostic_root_guardian.rs\"]\nmod guardian;\n";
const FAULT_MODULE: &str =
    "\n#[path = \"diagnostic_root_guardian_fault_tests.rs\"]\nmod guardians;\n";
const COMPAT: &str = r####"
## Active local cgroup root guardian reader

The private local guardian reader uses hc074-cgroup-root-enrollment-v2 without
changing its 104-byte HCRG0742 packet. It accepts one creator-authenticated Enroll
with three CLOEXEC directory/kill/events rights. Actual root-owned cgroup2
retained/named identities and empty unfrozen domain must match the inherited
snapshot before a rights-free Armed reply. The ordinary parent requires the
exact clone3 child PID, live pidfd, real UID/GID, binding and original timer.
Both requests precede waiting for replies. Missing/extra rights, malformed or
unknown bytes, truncation, substituted credentials/binding or replay refuse.
No Pending packet, downgrade, reconnect, migration or durable replay slot exists.

Each isolated child retains its root and the same original fence/timer OFDs.
Each additionally opens its own read-only events watch OFD on the bound inode;
notification consumption must not be shared between guardians. Owner pidfd,
timer or channel termination and observed root drift enter permanent revocation.
Only the retained original kill FD is used, including for late population.
Refused and Revoking replies echo the binding with no rights and are best effort
not completion receipts. No unlock, all-writer cessation or original Armed
capability follows. This local reader is not a production owner constructor,
held-unit projection, trust enrollment or server execution API. Original v1
direct-child predicates, original projections and old packet claims stay intact.
"####;
const GATE: &str = r####"
# BEGIN ROOT GUARDIAN GATE

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.root-guardian-controls"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_root_guardian_fault_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "cgroup_root_guardian_fault_matrix"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "manual isolated local root cgroup2 guardians; no service, original owner or product"
tier = "manual"
required_features = []
required_env = ["HC074_ROOT_GUARDIAN_FIXTURE"]
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","cgroup_root_guardian_fault_matrix","--","--ignored","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

# END ROOT GUARDIAN GATE
"####;
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let current = creator::normalize(path, raw);
    let raw = current.as_str();
    let extra = match path {
        "tools/long-run-supervisor-074/src/diagnostic_cgroup_root.rs" => MODULE,
        "tools/long-run-supervisor-074/src/diagnostic_cgroup_root_fault_tests.rs" => FAULT_MODULE,
        "docs/COMPAT.md" => COMPAT,
        "docs/testing/gated-test-registry.toml" => GATE,
        _ => return raw.to_owned(),
    };
    assert_eq!(raw.matches(extra).count(), 1, "guardian addition: {path}");
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
fn root_guardian_preserves_every_previous_source_and_protocol() {
    let listing = git(&[
        "ls-tree",
        "-r",
        "--name-only",
        BASE,
        "--",
        "tools/long-run-supervisor-074/src",
    ]);
    let files: Vec<_> = listing.lines().filter(|p| p.ends_with(".rs")).collect();
    assert_eq!(files.len(), 389);
    for path in files.into_iter().chain([
        "docs/COMPAT.md",
        "docs/testing/gated-test-registry.toml",
        "tools/long-run-supervisor-074/tests/prearmed_watchdog.py",
        "tools/long-run-supervisor-074/tests/redundant_watchdog.py",
    ]) {
        assert!(
            normalize(path, &read(path)) == git(&["show", &format!("{BASE}:{path}")]),
            "old body drift: {path}"
        );
    }
}
#[test]
fn root_guardian_requires_stack_only_child_and_closed_original_capabilities() {
    let raw = read("tools/long-run-supervisor-074/src/diagnostic_root_guardian_raw.rs");
    for word in [
        "Vec",
        "vec!",
        "Box",
        "String",
        "File::",
        "format!",
        "io::",
        "timerfd_create",
        "timerfd_settime",
        "flock",
        "LOCK_UN",
        "StartTransientUnit",
    ] {
        assert!(!raw.contains(word), "post-clone forbidden: {word}");
    }
    for word in [
        "SCM_CREDENTIALS",
        "SCM_RIGHTS",
        "MSG_CMSG_CLOEXEC",
        "SYS_close_range",
        "POLLPRI",
        "cgroup.kill",
        "retained = Some",
        "let watch = open",
    ] {
        assert!(raw.contains(word), "missing guard: {word}");
    }
    let revoke = raw
        .split("fn revoke(")
        .nth(1)
        .unwrap()
        .split("pub(super) fn child")
        .next()
        .unwrap();
    assert!(revoke.find("let observed = bytes").unwrap() < revoke.find("libc::pwrite").unwrap());
    let parent = read("tools/long-run-supervisor-074/src/diagnostic_root_guardian.rs");
    for word in [
        "timerfd_create",
        "timerfd_settime",
        "timerfd_gettime",
        "LOCK_UN",
        "libc::flock(",
        "StartTransientUnit",
    ] {
        assert!(!parent.contains(word), "{word}");
    }
    for word in [
        "remaining_from_info",
        "SYS_pidfd_open",
        "SYS_clone3",
        "self.consumed = true",
        "guard.consumed = true",
        "self.notice(0)?",
        "self.notice(1)?",
    ] {
        assert!(parent.contains(word), "{word}");
    }
    let contract: toml::Value = toml::from_str(&read(
        "docs/testing/performance/0.74/root-guardian-contract.toml",
    ))
    .unwrap();
    for key in [
        "creator_sigkill_tested",
        "original_owner_constructor",
        "original_armed_capability",
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
fn root_guardian_exact_adapter_rejects_missing_duplicate_and_old_body_mutation() {
    for (path, extra) in [
        (
            "tools/long-run-supervisor-074/src/diagnostic_cgroup_root.rs",
            MODULE,
        ),
        (
            "tools/long-run-supervisor-074/src/diagnostic_cgroup_root_fault_tests.rs",
            FAULT_MODULE,
        ),
        ("docs/COMPAT.md", COMPAT),
        ("docs/testing/gated-test-registry.toml", GATE),
    ] {
        let raw = read(path);
        assert!(std::panic::catch_unwind(|| normalize(path, &raw.replacen(extra, "", 1))).is_err());
        assert!(std::panic::catch_unwind(|| normalize(path, &format!("{raw}{extra}"))).is_err());
        assert_ne!(
            normalize(path, &format!("{raw}/* predicate mutation */")),
            git(&["show", &format!("{BASE}:{path}")])
        );
    }
}
