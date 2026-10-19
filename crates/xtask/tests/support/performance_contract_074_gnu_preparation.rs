//! Test-only clock facts cannot change the frozen original validation bodies.
const BASE: &str = "e9aff331";
const GATE: &str = r#"
# BEGIN GNU PREPARATION GATE

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.gnu-preparation-attribution"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_gnu_preparation_cost_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "gnu_preparation_cost_attribution"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "manual nonroot unchanged preparation cost and causal clock facts; not healthy admission"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","gnu_preparation_cost_attribution","--","--ignored","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

# END GNU PREPARATION GATE
"#;
const ADDITIONS: &[(&str, &str)] = &[
    ("tools/long-run-supervisor-074/src/diagnostic_collector_causal.rs",
     "    #[cfg(test)]\n    time_observation::observe(\"window\", issued, &now);\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_collector_causal.rs",
     "\n#[cfg(test)]\n#[path = \"diagnostic_causal_time_observation.rs\"]\npub(crate) mod time_observation;\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_validation_frame.rs",
     "        #[cfg(test)]\n        super::super::time_observation::observe(\"final_clocks\", &causal.issued, &now);\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_original_root_guardian_frame_tests.rs",
     "\n#[path = \"diagnostic_gnu_preparation_cost_tests.rs\"]\nmod preparation_cost;\n"),
];
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let mut raw = raw.to_owned();
    for (_, extra) in ADDITIONS.iter().filter(|(p, _)| *p == path) {
        assert_eq!(
            raw.matches(extra).count(),
            1,
            "preparation addition: {path}"
        );
        raw = raw.replacen(extra, "", 1);
    }
    if path == "docs/testing/gated-test-registry.toml" {
        assert_eq!(
            raw.matches(GATE).count(),
            1,
            "exact attribution registration"
        );
        return raw.replacen(GATE, "", 1);
    }
    raw
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
fn gnu_preparation_preserves_every_previous_body_and_protocol() {
    let listing = git(&[
        "ls-tree",
        "-r",
        "--name-only",
        BASE,
        "--",
        "tools/long-run-supervisor-074/src",
    ]);
    let files: Vec<_> = listing.lines().filter(|p| p.ends_with(".rs")).collect();
    assert_eq!(files.len(), 402);
    for path in files.into_iter().chain([
        "docs/COMPAT.md",
        "docs/testing/gated-test-registry.toml",
        "tools/long-run-supervisor-074/tests/prearmed_watchdog.py",
        "tools/long-run-supervisor-074/tests/redundant_watchdog.py",
        "tools/long-run-supervisor-074/tests/root_creator_sigkill.py",
    ]) {
        assert_eq!(
            normalize(path, &read(path)),
            git(&["show", &format!("{BASE}:{path}")]),
            "{path}"
        );
    }
}
#[test]
fn gnu_preparation_exact_hooks_reject_deletion_duplication_and_body_drift() {
    for (path, extra) in ADDITIONS.iter().chain(std::iter::once(&(
        "docs/testing/gated-test-registry.toml",
        GATE,
    ))) {
        let raw = read(path);
        assert!(std::panic::catch_unwind(|| normalize(path, &raw.replacen(extra, "", 1))).is_err());
        assert!(std::panic::catch_unwind(|| normalize(path, &format!("{raw}{extra}"))).is_err());
        assert_ne!(
            normalize(path, &format!("{raw}/* changed guard */")),
            git(&["show", &format!("{BASE}:{path}")])
        );
    }
}
#[test]
fn gnu_preparation_observations_have_no_clock_or_execution_authority() {
    let raw = read("tools/long-run-supervisor-074/src/diagnostic_causal_time_observation.rs");
    for forbidden in [
        "clock_gettime",
        "timerfd",
        "unsafe",
        "KernelTimePoint {",
        "Command::",
        "LOCK_UN",
    ] {
        assert!(!raw.contains(forbidden), "{forbidden}");
    }
    assert!(raw.contains("issued.boot_id()"));
    assert!(raw.contains("now.boot_id()"));
    assert!(raw.contains("CONTROLLER_LOSS_SECONDS * 1_000_000_000"));
    let registry = read("docs/testing/gated-test-registry.toml");
    let extra = registry
        .split_once("# BEGIN GNU PREPARATION GATE")
        .unwrap()
        .1;
    assert_eq!(extra.matches("[[gate]]").count(), 1);
    assert!(extra.contains("ship_mandatory = false"));
    assert!(extra.contains("gnu_preparation_cost_attribution"));
}
