//! Exact approved test-only instrumentation; no runtime guard normalization.
const BASELINE: &str = "88a623f6";
const HOOKS: [(&str, &str); 8] = [
    (
        "diagnostic_ipc.rs",
        r#"#[cfg(test)]
#[path = "diagnostic_validation_trace.rs"]
pub(crate) mod validation_trace;
#[cfg(test)]
#[path = "diagnostic_validation_trace_tests.rs"]
mod validation_trace_tests;

"#,
    ),
    (
        "diagnostic_ipc.rs",
        r#"    #[cfg(test)]
    let _authentication_trace = validation_trace::authentication();
"#,
    ),
    (
        "diagnostic_ipc.rs",
        r#"    #[cfg(test)]
    let signature_trace = validation_trace::signature();
"#,
    ),
    (
        "diagnostic_ipc.rs",
        r#"    #[cfg(test)]
    drop(signature_trace);
"#,
    ),
    (
        "diagnostic_ipc.rs",
        r#"        #[cfg(test)]
        validation_trace::time_refusal(
            r.operation,
            r.issued_at_unix_seconds,
            r.expires_at_unix_seconds,
            now_unix_seconds,
        );
"#,
    ),
    (
        "diagnostic_worker_policy.rs",
        r#"    #[cfg(test)]
    let _worker_policy_trace = crate::diagnostic_ipc::validation_trace::worker_policy();
"#,
    ),
    (
        "diagnostic_kernel_refusal_trace.rs",
        r#"    let _validation = crate::diagnostic_ipc::validation_trace::ambient();
"#,
    ),
    (
        "diagnostic_collector_progress_tests.rs",
        r#"#[path = "diagnostic_validation_cost_tests.rs"]
mod validation_cost_tests;
"#,
    ),
];
pub(super) fn without_instrumentation(path: &str, raw: &str) -> String {
    let mut raw = super::worker_proof::restore_worker(path, raw);
    for (file, hook) in HOOKS {
        if path.ends_with(file) {
            assert_eq!(
                raw.matches(hook).count(),
                1,
                "{file}: approved hook missing/duplicated"
            );
            raw = raw.replacen(hook, "", 1);
        }
    }
    raw
}
pub(super) const REGISTRY_ADDITION: &str = r#"
# BEGIN VALIDATION COST PROBE GATE
[[gate]]
id = "tool.hydracache-long-run-supervisor-074.validation-cost-original-stages-preserve-guards-and-files"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_validation_cost_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "validation_cost_original_stages_preserve_guards_and_files"
cfg = 'target_os = "linux"'
env = ""
reason = "Manual nonroot debug fixture stage/call/signature cost attribution; no production optimization or execution"
tier = "manual"
required_features = []
required_env = []
required_tools = ["cargo"]
timeout_seconds = 900
owner_release = "0.74.0"
ship_mandatory = false
artifacts = []

[gate.ci]
workflow = ".github/workflows/ci.yml"
job = "gated-proof-registry"
step = "Run registered gated proofs"

[gate.command]
program = "cargo"
args = ["test", "-p", "hydracache-long-run-supervisor-074", "--lib", "--locked", "diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::validation_cost_tests::validation_cost_original_stages_preserve_guards_and_files", "--", "--ignored", "--exact", "--nocapture"]
cwd = "."
platform = "linux"

[gate.command.env]
# END VALIDATION COST PROBE GATE
"#;
pub(super) fn without_registry_addition(raw: &str) -> String {
    let raw = super::worker_proof::restore_reason(raw);
    assert_eq!(raw.matches(REGISTRY_ADDITION).count(), 1);
    raw.replacen(REGISTRY_ADDITION, "", 1)
}
#[test]
fn validation_cost_exact_test_only_hooks_preserve_all_original_predicates() {
    let read = |p: &str| {
        std::fs::read_to_string(super::root().join(p))
            .unwrap()
            .replace('\r', "")
    };
    let old = |p: &str| {
        let out = std::process::Command::new("git")
            .args(["show", &format!("{BASELINE}:{p}")])
            .current_dir(super::root())
            .output()
            .unwrap();
        assert!(out.status.success());
        String::from_utf8(out.stdout).unwrap().replace('\r', "")
    };
    for file in [
        "diagnostic_ipc.rs",
        "diagnostic_worker_policy.rs",
        "diagnostic_kernel_refusal_trace.rs",
        "diagnostic_collector_progress_tests.rs",
    ] {
        let p = format!("tools/long-run-supervisor-074/src/{file}");
        let current = read(&p);
        let stripped = without_instrumentation(&p, &current);
        assert_eq!(stripped, old(&p), "old predicates changed: {file}");
        let changed = format!("{current}\nfn unapproved_tail() {{}}\n");
        assert_ne!(without_instrumentation(&p, &changed), old(&p));
        if file == "diagnostic_ipc.rs" {
            let future = current.replacen(
                "now_unix_seconds < r.issued_at_unix_seconds",
                "now_unix_seconds <= r.issued_at_unix_seconds",
                1,
            );
            let expiry = current.replacen(
                "now_unix_seconds >= r.expires_at_unix_seconds",
                "now_unix_seconds > r.expires_at_unix_seconds",
                1,
            );
            assert_ne!(without_instrumentation(&p, &future), old(&p));
            assert_ne!(without_instrumentation(&p, &expiry), old(&p));
        }
    }
    let registry = "docs/testing/gated-test-registry.toml";
    assert_eq!(without_registry_addition(&read(registry)), old(registry));
    for p in [
        "Cargo.lock",
        "docs/COMPAT.md",
        "docs/testing/performance/0.74/qualification-manifest.toml",
        "tools/long-run-supervisor-074/src/diagnostic_collector_progress.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_causal.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_reserved.rs",
        "tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs",
        "tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs",
        "tools/long-run-supervisor-074/src/diagnostic_worker_kernel_binding.rs",
        "tools/long-run-supervisor-074/src/diagnostic_worker_progress.rs",
        "tools/long-run-supervisor-074/src/diagnostic_oidc_input.rs",
        "tools/long-run-supervisor-074/src/diagnostic_controller_build_input.rs",
        "tools/long-run-supervisor-074/src/diagnostic_controller_trust_input.rs",
    ] {
        assert_eq!(read(p), old(p), "out of scope mutation: {p}");
    }
    let trace = read("tools/long-run-supervisor-074/src/diagnostic_validation_trace.rs");
    for forbidden in [
        "DiagnosticRequest",
        "SignedDiagnosticRequest",
        "SigningKey",
        "KernelPacket",
        "fs::",
        "libc::",
        "SystemTime",
        "pub fn",
    ] {
        assert!(!trace.contains(forbidden), "{forbidden}");
    }
    assert!(trace.contains("try_borrow_mut") && trace.contains("std::thread::panicking()"));
    assert!(trace.contains("[Stats; 8]"));
    let probe = read("tools/long-run-supervisor-074/src/diagnostic_validation_cost_tests.rs");
    assert!(probe.contains(
        "#[ignore = \"manual nonroot original validation cost attribution; no server\"]"
    ));
    assert!(!probe.contains("sleep") && !probe.contains("test-threads"));
    println!("validation_cost_equivalence test_only=true old_guards=true renewal=false production_optimization=false starting=false");
}
