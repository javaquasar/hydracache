//! Exact test-only cuts; every pre-existing runtime predicate is preserved.
#[path = "performance_contract_074_original_root_join.rs"]
mod original_join;
const BASE: &str = "4e940734";
const MODULE: &str =
    "\n#[cfg(test)]\n#[path = \"diagnostic_root_creator_tests.rs\"]\nmod creator;\n";
const HOOKS: [&str; 4] = [
    "        #[cfg(test)]\n        creator::cut(self, 1);\n",
    "        #[cfg(test)]\n        creator::cut(self, 2);\n",
    "        #[cfg(test)]\n        creator::cut(self, 3);\n",
    "        #[cfg(test)]\n        creator::cut(self, 5);\n",
];
const COMPAT: &str = r####"
## Local root guardian creator cut carrier

The external local-only root fixture uses a test-only 104-byte HCRGCUT1 carrier:
eight magic bytes, one stage byte 0 through 5 and 95 zero reserved bytes. Stage
zero transfers exactly two guardian pidfds; later stages transfer no rights.
The harness checks exact creator SCM_CREDENTIALS, atomic CLOEXEC rights, packet
length, reserved bytes and truncation. Unknown versions or stages refuse; no
migration, durable replay, reconnect or production reader exists. Only an
explicit ignored libtest victim and owning local subreaper use this carrier.
This does not change HCRG0742, its reader, or the original guardian v1 protocol.
"####;
const GATE: &str = r####"
# BEGIN ROOT CREATOR GATE

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.root-creator-controls"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_root_creator_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "root_creator_victim"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "external root fixture creator SIGKILL; no service or product"
tier = "manual"
required_features = []
required_env = ["HC074_CREATOR_FIXTURE", "HC074_CREATOR_TEST_BINARY"]
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
args = ["tools/long-run-supervisor-074/tests/root_creator_sigkill.py","--binary-env","HC074_CREATOR_TEST_BINARY"]
cwd = "."
platform = "linux"

[gate.command.env]

# END ROOT CREATOR GATE
"####;
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let current = original_join::normalize(path, raw);
    let raw = current.as_str();
    let additions: Vec<&str> = match path {
        "tools/long-run-supervisor-074/src/diagnostic_root_guardian.rs" => {
            std::iter::once(MODULE).chain(HOOKS).collect()
        }
        "docs/COMPAT.md" => vec![COMPAT],
        "docs/testing/gated-test-registry.toml" => vec![GATE],
        _ => return raw.to_owned(),
    };
    let mut value = raw.to_owned();
    for extra in additions {
        assert_eq!(
            value.matches(extra).count(),
            1,
            "creator exact addition: {path}"
        );
        value = value.replacen(extra, "", 1);
    }
    value
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
fn root_creator_preserves_all_previous_sources_and_runtime_protocols() {
    let listing = git(&[
        "ls-tree",
        "-r",
        "--name-only",
        BASE,
        "--",
        "tools/long-run-supervisor-074/src",
    ]);
    let sources: Vec<_> = listing.lines().filter(|p| p.ends_with(".rs")).collect();
    assert_eq!(sources.len(), 393);
    for path in sources.into_iter().chain([
        "docs/COMPAT.md",
        "docs/testing/gated-test-registry.toml",
        "tools/long-run-supervisor-074/tests/prearmed_watchdog.py",
        "tools/long-run-supervisor-074/tests/redundant_watchdog.py",
    ]) {
        assert!(
            normalize(path, &read(path)) == git(&["show", &format!("{BASE}:{path}")]),
            "{path}"
        );
    }
}
#[test]
fn root_creator_external_harness_requires_actual_death_and_closed_completion() {
    let driver = read("tools/long-run-supervisor-074/tests/root_creator_sigkill.py");
    for item in [
        "os.pidfd_open(owner.pid)",
        "signal.pidfd_send_signal(owner_pin, signal.SIGKILL)",
        "owner.wait(timeout=5) == -signal.SIGKILL",
        "parent_pid(pid) == os.getpid()",
        "socket.MSG_CMSG_CLOEXEC",
        "locked(fence_path, True)",
        "cleanup(binary)",
        "previous.value",
        "range(1, 6)",
        "original_authority=false completion=false",
    ] {
        assert!(driver.contains(item), "{item}");
    }
    let victim = read("tools/long-run-supervisor-074/src/diagnostic_root_creator_tests.rs");
    assert!(victim.contains("pair.enroll_once().unwrap()"));
    assert!(victim.contains("libc::pause()"));
    assert!(!victim.contains("timerfd_settime"));
    let contract: toml::Value = toml::from_str(&read(
        "docs/testing/performance/0.74/root-creator-contract.toml",
    ))
    .unwrap();
    for key in [
        "original_armed_capability",
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
fn root_creator_adapter_rejects_missing_duplicate_and_body_drift() {
    for (path, extra) in [
        (
            "tools/long-run-supervisor-074/src/diagnostic_root_guardian.rs",
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
