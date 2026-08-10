use hydracache_client_plane_spike::fault_proxy::{
    FaultAction, FaultPlan, FaultReplayArtifact, ProxyDirection, ProxyTerminal,
};
use hydracache_client_plane_spike::http2_drain::{
    DrainInitiator, DrainPhase, DrainPolicy, DrainReason, Http2DrainController,
};

fn artifact(case_id: &str, seed: u64, action: FaultAction) -> FaultReplayArtifact {
    FaultReplayArtifact::create(
        FaultPlan::new(case_id, seed, ProxyDirection::ClientToServer, vec![action]),
        vec![17, 31, 47, 71],
    )
    .unwrap()
}

#[test]
fn retained_half_close_trace_enters_bounded_client_initiated_drain() {
    let replay = artifact(
        "h20-client-half-close",
        1_592_590_354,
        FaultAction::HalfOpen,
    );
    assert_eq!(replay.trace.terminal, ProxyTerminal::HalfOpen);
    let mut drain = Http2DrainController::new(DrainPolicy::new(10).unwrap());
    drain.begin(DrainInitiator::ClientHalfClose, 0).unwrap();
    drain.first_goaway_flushed().unwrap();
    drain.final_goaway_flushed().unwrap();
    drain.tls_close_notify_completed().unwrap();
    assert_eq!(
        drain.snapshot().terminal_reason,
        Some(DrainReason::Graceful)
    );
}

#[test]
fn retained_block_trace_cannot_extend_drain_beyond_deadline() {
    let replay = artifact(
        "h20-uncooperative-timeout",
        1_592_590_355,
        FaultAction::BlockDirection,
    );
    assert_eq!(replay.trace.terminal, ProxyTerminal::Blocked);
    let mut drain = Http2DrainController::new(DrainPolicy::new(10).unwrap());
    drain.open_stream().unwrap();
    drain.begin(DrainInitiator::ServerShutdown, 0).unwrap();
    drain.first_goaway_flushed().unwrap();
    assert_eq!(drain.advance_to(9).unwrap(), None);
    assert_eq!(
        drain.advance_to(10).unwrap(),
        Some(DrainReason::DeadlineExpired)
    );
    assert_eq!(drain.snapshot().phase, DrainPhase::Closed);
    assert_eq!(drain.snapshot().active_streams, 0);
    assert_eq!(drain.snapshot().metrics.forced_stream_terminations, 1);
}

#[test]
fn retained_reset_trace_has_a_distinct_terminal_reason() {
    let replay = artifact("h20-peer-reset", 1_592_590_356, FaultAction::Reset);
    assert_eq!(replay.trace.terminal, ProxyTerminal::Reset);
    let mut drain = Http2DrainController::new(DrainPolicy::new(10).unwrap());
    drain.open_stream().unwrap();
    drain.begin(DrainInitiator::PeerGoAway, 0).unwrap();
    assert_eq!(drain.peer_reset().unwrap(), DrainReason::PeerReset);
    assert_eq!(drain.snapshot().active_streams, 0);
    assert_eq!(drain.snapshot().metrics.peer_resets, 1);
    assert_eq!(drain.snapshot().metrics.forced_stream_terminations, 1);
}
