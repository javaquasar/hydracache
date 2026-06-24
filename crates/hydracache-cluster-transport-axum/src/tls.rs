use std::net::{IpAddr, SocketAddr};

use hydracache::ClusterNodeId;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ClusterRoute;

/// Peer certificate material extracted by the HTTP/TLS acceptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TlsPeerCertificate {
    /// Logical HydraCache node id encoded in the peer certificate.
    pub node_id: ClusterNodeId,
    /// Certificate subject used only for diagnostics.
    pub subject: String,
    /// Subject alternative DNS names.
    pub dns_names: Vec<String>,
    /// Fingerprint of the issuing CA bundle.
    pub issuer_fingerprint: String,
    /// Unix epoch seconds after which the certificate is no longer valid.
    pub not_after_epoch_secs: u64,
}

impl TlsPeerCertificate {
    /// Create peer material for tests/adapters.
    pub fn new(
        node_id: impl Into<ClusterNodeId>,
        subject: impl Into<String>,
        issuer_fingerprint: impl Into<String>,
        not_after_epoch_secs: u64,
    ) -> Self {
        Self {
            node_id: node_id.into(),
            subject: subject.into(),
            dns_names: Vec::new(),
            issuer_fingerprint: issuer_fingerprint.into(),
            not_after_epoch_secs,
        }
    }

    /// Add one DNS SAN.
    pub fn with_dns_name(mut self, dns_name: impl Into<String>) -> Self {
        self.dns_names.push(dns_name.into());
        self
    }
}

/// Verified peer identity returned after mTLS checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TlsPeerIdentity {
    /// Authenticated logical node id.
    pub node_id: ClusterNodeId,
    /// Route that was protected by mTLS.
    pub route: ClusterRoute,
}

/// mTLS policy for cluster routes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutualTlsPolicy {
    trusted_ca_fingerprint: String,
    required_dns_suffix: Option<String>,
    now_epoch_secs: u64,
}

impl MutualTlsPolicy {
    /// Create a policy that trusts one CA fingerprint.
    pub fn new(trusted_ca_fingerprint: impl Into<String>) -> Self {
        Self {
            trusted_ca_fingerprint: trusted_ca_fingerprint.into(),
            required_dns_suffix: None,
            now_epoch_secs: 0,
        }
    }

    /// Require a DNS SAN suffix such as `.cluster.local`.
    pub fn require_dns_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.required_dns_suffix = Some(suffix.into());
        self
    }

    /// Set deterministic time for validation/tests.
    pub fn at_time(mut self, now_epoch_secs: u64) -> Self {
        self.now_epoch_secs = now_epoch_secs;
        self
    }

    /// Verify peer certificate material for one cluster route.
    pub fn verify_peer(
        &self,
        route: ClusterRoute,
        certificate: Option<&TlsPeerCertificate>,
    ) -> Result<TlsPeerIdentity, TlsError> {
        if !route_requires_mtls(route) {
            return Err(TlsError::RouteDoesNotRequireMtls(route.as_str()));
        }
        let certificate = certificate.ok_or(TlsError::ClientCertificateRequired)?;
        if certificate.issuer_fingerprint != self.trusted_ca_fingerprint {
            return Err(TlsError::UntrustedClientCertificate);
        }
        if certificate.not_after_epoch_secs <= self.now_epoch_secs {
            return Err(TlsError::ExpiredClientCertificate);
        }
        if let Some(suffix) = &self.required_dns_suffix {
            let allowed = certificate
                .dns_names
                .iter()
                .any(|dns_name| dns_name.ends_with(suffix));
            if !allowed {
                return Err(TlsError::DnsNameNotAllowed);
            }
        }
        Ok(TlsPeerIdentity {
            node_id: certificate.node_id.clone(),
            route,
        })
    }
}

/// Startup guard for listener exposure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TlsStartupPolicy {
    bind_addr: SocketAddr,
    tls_enabled: bool,
    acknowledge_insecure: bool,
}

impl TlsStartupPolicy {
    /// Create a startup policy for one listener.
    pub fn new(bind_addr: SocketAddr, tls_enabled: bool) -> Self {
        Self {
            bind_addr,
            tls_enabled,
            acknowledge_insecure: false,
        }
    }

    /// Explicitly acknowledge an insecure non-loopback listener.
    pub fn acknowledge_insecure(mut self, acknowledge: bool) -> Self {
        self.acknowledge_insecure = acknowledge;
        self
    }

    /// Validate listener security posture.
    pub fn validate(&self) -> Result<(), TlsStartupError> {
        if is_loopback(self.bind_addr.ip()) || self.tls_enabled || self.acknowledge_insecure {
            return Ok(());
        }
        Err(TlsStartupError::NonLoopbackWithoutTls {
            bind_addr: self.bind_addr,
        })
    }
}

/// Return whether the route is a cluster transport route protected by mTLS.
pub fn route_requires_mtls(route: ClusterRoute) -> bool {
    !matches!(route, ClusterRoute::Admin)
}

/// mTLS verification errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TlsError {
    /// The route is intentionally excluded from cluster mTLS enforcement.
    #[error("route {0} does not require cluster mTLS")]
    RouteDoesNotRequireMtls(&'static str),
    /// No client certificate reached the route.
    #[error("client certificate is required for cluster route")]
    ClientCertificateRequired,
    /// Certificate was not issued by the configured CA.
    #[error("client certificate was not issued by a trusted CA")]
    UntrustedClientCertificate,
    /// Certificate is expired at validation time.
    #[error("client certificate is expired")]
    ExpiredClientCertificate,
    /// Certificate SAN does not match the configured node DNS boundary.
    #[error("client certificate DNS name is outside the configured cluster boundary")]
    DnsNameNotAllowed,
}

/// Startup posture errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TlsStartupError {
    /// Externally reachable listener lacks TLS and explicit acknowledgement.
    #[error("listener {bind_addr} is non-loopback and requires TLS or explicit insecure ack")]
    NonLoopbackWithoutTls {
        /// Listener address.
        bind_addr: SocketAddr,
    },
}

fn is_loopback(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => ip.is_loopback(),
        IpAddr::V6(ip) => ip.is_loopback(),
    }
}
