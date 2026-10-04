//! Bounded, generation-aware replica acknowledgement tracker for the provisional 0.75 model.

use std::collections::{BTreeMap, BTreeSet};

use crate::value_plane_model_075::MutationIdentity;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AckTrackerBounds {
    pub max_in_flight: usize,
    pub max_in_flight_bytes: usize,
    pub max_expected_replicas: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReplicaGeneration {
    pub node: String,
    pub generation: u64,
}

impl ReplicaGeneration {
    pub fn new(node: impl Into<String>, generation: u64) -> Self {
        Self {
            node: node.into(),
            generation,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AckContract {
    pub partition: u32,
    pub epoch: u64,
    pub version: u64,
    pub checksum: u64,
    pub mutation: MutationIdentity,
    pub expected_replicas: BTreeSet<ReplicaGeneration>,
    pub required: usize,
    pub retained_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicaApplyAck {
    pub partition: u32,
    pub epoch: u64,
    pub version: u64,
    pub checksum: u64,
    pub mutation: MutationIdentity,
    pub replica: ReplicaGeneration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AckProgress {
    Pending { received: usize, required: usize },
    Satisfied { received: usize, required: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AckTrackerError {
    InvalidBound(&'static str),
    InvalidContract(&'static str),
    CapacityExceeded(&'static str),
    DuplicateMutation,
    UnknownMutation,
    UnexpectedReplica,
    DuplicateAck,
    ClaimMismatch(&'static str),
    NotSatisfied,
}

#[derive(Debug, Clone)]
struct AckState {
    contract: AckContract,
    received: BTreeSet<ReplicaGeneration>,
}

/// Tracks only in-flight proof ownership. Every terminal path must remove its entry.
#[derive(Debug)]
pub struct ReplicaAckTracker {
    bounds: AckTrackerBounds,
    retained_bytes: usize,
    in_flight: BTreeMap<MutationIdentity, AckState>,
}

impl ReplicaAckTracker {
    pub fn new(bounds: AckTrackerBounds) -> Result<Self, AckTrackerError> {
        for (name, value) in [
            ("in_flight", bounds.max_in_flight),
            ("in_flight_bytes", bounds.max_in_flight_bytes),
            ("expected_replicas", bounds.max_expected_replicas),
        ] {
            if value == 0 {
                return Err(AckTrackerError::InvalidBound(name));
            }
        }
        Ok(Self {
            bounds,
            retained_bytes: 0,
            in_flight: BTreeMap::new(),
        })
    }

    pub fn register(&mut self, contract: AckContract) -> Result<(), AckTrackerError> {
        if contract.epoch == 0 || contract.version == 0 {
            return Err(AckTrackerError::InvalidContract("epoch_or_version"));
        }
        if contract.expected_replicas.is_empty()
            || contract.required == 0
            || contract.required > contract.expected_replicas.len()
        {
            return Err(AckTrackerError::InvalidContract("required_replicas"));
        }
        if contract
            .expected_replicas
            .iter()
            .any(|replica| replica.node.is_empty() || replica.generation == 0)
        {
            return Err(AckTrackerError::InvalidContract("replica_generation"));
        }
        if contract.expected_replicas.len() > self.bounds.max_expected_replicas {
            return Err(AckTrackerError::CapacityExceeded("expected_replicas"));
        }
        if self.in_flight.contains_key(&contract.mutation) {
            return Err(AckTrackerError::DuplicateMutation);
        }
        if self.in_flight.len() >= self.bounds.max_in_flight {
            return Err(AckTrackerError::CapacityExceeded("in_flight"));
        }
        let next_bytes = self.retained_bytes.saturating_add(contract.retained_bytes);
        if next_bytes > self.bounds.max_in_flight_bytes {
            return Err(AckTrackerError::CapacityExceeded("in_flight_bytes"));
        }
        self.retained_bytes = next_bytes;
        self.in_flight.insert(
            contract.mutation.clone(),
            AckState {
                contract,
                received: BTreeSet::new(),
            },
        );
        Ok(())
    }

    pub fn record_ack(&mut self, ack: ReplicaApplyAck) -> Result<AckProgress, AckTrackerError> {
        let state = self
            .in_flight
            .get_mut(&ack.mutation)
            .ok_or(AckTrackerError::UnknownMutation)?;
        for (matches, field) in [
            (state.contract.partition == ack.partition, "partition"),
            (state.contract.epoch == ack.epoch, "epoch"),
            (state.contract.version == ack.version, "version"),
            (state.contract.checksum == ack.checksum, "checksum"),
        ] {
            if !matches {
                return Err(AckTrackerError::ClaimMismatch(field));
            }
        }
        if !state.contract.expected_replicas.contains(&ack.replica) {
            return Err(AckTrackerError::UnexpectedReplica);
        }
        if !state.received.insert(ack.replica) {
            return Err(AckTrackerError::DuplicateAck);
        }
        let received = state.received.len();
        let required = state.contract.required;
        if received >= required {
            Ok(AckProgress::Satisfied { received, required })
        } else {
            Ok(AckProgress::Pending { received, required })
        }
    }

    pub fn finish(&mut self, mutation: &MutationIdentity) -> Result<AckContract, AckTrackerError> {
        let state = self
            .in_flight
            .get(mutation)
            .ok_or(AckTrackerError::UnknownMutation)?;
        if state.received.len() < state.contract.required {
            return Err(AckTrackerError::NotSatisfied);
        }
        self.remove(mutation)
    }

    pub fn cancel(&mut self, mutation: &MutationIdentity) -> Result<AckContract, AckTrackerError> {
        self.remove(mutation)
    }

    pub fn timeout(&mut self, mutation: &MutationIdentity) -> Result<AckContract, AckTrackerError> {
        self.remove(mutation)
    }

    pub fn len(&self) -> usize {
        self.in_flight.len()
    }

    pub fn is_empty(&self) -> bool {
        self.in_flight.is_empty()
    }

    pub const fn retained_bytes(&self) -> usize {
        self.retained_bytes
    }

    fn remove(&mut self, mutation: &MutationIdentity) -> Result<AckContract, AckTrackerError> {
        let state = self
            .in_flight
            .remove(mutation)
            .ok_or(AckTrackerError::UnknownMutation)?;
        self.retained_bytes = self
            .retained_bytes
            .saturating_sub(state.contract.retained_bytes);
        Ok(state.contract)
    }
}
