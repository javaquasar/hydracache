//! Exact test-only phase insertions, not a new runtime schedule.
#[path = "performance_contract_074_fixed_slice.rs"]
mod fixed_slice;
const BASE: &str = "ad0347cb";
const PREFIX: &str = "tools/long-run-supervisor-074/src/";
const FILE: &str = "tools/long-run-supervisor-074/src/diagnostic_owned_placement_live_tests.rs";
const TRACE: &str = "tools/long-run-supervisor-074/src/diagnostic_placement_preparation_trace.rs";
const HOOK: &str =
    "\n#[path = \"diagnostic_placement_preparation_cost.rs\"]\npub(super) mod cost;\n";
const TEST: &str = "\n#[test]\n#[ignore = \"manual local original placement phase cost; never a healthy gate or product\"]\nfn privileged_placement_preparation_cost() {\n    preparation_trace::cost::capture(\"healthy\", || run_original(\"healthy\"));\n}\n";
const GATE: &str = r##"
# BEGIN PLACEMENT COST GATE
[[gate]]
id = "tool.hydracache-long-run-supervisor-074.placement-preparation-cost"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_owned_placement_live_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "privileged_placement_preparation_cost"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "manual local original placement phase cost; not functional gate, send or product"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","privileged_placement_preparation_cost","--","--ignored","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"
[gate.command.env]
# END PLACEMENT COST GATE
"##;
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let mut text = fixed_slice::normalize(path, raw);
    if path == FILE {
        assert_eq!(text.matches(TEST).count(), 1, "exact cost fixture");
        text = text.replacen(TEST, "", 1);
        for i in 0..8 {
            if i == 1 {
                continue;
            }
            let marker = format!("\n            preparation_trace::cost::phase({i});");
            assert_eq!(text.matches(&marker).count(), 1, "exact fixed phase");
            text = text.replacen(&marker, "", 1);
        }
    }
    for (file, exact) in [
        (TRACE, HOOK),
        ("docs/testing/gated-test-registry.toml", GATE),
    ] {
        if path == file {
            assert_eq!(text.matches(exact).count(), 1, "exact cost hook: {file}");
            text = text.replacen(exact, "", 1);
        }
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
fn placement_stage_cost_preserves_all_431_sources_wire_and_harness() {
    let files = git(&[
        "ls-tree",
        "-r",
        "--name-only",
        BASE,
        "--",
        "tools/long-run-supervisor-074/src",
    ]);
    assert_eq!(files.lines().count(), 431);
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
fn placement_stage_cost_adapter_rejects_missing_duplicate_and_changed_original() {
    for (path, hook) in [
        (FILE, TEST),
        (TRACE, HOOK),
        ("docs/testing/gated-test-registry.toml", GATE),
    ] {
        let raw = read(path);
        for bad in [raw.replacen(hook, "", 1), format!("{raw}{hook}")] {
            assert!(std::panic::catch_unwind(|| normalize(path, &bad)).is_err());
        }
        assert_ne!(
            normalize(path, &format!("{raw}\n// original drift\n")),
            git(&["show", &format!("{BASE}:{path}")])
        );
    }
    for i in [0, 2, 3, 4, 5, 6, 7] {
        let raw = read(FILE);
        let marker = format!("preparation_trace::cost::phase({i});");
        assert!(
            std::panic::catch_unwind(|| normalize(FILE, &raw.replacen(&marker, "", 1))).is_err()
        );
    }
}
#[test]
fn placement_stage_cost_is_observational_and_cannot_be_a_healthy_gate() {
    let source = read(&format!("{PREFIX}diagnostic_placement_preparation_cost.rs"));
    for required in [
        "super::inspect(call)",
        "super::resume_unwind(payload)",
        "trace::stage(STAGES[index])",
        "\"healthy_gate\":false",
        "\"guard_clock_added\":false",
        "\"overlapping_spans\":true",
        "\"wall_not_cpu\":true",
        "calls.get(),1",
    ] {
        assert!(
            source.replace(' ', "").contains(&required.replace(' ', "")),
            "{required}"
        );
    }
    for forbidden in [
        "clock_gettime",
        "CLOCK_BOOTTIME",
        "issue_progress_challenge",
        "enroll_once",
        "StartTransientUnit",
        "Command::",
        "LOCK_UN",
    ] {
        assert!(!source.contains(forbidden), "{forbidden}");
    }
}
