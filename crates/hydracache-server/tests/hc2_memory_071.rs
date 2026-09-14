use std::sync::Arc;

use hydracache_client_transport_axum::{ClientSurfaceLimits, ClientSurfaceState};
use hydracache_server::Hc2ClientPlaneService;

#[test]
fn idle_hc2_service_has_zero_owned_connection_state_and_redacted_metrics() {
    let state = Arc::new(
        ClientSurfaceState::new(ClientSurfaceLimits::default()).expect("client surface state"),
    );
    let service = Hc2ClientPlaneService::new(state, "secret-cluster-name");
    let accounting = service.accounting();
    assert_eq!(accounting.active_connections, 0);
    assert_eq!(accounting.active_subscriptions, 0);
    assert_eq!(accounting.active_sessions, 0);
    assert_eq!(accounting.pending_invocations, 0);
    assert_eq!(accounting.rejected_frames, 0);

    let metrics = service.prometheus_metrics();
    assert!(!metrics.contains("secret-cluster-name"));
    assert!(metrics.contains("hydracache_hc2_connections"));
    assert!(metrics.contains("hydracache_hc2_pending_invocations"));
}

#[test]
fn hc2_queue_count_and_transport_byte_limits_are_explicit_and_nonzero() {
    let limits = ClientSurfaceLimits::default();
    limits.validate().expect("default limits");
    assert!(limits.max_streams_per_connection > 0);
    assert!(limits.max_frame_bytes > 0);
    assert!(limits.max_value_bytes > 0);
    assert!(limits.max_batch_bytes > 0);
    // These are independent ceilings: max_frame_bytes is the HC/1 envelope
    // bound, while the shared decoded value/batch bounds also serve RESP and
    // HC/2. Do not infer one from another.
    assert!(limits.max_batch_entries > 0);
}
