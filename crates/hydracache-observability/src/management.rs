//! Versioned Management Center 2.0 read-model contracts.
//!
//! These types deliberately describe observations. They do not admit members,
//! choose placement, repair storage, or advance Raft state. Callers must
//! validate a snapshot before publishing it through a management transport.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Current `/management/v1` schema version.
pub const MANAGEMENT_API_SCHEMA_VERSION: u16 = 1;

/// Maximum encoded bytes accepted for an opaque node identity.
pub const MAX_MANAGEMENT_NODE_ID_BYTES: usize = 256;
/// Maximum encoded bytes accepted for an opaque trace or request digest.
pub const MAX_MANAGEMENT_OPAQUE_ID_BYTES: usize = 256;
/// Maximum number of placement candidates in one trace.
pub const MAX_PLACEMENT_CANDIDATES: usize = 512;
/// Maximum number of selected peers in one placement trace.
pub const MAX_PLACEMENT_SELECTED: usize = 64;
/// Maximum number of rejection reasons retained for one candidate.
pub const MAX_PLACEMENT_REASONS_PER_CANDIDATE: usize = 16;
/// Maximum number of labels retained in each placement constraint set.
pub const MAX_PLACEMENT_LABELS: usize = 64;
/// Maximum encoded bytes accepted for one placement label.
pub const MAX_PLACEMENT_LABEL_BYTES: usize = 128;
/// Maximum number of items returned by one management page.
pub const MAX_MANAGEMENT_PAGE_ITEMS: usize = 100;
/// Maximum encoded bytes accepted for an opaque pagination cursor.
pub const MAX_MANAGEMENT_CURSOR_BYTES: usize = 256;
/// Maximum warnings retained in one management response envelope.
pub const MAX_MANAGEMENT_WARNINGS: usize = 32;
/// Maximum number of member rows in one topology snapshot.
pub const MAX_MANAGEMENT_MEMBERS: usize = 100;
/// Maximum number of bounded formation transitions retained per member generation.
pub const MAX_MEMBER_FORMATION_TRANSITIONS: usize = 64;
/// Maximum encoded bytes for product version and configuration digest fields.
pub const MAX_MANAGEMENT_DIAGNOSTIC_STRING_BYTES: usize = 128;

/// Stable warning vocabulary carried by management response envelopes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ManagementWarningCode {
    SourceUnavailable,
    PartialObservation,
    StaleObservation,
    ResultTruncated,
    AuthorityUnavailable,
    StatusNotRetained,
    PeerTimeout,
    PeerIncompatible,
    DuplicateObservation,
    PeerIdentityMismatch,
    #[serde(other)]
    Unknown,
}

/// Bounded warning without raw internal errors or sensitive values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagementWarning {
    pub code: ManagementWarningCode,
    pub affected_count: Option<u64>,
}

/// Common metadata wrapped around every `/management/v1` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagementEnvelope<T> {
    pub schema_version: u16,
    pub observation_seq: u64,
    pub authority_epoch: Option<u64>,
    pub captured_at_unix_ms: u64,
    pub source: ManagementObservationSource,
    pub completeness: ManagementCompleteness,
    pub stale_after_ms: u64,
    pub warnings: Vec<ManagementWarning>,
    pub data: T,
}

impl<T> ManagementEnvelope<T> {
    /// Validate common version, source-truth, and cardinality rules.
    pub fn validate(&self) -> Result<(), ManagementContractError> {
        validate_schema(self.schema_version)?;
        validate_limit("warnings", self.warnings.len(), MAX_MANAGEMENT_WARNINGS)?;
        if self.stale_after_ms == 0 {
            return Err(ManagementContractError::Incoherent {
                field: "stale_after_ms",
                reason: "staleness window must be non-zero",
            });
        }
        if matches!(
            self.source,
            ManagementObservationSource::Unavailable | ManagementObservationSource::Unknown
        ) && self.completeness == ManagementCompleteness::Complete
        {
            return Err(ManagementContractError::Incoherent {
                field: "completeness",
                reason: "an unavailable source cannot be complete",
            });
        }
        if self.completeness == ManagementCompleteness::Unknown {
            return Err(ManagementContractError::Incoherent {
                field: "completeness",
                reason: "unknown completeness cannot be published",
            });
        }
        if self.completeness == ManagementCompleteness::Complete
            && self.warnings.iter().any(|warning| {
                matches!(
                    warning.code,
                    ManagementWarningCode::SourceUnavailable
                        | ManagementWarningCode::PartialObservation
                        | ManagementWarningCode::StaleObservation
                        | ManagementWarningCode::ResultTruncated
                        | ManagementWarningCode::AuthorityUnavailable
                        | ManagementWarningCode::PeerTimeout
                        | ManagementWarningCode::PeerIncompatible
                        | ManagementWarningCode::DuplicateObservation
                        | ManagementWarningCode::PeerIdentityMismatch
                )
            })
        {
            return Err(ManagementContractError::Incoherent {
                field: "warnings",
                reason: "complete response cannot report a missing required source",
            });
        }
        Ok(())
    }
}

/// Opaque server-issued pagination token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OpaqueCursor(String);

impl OpaqueCursor {
    /// Build a cursor. The issuing transport remains responsible for authenticity and expiry.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Borrow the encoded cursor.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Validate only the cross-transport encoded-size contract.
    pub fn validate(&self) -> Result<(), ManagementContractError> {
        validate_opaque_id("cursor", self.as_str(), MAX_MANAGEMENT_CURSOR_BYTES)
    }
}

/// Stable bounded page used by list endpoints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundedPage<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<OpaqueCursor>,
    pub truncated: bool,
}

impl<T> BoundedPage<T> {
    /// Validate the page cardinality and cursor encoding bounds.
    pub fn validate(&self) -> Result<(), ManagementContractError> {
        validate_limit("page.items", self.items.len(), MAX_MANAGEMENT_PAGE_ITEMS)?;
        if let Some(cursor) = &self.next_cursor {
            cursor.validate()?;
        }
        if self.next_cursor.is_some() && !self.truncated {
            return Err(ManagementContractError::Incoherent {
                field: "next_cursor",
                reason: "a continuation cursor requires truncated=true",
            });
        }
        Ok(())
    }
}

/// Capability identifiers used to drive Management Center navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagementCapabilityId {
    ClusterFormation,
    ConsensusProgress,
    PersistenceRecovery,
    #[serde(other)]
    Unknown,
}

/// Availability of one server-side management data source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagementCapabilityAvailability {
    Available,
    Partial,
    Unavailable,
    #[serde(other)]
    Unknown,
}

/// One capability and its stable unavailability reason, if any.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagementCapability {
    pub id: ManagementCapabilityId,
    pub availability: ManagementCapabilityAvailability,
    pub reason: Option<ManagementWarningCode>,
}

/// Stable, sorted capability negotiation payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagementCapabilities {
    pub capabilities: Vec<ManagementCapability>,
}

impl ManagementCapabilities {
    /// Validate stable order and reason/availability consistency.
    pub fn validate(&self) -> Result<(), ManagementContractError> {
        let ids = self
            .capabilities
            .iter()
            .map(|capability| capability.id)
            .collect::<Vec<_>>();
        validate_strict_order("capabilities", &ids)?;
        for capability in &self.capabilities {
            if capability.availability != ManagementCapabilityAvailability::Available
                && capability.reason.is_none()
            {
                return Err(ManagementContractError::Incoherent {
                    field: "capability.reason",
                    reason: "partial or unavailable capability requires a stable reason",
                });
            }
        }
        Ok(())
    }
}

/// Source quality for one management observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagementObservationSource {
    /// Observation was read from the live runtime source named by the source map.
    Live,
    /// Observation was produced by an explicitly modeled source.
    Modeled,
    /// The required source is unavailable.
    Unavailable,
    /// A newer producer supplied a source kind this reader does not understand.
    #[serde(other)]
    Unknown,
}

/// Whether all required inputs were present for one management observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagementCompleteness {
    /// Every required source participated in the observation.
    Complete,
    /// One or more required sources were missing, stale, truncated, or incompatible.
    Partial,
    /// A newer producer supplied a completeness kind this reader does not understand.
    #[serde(other)]
    Unknown,
}

/// Opaque, presentation-safe logical node identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OpaqueNodeIdentity(String);

impl OpaqueNodeIdentity {
    /// Build an identity. Validation occurs when its containing contract is validated.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Borrow the encoded identity.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Discovery progress for one peer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryState {
    Absent,
    Discovered,
    #[serde(other)]
    Unknown,
}

/// Inter-node transport progress for one peer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportState {
    Unreachable,
    Reachable,
    Authenticated,
    #[serde(other)]
    Unknown,
}

/// Authoritative membership admission progress for one peer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdmissionState {
    Pending,
    Admitted,
    Rejected,
    #[serde(other)]
    Unknown,
}

/// Consensus membership role for one peer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagementConsensusRole {
    None,
    Learner,
    Voter,
    #[serde(other)]
    Unknown,
}

/// Authoritative catch-up progress for one peer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatchUpState {
    Behind,
    Current,
    #[serde(other)]
    Unknown,
}

/// Whether one peer may currently serve its configured role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServingState {
    Blocked,
    Serving,
    Draining,
    #[serde(other)]
    Unknown,
}

/// Stable blocker vocabulary for cluster formation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FormationReasonCode {
    ClusterIdentityMismatch,
    DuplicateNodeIdentity,
    GenerationRegression,
    TransportUnreachable,
    PeerUnauthenticated,
    ProtocolIncompatible,
    NotAdmitted,
    LearnerBehind,
    QuorumUnavailable,
    AuthorityUnknown,
    Draining,
    SourceUnavailable,
    #[serde(other)]
    Unknown,
}

/// Read-only formation snapshot for one node generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClusterFormationSnapshot {
    pub schema_version: u16,
    pub node: OpaqueNodeIdentity,
    pub generation: u64,
    pub candidate_observation_seq: Option<u64>,
    pub authority_epoch: Option<u64>,
    pub discovery: DiscoveryState,
    pub transport: TransportState,
    pub admission: AdmissionState,
    pub consensus_role: ManagementConsensusRole,
    pub catch_up: CatchUpState,
    pub serving: ServingState,
    pub blocker: Option<FormationReasonCode>,
    pub source: ManagementObservationSource,
    pub completeness: ManagementCompleteness,
}

/// Stable reachability explanation for the committed-member view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MemberReachabilityReason {
    Healthy,
    Suspected,
    TransportUnreachable,
    SourceUnavailable,
    #[serde(other)]
    Unknown,
}

/// One bounded transition retained only for the current node generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemberFormationTransition {
    pub observation_seq: u64,
    pub authority_epoch: u64,
    pub serving: ServingState,
    pub blocker: Option<FormationReasonCode>,
}

/// Epoch-consistent committed member detail for the Management Center.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManagementMemberDetail {
    pub schema_version: u16,
    pub node: OpaqueNodeIdentity,
    pub generation: u64,
    pub authority_epoch: Option<u64>,
    pub observation_seq: u64,
    pub consensus_role: ManagementConsensusRole,
    pub product_version: Option<String>,
    pub protocol_compatible: Option<bool>,
    pub reachability: TransportState,
    pub reachability_reason: MemberReachabilityReason,
    pub draining: Option<bool>,
    pub uptime_seconds: Option<u64>,
    pub cpu_percent: Option<f64>,
    pub rss_bytes: Option<u64>,
    pub retained_bytes: Option<u64>,
    pub open_fds: Option<u64>,
    pub thread_count: Option<u64>,
    pub task_count: Option<u64>,
    pub client_count: Option<u64>,
    pub partition_count: Option<u64>,
    pub config_digest: Option<String>,
    pub formation: ClusterFormationSnapshot,
    pub timeline: Vec<MemberFormationTransition>,
    pub source: ManagementObservationSource,
    pub completeness: ManagementCompleteness,
}

impl ManagementMemberDetail {
    /// Validate identity, generation/epoch alignment, finite resources and bounded history.
    pub fn validate(&self) -> Result<(), ManagementContractError> {
        validate_schema(self.schema_version)?;
        validate_opaque_id(
            "member.node",
            self.node.as_str(),
            MAX_MANAGEMENT_NODE_ID_BYTES,
        )?;
        self.formation.validate()?;
        if self.node != self.formation.node || self.generation != self.formation.generation {
            return Err(ManagementContractError::Incoherent {
                field: "member.formation",
                reason: "formation identity and generation must match the member row",
            });
        }
        if self.authority_epoch != self.formation.authority_epoch {
            return Err(ManagementContractError::Incoherent {
                field: "member.authority_epoch",
                reason: "member and formation authority epoch must match",
            });
        }
        if self
            .cpu_percent
            .is_some_and(|value| !value.is_finite() || value < 0.0)
        {
            return Err(ManagementContractError::Incoherent {
                field: "member.cpu_percent",
                reason: "CPU percent must be finite and non-negative",
            });
        }
        for (field, value) in [
            ("member.product_version", self.product_version.as_deref()),
            ("member.config_digest", self.config_digest.as_deref()),
        ] {
            if let Some(value) = value {
                validate_opaque_id(field, value, MAX_MANAGEMENT_DIAGNOSTIC_STRING_BYTES)?;
            }
        }
        validate_limit(
            "member.timeline",
            self.timeline.len(),
            MAX_MEMBER_FORMATION_TRANSITIONS,
        )?;
        let mut last_seq = None;
        for transition in &self.timeline {
            if transition.authority_epoch != self.authority_epoch.unwrap_or_default() {
                return Err(ManagementContractError::Incoherent {
                    field: "member.timeline",
                    reason: "timeline contains a mixed authority epoch",
                });
            }
            if last_seq.is_some_and(|previous| transition.observation_seq <= previous) {
                return Err(ManagementContractError::UnstableOrder {
                    field: "member.timeline",
                });
            }
            last_seq = Some(transition.observation_seq);
        }
        Ok(())
    }
}

/// One per-member partition assignment count; partition identifiers stay out of metrics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartitionMemberCount {
    pub node: OpaqueNodeIdentity,
    pub primary: u64,
    pub backup: u64,
}

/// Epoch-consistent aggregate partition ownership and repair view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagementPartitionSnapshot {
    pub schema_version: u16,
    pub authority_epoch: Option<u64>,
    pub observation_seq: u64,
    pub total: u64,
    pub assigned: Option<u64>,
    pub unassigned: Option<u64>,
    pub distribution: Vec<PartitionMemberCount>,
    pub under_replicated: u64,
    pub zone_underspread: u64,
    pub repair_debt: u64,
    pub reshard_phase: String,
    pub reshard_moves_inflight: u64,
    pub backfill_lag: u64,
    pub placement_trace_id: Option<String>,
    pub source: ManagementObservationSource,
    pub completeness: ManagementCompleteness,
}

impl ManagementPartitionSnapshot {
    /// Validate aggregate arithmetic, stable distribution and opaque trace identifier.
    pub fn validate(&self) -> Result<(), ManagementContractError> {
        validate_schema(self.schema_version)?;
        validate_limit(
            "partition.distribution",
            self.distribution.len(),
            MAX_MANAGEMENT_MEMBERS,
        )?;
        let nodes = self
            .distribution
            .iter()
            .map(|entry| &entry.node)
            .collect::<Vec<_>>();
        validate_strict_order("partition.distribution", &nodes)?;
        if let (Some(assigned), Some(unassigned)) = (self.assigned, self.unassigned) {
            if assigned.saturating_add(unassigned) != self.total {
                return Err(ManagementContractError::Incoherent {
                    field: "partition.counts",
                    reason: "assigned plus unassigned must equal total",
                });
            }
            let distributed = self
                .distribution
                .iter()
                .map(|entry| entry.primary)
                .fold(0_u64, u64::saturating_add);
            if !self.distribution.is_empty() && distributed != assigned {
                return Err(ManagementContractError::Incoherent {
                    field: "partition.distribution",
                    reason: "primary distribution must sum to assigned partitions",
                });
            }
        }
        if let Some(trace_id) = &self.placement_trace_id {
            validate_opaque_id(
                "partition.placement_trace_id",
                trace_id,
                MAX_MANAGEMENT_OPAQUE_ID_BYTES,
            )?;
        }
        Ok(())
    }
}

impl ClusterFormationSnapshot {
    /// Validate cross-field authority and readiness invariants.
    pub fn validate(&self) -> Result<(), ManagementContractError> {
        validate_schema(self.schema_version)?;
        validate_opaque_id("node", self.node.as_str(), MAX_MANAGEMENT_NODE_ID_BYTES)?;

        if self.admission == AdmissionState::Admitted && self.authority_epoch.is_none() {
            return Err(ManagementContractError::Incoherent {
                field: "authority_epoch",
                reason: "admitted formation requires an authority epoch",
            });
        }
        if matches!(
            self.consensus_role,
            ManagementConsensusRole::Learner | ManagementConsensusRole::Voter
        ) && self.admission != AdmissionState::Admitted
        {
            return Err(ManagementContractError::Incoherent {
                field: "consensus_role",
                reason: "learner or voter role requires committed admission",
            });
        }
        if self.serving == ServingState::Serving
            && (self.source != ManagementObservationSource::Live
                || self.transport != TransportState::Authenticated
                || self.admission != AdmissionState::Admitted
                || !matches!(
                    self.consensus_role,
                    ManagementConsensusRole::Learner | ManagementConsensusRole::Voter
                )
                || self.catch_up != CatchUpState::Current)
        {
            return Err(ManagementContractError::Incoherent {
                field: "serving",
                reason: "serving requires live authenticated, admitted, consensus-current state",
            });
        }
        if self.completeness == ManagementCompleteness::Complete
            && (self.source == ManagementObservationSource::Unknown
                || self.discovery == DiscoveryState::Unknown
                || self.transport == TransportState::Unknown
                || self.admission == AdmissionState::Unknown
                || self.consensus_role == ManagementConsensusRole::Unknown
                || self.catch_up == CatchUpState::Unknown
                || self.serving == ServingState::Unknown)
        {
            return Err(ManagementContractError::Incoherent {
                field: "completeness",
                reason: "complete formation cannot contain an unknown required dimension",
            });
        }
        Ok(())
    }
}

/// Stable reason vocabulary for placement candidate decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlacementReasonCode {
    WrongCluster,
    WrongRole,
    NotAdmitted,
    Unreachable,
    StaleGeneration,
    ProtocolIncompatible,
    CapabilityMissing,
    CapacityInsufficient,
    ZoneConflict,
    HostConflict,
    RequiredLabelMissing,
    ExcludedLabel,
    AlreadySelected,
    IgnoredByPlan,
    QuorumWouldBeUnsafe,
    #[serde(other)]
    Unknown,
}

/// Placement lifecycle as observed by the management plane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlacementOutcome {
    Rejected,
    Proposed,
    Committed,
    Applied,
    Stale,
    #[serde(other)]
    Unknown,
}

/// Bounded, redacted summary of placement constraints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct BoundedPlacementConstraints {
    pub required_labels: Vec<String>,
    pub excluded_labels: Vec<String>,
    pub required_zones: u16,
    pub unique_hosts: bool,
    pub truncated: bool,
}

impl BoundedPlacementConstraints {
    fn validate(&self) -> Result<(), ManagementContractError> {
        validate_labels("required_labels", &self.required_labels)?;
        validate_labels("excluded_labels", &self.excluded_labels)?;
        Ok(())
    }
}

/// One selected or rejected placement candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlacementCandidateDecision {
    pub node: OpaqueNodeIdentity,
    pub selected: bool,
    pub reasons: Vec<PlacementReasonCode>,
}

/// Bounded page embedded in a placement decision trace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PlacementCandidatePage {
    pub items: Vec<PlacementCandidateDecision>,
    pub truncated: bool,
}

/// Deterministic read-only explanation of one placement decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlacementDecisionTrace {
    pub schema_version: u16,
    pub trace_id: String,
    pub topology_epoch: u64,
    pub request_digest: String,
    pub requested_replicas: u16,
    pub constraints: BoundedPlacementConstraints,
    pub candidates: PlacementCandidatePage,
    pub selected: Vec<OpaqueNodeIdentity>,
    pub tie_break_seed: Option<u64>,
    pub outcome: PlacementOutcome,
    pub commit_index: Option<u64>,
    pub applied_index: Option<u64>,
    pub reason: Option<PlacementReasonCode>,
    pub source: ManagementObservationSource,
    pub completeness: ManagementCompleteness,
}

impl PlacementDecisionTrace {
    /// Validate bounds, stable ordering, selection agreement, and progress coherence.
    pub fn validate(&self) -> Result<(), ManagementContractError> {
        validate_schema(self.schema_version)?;
        validate_opaque_id("trace_id", &self.trace_id, MAX_MANAGEMENT_OPAQUE_ID_BYTES)?;
        validate_opaque_id(
            "request_digest",
            &self.request_digest,
            MAX_MANAGEMENT_OPAQUE_ID_BYTES,
        )?;
        if self.requested_replicas == 0 {
            return Err(ManagementContractError::Incoherent {
                field: "requested_replicas",
                reason: "placement must request at least one replica",
            });
        }
        self.constraints.validate()?;
        validate_limit(
            "candidates",
            self.candidates.items.len(),
            MAX_PLACEMENT_CANDIDATES,
        )?;
        validate_limit("selected", self.selected.len(), MAX_PLACEMENT_SELECTED)?;
        if self.selected.len() > usize::from(self.requested_replicas) {
            return Err(ManagementContractError::Incoherent {
                field: "selected",
                reason: "selected peers exceed requested replica count",
            });
        }

        validate_strict_order("selected", &self.selected)?;
        for pair in self.candidates.items.windows(2) {
            let [left, right] = pair else { continue };
            let stable = (left.selected && !right.selected)
                || (left.selected == right.selected && left.node < right.node);
            if !stable {
                return Err(ManagementContractError::UnstableOrder {
                    field: "candidates",
                });
            }
        }

        for candidate in &self.candidates.items {
            validate_opaque_id(
                "candidate.node",
                candidate.node.as_str(),
                MAX_MANAGEMENT_NODE_ID_BYTES,
            )?;
            validate_limit(
                "candidate.reasons",
                candidate.reasons.len(),
                MAX_PLACEMENT_REASONS_PER_CANDIDATE,
            )?;
            validate_strict_order("candidate.reasons", &candidate.reasons)?;
            let listed_selected = self.selected.binary_search(&candidate.node).is_ok();
            if listed_selected != candidate.selected {
                return Err(ManagementContractError::Incoherent {
                    field: "candidate.selected",
                    reason: "candidate selected flag disagrees with selected peer list",
                });
            }
            if !candidate.selected && candidate.reasons.is_empty() {
                return Err(ManagementContractError::Incoherent {
                    field: "candidate.reasons",
                    reason: "rejected candidate requires at least one reason",
                });
            }
        }

        if let (Some(commit), Some(applied)) = (self.commit_index, self.applied_index) {
            if applied > commit {
                return Err(ManagementContractError::Incoherent {
                    field: "applied_index",
                    reason: "applied index cannot exceed commit index",
                });
            }
        }
        if matches!(
            self.outcome,
            PlacementOutcome::Committed | PlacementOutcome::Applied
        ) && self.commit_index.is_none()
        {
            return Err(ManagementContractError::Incoherent {
                field: "commit_index",
                reason: "committed or applied placement requires commit progress",
            });
        }
        if self.outcome == PlacementOutcome::Applied
            && (self.applied_index.is_none()
                || self.selected.len() != usize::from(self.requested_replicas))
        {
            return Err(ManagementContractError::Incoherent {
                field: "outcome",
                reason: "applied placement requires applied progress and every requested replica",
            });
        }
        Ok(())
    }
}

/// Count with explicit truncation/saturation disclosure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct BoundedCount {
    pub value: u64,
    pub truncated: bool,
}

impl BoundedCount {
    pub const fn exact(value: u64) -> Self {
        Self {
            value,
            truncated: false,
        }
    }
}

/// Scope affected by one durable recovery attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryScope {
    Node,
    Namespace,
    Region,
    RaftMetadata,
    #[serde(other)]
    Unknown,
}

/// Operator-facing recovery outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryOutcome {
    Clean,
    Repaired,
    Degraded,
    Refused,
    #[serde(other)]
    Unknown,
}

/// Current/terminal phase of one recovery attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryPhase {
    Discover,
    Validate,
    Rebuild,
    Reconcile,
    Complete,
    #[serde(other)]
    Unknown,
}

/// Artifact class involved in recovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryArtifactKind {
    DurableValueStore,
    RaftLog,
    RaftSnapshot,
    ClusterCheckpoint,
    Backup,
    DerivedIndex,
    #[serde(other)]
    Unknown,
}

/// Repair progress associated with recovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairState {
    NotRequired,
    Pending,
    Running,
    Complete,
    Failed,
    #[serde(other)]
    Unknown,
}

/// Stable reason vocabulary for durable recovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RecoveryReasonCode {
    ArtifactMissing,
    UnsupportedFormat,
    ChecksumMismatch,
    TruncatedArtifact,
    ForeignIdentity,
    StaleWatermark,
    Timeout,
    IoError,
    DiskFull,
    RepairSourceUnavailable,
    RepairFailed,
    ReconciliationFailed,
    StatusNotRetained,
    #[serde(other)]
    Unknown,
}

/// Version/epoch position verified by recovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryWatermark {
    pub authority_epoch: u64,
    pub version: u64,
}

/// Bounded, privacy-safe result of one recovery attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurableRecoveryStatus {
    pub schema_version: u16,
    pub scope: RecoveryScope,
    pub outcome: RecoveryOutcome,
    pub phase: RecoveryPhase,
    pub artifact: RecoveryArtifactKind,
    pub artifact_format_version: Option<u32>,
    pub validated_watermark: Option<RecoveryWatermark>,
    pub records_checked: BoundedCount,
    pub records_recovered: BoundedCount,
    pub records_discarded: BoundedCount,
    pub corrupt_records: BoundedCount,
    pub repair: RepairState,
    pub reason: Option<RecoveryReasonCode>,
    pub started_at_unix_ms: Option<u64>,
    pub completed_at_unix_ms: Option<u64>,
    pub source: ManagementObservationSource,
    pub completeness: ManagementCompleteness,
}

impl DurableRecoveryStatus {
    /// Validate that the advertised outcome is supported by its evidence.
    pub fn validate(&self) -> Result<(), ManagementContractError> {
        validate_schema(self.schema_version)?;
        if self.outcome == RecoveryOutcome::Clean
            && (self.phase != RecoveryPhase::Complete
                || !matches!(
                    self.repair,
                    RepairState::NotRequired | RepairState::Complete
                )
                || self.records_discarded.value != 0
                || self.corrupt_records.value != 0
                || self.records_checked.truncated
                || self.records_recovered.truncated
                || self.records_discarded.truncated
                || self.corrupt_records.truncated
                || self.completeness != ManagementCompleteness::Complete
                || self.source != ManagementObservationSource::Live)
        {
            return Err(ManagementContractError::Incoherent {
                field: "outcome",
                reason:
                    "clean recovery requires complete live verification with no loss or corruption",
            });
        }
        if self.outcome == RecoveryOutcome::Repaired
            && (self.phase != RecoveryPhase::Complete
                || self.repair != RepairState::Complete
                || (self.records_recovered.value == 0
                    && self.records_discarded.value == 0
                    && self.corrupt_records.value == 0))
        {
            return Err(ManagementContractError::Incoherent {
                field: "outcome",
                reason: "repaired recovery requires completed repair and retained defect evidence",
            });
        }
        if self.outcome == RecoveryOutcome::Refused && self.reason.is_none() {
            return Err(ManagementContractError::Incoherent {
                field: "reason",
                reason: "refused recovery requires a stable reason code",
            });
        }
        Ok(())
    }
}

/// Coherent Raft progress for one node generation and authority epoch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsensusProgressSnapshot {
    pub schema_version: u16,
    pub node: OpaqueNodeIdentity,
    pub generation: u64,
    pub authority_epoch: Option<u64>,
    pub commit_index: Option<u64>,
    pub applied_index: Option<u64>,
    pub last_snapshot_index: Option<u64>,
    pub catch_up_target: Option<u64>,
    pub source: ManagementObservationSource,
    pub completeness: ManagementCompleteness,
}

impl ConsensusProgressSnapshot {
    /// Validate monotone relationships between snapshot, apply, commit, and target positions.
    pub fn validate(&self) -> Result<(), ManagementContractError> {
        validate_schema(self.schema_version)?;
        validate_opaque_id("node", self.node.as_str(), MAX_MANAGEMENT_NODE_ID_BYTES)?;
        if let (Some(commit), Some(applied)) = (self.commit_index, self.applied_index) {
            if applied > commit {
                return Err(ManagementContractError::Incoherent {
                    field: "applied_index",
                    reason: "applied index cannot exceed commit index",
                });
            }
        }
        if let (Some(snapshot), Some(applied)) = (self.last_snapshot_index, self.applied_index) {
            if snapshot > applied {
                return Err(ManagementContractError::Incoherent {
                    field: "last_snapshot_index",
                    reason: "installed snapshot cannot exceed applied progress",
                });
            }
        }
        if let (Some(target), Some(applied)) = (self.catch_up_target, self.applied_index) {
            if target < applied {
                return Err(ManagementContractError::Incoherent {
                    field: "catch_up_target",
                    reason: "catch-up target cannot trail applied progress",
                });
            }
        }
        Ok(())
    }

    /// Return apply lag only for a complete coherent observation with both positions.
    pub fn apply_lag(&self) -> Option<u64> {
        if self.completeness != ManagementCompleteness::Complete || self.validate().is_err() {
            return None;
        }
        Some(self.commit_index?.saturating_sub(self.applied_index?))
    }
}

/// Validation failure for a Management Center read-model contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManagementContractError {
    UnsupportedSchema {
        found: u16,
        expected: u16,
    },
    InvalidOpaqueId {
        field: &'static str,
        max_bytes: usize,
        actual_bytes: usize,
    },
    LimitExceeded {
        field: &'static str,
        limit: usize,
        actual: usize,
    },
    UnstableOrder {
        field: &'static str,
    },
    Incoherent {
        field: &'static str,
        reason: &'static str,
    },
}

impl fmt::Display for ManagementContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSchema { found, expected } => write!(
                formatter,
                "unsupported management schema version {found}; expected {expected}"
            ),
            Self::InvalidOpaqueId {
                field,
                max_bytes,
                actual_bytes,
            } => write!(
                formatter,
                "invalid {field}: expected 1..={max_bytes} bytes, got {actual_bytes}"
            ),
            Self::LimitExceeded {
                field,
                limit,
                actual,
            } => write!(
                formatter,
                "management {field} limit exceeded: maximum {limit}, got {actual}"
            ),
            Self::UnstableOrder { field } => {
                write!(formatter, "management {field} must be strictly ordered")
            }
            Self::Incoherent { field, reason } => {
                write!(formatter, "incoherent management {field}: {reason}")
            }
        }
    }
}

impl std::error::Error for ManagementContractError {}

fn validate_schema(schema_version: u16) -> Result<(), ManagementContractError> {
    if schema_version != MANAGEMENT_API_SCHEMA_VERSION {
        return Err(ManagementContractError::UnsupportedSchema {
            found: schema_version,
            expected: MANAGEMENT_API_SCHEMA_VERSION,
        });
    }
    Ok(())
}

fn validate_opaque_id(
    field: &'static str,
    value: &str,
    max_bytes: usize,
) -> Result<(), ManagementContractError> {
    let actual_bytes = value.len();
    if actual_bytes == 0 || actual_bytes > max_bytes {
        return Err(ManagementContractError::InvalidOpaqueId {
            field,
            max_bytes,
            actual_bytes,
        });
    }
    Ok(())
}

fn validate_limit(
    field: &'static str,
    actual: usize,
    limit: usize,
) -> Result<(), ManagementContractError> {
    if actual > limit {
        return Err(ManagementContractError::LimitExceeded {
            field,
            limit,
            actual,
        });
    }
    Ok(())
}

fn validate_strict_order<T: Ord>(
    field: &'static str,
    values: &[T],
) -> Result<(), ManagementContractError> {
    if values.windows(2).any(|window| window[0] >= window[1]) {
        return Err(ManagementContractError::UnstableOrder { field });
    }
    Ok(())
}

fn validate_labels(field: &'static str, labels: &[String]) -> Result<(), ManagementContractError> {
    validate_limit(field, labels.len(), MAX_PLACEMENT_LABELS)?;
    validate_strict_order(field, labels)?;
    for label in labels {
        validate_opaque_id(field, label, MAX_PLACEMENT_LABEL_BYTES)?;
    }
    Ok(())
}
