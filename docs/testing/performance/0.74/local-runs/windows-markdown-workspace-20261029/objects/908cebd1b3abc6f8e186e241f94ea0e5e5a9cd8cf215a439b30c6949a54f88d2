#![cfg(target_os = "linux")]
use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::starting_record::*;

#[test]
fn collector_starting_record_empty_input_is_not_a_runtime_capability() {
    assert!(encode_collector_starting_record(b"", b"", b"").is_err());
    assert!(validate_retained_collector_starting(b"", b"", b"").is_err());
}
