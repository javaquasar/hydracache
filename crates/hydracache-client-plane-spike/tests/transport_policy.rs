use hydracache_client_plane_spike::policy::{
    AdapterMaturity, ClientTransportPolicy, DiscoveryAdvertisement, ServerTransportPolicy,
    TransportEndpoint, TransportFailure, TransportPolicyError,
};
use hydracache_client_plane_spike::{TransportCandidate, HC2_GENERATION};

fn endpoint(
    candidate: TransportCandidate,
    authority: &str,
    maturity: AdapterMaturity,
) -> TransportEndpoint {
    TransportEndpoint::new(candidate, authority, maturity)
}

fn all_transports() -> DiscoveryAdvertisement {
    ServerTransportPolicy::validate(
        HC2_GENERATION,
        vec![
            endpoint(
                TransportCandidate::GrpcBidirectional,
                "cache.example:7443",
                AdapterMaturity::Preview,
            ),
            endpoint(
                TransportCandidate::Http2Bidirectional,
                "cache.example:7444",
                AdapterMaturity::Experimental,
            ),
            endpoint(
                TransportCandidate::DedicatedTcpTls,
                "cache.example:7445",
                AdapterMaturity::Experimental,
            ),
        ],
    )
    .unwrap()
    .advertise("cluster-a", 42)
    .unwrap()
}

#[test]
fn server_advertises_only_ready_mtls_listeners_at_proven_maturity() {
    let mut unready = endpoint(
        TransportCandidate::DedicatedTcpTls,
        "cache.example:7445",
        AdapterMaturity::Experimental,
    );
    unready.ready = false;
    let advertisement = ServerTransportPolicy::validate(
        HC2_GENERATION,
        vec![
            endpoint(
                TransportCandidate::GrpcBidirectional,
                "cache.example:7443",
                AdapterMaturity::Preview,
            ),
            unready,
        ],
    )
    .unwrap()
    .advertise("cluster-a", 7)
    .unwrap();
    assert_eq!(advertisement.endpoints.len(), 1);
    assert_eq!(
        advertisement.endpoints[0].candidate,
        TransportCandidate::GrpcBidirectional
    );
}

#[test]
fn server_policy_rejects_insecure_duplicate_and_unproven_adapters() {
    let mut insecure = endpoint(
        TransportCandidate::GrpcBidirectional,
        "cache.example:7443",
        AdapterMaturity::Preview,
    );
    insecure.require_mtls = false;
    assert_eq!(
        ServerTransportPolicy::validate(HC2_GENERATION, vec![insecure]),
        Err(TransportPolicyError::InsecureTransport(
            TransportCandidate::GrpcBidirectional
        ))
    );

    let tcp = endpoint(
        TransportCandidate::DedicatedTcpTls,
        "cache.example:7445",
        AdapterMaturity::Experimental,
    );
    assert_eq!(
        ServerTransportPolicy::validate(HC2_GENERATION, vec![tcp.clone(), tcp]),
        Err(TransportPolicyError::DuplicateTransport(
            TransportCandidate::DedicatedTcpTls
        ))
    );

    assert_eq!(
        ServerTransportPolicy::validate(
            HC2_GENERATION,
            vec![endpoint(
                TransportCandidate::Http2Bidirectional,
                "cache.example:7444",
                AdapterMaturity::Stable,
            )],
        ),
        Err(TransportPolicyError::UnprovenMaturity(
            TransportCandidate::Http2Bidirectional
        ))
    );
}

#[test]
fn pinned_transport_never_falls_back() {
    let advertisement = all_transports();
    let (first, mut selection) =
        ClientTransportPolicy::Pinned(TransportCandidate::GrpcBidirectional)
            .begin(&advertisement)
            .unwrap();
    assert_eq!(first.candidate, TransportCandidate::GrpcBidirectional);
    assert_eq!(
        selection.after_failure(TransportFailure::Availability),
        Err(TransportPolicyError::FallbackDisabled)
    );
}

#[test]
fn ordered_policy_falls_back_only_for_availability_or_authenticated_unsupported() {
    let advertisement = all_transports();
    let policy = ClientTransportPolicy::Ordered {
        preference: vec![
            TransportCandidate::GrpcBidirectional,
            TransportCandidate::Http2Bidirectional,
            TransportCandidate::DedicatedTcpTls,
        ],
        minimum_maturity: AdapterMaturity::Experimental,
        allow_availability_fallback: true,
    };
    let (first, mut selection) = policy.begin(&advertisement).unwrap();
    assert_eq!(first.candidate, TransportCandidate::GrpcBidirectional);
    assert_eq!(
        selection
            .after_failure(TransportFailure::Availability)
            .unwrap()
            .candidate,
        TransportCandidate::Http2Bidirectional
    );
    assert_eq!(
        selection
            .after_failure(TransportFailure::AuthenticatedUnsupported)
            .unwrap()
            .candidate,
        TransportCandidate::DedicatedTcpTls
    );
}

#[test]
fn security_and_protocol_failures_cannot_trigger_downgrade() {
    for failure in [
        TransportFailure::TlsVerification,
        TransportFailure::Authentication,
        TransportFailure::Authorization,
        TransportFailure::GenerationMismatch,
        TransportFailure::CapabilityMismatch,
        TransportFailure::MalformedPeer,
    ] {
        let (_, mut selection) = ClientTransportPolicy::Ordered {
            preference: TransportCandidate::ALL.to_vec(),
            minimum_maturity: AdapterMaturity::Experimental,
            allow_availability_fallback: true,
        }
        .begin(&all_transports())
        .unwrap();
        assert_eq!(
            selection.after_failure(failure),
            Err(TransportPolicyError::DowngradeForbidden(failure))
        );
    }
}

#[test]
fn unauthenticated_or_wrong_generation_discovery_is_rejected() {
    let mut advertisement = all_transports();
    assert_eq!(
        advertisement.validate(false),
        Err(TransportPolicyError::UnauthenticatedDiscovery)
    );
    advertisement.generation = HC2_GENERATION + 1;
    assert_eq!(
        advertisement.validate(true),
        Err(TransportPolicyError::UnknownGeneration(HC2_GENERATION + 1))
    );
}
