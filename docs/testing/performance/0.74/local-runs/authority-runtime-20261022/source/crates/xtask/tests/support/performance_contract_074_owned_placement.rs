//! Exact additive same-owner join; old validation bodies and wire stay frozen.
#[path = "performance_contract_074_placement_preparation.rs"]
mod preparation;
const BASE: &str = "f4d7f44c";
const PREFIX: &str = "tools/long-run-supervisor-074/src/";
const HOOKS: &[(&str, &str)] = &[
    ("diagnostic_owned_root_manager.rs", "\n#[cfg(test)]\n#[path = \"diagnostic_owned_placement_tests.rs\"]\nmod placement_tests;\n\n#[path = \"diagnostic_owned_placement.rs\"]\nmod placement;\n"),
    ("diagnostic_root_guardian_original_kernel.rs", "\n#[path = \"diagnostic_root_placement_binding.rs\"]\nmod placement;\n"),
    ("diagnostic_privileged_root_frame_tests.rs", "\n#[path = \"diagnostic_owned_placement_live_tests.rs\"]\nmod owned_placement;\n"),
];
const GATE: &str = r##"
# BEGIN OWNED PLACEMENT GATE
[[gate]]
id = "tool.hydracache-long-run-supervisor-074.owned-placement-live"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_owned_placement_live_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "privileged_owned_placement_live_join"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "manual local signed constraints under same original manager/root; no send or product"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","privileged_owned_placement_live_join","--","--ignored","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"
[gate.command.env]
# END OWNED PLACEMENT GATE
"##;
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let mut text = preparation::normalize(path, raw);
    for (file, hook) in HOOKS {
        if path == format!("{PREFIX}{file}") {
            assert_eq!(
                text.matches(hook).count(),
                1,
                "exact owned placement hook: {path}"
            );
            text = text.replacen(hook, "", 1);
        }
    }
    if path == "docs/testing/gated-test-registry.toml" {
        assert_eq!(text.matches(GATE).count(), 1, "exact owned placement gate");
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
fn owned_placement_preserves_all_425_prior_sources_wire_and_external_harness() {
    let files = git(&[
        "ls-tree",
        "-r",
        "--name-only",
        BASE,
        "--",
        "tools/long-run-supervisor-074/src",
    ]);
    assert_eq!(files.lines().count(), 425);
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
fn owned_placement_adapter_refuses_missing_duplicate_and_original_body_drift() {
    for (path, hook) in HOOKS
        .iter()
        .map(|(file, hook)| (format!("{PREFIX}{file}"), *hook))
        .chain([("docs/testing/gated-test-registry.toml".into(), GATE)])
    {
        let raw = read(&path);
        for changed in [raw.replacen(hook, "", 1), format!("{raw}{hook}")] {
            assert!(std::panic::catch_unwind(|| normalize(&path, &changed)).is_err());
        }
        let changed = raw.replacen('\n', "\n// original body drift\n", 1);
        assert_ne!(
            normalize(&path, &changed),
            git(&["show", &format!("{BASE}:{path}")])
        );
    }
}
#[test]
fn owned_placement_reuses_original_round_key_and_retirement_without_execution() {
    let body = read(&format!("{PREFIX}diagnostic_owned_placement.rs"));
    for required in [
        "controller.controller.source.policy.verifying_key",
        "prepared.consent_to_sign()",
        "self.constraints()?;\n        self.owner.full_round()?;\n        self.constraints()",
        "join(",
        "controls.binding_for_placement()? != root",
        "retire(owner, controls)",
        "held: OwnedRootGuardians",
        "held: Box<OwnedRootGuardians",
        "raw: Vec<u8>",
        "Some(&self.snapshot)",
    ] {
        assert!(body.contains(required), "{required}");
    }
    for forbidden in [
        "unsafe",
        "File::",
        "Command::",
        "sendmsg",
        "StartTransientUnit",
        "LOCK_UN",
        "pub(crate) fn start",
        "pub(crate) fn release",
        "pub(crate) fn enroll",
        "pub(crate) fn descriptors",
        "impl Drop for OwnedPlacement",
    ] {
        assert!(!body.contains(forbidden), "{forbidden}");
    }
    let root = read(&format!("{PREFIX}diagnostic_root_placement_binding.rs"));
    for required in [
        "self.observe()?",
        "!self.pair.consumed",
        "!self.pair.armed",
        "self.pair.refused",
        "self.pair.frame.enroll",
        "self.pair.root.identity.device",
        "self.pair.root.identity.inode",
        "self.refuse()",
    ] {
        assert!(root.contains(required), "{required}");
    }
    for forbidden in ["unsafe", "open(", "read_link", "from_raw_fd", "kill("] {
        assert!(!root.contains(forbidden), "{forbidden}");
    }
    let units = read(&format!("{PREFIX}diagnostic_owned_placement_tests.rs"));
    for required in [
        "0x074e_6a1d",
        "0..128",
        "paths.len(), 36",
        "no descriptor extraction",
        "binding must never reenroll",
    ] {
        assert!(units.contains(required), "{required}");
    }
    let live = read(&format!("{PREFIX}diagnostic_owned_placement_live_tests.rs"));
    for required in [
        "retained_context_refuses_for_test(campaigns)",
        "fence_held_while_owned=true",
        "timer_live_before_drop=true",
        "error.placement_timer_live_for_fixture()",
        "assert!(!cgroup::drain(&mut out))",
        "case != \"unarmed\"",
    ] {
        assert!(live.contains(required), "{required}");
    }
}
