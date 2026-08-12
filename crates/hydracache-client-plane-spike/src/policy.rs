//! Fail-closed server and client transport-selection policy for the HC/2 spike.

pub mod signed_discovery;

use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};

use thiserror::Error;

use crate::endpoint::{EndpointOrigin, EndpointParseError, TransportAuthority};
use crate::{ConnectionGeneration, TransportCandidate, HC2_GENERATION};

const MAX_CLUSTER_ID_BYTES: usize = 128;
const MAX_NODE_ID_BYTES: usize = 128;
const MAX_DISCOVERY_NODES: usize = 256;
const MAX_NODE_ENDPOINTS: usize = TransportCandidate::ALL.len();
static NEXT_ATTEMPT_ID: AtomicU64 = AtomicU64::new(1);

fn allocate_attempt_id() -> Result<NonZeroU64, TransportPolicyError> {
    NEXT_ATTEMPT_ID
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current.checked_add(1).filter(|next| *next != 0)
        })
        .ok()
        .and_then(NonZeroU64::new)
        .ok_or(TransportPolicyError::AttemptSequenceExhausted)
}

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
    pub authority: TransportAuthority,
    pub maturity: AdapterMaturity,
    pub require_mtls: bool,
    pub ready: bool,
}

impl TransportEndpoint {
    pub fn parse(
        candidate: TransportCandidate,
        uri: &str,
        server_name: &str,
        maturity: AdapterMaturity,
        origin: EndpointOrigin,
    ) -> Result<Self, EndpointParseError> {
        Ok(Self {
            candidate,
            authority: TransportAuthority::parse(candidate, uri, server_name, origin)?,
            maturity,
            require_mtls: true,
            ready: true,
        })
    }

    fn validate(&self) -> Result<(), TransportPolicyError> {
        if self.authority.candidate() != self.candidate {
            return Err(TransportPolicyError::EndpointTransportMismatch);
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
        Ok(Self { endpoints })
    }

    /// Build one node record from listeners that successfully bound and became ready.
    pub fn advertise_node(
        &self,
        node_id: impl Into<String>,
        node_epoch: u64,
    ) -> Result<NodeAdvertisement, TransportPolicyError> {
        let node = NodeAdvertisement {
            node_id: NodeId::parse(node_id.into())?,
            epoch: node_epoch,
            endpoints: self
                .endpoints
                .iter()
                .filter(|endpoint| endpoint.ready)
                .cloned()
                .collect(),
        };
        node.validate()?;
        Ok(node)
    }
}

/// Stable node identity scoped to one authenticated cluster.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NodeId(String);

impl NodeId {
    pub fn parse(value: impl Into<String>) -> Result<Self, TransportPolicyError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > MAX_NODE_ID_BYTES
            || !value.is_ascii()
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(TransportPolicyError::InvalidNodeId);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One node's bounded, ready client-plane listeners.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeAdvertisement {
    node_id: NodeId,
    epoch: u64,
    endpoints: Vec<TransportEndpoint>,
}

impl NodeAdvertisement {
    pub fn node_id(&self) -> &NodeId {
        &self.node_id
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn endpoints(&self) -> &[TransportEndpoint] {
        &self.endpoints
    }

    fn validate(&self) -> Result<(), TransportPolicyError> {
        if self.epoch == 0 {
            return Err(TransportPolicyError::InvalidNodeEpoch);
        }
        if self.endpoints.is_empty() || self.endpoints.len() > MAX_NODE_ENDPOINTS {
            return Err(TransportPolicyError::InvalidNodeEndpointCount);
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
                return Err(TransportPolicyError::ContradictoryNodeTransport(
                    self.node_id.clone(),
                    endpoint.candidate,
                ));
            }
        }
        Ok(())
    }
}

/// Authenticated discovery payload consumed by an HC/2 client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryAdvertisement {
    pub cluster_id: String,
    pub epoch: u64,
    pub generation: u16,
    nodes: Vec<NodeAdvertisement>,
}

impl DiscoveryAdvertisement {
    pub fn assemble(
        cluster_id: impl Into<String>,
        epoch: u64,
        generation: u16,
        nodes: Vec<NodeAdvertisement>,
    ) -> Result<Self, TransportPolicyError> {
        let advertisement = Self {
            cluster_id: cluster_id.into(),
            epoch,
            generation,
            nodes,
        };
        advertisement.validate_payload()?;
        Ok(advertisement)
    }

    fn validate_payload(&self) -> Result<(), TransportPolicyError> {
        if self.cluster_id.trim().is_empty() || self.cluster_id.len() > MAX_CLUSTER_ID_BYTES {
            return Err(TransportPolicyError::InvalidClusterId);
        }
        if self.generation != HC2_GENERATION {
            return Err(TransportPolicyError::UnknownGeneration(self.generation));
        }
        if self.epoch == 0 {
            return Err(TransportPolicyError::InvalidDiscoveryEpoch);
        }
        if self.nodes.is_empty() || self.nodes.len() > MAX_DISCOVERY_NODES {
            return Err(TransportPolicyError::NoEnabledTransport);
        }
        let mut node_ids = BTreeSet::new();
        let mut authorities = BTreeSet::new();
        for node in &self.nodes {
            node.validate()?;
            if !node_ids.insert(node.node_id.clone()) {
                return Err(TransportPolicyError::DuplicateNode(node.node_id.clone()));
            }
            for endpoint in &node.endpoints {
                if !authorities.insert(endpoint.authority.canonical_uri()) {
                    return Err(TransportPolicyError::ConflictingAuthority);
                }
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

    pub fn nodes(&self) -> &[NodeAdvertisement] {
        &self.inner.nodes
    }
}

/// Result of applying authenticated discovery to the client's monotonic view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoveryUpdate {
    Advanced,
    Unchanged,
}

/// Stateful rollback/equivocation gate retained across reconnects.
#[derive(Debug, Default)]
pub struct DiscoveryState {
    cluster_id: Option<String>,
    highest_epoch: u64,
    node_epochs: BTreeMap<NodeId, u64>,
    accepted: Option<AuthenticatedAdvertisement>,
}

impl DiscoveryState {
    pub fn accept(
        &mut self,
        advertisement: AuthenticatedAdvertisement,
    ) -> Result<DiscoveryUpdate, TransportPolicyError> {
        if let Some(cluster_id) = &self.cluster_id {
            if cluster_id != advertisement.cluster_id() {
                return Err(TransportPolicyError::UnexpectedClusterChange);
            }
        }
        if advertisement.epoch() < self.highest_epoch {
            return Err(TransportPolicyError::DiscoveryRollback);
        }
        if advertisement.epoch() == self.highest_epoch && self.highest_epoch != 0 {
            return if self.accepted.as_ref() == Some(&advertisement) {
                Ok(DiscoveryUpdate::Unchanged)
            } else {
                Err(TransportPolicyError::DiscoveryEquivocation)
            };
        }
        for node in advertisement.nodes() {
            if self
                .node_epochs
                .get(node.node_id())
                .is_some_and(|epoch| node.epoch() < *epoch)
            {
                return Err(TransportPolicyError::NodeEpochRollback(
                    node.node_id().clone(),
                ));
            }
        }

        self.cluster_id = Some(advertisement.cluster_id().to_owned());
        self.highest_epoch = advertisement.epoch();
        for node in advertisement.nodes() {
            self.node_epochs
                .insert(node.node_id().clone(), node.epoch());
        }
        self.accepted = Some(advertisement);
        Ok(DiscoveryUpdate::Advanced)
    }

    pub fn accepted(&self) -> Option<&AuthenticatedAdvertisement> {
        self.accepted.as_ref()
    }

    /// Explicit operator action for intentional cluster replacement.
    pub fn reset_for_cluster_replacement(&mut self) {
        *self = Self::default();
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
    current: AttemptBinding,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AttemptBinding {
    attempt_id: NonZeroU64,
    sequence: u8,
    connection_generation: ConnectionGeneration,
    cluster_id: String,
    discovery_epoch: u64,
    endpoint: TransportEndpoint,
}

/// Current transport attempt. Its provenance is opaque to SDK consumers.
///
/// ```compile_fail
/// use hydracache_client_plane_spike::policy::{AttemptOutcome, TransportFailure};
/// let forged = AttemptOutcome { failure: TransportFailure::AuthenticatedUnsupported };
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportAttempt {
    binding: AttemptBinding,
}

impl TransportAttempt {
    /// Endpoint that the adapter must actually attempt.
    pub fn endpoint(&self) -> &TransportEndpoint {
        &self.binding.endpoint
    }

    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "reserved for the H01 production adapter")
    )]
    pub(crate) fn availability_failure(&self) -> AttemptOutcome {
        AttemptOutcome {
            binding: self.binding.clone(),
            failure: TransportFailure::Availability,
        }
    }

    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "reserved for the H01 production adapter")
    )]
    pub(crate) fn authenticated_unsupported(
        &self,
        evidence: VerifiedAttemptEvidence,
    ) -> Result<AttemptOutcome, TransportPolicyError> {
        if evidence.binding != self.binding {
            return Err(TransportPolicyError::UnverifiedAttemptOutcome);
        }
        Ok(AttemptOutcome {
            binding: self.binding.clone(),
            failure: TransportFailure::AuthenticatedUnsupported,
        })
    }

    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "reserved for the H01 production adapter")
    )]
    pub(crate) fn terminal_failure(
        &self,
        failure: TransportFailure,
    ) -> Result<AttemptOutcome, TransportPolicyError> {
        if failure.permits_fallback() {
            return Err(TransportPolicyError::UnverifiedAttemptOutcome);
        }
        Ok(AttemptOutcome {
            binding: self.binding.clone(),
            failure,
        })
    }
}

/// Opaque result accepted by the fallback state machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttemptOutcome {
    binding: AttemptBinding,
    failure: TransportFailure,
}

/// Proof that an unsupported response was decoded after authenticating the
/// exact peer and endpoint of an active attempt.
#[derive(Debug, Clone)]
pub struct VerifiedAttemptEvidence {
    binding: AttemptBinding,
}

impl VerifiedAttemptEvidence {
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "reserved for the H01 production adapter")
    )]
    pub(crate) fn verified_peer(attempt: &TransportAttempt) -> Self {
        Self {
            binding: attempt.binding.clone(),
        }
    }
}

impl ClientTransportPolicy {
    pub fn begin(
        &self,
        advertisement: &AuthenticatedAdvertisement,
        connection_generation: ConnectionGeneration,
    ) -> Result<(TransportAttempt, TransportSelection), TransportPolicyError> {
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
            if let Some(endpoint) = advertisement.nodes().iter().find_map(|node| {
                node.endpoints().iter().find(|endpoint| {
                    endpoint.candidate == preferred && endpoint.maturity >= minimum_maturity
                })
            }) {
                candidates.push(endpoint.clone());
            }
        }
        let first_endpoint = candidates
            .first()
            .cloned()
            .ok_or(TransportPolicyError::NoCompatibleTransport)?;
        let first = AttemptBinding {
            attempt_id: allocate_attempt_id()?,
            sequence: 1,
            connection_generation,
            cluster_id: advertisement.cluster_id().to_owned(),
            discovery_epoch: advertisement.epoch(),
            endpoint: first_endpoint,
        };
        Ok((
            TransportAttempt {
                binding: first.clone(),
            },
            TransportSelection {
                candidates,
                next: 1,
                allow_availability_fallback,
                current: first,
            },
        ))
    }
}

impl TransportSelection {
    pub fn after_outcome(
        &mut self,
        outcome: AttemptOutcome,
    ) -> Result<TransportAttempt, TransportPolicyError> {
        if outcome.binding != self.current {
            return Err(TransportPolicyError::WrongOrStaleAttempt);
        }
        if !outcome.failure.permits_fallback() {
            return Err(TransportPolicyError::DowngradeForbidden(outcome.failure));
        }
        if !self.allow_availability_fallback {
            return Err(TransportPolicyError::FallbackDisabled);
        }
        let endpoint = self
            .candidates
            .get(self.next)
            .cloned()
            .ok_or(TransportPolicyError::NoCompatibleTransport)?;
        let sequence = self
            .current
            .sequence
            .checked_add(1)
            .ok_or(TransportPolicyError::AttemptSequenceExhausted)?;
        self.next += 1;
        self.current = AttemptBinding {
            attempt_id: allocate_attempt_id()?,
            sequence,
            connection_generation: self.current.connection_generation,
            cluster_id: self.current.cluster_id.clone(),
            discovery_epoch: self.current.discovery_epoch,
            endpoint,
        };
        Ok(TransportAttempt {
            binding: self.current.clone(),
        })
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TransportPolicyError {
    #[error("HC/2 requires at least one enabled transport")]
    NoEnabledTransport,
    #[error("duplicate transport candidate {0:?}")]
    DuplicateTransport(TransportCandidate),
    #[error("parsed endpoint transport does not match its policy entry")]
    EndpointTransportMismatch,
    #[error("HC/2 transport {0:?} must require mTLS")]
    InsecureTransport(TransportCandidate),
    #[error("configured maturity exceeds evidence for {0:?}")]
    UnprovenMaturity(TransportCandidate),
    #[error("unsupported HC/2 generation {0}")]
    UnknownGeneration(u16),
    #[error("cluster id is empty or exceeds its bound")]
    InvalidClusterId,
    #[error("discovery epoch must be non-zero")]
    InvalidDiscoveryEpoch,
    #[error("node id is empty, oversized, non-ASCII, or contains invalid characters")]
    InvalidNodeId,
    #[error("node epoch must be non-zero")]
    InvalidNodeEpoch,
    #[error("node must advertise one bounded unique endpoint per transport")]
    InvalidNodeEndpointCount,
    #[error("duplicate node id {0:?}")]
    DuplicateNode(NodeId),
    #[error("node {0:?} has contradictory entries for {1:?}")]
    ContradictoryNodeTransport(NodeId, TransportCandidate),
    #[error("one canonical endpoint authority is assigned to multiple nodes")]
    ConflictingAuthority,
    #[error("discovery cluster identity does not match the verified channel")]
    ClusterIdentityMismatch,
    #[error("authenticated discovery attempted to change the bound cluster")]
    UnexpectedClusterChange,
    #[error("authenticated discovery epoch rolled back")]
    DiscoveryRollback,
    #[error("authenticated discovery contradicted the accepted same-epoch document")]
    DiscoveryEquivocation,
    #[error("node {0:?} epoch rolled back in a newer discovery document")]
    NodeEpochRollback(NodeId),
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
    #[error("fallback outcome belongs to the wrong or a stale attempt")]
    WrongOrStaleAttempt,
    #[error("unsupported outcome was not proven by the attempt's authenticated peer")]
    UnverifiedAttemptOutcome,
    #[error("bounded transport attempt sequence exhausted")]
    AttemptSequenceExhausted,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn endpoint(
        candidate: TransportCandidate,
        authority: &str,
        maturity: AdapterMaturity,
    ) -> TransportEndpoint {
        let scheme = match candidate {
            TransportCandidate::GrpcBidirectional => "hc2+grpc",
            TransportCandidate::Http2Bidirectional => "hc2+h2",
            TransportCandidate::DedicatedTcpTls => "hc2+tls",
        };
        TransportEndpoint::parse(
            candidate,
            &format!("{scheme}://{authority}"),
            "cache.example",
            maturity,
            EndpointOrigin::AuthenticatedDiscovery,
        )
        .unwrap()
    }

    fn raw_all_transports() -> DiscoveryAdvertisement {
        let node = ServerTransportPolicy::validate(
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
        .advertise_node("node-a", 1)
        .unwrap();
        DiscoveryAdvertisement::assemble("cluster-a", 42, HC2_GENERATION, vec![node]).unwrap()
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
        let node = ServerTransportPolicy::validate(
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
        .advertise_node("node-a", 7)
        .unwrap();
        assert_eq!(node.endpoints().len(), 1);
        assert_eq!(
            node.endpoints()[0].candidate,
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

        let wrong_adapter = TransportEndpoint {
            candidate: TransportCandidate::Http2Bidirectional,
            authority: TransportAuthority::parse(
                TransportCandidate::GrpcBidirectional,
                "hc2+grpc://cache.example:7443",
                "cache.example",
                EndpointOrigin::AuthenticatedDiscovery,
            )
            .unwrap(),
            maturity: AdapterMaturity::Experimental,
            require_mtls: true,
            ready: true,
        };
        assert_eq!(
            ServerTransportPolicy::validate(HC2_GENERATION, vec![wrong_adapter]),
            Err(TransportPolicyError::EndpointTransportMismatch)
        );
    }

    #[test]
    fn pinned_transport_never_falls_back() {
        let advertisement = all_transports();
        let (first, mut selection) =
            ClientTransportPolicy::Pinned(TransportCandidate::GrpcBidirectional)
                .begin(&advertisement, ConnectionGeneration::FIRST)
                .unwrap();
        assert_eq!(
            first.endpoint().candidate,
            TransportCandidate::GrpcBidirectional
        );
        assert_eq!(
            selection.after_outcome(first.availability_failure()),
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
        let (first, mut selection) = policy
            .begin(&advertisement, ConnectionGeneration::FIRST)
            .unwrap();
        assert_eq!(
            first.endpoint().candidate,
            TransportCandidate::GrpcBidirectional
        );
        let second = selection
            .after_outcome(first.availability_failure())
            .unwrap();
        assert_eq!(
            second.endpoint().candidate,
            TransportCandidate::Http2Bidirectional
        );
        let verified = VerifiedAttemptEvidence::verified_peer(&second);
        let third = selection
            .after_outcome(second.authenticated_unsupported(verified).unwrap())
            .unwrap();
        assert_eq!(
            third.endpoint().candidate,
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
            let (attempt, mut selection) = ClientTransportPolicy::Ordered {
                preference: TransportCandidate::ALL.to_vec(),
                minimum_maturity: AdapterMaturity::Experimental,
                allow_availability_fallback: true,
            }
            .begin(&all_transports(), ConnectionGeneration::FIRST)
            .unwrap();
            assert_eq!(
                selection.after_outcome(attempt.terminal_failure(failure).unwrap()),
                Err(TransportPolicyError::DowngradeForbidden(failure))
            );
        }
    }

    #[test]
    fn fallback_rejects_wrong_stale_and_unverified_attempt_provenance() {
        let policy = ClientTransportPolicy::Ordered {
            preference: TransportCandidate::ALL.to_vec(),
            minimum_maturity: AdapterMaturity::Experimental,
            allow_availability_fallback: true,
        };
        let advertisement = all_transports();
        let generation = ConnectionGeneration::FIRST;
        let (first, mut selection) = policy.begin(&advertisement, generation).unwrap();
        let stale_outcome = first.availability_failure();
        let second = selection
            .after_outcome(first.availability_failure())
            .unwrap();

        assert_eq!(
            selection.after_outcome(stale_outcome),
            Err(TransportPolicyError::WrongOrStaleAttempt)
        );
        assert_eq!(
            first.authenticated_unsupported(VerifiedAttemptEvidence::verified_peer(&second)),
            Err(TransportPolicyError::UnverifiedAttemptOutcome)
        );
        assert_eq!(
            first.terminal_failure(TransportFailure::AuthenticatedUnsupported),
            Err(TransportPolicyError::UnverifiedAttemptOutcome)
        );

        let (same_generation_other_attempt, _) = policy.begin(&advertisement, generation).unwrap();
        assert_eq!(
            selection.after_outcome(same_generation_other_attempt.availability_failure()),
            Err(TransportPolicyError::WrongOrStaleAttempt)
        );

        let next_generation = generation.checked_next().unwrap();
        let (other_generation_attempt, _) = policy.begin(&advertisement, next_generation).unwrap();
        assert_eq!(
            selection.after_outcome(other_generation_attempt.availability_failure()),
            Err(TransportPolicyError::WrongOrStaleAttempt)
        );

        let valid_unsupported = second
            .authenticated_unsupported(VerifiedAttemptEvidence::verified_peer(&second))
            .unwrap();
        assert_eq!(
            selection
                .after_outcome(valid_unsupported)
                .unwrap()
                .endpoint()
                .candidate,
            TransportCandidate::GrpcBidirectional
        );
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
        assert_eq!(advertisement.nodes().len(), 1);
        assert_eq!(advertisement.nodes()[0].node_id().as_str(), "node-a");
        assert_eq!(advertisement.nodes()[0].endpoints().len(), 3);
    }

    #[test]
    fn three_nodes_may_advertise_the_same_transport() {
        let mut nodes = Vec::new();
        for (node_id, port, epoch) in [
            ("node-a", 7443, 1),
            ("node-b", 7543, 1),
            ("node-c", 7643, 2),
        ] {
            nodes.push(
                ServerTransportPolicy::validate(
                    HC2_GENERATION,
                    vec![endpoint(
                        TransportCandidate::GrpcBidirectional,
                        &format!("cache.example:{port}"),
                        AdapterMaturity::Preview,
                    )],
                )
                .unwrap()
                .advertise_node(node_id, epoch)
                .unwrap(),
            );
        }
        let advertisement =
            DiscoveryAdvertisement::assemble("cluster-a", 9, HC2_GENERATION, nodes).unwrap();
        assert_eq!(advertisement.nodes.len(), 3);
        assert_eq!(advertisement.nodes[2].epoch(), 2);
    }

    #[test]
    fn duplicate_nodes_conflicting_authorities_and_oversized_documents_fail() {
        let node = ServerTransportPolicy::validate(
            HC2_GENERATION,
            vec![endpoint(
                TransportCandidate::GrpcBidirectional,
                "cache.example:7443",
                AdapterMaturity::Preview,
            )],
        )
        .unwrap()
        .advertise_node("node-a", 1)
        .unwrap();

        assert_eq!(
            DiscoveryAdvertisement::assemble(
                "cluster-a",
                1,
                HC2_GENERATION,
                vec![node.clone(), node.clone()],
            ),
            Err(TransportPolicyError::DuplicateNode(
                NodeId::parse("node-a").unwrap()
            ))
        );

        let mut other = node.clone();
        other.node_id = NodeId::parse("node-b").unwrap();
        assert_eq!(
            DiscoveryAdvertisement::assemble(
                "cluster-a",
                1,
                HC2_GENERATION,
                vec![node.clone(), other],
            ),
            Err(TransportPolicyError::ConflictingAuthority)
        );

        assert_eq!(
            DiscoveryAdvertisement::assemble(
                "cluster-a",
                1,
                HC2_GENERATION,
                vec![node; MAX_DISCOVERY_NODES + 1],
            ),
            Err(TransportPolicyError::NoEnabledTransport)
        );

        let contradictory = NodeAdvertisement {
            node_id: NodeId::parse("node-z").unwrap(),
            epoch: 1,
            endpoints: vec![
                endpoint(
                    TransportCandidate::GrpcBidirectional,
                    "cache.example:7443",
                    AdapterMaturity::Preview,
                ),
                endpoint(
                    TransportCandidate::GrpcBidirectional,
                    "cache.example:7543",
                    AdapterMaturity::Preview,
                ),
            ],
        };
        assert_eq!(
            contradictory.validate(),
            Err(TransportPolicyError::ContradictoryNodeTransport(
                NodeId::parse("node-z").unwrap(),
                TransportCandidate::GrpcBidirectional,
            ))
        );
    }

    #[test]
    fn discovery_state_rejects_replay_equivocation_and_cluster_swap() {
        let first = all_transports();
        let mut state = DiscoveryState::default();
        assert_eq!(state.accept(first.clone()), Ok(DiscoveryUpdate::Advanced));
        assert_eq!(state.accept(first), Ok(DiscoveryUpdate::Unchanged));

        let mut replay = raw_all_transports();
        replay.epoch = 41;
        let replay = replay
            .authenticate(VerifiedDiscoveryEvidence::verified_channel("cluster-a"))
            .unwrap();
        assert_eq!(
            state.accept(replay),
            Err(TransportPolicyError::DiscoveryRollback)
        );

        let mut equivocation = raw_all_transports();
        equivocation.nodes[0].epoch = 2;
        let equivocation = equivocation
            .authenticate(VerifiedDiscoveryEvidence::verified_channel("cluster-a"))
            .unwrap();
        assert_eq!(
            state.accept(equivocation),
            Err(TransportPolicyError::DiscoveryEquivocation)
        );

        let mut other = raw_all_transports();
        other.cluster_id = "cluster-b".into();
        other.epoch = 43;
        let other = other
            .authenticate(VerifiedDiscoveryEvidence::verified_channel("cluster-b"))
            .unwrap();
        assert_eq!(
            state.accept(other.clone()),
            Err(TransportPolicyError::UnexpectedClusterChange)
        );
        state.reset_for_cluster_replacement();
        assert_eq!(state.accept(other), Ok(DiscoveryUpdate::Advanced));
    }

    #[test]
    fn reconnect_rolls_forward_but_never_rolls_back_a_known_node() {
        let mut state = DiscoveryState::default();
        state.accept(all_transports()).unwrap();

        let mut next = raw_all_transports();
        next.epoch = 43;
        next.nodes[0].epoch = 2;
        let next = next
            .authenticate(VerifiedDiscoveryEvidence::verified_channel("cluster-a"))
            .unwrap();
        assert_eq!(state.accept(next), Ok(DiscoveryUpdate::Advanced));
        assert_eq!(state.accepted().unwrap().epoch(), 43);

        let mut stale_node = raw_all_transports();
        stale_node.epoch = 44;
        stale_node.nodes[0].epoch = 1;
        let stale_node = stale_node
            .authenticate(VerifiedDiscoveryEvidence::verified_channel("cluster-a"))
            .unwrap();
        assert_eq!(
            state.accept(stale_node),
            Err(TransportPolicyError::NodeEpochRollback(
                NodeId::parse("node-a").unwrap()
            ))
        );
    }
}
