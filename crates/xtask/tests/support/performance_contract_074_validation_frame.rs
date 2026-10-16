//! Preserve every original predicate; only the admitted outer composition is new.
const RUNTIME: &str = "#[path = \"diagnostic_validation_frame.rs\"]\nmod validation_frame;\npub use validation_frame::{OriginalFramedConsent, OriginalFramedConsentFailure};\n\n";
const TESTS: &str =
    "#[path = \"diagnostic_validation_frame_tests.rs\"]\nmod validation_frame_tests;\n";
const GATE: &str =
    "\n#[path = \"support/performance_contract_074_validation_frame.rs\"]\nmod validation_frame;\n";
const ADAPTER: &str = "    let current = super::validation_frame::normalize(path, raw);\n    let raw = current.as_str();\n";
const OLD_ASSERT: &str =
    "        assert_eq!(read(path), old(path), \"budget changed runtime: {path}\");\n";
const NEW_ASSERT: &str = r#"        assert_eq!(
            super::validation_frame::normalize(path, &read(path)),
            old(path),
            "budget changed runtime: {path}"
        );
"#;
const OLD_TEST_RAW: &str =
    "    let raw = read(path);\n    assert_eq!(raw.matches(registration).count(), 1);";
const NEW_TEST_RAW: &str = "    let raw = super::validation_frame::normalize(path, &read(path));\n    assert_eq!(raw.matches(registration).count(), 1);";
const REGISTRY: &str = r#"
# BEGIN VALIDATION FRAME BUDGET GATE
[[gate]]
id = "tool.hydracache-long-run-supervisor-074.validation-frame-budget"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_validation_frame_budget_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "validation_frame_publication_budget"
cfg = 'target_os = "linux"'
env = ""
reason = "Manual nonroot original validation frame; no publication or execution"
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
args = ["test","--config","profile.dev.package.ed25519-dalek.opt-level=3","--config","profile.dev.package.curve25519-dalek.opt-level=3","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::budget::validation_frame_publication_budget","--","--ignored","--exact","--nocapture"]
cwd = "."
platform = "linux"

[gate.command.env]
# END VALIDATION FRAME BUDGET GATE

# BEGIN VALIDATION FRAME DRIFT GATE
[[gate]]
id = "tool.hydracache-long-run-supervisor-074.validation-frame-drift"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_validation_frame_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "validation_frame_live_drift_at_every_cut_and_reference_refusal"
cfg = 'target_os = "linux"'
env = ""
reason = "Manual nonroot original validation frame; no publication or execution"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::validation_frame_live_drift_at_every_cut_and_reference_refusal","--","--ignored","--exact","--nocapture"]
cwd = "."
platform = "linux"

[gate.command.env]
# END VALIDATION FRAME DRIFT GATE
"#;
fn once(raw: &str, new: &str, old: &str, path: &str) -> String {
    assert_eq!(
        raw.matches(new).count(),
        1,
        "unapproved frame adapter: {path}"
    );
    raw.replacen(new, old, 1)
}
pub(super) fn normalize(path: &str, raw: &str) -> String {
    match path {
        "tools/long-run-supervisor-074/src/diagnostic_original_start_consent.rs" => {
            once(raw, RUNTIME, "", path)
        }
        "tools/long-run-supervisor-074/src/diagnostic_original_start_consent_tests.rs" => {
            once(raw, TESTS, "", path)
        }
        "crates/xtask/tests/performance_contract_074.rs" => once(raw, GATE, "", path),
        "docs/testing/gated-test-registry.toml" => once(raw, REGISTRY, "", path),
        "crates/xtask/tests/support/performance_contract_074_consent_budget.rs" => {
            let raw = once(raw, ADAPTER, "", path);
            let raw = once(&raw, NEW_ASSERT, OLD_ASSERT, path);
            once(&raw, NEW_TEST_RAW, OLD_TEST_RAW, path)
        }
        _ => raw.into(),
    }
}
#[test]
fn validation_frame_preserves_original_guards_and_frozen_workload() {
    let read = |path: &str| {
        std::fs::read_to_string(super::root().join(path))
            .unwrap()
            .replace('\r', "")
    };
    let old = |path: &str| {
        let out = std::process::Command::new("git")
            .args(["show", &format!("47372f9f:{path}")])
            .current_dir(super::root())
            .output()
            .unwrap();
        assert!(out.status.success());
        String::from_utf8(out.stdout).unwrap().replace('\r', "")
    };
    for path in [
        "tools/long-run-supervisor-074/src/diagnostic_original_start_consent.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_start_consent_tests.rs",
        "crates/xtask/tests/performance_contract_074.rs",
        "crates/xtask/tests/support/performance_contract_074_consent_budget.rs",
        "docs/testing/gated-test-registry.toml",
    ] {
        let raw = read(path);
        assert_eq!(
            normalize(path, &raw),
            old(path),
            "historical guard changed: {path}"
        );
        assert_ne!(
            normalize(path, &format!("{raw}\n// unapproved\n")),
            old(path)
        );
    }
    for path in [
        "Cargo.toml",
        "Cargo.lock",
        "tools/long-run-supervisor-074/Cargo.toml",
        "docs/COMPAT.md",
        "tools/long-run-supervisor-074/src/diagnostic_consent_budget_tests.rs",
        "tools/long-run-supervisor-074/src/diagnostic_controller_request_attempt.rs",
        "tools/long-run-supervisor-074/src/diagnostic_oidc_input.rs",
        "tools/long-run-supervisor-074/src/diagnostic_controller_build_input.rs",
        "tools/long-run-supervisor-074/src/diagnostic_controller_trust_input.rs",
        "tools/long-run-supervisor-074/src/diagnostic_worker_policy.rs",
        "tools/long-run-supervisor-074/src/diagnostic_worker_kernel_binding.rs",
        "tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs",
        "tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs",
        "tools/long-run-supervisor-074/src/diagnostic_kernel_progress.rs",
        "tools/long-run-supervisor-074/src/diagnostic_kernel_start_consent.rs",
        "tools/long-run-supervisor-074/src/diagnostic_worker_progress.rs",
        "tools/long-run-supervisor-074/src/diagnostic_worker_start_consent.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_reserved.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_progress.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_causal.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_reservation_storage.rs",
        "tools/long-run-supervisor-074/src/diagnostic_transition.rs",
        "docs/testing/performance/0.74/qualification-manifest.toml",
    ] {
        assert_eq!(
            read(path),
            old(path),
            "original leaf/workload changed: {path}"
        );
    }
    let path = "tools/long-run-supervisor-074/src/diagnostic_original_start_consent.rs";
    assert!(std::panic::catch_unwind(|| normalize(
        path,
        &read(path).replace("mod validation_frame;", "mod unchecked;")
    ))
    .is_err());
    let frame = read("tools/long-run-supervisor-074/src/diagnostic_validation_frame.rs");
    let prepared_predicate = |raw: &str| {
        let start = raw.find("if body.worker_policy_sha256 != policy").unwrap();
        let raw = &raw[start..];
        let end = ".map_err(|_| OriginalStartConsentError::Binding)?;";
        raw[..raw.find(end).unwrap() + end.len()]
            .lines()
            .map(str::trim)
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert_eq!(
        prepared_predicate(&frame),
        prepared_predicate(&old(
            "tools/long-run-supervisor-074/src/diagnostic_original_start_consent.rs"
        ))
    );
    for marker in [
        "self.round(before).and_then(|_| self.round(|_| {}))",
        "fn final_clocks",
        "r.last = now",
        "original_reservation_time(Some(&r.last))",
        "super::super::check",
        "super::super::window",
        "super::super::age",
        "validate_transition(&r.state, &body.starting_state)",
        "self.original.authenticate()?",
        "StorageBefore",
        "StorageAfter",
        "CONTROLLER_LOSS_SECONDS",
        "TOTAL_SECONDS",
        ".live()",
        ".progress()",
    ] {
        assert!(frame.contains(marker), "{marker}");
    }
    for forbidden in [
        "pub fn start_once",
        "pub fn publish",
        "std::process",
        "std::fs",
        "SystemTime",
        "impl Clone",
        "validated: bool",
    ] {
        assert!(
            !frame
                .lines()
                .filter(|l| !l.trim_start().starts_with("///"))
                .collect::<Vec<_>>()
                .join("\n")
                .contains(forbidden),
            "{forbidden}"
        );
    }
    // Exact source restoration: same three fresh chains and nineteen checks.
    let baseline = old("tools/long-run-supervisor-074/src/diagnostic_consent_budget_tests.rs");
    let start = "#[test]\n#[ignore = \"manual nonroot publication-free original consent budget attribution\"]";
    let baseline = &baseline[baseline.find(start).unwrap()..];
    let candidate =
        read("tools/long-run-supervisor-074/src/diagnostic_validation_frame_budget_tests.rs");
    let candidate = &candidate[candidate.find(start).unwrap()..];
    let with_frame = "let mut accepted = prepared\n                        .accept_consent(packet)\n                        .unwrap()\n                        .into_validation_frame()\n                        .unwrap();";
    let candidate = once(
        candidate,
        with_frame,
        "let mut accepted = prepared.accept_consent(packet).unwrap();",
        "candidate budget",
    )
    .replace(
        "fn validation_frame_publication_budget()",
        "fn original_consent_publication_budget()",
    )
    .replace("HC074-FRAME-BUDGET", "HC074-CONSENT-BUDGET");
    assert_eq!(candidate, baseline);
}
