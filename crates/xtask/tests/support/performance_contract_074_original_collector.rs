#[test]
fn original_collector_preflight_preserves_kernel_sources_and_no_start_boundary() {
    let c = super::contract("diagnostic-original-collector-contract.toml");
    assert_eq!(
        c["kernel_packet_hold_ns"].as_integer(),
        Some(60_000_000_000)
    );
    assert_eq!(c["max_signed_intent_bytes"].as_integer(), Some(8192));
    for field in [
        "acknowledged_original_attempt_borrow",
        "same_authority_worker_packet_controller_and_fence",
        "kernel_peer_and_receiver_clock_only",
        "exact_original_request_nonce_controller_and_window",
        "initial_reserved_projection_only",
        "existing_guards_compared_with_exact_registration_exclusions",
        "sticky_cross_refusal_without_retry_io",
    ] {
        assert_eq!(c[field].as_bool(), Some(true), "{field}");
    }
    for field in [
        "durable_reserved_or_starting_state_proved",
        "outputs_or_pipe_transfer_added",
        "root_constructor_or_start_authority_added",
        "new_durable_write_boundary",
        "production_enrollment_or_server_admission",
        "qualification_or_performance_claim",
    ] {
        assert_eq!(c[field].as_bool(), Some(false), "{field}");
    }
    let read = |path: &str| {
        std::fs::read_to_string(
            super::root().join(format!("tools/long-run-supervisor-074/src/{path}")),
        )
        .unwrap()
    };
    let kernel = read("diagnostic_kernel_collector_inputs.rs");
    for marker in [
        "pub(crate) fn original_collector_inputs",
        "self.sender.revalidate(&self.origin)?",
        "valid_hold(&self.received, &now)?",
        "gids.sort_unstable()",
        "gids.dedup()",
        "self.latch_refusal()",
    ] {
        assert!(kernel.contains(marker), "{marker}");
    }
    assert!(!kernel.contains("pub fn"));
    let source = read("diagnostic_original_collector.rs");
    let runtime = source
        .lines()
        .filter(|l| !l.trim_start().starts_with("///"))
        .collect::<Vec<_>>()
        .join("\n");
    for marker in [
        "&'bind mut ControllerRequestAttempt",
        "state.revision != 1",
        "state.completed_cells != 0",
        "body.request_id != request.request_id",
        "body.nonce_sha256 != request.nonce_sha256",
        "body.controller != request.controller",
        "body.issued_at_unix_seconds != request.issued_at_unix_seconds",
        "body.expires_at_unix_seconds != request.expires_at_unix_seconds",
        "state.identity != request.identity",
        "attempt.revalidate()?",
        "latch(self.attempt.original",
    ] {
        assert!(source.contains(marker), "{marker}");
    }
    for forbidden in [
        "DiagnosticPeer",
        "IntentCheck",
        "pub fn start",
        "pub fn prepare",
        "pub fn into_",
        "std::fs",
        "std::process",
        "libc::",
        "StartTransientUnit",
        "DiagnosticBackend",
    ] {
        assert!(!runtime.contains(forbidden), "{forbidden}");
    }
    println!("original_collector_contract peer_time_assertions=false original_fence=true root_start=false");
}
