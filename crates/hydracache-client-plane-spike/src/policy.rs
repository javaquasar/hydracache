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
        advertisement.validate(true)?;
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
    pub fn validate(&self, authenticated: bool) -> Result<(), TransportPolicyError> {
        if !authenticated {
            return Err(TransportPolicyError::UnauthenticatedDiscovery);
        }
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
        advertisement: &DiscoveryAdvertisement,
    ) -> Result<(TransportEndpoint, TransportSelection), TransportPolicyError> {
        advertisement.validate(true)?;
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
            if let Some(endpoint) = advertisement.endpoints.iter().find(|endpoint| {
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
    #[error("discovery must arrive through an authenticated channel")]
    UnauthenticatedDiscovery,
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
