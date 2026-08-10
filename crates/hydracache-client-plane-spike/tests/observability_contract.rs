use std::collections::BTreeSet;

use hydracache_client_plane_spike::http2_drain::DrainReason;
use hydracache_client_plane_spike::observability::{
    ClientPlaneDiagnostics, DeadlineOutcome, DiagnosticExport, DiagnosticSalt, DiagnosticSink,
    MetricKind, MetricSample, ObservabilityError, StateLabel, TraceSample,
    DIAGNOSTICS_SCHEMA_VERSION, MAX_METRIC_SERIES_PER_CONNECTION, MAX_TRACE_RECORDS,
    TENANT_BUCKET_COUNT,
};
use hydracache_client_plane_spike::security::SecurityRejection;
use hydracache_client_plane_spike::{
    BootstrapConnection, ClientAuthorizationPolicy, ConnectionGeneration, PeerIdentity,
    SpikeConnection, SpikeLimits, TransportCandidate, HC2_GENERATION,
};

const TENANT: &str = "tenant-secret-never-export";

fn salt(fill: u8) -> DiagnosticSalt {
    DiagnosticSalt::new([fill; 32]).unwrap()
}

fn ready(generation: ConnectionGeneration, limits: SpikeLimits) -> SpikeConnection {
    BootstrapConnection::new(TransportCandidate::GrpcBidirectional, generation, limits)
        .verify_tls(true)
        .unwrap()
        .authenticate(PeerIdentity::verified("client-secret-never-export", TENANT))
        .unwrap()
        .authorize(&ClientAuthorizationPolicy::exact("client-secret-never-export", TENANT).unwrap())
        .unwrap()
        .negotiate(HC2_GENERATION)
        .unwrap()
}

fn metric<'a>(export: &'a DiagnosticExport, name: &str) -> &'a MetricSample {
    export
        .metrics
        .iter()
        .find(|sample| sample.name == name && sample.reason.is_none())
        .unwrap_or_else(|| panic!("missing metric {name}"))
}

#[test]
fn runtime_failures_reconnect_and_terminal_reasons_increment_named_series() {
    let limits = SpikeLimits {
        max_event_frames: 1,
        ..SpikeLimits::default()
    };
    let mut connection = ready(ConnectionGeneration::FIRST, limits);
    let mut diagnostics = ClientPlaneDiagnostics::new(
        TransportCandidate::GrpcBidirectional,
        ConnectionGeneration::FIRST,
        TENANT,
        &salt(7),
        32,
    )
    .unwrap();

    connection.begin_invocation(11).unwrap();
    assert!(connection
        .cancel_invocation(ConnectionGeneration::FIRST, 11)
        .unwrap());
    connection.register_subscription(17).unwrap();
    connection
        .push_event(ConnectionGeneration::FIRST, 17, b"first")
        .unwrap();
    connection
        .push_event(ConnectionGeneration::FIRST, 17, b"overflow")
        .unwrap();
    connection.reserve_retry().unwrap();
    connection.register_deadline(19).unwrap();
    connection
        .open_session(ConnectionGeneration::FIRST, 23, 64)
        .unwrap();
    diagnostics.observe_connection(&connection).unwrap();
    diagnostics.record_retry_scheduled();
    diagnostics.record_retry_exhausted();
    diagnostics.record_repair();
    diagnostics.record_security_rejection(SecurityRejection::HostnameMismatch);
    diagnostics.record_security_rejection(SecurityRejection::AuthorizationDenied);
    diagnostics.record_deadline(DeadlineOutcome::TimedOut);
    diagnostics.record_session_heartbeat();
    diagnostics.record_session_loss();
    diagnostics.record_drain(DrainReason::DeadlineExpired);

    let next = ConnectionGeneration::FIRST.checked_next().unwrap();
    let next_connection = ready(next, limits);
    diagnostics.observe_connection(&next_connection).unwrap();
    let export = diagnostics.snapshot();

    assert_eq!(export.schema, DIAGNOSTICS_SCHEMA_VERSION);
    assert_eq!(
        metric(&export, "hydracache_hc2_cancellations_total").value,
        1
    );
    assert_eq!(metric(&export, "hydracache_hc2_event_gaps_total").value, 1);
    assert_eq!(metric(&export, "hydracache_hc2_reconnects_total").value, 1);
    assert_eq!(
        metric(&export, "hydracache_hc2_retries_scheduled_total").value,
        1
    );
    assert_eq!(
        metric(&export, "hydracache_hc2_retries_exhausted_total").value,
        1
    );
    assert_eq!(metric(&export, "hydracache_hc2_repairs_total").value, 1);
    assert_eq!(
        metric(&export, "hydracache_hc2_session_heartbeats_total").value,
        1
    );
    assert_eq!(
        metric(&export, "hydracache_hc2_session_losses_total").value,
        1
    );
    assert_eq!(
        metric(&export, "hydracache_hc2_connection_generation").value,
        2
    );
    assert!(export.metrics.iter().any(|sample| {
        sample.name == "hydracache_hc2_tls_rejections_total"
            && sample.reason == Some("hostname_mismatch")
            && sample.value == 1
    }));
    assert!(export.metrics.iter().any(|sample| {
        sample.name == "hydracache_hc2_auth_rejections_total"
            && sample.reason == Some("authorization_denied")
            && sample.value == 1
    }));
    assert!(export.metrics.iter().any(|sample| {
        sample.name == "hydracache_hc2_deadline_outcomes_total"
            && sample.reason == Some("timed_out")
            && sample.value == 1
    }));
    assert!(export.metrics.iter().any(|sample| {
        sample.name == "hydracache_hc2_drains_total"
            && sample.reason == Some("deadline_expired")
            && sample.value == 1
    }));
}

#[test]
fn closed_runtime_exports_zero_resource_gauges() {
    let mut connection = ready(ConnectionGeneration::FIRST, SpikeLimits::default());
    connection.begin_invocation(1).unwrap();
    connection.reserve_retry().unwrap();
    connection.register_deadline(1).unwrap();
    connection
        .open_session(ConnectionGeneration::FIRST, 1, 32)
        .unwrap();
    let mut diagnostics = ClientPlaneDiagnostics::new(
        TransportCandidate::GrpcBidirectional,
        ConnectionGeneration::FIRST,
        TENANT,
        &salt(9),
        8,
    )
    .unwrap();
    diagnostics.observe_connection(&connection).unwrap();
    assert_eq!(
        metric(
            &diagnostics.snapshot(),
            "hydracache_hc2_pending_invocations"
        )
        .value,
        1
    );

    assert_eq!(connection.disconnect(), Default::default());
    diagnostics.observe_connection(&connection).unwrap();
    let export = diagnostics.snapshot();
    for sample in export.metrics.iter().filter(|sample| {
        sample.kind == MetricKind::Gauge
            && !matches!(
                sample.name,
                "hydracache_hc2_connection_state" | "hydracache_hc2_connection_generation"
            )
    }) {
        assert_eq!(sample.value, 0, "{} leaked after close", sample.name);
    }
    assert_eq!(metric(&export, "hydracache_hc2_connection_state").value, 1);
    assert_eq!(
        metric(&export, "hydracache_hc2_connection_state").state,
        Some(StateLabel::Closed)
    );
}

#[test]
fn schema_cardinality_trace_memory_and_privacy_are_bounded() {
    assert!(matches!(
        DiagnosticSalt::new([0; 32]),
        Err(ObservabilityError::InvalidSalt)
    ));
    assert!(matches!(
        ClientPlaneDiagnostics::new(
            TransportCandidate::DedicatedTcpTls,
            ConnectionGeneration::FIRST,
            TENANT,
            &salt(1),
            MAX_TRACE_RECORDS + 1,
        ),
        Err(ObservabilityError::InvalidTraceCapacity)
    ));

    let mut diagnostics = ClientPlaneDiagnostics::new(
        TransportCandidate::DedicatedTcpTls,
        ConnectionGeneration::FIRST,
        TENANT,
        &salt(1),
        3,
    )
    .unwrap();
    for _ in 0..10 {
        diagnostics.record_retry_scheduled();
    }
    for reason in [
        SecurityRejection::UnknownCa,
        SecurityRejection::MissingClientCertificate,
        SecurityRejection::AuthorizationDenied,
    ] {
        diagnostics.record_security_rejection(reason);
    }
    for outcome in [
        DeadlineOutcome::Completed,
        DeadlineOutcome::TimedOut,
        DeadlineOutcome::Cancelled,
    ] {
        diagnostics.record_deadline(outcome);
    }
    for reason in [
        DrainReason::Graceful,
        DrainReason::DeadlineExpired,
        DrainReason::PeerReset,
    ] {
        diagnostics.record_drain(reason);
    }
    let export = diagnostics.snapshot();
    assert!(export.metrics.len() <= MAX_METRIC_SERIES_PER_CONNECTION);
    assert_eq!(export.traces.len(), 3);
    assert!(metric(&export, "hydracache_hc2_trace_dropped_total").value > 0);
    assert!(export
        .metrics
        .iter()
        .all(|sample| sample.labels.tenant_bucket < TENANT_BUCKET_COUNT));

    let metric_names = export
        .metrics
        .iter()
        .map(|sample| sample.name)
        .collect::<BTreeSet<_>>();
    let expected_names = [
        "hydracache_hc2_connection_state",
        "hydracache_hc2_connection_generation",
        "hydracache_hc2_pending_invocations",
        "hydracache_hc2_subscriptions",
        "hydracache_hc2_reply_queue_frames",
        "hydracache_hc2_reply_queue_bytes",
        "hydracache_hc2_event_queue_frames",
        "hydracache_hc2_event_queue_bytes",
        "hydracache_hc2_control_queue_frames",
        "hydracache_hc2_control_queue_bytes",
        "hydracache_hc2_retry_slots",
        "hydracache_hc2_reconnect_slots",
        "hydracache_hc2_deadlines",
        "hydracache_hc2_topology_nodes",
        "hydracache_hc2_topology_bytes",
        "hydracache_hc2_sessions",
        "hydracache_hc2_session_bytes",
        "hydracache_hc2_dispatched_frames_total",
        "hydracache_hc2_rejected_frames_total",
        "hydracache_hc2_resource_rejections_total",
        "hydracache_hc2_cancellations_total",
        "hydracache_hc2_event_gaps_total",
        "hydracache_hc2_stale_generation_frames_total",
        "hydracache_hc2_reconnects_total",
        "hydracache_hc2_retries_scheduled_total",
        "hydracache_hc2_retries_exhausted_total",
        "hydracache_hc2_repairs_total",
        "hydracache_hc2_session_heartbeats_total",
        "hydracache_hc2_session_losses_total",
        "hydracache_hc2_trace_dropped_total",
        "hydracache_hc2_tls_rejections_total",
        "hydracache_hc2_auth_rejections_total",
        "hydracache_hc2_deadline_outcomes_total",
        "hydracache_hc2_drains_total",
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    assert_eq!(metric_names, expected_names);
    assert!(metric_names.len() < export.metrics.len());
    let buckets = (0..1_000)
        .map(|index| {
            ClientPlaneDiagnostics::new(
                TransportCandidate::DedicatedTcpTls,
                ConnectionGeneration::FIRST,
                &format!("tenant-{index}"),
                &salt(1),
                1,
            )
            .unwrap()
            .snapshot()
            .metrics[0]
                .labels
                .tenant_bucket
        })
        .collect::<BTreeSet<_>>();
    assert!(buckets.len() <= usize::from(TENANT_BUCKET_COUNT));
    let json = serde_json::to_string(&export).unwrap();
    for forbidden in [
        TENANT,
        "client-secret-never-export",
        "cache-key-secret",
        "cache-value-secret",
        "credential-secret",
        "certificate-secret",
    ] {
        assert!(!json.contains(forbidden));
    }
    assert!(!format!("{:?}", salt(1)).contains("010101"));
}

#[derive(Default)]
struct CountingSink {
    metrics: usize,
    traces: usize,
}

impl DiagnosticSink for CountingSink {
    fn metric(&mut self, _sample: &MetricSample) {
        self.metrics += 1;
    }

    fn trace(&mut self, _sample: &TraceSample) {
        self.traces += 1;
    }
}

#[test]
fn typed_sink_receives_exact_snapshot_without_dynamic_labels() {
    let diagnostics = ClientPlaneDiagnostics::new(
        TransportCandidate::Http2Bidirectional,
        ConnectionGeneration::FIRST,
        TENANT,
        &salt(3),
        8,
    )
    .unwrap();
    let snapshot = diagnostics.snapshot();
    let mut sink = CountingSink::default();
    diagnostics.export_to(&mut sink);
    assert_eq!(sink.metrics, snapshot.metrics.len());
    assert_eq!(sink.traces, snapshot.traces.len());
}
