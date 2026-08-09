//! Fail-closed server and client transport-selection policy for the HC/2 spike.

use std::collections::BTreeSet;

use thiserror::Error;

use crate::{TransportCandidate, HC2_GENERATION};

const MAX_CLUSTER_ID_BYTES: usize = 128;
const MAX_AUTHORITY_BYTES: usize = 512;

/// Evidence maturity of one independently gated transport adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AdapterMaturity {
    /// Test-only adapter requiring explicit opt-in.
    Experimental,
    /// Usable for preview cohorts, but not a stable compatibility promise.
    Preview,
    /// Fully release-gated adapter.
    Stable,
}

impl TransportCandidate {
    /// Highest maturity justified by current repository evidence.
    pub const fn proven_maturity(self) -> AdapterMaturity {
        match self {
            Self::GrpcBidirectional => AdapterMaturity::Preview,
            Self::DedicatedTcpTls | Self::Http2Bidirectional => AdapterMaturity::Experimental,
        }
    }
}

/// One configured and ready client-plane listener.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportEndpoint {
    pub candidate: TransportCandidate,
    pub authority: String,
    pub maturity: AdapterMaturity,
    pub require_mtls: bool,
    pub ready: bool,
}

impl TransportEndpoint {
    pub fn new(
        candidate: TransportCandidate,
        authority: impl Into<String>,
        maturity: AdapterMaturity,
    ) -> Self {
        Self {
            candidate,
            authority: authority.into(),
            maturity,
            require_mtls: true,
            ready: true,
        }
    }

    fn validate(&self) -> Result<(), TransportPolicyError> {
        if self.authority.trim().is_empty() || self.authority.len() > MAX_AUTHORITY_BYTES {
            return Err(TransportPolicyError::InvalidAuthority);
        }
        if !self.require_mtls {
            return Err(TransportPolicyError::InsecureTransport(self.candidate));
        }
        if self.maturity > self.candidate.proven_maturity() {
            return Err(TransportPolicyError::UnprovenMaturity(self.candidate));
        }
        Ok(())
    }
}

/// Validated cluster-side listener policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerTransportPolicy {
    generation: u16,
    endpoints: Vec<TransportEndpoint>,
}

impl ServerTransportPolicy {
    pub fn validate(
        generation: u16,
        endpoints: Vec<TransportEndpoint>,
    ) -> Result<Self, TransportPolicyError> {
        if generation != HC2_GENERATION {
            return Err(TransportPolicyError::UnknownGeneration(generation));
        }
        if endpoints.is_empty() {
            return Err(TransportPolicyError::NoEnabledTransport);
        }
        let mut candidates = BTreeSet::new();
        for endpoint in &endpoints {
            endpoint.validate()?;
            if !candidates.insert(endpoint.candidate) {
                return Err(TransportPolicyError::DuplicateTransport(endpoint.candidate));
            }
        }
        Ok(Self {
            generation,
            endpoints,
        })
    }

    /// Advertise only listeners that have successfully bound and become ready.
    pub fn advertise(
        &self,
        cluster_id: impl Into<String>,
        epoch: u64,
    ) -> Result<DiscoveryAdvertisement, TransportPolicyError> {
        let advertisement = DiscoveryAdvertisement {
            cluster_id: cluster_id.into(),
            epoch,
            generation: self.generation,
            endpoints: self
                .endpoints
                .iter()
                .filter(|endpoint| endpoint.ready)
                .cloned()
                .collect(),
        };
        advertisement.validate_payload()?;
        Ok(advertisement)
    }
}

/// Authenticated discovery payload consumed by an HC/2 client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryAdvertisement {
    pub cluster_id: String,
    pub epoch: u64,
    pub generation: u16,
    pub endpoints: Vec<TransportEndpoint>,
}

impl DiscoveryAdvertisement {
    fn validate_payload(&self) -> Result<(), TransportPolicyError> {
        if self.cluster_id.trim().is_empty() || self.cluster_id.len() > MAX_CLUSTER_ID_BYTES {
            return Err(TransportPolicyError::InvalidClusterId);
        }
        if self.generation != HC2_GENERATION {
            return Err(TransportPolicyError::UnknownGeneration(self.generation));
        }
        if self.endpoints.is_empty() {
            return Err(TransportPolicyError::NoEnabledTransport);
        }
        let mut candidates = BTreeSet::new();
        for endpoint in &self.endpoints {
            endpoint.validate()?;
            if !endpoint.ready {
                return Err(TransportPolicyError::UnreadyAdvertisement(
                    endpoint.candidate,
                ));
            }
            if !candidates.insert(endpoint.candidate) {
                return Err(TransportPolicyError::DuplicateTransport(endpoint.candidate));
            }
        }
        Ok(())
    }

    pub fn authenticate(
        self,
        evidence: VerifiedDiscoveryEvidence,
    ) -> Result<AuthenticatedAdvertisement, TransportPolicyError> {
        self.validate_payload()?;
        if self.cluster_id != evidence.cluster_id {
            return Err(TransportPolicyError::ClusterIdentityMismatch);
        }
        Ok(AuthenticatedAdvertisement { inner: self })
    }
}

/// Discovery that crossed a verified transport or signature boundary.
///
/// The wrapper has no public constructor. Raw decoded discovery therefore
/// cannot be passed to client selection accidentally.
///
/// ```compile_fail
/// use hydracache_client_plane_spike::policy::{
///     AuthenticatedAdvertisement, DiscoveryAdvertisement,
/// };
/// let raw: DiscoveryAdvertisement = todo!();
/// let forged = AuthenticatedAdvertisement { inner: raw };
/// ```
///
/// ```compile_fail
/// use hydracache_client_plane_spike::policy::VerifiedDiscoveryEvidence;
/// let forged = VerifiedDiscoveryEvidence { cluster_id: "cluster-a".into() };
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedAdvertisement {
    inner: DiscoveryAdvertisement,
}

impl AuthenticatedAdvertisement {
    pub fn cluster_id(&self) -> &str {
        &self.inner.cluster_id
    }

    pub fn epoch(&self) -> u64 {
        self.inner.epoch
    }

    pub fn generation(&self) -> u16 {
        self.inner.generation
    }

    pub fn endpoints(&self) -> &[TransportEndpoint] {
        &self.inner.endpoints
    }
}

/// Opaque proof produced only by a verified adapter boundary in this crate.
///
/// The type is public so the checked conversion can appear in the API, while
/// its representation and constructor remain crate-private. H01 will call the
/// constructor only after its real channel/signature verification succeeds.
#[derive(Debug, Clone)]
pub struct VerifiedDiscoveryEvidence {
    cluster_id: String,
}

impl VerifiedDiscoveryEvidence {
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "reserved for the production verified adapter introduced by H01"
        )
    )]
    pub(crate) fn verified_channel(cluster_id: impl Into<String>) -> Self {
        Self {
            cluster_id: cluster_id.into(),
        }
    }
}

/// Client choice: exact transport, or an explicit ordered availability policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientTransportPolicy {
    Pinned(TransportCandidate),
    Ordered {
        preference: Vec<TransportCandidate>,
        minimum_maturity: AdapterMaturity,
        allow_availability_fallback: bool,
    },
}

/// Failures that may or may not permit trying another advertised adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportFailure {
    Availability,
    AuthenticatedUnsupported,
    TlsVerification,
    Authentication,
    Authorization,
    GenerationMismatch,
    CapabilityMismatch,
    MalformedPeer,
}

impl TransportFailure {
    const fn permits_fallback(self) -> bool {
        matches!(self, Self::Availability | Self::AuthenticatedUnsupported)
    }
}

/// Stateful, bounded selection sequence for one connection attempt.
#[derive(Debug, Clone)]
pub struct TransportSelection {
    candidates: Vec<TransportEndpoint>,
    next: usize,
    allow_availability_fallback: bool,
}

impl ClientTransportPolicy {
    pub fn begin(
        &self,
        advertisement: &AuthenticatedAdvertisement,
    ) -> Result<(TransportEndpoint, TransportSelection), TransportPolicyError> {
        let endpoints = advertisement.endpoints();
        let (preference, minimum_maturity, allow_availability_fallback) = match self {
            Self::Pinned(candidate) => (vec![*candidate], AdapterMaturity::Experimental, false),
            Self::Ordered {
                preference,
                minimum_maturity,
                allow_availability_fallback,
            } => {
                if preference.is_empty()
                    || preference.iter().copied().collect::<BTreeSet<_>>().len() != preference.len()
                {
                    return Err(TransportPolicyError::InvalidPreference);
                }
                (
                    preference.clone(),
                    *minimum_maturity,
                    *allow_availability_fallback,
                )
            }
        };

        let mut candidates = Vec::new();
        for preferred in preference {
            if let Some(endpoint) = endpoints.iter().find(|endpoint| {
                endpoint.candidate == preferred && endpoint.maturity >= minimum_maturity
            }) {
                candidates.push(endpoint.clone());
            }
        }
        let first = candidates
            .first()
            .cloned()
            .ok_or(TransportPolicyError::NoCompatibleTransport)?;
        Ok((
            first,
            TransportSelection {
                candidates,
                next: 1,
                allow_availability_fallback,
            },
        ))
    }
}

impl TransportSelection {
    pub fn after_failure(
        &mut self,
        failure: TransportFailure,
    ) -> Result<TransportEndpoint, TransportPolicyError> {
        if !failure.permits_fallback() {
            return Err(TransportPolicyError::DowngradeForbidden(failure));
        }
        if !self.allow_availability_fallback {
            return Err(TransportPolicyError::FallbackDisabled);
        }
        let endpoint = self
            .candidates
            .get(self.next)
            .cloned()
            .ok_or(TransportPolicyError::NoCompatibleTransport)?;
        self.next += 1;
        Ok(endpoint)
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TransportPolicyError {
    #[error("HC/2 requires at least one enabled transport")]
    NoEnabledTransport,
    #[error("duplicate transport candidate {0:?}")]
    DuplicateTransport(TransportCandidate),
    #[error("transport authority is empty or exceeds its bound")]
    InvalidAuthority,
    #[error("HC/2 transport {0:?} must require mTLS")]
    InsecureTransport(TransportCandidate),
    #[error("configured maturity exceeds evidence for {0:?}")]
    UnprovenMaturity(TransportCandidate),
    #[error("unsupported HC/2 generation {0}")]
    UnknownGeneration(u16),
    #[error("cluster id is empty or exceeds its bound")]
    InvalidClusterId,
    #[error("discovery cluster identity does not match the verified channel")]
    ClusterIdentityMismatch,
    #[error("unready endpoint {0:?} was advertised")]
    UnreadyAdvertisement(TransportCandidate),
    #[error("client transport preference is empty or contains duplicates")]
    InvalidPreference,
    #[error("no advertised transport satisfies the client policy")]
    NoCompatibleTransport,
    #[error("transport fallback is disabled")]
    FallbackDisabled,
    #[error("fallback after {0:?} would be a downgrade")]
    DowngradeForbidden(TransportFailure),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn endpoint(
        candidate: TransportCandidate,
        authority: &str,
        maturity: AdapterMaturity,
    ) -> TransportEndpoint {
        TransportEndpoint::new(candidate, authority, maturity)
    }

    fn raw_all_transports() -> DiscoveryAdvertisement {
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

    fn all_transports() -> AuthenticatedAdvertisement {
        raw_all_transports()
            .authenticate(VerifiedDiscoveryEvidence::verified_channel("cluster-a"))
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
    fn ordered_policy_falls_back_only_for_safe_failures() {
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
    fn wrong_generation_cannot_be_authenticated() {
        let mut advertisement = raw_all_transports();
        advertisement.generation = HC2_GENERATION + 1;
        assert_eq!(
            advertisement.authenticate(VerifiedDiscoveryEvidence::verified_channel("cluster-a")),
            Err(TransportPolicyError::UnknownGeneration(HC2_GENERATION + 1))
        );
    }

    #[test]
    fn discovery_for_another_cluster_cannot_be_authenticated() {
        assert_eq!(
            raw_all_transports()
                .authenticate(VerifiedDiscoveryEvidence::verified_channel("cluster-b")),
            Err(TransportPolicyError::ClusterIdentityMismatch)
        );
    }

    #[test]
    fn authenticated_wrapper_exposes_read_only_bounded_metadata() {
        let advertisement = all_transports();
        assert_eq!(advertisement.cluster_id(), "cluster-a");
        assert_eq!(advertisement.epoch(), 42);
        assert_eq!(advertisement.generation(), HC2_GENERATION);
        assert_eq!(advertisement.endpoints().len(), 3);
    }
}
