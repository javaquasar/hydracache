//! One portable byte contract; every existing original-owner runtime stays exact.
const COMPAT: &str = r#"The opt-in collector start-consent byte artifact uses body
`diagnostic-collector-start-consent-074-v1`, envelope
`diagnostic-signed-collector-start-consent-074-v1` and Ed25519 domain
`hydracache-diagnostic-collector-start-consent-074-v1`, NUL and canonical LF JSON.
Body/envelope ceilings are 4096/8192 bytes. It binds first-cell Starting revision
two, original request/reservation/state/challenge/response and worker/unit hashes,
policy epoch, controller/source/preset and a positive signing window at most
60 seconds. Only matched opt-in library signers/inspectors understand it; old
request, collector-intent and challenge readers refuse rather than reinterpret
it. Unknown, duplicate, future, partial, noncanonical, oversized or wrong-signature
input refuses. Signature inspection with a supplied public key returns untrusted
data, not freshness, enrolled trust, replay, durable lifecycle or start authority.
No installed consumer, migration or automatic downgrade is provided.

"#;
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
        super::start_consent::normalize(p, &raw)
    };
"#;
const GATE: &str =
    "\n#[path = \"support/performance_contract_074_start_consent.rs\"]\nmod start_consent;\n";
const ADAPTERS: &[&str] = &[
    "crates/xtask/tests/support/performance_contract_074_collector_causal.rs",
    "crates/xtask/tests/support/performance_contract_074_validation_cost.rs",
    "crates/xtask/tests/support/performance_contract_074_worker_proof.rs",
    "crates/xtask/tests/support/performance_contract_074_short_fence.rs",
    "crates/xtask/tests/support/performance_contract_074_claim_construction.rs",
];
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let current = super::original_start_consent::normalize(path, raw);
    let raw = current.as_str();
    let (new, old) = if path == "docs/COMPAT.md" {
        (COMPAT, "")
    } else if path == "crates/xtask/tests/performance_contract_074.rs" {
        (GATE, "")
    } else if ADAPTERS.contains(&path) {
        (NEW_READ, OLD_READ)
    } else {
        return raw.into();
    };
    assert_eq!(raw.matches(new).count(), 1, "unapproved adapter: {path}");
    raw.replacen(new, old, 1)
}
#[test]
fn start_consent_is_data_only_and_preserves_all_original_runtime_and_static_guards() {
    let read = |p: &str| {
        std::fs::read_to_string(super::root().join(p))
            .unwrap()
            .replace('\r', "")
    };
    let old = |p: &str| {
        let out = std::process::Command::new("git")
            .args(["show", &format!("fdf0474f:{p}")])
            .current_dir(super::root())
            .output()
            .unwrap();
        assert!(out.status.success());
        String::from_utf8(out.stdout).unwrap().replace('\r', "")
    };
    for p in ADAPTERS.iter().copied().chain([
        "docs/COMPAT.md",
        "crates/xtask/tests/performance_contract_074.rs",
    ]) {
        let raw = read(p);
        assert_eq!(normalize(p, &raw), old(p));
        assert_ne!(
            normalize(p, &format!("{raw}\nfn unapproved_tail() {{}}\n")),
            old(p)
        );
    }
    assert!(std::panic::catch_unwind(|| normalize(
        "docs/COMPAT.md",
        &read("docs/COMPAT.md").replace("4096/8192", "8192/16384")
    ))
    .is_err());
    let p = "tools/long-run-supervisor-074/src/lib.rs";
    let raw = super::live_walk::normalize(p, &read(p));
    let registration = "pub mod diagnostic_start_consent;\n";
    assert_eq!(raw.matches(registration).count(), 1);
    assert_eq!(raw.replacen(registration, "", 1), old(p));
    for p in [
        "Cargo.toml",
        "Cargo.lock",
        "tools/long-run-supervisor-074/Cargo.toml",
        "docs/testing/gated-test-registry.toml",
        "docs/testing/performance/0.74/qualification-manifest.toml",
        "tools/long-run-supervisor-074/src/host_execution.rs",
        "tools/long-run-supervisor-074/src/diagnostic_ipc.rs",
        "tools/long-run-supervisor-074/src/diagnostic_lease.rs",
        "tools/long-run-supervisor-074/src/diagnostic_transition.rs",
        "tools/long-run-supervisor-074/src/diagnostic_worker_policy.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_intent.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_causal.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_causal_tests.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_reserved.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_reservation_storage.rs",
        "tools/long-run-supervisor-074/src/diagnostic_worker_trust_input.rs",
        "tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs",
        "tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs",
        "tools/long-run-supervisor-074/src/diagnostic_kernel_progress.rs",
        "tools/long-run-supervisor-074/src/diagnostic_reviewed_trust.rs",
        "tools/long-run-supervisor-074/src/diagnostic_output_preparation.rs",
        "tools/long-run-supervisor-074/src/server.rs",
        "tools/long-run-supervisor-074/tests/diagnostic_ipc.rs",
        "tools/long-run-supervisor-074/tests/diagnostic_lease.rs",
        "tools/long-run-supervisor-074/tests/host_execution.rs",
        "docs/testing/performance/0.74/local-runs/claim-construction-20261016/manifest.json",
    ] {
        assert_eq!(
            super::original_start_consent::normalize(p, &read(p)),
            old(p),
            "original guard/input changed: {p}"
        );
    }
    let source = read("tools/long-run-supervisor-074/src/diagnostic_start_consent.rs");
    let runtime = source
        .lines()
        .filter(|l| !l.trim_start().starts_with("//! ") && !l.trim_start().starts_with("///"))
        .collect::<Vec<_>>()
        .join("\n");
    for forbidden in [
        "pub fn start_once",
        "std::fs",
        "std::process",
        "KernelPacket",
        "DiagnosticBackend",
        "FixedAuthority",
        "SystemTime",
        "SigningKey",
        "impl Drop",
        "pub fn publish",
        "pub fn recover",
        "pub fn into_",
    ] {
        assert!(!runtime.contains(forbidden), "{forbidden}");
    }
    for marker in [
        "verify_strict",
        "deny_unknown_fields",
        "Some(1..=60)",
        "MAX_BODY_BYTES",
        "MAX_ENVELOPE_BYTES",
        "validate_state(s)",
        "start-cell-one",
    ] {
        assert!(runtime.contains(marker), "{marker}");
    }
    println!("start_consent_contract byte_signature_only=true original_runtime_unchanged=true historical_assertions_preserved=true runtime_admission=false server=false");
}
