//! Causal post-ack response is not a Starting or lifetime authority handoff.
use sha2::{Digest, Sha256};
pub(super) const REGISTRY_ADDITION: &str = r#"
# BEGIN COLLECTOR CAUSAL PROOF GATE
[[gate]]
id = "tool.hydracache-long-run-supervisor-074.collector-causal-real-response-prequeued-refusal-and-entropy-failure-keep-fence"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_collector_causal_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "collector_causal_real_response_prequeued_refusal_and_entropy_failure_keep_fence"
cfg = 'target_os = "linux"'
env = ""
reason = "Manual nonroot original post-Reserved challenge, prequeued response, entropy and real expiry proof; no execution"
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
args = ["test", "-p", "hydracache-long-run-supervisor-074", "--lib", "--locked", "diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::causal_tests::collector_causal_real_response_prequeued_refusal_and_entropy_failure_keep_fence", "--", "--ignored", "--exact", "--nocapture"]
cwd = "."
platform = "linux"

[gate.command.env]
# END COLLECTOR CAUSAL PROOF GATE
"#;
pub(super) fn without_registry_addition(raw: &str) -> String {
    let raw = super::validation_cost::without_registry_addition(raw);
    assert_eq!(raw.matches(REGISTRY_ADDITION).count(), 1);
    raw.replacen(REGISTRY_ADDITION, "", 1)
}
pub(super) const COMPAT_ADDITION: &str = r#"| Separate 0.74 collector progress challenge | `diagnostic-collector-progress-challenge-074-v1` | opt-in Linux original-owner library only; no installed consumer | Canonical LF JSON, at most 4096 bytes; 32 system-random bytes, original controller-attempt/Reserved/canonical-state hashes, identity, revision 1 and original-kernel issuance BOOTTIME. SHA256 of `hydracache-diagnostic-collector-progress-challenge-074-v1`, NUL and the complete LF document is echoed as nonce_sha256 in an otherwise unchanged signed DiagnosticRequest v1 Heartbeat. | Unknown/future/duplicate/noncanonical/oversized/partial documents refuse. Old clients cannot parse or downgrade this challenge; deploy a matched consumer. Byte inspection computes a digest, never an owner or acknowledgement. Exact owned nonce, original live inputs and exclusive challenge deadline are required; no regeneration, retry, recovered permission, durable progress, Starting or execution authority. |
"#;
#[test]
fn collector_causal_requires_original_post_ack_challenge_and_no_admission_renewal() {
    let c = super::contract("diagnostic-collector-causal-contract.toml");
    for key in [
        "post_ack_system_random_challenge",
        "request_reserved_and_state_commitments",
        "same_original_authority_fence_and_progress_guard",
        "immutable_issuance_and_nonce_without_regeneration",
        "prequeued_signed_message_cannot_answer_new_challenge",
        "one_consuming_response_and_sticky_failure",
        "byte_inspection_cannot_recover_owner",
    ] {
        assert_eq!(c[key].as_bool(), Some(true), "{key}");
    }
    for key in [
        "new_starting_or_durable_write",
        "admission_or_runtime_lease_renewal",
        "continuous_controller_presence_or_full_record_review",
        "production_enrollment_or_server_admission",
        "qualification_or_performance_claim",
    ] {
        assert_eq!(c[key].as_bool(), Some(false), "{key}");
    }
    for (key, value) in [
        ("max_challenge_bytes", 4096),
        ("entropy_bytes", 32),
        ("challenge_seconds", 10),
        ("kernel_packet_hold_ns", 60_000_000_000),
    ] {
        assert_eq!(c[key].as_integer(), Some(value), "{key}");
    }
    let read = |p: &str| {
        let raw = std::fs::read_to_string(super::root().join(p))
            .unwrap()
            .replace('\r', "");
        super::start_consent::normalize(p, &raw)
    };
    let historical = |p: &str| {
        let out = std::process::Command::new("git")
            .args([
                "show",
                &format!("7dff8a6bb04f3786016e699020c87d5ee8889c4e:{p}"),
            ])
            .current_dir(super::root())
            .output()
            .unwrap();
        assert!(out.status.success());
        String::from_utf8(out.stdout).unwrap().replace('\r', "")
    };
    for (file, registration) in [
        (
            "diagnostic_collector_progress.rs",
            "#[path = \"diagnostic_collector_causal.rs\"]\npub mod causal;\n",
        ),
        (
            "diagnostic_collector_progress_tests.rs",
            "#[path = \"diagnostic_collector_causal_tests.rs\"]\nmod causal_tests;\n",
        ),
    ] {
        let p = format!("tools/long-run-supervisor-074/src/{file}");
        let raw = super::validation_cost::without_instrumentation(&p, &read(&p));
        assert_eq!(raw.matches(registration).count(), 1);
        assert_eq!(
            raw.replacen(registration, "", 1),
            historical(&p),
            "prior progress bodies changed"
        );
    }
    for p in [
        "Cargo.lock",
        "docs/testing/performance/0.74/qualification-manifest.toml",
        "tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs",
        "tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs",
        "tools/long-run-supervisor-074/src/diagnostic_kernel_progress.rs",
        "tools/long-run-supervisor-074/src/diagnostic_worker_progress.rs",
        "tools/long-run-supervisor-074/src/diagnostic_worker_kernel_binding.rs",
        "tools/long-run-supervisor-074/src/diagnostic_lease.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_reservation_storage.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_reserved.rs",
        "tools/long-run-supervisor-074/src/diagnostic_controller_request_attempt.rs",
        "tools/long-run-supervisor-074/src/diagnostic_controller_fence_preflight_tests.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_reserved_crash_tests.rs",
    ] {
        assert_eq!(read(p), historical(p), "out-of-scope change: {p}");
    }
    let compat = read("docs/COMPAT.md");
    let registry = read("docs/testing/gated-test-registry.toml");
    assert_eq!(
        without_registry_addition(&registry),
        historical("docs/testing/gated-test-registry.toml")
    );
    assert_eq!(c["live_composition_tier"].as_str(), Some("manual"));
    assert_eq!(c["manual_harness_seconds"].as_integer(), Some(900));
    assert_eq!(compat.matches(COMPAT_ADDITION).count(), 1);
    assert_eq!(
        compat.replacen(COMPAT_ADDITION, "", 1),
        historical("docs/COMPAT.md")
    );
    let source = read("tools/long-run-supervisor-074/src/diagnostic_collector_causal.rs");
    let runtime = source
        .lines()
        .filter(|l| !l.trim_start().starts_with("///"))
        .collect::<Vec<_>>()
        .join("\n");
    let compact = runtime.split_whitespace().collect::<String>();
    for marker in [
        "SystemRandom::new().fill(&mut random)",
        "let random = random()?",
        "after: issued.clone()",
        "self.original.revalidate()?",
        "response.request.nonce_sha256 != self.nonce",
        "window(&mut self.original.reservation, &self.issued)",
        "self.original.refuse()",
        "collector_challenge_nonce(raw)? != nonce",
        "Some(Box::new(packet))",
        "controller_attempt_sha256 != sha256_hex(&reservation.request_raw)",
        "collector_reserved_sha256 != sha256_hex(&reservation.raw)",
        "CONTROLLER_LOSS_SECONDS * 1_000_000_000",
    ] {
        assert!(
            compact.contains(&marker.split_whitespace().collect::<String>()),
            "{marker}"
        );
    }
    assert!(
        runtime.find("self.reservation.revalidate()?").unwrap()
            < runtime.find("let random = random()?").unwrap()
    );
    assert!(
        runtime.find("let random = random()?").unwrap()
            < runtime.find("let issued = self").unwrap()
    );
    for forbidden in [
        "pub fn start",
        "pub fn publish",
        "pub fn renew",
        "pub fn refresh",
        "pub fn recover",
        "pub fn into_",
        "impl Clone",
        "std::fs",
        "std::process",
        "DiagnosticPeer",
        "FixedAuthorityRegistry::",
        "received =",
    ] {
        assert!(!runtime.contains(forbidden), "{forbidden}");
    }
    let tests = read("tools/long-run-supervisor-074/src/diagnostic_collector_causal_tests.rs");
    let ignore = "#[ignore = \"manual nonroot fixed-window causal proof; no server\"]\n";
    assert_eq!(tests.matches(ignore).count(), 1);
    let before = tests.replacen(ignore, "", 1);
    assert_eq!(
        Sha256::digest(before.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "d739cb212a90d6ac4febe7b4585b287abc2d0afb9e3d3d98b2f1c71fe3d7038c",
        "tiering must not rewrite the composition body"
    );
    for marker in [
        "libc::send",
        "receiver.try_receive()",
        "CollectorCausalError::Binding",
        "CollectorCausalError::Entropy",
        "retained_context_refuses_for_test",
        "busy(root)",
    ] {
        assert!(tests.contains(marker), "{marker}");
    }
    println!("collector_causal_contract post_ack_commitment=true original_guards=true starting=false renewal=false");
}
