//! One exact approved substitution; no removal of live guard observations.
const BASELINE: &str = "73f71f07";
const OLD: &str = r#"            let observed = verify_document(bytes, current_trust, asserted_context)?;
            if observed != self.original {
                return Err(WorkerPolicyError::Drift);
            }
"#;
const CANDIDATE: &str = r#"            // Constructor proof is bound to the original envelope, trust and context above.
            // Also retain detection of corruption in the stored projection, not just input bytes.
            if sha256_hex(&canonical_line(&self.original)?) != self.trust.policy_sha256 {
                return Err(WorkerPolicyError::Drift);
            }
"#;
const REGISTRATION: &str = r#"
#[cfg(test)]
#[path = "diagnostic_worker_proof_reuse_tests.rs"]
mod proof_reuse_tests;
"#;
const OLD_REASON: &str = r#"reason = "Manual nonroot debug fixture stage/call/signature cost attribution; no production optimization or execution""#;
const NEW_REASON: &str = r#"reason = "Manual nonroot debug fixture stage/call/signature cost attribution; no product workload or execution admission""#;
pub(super) fn restore_worker(path: &str, raw: &str) -> String {
    if !path.ends_with("diagnostic_worker_policy.rs") {
        return raw.into();
    }
    assert_eq!(raw.matches(CANDIDATE).count(), 1);
    assert_eq!(raw.matches(REGISTRATION).count(), 1);
    raw.replacen(CANDIDATE, OLD, 1)
        .replacen(REGISTRATION, "", 1)
}
pub(super) fn restore_reason(raw: &str) -> String {
    assert_eq!(raw.matches(NEW_REASON).count(), 1);
    raw.replacen(NEW_REASON, OLD_REASON, 1)
}
#[test]
fn worker_proof_reuse_is_one_owned_substitution_with_full_reference_and_live_guards() {
    let read = |p: &str| {
        let raw = crate::runtime_io::read_to_string(super::root().join(p))
            .unwrap()
            .replace('\r', "");
        super::start_consent::normalize(p, &raw)
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
    let p = "tools/long-run-supervisor-074/src/diagnostic_worker_policy.rs";
    let raw = read(p);
    let original = old(p);
    assert_eq!(restore_worker(p, &raw), original);
    let gate = "crates/xtask/tests/performance_contract_074.rs";
    let current_gate = super::short_fence::restore_gate(&read(gate));
    let registration =
        "\n#[path = \"support/performance_contract_074_worker_proof.rs\"]\nmod worker_proof;\n";
    let new_marker = "\"sha256_hex(&canonical_line(&self.original)?) != self.trust.policy_sha256\"";
    assert_eq!(current_gate.matches(registration).count(), 1);
    assert_eq!(current_gate.matches(new_marker).count(), 1);
    assert_eq!(
        current_gate.replacen(registration, "", 1).replacen(
            new_marker,
            "\"observed != self.original\"",
            1
        ),
        old(gate)
    );
    let without_trust = raw.replacen(
        "current_trust != &self.trust",
        "current_trust == &self.trust",
        1,
    );
    let without_context = raw.replacen(
        "asserted_context != &self.context",
        "asserted_context == &self.context",
        1,
    );
    let without_envelope = raw.replacen(
        "sha256_hex(bytes) != self.envelope_sha256",
        "sha256_hex(bytes) == self.envelope_sha256",
        1,
    );
    for mutation in [
        without_trust,
        without_context,
        without_envelope,
        format!("{raw}\nfn unapproved_tail() {{}}\n"),
    ] {
        assert_ne!(restore_worker(p, &mutation), original);
    }
    assert!(std::panic::catch_unwind(|| restore_worker(
        p,
        &raw.replacen(CANDIDATE, "            Ok(())?;\n", 1)
    ))
    .is_err());
    let tests = read("tools/long-run-supervisor-074/src/diagnostic_worker_proof_reuse_tests.rs");
    let reference = &tests[tests.find("    fn revalidate_full_reference(").unwrap()
        ..tests.find("\n}\n\nfn hex(").unwrap()];
    let full = &original[original.find("    pub fn revalidate(").unwrap()
        ..original.find("\n}\n\npub fn verify_worker_policy").unwrap()];
    assert_eq!(
        reference.replacen("fn revalidate_full_reference(", "pub fn revalidate(", 1),
        full
    );
    let probe = "tools/long-run-supervisor-074/src/diagnostic_validation_cost_tests.rs";
    let old_assert = "        assert!(stats.worker_policy_calls > 0);";
    let new_assert = "        assert_eq!(\n            stats.worker_policy_calls,\n            2 * u64::from(stage == Stage::Setup)\n        );";
    let current = read(probe);
    assert_eq!(current.matches(new_assert).count(), 1);
    assert_eq!(
        current.replacen(new_assert, old_assert, 1),
        old(probe),
        "probe workload/semantics changed"
    );
    let registry = "docs/testing/gated-test-registry.toml";
    assert_eq!(restore_reason(&read(registry)), old(registry));
    for p in [
        "Cargo.lock",
        "Cargo.toml",
        "docs/COMPAT.md",
        "tools/long-run-supervisor-074/Cargo.toml",
        "docs/testing/performance/0.74/qualification-manifest.toml",
        "tools/long-run-supervisor-074/src/diagnostic_worker_trust_input_tests.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_progress_tests.rs",
        "docs/testing/performance/0.74/local-runs/worker-proof-reuse-20261016/manifest.json",
        "tools/long-run-supervisor-074/src/diagnostic_worker_trust_input.rs",
        "tools/long-run-supervisor-074/src/diagnostic_worker_kernel_binding.rs",
        "tools/long-run-supervisor-074/src/diagnostic_worker_progress.rs",
        "tools/long-run-supervisor-074/src/diagnostic_controller_trust_input.rs",
        "tools/long-run-supervisor-074/src/diagnostic_controller_build_input.rs",
        "tools/long-run-supervisor-074/src/diagnostic_oidc_input.rs",
        "tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs",
        "tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_reserved.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_progress.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_causal.rs",
        "tools/long-run-supervisor-074/src/diagnostic_validation_trace.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_causal_tests.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_progress_tests.rs",
        "tools/long-run-supervisor-074/src/diagnostic_lease.rs",
        "tools/long-run-supervisor-074/src/server.rs",
    ] {
        assert_eq!(read(p), old(p), "live guard/qualification change: {p}");
    }
    println!("worker_proof_reuse_contract owned_proof=true live_guards_unchanged=true fields_unchanged=true starting=false");
}
