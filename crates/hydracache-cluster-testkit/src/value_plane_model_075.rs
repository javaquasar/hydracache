//! Bounded, domain-only reference model for the provisional 0.75 value plane.
//!
//! This module deliberately owns no wire numbers, durable format identities, wall-clock
//! reads, or production routing.  It is a small executable specification for mutation
//! ordering, epoch fencing, replica proof, deduplication, and deterministic fault replay.

use std::collections::BTreeMap;
use std::fmt;

/// Explicit limits for every retained collection and request shape in the model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValuePlaneBounds {
    pub max_key_bytes: usize,
    pub max_value_bytes: usize,
    pub max_bulk_items: usize,
    pub max_history_events: usize,
    pub max_dedup_entries: usize,
    pub max_fault_steps: usize,
    pub max_linearizability_operations: usize,
    pub max_linearizability_search_states: usize,
}

impl Default for ValuePlaneBounds {
    fn default() -> Self {
        Self {
            max_key_bytes: 4 * 1024,
            max_value_bytes: 16 * 1024 * 1024,
            max_bulk_items: 256,
            max_history_events: 4_096,
            max_dedup_entries: 4_096,
            max_fault_steps: 64,
            max_linearizability_operations: 64,
            max_linearizability_search_states: 100_000,
        }
    }
}

impl ValuePlaneBounds {
    fn validate(self) -> Result<Self, ValuePlaneError> {
        for (name, value) in [
            ("key_bytes", self.max_key_bytes),
            ("value_bytes", self.max_value_bytes),
            ("bulk_items", self.max_bulk_items),
            ("history_events", self.max_history_events),
            ("dedup_entries", self.max_dedup_entries),
            ("fault_steps", self.max_fault_steps),
            (
                "linearizability_operations",
                self.max_linearizability_operations,
            ),
            (
                "linearizability_search_states",
                self.max_linearizability_search_states,
            ),
        ] {
            if value == 0 {
                return Err(ValuePlaneError::InvalidBound(name));
            }
        }
        Ok(self)
    }
}

/// Canonical domain identity. The hash/partition algorithm is intentionally not specified here.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CanonicalMapKey {
    pub tenant: String,
    pub namespace: String,
    pub namespace_generation: u64,
    pub key_bytes: Vec<u8>,
    pub partition: u32,
}

impl CanonicalMapKey {
    pub fn new(
        tenant: impl Into<String>,
        namespace: impl Into<String>,
        namespace_generation: u64,
        key_bytes: Vec<u8>,
        partition: u32,
    ) -> Self {
        Self {
            tenant: tenant.into(),
            namespace: namespace.into(),
            namespace_generation,
            key_bytes,
            partition,
        }
    }
}

/// Logical test time. It has no relationship to system or monotonic wall clock time.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct LogicalTime(u64);

impl LogicalTime {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u64 {
        self.0
    }

    fn saturating_add(self, delta: u64) -> Self {
        Self(self.0.saturating_add(delta))
    }
}

/// Stable caller-owned identity for retry/dedup semantics.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MutationIdentity {
    pub client: String,
    pub sequence: u64,
}

impl MutationIdentity {
    pub fn new(client: impl Into<String>, sequence: u64) -> Self {
        Self {
            client: client.into(),
            sequence,
        }
    }
}

/// Opaque digest supplied by the test/reference encoder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MutationDigest(u64);

impl MutationDigest {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

/// Unambiguous TTL intent used by server-atomic mutations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TtlDirective {
    Preserve,
    Eternal,
    ExpireAfter(u64),
}

/// Canonical single-key operation set needed by the foundation model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MutationOperation {
    Put {
        value: Vec<u8>,
        ttl: TtlDirective,
    },
    PutIfAbsent {
        value: Vec<u8>,
        ttl: TtlDirective,
    },
    ReplaceIfPresent {
        value: Vec<u8>,
        ttl: TtlDirective,
    },
    ReplaceIfValue {
        expected: Vec<u8>,
        value: Vec<u8>,
        ttl: TtlDirective,
    },
    GetAndPut {
        value: Vec<u8>,
        ttl: TtlDirective,
    },
    GetAndRemove,
    Delete,
}

impl MutationOperation {
    fn encoded_value_bytes(&self) -> usize {
        match self {
            Self::Put { value, .. }
            | Self::PutIfAbsent { value, .. }
            | Self::ReplaceIfPresent { value, .. }
            | Self::GetAndPut { value, .. } => value.len(),
            Self::ReplaceIfValue {
                expected, value, ..
            } => expected.len().saturating_add(value.len()),
            Self::GetAndRemove | Self::Delete => 0,
        }
    }
}

/// Immutable request admitted into the reference model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutationPlan {
    pub identity: MutationIdentity,
    pub digest: MutationDigest,
    pub key: CanonicalMapKey,
    pub operation: MutationOperation,
    pub expected_epoch: u64,
}

impl MutationPlan {
    pub fn new(
        identity: MutationIdentity,
        digest: MutationDigest,
        key: CanonicalMapKey,
        operation: MutationOperation,
        expected_epoch: u64,
    ) -> Self {
        Self {
            identity,
            digest,
            key,
            operation,
            expected_epoch,
        }
    }
}

/// Ordered lifecycle stages. `OutcomeUnknown` is a terminal response-loss branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MutationStage {
    Received,
    Admitted,
    Routed,
    OwnerDecided,
    OwnerApplied,
    VisibleOwner,
    ReplicaProved,
    Acknowledged,
    Responded,
    OutcomeUnknown,
}

impl MutationStage {
    fn next(self) -> Option<Self> {
        match self {
            Self::Received => Some(Self::Admitted),
            Self::Admitted => Some(Self::Routed),
            Self::Routed => Some(Self::OwnerDecided),
            Self::OwnerDecided => Some(Self::OwnerApplied),
            Self::OwnerApplied => Some(Self::VisibleOwner),
            Self::VisibleOwner => Some(Self::ReplicaProved),
            Self::ReplicaProved => Some(Self::Acknowledged),
            Self::Acknowledged => Some(Self::Responded),
            Self::Responded | Self::OutcomeUnknown => None,
        }
    }
}

/// What a caller can safely conclude from an observed completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutcomeCertainty {
    Certain,
    OutcomeUnknown,
}

/// Canonical mutation result retained for exact replay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutationOutcome {
    pub applied: bool,
    pub previous: Option<Vec<u8>>,
    pub current: Option<Vec<u8>>,
    pub version: Option<u64>,
    pub certainty: OutcomeCertainty,
}

/// Retained dedup and progress entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutationRecord {
    pub stage: MutationStage,
    pub outcome: Option<MutationOutcome>,
    pub replica_proof: Option<String>,
    plan: MutationPlan,
}

/// One bounded transition-history event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutationHistoryEvent {
    pub identity: MutationIdentity,
    pub stage: MutationStage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum EntryState {
    Live {
        value: Vec<u8>,
        expires_at: Option<LogicalTime>,
        version: u64,
    },
    Tombstone {
        version: u64,
    },
}

impl EntryState {
    fn version(&self) -> u64 {
        match self {
            Self::Live { version, .. } | Self::Tombstone { version } => *version,
        }
    }
}

/// Fail-closed model errors, intentionally free of wire/durable numeric identities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValuePlaneError {
    InvalidBound(&'static str),
    BoundExceeded {
        bound: &'static str,
        limit: usize,
        actual: usize,
    },
    UnknownMutation,
    MutationIdentityConflict,
    IllegalStageTransition {
        from: MutationStage,
        to: MutationStage,
    },
    StaleEpoch {
        expected: u64,
        actual: u64,
    },
    ReplicaProofRequired,
    ReplicaProofMismatch,
    EpochMustAdvance,
}

impl fmt::Display for ValuePlaneError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ValuePlaneError {}

/// Executable bounded reference state.
#[derive(Debug, Clone)]
pub struct ReferenceValuePlane {
    bounds: ValuePlaneBounds,
    epoch: u64,
    owner: String,
    logical_time: LogicalTime,
    connection_generation: u64,
    entries: BTreeMap<CanonicalMapKey, EntryState>,
    mutations: BTreeMap<MutationIdentity, MutationRecord>,
    history: Vec<MutationHistoryEvent>,
}

impl ReferenceValuePlane {
    pub fn new(bounds: ValuePlaneBounds, epoch: u64) -> Result<Self, ValuePlaneError> {
        Ok(Self {
            bounds: bounds.validate()?,
            epoch,
            owner: "owner-a".to_owned(),
            logical_time: LogicalTime::default(),
            connection_generation: 1,
            entries: BTreeMap::new(),
            mutations: BTreeMap::new(),
            history: Vec::new(),
        })
    }

    pub const fn epoch(&self) -> u64 {
        self.epoch
    }

    pub const fn connection_generation(&self) -> u64 {
        self.connection_generation
    }

    pub fn set_logical_time(&mut self, time: LogicalTime) {
        self.logical_time = time;
    }

    pub fn advance_logical_time(&mut self, delta: u64) {
        self.logical_time = self.logical_time.saturating_add(delta);
    }

    pub fn mutation(&self, identity: &MutationIdentity) -> Option<&MutationRecord> {
        self.mutations.get(identity)
    }

    pub fn history(&self) -> &[MutationHistoryEvent] {
        &self.history
    }

    pub fn live_value(&self, key: &CanonicalMapKey) -> Option<&[u8]> {
        match self.entries.get(key) {
            Some(EntryState::Live {
                value, expires_at, ..
            }) if expires_at.is_none_or(|deadline| deadline > self.logical_time) => Some(value),
            Some(EntryState::Live { .. }) | Some(EntryState::Tombstone { .. }) | None => None,
        }
    }

    pub fn validate_bulk(&self, plans: &[MutationPlan]) -> Result<(), ValuePlaneError> {
        self.check_bound("bulk_items", self.bounds.max_bulk_items, plans.len())?;
        for plan in plans {
            self.validate_plan(plan)?;
        }
        Ok(())
    }

    /// Receive or replay a mutation. A replay returns the exact retained record.
    pub fn receive(
        &mut self,
        plan: MutationPlan,
    ) -> Result<Option<MutationRecord>, ValuePlaneError> {
        self.validate_plan(&plan)?;
        if let Some(existing) = self.mutations.get(&plan.identity) {
            if existing.plan.digest != plan.digest {
                return Err(ValuePlaneError::MutationIdentityConflict);
            }
            return Ok(Some(existing.clone()));
        }
        self.check_bound(
            "dedup_entries",
            self.bounds.max_dedup_entries,
            self.mutations.len().saturating_add(1),
        )?;
        self.reserve_history_event()?;
        let identity = plan.identity.clone();
        self.mutations.insert(
            identity.clone(),
            MutationRecord {
                stage: MutationStage::Received,
                outcome: None,
                replica_proof: None,
                plan,
            },
        );
        self.history.push(MutationHistoryEvent {
            identity,
            stage: MutationStage::Received,
        });
        Ok(None)
    }

    pub fn advance(
        &mut self,
        identity: &MutationIdentity,
        to: MutationStage,
    ) -> Result<(), ValuePlaneError> {
        let from = self
            .mutations
            .get(identity)
            .ok_or(ValuePlaneError::UnknownMutation)?
            .stage;
        if from.next() != Some(to) || to == MutationStage::OwnerApplied {
            return Err(ValuePlaneError::IllegalStageTransition { from, to });
        }
        self.record_stage(identity, to)
    }

    pub fn apply_owner(
        &mut self,
        identity: &MutationIdentity,
    ) -> Result<MutationOutcome, ValuePlaneError> {
        let record = self
            .mutations
            .get(identity)
            .ok_or(ValuePlaneError::UnknownMutation)?;
        if record.stage != MutationStage::OwnerDecided {
            return Err(ValuePlaneError::IllegalStageTransition {
                from: record.stage,
                to: MutationStage::OwnerApplied,
            });
        }
        if record.plan.expected_epoch != self.epoch {
            return Err(ValuePlaneError::StaleEpoch {
                expected: record.plan.expected_epoch,
                actual: self.epoch,
            });
        }
        self.reserve_history_event()?;
        let plan = record.plan.clone();
        let outcome = self.apply_operation(&plan.key, &plan.operation);
        let record = self
            .mutations
            .get_mut(identity)
            .expect("mutation checked above");
        record.stage = MutationStage::OwnerApplied;
        record.outcome = Some(outcome.clone());
        self.history.push(MutationHistoryEvent {
            identity: identity.clone(),
            stage: MutationStage::OwnerApplied,
        });
        Ok(outcome)
    }

    pub fn mark_visible_owner(
        &mut self,
        identity: &MutationIdentity,
    ) -> Result<(), ValuePlaneError> {
        self.require_and_record(
            identity,
            MutationStage::OwnerApplied,
            MutationStage::VisibleOwner,
        )
    }

    pub fn prove_replica(
        &mut self,
        identity: &MutationIdentity,
        replica: impl Into<String>,
    ) -> Result<(), ValuePlaneError> {
        let replica = replica.into();
        let record = self
            .mutations
            .get(identity)
            .ok_or(ValuePlaneError::UnknownMutation)?;
        if record.stage != MutationStage::VisibleOwner {
            return Err(ValuePlaneError::IllegalStageTransition {
                from: record.stage,
                to: MutationStage::ReplicaProved,
            });
        }
        if replica.is_empty() || replica == self.owner {
            return Err(ValuePlaneError::ReplicaProofMismatch);
        }
        self.reserve_history_event()?;
        let record = self
            .mutations
            .get_mut(identity)
            .expect("mutation checked above");
        record.stage = MutationStage::ReplicaProved;
        record.replica_proof = Some(replica);
        self.history.push(MutationHistoryEvent {
            identity: identity.clone(),
            stage: MutationStage::ReplicaProved,
        });
        Ok(())
    }

    pub fn acknowledge(
        &mut self,
        identity: &MutationIdentity,
    ) -> Result<MutationOutcome, ValuePlaneError> {
        let record = self
            .mutations
            .get(identity)
            .ok_or(ValuePlaneError::UnknownMutation)?;
        if record.replica_proof.is_none() {
            return Err(ValuePlaneError::ReplicaProofRequired);
        }
        if record.stage != MutationStage::ReplicaProved {
            return Err(ValuePlaneError::IllegalStageTransition {
                from: record.stage,
                to: MutationStage::Acknowledged,
            });
        }
        self.record_stage(identity, MutationStage::Acknowledged)?;
        self.mutations
            .get(identity)
            .and_then(|record| record.outcome.clone())
            .ok_or(ValuePlaneError::UnknownMutation)
    }

    pub fn respond(
        &mut self,
        identity: &MutationIdentity,
    ) -> Result<MutationOutcome, ValuePlaneError> {
        self.require_and_record(
            identity,
            MutationStage::Acknowledged,
            MutationStage::Responded,
        )?;
        self.outcome_with_certainty(identity, OutcomeCertainty::Certain)
    }

    pub fn lose_response(
        &mut self,
        identity: &MutationIdentity,
    ) -> Result<MutationOutcome, ValuePlaneError> {
        self.require_and_record(
            identity,
            MutationStage::Acknowledged,
            MutationStage::OutcomeUnknown,
        )?;
        self.outcome_with_certainty(identity, OutcomeCertainty::OutcomeUnknown)
    }

    pub fn promote(&mut self, epoch: u64, owner: impl Into<String>) -> Result<(), ValuePlaneError> {
        self.advance_epoch(epoch, owner)
    }

    pub fn rebalance(
        &mut self,
        epoch: u64,
        owner: impl Into<String>,
    ) -> Result<(), ValuePlaneError> {
        self.advance_epoch(epoch, owner)
    }

    pub fn reconnect(&mut self) {
        self.connection_generation = self.connection_generation.saturating_add(1);
    }

    fn advance_epoch(
        &mut self,
        epoch: u64,
        owner: impl Into<String>,
    ) -> Result<(), ValuePlaneError> {
        if epoch <= self.epoch {
            return Err(ValuePlaneError::EpochMustAdvance);
        }
        self.epoch = epoch;
        self.owner = owner.into();
        Ok(())
    }

    fn outcome_with_certainty(
        &mut self,
        identity: &MutationIdentity,
        certainty: OutcomeCertainty,
    ) -> Result<MutationOutcome, ValuePlaneError> {
        let record = self
            .mutations
            .get_mut(identity)
            .ok_or(ValuePlaneError::UnknownMutation)?;
        let mut outcome = record
            .outcome
            .clone()
            .ok_or(ValuePlaneError::UnknownMutation)?;
        outcome.certainty = certainty;
        record.outcome = Some(outcome.clone());
        Ok(outcome)
    }

    fn require_and_record(
        &mut self,
        identity: &MutationIdentity,
        from: MutationStage,
        to: MutationStage,
    ) -> Result<(), ValuePlaneError> {
        let actual = self
            .mutations
            .get(identity)
            .ok_or(ValuePlaneError::UnknownMutation)?
            .stage;
        if actual != from {
            return Err(ValuePlaneError::IllegalStageTransition { from: actual, to });
        }
        self.record_stage(identity, to)
    }

    fn record_stage(
        &mut self,
        identity: &MutationIdentity,
        stage: MutationStage,
    ) -> Result<(), ValuePlaneError> {
        self.reserve_history_event()?;
        self.mutations
            .get_mut(identity)
            .ok_or(ValuePlaneError::UnknownMutation)?
            .stage = stage;
        self.history.push(MutationHistoryEvent {
            identity: identity.clone(),
            stage,
        });
        Ok(())
    }

    fn reserve_history_event(&self) -> Result<(), ValuePlaneError> {
        self.check_bound(
            "history_events",
            self.bounds.max_history_events,
            self.history.len().saturating_add(1),
        )
    }

    fn validate_plan(&self, plan: &MutationPlan) -> Result<(), ValuePlaneError> {
        self.check_bound(
            "key_bytes",
            self.bounds.max_key_bytes,
            plan.key.key_bytes.len(),
        )?;
        self.check_bound(
            "value_bytes",
            self.bounds.max_value_bytes,
            plan.operation.encoded_value_bytes(),
        )
    }

    fn check_bound(
        &self,
        bound: &'static str,
        limit: usize,
        actual: usize,
    ) -> Result<(), ValuePlaneError> {
        if actual > limit {
            Err(ValuePlaneError::BoundExceeded {
                bound,
                limit,
                actual,
            })
        } else {
            Ok(())
        }
    }

    fn apply_operation(
        &mut self,
        key: &CanonicalMapKey,
        operation: &MutationOperation,
    ) -> MutationOutcome {
        let previous = self.live_value(key).map(<[u8]>::to_vec);
        let previous_expiry = match self.entries.get(key) {
            Some(EntryState::Live { expires_at, .. }) if previous.is_some() => *expires_at,
            _ => None,
        };
        let next_version = self
            .entries
            .get(key)
            .map_or(1, |entry| entry.version().saturating_add(1));
        let (applied, replacement, ttl) = match operation {
            MutationOperation::Put { value, ttl } | MutationOperation::GetAndPut { value, ttl } => {
                (true, Some(value.clone()), Some(*ttl))
            }
            MutationOperation::PutIfAbsent { value, ttl } => {
                (previous.is_none(), Some(value.clone()), Some(*ttl))
            }
            MutationOperation::ReplaceIfPresent { value, ttl } => {
                (previous.is_some(), Some(value.clone()), Some(*ttl))
            }
            MutationOperation::ReplaceIfValue {
                expected,
                value,
                ttl,
            } => (
                previous.as_ref() == Some(expected),
                Some(value.clone()),
                Some(*ttl),
            ),
            MutationOperation::GetAndRemove | MutationOperation::Delete => {
                (previous.is_some(), None, None)
            }
        };

        let version = if applied {
            if let Some(value) = replacement {
                let expires_at = match ttl.expect("replacement operations carry ttl") {
                    TtlDirective::Preserve => previous_expiry,
                    TtlDirective::Eternal => None,
                    TtlDirective::ExpireAfter(delta) => {
                        Some(self.logical_time.saturating_add(delta))
                    }
                };
                self.entries.insert(
                    key.clone(),
                    EntryState::Live {
                        value,
                        expires_at,
                        version: next_version,
                    },
                );
            } else {
                self.entries.insert(
                    key.clone(),
                    EntryState::Tombstone {
                        version: next_version,
                    },
                );
            }
            Some(next_version)
        } else {
            None
        };
        let current = self.live_value(key).map(<[u8]>::to_vec);
        MutationOutcome {
            applied,
            previous,
            current,
            version,
            certainty: OutcomeCertainty::Certain,
        }
    }
}

/// Deterministic failure classes applied at explicit lifecycle checkpoints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FaultKind {
    OwnerLoss,
    BackupLoss,
    Promotion { epoch: u64 },
    Rebalance { epoch: u64 },
    Reconnect,
    ResponseLoss,
}

impl FaultKind {
    pub const OWNER_LOSS_CLASS: &'static str = "owner_loss";
    pub const BACKUP_LOSS_CLASS: &'static str = "backup_loss";
    pub const PROMOTION_CLASS: &'static str = "promotion";
    pub const REBALANCE_CLASS: &'static str = "rebalance";
    pub const RECONNECT_CLASS: &'static str = "reconnect";
    pub const RESPONSE_LOSS_CLASS: &'static str = "response_loss";

    fn class(&self) -> &'static str {
        match self {
            Self::OwnerLoss => Self::OWNER_LOSS_CLASS,
            Self::BackupLoss => Self::BACKUP_LOSS_CLASS,
            Self::Promotion { .. } => Self::PROMOTION_CLASS,
            Self::Rebalance { .. } => Self::REBALANCE_CLASS,
            Self::Reconnect => Self::RECONNECT_CLASS,
            Self::ResponseLoss => Self::RESPONSE_LOSS_CLASS,
        }
    }
}

/// One explicit fault and the logical mutation stage at which it fires.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaultCheckpoint {
    pub stage: MutationStage,
    pub fault: FaultKind,
}

impl FaultCheckpoint {
    pub const fn new(stage: MutationStage, fault: FaultKind) -> Self {
        Self { stage, fault }
    }
}

/// Bounded replayable fault schedule. Ordering and fingerprint are platform-independent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeterministicFaultSchedule {
    seed: u64,
    checkpoints: Vec<FaultCheckpoint>,
}

impl DeterministicFaultSchedule {
    const DEFAULT_MAX_STEPS: usize = 64;

    pub fn new(seed: u64, checkpoints: Vec<FaultCheckpoint>) -> Result<Self, ValuePlaneError> {
        Self::new_bounded(seed, checkpoints, Self::DEFAULT_MAX_STEPS)
    }

    pub fn new_bounded(
        seed: u64,
        checkpoints: Vec<FaultCheckpoint>,
        max_steps: usize,
    ) -> Result<Self, ValuePlaneError> {
        if max_steps == 0 {
            return Err(ValuePlaneError::InvalidBound("fault_steps"));
        }
        if checkpoints.len() > max_steps {
            return Err(ValuePlaneError::BoundExceeded {
                bound: "fault_steps",
                limit: max_steps,
                actual: checkpoints.len(),
            });
        }
        Ok(Self { seed, checkpoints })
    }

    pub const fn seed(&self) -> u64 {
        self.seed
    }

    pub fn checkpoints(&self) -> &[FaultCheckpoint] {
        &self.checkpoints
    }

    pub fn includes(&self, class: &str) -> bool {
        self.checkpoints
            .iter()
            .any(|checkpoint| checkpoint.fault.class() == class)
    }

    pub fn replay_fingerprint(&self) -> u64 {
        const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
        const PRIME: u64 = 0x100_0000_01b3;
        let mut hash = OFFSET ^ self.seed;
        for checkpoint in &self.checkpoints {
            for byte in format!("{:?}:{:?}", checkpoint.stage, checkpoint.fault).bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(PRIME);
            }
        }
        hash
    }
}

/// Stable identity of the bounded authority model used by receipts and gates.
pub const AUTHORITY_MODEL_ID: &str = "hydracache.imap.value-plane.authority.v1";

/// Invariants checked after every explored transition.
pub const AUTHORITY_INVARIANT_IDS: &[&str] = &[
    "HC-IMAP-INV-ACK-REQUIRES-REPLICA-PROOF",
    "HC-IMAP-INV-REPLICA-PROOF-DOES-NOT-EXCEED-OWNER-APPLY",
    "HC-IMAP-INV-SERVING-OWNER-IS-LIVE",
    "HC-IMAP-INV-PROMOTED-OWNER-COVERS-ACKNOWLEDGED-PREFIX",
    "HC-IMAP-INV-RESPONSE-REQUIRES-ACKNOWLEDGEMENT",
    "HC-IMAP-INV-TOMBSTONE-DOMINATES-OLDER-LIVE-VALUE",
];

/// Hard exploration bounds. Zero values are rejected rather than interpreted as unlimited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthorityModelBounds {
    pub max_depth: usize,
    pub max_states: usize,
}

impl Default for AuthorityModelBounds {
    fn default() -> Self {
        Self {
            max_depth: 8,
            max_states: 20_000,
        }
    }
}

/// Pure authority state. It contains no process handles, transports, or wall-clock values.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AuthorityModelSnapshot {
    pub epoch: u64,
    pub applied_version: u64,
    pub replica_proved_version: u64,
    pub acknowledged_version: u64,
    pub responded_version: u64,
    pub served_version: u64,
    pub live_version: u64,
    pub tombstone_version: u64,
    pub owner_alive: bool,
    pub backup_alive: bool,
    pub serving_owner: bool,
    pub outcome_unknown: bool,
}

impl Default for AuthorityModelSnapshot {
    fn default() -> Self {
        Self {
            epoch: 1,
            applied_version: 0,
            replica_proved_version: 0,
            acknowledged_version: 0,
            responded_version: 0,
            served_version: 0,
            live_version: 0,
            tombstone_version: 0,
            owner_alive: true,
            backup_alive: true,
            serving_owner: true,
            outcome_unknown: false,
        }
    }
}

/// Closed transition vocabulary explored by [`AuthorityModelExplorer`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuthorityModelAction {
    ApplyValue,
    ApplyTombstone,
    ProveReplica,
    Acknowledge,
    Respond,
    LoseResponse,
    LoseOwner,
    LoseBackup,
    RestoreBackup,
    PromoteBackup,
    Rebalance,
}

/// One invariant violation with the shortest explored action prefix that exposed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityModelViolation {
    pub invariant_id: &'static str,
    pub detail: String,
    pub trace: Vec<AuthorityModelAction>,
}

/// Deterministic bounded exploration result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityModelReport {
    pub model_id: &'static str,
    pub explored_states: usize,
    pub explored_transitions: usize,
    pub max_depth_reached: usize,
    pub truncated: bool,
    pub violations: Vec<AuthorityModelViolation>,
}

impl AuthorityModelReport {
    pub fn passed(&self) -> bool {
        self.violations.is_empty() && !self.truncated
    }
}

/// Exhaustive bounded checker for ACK, response-loss, promotion, rebalance and tombstone safety.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthorityModelExplorer {
    bounds: AuthorityModelBounds,
}

impl AuthorityModelExplorer {
    pub fn new(bounds: AuthorityModelBounds) -> Result<Self, ValuePlaneError> {
        if bounds.max_depth == 0 {
            return Err(ValuePlaneError::InvalidBound("authority_model_depth"));
        }
        if bounds.max_states == 0 {
            return Err(ValuePlaneError::InvalidBound("authority_model_states"));
        }
        Ok(Self { bounds })
    }

    pub fn explore(&self) -> AuthorityModelReport {
        use std::collections::{BTreeSet, VecDeque};

        let initial = AuthorityModelSnapshot::default();
        let mut seen = BTreeSet::from([initial.clone()]);
        let mut queue = VecDeque::from([(initial, Vec::new())]);
        let mut transitions = 0_usize;
        let mut max_depth = 0_usize;
        let mut truncated = false;
        let mut violations = Vec::new();

        while let Some((state, trace)) = queue.pop_front() {
            max_depth = max_depth.max(trace.len());
            for (invariant_id, detail) in authority_invariant_violations(&state) {
                violations.push(AuthorityModelViolation {
                    invariant_id,
                    detail,
                    trace: trace.clone(),
                });
            }
            if trace.len() >= self.bounds.max_depth {
                continue;
            }
            for action in AuthorityModelAction::all() {
                let Some(next) = apply_authority_action(&state, action) else {
                    continue;
                };
                transitions = transitions.saturating_add(1);
                if seen.contains(&next) {
                    continue;
                }
                if seen.len() >= self.bounds.max_states {
                    truncated = true;
                    continue;
                }
                let mut next_trace = trace.clone();
                next_trace.push(action);
                seen.insert(next.clone());
                queue.push_back((next, next_trace));
            }
        }

        AuthorityModelReport {
            model_id: AUTHORITY_MODEL_ID,
            explored_states: seen.len(),
            explored_transitions: transitions,
            max_depth_reached: max_depth,
            truncated,
            violations,
        }
    }
}

impl AuthorityModelAction {
    fn all() -> [Self; 11] {
        [
            Self::ApplyValue,
            Self::ApplyTombstone,
            Self::ProveReplica,
            Self::Acknowledge,
            Self::Respond,
            Self::LoseResponse,
            Self::LoseOwner,
            Self::LoseBackup,
            Self::RestoreBackup,
            Self::PromoteBackup,
            Self::Rebalance,
        ]
    }
}

/// Return every authority invariant violation without panicking, for gates and canaries.
pub fn authority_invariant_violations(
    state: &AuthorityModelSnapshot,
) -> Vec<(&'static str, String)> {
    let mut violations = Vec::new();
    if state.acknowledged_version > state.replica_proved_version {
        violations.push((
            AUTHORITY_INVARIANT_IDS[0],
            "acknowledged version exceeds replica proof".to_owned(),
        ));
    }
    if state.replica_proved_version > state.applied_version {
        violations.push((
            AUTHORITY_INVARIANT_IDS[1],
            "replica proof exceeds owner-applied version".to_owned(),
        ));
    }
    if state.serving_owner && !state.owner_alive {
        violations.push((
            AUTHORITY_INVARIANT_IDS[2],
            "a dead owner remains serving".to_owned(),
        ));
    }
    if state.serving_owner && state.served_version < state.acknowledged_version {
        violations.push((
            AUTHORITY_INVARIANT_IDS[3],
            "serving owner is behind the acknowledged prefix".to_owned(),
        ));
    }
    if state.responded_version > state.acknowledged_version {
        violations.push((
            AUTHORITY_INVARIANT_IDS[4],
            "response version exceeds acknowledgement".to_owned(),
        ));
    }
    if state.live_version != 0
        && state.tombstone_version != 0
        && state.live_version <= state.tombstone_version
    {
        violations.push((
            AUTHORITY_INVARIANT_IDS[5],
            "an older live value is visible behind a tombstone".to_owned(),
        ));
    }
    violations
}

fn apply_authority_action(
    state: &AuthorityModelSnapshot,
    action: AuthorityModelAction,
) -> Option<AuthorityModelSnapshot> {
    let mut next = state.clone();
    match action {
        AuthorityModelAction::ApplyValue if state.owner_alive && state.serving_owner => {
            next.applied_version = state.applied_version.saturating_add(1);
            next.live_version = next.applied_version;
            next.served_version = next.applied_version;
            next.outcome_unknown = false;
        }
        AuthorityModelAction::ApplyTombstone if state.owner_alive && state.serving_owner => {
            next.applied_version = state.applied_version.saturating_add(1);
            next.tombstone_version = next.applied_version;
            next.live_version = 0;
            next.served_version = next.applied_version;
            next.outcome_unknown = false;
        }
        AuthorityModelAction::ProveReplica
            if state.backup_alive && state.replica_proved_version < state.applied_version =>
        {
            next.replica_proved_version = state.applied_version;
        }
        AuthorityModelAction::Acknowledge
            if state.applied_version > state.acknowledged_version
                && state.replica_proved_version >= state.applied_version =>
        {
            next.acknowledged_version = state.applied_version;
        }
        AuthorityModelAction::Respond if state.acknowledged_version > state.responded_version => {
            next.responded_version = state.acknowledged_version;
            next.outcome_unknown = false;
        }
        AuthorityModelAction::LoseResponse if state.acknowledged_version > 0 => {
            next.outcome_unknown = true;
        }
        AuthorityModelAction::LoseOwner if state.owner_alive => {
            next.owner_alive = false;
            next.serving_owner = false;
        }
        AuthorityModelAction::LoseBackup if state.backup_alive => {
            next.backup_alive = false;
        }
        AuthorityModelAction::RestoreBackup if !state.backup_alive => {
            next.backup_alive = true;
        }
        AuthorityModelAction::PromoteBackup
            if !state.owner_alive
                && state.backup_alive
                && state.replica_proved_version == state.applied_version
                && state.replica_proved_version >= state.acknowledged_version =>
        {
            next.epoch = state.epoch.saturating_add(1);
            next.owner_alive = true;
            next.serving_owner = true;
            next.applied_version = state.replica_proved_version;
            next.served_version = state.replica_proved_version;
            next.backup_alive = false;
        }
        AuthorityModelAction::Rebalance
            if state.owner_alive
                && state.backup_alive
                && state.replica_proved_version == state.applied_version =>
        {
            next.epoch = state.epoch.saturating_add(1);
            next.applied_version = state.replica_proved_version;
            next.served_version = state.replica_proved_version;
        }
        _ => return None,
    }
    Some(next)
}
