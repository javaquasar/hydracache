//! Additive observation fixture; historical runtime remains exact.
const GATE: &str =
    "\n#[path = \"support/performance_contract_074_consent_budget.rs\"]\nmod consent_budget;\n";
const ADAPTER: &str = "    let current = super::consent_budget::normalize(path, raw);\n    let raw = current.as_str();\n";
const REGISTRY: &str = r#"
# BEGIN CONSENT BUDGET GATE
[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-consent-publication-budget"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_consent_budget_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_consent_publication_budget"
cfg = 'target_os = "linux"'
env = ""
reason = "Manual nonroot publication-free budget attribution; refusal is retained, not admission"
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
args = ["test", "--config", "profile.dev.package.ed25519-dalek.opt-level=3", "--config", "profile.dev.package.curve25519-dalek.opt-level=3", "-p", "hydracache-long-run-supervisor-074", "--lib", "--locked", "diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::budget_tests::original_consent_publication_budget", "--", "--ignored", "--exact", "--nocapture"]
cwd = "."
platform = "linux"

[gate.command.env]
# END CONSENT BUDGET GATE
"#;
const OWNER_REGISTRY: &str = r#"
# BEGIN CONSENT OWNER ATTRIBUTION GATE
[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-consent-validation-owner-attribution"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_consent_budget_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_consent_validation_owner_attribution"
cfg = 'target_os = "linux"'
env = ""
reason = "Manual nonroot inclusive owner cost attribution; no publication or admission"
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
args = ["test", "--config", "profile.dev.package.ed25519-dalek.opt-level=3", "--config", "profile.dev.package.curve25519-dalek.opt-level=3", "-p", "hydracache-long-run-supervisor-074", "--lib", "--locked", "diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::budget_tests::original_consent_validation_owner_attribution", "--", "--ignored", "--exact", "--nocapture"]
cwd = "."
platform = "linux"

[gate.command.env]
# END CONSENT OWNER ATTRIBUTION GATE
"#;
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let extra = match path {
        "crates/xtask/tests/performance_contract_074.rs" => GATE,
        "docs/testing/gated-test-registry.toml" => REGISTRY,
        _ => return raw.into(),
    };
    assert_eq!(
        raw.matches(extra).count(),
        1,
        "unapproved budget adapter: {path}"
    );
    let clean = raw.replacen(extra, "", 1);
    if path == "docs/testing/gated-test-registry.toml" {
        assert_eq!(clean.matches(OWNER_REGISTRY).count(), 1);
        clean.replacen(OWNER_REGISTRY, "", 1)
    } else {
        clean
    }
}
#[test]
fn consent_budget_is_additive_and_keeps_original_runtime_and_deadlines() {
    let read = |path: &str| {
        std::fs::read_to_string(super::root().join(path))
            .unwrap()
            .replace('\r', "")
    };
    let old = |path: &str| {
        let out = std::process::Command::new("git")
            .args(["show", &format!("22c79406:{path}")])
            .current_dir(super::root())
            .output()
            .unwrap();
        assert!(out.status.success());
        String::from_utf8(out.stdout).unwrap().replace('\r', "")
    };
    for path in [
        "tools/long-run-supervisor-074/src/diagnostic_original_start_consent.rs",
        "tools/long-run-supervisor-074/src/diagnostic_kernel_start_consent.rs",
        "tools/long-run-supervisor-074/src/diagnostic_worker_start_consent.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_reserved.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_reservation_storage.rs",
        "tools/long-run-supervisor-074/src/diagnostic_kernel_progress.rs",
        "tools/long-run-supervisor-074/src/diagnostic_transition.rs",
        "docs/testing/performance/0.74/qualification-manifest.toml",
    ] {
        assert_eq!(read(path), old(path), "budget changed runtime: {path}");
    }
    for path in [
        "crates/xtask/tests/performance_contract_074.rs",
        "docs/testing/gated-test-registry.toml",
    ] {
        let raw = read(path);
        assert_eq!(normalize(path, &raw), old(path));
        assert_ne!(
            normalize(path, &format!("{raw}\n# unapproved\n")),
            old(path)
        );
    }
    let path = "crates/xtask/tests/support/performance_contract_074_original_start_consent.rs";
    let raw = read(path);
    assert_eq!(raw.matches(ADAPTER).count(), 1);
    assert_eq!(raw.replacen(ADAPTER, "", 1), old(path));
    let path = "tools/long-run-supervisor-074/src/diagnostic_original_start_consent_tests.rs";
    let registration = "#[path = \"diagnostic_consent_budget_tests.rs\"]\nmod budget_tests;\n";
    let raw = read(path);
    assert_eq!(raw.matches(registration).count(), 1);
    assert_eq!(raw.replacen(registration, "", 1), old(path));
    let fixture = read("tools/long-run-supervisor-074/src/diagnostic_consent_budget_tests.rs");
    for marker in [
        "for sample in 0..3",
        "for check in 0..19",
        "accepted.revalidate()",
        "count(), 2",
        "busy(root)",
        "retained_context_refuses_for_test(root)",
    ] {
        assert!(fixture.contains(marker), "{marker}");
    }
}
