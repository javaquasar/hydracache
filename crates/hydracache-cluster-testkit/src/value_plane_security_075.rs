//! Executable security guards for the provisional 0.75 internal value-plane contract.

use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecurityBounds {
    pub max_envelope_bytes: usize,
    pub max_key_bytes: usize,
    pub max_value_bytes: usize,
    pub max_proxy_hops: u8,
    pub max_replay_entries: usize,
    pub max_audit_records: usize,
    pub trust_overlap_ticks: u64,
}

impl Default for SecurityBounds {
    fn default() -> Self {
        Self {
            max_envelope_bytes: 8 * 1024 * 1024,
            max_key_bytes: 1024 * 1024,
            max_value_bytes: 4 * 1024 * 1024,
            max_proxy_hops: 1,
            max_replay_entries: 4_096,
            max_audit_records: 4_096,
            trust_overlap_ticks: 16,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecurityError {
    InvalidBound(&'static str),
    UnauthenticatedRoute,
    TenantSubstitution,
    StaleNamespaceGeneration { expected: u64, actual: u64 },
    StaleTopologyEpoch { expected: u64, actual: u64 },
    RedirectLoop { limit: u8, actual: u8 },
    DeadlineExpired,
    UnknownTrustEpoch(u64),
    EnvelopeTooLarge { limit: usize, actual: usize },
    KeyTooLarge { limit: usize, actual: usize },
    ValueTooLarge { limit: usize, actual: usize },
    ReplayCapacityExhausted,
    ReplayDigestConflict,
    ReplayTooOld,
    PermissionDenied,
    PolicyEpochMismatch,
    InvalidReplicaProof,
    AuditCapacityExhausted,
}

impl std::fmt::Display for SecurityError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for SecurityError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteClaims<'a> {
    pub authenticated: bool,
    pub authenticated_tenant: &'a str,
    pub claimed_tenant: &'a str,
    pub expected_namespace_generation: u64,
    pub actual_namespace_generation: u64,
    pub expected_topology_epoch: u64,
    pub actual_topology_epoch: u64,
    pub proxy_hops: u8,
    pub deadline_remaining_ticks: u64,
    pub trust_epoch: u64,
    pub encoded_envelope_bytes: usize,
    pub key_bytes: usize,
    pub value_bytes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrustEpochWindow {
    current: u64,
    previous: Option<(u64, u64)>,
    overlap_ticks: u64,
}

impl TrustEpochWindow {
    pub const fn current_epoch(&self) -> u64 {
        self.current
    }

    pub fn new(current: u64, overlap_ticks: u64) -> Result<Self, SecurityError> {
        if current == 0 {
            return Err(SecurityError::InvalidBound("trust_epoch"));
        }
        if overlap_ticks == 0 {
            return Err(SecurityError::InvalidBound("trust_overlap_ticks"));
        }
        Ok(Self {
            current,
            previous: None,
            overlap_ticks,
        })
    }

    pub fn rotate(&mut self, next: u64, now_tick: u64) -> Result<(), SecurityError> {
        if next <= self.current {
            return Err(SecurityError::UnknownTrustEpoch(next));
        }
        self.previous = Some((self.current, now_tick.saturating_add(self.overlap_ticks)));
        self.current = next;
        Ok(())
    }

    pub fn accepts(&self, epoch: u64, now_tick: u64) -> bool {
        epoch == self.current
            || self
                .previous
                .is_some_and(|(previous, expires)| epoch == previous && now_tick <= expires)
    }
}

pub fn validate_route(
    claims: &RouteClaims<'_>,
    bounds: SecurityBounds,
    trust: TrustEpochWindow,
    now_tick: u64,
) -> Result<(), SecurityError> {
    validate_bounds(bounds)?;
    if !claims.authenticated {
        return Err(SecurityError::UnauthenticatedRoute);
    }
    if claims.authenticated_tenant != claims.claimed_tenant {
        return Err(SecurityError::TenantSubstitution);
    }
    if claims.actual_namespace_generation != claims.expected_namespace_generation {
        return Err(SecurityError::StaleNamespaceGeneration {
            expected: claims.expected_namespace_generation,
            actual: claims.actual_namespace_generation,
        });
    }
    if claims.actual_topology_epoch != claims.expected_topology_epoch {
        return Err(SecurityError::StaleTopologyEpoch {
            expected: claims.expected_topology_epoch,
            actual: claims.actual_topology_epoch,
        });
    }
    if claims.proxy_hops > bounds.max_proxy_hops {
        return Err(SecurityError::RedirectLoop {
            limit: bounds.max_proxy_hops,
            actual: claims.proxy_hops,
        });
    }
    if claims.deadline_remaining_ticks == 0 {
        return Err(SecurityError::DeadlineExpired);
    }
    if !trust.accepts(claims.trust_epoch, now_tick) {
        return Err(SecurityError::UnknownTrustEpoch(claims.trust_epoch));
    }
    for (actual, limit, error) in [
        (
            claims.encoded_envelope_bytes,
            bounds.max_envelope_bytes,
            SecurityError::EnvelopeTooLarge {
                limit: bounds.max_envelope_bytes,
                actual: claims.encoded_envelope_bytes,
            },
        ),
        (
            claims.key_bytes,
            bounds.max_key_bytes,
            SecurityError::KeyTooLarge {
                limit: bounds.max_key_bytes,
                actual: claims.key_bytes,
            },
        ),
        (
            claims.value_bytes,
            bounds.max_value_bytes,
            SecurityError::ValueTooLarge {
                limit: bounds.max_value_bytes,
                actual: claims.value_bytes,
            },
        ),
    ] {
        if actual > limit {
            return Err(error);
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReplayIdentity {
    pub tenant: String,
    pub client: String,
    pub request: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayDisposition {
    New,
    ExactReplay,
}

#[derive(Debug, Clone)]
pub struct ReplayGuard {
    maximum: usize,
    digests: BTreeMap<ReplayIdentity, [u8; 32]>,
}

impl ReplayGuard {
    pub fn new(maximum: usize) -> Result<Self, SecurityError> {
        if maximum == 0 {
            return Err(SecurityError::InvalidBound("replay_entries"));
        }
        Ok(Self {
            maximum,
            digests: BTreeMap::new(),
        })
    }

    pub fn admit(
        &mut self,
        identity: ReplayIdentity,
        canonical_payload: &[u8],
    ) -> Result<ReplayDisposition, SecurityError> {
        let digest = fingerprint(canonical_payload);
        if let Some(retained) = self.digests.get(&identity) {
            return if retained == &digest {
                Ok(ReplayDisposition::ExactReplay)
            } else {
                Err(SecurityError::ReplayDigestConflict)
            };
        }
        if self.digests.len() >= self.maximum {
            return Err(SecurityError::ReplayCapacityExhausted);
        }
        self.digests.insert(identity, digest);
        Ok(ReplayDisposition::New)
    }
}

/// An active lane keeps its monotonic retirement fence even when it retains no digest.
/// Callers may remove a lane only after its member generation or trust epoch is rejected.
#[derive(Debug, Clone)]
pub struct ReplayWindow {
    maximum: usize,
    maximum_lanes: usize,
    lanes: BTreeMap<String, ReplayLane>,
    observations: ReplayWindowObservations,
}
#[derive(Debug, Clone, Default)]
struct ReplayLane {
    floor: u64,
    digests: BTreeMap<u64, [u8; 32]>,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReplayWindowObservations {
    pub retained: usize,
    pub lanes: usize,
    pub retired: u64,
    pub accepted: u64,
    pub replays: u64,
    pub stale: u64,
    pub conflicts: u64,
}
impl ReplayWindow {
    pub fn new(maximum: usize, maximum_lanes: usize) -> Result<Self, SecurityError> {
        if maximum == 0 || maximum > 65_536 || maximum_lanes == 0 || maximum_lanes > 128 {
            return Err(SecurityError::InvalidBound("replay_window"));
        }
        Ok(Self {
            maximum,
            maximum_lanes,
            lanes: BTreeMap::new(),
            observations: ReplayWindowObservations::default(),
        })
    }
    pub fn admit(
        &mut self,
        lane: &str,
        nonce: u64,
        payload: &[u8],
    ) -> Result<ReplayDisposition, SecurityError> {
        if lane.len() > 256 || lane.is_empty() {
            return Err(SecurityError::InvalidBound("replay_lane"));
        }
        let digest = fingerprint(payload);
        if let Some(current) = self.lanes.get(lane) {
            if nonce <= current.floor {
                self.observations.stale = self.observations.stale.saturating_add(1);
                return Err(SecurityError::ReplayTooOld);
            }
            if let Some(previous) = current.digests.get(&nonce) {
                if previous != &digest {
                    self.observations.conflicts = self.observations.conflicts.saturating_add(1);
                    return Err(SecurityError::ReplayDigestConflict);
                }
                self.observations.replays = self.observations.replays.saturating_add(1);
                return Ok(ReplayDisposition::ExactReplay);
            }
        } else if self.lanes.len() == self.maximum_lanes {
            return Err(SecurityError::ReplayCapacityExhausted);
        }
        if nonce == 0 {
            return Err(SecurityError::ReplayTooOld);
        }
        if self.observations.retained == self.maximum {
            let victim = self
                .lanes
                .iter()
                .filter_map(|(name, current)| {
                    current
                        .digests
                        .first_key_value()
                        .map(|(nonce, _)| (*nonce, name.clone()))
                })
                .min()
                .expect("full replay window");
            if victim.1 == lane && nonce <= victim.0 {
                self.observations.stale = self.observations.stale.saturating_add(1);
                return Err(SecurityError::ReplayTooOld);
            }
            let current = self.lanes.get_mut(&victim.1).expect("replay victim");
            current.digests.remove(&victim.0);
            current.floor = current.floor.max(victim.0);
            self.observations.retired = self.observations.retired.saturating_add(1);
            self.observations.retained -= 1;
        }
        self.lanes
            .entry(lane.into())
            .or_default()
            .digests
            .insert(nonce, digest);
        self.observations.retained += 1;
        self.observations.accepted = self.observations.accepted.saturating_add(1);
        self.observations.lanes = self.lanes.len();
        Ok(ReplayDisposition::New)
    }
    pub(crate) fn retain_lanes(&mut self, mut active: impl FnMut(&str) -> bool) {
        self.lanes.retain(|name, _| active(name));
        self.observations.retained = self.lanes.values().map(|lane| lane.digests.len()).sum();
        self.observations.lanes = self.lanes.len();
    }
    pub fn observations(&self) -> ReplayWindowObservations {
        self.observations
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantOperation {
    Read,
    Write,
    Listen,
    Admin,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GrantRights {
    pub read: bool,
    pub write: bool,
    pub listen: bool,
    pub admin: bool,
}
impl GrantRights {
    pub const ALL: Self = Self {
        read: true,
        write: true,
        listen: true,
        admin: true,
    };
    fn contains(self, rights: Self) -> bool {
        (!rights.read || self.read)
            && (!rights.write || self.write)
            && (!rights.listen || self.listen)
            && (!rights.admin || self.admin)
    }
    pub fn permits(self, operation: GrantOperation) -> bool {
        match operation {
            GrantOperation::Read => self.read,
            GrantOperation::Write => self.write,
            GrantOperation::Listen => self.listen,
            GrantOperation::Admin => self.admin,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrantKeys {
    All,
    Prefixes(Vec<Vec<u8>>),
}
#[derive(Clone, PartialEq, Eq)]
pub struct NamespaceGrant {
    tenant: String,
    namespace: String,
    generation: u64,
    epoch: u64,
    rights: GrantRights,
    keys: GrantKeys,
}
impl std::fmt::Debug for NamespaceGrant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NamespaceGrant")
            .field("epoch", &self.epoch)
            .field("rights", &self.rights)
            .field("scope", &"[redacted]")
            .finish()
    }
}
impl NamespaceGrant {
    pub fn new(
        tenant: &str,
        namespace: &str,
        generation: u64,
        epoch: u64,
        rights: GrantRights,
        keys: GrantKeys,
    ) -> Result<Self, SecurityError> {
        if tenant.is_empty()
            || namespace.is_empty()
            || tenant.len() > 128
            || namespace.len() > 128
            || generation == 0
            || epoch == 0
        {
            return Err(SecurityError::InvalidBound("grant_identity"));
        }
        if let GrantKeys::Prefixes(prefixes) = &keys {
            if prefixes.is_empty()
                || prefixes.len() > 16
                || prefixes
                    .iter()
                    .any(|prefix| prefix.is_empty() || prefix.len() > 4_096)
                || prefixes.iter().map(Vec::len).sum::<usize>() > 16_384
            {
                return Err(SecurityError::InvalidBound("grant_keys"));
            }
        }
        Ok(Self {
            tenant: tenant.into(),
            namespace: namespace.into(),
            generation,
            epoch,
            rights,
            keys,
        })
    }
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
    pub fn owned_bytes(&self) -> usize {
        64 + self.tenant.len()
            + self.namespace.len()
            + match &self.keys {
                GrantKeys::All => 0,
                GrantKeys::Prefixes(prefixes) => {
                    prefixes.iter().map(|prefix| 24 + prefix.len()).sum()
                }
            }
    }
    pub fn matches(&self, tenant: &str, namespace: &str, generation: u64) -> bool {
        self.tenant == tenant && self.namespace == namespace && self.generation == generation
    }
    pub fn key_allowed(&self, key: &[u8]) -> bool {
        match &self.keys {
            GrantKeys::All => true,
            GrantKeys::Prefixes(prefixes) => prefixes.iter().any(|prefix| key.starts_with(prefix)),
        }
    }
    pub fn all_keys(&self) -> bool {
        matches!(self.keys, GrantKeys::All)
    }
    pub fn authorize(
        &self,
        operation: GrantOperation,
        keys: &[Vec<u8>],
        map_wide: bool,
    ) -> Result<(), SecurityError> {
        if !self.rights.permits(operation)
            || (map_wide && !self.all_keys())
            || keys.iter().any(|key| !self.key_allowed(key))
        {
            return Err(SecurityError::PermissionDenied);
        }
        Ok(())
    }
    pub fn attenuate(&self, rights: GrantRights, keys: GrantKeys) -> Result<Self, SecurityError> {
        if !self.rights.contains(rights)
            || match &keys {
                GrantKeys::All => !self.all_keys(),
                GrantKeys::Prefixes(prefixes) => {
                    prefixes.iter().any(|prefix| !self.key_allowed(prefix))
                }
            }
        {
            return Err(SecurityError::PermissionDenied);
        }
        Self::new(
            &self.tenant,
            &self.namespace,
            self.generation,
            self.epoch,
            rights,
            keys,
        )
    }
    pub fn digest(&self) -> [u8; 32] {
        let mut hash = Sha256::new();
        for value in [&self.tenant, &self.namespace] {
            hash.update(value.len().to_le_bytes());
            hash.update(value.as_bytes());
        }
        hash.update(self.generation.to_le_bytes());
        hash.update(self.epoch.to_le_bytes());
        hash.update([
            self.rights.read as u8,
            self.rights.write as u8,
            self.rights.listen as u8,
            self.rights.admin as u8,
        ]);
        match &self.keys {
            GrantKeys::All => hash.update([0]),
            GrantKeys::Prefixes(prefixes) => {
                hash.update([1]);
                for prefix in prefixes {
                    hash.update(prefix.len().to_le_bytes());
                    hash.update(prefix);
                }
            }
        }
        hash.finalize().into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplicaProofClaims {
    pub authenticated: bool,
    pub expected_partition: u32,
    pub partition: u32,
    pub expected_epoch: u64,
    pub epoch: u64,
    pub expected_version: u64,
    pub version: u64,
    pub expected_checksum: [u8; 32],
    pub checksum: [u8; 32],
}

pub fn validate_replica_proof(claims: ReplicaProofClaims) -> Result<(), SecurityError> {
    if claims.authenticated
        && claims.expected_partition == claims.partition
        && claims.expected_epoch == claims.epoch
        && claims.expected_version == claims.version
        && claims.expected_checksum == claims.checksum
    {
        Ok(())
    } else {
        Err(SecurityError::InvalidReplicaProof)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditRecord {
    pub tenant_fingerprint: [u8; 32],
    pub namespace_fingerprint: [u8; 32],
    pub key_fingerprint: [u8; 32],
    pub operation: &'static str,
    pub result: &'static str,
}

#[derive(Debug, Clone)]
pub struct RedactedAuditLog {
    maximum: usize,
    records: VecDeque<AuditRecord>,
}

impl RedactedAuditLog {
    pub fn new(maximum: usize) -> Result<Self, SecurityError> {
        if maximum == 0 {
            return Err(SecurityError::InvalidBound("audit_records"));
        }
        Ok(Self {
            maximum,
            records: VecDeque::new(),
        })
    }

    pub fn record(
        &mut self,
        tenant: &[u8],
        namespace: &[u8],
        key: &[u8],
        operation: &'static str,
        result: &'static str,
    ) -> Result<(), SecurityError> {
        if self.records.len() >= self.maximum {
            return Err(SecurityError::AuditCapacityExhausted);
        }
        self.records.push_back(AuditRecord {
            tenant_fingerprint: fingerprint(tenant),
            namespace_fingerprint: fingerprint(namespace),
            key_fingerprint: fingerprint(key),
            operation,
            result,
        });
        Ok(())
    }

    pub fn records(&self) -> impl Iterator<Item = &AuditRecord> {
        self.records.iter()
    }
}

pub fn fingerprint(value: &[u8]) -> [u8; 32] {
    Sha256::digest(value).into()
}

fn validate_bounds(bounds: SecurityBounds) -> Result<(), SecurityError> {
    for (name, value) in [
        ("envelope_bytes", bounds.max_envelope_bytes),
        ("key_bytes", bounds.max_key_bytes),
        ("value_bytes", bounds.max_value_bytes),
        ("proxy_hops", usize::from(bounds.max_proxy_hops)),
        ("replay_entries", bounds.max_replay_entries),
        ("audit_records", bounds.max_audit_records),
        ("trust_overlap_ticks", bounds.trust_overlap_ticks as usize),
    ] {
        if value == 0 {
            return Err(SecurityError::InvalidBound(name));
        }
    }
    Ok(())
}
