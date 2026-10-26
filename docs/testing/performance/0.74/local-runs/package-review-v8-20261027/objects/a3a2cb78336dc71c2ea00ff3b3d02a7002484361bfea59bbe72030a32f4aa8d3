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
    println!(
        "original_collector_contract peer_time_assertions=false original_fence=true root_start=false"
    );
}

#[test]
fn observed_reservation_freezes_kernel_origin_without_publishing_or_launching() {
    let c = super::contract("diagnostic-observed-reservation-contract.toml");
    assert_eq!(
        c["kernel_packet_hold_ns"].as_integer(),
        Some(60_000_000_000)
    );
    for field in [
        "consuming_original_request_owner",
        "kernel_boot_and_boottime_only",
        "one_frozen_initial_state",
        "original_packet_hold_and_request_window_unchanged",
        "no_caller_state_or_clock_in_collector_binding",
        "distinct_guard_without_supplied_state_conversion",
        "sticky_original_refusal_and_fence_retention",
    ] {
        assert_eq!(c[field].as_bool(), Some(true), "{field}");
    }
    for field in [
        "durable_reserved_or_starting_acknowledgement",
        "root_output_or_start",
        "new_durable_write_boundary",
        "production_enrollment_or_server_admission",
        "qualification_or_performance_claim",
    ] {
        assert_eq!(c[field].as_bool(), Some(false), "{field}");
    }
    let read = |name: &str| {
        std::fs::read_to_string(
            super::root().join(format!("tools/long-run-supervisor-074/src/{name}")),
        )
        .unwrap()
    };
    let source = read("diagnostic_observed_reservation.rs");
    let runtime = source
        .lines()
        .filter(|l| !l.trim_start().starts_with("///"))
        .collect::<Vec<_>>()
        .join("\n");
    for marker in [
        "attempt: ControllerRequestAttempt<",
        "last: Option<KernelTimePoint>",
        "original_reservation_time(self.last.as_ref())?",
        "if self.state.is_none()",
        "let ns = now.boottime_ns()",
        "now.boot_id() != controller.request.identity.boot_id",
        "reserved_monotonic_ns: ns",
        "last_observed_monotonic_ns: ns",
        "controller_monotonic_ns: ns",
        "revision: 1",
        "stage: DiagnosticStage::Reserved",
        "completed_cells: 0",
        "pub fn into_observed_reservation(\n        self,",
        "owner: &'bind mut OriginalObservedReservation<",
        "self.owner.check_collector(&self.bytes)",
        "self.check_collector(bytes)?",
        "latch(self.attempt.original",
    ] {
        assert!(source.contains(marker), "{marker}");
    }
    assert_eq!(runtime.matches("self.state = Some(").count(), 1);
    let compact = runtime.split_whitespace().collect::<String>();
    assert!(compact.contains("self.state.as_ref().expect(\"observedoriginalstate\").clone()"));
    for forbidden in [
        "std::fs",
        "std::process",
        "libc::",
        "DiagnosticPeer",
        "IntentCheck",
        "pub fn start",
        "pub fn prepare",
        "pub fn publish",
        "pub fn reset",
        "pub fn refresh",
        "pub fn recover",
        "impl Clone",
        "&mut DiagnosticState",
        "FixedAuthorityIntentRead",
        "FixedStartingIntentRead",
        "DiagnosticBackend",
    ] {
        assert!(!runtime.contains(forbidden), "{forbidden}");
    }
    let bind = runtime
        .split("pub fn bind_collector_requirements")
        .nth(1)
        .unwrap();
    let signature = bind.split(") -> Result<").next().unwrap();
    assert!(
        !signature.contains("state") && !signature.contains("clock") && !signature.contains("peer")
    );
    let kernel = read("diagnostic_kernel_collector_inputs.rs");
    let sample = kernel
        .split("pub(crate) fn original_reservation_time")
        .nth(1)
        .unwrap()
        .split("pub(crate) fn original_collector_inputs")
        .next()
        .unwrap();
    for marker in [
        "self.origin.sample()?",
        "valid_hold(&self.received, &now)?",
        "ordered(previous, &now)?",
        "self.sender.revalidate(&self.origin)?",
        "self.latch_refusal()",
    ] {
        assert!(sample.contains(marker), "{marker}");
    }
    assert!(!sample.contains("valid_hold(previous") && !sample.contains("self.received ="));
    assert!(!kernel.contains("pub fn"));
    println!(
        "observed_reservation_contract kernel_origin=true frozen_state=true durable_ack=false root_start=false"
    );
}
