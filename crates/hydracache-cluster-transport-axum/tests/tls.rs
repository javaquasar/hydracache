use hydracache::ClusterNodeId;
use hydracache_cluster_transport_axum::tls::{
    route_requires_mtls, MutualTlsPolicy, TlsError, TlsPeerCertificate, TlsStartupError,
    TlsStartupPolicy,
};
use hydracache_cluster_transport_axum::ClusterRoute;

fn valid_cert() -> TlsPeerCertificate {
    TlsPeerCertificate::new("member-a", "CN=member-a", "ca-prod", 2_000)
        .with_dns_name("member-a.hydracache.svc.cluster.local")
}

#[test]
fn tls_mtls_handshake_required_for_cluster_routes() {
    let policy = MutualTlsPolicy::new("ca-prod")
        .require_dns_suffix(".svc.cluster.local")
        .at_time(1_000);

    let identity = policy
        .verify_peer(ClusterRoute::Replicate, Some(&valid_cert()))
        .unwrap();

    assert_eq!(identity.node_id, ClusterNodeId::from("member-a"));
    assert_eq!(identity.route, ClusterRoute::Replicate);
    assert_eq!(
        policy.verify_peer(ClusterRoute::RaftAppend, None),
        Err(TlsError::ClientCertificateRequired)
    );
    assert!(route_requires_mtls(ClusterRoute::PeerFetch));
    assert!(!route_requires_mtls(ClusterRoute::Admin));
}

#[test]
fn tls_untrusted_or_expired_client_cert_is_rejected() {
    let policy = MutualTlsPolicy::new("ca-prod").at_time(1_000);
    let untrusted = TlsPeerCertificate::new("member-a", "CN=member-a", "ca-staging", 2_000);
    let expired = TlsPeerCertificate::new("member-a", "CN=member-a", "ca-prod", 999);

    assert_eq!(
        policy.verify_peer(ClusterRoute::Replicate, Some(&untrusted)),
        Err(TlsError::UntrustedClientCertificate)
    );
    assert_eq!(
        policy.verify_peer(ClusterRoute::Replicate, Some(&expired)),
        Err(TlsError::ExpiredClientCertificate)
    );
}

#[test]
fn tls_dns_boundary_is_enforced_when_configured() {
    let policy = MutualTlsPolicy::new("ca-prod")
        .require_dns_suffix(".svc.cluster.local")
        .at_time(1_000);
    let wrong_dns = TlsPeerCertificate::new("member-a", "CN=member-a", "ca-prod", 2_000)
        .with_dns_name("member-a.example.com");

    assert_eq!(
        policy.verify_peer(ClusterRoute::PeerFetch, Some(&wrong_dns)),
        Err(TlsError::DnsNameNotAllowed)
    );
}

#[test]
fn tls_non_loopback_without_tls_refuses_to_start_unless_acked() {
    let public_addr = "0.0.0.0:7000".parse().unwrap();

    assert_eq!(
        TlsStartupPolicy::new(public_addr, false).validate(),
        Err(TlsStartupError::NonLoopbackWithoutTls {
            bind_addr: public_addr
        })
    );
    assert!(TlsStartupPolicy::new(public_addr, false)
        .acknowledge_insecure(true)
        .validate()
        .is_ok());
    assert!(
        TlsStartupPolicy::new("127.0.0.1:7000".parse().unwrap(), false)
            .validate()
            .is_ok()
    );
}
