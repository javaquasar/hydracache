//! Test-only cross-surface projection into one canonical reference map.

use std::collections::{BTreeMap, BTreeSet};

use crate::value_plane_model_075::{
    CanonicalMapKey, MutationDigest, MutationIdentity, MutationOperation, MutationOutcome,
    MutationPlan, MutationStage, ReferenceValuePlane, ValuePlaneBounds, ValuePlaneError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SurfaceId {
    Resp,
    Hc1,
    Hc2Rust,
    Hc2Java,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SurfaceCapability {
    Read,
    Mutate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngressPrincipal {
    pub tenant: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfaceOperation {
    Read,
    Mutation {
        identity: MutationIdentity,
        operation: MutationOperation,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedMapCommand {
    pub surface: SurfaceId,
    pub tenant: String,
    pub namespace: String,
    pub namespace_generation: u64,
    pub key: Vec<u8>,
    pub operation: SurfaceOperation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfaceOutcome {
    Read(Option<Vec<u8>>),
    Mutation(MutationOutcome),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfaceError {
    InvalidCommand(&'static str),
    TenantSubstitution,
    Unsupported,
    Model(ValuePlaneError),
}

impl std::fmt::Display for SurfaceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for SurfaceError {}

impl From<ValuePlaneError> for SurfaceError {
    fn from(value: ValuePlaneError) -> Self {
        Self::Model(value)
    }
}

impl VerifiedMapCommand {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        surface: SurfaceId,
        principal: &IngressPrincipal,
        requested_tenant: &str,
        namespace: impl Into<String>,
        namespace_generation: u64,
        key: Vec<u8>,
        operation: SurfaceOperation,
    ) -> Result<Self, SurfaceError> {
        if principal.tenant != requested_tenant {
            return Err(SurfaceError::TenantSubstitution);
        }
        let namespace = namespace.into();
        if principal.tenant.is_empty()
            || namespace.is_empty()
            || namespace_generation == 0
            || key.is_empty()
        {
            return Err(SurfaceError::InvalidCommand("identity"));
        }
        Ok(Self {
            surface,
            tenant: principal.tenant.clone(),
            namespace,
            namespace_generation,
            key,
            operation,
        })
    }
}

#[derive(Debug)]
pub struct CrossSurfaceReferenceMap {
    partitions: u32,
    epoch: u64,
    plane: ReferenceValuePlane,
    capabilities: BTreeMap<SurfaceId, BTreeSet<SurfaceCapability>>,
    event_count: u64,
    accounted_value_bytes: usize,
}

impl CrossSurfaceReferenceMap {
    pub fn new(partitions: u32, bounds: ValuePlaneBounds) -> Result<Self, SurfaceError> {
        if partitions == 0 {
            return Err(SurfaceError::InvalidCommand("partitions"));
        }
        let capabilities = [
            SurfaceId::Resp,
            SurfaceId::Hc1,
            SurfaceId::Hc2Rust,
            SurfaceId::Hc2Java,
        ]
        .into_iter()
        .map(|surface| {
            (
                surface,
                BTreeSet::from([SurfaceCapability::Read, SurfaceCapability::Mutate]),
            )
        })
        .collect();
        Ok(Self {
            partitions,
            epoch: 1,
            plane: ReferenceValuePlane::new(bounds, 1)?,
            capabilities,
            event_count: 0,
            accounted_value_bytes: 0,
        })
    }

    pub fn disable(&mut self, surface: SurfaceId, capability: SurfaceCapability) {
        if let Some(capabilities) = self.capabilities.get_mut(&surface) {
            capabilities.remove(&capability);
        }
    }

    pub fn execute(&mut self, command: VerifiedMapCommand) -> Result<SurfaceOutcome, SurfaceError> {
        let required = match &command.operation {
            SurfaceOperation::Read => SurfaceCapability::Read,
            SurfaceOperation::Mutation { .. } => SurfaceCapability::Mutate,
        };
        if !self
            .capabilities
            .get(&command.surface)
            .is_some_and(|capabilities| capabilities.contains(&required))
        {
            return Err(SurfaceError::Unsupported);
        }
        let partition = reference_partition(&command.key, self.partitions);
        let key = CanonicalMapKey::new(
            command.tenant,
            command.namespace,
            command.namespace_generation,
            command.key,
            partition,
        );
        match command.operation {
            SurfaceOperation::Read => Ok(SurfaceOutcome::Read(
                self.plane.live_value(&key).map(<[u8]>::to_vec),
            )),
            SurfaceOperation::Mutation {
                identity,
                operation,
            } => {
                let digest = operation_digest(&key, &operation);
                let plan =
                    MutationPlan::new(identity.clone(), digest, key.clone(), operation, self.epoch);
                if let Some(record) = self.plane.receive(plan)? {
                    return Ok(SurfaceOutcome::Mutation(
                        record.outcome.ok_or(ValuePlaneError::UnknownMutation)?,
                    ));
                }
                for stage in [
                    MutationStage::Admitted,
                    MutationStage::Routed,
                    MutationStage::OwnerDecided,
                ] {
                    self.plane.advance(&identity, stage)?;
                }
                let previous_bytes = self.plane.live_value(&key).map_or(0, <[u8]>::len);
                self.plane.apply_owner(&identity)?;
                self.plane.mark_visible_owner(&identity)?;
                self.plane.prove_replica(&identity, "reference-backup")?;
                self.plane.acknowledge(&identity)?;
                let outcome = self.plane.respond(&identity).map_err(SurfaceError::from)?;
                if outcome.applied {
                    let current_bytes = outcome.current.as_ref().map_or(0, Vec::len);
                    self.accounted_value_bytes = self
                        .accounted_value_bytes
                        .saturating_sub(previous_bytes)
                        .saturating_add(current_bytes);
                    self.event_count = self.event_count.saturating_add(1);
                }
                Ok(SurfaceOutcome::Mutation(outcome))
            }
        }
    }

    pub const fn event_count(&self) -> u64 {
        self.event_count
    }

    pub const fn accounted_value_bytes(&self) -> usize {
        self.accounted_value_bytes
    }
}

fn reference_partition(key: &[u8], partitions: u32) -> u32 {
    key.iter().fold(0_u32, |hash, byte| {
        hash.wrapping_mul(16777619) ^ u32::from(*byte)
    }) % partitions
}

fn operation_digest(key: &CanonicalMapKey, operation: &MutationOperation) -> MutationDigest {
    let mut digest = key
        .key_bytes
        .iter()
        .fold(0xcbf2_9ce4_8422_2325, |digest, byte| {
            (digest ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3)
        });
    for byte in format!("{operation:?}").bytes() {
        digest = (digest ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3);
    }
    MutationDigest::new(digest)
}
