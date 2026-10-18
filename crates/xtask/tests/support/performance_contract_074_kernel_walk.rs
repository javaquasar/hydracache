//! Exact test-only additions, never removal or relocation of an old guard.
#[path = "performance_contract_074_boot_buffer.rs"]
mod boot_buffer;
const BASE: &str = "be06c237c3c8c3b6cf4e5eb608c90d0f9feaa47c";
const INSERTIONS: &[(&str, &str)] = &[
    ("tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs", "        #[cfg(test)]\n        let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(\n            crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::PacketContext,\n        );\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs", "    #[cfg(test)]\n    let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(\n        crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Document,\n    );\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs", "    #[cfg(test)]\n    let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(\n        crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Projection,\n    );\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs", "        #[cfg(test)]\n        let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(\n            crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Sender,\n        );\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs", "    #[cfg(test)]\n    let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(\n        crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Alive,\n    );\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs", "    #[cfg(test)]\n    let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(\n        crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Namespace,\n    );\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs", "    #[cfg(test)]\n    let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(\n        crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Boot,\n    );\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs", "        #[cfg(test)]\n        let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(\n            crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Origin,\n        );\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs", "        #[cfg(test)]\n        let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(\n            crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Sample,\n        );\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs", "        #[cfg(test)]\n        let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(\n            crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::WorkerOrigin,\n        );\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_validation_trace.rs", "\n#[cfg(target_os = \"linux\")]\n#[path = \"diagnostic_kernel_walk_trace.rs\"]\npub(crate) mod kernel_walk;\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_original_live_walk_cost_tests.rs", "\n#[path = \"diagnostic_original_kernel_walk_cost_tests.rs\"]\nmod kernel_walk_tests;\n"),
];
const GATE: &str = r###"
# BEGIN ORIGINAL KERNEL WALK GATE

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-kernel-walk"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_kernel_walk_cost_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_kernel_context_walk_attribution"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "manual nonroot original kernel walk attribution; no reservation or execution"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::leaf_cost_tests::component_cost_tests::live_walk_tests::kernel_walk_tests::original_kernel_context_walk_attribution","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

# END ORIGINAL KERNEL WALK GATE
"###;
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let mut clean = boot_buffer::normalize(path, raw);
    for &(p, exact) in INSERTIONS {
        if path == p {
            assert_eq!(clean.matches(exact).count(), 1, "kernel span drift: {path}");
            clean = clean.replacen(exact, "", 1);
        }
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
    std::fs::read_to_string(root().join(path))
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
fn kernel_walk_preserves_all_old_rust_sources_after_exact_test_only_removal() {
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
    assert_eq!(paths.len(), 351);
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
fn kernel_walk_exact_removal_cannot_hide_any_changed_statement() {
    for &(path, exact) in INSERTIONS {
        let raw = read(path);
        let restored = normalize(path, &raw);
        assert_eq!(restored, old(path));
        let changed = raw.replacen(exact, &format!("{exact}/* changed live schedule */\n"), 1);
        // A stricter enclosing candidate adapter may refuse the mutation
        // before this instrumentation adapter can restore the old body.
        assert!(std::panic::catch_unwind(|| normalize(path, &changed))
            .map_or(true, |value| value != restored));
        assert!(std::panic::catch_unwind(|| normalize(path, &raw.replacen(exact, "", 1))).is_err());
    }
}
