//! Dev-only 0.75 interface seams. No production crate may depend on this module.

use async_trait::async_trait;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NamespaceIdentity {
    pub tenant: u64,
    pub namespace: u64,
    pub generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartitionRoute {
    pub partition: u32,
    pub epoch: u64,
    pub owner: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplicaProof {
    pub partition: u32,
    pub epoch: u64,
    pub version: u64,
    pub backup: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProvisionalBackendError {
    DisabledBlockedBy074,
    StaleGeneration,
    StaleEpoch,
    MissingReplicaProof,
    InjectedFault,
}

/// Deliberately unforgeable outside this module: the only constructible state is disabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProvisionalCapability {
    state: CapabilityState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CapabilityState {
    DisabledBlockedBy074,
}

impl ProvisionalCapability {
    pub fn disabled() -> Self {
        Self {
            state: CapabilityState::DisabledBlockedBy074,
        }
    }

    pub fn require_enabled(self) -> Result<(), ProvisionalBackendError> {
        match self.state {
            CapabilityState::DisabledBlockedBy074 => {
                Err(ProvisionalBackendError::DisabledBlockedBy074)
            }
        }
    }
}

impl Default for ProvisionalCapability {
    fn default() -> Self {
        Self::disabled()
    }
}

#[async_trait]
pub trait ProvisionalClientDataBackend: Send + Sync {
    async fn get(
        &self,
        namespace: NamespaceIdentity,
        key: &[u8],
    ) -> Result<Option<Vec<u8>>, ProvisionalBackendError>;

    async fn put(
        &self,
        namespace: NamespaceIdentity,
        route: PartitionRoute,
        key: Vec<u8>,
        value: Vec<u8>,
    ) -> Result<u64, ProvisionalBackendError>;
}

pub trait PartitionRouteOracle: Send + Sync {
    fn route(&self, namespace: NamespaceIdentity, key: &[u8]) -> PartitionRoute;
}

pub trait ReplicaProofOracle: Send + Sync {
    fn validate(&self, route: PartitionRoute, proof: ReplicaProof) -> bool;
}

#[async_trait]
pub trait FaultInjectableBackend: Send + Sync {
    async fn inject_before_owner_apply(
        &self,
        route: PartitionRoute,
    ) -> Result<(), ProvisionalBackendError>;

    async fn inject_after_replica_proof(
        &self,
        proof: ReplicaProof,
    ) -> Result<(), ProvisionalBackendError>;
}
