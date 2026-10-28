//! Online, runtime-only transfer of actual owner decisions. The initial snapshot is followed
//! by sparse record, outcome and retirement decisions, never reevaluated commands.
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TransferComponent {
    Data,
    Journal,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnlineTransferPhase {
    Snapshot,
    Delta,
    Ready,
    Committed,
    Aborted,
    RestartRequired,
}
#[derive(Debug, Clone, Copy)]
pub struct OnlineTransferOptions {
    pub chunk_bytes: usize,
    pub max_snapshot_bytes: usize,
    pub max_delta_images: usize,
    pub max_delta_bytes: usize,
}
impl Default for OnlineTransferOptions {
    fn default() -> Self {
        Self {
            chunk_bytes: 16 * 1_024,
            max_snapshot_bytes: 8 * 1_024 * 1_024,
            max_delta_images: 64,
            max_delta_bytes: 8 * 1_024 * 1_024,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferSnapshotManifest {
    pub component: TransferComponent,
    pub stamp: ReplicaStamp,
    pub total_bytes: usize,
    pub chunk_count: usize,
    pub checksum: [u8; 32],
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnlineTransferManifest {
    pub id: u64,
    pub partition: u32,
    pub source: String,
    pub target: String,
    pub source_epoch: u64,
    pub snapshots: Vec<TransferSnapshotManifest>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnlineTransferChunk {
    pub transfer: u64,
    pub component: TransferComponent,
    pub index: usize,
    pub bytes: Vec<u8>,
    pub checksum: [u8; 32],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferAcceptance {
    Stored,
    Replayed,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnlineDeltaDescriptor {
    pub sequence: u64,
    pub component: TransferComponent,
    pub base: ReplicaStamp,
    pub target: ReplicaStamp,
    pub payload_bytes: usize,
    pub checksum: [u8; 32],
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnlineTransferReadiness {
    pub id: u64,
    pub partition: u32,
    pub source: String,
    pub target: String,
    pub phase: OnlineTransferPhase,
    pub pending_deltas: usize,
    pub staged_bytes: usize,
    pub ready: bool,
    pub restart_reason: Option<&'static str>,
}

struct SnapshotComponent {
    manifest: TransferSnapshotManifest,
    image: Option<ServiceReplicaImage>,
    cursor: ServiceReplicaCursor,
    payload: Vec<u8>,
    accepted: BTreeMap<usize, Vec<u8>>,
    staging: UnifiedIMapService,
}
struct DecisionDelta {
    descriptor: OnlineDeltaDescriptor,
    decisions: ServiceReplicaDelta,
    payload: Vec<u8>,
}
pub(super) struct OnlineTransferState {
    manifest: OnlineTransferManifest,
    options: OnlineTransferOptions,
    phase: OnlineTransferPhase,
    components: BTreeMap<TransferComponent, SnapshotComponent>,
    captured: BTreeMap<TransferComponent, ReplicaStamp>,
    deltas: VecDeque<DecisionDelta>,
    applied: VecDeque<(u64, [u8; 32])>,
    next_delta: u64,
    restart_reason: Option<&'static str>,
}
impl OnlineTransferState {
    pub(super) fn owned_bytes(&self) -> usize {
        let mut bytes = 256usize;
        for part in self.components.values() {
            let usage = part.staging.resource_usage();
            bytes = bytes
                .saturating_add(part.payload.len())
                .saturating_add(part.accepted.values().map(Vec::len).sum::<usize>())
                .saturating_add(part.accepted.len().saturating_mul(32));
            bytes = [
                usage.values,
                usage.outcomes,
                usage.pending,
                usage.listeners,
                usage.streams,
                usage.resync,
            ]
            .into_iter()
            .fold(bytes, usize::saturating_add);
        }
        bytes = self.deltas.iter().fold(bytes, |sum, delta| {
            sum.saturating_add(192).saturating_add(delta.payload.len())
        });
        bytes.saturating_add(self.applied.len().saturating_mul(48))
    }
    fn delta_bytes(&self) -> usize {
        self.deltas
            .iter()
            .map(|delta| delta.payload.len().saturating_add(192))
            .sum()
    }
    fn resident_bytes(&self) -> usize {
        self.components.values().fold(
            self.deltas.iter().fold(self.owned_bytes(), |sum, delta| {
                sum.saturating_add(delta.decisions.owned_payload_bytes())
            }),
            |sum, part| {
                sum.saturating_add(part.cursor.owned_payload_bytes())
                    .saturating_add(
                        part.image
                            .as_ref()
                            .map_or(0, ServiceReplicaImage::owned_payload_bytes),
                    )
            },
        )
    }
    fn active(&self) -> bool {
        matches!(
            self.phase,
            OnlineTransferPhase::Snapshot | OnlineTransferPhase::Delta | OnlineTransferPhase::Ready
        )
    }
    fn restart(&mut self, reason: &'static str) {
        self.phase = OnlineTransferPhase::RestartRequired;
        self.restart_reason = Some(reason);
        self.deltas.clear();
        self.components.clear();
    }
    fn readiness(&self) -> OnlineTransferReadiness {
        OnlineTransferReadiness {
            id: self.manifest.id,
            partition: self.manifest.partition,
            source: self.manifest.source.clone(),
            target: self.manifest.target.clone(),
            phase: self.phase,
            pending_deltas: self.deltas.len(),
            staged_bytes: self.resident_bytes(),
            ready: self.phase == OnlineTransferPhase::Ready,
            restart_reason: self.restart_reason,
        }
    }
}

impl ServiceClusterHarness {
    pub(super) fn has_active_transfer(&self, partition: u32) -> bool {
        self.runtime
            .transfers
            .values()
            .any(|state| state.manifest.partition == partition && state.active())
    }
    pub fn begin_online_transfer(
        &mut self,
        p: u32,
        target: &str,
        options: OnlineTransferOptions,
    ) -> Result<u64, ServiceClusterError> {
        self.ensure_namespace()?;
        self.require_live(target)?;
        let route = self
            .assignments
            .get(p as usize)
            .ok_or(ServiceClusterError::InvalidBound)?
            .route
            .clone();
        self.require_live(&route.owner)?;
        if target == route.owner
            || options.chunk_bytes == 0
            || options.chunk_bytes > 64 * 1_024
            || options.max_snapshot_bytes == 0
            || options.max_snapshot_bytes > self.runtime.limits.max_transfer_bytes
            || options.max_delta_images == 0
            || options.max_delta_images > 1_024
            || options.max_delta_bytes == 0
            || options.max_delta_bytes > self.runtime.limits.max_transfer_bytes
            || self.runtime.transfers.len() >= self.runtime.limits.max_transfers
            || self
                .runtime
                .transfers
                .values()
                .any(|transfer| transfer.manifest.partition == p && transfer.active())
        {
            return Err(ServiceClusterError::InvalidBound);
        }
        let id = self.runtime.next_transfer;
        let next = id.checked_add(1).ok_or(ServiceClusterError::InvalidBound)?;
        let used = self
            .runtime
            .transfers
            .values()
            .map(OnlineTransferState::resident_bytes)
            .sum::<usize>();
        let mut admission = 256usize;
        for component in [TransferComponent::Data, TransferComponent::Journal] {
            let source = self.service(&route.owner, p, component == TransferComponent::Journal)?;
            source.replica_stamp()?;
            let (image_bytes, cursor_bytes) = source.preflight_replica_snapshot_with_cursor()?;
            let payload_bytes = source.replica_transport_size(options.max_snapshot_bytes)?;
            if payload_bytes.div_ceil(options.chunk_bytes) > 4_096 {
                return Err(ServiceClusterError::InvalidBound);
            }
            admission = admission
                .checked_add(image_bytes)
                .and_then(|n| n.checked_add(cursor_bytes))
                .and_then(|n| n.checked_add(payload_bytes))
                .ok_or(MapServiceError::ResourceBudget("transfer_staging"))?;
        }
        if used.saturating_add(admission) > self.runtime.limits.max_transfer_bytes {
            return Err(MapServiceError::ResourceBudget("transfer_staging").into());
        }
        let mut components = BTreeMap::new();
        let mut manifests = Vec::new();
        let mut captured = BTreeMap::new();
        for component in [TransferComponent::Data, TransferComponent::Journal] {
            let source = self.service(&route.owner, p, component == TransferComponent::Journal)?;
            let image = source.export_replica_image()?;
            let cursor = image.replication_cursor()?;
            let payload = image.transport_payload(options.max_snapshot_bytes)?;
            let count = payload.len().div_ceil(options.chunk_bytes);
            if count == 0 || count > 4_096 {
                return Err(ServiceClusterError::InvalidBound);
            }
            let manifest = TransferSnapshotManifest {
                component,
                stamp: image.stamp(),
                total_bytes: payload.len(),
                chunk_count: count,
                checksum: Sha256::digest(&payload).into(),
            };
            captured.insert(component, manifest.stamp);
            manifests.push(manifest.clone());
            components.insert(
                component,
                SnapshotComponent {
                    manifest,
                    image: Some(image),
                    cursor,
                    payload,
                    accepted: BTreeMap::new(),
                    staging: UnifiedIMapService::with_limits(
                        self.collection,
                        self.generation,
                        self.service_limits,
                    )?,
                },
            );
        }
        let state = OnlineTransferState {
            manifest: OnlineTransferManifest {
                id,
                partition: p,
                source: route.owner,
                target: target.into(),
                source_epoch: route.epoch,
                snapshots: manifests,
            },
            options,
            phase: OnlineTransferPhase::Snapshot,
            components,
            captured,
            deltas: VecDeque::new(),
            applied: VecDeque::new(),
            next_delta: 1,
            restart_reason: None,
        };
        let used = self
            .runtime
            .transfers
            .values()
            .map(OnlineTransferState::resident_bytes)
            .sum::<usize>();
        if used.saturating_add(state.resident_bytes()) > self.runtime.limits.max_transfer_bytes {
            return Err(MapServiceError::ResourceBudget("transfer_staging").into());
        }
        self.runtime.next_transfer = next;
        self.runtime.transfers.insert(id, state);
        Ok(id)
    }

    pub fn online_transfer_manifest(
        &self,
        id: u64,
    ) -> Result<OnlineTransferManifest, ServiceClusterError> {
        Ok(self
            .runtime
            .transfers
            .get(&id)
            .ok_or(ServiceClusterError::UnknownTransfer)?
            .manifest
            .clone())
    }
    pub fn transfer_readiness(&self) -> Vec<OnlineTransferReadiness> {
        self.runtime
            .transfers
            .values()
            .map(|state| {
                let mut readiness = state.readiness();
                readiness.ready &= self.validate_transfer_route(state).is_ok();
                readiness
            })
            .collect()
    }
    pub fn online_transfer_chunk(
        &self,
        id: u64,
        component: TransferComponent,
        index: usize,
    ) -> Result<OnlineTransferChunk, ServiceClusterError> {
        let state = self
            .runtime
            .transfers
            .get(&id)
            .ok_or(ServiceClusterError::UnknownTransfer)?;
        if state.phase != OnlineTransferPhase::Snapshot {
            return Err(ServiceClusterError::TransferState);
        }
        self.require_live(&state.manifest.source)?;
        let snapshot = state
            .components
            .get(&component)
            .ok_or(ServiceClusterError::ChunkOutOfRange)?;
        if index >= snapshot.manifest.chunk_count {
            return Err(ServiceClusterError::ChunkOutOfRange);
        }
        let start = index
            .checked_mul(state.options.chunk_bytes)
            .ok_or(ServiceClusterError::ChunkOutOfRange)?;
        let bytes = snapshot.payload
            [start..(start + state.options.chunk_bytes).min(snapshot.payload.len())]
            .to_vec();
        let checksum = chunk_checksum(id, component, index, &bytes);
        Ok(OnlineTransferChunk {
            transfer: id,
            component,
            index,
            bytes,
            checksum,
        })
    }
    pub fn accept_online_transfer_chunk(
        &mut self,
        id: u64,
        chunk: OnlineTransferChunk,
    ) -> Result<TransferAcceptance, ServiceClusterError> {
        self.ensure_namespace()?;
        let identity = self
            .runtime
            .transfers
            .get(&id)
            .ok_or(ServiceClusterError::UnknownTransfer)?;
        self.require_live(&identity.manifest.target)?;
        self.validate_transfer_identity(identity)?;
        if identity.phase != OnlineTransferPhase::Snapshot || chunk.transfer != id {
            return Err(ServiceClusterError::TransferState);
        }
        let snapshot = identity
            .components
            .get(&chunk.component)
            .ok_or(ServiceClusterError::ChunkOutOfRange)?;
        if chunk.index >= snapshot.manifest.chunk_count
            || chunk.bytes.len() > identity.options.chunk_bytes
        {
            return Err(ServiceClusterError::ChunkOutOfRange);
        }
        self.authorize_message(
            ServiceMessageKind::ReplicaApply,
            &identity.manifest.source,
            &identity.manifest.target,
            identity.manifest.partition,
            Some(snapshot.manifest.stamp),
            &chunk.bytes,
        )?;
        let total = self
            .runtime
            .transfers
            .values()
            .map(OnlineTransferState::resident_bytes)
            .sum::<usize>();
        let state = self
            .runtime
            .transfers
            .get_mut(&id)
            .ok_or(ServiceClusterError::UnknownTransfer)?;
        if state.phase != OnlineTransferPhase::Snapshot || chunk.transfer != id {
            return Err(ServiceClusterError::TransferState);
        }
        let snapshot = state
            .components
            .get_mut(&chunk.component)
            .ok_or(ServiceClusterError::ChunkOutOfRange)?;
        if chunk.index >= snapshot.manifest.chunk_count
            || chunk.bytes.len() > state.options.chunk_bytes
        {
            return Err(ServiceClusterError::ChunkOutOfRange);
        }
        if chunk_checksum(id, chunk.component, chunk.index, &chunk.bytes) != chunk.checksum {
            return Err(ServiceClusterError::ConflictingChunk);
        }
        if let Some(existing) = snapshot.accepted.get(&chunk.index) {
            return if existing == &chunk.bytes {
                Ok(TransferAcceptance::Replayed)
            } else {
                Err(ServiceClusterError::ConflictingChunk)
            };
        }
        let start = chunk.index * state.options.chunk_bytes;
        let expected = &snapshot.payload
            [start..(start + state.options.chunk_bytes).min(snapshot.payload.len())];
        if expected != chunk.bytes.as_slice() {
            return Err(ServiceClusterError::ConflictingChunk);
        }
        if total.saturating_add(chunk.bytes.len()).saturating_add(32)
            > self.runtime.limits.max_transfer_bytes
        {
            return Err(MapServiceError::ResourceBudget("transfer_staging").into());
        }
        snapshot.accepted.insert(chunk.index, chunk.bytes);
        Ok(TransferAcceptance::Stored)
    }

    pub fn finish_online_snapshot(&mut self, id: u64) -> Result<(), ServiceClusterError> {
        self.ensure_namespace()?;
        let mut state = self
            .runtime
            .transfers
            .remove(&id)
            .ok_or(ServiceClusterError::UnknownTransfer)?;
        let result = (|| {
            self.validate_transfer_route(&state)?;
            if state.phase != OnlineTransferPhase::Snapshot {
                return Err(ServiceClusterError::TransferState);
            }
            for snapshot in state.components.values() {
                if snapshot.accepted.len() != snapshot.manifest.chunk_count {
                    return Err(ServiceClusterError::MissingChunks);
                }
                let mut digest = Sha256::new();
                for index in 0..snapshot.manifest.chunk_count {
                    digest.update(
                        snapshot
                            .accepted
                            .get(&index)
                            .ok_or(ServiceClusterError::MissingChunks)?,
                    );
                }
                if <[u8; 32]>::from(digest.finalize()) != snapshot.manifest.checksum {
                    return Err(ServiceClusterError::ConflictingChunk);
                }
            }
            for snapshot in state.components.values() {
                self.authorize_message(
                    ServiceMessageKind::ReplicaApply,
                    &state.manifest.source,
                    &state.manifest.target,
                    state.manifest.partition,
                    Some(snapshot.manifest.stamp),
                    &snapshot.payload,
                )?;
            }
            for snapshot in state.components.values_mut() {
                snapshot.staging.install_replica_image(
                    snapshot
                        .image
                        .take()
                        .ok_or(ServiceClusterError::TransferState)?,
                )?;
                snapshot.payload.clear();
                snapshot.accepted.clear();
            }
            state.phase = OnlineTransferPhase::Delta;
            Ok(())
        })();
        self.runtime.transfers.insert(id, state);
        result
    }

    pub fn next_online_delta(
        &self,
        id: u64,
    ) -> Result<Option<OnlineDeltaDescriptor>, ServiceClusterError> {
        Ok(self
            .runtime
            .transfers
            .get(&id)
            .ok_or(ServiceClusterError::UnknownTransfer)?
            .deltas
            .front()
            .map(|delta| delta.descriptor.clone()))
    }
    pub fn online_delta_measure(
        &self,
        id: u64,
        sequence: u64,
    ) -> Result<ReplicaDeltaMeasure, ServiceClusterError> {
        self.runtime
            .transfers
            .get(&id)
            .ok_or(ServiceClusterError::UnknownTransfer)?
            .deltas
            .iter()
            .find(|delta| delta.descriptor.sequence == sequence)
            .map(|delta| delta.decisions.measure())
            .ok_or(ServiceClusterError::DeltaOutOfOrder)
    }

    pub fn apply_online_delta(
        &mut self,
        id: u64,
        sequence: u64,
        checksum: [u8; 32],
    ) -> Result<TransferAcceptance, ServiceClusterError> {
        self.ensure_namespace()?;
        let mut state = self
            .runtime
            .transfers
            .remove(&id)
            .ok_or(ServiceClusterError::UnknownTransfer)?;
        let result = (|| {
            self.validate_transfer_route(&state)?;
            if !matches!(
                state.phase,
                OnlineTransferPhase::Delta | OnlineTransferPhase::Ready
            ) {
                return Err(ServiceClusterError::TransferState);
            }
            if let Some((_, original)) = state
                .applied
                .iter()
                .find(|(applied, _)| *applied == sequence)
            {
                return if *original == checksum {
                    Ok(TransferAcceptance::Replayed)
                } else {
                    Err(ServiceClusterError::ConflictingDelta)
                };
            }
            let delta = state
                .deltas
                .front()
                .ok_or(ServiceClusterError::DeltaOutOfOrder)?;
            if delta.descriptor.sequence != sequence {
                return Err(ServiceClusterError::DeltaOutOfOrder);
            }
            if delta.descriptor.checksum != checksum
                || <[u8; 32]>::from(Sha256::digest(&delta.payload)) != checksum
            {
                return Err(ServiceClusterError::ConflictingDelta);
            }
            let staging = &state.components[&delta.descriptor.component].staging;
            if staging.replica_stamp()? != delta.descriptor.base {
                return Err(ServiceClusterError::DeltaOutOfOrder);
            }
            self.authorize_message(
                ServiceMessageKind::ReplicaApply,
                &state.manifest.source,
                &state.manifest.target,
                state.manifest.partition,
                Some(delta.descriptor.target),
                &delta.payload,
            )?;
            let delta = state.deltas.pop_front().expect("verified delta head");
            let installed = staging.install_replica_delta(delta.decisions)?;
            if installed != delta.descriptor.target {
                return Err(ServiceClusterError::ReplicaNotProved);
            }
            state.applied.push_back((sequence, checksum));
            if state.applied.len() > state.options.max_delta_images {
                state.applied.pop_front();
            }
            state.phase = OnlineTransferPhase::Delta;
            Ok(TransferAcceptance::Stored)
        })();
        self.runtime.transfers.insert(id, state);
        result
    }

    pub fn ready_online_transfer(
        &mut self,
        id: u64,
    ) -> Result<OnlineTransferReadiness, ServiceClusterError> {
        self.ensure_namespace()?;
        self.capture_online_changes();
        let mut state = self
            .runtime
            .transfers
            .remove(&id)
            .ok_or(ServiceClusterError::UnknownTransfer)?;
        let result = (|| {
            self.validate_transfer_route(&state)?;
            if !matches!(
                state.phase,
                OnlineTransferPhase::Delta | OnlineTransferPhase::Ready
            ) || !state.deltas.is_empty()
            {
                return Err(ServiceClusterError::TransferState);
            }
            for component in [TransferComponent::Data, TransferComponent::Journal] {
                let source = self.service(
                    &state.manifest.source,
                    state.manifest.partition,
                    component == TransferComponent::Journal,
                )?;
                let actual = source.replica_stamp()?;
                if state.components[&component].staging.replica_stamp()? != actual {
                    return Err(ServiceClusterError::ReplicaNotProved);
                }
            }
            state.phase = OnlineTransferPhase::Ready;
            Ok(state.readiness())
        })();
        self.runtime.transfers.insert(id, state);
        result
    }

    pub fn commit_online_transfer(
        &mut self,
        id: u64,
    ) -> Result<ServicePartitionAssignment, ServiceClusterError> {
        self.ensure_namespace()?;
        self.capture_online_changes();
        let mut state = self
            .runtime
            .transfers
            .remove(&id)
            .ok_or(ServiceClusterError::UnknownTransfer)?;
        let result = (|| {
            self.validate_transfer_route(&state)?;
            if state.phase != OnlineTransferPhase::Ready || !state.deltas.is_empty() {
                return Err(ServiceClusterError::TransferState);
            }
            let other = self
                .runtime
                .transfers
                .values()
                .map(OnlineTransferState::resident_bytes)
                .sum::<usize>();
            let extra = state
                .components
                .values()
                .map(|part| part.staging.preflight_replica_image())
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .max()
                .unwrap_or(0);
            if other
                .saturating_add(state.resident_bytes())
                .saturating_add(extra)
                > self.runtime.limits.max_transfer_bytes
            {
                return Err(MapServiceError::ResourceBudget("transfer_commit_image").into());
            }
            let p = state.manifest.partition;
            let epoch = state
                .manifest
                .source_epoch
                .checked_add(1)
                .ok_or(ServiceClusterError::InvalidBound)?;
            for component in [TransferComponent::Data, TransferComponent::Journal] {
                let source = self.service(
                    &state.manifest.source,
                    p,
                    component == TransferComponent::Journal,
                )?;
                let expected = source.replica_stamp()?;
                let staged = &state.components[&component].staging;
                if staged.replica_stamp()? != expected {
                    return Err(ServiceClusterError::ReplicaNotProved);
                }
                let image = staged.export_replica_image()?;
                let payload = image.transport_payload(state.options.max_snapshot_bytes)?;
                self.authorize_message(
                    ServiceMessageKind::ReplicaApply,
                    &state.manifest.source,
                    &state.manifest.target,
                    p,
                    Some(expected),
                    &payload,
                )?;
                self.service(
                    &state.manifest.target,
                    p,
                    component == TransferComponent::Journal,
                )?
                .install_replica_image(image)?;
                self.authorize_message(
                    ServiceMessageKind::Ack,
                    &state.manifest.target,
                    &state.manifest.source,
                    p,
                    Some(expected),
                    &expected.digest,
                )?;
            }
            let assignment = &mut self.assignments[p as usize];
            assignment.route = ServicePartitionAssignment {
                partition: p,
                epoch,
                owner: state.manifest.target.clone(),
                backup: state.manifest.source.clone(),
            };
            assignment.required_data = state.components[&TransferComponent::Data]
                .staging
                .replica_stamp()?;
            assignment.required_journal = state.components[&TransferComponent::Journal]
                .staging
                .replica_stamp()?;
            assignment.backup_data = Some(assignment.required_data);
            assignment.backup_journal = Some(assignment.required_journal);
            let committed = assignment.route.clone();
            state.phase = OnlineTransferPhase::Committed;
            state.components.clear();
            state.applied.clear();
            self.refresh_subscriptions();
            Ok(committed)
        })();
        self.runtime.transfers.insert(id, state);
        result
    }

    pub fn abort_online_transfer(&mut self, id: u64) -> Result<(), ServiceClusterError> {
        let state = self
            .runtime
            .transfers
            .get_mut(&id)
            .ok_or(ServiceClusterError::UnknownTransfer)?;
        if state.phase == OnlineTransferPhase::Committed {
            return Err(ServiceClusterError::TransferState);
        }
        state.phase = OnlineTransferPhase::Aborted;
        state.components.clear();
        state.deltas.clear();
        state.applied.clear();
        Ok(())
    }
    pub fn cleanup_online_transfer(&mut self, id: u64) -> Result<bool, ServiceClusterError> {
        let Some(state) = self.runtime.transfers.get(&id) else {
            return Ok(false);
        };
        if state.active() {
            return Err(ServiceClusterError::TransferState);
        }
        self.runtime.transfers.remove(&id);
        Ok(true)
    }

    fn validate_transfer_route(
        &self,
        state: &OnlineTransferState,
    ) -> Result<(), ServiceClusterError> {
        self.require_live(&state.manifest.source)?;
        self.require_live(&state.manifest.target)?;
        self.validate_transfer_identity(state)
    }
    fn validate_transfer_identity(
        &self,
        state: &OnlineTransferState,
    ) -> Result<(), ServiceClusterError> {
        let route = &self.assignments[state.manifest.partition as usize].route;
        if route.epoch != state.manifest.source_epoch || route.owner != state.manifest.source {
            return Err(ServiceClusterError::StaleEpoch);
        }
        Ok(())
    }
    pub(super) fn preflight_transfer_call(
        &self,
        command: &ServiceCommand,
    ) -> Result<(), ServiceClusterError> {
        self.validate_request(command)?;
        if !is_mutating_operation(&command.operation) {
            return Ok(());
        }
        for state in self
            .runtime
            .transfers
            .values()
            .filter(|state| state.active())
        {
            let touches = self
                .groups(
                    command,
                    service_operation_scope(&command.operation)
                        .ok_or(ServiceClusterError::UnsupportedScope)?,
                )?
                .iter()
                .any(|(p, _, _)| *p == state.manifest.partition)
                || command
                    .identity
                    .as_ref()
                    .is_some_and(|id| self.coordinator_for(id) == state.manifest.partition);
            if touches && state.deltas.len().saturating_add(2) > state.options.max_delta_images {
                return Err(ServiceClusterError::TransferBackpressure);
            }
        }
        Ok(())
    }
    pub(super) fn capture_online_changes(&mut self) {
        let ids = self.runtime.transfers.keys().copied().collect::<Vec<_>>();
        for id in ids {
            let Some(mut state) = self.runtime.transfers.remove(&id) else {
                continue;
            };
            if state.active() {
                if self.validate_transfer_identity(&state).is_err() {
                    state.restart("source_or_target_changed");
                } else if self.nodes[&state.manifest.source].live {
                    for component in [TransferComponent::Data, TransferComponent::Journal] {
                        let source = match self.service(
                            &state.manifest.source,
                            state.manifest.partition,
                            component == TransferComponent::Journal,
                        ) {
                            Ok(source) => source,
                            Err(_) => {
                                state.restart("source_unavailable");
                                break;
                            }
                        };
                        let actual = match source.replica_stamp() {
                            Ok(stamp) => stamp,
                            Err(MapServiceError::ActiveOwners) => continue,
                            Err(_) => {
                                state.restart("source_stamp_unavailable");
                                break;
                            }
                        };
                        let base = state.captured[&component];
                        if actual == base {
                            continue;
                        }
                        let measured = match source.preflight_replica_delta(
                            &state.components[&component].cursor,
                            state.options.max_delta_bytes,
                        ) {
                            Ok(measured) => measured,
                            Err(MapServiceError::ActiveOwners) => continue,
                            Err(_) => {
                                state.restart("delta_bytes");
                                break;
                            }
                        };
                        let predicted = measured.payload_bytes;
                        let used = self
                            .runtime
                            .transfers
                            .values()
                            .map(OnlineTransferState::resident_bytes)
                            .sum::<usize>();
                        if used
                            .saturating_add(state.resident_bytes())
                            .saturating_add(predicted)
                            .saturating_add(measured.owned_bytes)
                            .saturating_add(measured.cursor_growth)
                            .saturating_add(192)
                            > self.runtime.limits.max_transfer_bytes
                        {
                            state.restart("aggregate_transfer_budget");
                            break;
                        }
                        if state.deltas.len() == state.options.max_delta_images
                            || state
                                .delta_bytes()
                                .saturating_add(predicted)
                                .saturating_add(192)
                                > state.options.max_delta_bytes
                        {
                            state.restart("delta_log_full");
                            break;
                        }
                        let decisions = match source.export_replica_delta(
                            &mut state
                                .components
                                .get_mut(&component)
                                .expect("snapshot component")
                                .cursor,
                            state.options.max_delta_bytes,
                        ) {
                            Ok(decisions) => decisions,
                            Err(_) => {
                                state.restart("decision_delta_budget");
                                break;
                            }
                        };
                        let payload =
                            match decisions.transport_payload(state.options.max_delta_bytes) {
                                Ok(payload) => payload,
                                Err(_) => {
                                    state.restart("delta_bytes");
                                    break;
                                }
                            };
                        if state.deltas.len() == state.options.max_delta_images
                            || state
                                .delta_bytes()
                                .saturating_add(payload.len())
                                .saturating_add(192)
                                > state.options.max_delta_bytes
                        {
                            state.restart("delta_log_full");
                            break;
                        }
                        let Some(next) = state.next_delta.checked_add(1) else {
                            state.restart("delta_sequence_exhausted");
                            break;
                        };
                        let descriptor = OnlineDeltaDescriptor {
                            sequence: state.next_delta,
                            component,
                            base,
                            target: actual,
                            payload_bytes: payload.len(),
                            checksum: Sha256::digest(&payload).into(),
                        };
                        state.next_delta = next;
                        state.captured.insert(component, actual);
                        state.deltas.push_back(DecisionDelta {
                            descriptor,
                            decisions,
                            payload,
                        });
                        if state.phase == OnlineTransferPhase::Ready {
                            state.phase = OnlineTransferPhase::Delta;
                        }
                    }
                    let other = self
                        .runtime
                        .transfers
                        .values()
                        .map(OnlineTransferState::resident_bytes)
                        .sum::<usize>();
                    if state.active()
                        && other.saturating_add(state.resident_bytes())
                            > self.runtime.limits.max_transfer_bytes
                    {
                        state.restart("aggregate_transfer_budget");
                    }
                }
            }
            self.runtime.transfers.insert(id, state);
        }
    }
}

fn chunk_checksum(id: u64, component: TransferComponent, index: usize, bytes: &[u8]) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(id.to_be_bytes());
    digest.update([component as u8]);
    digest.update(index.to_be_bytes());
    digest.update(bytes);
    digest.finalize().into()
}
