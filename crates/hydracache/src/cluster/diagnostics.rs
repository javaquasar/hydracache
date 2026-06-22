use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterDiagnostics {
    /// Cluster name.
    pub cluster_name: String,
    /// Local runtime role.
    pub role: ClusterRole,
    /// Local node id.
    pub node_id: ClusterNodeId,
    /// Local process generation.
    pub generation: ClusterGeneration,
    /// Current cluster epoch observed by the in-memory model.
    pub epoch: ClusterEpoch,
    /// Number of admitted member nodes.
    pub member_count: usize,
    /// Number of connected clients.
    pub client_count: usize,
    /// Configured bootstrap addresses.
    pub bootstrap: Vec<String>,
    /// Whether this cache has an attached in-memory cluster runtime.
    pub connected: bool,
    /// Number of active invalidation bus receivers.
    pub invalidation_subscribers: usize,
    /// Number of active cluster membership event subscribers.
    pub membership_subscribers: usize,
    /// Local embedded runtime lifecycle snapshot.
    pub lifecycle: ClusterLifecycleDiagnostics,
}

impl ClusterDiagnostics {
    /// Return whether this diagnostics snapshot belongs to a local cache role.
    pub fn is_local_role(&self) -> bool {
        self.role == ClusterRole::Local
    }

    /// Return whether this diagnostics snapshot belongs to a client runtime.
    pub fn is_client_role(&self) -> bool {
        self.role == ClusterRole::Client
    }

    /// Return whether this diagnostics snapshot belongs to a member runtime.
    pub fn is_member_role(&self) -> bool {
        self.role == ClusterRole::Member
    }

    /// Return the total number of admitted members and connected clients.
    pub fn participant_count(&self) -> usize {
        self.member_count.saturating_add(self.client_count)
    }

    /// Return the number of configured bootstrap addresses.
    pub fn bootstrap_count(&self) -> usize {
        self.bootstrap.len()
    }

    /// Return whether at least one member is currently admitted.
    pub fn has_members(&self) -> bool {
        self.member_count > 0
    }

    /// Return whether at least one client is currently connected.
    pub fn has_clients(&self) -> bool {
        self.client_count > 0
    }

    /// Return whether at least one bootstrap address is configured.
    pub fn has_bootstrap(&self) -> bool {
        !self.bootstrap.is_empty()
    }

    /// Return whether the invalidation bus has active receivers.
    pub fn has_invalidation_subscribers(&self) -> bool {
        self.invalidation_subscribers > 0
    }

    /// Return whether the membership event bus has active receivers.
    pub fn has_membership_subscribers(&self) -> bool {
        self.membership_subscribers > 0
    }

    /// Return whether the current view contains more than one participant.
    pub fn has_multiple_participants(&self) -> bool {
        self.participant_count() > 1
    }

    /// Return whether this runtime appears connected to a usable cluster view.
    pub fn is_operational(&self) -> bool {
        self.connected && self.lifecycle.is_running() && self.participant_count() > 0
    }
}

/// Machine-readable reason used by [`ClusterHealthState`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ClusterHealthReason {
    /// At least one background cluster component is not running.
    LifecycleNotRunning,
    /// The runtime sees no admitted members or connected clients.
    NoParticipants,
    /// The distributed invalidation receiver lagged behind its source stream.
    LaggedReceivers { count: u64 },
    /// Transport frames could not be decoded.
    DecodeErrors { count: u64 },
    /// Publishing an invalidation to the bus failed.
    PublishFailures { count: u64 },
    /// The invalidation receiver stream closed.
    ReceiverClosed { count: u64 },
    /// A stale process generation was fenced off.
    StaleGenerationRejections { count: u64 },
    /// Peer-fetch or owner-load transport auth rejected a caller.
    PeerFetchAuthFailures { count: u64 },
    /// Peer-fetch or owner-load transport wire compatibility rejected a caller.
    WireVersionRejections { count: u64 },
    /// Gossip state was reset recently enough to matter for staging.
    GossipResetRecent {
        /// Age of the most recent observed tombstone/reset signal.
        tombstone_age_ms: u64,
        /// Number of observed gossip resets since process start.
        reset_count: u64,
    },
}

/// Derived staging health state with machine-readable reasons.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
#[non_exhaustive]
pub enum ClusterHealthState {
    /// All checked staging invariants hold.
    Healthy,
    /// The runtime is usable, but at least one soft signal is degraded.
    Degraded { reasons: Vec<ClusterHealthReason> },
    /// The runtime is not safe to run staging traffic against.
    NotReady { reasons: Vec<ClusterHealthReason> },
}

impl ClusterHealthState {
    /// Return `true` only when the derived state is healthy.
    pub fn ready_for_staging(&self) -> bool {
        matches!(self, Self::Healthy)
    }

    /// Return the machine-readable reasons, if the state is not healthy.
    pub fn reasons(&self) -> &[ClusterHealthReason] {
        match self {
            Self::Healthy => &[],
            Self::Degraded { reasons } | Self::NotReady { reasons } => reasons,
        }
    }
}

/// Point-in-time owner/remote/hot fill counters for cluster staging.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ClusterFillCounters {
    /// This node owned the key and loaded it from the origin.
    pub owner_load_success: u64,
    /// Owner-side origin loads that returned an error.
    pub owner_load_errors: u64,
    /// This node fetched encoded bytes from the owner over peer-fetch.
    pub remote_fetch_success: u64,
    /// Remote peer-fetch attempts that failed.
    pub remote_fetch_errors: u64,
    /// This node served a non-owned hot-copy without contacting the owner.
    pub hot_cache_hits: u64,
}

impl ClusterFillCounters {
    /// Return the total number of successful fill/hit observations.
    pub fn successful_events(&self) -> u64 {
        self.owner_load_success
            .saturating_add(self.remote_fetch_success)
            .saturating_add(self.hot_cache_hits)
    }

    /// Return the total number of owner/remote fill errors.
    pub fn error_events(&self) -> u64 {
        self.owner_load_errors
            .saturating_add(self.remote_fetch_errors)
    }
}

/// Three-part groupcache-style counters for pilot dashboards.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ClusterCacheCounters {
    /// This node loaded a key for which it was the owner.
    pub owner_load_total: u64,
    /// This node fetched encoded bytes from another owner.
    pub remote_fetch_total: u64,
    /// This node served a borrowed hot near-cache copy.
    pub hot_cache_hit_total: u64,
}

impl From<ClusterFillCounters> for ClusterCacheCounters {
    fn from(value: ClusterFillCounters) -> Self {
        Self {
            owner_load_total: value.owner_load_success,
            remote_fetch_total: value.remote_fetch_success,
            hot_cache_hit_total: value.hot_cache_hits,
        }
    }
}

/// Additional staging counters that do not belong to local cache stats.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ClusterStagingCounters {
    /// Peer-fetch/owner-load auth failures observed by the staging gate.
    pub peer_fetch_auth_failures: u64,
    /// Wire-version rejections observed by the staging gate.
    pub wire_version_rejections: u64,
    /// Stale-generation publish/admission attempts rejected by fencing.
    pub stale_generation_rejected: u64,
    /// Age of the most recent gossip tombstone/reset signal.
    pub tombstone_age_ms: u64,
    /// Number of observed gossip resets since process start.
    pub gossip_reset_count: u64,
    /// Quorum/read-after-write barrier timeouts observed by pilot reads.
    pub barrier_timeouts: u64,
    /// Near-cache conservative invalidations caused by watermark repair.
    pub near_cache_conservative_invalidations: u64,
    /// Lifecycle graceful stop events observed by pilot probes.
    pub lifecycle_stop_count: u64,
    /// Lifecycle restart/rejoin events observed by pilot probes.
    pub lifecycle_restart_count: u64,
}

/// Boolean readiness contract for a controlled internal production pilot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ClusterPilotReadiness {
    /// Declared transport-security posture.
    pub transport_posture: TransportPosture,
    /// Whether at least one member is admitted.
    pub has_members: bool,
    /// Number of admitted member nodes.
    pub member_count: usize,
    /// Whether the member count is inside the supported 2..=5 pilot range.
    pub within_supported_size: bool,
    /// Whether strict current wire compatibility is enabled.
    pub strict_wire_compatibility: bool,
    /// Whether invalidation diagnostics are free of hard errors.
    pub diagnostics_clean: bool,
    /// Whether the local cluster runtime lifecycle is running.
    pub lifecycle_operational: bool,
    /// Whether at least one authoritative topology epoch has been committed.
    pub topology_committed: bool,
}

impl ClusterPilotReadiness {
    /// Single boolean gate used by tests, actuator, and release docs.
    pub fn is_pilot_ready(&self) -> bool {
        self.transport_posture.is_safe()
            && self.has_members
            && self.within_supported_size
            && self.strict_wire_compatibility
            && self.diagnostics_clean
            && self.lifecycle_operational
            && self.topology_committed
    }

    /// Human-facing highlights that should be rendered loudly by adapters.
    pub fn highlights(&self) -> Vec<&'static str> {
        self.transport_posture.highlight().into_iter().collect()
    }
}

/// Dashboard-ready pilot snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ClusterPilotReport {
    /// Boolean readiness gates.
    pub readiness: ClusterPilotReadiness,
    /// Three-part owner/remote/hot cache counters.
    pub counters: ClusterCacheCounters,
    /// Current metadata epoch.
    pub epoch: u64,
    /// Local process generation.
    pub generation: u64,
    /// Partition-table stamp for ownership-view drift detection.
    pub stamp: u64,
    /// Invalidation messages published by this cache.
    pub invalidations_published: u64,
    /// Invalidation messages received by this cache.
    pub invalidations_received: u64,
    /// Invalidation messages applied by this cache.
    pub invalidations_applied: u64,
    /// Invalidation receiver lag events.
    pub invalidation_lagged: u64,
    /// Invalidation decode errors.
    pub decode_errors: u64,
    /// Invalidation publish failures.
    pub publish_failures: u64,
    /// Invalidation receiver closed events.
    pub receiver_closed: u64,
    /// Owner-load successes.
    pub owner_load_success: u64,
    /// Owner-load errors.
    pub owner_load_errors: u64,
    /// Remote peer-fetch successes.
    pub remote_fetch_success: u64,
    /// Remote peer-fetch errors.
    pub remote_fetch_errors: u64,
    /// Auth failures observed by pilot probes.
    pub auth_failures: u64,
    /// Wire-version failures observed by pilot probes.
    pub wire_version_failures: u64,
    /// Stale-generation rejections.
    pub stale_generation_rejections: u64,
    /// Barrier timeouts.
    pub barrier_timeouts: u64,
    /// Near-cache conservative invalidations caused by repair.
    pub near_cache_conservative_invalidations: u64,
    /// Lifecycle stop count.
    pub lifecycle_stop_count: u64,
    /// Lifecycle restart count.
    pub lifecycle_restart_count: u64,
    /// Declared transport-security posture.
    pub transport_posture: TransportPosture,
    /// Loud actuator highlights such as `AUTH MISSING`.
    pub highlights: Vec<String>,
}

/// Staging-focused health summary derived from diagnostics and counters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ClusterStagingHealth {
    /// Local runtime role.
    pub role: ClusterRole,
    /// Local node id.
    pub node_id: String,
    /// Whether this runtime is connected to a cluster view.
    pub connected: bool,
    /// Number of admitted member nodes.
    pub member_count: usize,
    /// Number of connected clients.
    pub client_count: usize,
    /// Current metadata epoch.
    pub epoch: u64,
    /// Local process generation.
    pub generation: u64,
    /// Invalidation messages published by this cache.
    pub invalidations_published: u64,
    /// Invalidation messages received by this cache.
    pub invalidations_received: u64,
    /// Invalidation messages applied by this cache.
    pub invalidations_applied: u64,
    /// Invalidation receiver lag events.
    pub lagged_receivers: u64,
    /// Invalidation decode errors.
    pub decode_errors: u64,
    /// Invalidation publish failures.
    pub publish_failures: u64,
    /// Invalidation receiver closed events.
    pub receiver_closed: u64,
    /// Owner-side origin loads that returned a value.
    pub owner_load_success: u64,
    /// Owner-side origin loads that returned an error.
    pub owner_load_errors: u64,
    /// Remote peer-fetch calls that returned a value.
    pub remote_fetch_success: u64,
    /// Remote peer-fetch calls that failed.
    pub remote_fetch_errors: u64,
    /// Hot near-cache hits for non-owned values.
    pub hot_cache_hits: u64,
    /// Auth failures observed by peer-fetch/owner-load staging checks.
    pub peer_fetch_auth_failures: u64,
    /// Wire-version rejections observed by staging checks.
    pub wire_version_rejections: u64,
    /// Stale-generation attempts rejected by fencing.
    pub stale_generation_rejected: u64,
    /// Age of the most recent gossip tombstone/reset signal.
    pub tombstone_age_ms: u64,
    /// Number of observed gossip resets since process start.
    pub gossip_reset_count: u64,
    /// Quorum/read-after-write barrier timeouts observed by pilot reads.
    pub barrier_timeouts: u64,
    /// Near-cache conservative invalidations caused by watermark repair.
    pub near_cache_conservative_invalidations: u64,
    /// Lifecycle graceful stop events observed by pilot probes.
    pub lifecycle_stop_count: u64,
    /// Lifecycle restart/rejoin events observed by pilot probes.
    pub lifecycle_restart_count: u64,
    /// Derived overall staging health state.
    pub state: ClusterHealthState,
}

impl ClusterStagingHealth {
    /// Derive staging health from cluster diagnostics and logical counters.
    pub fn from_parts(
        diagnostics: ClusterDiagnostics,
        stats: CacheStats,
        fill: ClusterFillCounters,
        staging: ClusterStagingCounters,
    ) -> Self {
        let state = derive_cluster_health_state(&diagnostics, &stats, &staging);
        Self {
            role: diagnostics.role,
            node_id: diagnostics.node_id.to_string(),
            connected: diagnostics.connected,
            member_count: diagnostics.member_count,
            client_count: diagnostics.client_count,
            epoch: diagnostics.epoch.value(),
            generation: diagnostics.generation.value(),
            invalidations_published: stats.distributed_invalidations_published,
            invalidations_received: stats.distributed_invalidations_received,
            invalidations_applied: stats.distributed_invalidations_applied,
            lagged_receivers: stats.distributed_invalidation_lagged,
            decode_errors: stats.distributed_invalidation_decode_errors,
            publish_failures: stats.distributed_invalidation_publish_failures,
            receiver_closed: stats.distributed_invalidation_receiver_closed,
            owner_load_success: fill.owner_load_success,
            owner_load_errors: fill.owner_load_errors,
            remote_fetch_success: fill.remote_fetch_success,
            remote_fetch_errors: fill.remote_fetch_errors,
            hot_cache_hits: fill.hot_cache_hits,
            peer_fetch_auth_failures: staging.peer_fetch_auth_failures,
            wire_version_rejections: staging.wire_version_rejections,
            stale_generation_rejected: staging.stale_generation_rejected,
            tombstone_age_ms: staging.tombstone_age_ms,
            gossip_reset_count: staging.gossip_reset_count,
            barrier_timeouts: staging.barrier_timeouts,
            near_cache_conservative_invalidations: staging.near_cache_conservative_invalidations,
            lifecycle_stop_count: staging.lifecycle_stop_count,
            lifecycle_restart_count: staging.lifecycle_restart_count,
            state,
        }
    }
}

/// Structured cluster load report used by deterministic staging gates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClusterLoadReport {
    /// Number of cluster nodes participating in the scenario.
    pub nodes: usize,
    /// Total logical operations driven by the scenario.
    pub requests: usize,
    /// Logical read operations.
    pub read_ops: usize,
    /// Logical invalidation operations.
    pub invalidation_ops: usize,
    /// Invalidation messages published.
    pub published: u64,
    /// Invalidation messages received.
    pub received: u64,
    /// Invalidation messages applied.
    pub applied: u64,
    /// Receiver lag events.
    pub lagged: u64,
    /// Decode errors.
    pub decode_errors: u64,
    /// Publish failures.
    pub publish_failures: u64,
    /// Receiver closed events.
    pub receiver_closed: u64,
    /// Stale-generation attempts rejected by fencing.
    pub stale_generation_rejected: u64,
    /// Peer-fetch or owner-load auth failures observed by the gate probes.
    pub peer_fetch_auth_failures: u64,
    /// Peer-fetch or owner-load wire-version rejections observed by the gate probes.
    pub wire_version_rejections: u64,
    /// Owner-side origin load successes.
    pub owner_load_success: u64,
    /// Remote peer-fetch successes.
    pub remote_fetch_success: u64,
    /// Hot near-cache hits for non-owned values.
    pub hot_cache_hits: u64,
    /// Recorded wall-clock duration. Deterministic gates must not assert on it.
    pub elapsed_ms: u64,
}

impl ClusterLoadReport {
    /// Return whether read + invalidation operations match total requests.
    pub fn totals_match_requests(&self) -> bool {
        self.read_ops.saturating_add(self.invalidation_ops) == self.requests
    }

    /// Return whether the invalidation health counters are clean.
    pub fn has_clean_invalidation_health(&self) -> bool {
        self.lagged == 0
            && self.decode_errors == 0
            && self.publish_failures == 0
            && self.receiver_closed == 0
    }
}

fn derive_cluster_health_state(
    diagnostics: &ClusterDiagnostics,
    stats: &CacheStats,
    staging: &ClusterStagingCounters,
) -> ClusterHealthState {
    let mut hard_reasons = Vec::new();
    let mut soft_reasons = Vec::new();

    if !diagnostics.lifecycle.is_running() {
        hard_reasons.push(ClusterHealthReason::LifecycleNotRunning);
    }
    if diagnostics.participant_count() == 0 {
        hard_reasons.push(ClusterHealthReason::NoParticipants);
    }
    if stats.distributed_invalidation_decode_errors > 0 {
        hard_reasons.push(ClusterHealthReason::DecodeErrors {
            count: stats.distributed_invalidation_decode_errors,
        });
    }
    if stats.distributed_invalidation_publish_failures > 0 {
        hard_reasons.push(ClusterHealthReason::PublishFailures {
            count: stats.distributed_invalidation_publish_failures,
        });
    }
    if stats.distributed_invalidation_receiver_closed > 0 {
        hard_reasons.push(ClusterHealthReason::ReceiverClosed {
            count: stats.distributed_invalidation_receiver_closed,
        });
    }

    if stats.distributed_invalidation_lagged > 0 {
        soft_reasons.push(ClusterHealthReason::LaggedReceivers {
            count: stats.distributed_invalidation_lagged,
        });
    }
    if staging.peer_fetch_auth_failures > 0 {
        soft_reasons.push(ClusterHealthReason::PeerFetchAuthFailures {
            count: staging.peer_fetch_auth_failures,
        });
    }
    if staging.wire_version_rejections > 0 {
        soft_reasons.push(ClusterHealthReason::WireVersionRejections {
            count: staging.wire_version_rejections,
        });
    }
    if staging.gossip_reset_count > 0 {
        soft_reasons.push(ClusterHealthReason::GossipResetRecent {
            tombstone_age_ms: staging.tombstone_age_ms,
            reset_count: staging.gossip_reset_count,
        });
    }

    if hard_reasons.is_empty() {
        if soft_reasons.is_empty() {
            ClusterHealthState::Healthy
        } else {
            ClusterHealthState::Degraded {
                reasons: soft_reasons,
            }
        }
    } else {
        hard_reasons.extend(soft_reasons);
        ClusterHealthState::NotReady {
            reasons: hard_reasons,
        }
    }
}

/// Ownership diagnostics visible from a cluster control plane.
///
/// This is intentionally separate from [`ClusterDiagnostics`] so ownership
/// counters can evolve without adding fields to the externally constructible
/// runtime diagnostics snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ClusterOwnershipDiagnostics {
    /// Resolver name used by the control plane.
    pub resolver: &'static str,
    /// Number of ownership resolution attempts handled by this control plane.
    pub resolutions: u64,
    /// Number of ownership resolutions that found no admitted member owner.
    pub no_owner: u64,
    /// Monotonic ownership table stamp for stale-view detection.
    pub stamp: u64,
}

impl ClusterOwnershipDiagnostics {
    /// Create an ownership diagnostics snapshot.
    pub fn new(resolver: &'static str, resolutions: u64, no_owner: u64, stamp: u64) -> Self {
        Self {
            resolver,
            resolutions,
            no_owner,
            stamp,
        }
    }

    /// Number of ownership resolutions that selected an owner.
    pub fn owner_found(&self) -> u64 {
        self.resolutions.saturating_sub(self.no_owner)
    }

    /// Return whether any ownership resolution has been attempted.
    pub fn has_resolutions(&self) -> bool {
        self.resolutions > 0
    }

    /// Ratio of ownership resolutions that found an admitted owner.
    pub fn owner_found_ratio(&self) -> Option<f64> {
        (self.resolutions > 0).then(|| self.owner_found() as f64 / self.resolutions as f64)
    }
}

/// Discovery diagnostics visible from a [`HydraCache`] client/member runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterDiscoveryDiagnostics {
    /// Local node id that owns this diagnostics snapshot.
    pub local_node_id: ClusterNodeId,
    /// Latest candidate snapshots known to the discovery adapter.
    pub candidates: Vec<ClusterCandidate>,
    /// Discovery events known to the discovery adapter.
    pub events: Vec<ClusterDiscoveryEvent>,
}

impl ClusterDiscoveryDiagnostics {
    /// Number of latest candidate snapshots.
    pub fn candidate_count(&self) -> usize {
        self.candidates.len()
    }

    /// Number of discovery events.
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Return whether discovery has observed at least one candidate.
    pub fn has_candidates(&self) -> bool {
        !self.candidates.is_empty()
    }

    /// Return whether discovery has recorded at least one event.
    pub fn has_events(&self) -> bool {
        !self.events.is_empty()
    }
}
