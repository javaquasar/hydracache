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
