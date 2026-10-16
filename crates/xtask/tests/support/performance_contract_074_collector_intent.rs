//! Keep signed requirements separate from original-start authority and legacy bytes.
use std::process::Command;

fn historical(path: &str) -> String {
    let out = Command::new("git")
        .args([
            "show",
            &format!("46f721e9ad98d5404f3682819b11be550c36f566:{path}"),
        ])
        .current_dir(super::root())
        .output()
        .unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout).unwrap().replace('\r', "")
}
#[test]
fn collector_intent_keeps_legacy_bytes_and_execution_boundary_closed() {
    let c = super::contract("diagnostic-collector-intent-contract.toml");
    for (field, value) in [
        ("max_body_bytes", 4096),
        ("max_envelope_bytes", 8192),
        ("max_window_seconds", 60),
        ("stream_byte_limit", 8_388_608),
        ("combined_byte_limit", 16_777_216),
    ] {
        assert_eq!(c[field].as_integer(), Some(value), "{field}");
    }
    for field in [
        "distinct_body_envelope_unit_and_signature_domain",
        "legacy_verifier_and_unit_builder_unchanged",
        "fixed_typed_ordered_unit_projection",
        "no_worker_regular_file_output",
        "sole_root_collector_requirements_signed",
        "exclusive_original_worker_borrow",
        "original_state_peer_configuration_and_envelope_pins",
        "sticky_guard_and_original_worker_refusal",
    ] {
        assert_eq!(c[field].as_bool(), Some(true), "{field}");
    }
    for field in [
        "actual_pipe_objects_or_root_ownership_proved",
        "live_clock_peer_or_host_proved",
        "original_start_or_all_writer_cessation_proved",
        "durable_lifecycle_or_replay_prevention_added",
        "production_or_server_admission",
        "qualification_or_performance_claim",
    ] {
        assert_eq!(c[field].as_bool(), Some(false), "{field}");
    }
    let path = "tools/long-run-supervisor-074/src/diagnostic_intent_projection.rs";
    let current = std::fs::read_to_string(super::root().join(path))
        .unwrap()
        .replace('\r', "");
    let registration = "\n#[path = \"diagnostic_collector_intent.rs\"]\npub mod collector;\n";
    assert_eq!(current.matches(registration).count(), 1);
    assert_eq!(
        current.replacen(registration, "", 1),
        historical(path),
        "legacy byte verifier must remain unchanged"
    );
    for path in [
        "tools/long-run-supervisor-074/src/diagnostic_unit.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector.rs",
        "tools/long-run-supervisor-074/src/diagnostic_starting_intent.rs",
        "tools/long-run-supervisor-074/src/diagnostic_controller_request_attempt.rs",
        "docs/testing/gated-test-registry.toml",
        "docs/testing/performance/0.74/qualification-manifest.toml",
        "Cargo.lock",
    ] {
        let mut current = std::fs::read_to_string(super::root().join(path))
            .unwrap()
            .replace('\r', "");
        current = super::original_start_consent::normalize(path, &current);
        if path.ends_with("diagnostic_controller_request_attempt.rs") {
            let registration =
                "\n#[path = \"diagnostic_original_collector.rs\"]\npub mod collector_preflight;\n";
            assert_eq!(current.matches(registration).count(), 1);
            current = current.replacen(registration, "", 1);
        }
        if path.ends_with("gated-test-registry.toml") {
            current = super::collector_causal::without_registry_addition(&current);
            let addition = super::collector_reserved::REGISTRY_ADDITION;
            assert_eq!(current.matches(addition).count(), 1);
            current = current.replacen(addition, "", 1);
        }
        assert_eq!(current, historical(path), "out-of-scope mutation: {path}");
    }
    let source = std::fs::read_to_string(
        super::root().join("tools/long-run-supervisor-074/src/diagnostic_collector_intent.rs"),
    )
    .unwrap();
    // Inspect runtime source, excluding only the documented compile-fail examples.
    let runtime = source
        .lines()
        .filter(|l| !l.trim_start().starts_with("///"))
        .collect::<Vec<_>>()
        .join("\n");
    for forbidden in [
        "std::fs",
        "std::process",
        "libc::",
        "Command::new",
        "StartTransientUnit",
        "DiagnosticBackend",
        "pub fn start",
        "pub fn into_",
        "pub fn reset",
        "pub fn refresh",
        "pub fn publish",
        "impl Clone",
    ] {
        assert!(
            !runtime.contains(forbidden),
            "unexpected execution/authority: {forbidden}"
        );
    }
    for marker in [
        "verify_strict",
        "self.refused = true",
        "check.state != &self.state",
        "body.worker_policy_epoch != worker.original.policy_epoch",
        "super::project_reserved_state(state)?",
        "both-kernel-read-eofs-after-drain",
        "all-evidence-writers-explicitly-closed",
        "all-creator-manager-writer-copies-closed",
        "original-cgroup-revoked-and-empty",
    ] {
        assert!(source.contains(marker), "missing {marker}");
    }
    println!(
        "collector_intent_contract legacy_unchanged=true requirements_only=true execution=false"
    );
}
