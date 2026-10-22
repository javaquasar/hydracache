//! Strip only exact test instrumentation; all old statements remain byte-exact.
#[path = "performance_contract_074_kernel_walk.rs"]
mod kernel_walk;
const BASE: &str = "b2adb5b73a2f66fc690637260c100b5c8489aa35";
const INSERTIONS: &[(&str, &str)] = &[
    ("crates/xtask/tests/performance_contract_074.rs", "\n#[path = \"support/performance_contract_074_live_walk.rs\"]\nmod live_walk;\n"),
    ("crates/xtask/tests/performance_contract_074.rs", "    let source = live_walk::normalize(\n        \"tools/long-run-supervisor-074/src/diagnostic_worker_context.rs\",\n        &source.replace('\\r', \"\"),\n    );\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_worker_kernel_binding.rs", "        #[cfg(test)]\n        let _live_walk = crate::diagnostic_ipc::validation_trace::live_walk::span(\n            crate::diagnostic_ipc::validation_trace::live_walk::Kind::KernelWorker,\n        );\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_worker_trust_input.rs", "    #[cfg(test)]\n    let _live_walk = crate::diagnostic_ipc::validation_trace::live_walk::span(\n        crate::diagnostic_ipc::validation_trace::live_walk::Kind::ActiveAuthority,\n    );\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_worker_trust_input.rs", "        #[cfg(test)]\n        let _live_walk = crate::diagnostic_ipc::validation_trace::live_walk::span(\n            crate::diagnostic_ipc::validation_trace::live_walk::Kind::WorkerSource,\n        );\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_worker_trust_input.rs", "        #[cfg(test)]\n        let _live_walk = crate::diagnostic_ipc::validation_trace::live_walk::span(\n            crate::diagnostic_ipc::validation_trace::live_walk::Kind::WorkerPolicy,\n        );\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs", "        #[cfg(test)]\n        let _live_walk = crate::diagnostic_ipc::validation_trace::live_walk::span(\n            crate::diagnostic_ipc::validation_trace::live_walk::Kind::KernelContext,\n        );\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_worker_context.rs", "        #[cfg(test)]\n        let _live_walk = crate::diagnostic_ipc::validation_trace::live_walk::span(\n            crate::diagnostic_ipc::validation_trace::live_walk::Kind::LocalContext,\n        );\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_validation_trace.rs", "\n#[cfg(target_os = \"linux\")]\n#[path = \"diagnostic_live_walk_trace.rs\"]\npub(crate) mod live_walk;\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_controller_component_cost_tests.rs", "\n#[path = \"diagnostic_original_live_walk_cost_tests.rs\"]\nmod live_walk_tests;\n"),
];
const GATE: &str = r###"
# BEGIN ORIGINAL LIVE WALK GATE

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-live-worker-walk"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_live_walk_cost_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_live_worker_walk_attribution"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "manual nonroot original live worker walk attribution; no reservation or execution"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::leaf_cost_tests::component_cost_tests::live_walk_tests::original_live_worker_walk_attribution","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

# END ORIGINAL LIVE WALK GATE
"###;
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let current = kernel_walk::normalize(path, raw);
    let raw = current.as_str();
    let mut clean = raw.to_owned();
    for &(changed, exact) in INSERTIONS {
        if path == changed {
            assert_eq!(
                clean.matches(exact).count(),
                1,
                "live walk insertion drift: {path}"
            );
            clean = clean.replacen(exact, "", 1);
        }
    }
    if path == "crates/xtask/tests/support/performance_contract_074_safety_alarm.rs" {
        let exact = "crate::live_walk::normalize(path, &current.replace('\\r', \"\")),";
        assert_eq!(clean.matches(exact).count(), 1);
        clean = clean.replacen(exact, "current.replace('\\r', \"\"),", 1);
    }
    if path == "docs/testing/gated-test-registry.toml" {
        assert_eq!(clean.matches(GATE).count(), 1);
        clean = clean.replacen(GATE, "", 1);
    }
    clean
}
fn root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn read(path: &str) -> String {
    crate::runtime_io::read_to_string(root().join(path))
        .unwrap()
        .replace('\r', "")
}
fn old(path: &str) -> String {
    let out = std::process::Command::new("git")
        .args(["show", &format!("{BASE}:{path}")])
        .current_dir(root())
        .output()
        .unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout).unwrap().replace('\r', "")
}
#[test]
fn live_walk_spans_preserve_every_old_rust_body_after_exact_test_only_removal() {
    let out = std::process::Command::new("git")
        .args([
            "ls-tree",
            "-r",
            "--name-only",
            BASE,
            "--",
            "tools/long-run-supervisor-074/src",
        ])
        .current_dir(root())
        .output()
        .unwrap();
    assert!(out.status.success());
    let raw = String::from_utf8(out.stdout).unwrap();
    let paths: Vec<_> = raw.lines().filter(|p| p.ends_with(".rs")).collect();
    assert_eq!(paths.len(), 349);
    for path in paths {
        assert_eq!(normalize(path, &read(path)), old(path), "{path}");
    }
    assert_eq!(
        normalize(
            "docs/testing/gated-test-registry.toml",
            &read("docs/testing/gated-test-registry.toml")
        ),
        old("docs/testing/gated-test-registry.toml")
    );
}
#[test]
fn live_walk_equivalence_detects_every_changed_original_statement() {
    for &(path, exact) in INSERTIONS {
        let raw = read(path);
        let restored = normalize(path, &raw);
        assert_eq!(restored, old(path));
        let changed = raw.replacen(exact, &format!("{exact}/* guard schedule changed */\n"), 1);
        assert_ne!(normalize(path, &changed), restored);
        let missing = raw.replacen(exact, "", 1);
        assert!(std::panic::catch_unwind(|| normalize(path, &missing)).is_err());
    }
}
