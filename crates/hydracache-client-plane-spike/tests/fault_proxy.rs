use std::time::Duration;

use hydracache_client_plane_spike::fault_proxy::{
    execute, proxy_one_way, FaultAction, FaultPlan, FaultProxyError, FaultReplayArtifact,
    ProxyDirection, ProxyTerminal,
};
use hydracache_client_plane_spike::{
    ConnectionGeneration, FrameKind, SpikeCodec, SpikeFrame, TransportCandidate,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn plan(case_id: &str, seed: u64, actions: Vec<FaultAction>) -> FaultPlan {
    FaultPlan::new(case_id, seed, ProxyDirection::ClientToServer, actions)
}

#[test]
fn same_seed_replays_exact_schedule_and_different_seed_changes_fragmentation() {
    let input = vec![b"abcdefghijklmnopqrstuvwxyz".to_vec()];
    let first = execute(
        &plan(
            "fragment-replay",
            19,
            vec![FaultAction::Fragment { max_chunk_bytes: 7 }],
        ),
        input.clone(),
    )
    .unwrap();
    let replayed = execute(
        &plan(
            "fragment-replay",
            19,
            vec![FaultAction::Fragment { max_chunk_bytes: 7 }],
        ),
        input.clone(),
    )
    .unwrap();
    let changed = execute(
        &plan(
            "fragment-replay",
            20,
            vec![FaultAction::Fragment { max_chunk_bytes: 7 }],
        ),
        input,
    )
    .unwrap();

    assert_eq!(first, replayed);
    assert_ne!(first.trace.deliveries, changed.trace.deliveries);
    assert_eq!(first.output_bytes(), b"abcdefghijklmnopqrstuvwxyz");
}

#[test]
fn every_declared_action_has_a_bounded_explicit_trace() {
    let cases = [
        ("pass", FaultAction::Pass, ProxyTerminal::Open),
        (
            "fragment",
            FaultAction::Fragment { max_chunk_bytes: 3 },
            ProxyTerminal::Open,
        ),
        (
            "coalesce",
            FaultAction::Coalesce { max_chunks: 2 },
            ProxyTerminal::Open,
        ),
        (
            "delay",
            FaultAction::Delay { ticks: 3 },
            ProxyTerminal::Open,
        ),
        ("reorder", FaultAction::ReorderAdjacent, ProxyTerminal::Open),
        (
            "duplicate",
            FaultAction::Duplicate { copies: 1 },
            ProxyTerminal::Open,
        ),
        (
            "drop",
            FaultAction::Drop { every_nth: 2 },
            ProxyTerminal::Open,
        ),
        ("block", FaultAction::BlockDirection, ProxyTerminal::Blocked),
        ("half-open", FaultAction::HalfOpen, ProxyTerminal::HalfOpen),
        ("reset", FaultAction::Reset, ProxyTerminal::Reset),
        (
            "late",
            FaultAction::LateDelivery { ticks: 7 },
            ProxyTerminal::Open,
        ),
        (
            "pressure",
            FaultAction::BandwidthPressure {
                bytes_per_tick: 2,
                window_bytes: 4,
            },
            ProxyTerminal::Open,
        ),
        (
            "close",
            FaultAction::CloseAfterBytes { bytes: 5 },
            ProxyTerminal::ClosedAfterBytes,
        ),
    ];

    for (case_id, action, terminal) in cases {
        let execution = execute(
            &plan(case_id, 19, vec![action.clone()]),
            vec![b"abcd".to_vec(), b"efgh".to_vec()],
        )
        .unwrap();
        assert_eq!(execution.trace.terminal, terminal, "{case_id}");
        assert_eq!(execution.trace.actions.len(), 1, "{case_id}");
        assert_eq!(execution.trace.actions[0].action, action, "{case_id}");
        assert!(execution.trace.deliveries.len() <= 16, "{case_id}");
        assert!(execution.trace.output_bytes <= 16, "{case_id}");
        assert!(
            serde_json::to_vec(&execution.trace).unwrap().len() < 8 * 1024,
            "{case_id}"
        );
    }
}

#[test]
fn half_open_can_retain_then_late_deliver_bytes() {
    let execution = execute(
        &plan(
            "half-open-late",
            19,
            vec![
                FaultAction::HalfOpen,
                FaultAction::LateDelivery { ticks: 11 },
            ],
        ),
        vec![b"reply".to_vec()],
    )
    .unwrap();
    assert_eq!(execution.trace.terminal, ProxyTerminal::HalfOpen);
    assert_eq!(execution.deliveries[0].tick, 11);
    assert_eq!(execution.output_bytes(), b"reply");
}

#[test]
fn semantic_preservation_and_fail_closed_faults_match_all_candidates() {
    for candidate in TransportCandidate::ALL {
        let codec = SpikeCodec::new(candidate, 64 * 1024);
        let frame = SpikeFrame::current(
            ConnectionGeneration::FIRST,
            FrameKind::Invocation,
            41,
            b"same-semantics".to_vec(),
        );
        let encoded = codec.encode(&frame).unwrap();
        let split = vec![encoded[..5].to_vec(), encoded[5..].to_vec()];
        let preserved = execute(
            &plan(
                "candidate-preservation",
                0x1919,
                vec![
                    FaultAction::Fragment { max_chunk_bytes: 5 },
                    FaultAction::Coalesce { max_chunks: 3 },
                    FaultAction::Delay { ticks: 2 },
                    FaultAction::BandwidthPressure {
                        bytes_per_tick: 7,
                        window_bytes: 21,
                    },
                ],
            ),
            split.clone(),
        )
        .unwrap();
        assert_eq!(codec.decode(&preserved.output_bytes()).unwrap(), frame);

        let truncated = execute(
            &plan(
                "candidate-close",
                0x1919,
                vec![FaultAction::CloseAfterBytes {
                    bytes: encoded.len() - 1,
                }],
            ),
            split.clone(),
        )
        .unwrap();
        assert!(codec.decode(&truncated.output_bytes()).is_err());
        assert_eq!(truncated.trace.terminal, ProxyTerminal::ClosedAfterBytes);

        let duplicated = execute(
            &plan(
                "candidate-duplicate",
                0x1919,
                vec![FaultAction::Duplicate { copies: 1 }],
            ),
            split,
        )
        .unwrap();
        assert!(codec.decode(&duplicated.output_bytes()).is_err());
    }
}

#[test]
fn replay_artifact_is_payload_free_exact_and_tamper_evident() {
    let artifact = FaultReplayArtifact::create(
        plan(
            "retained-h19",
            0x5eed_0019,
            vec![
                FaultAction::Fragment {
                    max_chunk_bytes: 11,
                },
                FaultAction::Coalesce { max_chunks: 3 },
                FaultAction::Delay { ticks: 2 },
                FaultAction::BandwidthPressure {
                    bytes_per_tick: 13,
                    window_bytes: 52,
                },
            ],
        ),
        vec![17, 31, 47],
    )
    .unwrap();
    artifact.verify().unwrap();
    let json = serde_json::to_string_pretty(&artifact).unwrap();
    assert!(json.len() < 32 * 1024);
    assert!(!json.contains("application_payload"));
    let decoded: FaultReplayArtifact = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, artifact);

    let mut tampered = artifact;
    tampered.trace.output_bytes += 1;
    assert!(matches!(
        tampered.verify(),
        Err(FaultProxyError::ReplayMismatch)
    ));
}

#[test]
fn invalid_or_unbounded_plans_fail_closed() {
    let mut invalid = plan("invalid", 1, vec![FaultAction::Duplicate { copies: 5 }]);
    assert!(matches!(
        execute(&invalid, vec![vec![1]]),
        Err(FaultProxyError::InvalidPlan("copies"))
    ));
    invalid.actions = vec![FaultAction::Pass];
    invalid.max_buffered_bytes = 2;
    assert!(matches!(
        execute(&invalid, vec![vec![1, 2, 3]]),
        Err(FaultProxyError::BufferLimitExceeded)
    ));
    invalid.max_buffered_bytes = 16;
    invalid.max_trace_events = 1;
    assert!(matches!(
        execute(&invalid, vec![vec![1], vec![2]]),
        Err(FaultProxyError::TraceLimitExceeded)
    ));

    let after_terminal = plan(
        "after-terminal",
        1,
        vec![FaultAction::Reset, FaultAction::LateDelivery { ticks: 1 }],
    );
    assert!(matches!(
        execute(&after_terminal, vec![vec![1]]),
        Err(FaultProxyError::InvalidPlan("action_after_terminal"))
    ));

    let mut projected_overflow = plan(
        "projected-overflow",
        1,
        vec![FaultAction::Duplicate { copies: 4 }],
    );
    projected_overflow.max_buffered_bytes = 8;
    assert!(matches!(
        execute(&projected_overflow, vec![vec![1, 2]]),
        Err(FaultProxyError::BufferLimitExceeded)
    ));

    let close_beyond_eof = execute(
        &plan(
            "close-beyond-eof",
            1,
            vec![FaultAction::CloseAfterBytes { bytes: 3 }],
        ),
        vec![vec![1, 2]],
    )
    .unwrap();
    assert_eq!(close_beyond_eof.trace.terminal, ProxyTerminal::Open);
}

#[tokio::test]
async fn one_way_proxy_operates_on_real_async_byte_streams() {
    let (mut source_writer, mut source_reader) = tokio::io::duplex(256);
    let (mut sink_writer, mut sink_reader) = tokio::io::duplex(256);
    source_writer.write_all(b"real-socket-path").await.unwrap();
    source_writer.shutdown().await.unwrap();

    let trace = proxy_one_way(
        &mut source_reader,
        &mut sink_writer,
        &plan(
            "real-async-stream",
            19,
            vec![
                FaultAction::Fragment { max_chunk_bytes: 4 },
                FaultAction::Coalesce { max_chunks: 2 },
                FaultAction::Delay { ticks: 1 },
            ],
        ),
        3,
        Duration::ZERO,
    )
    .await
    .unwrap();
    sink_writer.shutdown().await.unwrap();
    let mut output = Vec::new();
    sink_reader.read_to_end(&mut output).await.unwrap();

    assert_eq!(output, b"real-socket-path");
    assert_eq!(trace.terminal, ProxyTerminal::Open);
    assert_eq!(trace.input_chunks, 6);
}
