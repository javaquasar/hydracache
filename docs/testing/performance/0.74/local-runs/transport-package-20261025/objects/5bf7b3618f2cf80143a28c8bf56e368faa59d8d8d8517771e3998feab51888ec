//! Only additive private path observations; all old rounds remain byte-exact.
#[path = "fixed_slice_namespace_guard.rs"]
mod namespace;
const BASE: &str = "a33bd9a4";
const PREFIX: &str = "tools/long-run-supervisor-074/src/";
const HOOKS: &[(&str, &str)] = &[
    ("diagnostic_root_enrollment.rs", "\n#[cfg(all(target_os = \"linux\", target_arch = \"x86_64\"))]\npub(crate) use kernel::FixedSlice;\n"),
    ("diagnostic_cgroup_root.rs", "\n#[path = \"diagnostic_fixed_slice.rs\"]\nmod fixed_slice;\npub(crate) use fixed_slice::FixedSlice;\n"),
    ("diagnostic_root_placement_binding.rs", "\n#[path = \"diagnostic_fixed_slice_guardians.rs\"]\nmod fixed_slice;\n"),
    ("diagnostic_owned_placement.rs", "\n#[path = \"diagnostic_fixed_placement.rs\"]\nmod fixed_slice;\n"),
];
const GATE: &str = r##"
# BEGIN FIXED SLICE ANCHORS GATE
[[gate]]
id = "tool.hydracache-long-run-supervisor-074.fixed-slice-anchors"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_fixed_slice_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "privileged_fixed_slice_readonly_anchors"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "manual read-only local cgroup2 anchor check; no root creation or admission"
tier = "manual"
required_features = []
required_env = []
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","privileged_fixed_slice_readonly_anchors","--","--ignored","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"
[gate.command.env]
# END FIXED SLICE ANCHORS GATE
"##;
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let mut text = namespace::normalize(path, raw);
    for (file, hook) in HOOKS {
        if path == format!("{PREFIX}{file}") {
            assert_eq!(
                text.matches(hook).count(),
                1,
                "exact fixed slice hook: {path}"
            );
            text = text.replacen(hook, "", 1);
        }
    }
    if path == "docs/testing/gated-test-registry.toml" {
        assert_eq!(text.matches(GATE).count(), 1, "exact fixed slice gate");
        text = text.replacen(GATE, "", 1);
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
fn fixed_slice_preserves_all_432_original_sources_wire_registry_and_harness() {
    let files = git(&[
        "ls-tree",
        "-r",
        "--name-only",
        BASE,
        "--",
        "tools/long-run-supervisor-074/src",
    ]);
    assert_eq!(files.lines().count(), 432);
    for path in files.lines().chain([
        "docs/COMPAT.md",
        "docs/testing/gated-test-registry.toml",
        "tools/long-run-supervisor-074/tests/owned_root_creator_sigkill.py",
    ]) {
        assert_eq!(
            normalize(path, &read(path)),
            git(&["show", &format!("{BASE}:{path}")]),
            "{path}"
        );
    }
}
#[test]
fn fixed_slice_adapter_refuses_missing_duplicate_hooks_and_old_body_drift() {
    let hooks = HOOKS
        .iter()
        .map(|(file, hook)| (format!("{PREFIX}{file}"), *hook))
        .chain([("docs/testing/gated-test-registry.toml".into(), GATE)]);
    for (path, hook) in hooks {
        let raw = read(&path);
        for bad in [raw.replacen(hook, "", 1), format!("{raw}{hook}")] {
            assert!(std::panic::catch_unwind(|| normalize(&path, &bad)).is_err());
        }
        assert_ne!(
            normalize(&path, &format!("{raw}\n// old body drift\n")),
            git(&["show", &format!("{BASE}:{path}")])
        );
    }
}
#[test]
fn fixed_slice_is_readonly_owned_and_not_an_execution_or_release_capability() {
    let kernel = read(&format!("{PREFIX}diagnostic_fixed_slice.rs"));
    let owner = read(&format!("{PREFIX}diagnostic_fixed_placement.rs"));
    let guardians = read(&format!("{PREFIX}diagnostic_fixed_slice_guardians.rs"));
    for required in [
        "O_NOFOLLOW",
        "c\"system.slice\"",
        "identities(&anchors()?)?",
        "root.parent_identity != facts.system",
        "root.name != expected",
        "unchanged(&self.facts(root)?, &self.facts)",
    ] {
        assert!(kernel.contains(required), "{required}");
    }
    for required in [
        "controls.observe_slice(slice, snapshot.root.attempt_sha256)?;",
        "placement_join(owner, controls, raw, Some(snapshot))?;",
        "retire(owner, controls)",
        "held: Box::new(self)",
    ] {
        assert!(owner.contains(required), "{required}");
    }
    assert_eq!(
        guardians.matches("self.binding_for_placement()?").count(),
        2
    );
    for forbidden in [
        "O_WRONLY",
        "O_RDWR",
        "write_at",
        "sendmsg",
        "clock_gettime",
        "CLOCK_BOOTTIME",
        "Command::",
        "LOCK_UN",
        "StartTransientUnit",
        "enroll_once(",
    ] {
        for source in [&kernel, &owner, &guardians] {
            assert!(!source.contains(forbidden), "{forbidden}");
        }
    }
}
