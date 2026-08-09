use hydracache_client_plane_spike::{
    FrameKind, PeerIdentity, ResourceSnapshot, SpikeConnection, SpikeError, SpikeFrame,
    SpikeLimits, TransportCandidate, HC2_GENERATION,
};

fn ready(candidate: TransportCandidate, limits: SpikeLimits) -> SpikeConnection {
    let mut connection = SpikeConnection::new(candidate, limits);
    connection
        .authenticate(PeerIdentity::verified("w0-client", "w0-tenant"))
        .expect("verified identity");
    connection
        .negotiate(HC2_GENERATION)
        .expect("current generation");
    connection
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
        .complete_invocation(1, b"ok".to_vec())
        .expect("reply queued");
    connection.heartbeat().expect("heartbeat queued");
    connection
        .push_event(9001, b"entry-upserted".to_vec())
        .expect("event pushed");

    let first = connection.drain_next().expect("reply lane");
    let second = connection.drain_next().expect("control lane");
    let third = connection.drain_next().expect("event lane");
    assert_eq!(first.kind, FrameKind::Reply);
    assert_eq!(second.kind, FrameKind::Heartbeat);
    assert_eq!(third.kind, FrameKind::Event);

    let encoded = connection.encode(&SpikeFrame::current(
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
        connection.push_event(41, b"one".to_vec()).unwrap();
        connection.push_event(41, b"two".to_vec()).unwrap();
        connection.push_event(41, b"three".to_vec()).unwrap();

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
                .push_event(subscription_id, b"queued".to_vec())
                .unwrap();
        }
        connection
            .complete_invocation(1, b"queued".to_vec())
            .unwrap();
        connection.heartbeat().unwrap();

        assert_eq!(connection.disconnect(), ResourceSnapshot::default());
        assert_eq!(connection.begin_invocation(999), Err(SpikeError::Closed));
    }
}

#[test]
fn unknown_generation_never_reaches_dispatch() {
    for candidate in TransportCandidate::ALL {
        let mut connection = ready(candidate, SpikeLimits::default());
        let valid = connection
            .encode(&SpikeFrame::current(
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
        .encode(&SpikeFrame::current(FrameKind::Invocation, 1, Vec::new()))
        .unwrap();
    assert_eq!(target.receive(&bytes), Err(SpikeError::CandidateMismatch));
    assert_eq!(target.dispatch_count(), 0);
}

#[test]
fn authentication_precedes_negotiation_and_dispatch() {
    for candidate in TransportCandidate::ALL {
        let mut connection = SpikeConnection::new(candidate, SpikeLimits::default());
        assert_eq!(
            connection.negotiate(HC2_GENERATION),
            Err(SpikeError::UnverifiedIdentity)
        );
        assert_eq!(
            connection.authenticate(PeerIdentity {
                client_id: "client".into(),
                tenant: "tenant".into(),
                tls_verified: false,
            }),
            Err(SpikeError::UnverifiedIdentity)
        );
        assert_eq!(connection.dispatch_count(), 0);
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
            .push_event(77, b"injected-event".to_vec())
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
        unknown[10] = 0xff;
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
