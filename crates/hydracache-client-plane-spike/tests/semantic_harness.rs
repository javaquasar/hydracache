use hydracache_client_plane_spike::{
    AdmissionController, AdmissionLimits, AdmissionSnapshot, BootstrapConnection,
    ConnectionGeneration, ConnectionMetrics, ConnectionState, FrameKind, PeerIdentity,
    ResourceSnapshot, SpikeConnection, SpikeError, SpikeFrame, SpikeLimits, TransportCandidate,
    HC2_GENERATION,
};

const CONNECTION_GENERATION: ConnectionGeneration = ConnectionGeneration::FIRST;

fn ready(candidate: TransportCandidate, limits: SpikeLimits) -> SpikeConnection {
    ready_at(candidate, CONNECTION_GENERATION, limits)
}

fn ready_at(
    candidate: TransportCandidate,
    generation: ConnectionGeneration,
    limits: SpikeLimits,
) -> SpikeConnection {
    BootstrapConnection::new(candidate, generation, limits)
        .verify_tls(true)
        .expect("verified TLS")
        .authenticate(PeerIdentity::verified("w0-client", "w0-tenant"))
        .expect("verified identity")
        .negotiate(HC2_GENERATION)
        .expect("current generation")
}

#[test]
fn stale_connection_generation_cannot_mutate_reused_ids_after_reconnect() {
    let old = ConnectionGeneration::FIRST;
    let current = old.checked_next().unwrap();

    for candidate in TransportCandidate::ALL {
        let mut connection = ready_at(candidate, current, SpikeLimits::default());
        connection.begin_invocation(1).unwrap();
        connection.register_subscription(7).unwrap();
        let before = connection.resources();

        assert_eq!(
            connection.complete_invocation(old, 1, b"late-reply"),
            Err(SpikeError::StaleConnectionGeneration {
                expected: current.get(),
                actual: old.get(),
            })
        );
        assert_eq!(
            connection.cancel_invocation(old, 1),
            Err(SpikeError::StaleConnectionGeneration {
                expected: current.get(),
                actual: old.get(),
            })
        );
        assert_eq!(
            connection.push_event(old, 7, b"late-event"),
            Err(SpikeError::StaleConnectionGeneration {
                expected: current.get(),
                actual: old.get(),
            })
        );
        assert_eq!(
            connection.heartbeat(old),
            Err(SpikeError::StaleConnectionGeneration {
                expected: current.get(),
                actual: old.get(),
            })
        );

        let stale_wire = connection
            .encode(&SpikeFrame::current(
                old,
                FrameKind::Invocation,
                1,
                b"late-wire",
            ))
            .unwrap();
        assert_eq!(
            connection.receive(&stale_wire),
            Err(SpikeError::StaleConnectionGeneration {
                expected: current.get(),
                actual: old.get(),
            })
        );
        assert_eq!(connection.resources(), before);
        assert_eq!(connection.dispatch_count(), 0);
        assert_eq!(connection.metrics().stale_generation_frames, 5);

        connection
            .complete_invocation(current, 1, b"current-reply")
            .unwrap();
        assert!(!connection.cancel_invocation(current, 1).unwrap());
        connection.push_event(current, 7, b"current-event").unwrap();
        connection.heartbeat(current).unwrap();
        while let Some(frame) = connection.drain_next() {
            assert_eq!(frame.connection_generation, current);
        }
        assert_eq!(connection.resources().pending_invocations, 0);
    }
}

#[test]
fn connection_generation_is_nonzero_monotonic_and_never_wraps() {
    assert_eq!(
        ConnectionGeneration::new(0),
        Err(SpikeError::InvalidConnectionGeneration)
    );
    let maximum = ConnectionGeneration::new(u64::MAX).unwrap();
    assert_eq!(
        maximum.checked_next(),
        Err(SpikeError::ConnectionGenerationExhausted)
    );
}

fn exercise_common_semantics(candidate: TransportCandidate) {
    let limits = SpikeLimits {
        max_pending_invocations: 256,
        max_reply_frames: 256,
        max_event_frames: 4,
        ..SpikeLimits::default()
    };
    let mut connection = ready(candidate, limits);

    for correlation_id in 1..=256 {
        connection
            .begin_invocation(correlation_id)
            .expect("bounded invocation accepted");
    }
    assert_eq!(connection.resources().pending_invocations, 256);
    assert_eq!(
        connection.begin_invocation(257),
        Err(SpikeError::ResourceLimit("pending_invocations"))
    );

    connection
        .register_subscription(9001)
        .expect("subscription registered");
    connection
        .complete_invocation(CONNECTION_GENERATION, 1, b"ok".to_vec())
        .expect("reply queued");
    connection
        .heartbeat(CONNECTION_GENERATION)
        .expect("heartbeat queued");
    connection
        .push_event(CONNECTION_GENERATION, 9001, b"entry-upserted".to_vec())
        .expect("event pushed");

    let first = connection.drain_next().expect("reply lane");
    let second = connection.drain_next().expect("control lane");
    let third = connection.drain_next().expect("event lane");
    assert_eq!(first.kind, FrameKind::Reply);
    assert_eq!(second.kind, FrameKind::Heartbeat);
    assert_eq!(third.kind, FrameKind::Event);

    let encoded = connection.encode(&SpikeFrame::current(
        CONNECTION_GENERATION,
        FrameKind::Invocation,
        700,
        b"get".to_vec(),
    ));
    let decoded = connection
        .receive(&encoded.expect("frame encoded"))
        .expect("frame decoded");
    assert_eq!(decoded.correlation_id, 700);
    assert_eq!(decoded.payload, b"get");
}

#[test]
fn all_client_plane_candidates_pass_the_same_semantic_harness() {
    for candidate in TransportCandidate::ALL {
        exercise_common_semantics(candidate);
    }
}

#[test]
fn slow_event_consumer_is_bounded_and_reports_a_gap() {
    for candidate in TransportCandidate::ALL {
        let mut connection = ready(
            candidate,
            SpikeLimits {
                max_event_frames: 2,
                ..SpikeLimits::default()
            },
        );
        connection
            .register_subscription(41)
            .expect("subscription registered");
        connection
            .push_event(CONNECTION_GENERATION, 41, b"one".to_vec())
            .unwrap();
        connection
            .push_event(CONNECTION_GENERATION, 41, b"two".to_vec())
            .unwrap();
        connection
            .push_event(CONNECTION_GENERATION, 41, b"three".to_vec())
            .unwrap();

        assert_eq!(connection.resources().event_frames, 1);
        let gap = connection.drain_next().expect("explicit gap");
        assert_eq!(gap.kind, FrameKind::Gap);
        assert_eq!(gap.correlation_id, 41);
    }
}

#[test]
fn disconnect_cancels_every_pending_invocation_and_registration() {
    for candidate in TransportCandidate::ALL {
        let mut connection = ready(candidate, SpikeLimits::default());
        for correlation_id in 1..=32 {
            connection.begin_invocation(correlation_id).unwrap();
        }
        for subscription_id in 100..108 {
            connection.register_subscription(subscription_id).unwrap();
            connection
                .push_event(CONNECTION_GENERATION, subscription_id, b"queued".to_vec())
                .unwrap();
        }
        connection
            .complete_invocation(CONNECTION_GENERATION, 1, b"queued".to_vec())
            .unwrap();
        connection.heartbeat(CONNECTION_GENERATION).unwrap();

        assert_eq!(connection.disconnect(), ResourceSnapshot::default());
        assert_eq!(connection.disconnect(), ResourceSnapshot::default());
        assert_eq!(connection.begin_invocation(999), Err(SpikeError::Closed));
    }
}

#[test]
fn cancellation_and_completion_races_have_exactly_one_winner() {
    for candidate in TransportCandidate::ALL {
        let mut connection = ready(candidate, SpikeLimits::default());
        connection.begin_invocation(1).unwrap();
        assert!(connection
            .cancel_invocation(CONNECTION_GENERATION, 1)
            .unwrap());
        assert_eq!(
            connection.complete_invocation(CONNECTION_GENERATION, 1, b"late".to_vec()),
            Err(SpikeError::UnknownInvocation)
        );
        assert!(!connection
            .cancel_invocation(CONNECTION_GENERATION, 1)
            .unwrap());

        connection.begin_invocation(2).unwrap();
        connection
            .complete_invocation(CONNECTION_GENERATION, 2, b"done".to_vec())
            .unwrap();
        assert!(!connection
            .cancel_invocation(CONNECTION_GENERATION, 2)
            .unwrap());
        assert_eq!(connection.resources().pending_invocations, 0);
    }
}

#[test]
fn unknown_generation_never_reaches_dispatch() {
    for candidate in TransportCandidate::ALL {
        let mut connection = ready(candidate, SpikeLimits::default());
        let valid = connection
            .encode(&SpikeFrame::current(
                CONNECTION_GENERATION,
                FrameKind::Invocation,
                1,
                b"get".to_vec(),
            ))
            .unwrap();
        let mut future = valid.clone();
        future[8..10].copy_from_slice(&(HC2_GENERATION + 1).to_be_bytes());

        assert_eq!(
            connection.receive(&future),
            Err(SpikeError::UnknownGeneration(HC2_GENERATION + 1))
        );
        assert_eq!(connection.dispatch_count(), 0);

        assert_eq!(
            connection.receive(&valid[..valid.len() - 1]),
            Err(SpikeError::LengthMismatch)
        );
        assert_eq!(connection.dispatch_count(), 0);
    }
}

#[test]
fn candidate_prefaces_cannot_cross_decode() {
    let source = ready(TransportCandidate::DedicatedTcpTls, SpikeLimits::default());
    let mut target = ready(
        TransportCandidate::GrpcBidirectional,
        SpikeLimits::default(),
    );
    let bytes = source
        .encode(&SpikeFrame::current(
            CONNECTION_GENERATION,
            FrameKind::Invocation,
            1,
            Vec::new(),
        ))
        .unwrap();
    assert_eq!(target.receive(&bytes), Err(SpikeError::CandidateMismatch));
    assert_eq!(target.dispatch_count(), 0);
}

#[test]
fn bootstrap_rejects_unverified_tls_and_identity() {
    for candidate in TransportCandidate::ALL {
        assert!(matches!(
            BootstrapConnection::new(candidate, CONNECTION_GENERATION, SpikeLimits::default())
                .verify_tls(false),
            Err(SpikeError::UnverifiedIdentity)
        ));
        assert!(matches!(
            BootstrapConnection::new(candidate, CONNECTION_GENERATION, SpikeLimits::default())
                .verify_tls(true)
                .unwrap()
                .authenticate(PeerIdentity {
                    client_id: "client".into(),
                    tenant: "tenant".into(),
                    tls_verified: false,
                }),
            Err(SpikeError::UnverifiedIdentity)
        ));
    }
}

#[test]
fn connection_state_machine_blocks_new_work_while_draining() {
    for candidate in TransportCandidate::ALL {
        let mut connection = ready(candidate, SpikeLimits::default());
        assert_eq!(connection.state(), ConnectionState::Ready);
        connection.begin_invocation(1).unwrap();
        connection.begin_draining().unwrap();
        assert_eq!(connection.state(), ConnectionState::Draining);
        assert_eq!(connection.begin_invocation(2), Err(SpikeError::NotReady));
        connection
            .complete_invocation(CONNECTION_GENERATION, 1, b"done".to_vec())
            .unwrap();
        assert_eq!(
            connection.drain_next().map(|frame| frame.correlation_id),
            Some(1)
        );
        assert_eq!(connection.disconnect(), ResourceSnapshot::default());
        assert_eq!(connection.state(), ConnectionState::Closed);
        assert_eq!(
            connection.heartbeat(CONNECTION_GENERATION),
            Err(SpikeError::Closed)
        );
    }
}

#[test]
fn bounded_failures_are_atomic_and_observable_without_payload_data() {
    for candidate in TransportCandidate::ALL {
        let limits = SpikeLimits {
            max_pending_invocations: 2,
            max_reply_frames: 1,
            max_event_frames: 1,
            max_subscriptions: 1,
            ..SpikeLimits::default()
        };
        let mut connection = ready(candidate, limits);
        connection.begin_invocation(1).unwrap();
        connection.begin_invocation(2).unwrap();
        assert_eq!(
            connection.begin_invocation(3),
            Err(SpikeError::ResourceLimit("pending_invocations"))
        );
        connection
            .complete_invocation(CONNECTION_GENERATION, 1, b"private-value")
            .unwrap();
        assert_eq!(
            connection.complete_invocation(CONNECTION_GENERATION, 2, b"must-remain-pending",),
            Err(SpikeError::ResourceLimit("reply_queue"))
        );
        assert_eq!(connection.resources().pending_invocations, 1);
        assert!(connection
            .cancel_invocation(CONNECTION_GENERATION, 2)
            .unwrap());

        connection.register_subscription(7).unwrap();
        connection
            .push_event(CONNECTION_GENERATION, 7, b"private-event-1")
            .unwrap();
        connection
            .push_event(CONNECTION_GENERATION, 7, b"private-event-2")
            .unwrap();
        assert_eq!(
            connection.metrics(),
            ConnectionMetrics {
                resource_rejections: 2,
                cancelled_invocations: 1,
                event_gaps: 1,
                ..ConnectionMetrics::default()
            }
        );
    }
}

#[test]
fn subscription_ack_without_push_fails_the_common_harness_canary() {
    for candidate in TransportCandidate::ALL {
        let mut connection = ready(candidate, SpikeLimits::default());
        connection
            .register_subscription(77)
            .expect("registration acknowledged");

        assert!(
            connection.drain_next().is_none(),
            "an acknowledgement alone must not be mistaken for event delivery"
        );
        connection
            .push_event(CONNECTION_GENERATION, 77, b"injected-event".to_vec())
            .expect("event injected");
        assert_eq!(
            connection.drain_next().map(|frame| frame.kind),
            Some(FrameKind::Event),
            "the common harness requires bytes after acknowledgement"
        );
    }
}

#[test]
fn malformed_oversized_truncated_and_unknown_messages_fail_before_dispatch() {
    for candidate in TransportCandidate::ALL {
        let limits = SpikeLimits {
            max_frame_bytes: 64,
            ..SpikeLimits::default()
        };
        let mut connection = ready(candidate, limits);
        let valid = connection
            .encode(&SpikeFrame::current(
                CONNECTION_GENERATION,
                FrameKind::Invocation,
                1,
                b"get".to_vec(),
            ))
            .unwrap();

        assert_eq!(connection.receive(&valid[..10]), Err(SpikeError::Truncated));
        let mut trailing = valid.clone();
        trailing.push(0);
        assert_eq!(
            connection.receive(&trailing),
            Err(SpikeError::LengthMismatch)
        );
        let mut unknown = valid.clone();
        unknown[18] = 0xff;
        assert_eq!(
            connection.receive(&unknown),
            Err(SpikeError::UnknownMessageKind(0xff))
        );
        assert_eq!(
            connection.receive(&vec![0; limits.max_frame_bytes + 1]),
            Err(SpikeError::Oversized)
        );
        assert_eq!(connection.dispatch_count(), 0);
    }
}

#[test]
fn every_retained_resource_class_is_bounded_atomically_and_cleared_on_close() {
    for candidate in TransportCandidate::ALL {
        let reply_floor = SpikeFrame::current(CONNECTION_GENERATION, FrameKind::Reply, 1, b"abc")
            .retained_bytes();
        let gap_floor = SpikeFrame::current(
            CONNECTION_GENERATION,
            FrameKind::Gap,
            7,
            b"event_queue_overflow",
        )
        .retained_bytes();
        let limits = SpikeLimits {
            max_pending_invocations: 2,
            max_reply_frames: 2,
            max_reply_bytes: reply_floor,
            max_event_frames: 2,
            max_event_bytes: gap_floor,
            max_retry_slots: 1,
            max_reconnect_slots: 1,
            max_deadlines: 1,
            max_topology_nodes: 1,
            max_topology_bytes: 4,
            max_sessions: 1,
            max_session_bytes: 4,
            max_batch_items: 1,
            ..SpikeLimits::default()
        };
        let mut connection = ready(candidate, limits);
        connection.begin_invocation(1).unwrap();
        connection.begin_invocation(2).unwrap();
        connection
            .complete_invocation(CONNECTION_GENERATION, 1, b"abc")
            .unwrap();
        let reply_full = connection.resources();
        assert_eq!(reply_full.reply_bytes, reply_floor);
        assert_eq!(
            connection.complete_invocation(CONNECTION_GENERATION, 2, b"x"),
            Err(SpikeError::ResourceLimit("reply_queue"))
        );
        assert_eq!(connection.resources(), reply_full);
        assert!(connection
            .cancel_invocation(CONNECTION_GENERATION, 2)
            .unwrap());

        connection.check_batch_items(1).unwrap();
        assert_eq!(
            connection.check_batch_items(2),
            Err(SpikeError::ResourceLimit("batch_items"))
        );
        connection.reserve_retry().unwrap();
        assert_eq!(
            connection.reserve_retry(),
            Err(SpikeError::ResourceLimit("retry_slots"))
        );
        connection.reserve_reconnect().unwrap();
        assert_eq!(
            connection.reserve_reconnect(),
            Err(SpikeError::ResourceLimit("reconnect_slots"))
        );
        connection.register_deadline(41).unwrap();
        assert_eq!(
            connection.register_deadline(42),
            Err(SpikeError::ResourceLimit("deadlines"))
        );
        connection.install_topology(1, 4).unwrap();
        assert_eq!(
            connection.install_topology(2, 1),
            Err(SpikeError::ResourceLimit("topology"))
        );
        assert_eq!(connection.resources().topology_bytes, 4);
        connection
            .open_session(CONNECTION_GENERATION, 51, 4)
            .unwrap();
        assert_eq!(
            connection.open_session(CONNECTION_GENERATION, 52, 1),
            Err(SpikeError::ResourceLimit("sessions"))
        );
        assert_eq!(connection.resources().session_bytes, 4);

        connection.register_subscription(7).unwrap();
        connection
            .push_event(CONNECTION_GENERATION, 7, b"event_queue_overflow")
            .unwrap();
        assert_eq!(connection.resources().event_bytes, gap_floor);
        connection
            .push_event(CONNECTION_GENERATION, 7, b"x")
            .unwrap();
        assert_eq!(connection.resources().event_frames, 1);
        assert_eq!(connection.resources().event_bytes, gap_floor);
        assert_eq!(connection.metrics().event_gaps, 1);
        connection.heartbeat(CONNECTION_GENERATION).unwrap();
        assert!(connection.resources().control_bytes > 0);

        assert!(connection.release_retry());
        assert!(!connection.release_retry());
        assert!(connection.release_reconnect());
        assert!(!connection.release_reconnect());
        assert!(connection.remove_deadline(41));
        assert!(connection.close_session(CONNECTION_GENERATION, 51).unwrap());
        let _ = connection.drain_next();
        assert_eq!(connection.disconnect(), ResourceSnapshot::default());
    }
}

#[test]
fn invalid_zero_limits_fail_before_ready() {
    for candidate in TransportCandidate::ALL {
        let bootstrap = BootstrapConnection::new(
            candidate,
            CONNECTION_GENERATION,
            SpikeLimits {
                max_retry_slots: 0,
                ..SpikeLimits::default()
            },
        )
        .verify_tls(true)
        .unwrap()
        .authenticate(PeerIdentity::verified("w0-client", "w0-tenant"))
        .unwrap();
        assert!(matches!(
            bootstrap.negotiate(HC2_GENERATION),
            Err(SpikeError::InvalidLimit("max_retry_slots"))
        ));
    }
}

#[test]
fn identity_tenant_and_global_admission_is_atomic_and_raii_released() {
    let controller = AdmissionController::new(AdmissionLimits {
        max_per_identity: 1,
        max_per_tenant: 2,
        max_global: 3,
    })
    .unwrap();
    let first_identity = PeerIdentity::verified("client-a", "tenant-a");
    let second_identity = PeerIdentity::verified("client-b", "tenant-a");
    let third_identity = PeerIdentity::verified("client-c", "tenant-b");
    let same_tenant = PeerIdentity::verified("client-d", "tenant-a");
    let global_overflow = PeerIdentity::verified("client-e", "tenant-c");

    let first = controller.admit(&first_identity).unwrap();
    assert_eq!(
        controller.admit(&first_identity).unwrap_err(),
        SpikeError::ResourceLimit("identity_connections")
    );
    let second = controller.admit(&second_identity).unwrap();
    assert_eq!(
        controller.admit(&same_tenant).unwrap_err(),
        SpikeError::ResourceLimit("tenant_connections")
    );
    let third = controller.admit(&third_identity).unwrap();
    assert_eq!(
        controller.admit(&global_overflow).unwrap_err(),
        SpikeError::ResourceLimit("global_connections")
    );
    assert_eq!(
        controller.snapshot(),
        AdmissionSnapshot {
            global_connections: 3,
            active_identities: 3,
            active_tenants: 2,
        }
    );
    drop(first);
    drop(second);
    drop(third);
    assert_eq!(controller.snapshot(), AdmissionSnapshot::default());
}
