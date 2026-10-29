//! Explicit additive registrations; no normalization of previous guard bodies.
const REGISTRATIONS: [(&str, &str); 4] = [
    (
        "diagnostic_kernel_collector_inputs.rs",
        "\n#[path = \"diagnostic_kernel_progress.rs\"]\nmod progress;\n",
    ),
    (
        "diagnostic_worker_kernel_binding.rs",
        "#[path = \"diagnostic_worker_progress.rs\"]\nmod progress;\n",
    ),
    (
        "diagnostic_collector_reserved.rs",
        "#[path = \"diagnostic_collector_progress.rs\"]\npub mod progress;\n",
    ),
    (
        "diagnostic_collector_reserved_tests.rs",
        "#[path = \"diagnostic_collector_progress_tests.rs\"]\nmod progress_tests;\n",
    ),
];
pub(super) fn without_registration(path: &str, raw: &str) -> String {
    for (file, registration) in REGISTRATIONS {
        if path.ends_with(file) {
            assert_eq!(raw.matches(registration).count(), 1, "{file}");
            return raw.replacen(registration, "", 1);
        }
    }
    raw.into()
}
#[test]
fn collector_progress_requires_fresh_original_receiver_without_start_or_renewal() {
    let c = super::contract("diagnostic-collector-progress-contract.toml");
    for key in [
        "consumes_original_reservation_and_packet",
        "same_receiver_object_identity",
        "original_configured_controller_policy",
        "strict_new_receive_order",
        "exclusive_controller_loss_deadline",
        "original_admission_windows_unchanged",
        "original_authority_and_fence_retained",
        "no_recovered_ack_or_retry",
    ] {
        assert_eq!(c[key].as_bool(), Some(true), "{key}");
    }
    for key in [
        "starting_or_new_durable_write",
        "runtime_lease_or_heartbeat_renewal",
        "root_output_or_process_start",
        "server_or_qualification_admission",
        "performance_claim",
    ] {
        assert_eq!(c[key].as_bool(), Some(false), "{key}");
    }
    assert_eq!(
        c["kernel_packet_hold_ns"].as_integer(),
        Some(60_000_000_000)
    );
    assert_eq!(c["controller_loss_seconds"].as_integer(), Some(10));
    assert_eq!(c["total_seconds"].as_integer(), Some(300));
    let read = |file: &str| {
        let raw = crate::runtime_io::read_to_string(
            super::root().join(format!("tools/long-run-supervisor-074/src/{file}")),
        )
        .unwrap()
        .replace('\r', "");
        super::original_start_consent::normalize(
            &format!("tools/long-run-supervisor-074/src/{file}"),
            &raw,
        )
    };
    for file in REGISTRATIONS.map(|r| r.0).into_iter().chain([
        "diagnostic_kernel_admission.rs",
        "diagnostic_kernel_origin.rs",
        "diagnostic_lease.rs",
        "diagnostic_collector_reservation_storage.rs",
        "diagnostic_controller_fence_preflight_tests.rs",
    ]) {
        let path = format!("tools/long-run-supervisor-074/src/{file}");
        let old = std::process::Command::new("git")
            .args([
                "show",
                &format!("b9fb32e1da407d16b7073fbc94820667f3bfdfab:{path}"),
            ])
            .current_dir(super::root())
            .output()
            .unwrap();
        assert!(old.status.success());
        assert_eq!(
            without_registration(&path, &read(file)),
            String::from_utf8(old.stdout).unwrap().replace('\r', ""),
            "old predicates must remain exact: {file}"
        );
    }
    let kernel = read("diagnostic_kernel_progress.rs");
    for marker in [
        "Arc::ptr_eq(&self.origin, &next.origin)",
        "ordered(after, &next.received)?",
        "self.original_reservation_time(Some(after))?",
        "next.authenticate(policy)?",
        "CONTROLLER_LOSS_SECONDS * 1_000_000_000",
        "self.latch_refusal()",
        "next.latch_refusal()",
    ] {
        assert!(kernel.contains(marker), "{marker}");
    }
    let owner = read("diagnostic_collector_progress.rs");
    let worker = read("diagnostic_worker_progress.rs");
    assert_eq!(worker.matches("self.revalidate()?").count(), 2);
    assert!(worker.contains("self.latch_refusal()") && worker.contains("next.latch_refusal()"));
    let runtime = owner.split("#[cfg(test)]").next().unwrap();
    for marker in [
        "packet: KernelPacket",
        "let after = self.last.clone()",
        "Box::new(read)",
        "self.reservation.refuse()",
        "next.operation != DiagnosticOperation::Heartbeat",
        "next.request_id == original.request_id",
        "next.nonce_sha256 == original.nonce_sha256",
        "next.expected_state_revision != state.revision",
        "next.identity != original.identity",
        "next.controller != original.controller",
        "self.reservation.revalidate()?",
        "self.progress()",
    ] {
        assert!(owner.contains(marker), "{marker}");
    }
    for forbidden in [
        "pub fn start",
        "pub fn publish",
        "pub fn recover",
        "pub fn refresh",
        "pub fn into_",
        "impl Clone",
        "std::process",
        "std::fs",
        "FixedAuthorityRegistry::",
        "DiagnosticPeer",
        "now_unix_seconds:",
        "received =",
    ] {
        assert!(!runtime.contains(forbidden), "{forbidden}");
    }
    println!(
        "collector_progress_contract original_receiver=true admission_renewal=false starting=false"
    );
}
