//! Only new original-consent child registrations are removed for historical gates.
const GATE: &str = "\n#[path = \"support/performance_contract_074_original_start_consent.rs\"]\nmod original_start_consent;\n";
const ADAPTER: &str = "    let current = super::original_start_consent::normalize(path, raw);\n    let raw = current.as_str();\n";
const OLD_READ: &str = r#"    let read = |p: &str| {
        std::fs::read_to_string(super::root().join(p))
            .unwrap()
            .replace('\r', "")
    };
"#;
const NEW_READ: &str = r#"    let read = |p: &str| {
        let raw = std::fs::read_to_string(super::root().join(p))
            .unwrap()
            .replace('\r', "");
        super::original_start_consent::normalize(p, &raw)
    };
"#;
const OLD_ASSERT: &str =
    "        assert_eq!(read(p), old(p), \"original guard/input changed: {p}\");\n";
const NEW_ASSERT: &str = r#"        assert_eq!(
            super::original_start_consent::normalize(p, &read(p)),
            old(p),
            "original guard/input changed: {p}"
        );
"#;
const INTENT_ADAPTER: &str =
    "        current = super::original_start_consent::normalize(path, &current);\n";
const OLD_PROGRESS_READ: &str = r#"    let read = |file: &str| {
        std::fs::read_to_string(
            super::root().join(format!("tools/long-run-supervisor-074/src/{file}")),
        )
        .unwrap()
        .replace('\r', "")
    };
"#;
const NEW_PROGRESS_READ: &str = r#"    let read = |file: &str| {
        let raw = std::fs::read_to_string(
            super::root().join(format!("tools/long-run-supervisor-074/src/{file}")),
        )
        .unwrap()
        .replace('\r', "");
        super::original_start_consent::normalize(
            &format!("tools/long-run-supervisor-074/src/{file}"),
            &raw,
        )
    };
"#;
const REGISTRATIONS: &[(&str, &str)] = &[
    (
        "tools/long-run-supervisor-074/src/diagnostic_collector_causal.rs",
        "#[path = \"diagnostic_original_start_consent.rs\"]\npub mod start_consent;\n\n",
    ),
    (
        "tools/long-run-supervisor-074/src/diagnostic_collector_progress_tests.rs",
        "#[path = \"diagnostic_original_start_consent_tests.rs\"]\nmod start_consent_tests;\n\n",
    ),
    (
        "tools/long-run-supervisor-074/src/diagnostic_kernel_collector_inputs.rs",
        "#[path = \"diagnostic_kernel_start_consent.rs\"]\nmod start_consent;\n\n",
    ),
    (
        "tools/long-run-supervisor-074/src/diagnostic_worker_kernel_binding.rs",
        "#[path = \"diagnostic_worker_start_consent.rs\"]\nmod start_consent;\n\n",
    ),
];
pub(super) const REGISTRY: &str = r#"
# BEGIN ORIGINAL START CONSENT GATE
[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-start-consent-live-bindings-and-refusal-keep-fence"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_start_consent_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_start_consent_live_bindings_and_refusal_keep_fence"
cfg = 'target_os = "linux"'
env = ""
reason = "Manual nonroot original receiver consent, frozen bindings and retained fence; no execution"
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
args = ["test", "--config", "profile.dev.package.ed25519-dalek.opt-level=3", "--config", "profile.dev.package.curve25519-dalek.opt-level=3", "-p", "hydracache-long-run-supervisor-074", "--lib", "--locked", "diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::original_start_consent_live_bindings_and_refusal_keep_fence", "--", "--ignored", "--exact", "--nocapture"]
cwd = "."
platform = "linux"

[gate.command.env]
# END ORIGINAL START CONSENT GATE
"#;
fn replace_once(raw: &str, new: &str, old: &str, path: &str) -> String {
    assert_eq!(raw.matches(new).count(), 1, "unapproved adapter: {path}");
    raw.replacen(new, old, 1)
}
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let current = super::consent_budget::normalize(path, raw);
    let raw = current.as_str();
    if let Some((_, registration)) = REGISTRATIONS.iter().find(|(p, _)| *p == path) {
        replace_once(raw, registration, "", path)
    } else if path == "crates/xtask/tests/performance_contract_074.rs" {
        replace_once(raw, GATE, "", path)
    } else if path == "docs/testing/gated-test-registry.toml" {
        replace_once(raw, REGISTRY, "", path)
    } else if path == "crates/xtask/tests/support/performance_contract_074_start_consent.rs" {
        replace_once(
            &replace_once(raw, ADAPTER, "", path),
            NEW_ASSERT,
            OLD_ASSERT,
            path,
        )
    } else if path == "crates/xtask/tests/support/performance_contract_074_collector_reserved.rs" {
        replace_once(raw, NEW_READ, OLD_READ, path)
    } else if path == "crates/xtask/tests/support/performance_contract_074_collector_intent.rs" {
        replace_once(raw, INTENT_ADAPTER, "", path)
    } else if path == "crates/xtask/tests/support/performance_contract_074_collector_progress.rs" {
        replace_once(raw, NEW_PROGRESS_READ, OLD_PROGRESS_READ, path)
    } else {
        raw.into()
    }
}
#[test]
fn original_start_consent_keeps_historical_guards_and_has_no_execution_conversion() {
    let read = |p: &str| {
        std::fs::read_to_string(super::root().join(p))
            .unwrap()
            .replace('\r', "")
    };
    let old = |p: &str| {
        let out = std::process::Command::new("git")
            .args(["show", &format!("287cd72a:{p}")])
            .current_dir(super::root())
            .output()
            .unwrap();
        assert!(out.status.success());
        String::from_utf8(out.stdout).unwrap().replace('\r', "")
    };
    for p in REGISTRATIONS.iter().map(|(p, _)| *p).chain([
        "crates/xtask/tests/performance_contract_074.rs",
        "crates/xtask/tests/support/performance_contract_074_start_consent.rs",
        "crates/xtask/tests/support/performance_contract_074_collector_reserved.rs",
        "crates/xtask/tests/support/performance_contract_074_collector_intent.rs",
        "crates/xtask/tests/support/performance_contract_074_collector_progress.rs",
        "docs/testing/gated-test-registry.toml",
    ]) {
        let raw = read(p);
        assert_eq!(normalize(p, &raw), old(p), "old predicates changed: {p}");
        assert_ne!(normalize(p, &format!("{raw}\nfn extra() {{}}\n")), old(p));
    }
    assert!(std::panic::catch_unwind(|| normalize(
        REGISTRATIONS[0].0,
        &read(REGISTRATIONS[0].0).replace("pub mod start_consent;", "pub mod unchecked;")
    ))
    .is_err());
    for p in [
        "tools/long-run-supervisor-074/src/diagnostic_start_consent.rs",
        "tools/long-run-supervisor-074/src/diagnostic_transition.rs",
        "tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs",
        "tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs",
        "tools/long-run-supervisor-074/src/diagnostic_kernel_progress.rs",
        "tools/long-run-supervisor-074/src/diagnostic_worker_progress.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_reserved.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_reservation_storage.rs",
        "docs/COMPAT.md",
        "docs/testing/performance/0.74/qualification-manifest.toml",
    ] {
        assert_eq!(read(p), old(p), "out-of-scope: {p}");
    }
    let source = read("tools/long-run-supervisor-074/src/diagnostic_original_start_consent.rs");
    for marker in [
        "self.original.revalidate()?",
        "validate_transition(&r.state, &state)",
        "controller_authority_binding()?",
        "sha256_hex(self.original.original.packet.bytes())",
        "equal_body(self.prepared.consent_to_sign(), &body)",
        "self.prepared.revalidate()?",
        "self.packet.latch_refusal()",
    ] {
        assert!(source.contains(marker), "{marker}");
    }
    let runtime = source
        .lines()
        .filter(|l| !l.trim_start().starts_with("///"))
        .collect::<Vec<_>>()
        .join("\n");
    for forbidden in [
        "pub fn start_once",
        "pub fn publish",
        "std::fs",
        "std::process",
        "SystemTime",
        "impl Clone",
        "impl Drop",
    ] {
        assert!(!runtime.contains(forbidden), "{forbidden}");
    }
}
