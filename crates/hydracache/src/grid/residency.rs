use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::cluster::{ClusterEpoch, ClusterMember};
use crate::grid::elasticity::{RegionId, ZoneAwareReplicaSet, ZoneAwareReplicationStrategy};

/// Residency policy format registered in `docs/COMPAT.md`.
pub const RESIDENCY_POLICY_FORMAT_VERSION: u32 = 1;

/// A namespace/key residency policy committed through the authoritative control plane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResidencyPolicy {
    /// Durable policy format version.
    pub format_version: u32,
    /// Control-plane epoch that committed this policy.
    pub epoch: ClusterEpoch,
    allowed_regions: BTreeSet<RegionId>,
    min_replicas_in_policy: usize,
}

impl ResidencyPolicy {
    /// Build a policy that allows value bytes only in the supplied regions.
    pub fn new<R>(
        allowed_regions: impl IntoIterator<Item = R>,
        min_replicas_in_policy: usize,
        epoch: ClusterEpoch,
    ) -> Result<Self, ResidencyPolicyError>
    where
        R: Into<RegionId>,
    {
        let allowed_regions = allowed_regions.into_iter().map(Into::into).collect();
        let policy = Self {
            format_version: RESIDENCY_POLICY_FORMAT_VERSION,
            epoch,
            allowed_regions,
            min_replicas_in_policy: min_replicas_in_policy.max(1),
        };
        policy.validate()?;
        Ok(policy)
    }

    /// Override the serialized format version for compatibility tests.
    pub fn with_format_version(mut self, format_version: u32) -> Self {
        self.format_version = format_version;
        self
    }

    /// Return the allowed regions.
    pub fn allowed_regions(&self) -> &BTreeSet<RegionId> {
        &self.allowed_regions
    }

    /// Return the minimum required replicas inside policy.
    pub fn min_replicas_in_policy(&self) -> usize {
        self.min_replicas_in_policy
    }

    /// Return whether this policy allows value bytes in a region.
    pub fn allows_region(&self, region: &RegionId) -> bool {
        self.allowed_regions.contains(region)
    }

    fn validate(&self) -> Result<(), ResidencyPolicyError> {
        if self.format_version > RESIDENCY_POLICY_FORMAT_VERSION {
            return Err(ResidencyPolicyError::new(format!(
                "residency policy format {} is newer than supported {}",
                self.format_version, RESIDENCY_POLICY_FORMAT_VERSION
            )));
        }
        if self.allowed_regions.is_empty() {
            return Err(ResidencyPolicyError::new(
                "residency policy must contain at least one allowed region",
            ));
        }
        Ok(())
    }
}

/// Error returned while committing or validating residency policies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResidencyPolicyError {
    message: String,
}

impl ResidencyPolicyError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ResidencyPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ResidencyPolicyError {}

/// Authoritative residency policy set.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResidencyPolicySet {
    epoch: ClusterEpoch,
    namespace_policies: BTreeMap<String, ResidencyPolicy>,
    key_overrides: BTreeMap<String, BTreeMap<String, ResidencyPolicy>>,
}

impl ResidencyPolicySet {
    /// Create an empty policy set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the highest committed policy epoch.
    pub fn epoch(&self) -> ClusterEpoch {
        self.epoch
    }

    /// Commit a namespace policy at its control-plane epoch.
    pub fn commit_namespace_policy(
        &mut self,
        namespace: impl Into<String>,
        policy: ResidencyPolicy,
    ) -> Result<(), ResidencyPolicyError> {
        let namespace = normalize_non_empty("namespace", namespace.into())?;
        self.commit_policy_epoch(&policy)?;
        self.epoch = self.epoch.max(policy.epoch);
        self.namespace_policies.insert(namespace, policy);
        Ok(())
    }

    /// Commit a per-key policy override.
    pub fn commit_key_override(
        &mut self,
        namespace: impl Into<String>,
        key: impl Into<String>,
        policy: ResidencyPolicy,
    ) -> Result<(), ResidencyPolicyError> {
        let namespace = normalize_non_empty("namespace", namespace.into())?;
        let key = normalize_non_empty("key", key.into())?;
        self.commit_policy_epoch(&policy)?;
        self.epoch = self.epoch.max(policy.epoch);
        self.key_overrides
            .entry(namespace)
            .or_default()
            .insert(key, policy);
        Ok(())
    }

    /// Return the effective policy for a namespace/key pair.
    pub fn policy_for(&self, namespace: &str, key: &str) -> Option<&ResidencyPolicy> {
        self.key_overrides
            .get(namespace)
            .and_then(|keys| keys.get(key))
            .or_else(|| self.namespace_policies.get(namespace))
    }

    /// Return the effective policy epoch for a namespace/key pair.
    pub fn effective_epoch(&self, namespace: &str, key: &str) -> ClusterEpoch {
        self.policy_for(namespace, key)
            .map(|policy| policy.epoch)
            .unwrap_or(self.epoch)
    }

    fn commit_policy_epoch(&self, policy: &ResidencyPolicy) -> Result<(), ResidencyPolicyError> {
        policy.validate()?;
        if policy.epoch < self.epoch {
            return Err(ResidencyPolicyError::new(format!(
                "residency policy epoch {} is older than committed epoch {}",
                policy.epoch.value(),
                self.epoch.value()
            )));
        }
        Ok(())
    }
}

fn normalize_non_empty(field: &'static str, value: String) -> Result<String, ResidencyPolicyError> {
    let value = value.trim().to_owned();
    if value.is_empty() {
        return Err(ResidencyPolicyError::new(format!(
            "residency policy {field} must not be empty"
        )));
    }
    Ok(value)
}

/// Residency rejection kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResidencyRejectionKind {
    /// Placement could not satisfy RF inside the allowed region set.
    RejectPlacement,
    /// WAN value movement crossed a forbidden boundary.
    RefuseCrossBoundary,
    /// A read tried to serve value bytes from a forbidden region.
    RejectRead,
    /// A caller enforced an older policy epoch than the current one.
    StalePolicyEpoch,
}

/// Loud residency rejection surfaced to callers and audit sinks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResidencyRejection {
    /// Rejection kind.
    pub kind: ResidencyRejectionKind,
    /// Namespace being governed.
    pub namespace: String,
    /// Key being governed.
    pub key: String,
    /// Epoch that was enforced.
    pub policy_epoch: ClusterEpoch,
    /// Local/source region, when relevant.
    pub source_region: Option<RegionId>,
    /// Destination/serving region, when relevant.
    pub target_region: Option<RegionId>,
    /// Human-readable failure reason.
    pub reason: String,
}

impl ResidencyRejection {
    fn reject_placement(
        namespace: &str,
        key: &str,
        policy_epoch: ClusterEpoch,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            kind: ResidencyRejectionKind::RejectPlacement,
            namespace: namespace.to_owned(),
            key: key.to_owned(),
            policy_epoch,
            source_region: None,
            target_region: None,
            reason: reason.into(),
        }
    }

    fn refuse_cross_boundary(
        namespace: &str,
        key: &str,
        policy_epoch: ClusterEpoch,
        source_region: &RegionId,
        target_region: &RegionId,
    ) -> Self {
        Self {
            kind: ResidencyRejectionKind::RefuseCrossBoundary,
            namespace: namespace.to_owned(),
            key: key.to_owned(),
            policy_epoch,
            source_region: Some(source_region.clone()),
            target_region: Some(target_region.clone()),
            reason: format!(
                "value movement from {} to {} is forbidden by residency policy",
                source_region.as_str(),
                target_region.as_str()
            ),
        }
    }

    fn reject_read(
        namespace: &str,
        key: &str,
        policy_epoch: ClusterEpoch,
        serving_region: &RegionId,
    ) -> Self {
        Self {
            kind: ResidencyRejectionKind::RejectRead,
            namespace: namespace.to_owned(),
            key: key.to_owned(),
            policy_epoch,
            source_region: None,
            target_region: Some(serving_region.clone()),
            reason: format!(
                "serving value bytes from {} is forbidden by residency policy",
                serving_region.as_str()
            ),
        }
    }

    fn stale_epoch(
        namespace: &str,
        key: &str,
        policy_epoch: ClusterEpoch,
        observed_epoch: ClusterEpoch,
    ) -> Self {
        Self {
            kind: ResidencyRejectionKind::StalePolicyEpoch,
            namespace: namespace.to_owned(),
            key: key.to_owned(),
            policy_epoch,
            source_region: None,
            target_region: None,
            reason: format!(
                "observed residency policy epoch {} is older than current epoch {}",
                observed_epoch.value(),
                policy_epoch.value()
            ),
        }
    }
}

impl fmt::Display for ResidencyRejection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.reason)
    }
}

impl std::error::Error for ResidencyRejection {}

/// Residency decision used by diagnostics and policy dry-runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResidencyDecision {
    /// Operation is allowed at this policy epoch.
    Allow {
        /// Epoch that allowed the operation.
        policy_epoch: ClusterEpoch,
    },
    /// Operation is rejected loud.
    Reject(ResidencyRejection),
}

/// Bounded residency metrics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResidencyMetricsSnapshot {
    /// Placement rejections because policy RF could not be satisfied.
    pub residency_rejected_placement_total: u64,
    /// WAN value movements refused by policy.
    pub residency_refused_crossing_total: u64,
    /// Reads refused because the serving region is forbidden or stale.
    pub residency_rejected_read_total: u64,
    /// Existing value locations found out of policy after narrowing.
    pub residency_policy_narrowing_out_of_policy_total: u64,
}

/// Residency audit action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResidencyAuditAction {
    /// Placement was rejected.
    RejectPlacement,
    /// Cross-region value movement was refused.
    RefuseCrossBoundary,
    /// Read was rejected.
    RejectRead,
    /// Existing location was remediated after policy narrowing.
    PolicyNarrowingRemediation,
}

/// Append-only residency audit event retained until W6 installs a pluggable sink.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResidencyAuditEvent {
    /// Audit action.
    pub action: ResidencyAuditAction,
    /// Namespace.
    pub namespace: String,
    /// Key.
    pub key: String,
    /// Policy epoch.
    pub policy_epoch: ClusterEpoch,
    /// Source region, when relevant.
    pub source_region: Option<RegionId>,
    /// Target/serving region, when relevant.
    pub target_region: Option<RegionId>,
    /// Reason retained for snapshots, not metric labels.
    pub reason: String,
}

impl ResidencyAuditEvent {
    fn from_rejection(rejection: &ResidencyRejection) -> Self {
        let action = match rejection.kind {
            ResidencyRejectionKind::RejectPlacement => ResidencyAuditAction::RejectPlacement,
            ResidencyRejectionKind::RefuseCrossBoundary => {
                ResidencyAuditAction::RefuseCrossBoundary
            }
            ResidencyRejectionKind::RejectRead | ResidencyRejectionKind::StalePolicyEpoch => {
                ResidencyAuditAction::RejectRead
            }
        };
        Self {
            action,
            namespace: rejection.namespace.clone(),
            key: rejection.key.clone(),
            policy_epoch: rejection.policy_epoch,
            source_region: rejection.source_region.clone(),
            target_region: rejection.target_region.clone(),
            reason: rejection.reason.clone(),
        }
    }
}

/// Runtime policy enforcer for placement, WAN movement, reads, and policy changes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResidencyPolicyEnforcer {
    policies: ResidencyPolicySet,
    metrics: ResidencyMetricsSnapshot,
    audit_events: Vec<ResidencyAuditEvent>,
}

impl ResidencyPolicyEnforcer {
    /// Create an enforcer from an authoritative policy set.
    pub fn new(policies: ResidencyPolicySet) -> Self {
        Self {
            policies,
            metrics: ResidencyMetricsSnapshot::default(),
            audit_events: Vec::new(),
        }
    }

    /// Return the policy set.
    pub fn policies(&self) -> &ResidencyPolicySet {
        &self.policies
    }

    /// Return bounded residency metrics.
    pub fn metrics(&self) -> ResidencyMetricsSnapshot {
        self.metrics
    }

    /// Return retained residency audit events.
    pub fn audit_events(&self) -> &[ResidencyAuditEvent] {
        &self.audit_events
    }

    /// Place a key using only nodes legal for its effective policy.
    pub fn place_key(
        &mut self,
        strategy: &ZoneAwareReplicationStrategy,
        namespace: &str,
        key: &str,
        members: &[ClusterMember],
    ) -> Result<ZoneAwareReplicaSet, Box<ResidencyRejection>> {
        if let Some(policy) = self.policies.policy_for(namespace, key) {
            let required = strategy
                .replication_factor()
                .max(policy.min_replicas_in_policy());
            if let Some(replicas) = strategy.zone_replicas_for_key_in_regions(
                key,
                members,
                policy.allowed_regions(),
                required,
            ) {
                return Ok(replicas);
            }
            let rejection = ResidencyRejection::reject_placement(
                namespace,
                key,
                policy.epoch,
                format!("replication factor {required} cannot be satisfied inside allowed regions"),
            );
            self.record_rejection(rejection.clone());
            return Err(Box::new(rejection));
        }

        strategy.zone_replicas_for_key(key, members).ok_or_else(|| {
            let rejection = ResidencyRejection::reject_placement(
                namespace,
                key,
                self.policies.effective_epoch(namespace, key),
                "no members available for placement",
            );
            self.record_rejection(rejection.clone());
            Box::new(rejection)
        })
    }

    /// Refuse WAN value movement when the destination is outside policy.
    pub fn guard_cross_boundary(
        &mut self,
        namespace: &str,
        key: &str,
        source_region: &RegionId,
        target_region: &RegionId,
    ) -> Result<ResidencyDecision, Box<ResidencyRejection>> {
        let Some(policy) = self.policies.policy_for(namespace, key) else {
            return Ok(ResidencyDecision::Allow {
                policy_epoch: self.policies.effective_epoch(namespace, key),
            });
        };
        if policy.allows_region(target_region) {
            return Ok(ResidencyDecision::Allow {
                policy_epoch: policy.epoch,
            });
        }
        let rejection = ResidencyRejection::refuse_cross_boundary(
            namespace,
            key,
            policy.epoch,
            source_region,
            target_region,
        );
        self.record_rejection(rejection.clone());
        Err(Box::new(rejection))
    }

    /// Refuse reads from forbidden regions or stale policy epochs.
    pub fn guard_read(
        &mut self,
        namespace: &str,
        key: &str,
        serving_region: &RegionId,
        observed_epoch: ClusterEpoch,
    ) -> Result<ResidencyDecision, Box<ResidencyRejection>> {
        let Some(policy) = self.policies.policy_for(namespace, key) else {
            return Ok(ResidencyDecision::Allow {
                policy_epoch: self.policies.effective_epoch(namespace, key),
            });
        };
        if observed_epoch < policy.epoch {
            let rejection =
                ResidencyRejection::stale_epoch(namespace, key, policy.epoch, observed_epoch);
            self.record_rejection(rejection.clone());
            return Err(Box::new(rejection));
        }
        if policy.allows_region(serving_region) {
            return Ok(ResidencyDecision::Allow {
                policy_epoch: policy.epoch,
            });
        }
        let rejection =
            ResidencyRejection::reject_read(namespace, key, policy.epoch, serving_region);
        self.record_rejection(rejection.clone());
        Err(Box::new(rejection))
    }

    /// Return whether an include-value invalidation may carry bytes to a region.
    pub fn include_value_allowed(&self, namespace: &str, key: &str, region: &RegionId) -> bool {
        self.policies
            .policy_for(namespace, key)
            .map(|policy| policy.allows_region(region))
            .unwrap_or(true)
    }

    /// Detect out-of-policy value locations after a policy narrowing.
    pub fn plan_policy_narrowing(
        &mut self,
        locations: impl IntoIterator<Item = ResidencyValueLocation>,
    ) -> ResidencyNarrowingReport {
        let mut actions = Vec::new();
        for location in locations {
            let policy = self
                .policies
                .policy_for(&location.namespace, &location.key)
                .cloned();
            match policy {
                Some(policy) if !policy.allows_region(&location.region) => {
                    self.metrics.residency_policy_narrowing_out_of_policy_total = self
                        .metrics
                        .residency_policy_narrowing_out_of_policy_total
                        .saturating_add(1);
                    let action = ResidencyRemediationAction::Evict {
                        namespace: location.namespace.clone(),
                        key: location.key.clone(),
                        region: location.region.clone(),
                        policy_epoch: policy.epoch,
                    };
                    self.audit_events.push(ResidencyAuditEvent {
                        action: ResidencyAuditAction::PolicyNarrowingRemediation,
                        namespace: location.namespace,
                        key: location.key,
                        policy_epoch: policy.epoch,
                        source_region: Some(location.region),
                        target_region: None,
                        reason: "existing value location is outside narrowed residency policy"
                            .to_owned(),
                    });
                    actions.push(action);
                }
                Some(policy) => actions.push(ResidencyRemediationAction::Keep {
                    namespace: location.namespace,
                    key: location.key,
                    region: location.region,
                    policy_epoch: policy.epoch,
                }),
                None => actions.push(ResidencyRemediationAction::Keep {
                    namespace: location.namespace,
                    key: location.key,
                    region: location.region,
                    policy_epoch: self.policies.epoch(),
                }),
            }
        }
        ResidencyNarrowingReport { actions }
    }

    /// Choose an allowed failover home or report degraded without violating policy.
    pub fn choose_failover_home(
        &self,
        namespace: &str,
        key: &str,
        surviving_regions: impl IntoIterator<Item = RegionId>,
    ) -> ResidencyFailoverDecision {
        let policy = self.policies.policy_for(namespace, key);
        for region in surviving_regions {
            if policy
                .map(|policy| policy.allows_region(&region))
                .unwrap_or(true)
            {
                return ResidencyFailoverDecision::Promote {
                    target_region: region,
                    policy_epoch: policy
                        .map(|policy| policy.epoch)
                        .unwrap_or_else(|| self.policies.effective_epoch(namespace, key)),
                };
            }
        }
        ResidencyFailoverDecision::Degraded {
            policy_epoch: self.policies.effective_epoch(namespace, key),
            reason: "no surviving region is allowed by residency policy".to_owned(),
        }
    }

    fn record_rejection(&mut self, rejection: ResidencyRejection) {
        match rejection.kind {
            ResidencyRejectionKind::RejectPlacement => {
                self.metrics.residency_rejected_placement_total = self
                    .metrics
                    .residency_rejected_placement_total
                    .saturating_add(1);
            }
            ResidencyRejectionKind::RefuseCrossBoundary => {
                self.metrics.residency_refused_crossing_total = self
                    .metrics
                    .residency_refused_crossing_total
                    .saturating_add(1);
            }
            ResidencyRejectionKind::RejectRead | ResidencyRejectionKind::StalePolicyEpoch => {
                self.metrics.residency_rejected_read_total =
                    self.metrics.residency_rejected_read_total.saturating_add(1);
            }
        }
        self.audit_events
            .push(ResidencyAuditEvent::from_rejection(&rejection));
    }
}

/// One value location used when a narrowed policy is evaluated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResidencyValueLocation {
    /// Namespace.
    pub namespace: String,
    /// Key.
    pub key: String,
    /// Region that currently stores value bytes.
    pub region: RegionId,
}

impl ResidencyValueLocation {
    /// Build a value location.
    pub fn new(
        namespace: impl Into<String>,
        key: impl Into<String>,
        region: impl Into<RegionId>,
    ) -> Self {
        Self {
            namespace: namespace.into(),
            key: key.into(),
            region: region.into(),
        }
    }
}

/// Remediation action for a value location after policy narrowing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResidencyRemediationAction {
    /// Existing location is still legal.
    Keep {
        /// Namespace.
        namespace: String,
        /// Key.
        key: String,
        /// Region.
        region: RegionId,
        /// Policy epoch.
        policy_epoch: ClusterEpoch,
    },
    /// Existing location must be evicted because it is outside policy.
    Evict {
        /// Namespace.
        namespace: String,
        /// Key.
        key: String,
        /// Region.
        region: RegionId,
        /// Policy epoch.
        policy_epoch: ClusterEpoch,
    },
    /// Existing location cannot be fixed automatically and is marked degraded.
    MarkDegraded {
        /// Namespace.
        namespace: String,
        /// Key.
        key: String,
        /// Region.
        region: RegionId,
        /// Policy epoch.
        policy_epoch: ClusterEpoch,
        /// Reason.
        reason: String,
    },
}

/// Report for policy-narrowing remediation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResidencyNarrowingReport {
    /// Actions to execute or audit.
    pub actions: Vec<ResidencyRemediationAction>,
}

impl ResidencyNarrowingReport {
    /// Return whether every out-of-policy location has an explicit action.
    pub fn has_remediation(&self) -> bool {
        self.actions.iter().any(|action| {
            matches!(
                action,
                ResidencyRemediationAction::Evict { .. }
                    | ResidencyRemediationAction::MarkDegraded { .. }
            )
        })
    }
}

/// WAN send report after applying residency governance.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResidencyLinkSendReport {
    /// Entries checked against residency policy.
    pub checked: u64,
    /// Entries refused before sending.
    pub refused: u64,
    /// Whether the batch was admitted to the WAN link.
    pub sent: bool,
}

/// Failover decision that never chooses an out-of-policy home.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResidencyFailoverDecision {
    /// Promote the key/namespace to an allowed target region.
    Promote {
        /// Target region.
        target_region: RegionId,
        /// Policy epoch.
        policy_epoch: ClusterEpoch,
    },
    /// Report degraded because no surviving region is legal.
    Degraded {
        /// Policy epoch.
        policy_epoch: ClusterEpoch,
        /// Reason.
        reason: String,
    },
}
