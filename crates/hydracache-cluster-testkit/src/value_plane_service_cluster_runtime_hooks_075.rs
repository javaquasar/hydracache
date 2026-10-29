//! Runtime adapters over the actual cluster owners; no values are stored here.
use super::*;
use crate::value_plane_collection_075::ScanProjection;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceMessageKind {
    Request,
    ReplicaApply,
    Ack,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceMessageContext {
    pub kind: ServiceMessageKind,
    pub source: String,
    pub target: String,
    pub source_generation: u64,
    pub target_generation: u64,
    pub partition: u32,
    pub epoch: u64,
    pub request_digest: [u8; 32],
    pub mutation_identity: Option<MutationIdentity>,
    pub replica_stamp: Option<ReplicaStamp>,
    pub now: u64,
    pub deadline: u64,
    pub payload_digest: [u8; 32],
    pub payload_bytes: usize,
}

pub trait ServiceMessageGuard: Send + Sync {
    fn authorize(&self, message: &ServiceMessageContext) -> Result<(), ServiceClusterError>;
}

pub(super) struct Invocation {
    pub digest: [u8; 32],
    pub identity: Option<MutationIdentity>,
    pub now: u64,
    pub deadline: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClusterRuntimeLimits {
    pub max_subscriptions: usize,
    pub max_subscription_bytes: usize,
    pub max_transfers: usize,
    pub max_transfer_bytes: usize,
}
impl Default for ClusterRuntimeLimits {
    fn default() -> Self {
        Self {
            max_subscriptions: 128,
            max_subscription_bytes: 16 * 1_024 * 1_024,
            max_transfers: 4,
            max_transfer_bytes: 32 * 1_024 * 1_024,
        }
    }
}

pub(super) struct ClusterRuntimeState {
    pub limits: ClusterRuntimeLimits,
    pub namespace_fenced: bool,
    pub draining: BTreeSet<String>,
    pub subscriptions: BTreeMap<u64, subscriptions::SubscriptionState>,
    pub subscription_streams: BTreeMap<u64, usize>,
    pub transfers: BTreeMap<u64, transfer::OnlineTransferState>,
    pub next_subscription: u64,
    pub next_transfer: u64,
    pub deferred_wakes: Vec<Waker>,
    pub guard: Option<Arc<dyn ServiceMessageGuard>>,
    pub invocation: Option<Invocation>,
}
impl Default for ClusterRuntimeState {
    fn default() -> Self {
        Self {
            limits: ClusterRuntimeLimits::default(),
            namespace_fenced: false,
            draining: BTreeSet::new(),
            subscriptions: BTreeMap::new(),
            subscription_streams: BTreeMap::new(),
            transfers: BTreeMap::new(),
            next_subscription: 1,
            next_transfer: 1,
            deferred_wakes: Vec::new(),
            guard: None,
            invocation: None,
        }
    }
}

fn add_resources(total: &mut ServiceResourceUsage, part: ServiceResourceUsage) {
    total.values = total.values.saturating_add(part.values);
    total.outcomes = total.outcomes.saturating_add(part.outcomes);
    total.pending = total.pending.saturating_add(part.pending);
    total.listeners = total.listeners.saturating_add(part.listeners);
    total.streams = total.streams.saturating_add(part.streams);
    total.resync = total.resync.saturating_add(part.resync);
}
fn add_owners(total: &mut NamespaceOwners, part: NamespaceOwners) {
    total.values += part.values;
    total.completed_outcomes += part.completed_outcomes;
    total.retirement_fences += part.retirement_fences;
    total.pending += part.pending;
    total.listeners += part.listeners;
    total.streams += part.streams;
    total.resync_leases += part.resync_leases;
    total.resync_bytes += part.resync_bytes;
}

impl ServiceClusterHarness {
    pub(super) fn all_services(&self) -> Vec<UnifiedIMapService> {
        self.nodes
            .values()
            .flat_map(|node| node.data.iter().chain(&node.journal))
            .cloned()
            .collect()
    }
    pub(super) fn ensure_namespace(&self) -> Result<(), ServiceClusterError> {
        if self.runtime.namespace_fenced {
            Err(MapServiceError::NamespaceDeleted.into())
        } else {
            Ok(())
        }
    }
    pub const fn generation(&self) -> u64 {
        self.generation
    }
    pub const fn partition_count(&self) -> u32 {
        self.bounds.partitions
    }
    pub fn node_ids(&self) -> Vec<String> {
        self.nodes.keys().cloned().collect()
    }
    pub fn node_is_live(&self, node: &str) -> Result<bool, ServiceClusterError> {
        Ok(self
            .nodes
            .get(node)
            .ok_or(ServiceClusterError::UnknownNode)?
            .live)
    }
    pub fn node_generation(&self, node: &str) -> Result<u64, ServiceClusterError> {
        Ok(self
            .nodes
            .get(node)
            .ok_or(ServiceClusterError::UnknownNode)?
            .generation)
    }
    pub fn node_is_draining(&self, node: &str) -> bool {
        self.runtime.draining.contains(node)
    }
    pub fn logical_time(&self) -> u64 {
        self.all_services()
            .iter()
            .map(UnifiedIMapService::logical_time)
            .max()
            .unwrap_or(0)
    }
    pub fn max_revision(&self) -> u64 {
        self.all_services()
            .iter()
            .map(UnifiedIMapService::revision)
            .max()
            .unwrap_or(1)
    }
    pub fn is_namespace_fenced(&self) -> bool {
        self.runtime.namespace_fenced
    }

    pub fn set_runtime_limits(
        &mut self,
        limits: ClusterRuntimeLimits,
    ) -> Result<(), ServiceClusterError> {
        if [
            limits.max_subscriptions,
            limits.max_subscription_bytes,
            limits.max_transfers,
            limits.max_transfer_bytes,
        ]
        .contains(&0)
            || !self.runtime.subscriptions.is_empty()
            || !self.runtime.subscription_streams.is_empty()
            || !self.runtime.transfers.is_empty()
        {
            return Err(ServiceClusterError::InvalidBound);
        }
        self.runtime.limits = limits;
        Ok(())
    }

    pub fn aggregate_resource_usage(&self) -> ServiceResourceUsage {
        let mut total = ServiceResourceUsage::default();
        for service in self.all_services() {
            add_resources(&mut total, service.resource_usage());
        }
        total.listeners = total.listeners.saturating_add(
            self.runtime
                .subscriptions
                .values()
                .map(subscriptions::SubscriptionState::owned_bytes)
                .sum::<usize>(),
        );
        total.streams = total
            .streams
            .saturating_add(self.runtime.subscription_streams.values().sum::<usize>());
        total.resync = total.resync.saturating_add(
            self.runtime
                .transfers
                .values()
                .map(transfer::OnlineTransferState::owned_bytes)
                .sum::<usize>(),
        );
        total
    }
    pub fn aggregate_namespace_owners(&self) -> NamespaceOwners {
        let mut total = NamespaceOwners::default();
        for service in self.all_services() {
            add_owners(&mut total, service.namespace_owners());
        }
        total.listeners += self.runtime.subscriptions.len();
        total.streams += self.runtime.subscription_streams.len();
        total.resync_leases += self.runtime.transfers.len();
        total
    }
    pub fn aggregate_statistics(&self) -> ServiceStatistics {
        let mut total = ServiceStatistics::default();
        for service in self.all_services() {
            let part = service.statistics();
            total.live_entries += part.live_entries;
            total.value_bytes += part.value_bytes;
            total.retained_outcomes += part.retained_outcomes;
            total.listeners += part.listeners;
            total.queued_events += part.queued_events;
            total.inflight += part.inflight;
            total.rejections = total.rejections.saturating_add(part.rejections);
            total.cancellations = total.cancellations.saturating_add(part.cancellations);
            total.requests = total.requests.saturating_add(part.requests);
            total.active_streams += part.active_streams;
            total.active_cursors += part.active_cursors;
        }
        total.listeners += self.runtime.subscriptions.len();
        total.active_streams += self.runtime.subscription_streams.len();
        total.queued_events += self
            .runtime
            .subscriptions
            .values()
            .map(|sub| sub.queue.len())
            .sum::<usize>();
        total
    }

    pub fn validate_request(&self, command: &ServiceCommand) -> Result<(), ServiceClusterError> {
        self.ensure_namespace()?;
        let node = &self
            .assignments
            .first()
            .ok_or(ServiceClusterError::UnknownNode)?
            .route
            .owner;
        Ok(self.service(node, 0, false)?.validate_request(command)?)
    }

    pub fn preflight_registry_growth(
        &self,
        command: &ServiceCommand,
    ) -> Result<usize, ServiceClusterError> {
        self.validate_request(command)?;
        let scope = service_operation_scope(&command.operation)
            .ok_or(ServiceClusterError::UnsupportedScope)?;
        if !self.supports_operation(&command.operation) {
            return Err(ServiceClusterError::UnsupportedScope);
        }
        let mut growth = 0usize;
        for (p, mut group, _) in self.groups(command, scope)? {
            let route = &self.assignments[p as usize].route;
            let owner = self.service(&route.owner, p, false)?;
            if let Some(identity) = &command.identity {
                if is_mutating_operation(&command.operation) {
                    group.identity = Some(data_identity(identity, p));
                }
            }
            let local = owner.preflight_registry_growth(&group)?;
            let used = owner.resource_usage();
            growth = growth
                .saturating_add(local.saturating_mul(2))
                .saturating_add(used.values)
                .saturating_add(used.outcomes)
                .saturating_add(local)
                .saturating_add(256);
        }
        if is_mutating_operation(&command.operation) {
            let identity = command
                .identity
                .as_ref()
                .ok_or(MapServiceError::InvalidArgument)?;
            let p = self.coordinator_for(identity);
            let route = &self.assignments[p as usize].route;
            let journal = self.service(&route.owner, p, true)?;
            let used = journal.resource_usage();
            growth = growth
                .saturating_add(used.outcomes)
                .saturating_add(
                    command
                        .test_transport_size(self.bounds.max_frame_bytes)?
                        .saturating_mul(4),
                )
                .saturating_add(1_024);
        }
        let transfer_room = self.runtime.limits.max_transfer_bytes.saturating_sub(
            self.runtime
                .transfers
                .values()
                .map(transfer::OnlineTransferState::owned_bytes)
                .sum::<usize>(),
        );
        if !self.runtime.transfers.is_empty() {
            growth = growth.saturating_add(transfer_room);
        }
        let subscription_room = self.runtime.limits.max_subscription_bytes.saturating_sub(
            self.runtime
                .subscriptions
                .values()
                .map(subscriptions::SubscriptionState::owned_bytes)
                .sum::<usize>(),
        );
        if !self.runtime.subscriptions.is_empty() {
            growth = growth.saturating_add(subscription_room);
        }
        if growth == usize::MAX {
            return Err(ServiceClusterError::InvalidBound);
        }
        Ok(growth)
    }

    pub fn partition_replica_bound(&self, p: u32) -> Result<usize, ServiceClusterError> {
        self.ensure_namespace()?;
        let route = &self
            .assignments
            .get(p as usize)
            .ok_or(ServiceClusterError::InvalidBound)?
            .route;
        let data = self
            .service(&route.owner, p, false)?
            .replica_transport_size(self.bounds.max_frame_bytes)?;
        let journal = self
            .service(&route.owner, p, true)?
            .replica_transport_size(self.bounds.max_frame_bytes)?;
        data.checked_add(journal)
            .and_then(|bytes| bytes.checked_add(1_024))
            .ok_or(ServiceClusterError::InvalidBound)
    }
    pub fn command_replica_bound(
        &self,
        command: &ServiceCommand,
    ) -> Result<usize, ServiceClusterError> {
        self.validate_request(command)?;
        let scope = service_operation_scope(&command.operation)
            .ok_or(ServiceClusterError::UnsupportedScope)?;
        let request_bytes = command.test_transport_size(self.bounds.max_frame_bytes)?;
        let mut bytes = request_bytes.saturating_add(1_024);
        let mut result_bound = 64usize;
        for (p, mut group, _) in self.groups(command, scope)? {
            if let Some(id) = &command.identity {
                group.identity = Some(data_identity(id, p));
            }
            let owner = &self.assignments[p as usize].route.owner;
            let service = self.service(owner, p, false)?;
            let base = service.replica_transport_size(self.bounds.max_frame_bytes)?;
            let result = if is_mutating_operation(&command.operation) {
                match service.recover_command(
                    &group,
                    group
                        .identity
                        .as_ref()
                        .ok_or(MapServiceError::InvalidArgument)?,
                )? {
                    ServiceRecovery::Completed(ref result) => service_result_bytes(result),
                    ServiceRecovery::Pending => 64,
                    ServiceRecovery::Unknown => service.command_result_bound(&group)?,
                }
            } else {
                service.encoded_response_bound(&group)?.saturating_add(64)
            };
            result_bound = result_bound.saturating_add(result.saturating_sub(64));
            let prediction = base
                .saturating_add(
                    group
                        .test_transport_size(self.bounds.max_frame_bytes)?
                        .saturating_mul(2),
                )
                .saturating_add(result.saturating_mul(16))
                .saturating_add(2_048);
            if prediction > self.bounds.max_frame_bytes {
                return Err(MapServiceError::ResourceBudget("replica_payload").into());
            }
            bytes = bytes.saturating_add(prediction).saturating_add(128);
        }
        if let Some(id) = &command.identity {
            if is_mutating_operation(&command.operation) {
                let p = self.coordinator_for(id);
                let route = &self.assignments[p as usize].route;
                let base = self
                    .service(&route.owner, p, true)?
                    .replica_transport_size(self.bounds.max_frame_bytes)?;
                let prediction = base
                    .saturating_add(result_bound.saturating_mul(16))
                    .saturating_add(2_048);
                if prediction > self.bounds.max_frame_bytes {
                    return Err(MapServiceError::ResourceBudget("replica_payload").into());
                }
                bytes = bytes.saturating_add(prediction).saturating_add(128);
            }
        }
        if bytes == usize::MAX {
            return Err(ServiceClusterError::InvalidBound);
        }
        Ok(bytes)
    }

    pub fn pending_identities(&self) -> Vec<MutationIdentity> {
        self.nodes
            .values()
            .flat_map(|node| node.journal.iter())
            .flat_map(UnifiedIMapService::pending_identities)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
    pub fn reserve_operation(
        &mut self,
        identity: MutationIdentity,
        command: &ServiceCommand,
    ) -> Result<(), ServiceClusterError> {
        self.validate_request(command)?;
        if command.identity.as_ref() != Some(&identity)
            || !is_mutating_operation(&command.operation)
        {
            return Err(MapServiceError::InvalidArgument.into());
        }
        self.reserve_transfer_call(command)?;
        let p = self.coordinator_for(&identity);
        let route = self.assignments[p as usize].route.clone();
        self.require_live(&route.owner)?;
        let mut bytes = 64usize;
        for (p, mut group, _) in self.groups(
            command,
            service_operation_scope(&command.operation)
                .ok_or(ServiceClusterError::UnsupportedScope)?,
        )? {
            group.identity = Some(data_identity(&identity, p));
            let owner = &self.assignments[p as usize].route.owner;
            bytes = bytes.saturating_add(
                self.service(owner, p, false)?
                    .command_result_bound(&group)?
                    .saturating_sub(64),
            );
        }
        self.service(&route.owner, p, true)?
            .admit_external_operation(identity, request_digest(command), bytes)?;
        self.assignments[p as usize].required_journal = self
            .service(&route.owner, p, true)?
            .journal_replica_stamp()?;
        if !self.replicate(p, true, ServiceLinkFaults::default())? {
            return Err(ServiceClusterError::ReplicaNotProved);
        }
        self.capture_online_changes();
        Ok(())
    }
    pub fn reserve_operation_guarded(
        &mut self,
        entry: &str,
        command: &ServiceCommand,
        guard: Arc<dyn ServiceMessageGuard>,
    ) -> Result<(), ServiceClusterError> {
        if command.cancelled || command.now >= command.deadline {
            return Err(MapServiceError::InvalidArgument.into());
        }
        self.validate_request(command)?;
        self.require_live(entry)?;
        if self.node_is_draining(entry) {
            return Err(ServiceClusterError::NodeUnavailable);
        }
        let identity = command
            .identity
            .clone()
            .ok_or(MapServiceError::InvalidArgument)?;
        let previous = self.runtime.invocation.replace(Invocation {
            digest: command.test_request_digest(),
            identity: Some(identity.clone()),
            now: command.now,
            deadline: command.deadline,
        });
        let result = self.with_message_guard(guard, |cluster| {
            let p = cluster.coordinator_for(&identity);
            let target = cluster.assignments[p as usize].route.owner.clone();
            let payload = command.test_transport_payload(cluster.bounds.max_frame_bytes)?;
            cluster.authorize_message(
                ServiceMessageKind::Request,
                entry,
                &target,
                p,
                None,
                &payload,
            )?;
            cluster.reserve_transfer_call(command)?;
            cluster.reserve_operation(identity, command)
        });
        self.runtime.invocation = previous;
        result
    }
    pub fn recover_invocation(
        &self,
        command: &ServiceCommand,
    ) -> Result<ServiceRecovery, ServiceClusterError> {
        self.validate_request(command)?;
        let identity = command
            .identity
            .as_ref()
            .ok_or(MapServiceError::InvalidArgument)?;
        let p = self.coordinator_for(identity);
        let owner = &self.assignments[p as usize].route.owner;
        self.require_live(owner)?;
        Ok(self
            .service(owner, p, true)?
            .recover_external_operation(identity, request_digest(command))?)
    }
    pub fn cancel_pending_identity(
        &mut self,
        identity: &MutationIdentity,
    ) -> Result<bool, ServiceClusterError> {
        if self.runtime.namespace_fenced {
            let mut changed = false;
            for service in self.all_services() {
                changed |= service.cancel_admitted_identity(identity)?;
            }
            return Ok(changed);
        }
        let p = self.coordinator_for(identity);
        let route = self.assignments[p as usize].route.clone();
        self.require_live(&route.owner)?;
        let journal = self.service(&route.owner, p, true)?;
        if !journal.cancel_admitted_identity(identity)? {
            return Ok(false);
        }
        if let Ok(stamp) = journal.journal_replica_stamp() {
            self.assignments[p as usize].required_journal = stamp;
        }
        if !self.replicate(p, true, ServiceLinkFaults::default())? {
            return Err(ServiceClusterError::ReplicaNotProved);
        }
        self.capture_online_changes();
        Ok(true)
    }

    pub fn wake_listener_waiters_deferred(&mut self) -> Vec<Waker> {
        let mut wakes = self.take_deferred_wakes();
        for service in self.all_services() {
            wakes.extend(service.wake_listener_waiters_deferred());
        }
        for subscription in self.runtime.subscriptions.values_mut() {
            if let Some(waker) = subscription.waker.take() {
                wakes.push(waker);
            }
        }
        wakes
    }
    pub fn take_deferred_wakes(&mut self) -> Vec<Waker> {
        std::mem::take(&mut self.runtime.deferred_wakes)
    }
    pub fn fence_namespace_deferred(&mut self) -> Vec<Waker> {
        self.runtime.namespace_fenced = true;
        let mut wakes = self.take_deferred_wakes();
        for service in self.all_services() {
            wakes.extend(service.fence_namespace_deferred());
        }
        for sub in self.runtime.subscriptions.values_mut() {
            if let Some(waker) = sub.waker.take() {
                wakes.push(waker);
            }
        }
        wakes
    }
    pub fn reclaim_namespace_step(
        &mut self,
        maximum: usize,
    ) -> Result<NamespaceReclaimProgress, ServiceClusterError> {
        if !self.runtime.namespace_fenced
            || maximum == 0
            || maximum > self.collection.max_scan_items
        {
            return Err(ServiceClusterError::InvalidBound);
        }
        let mut reclaimed = 0;
        while reclaimed < maximum {
            let Some(id) = self.runtime.subscriptions.keys().next().copied() else {
                break;
            };
            self.unsubscribe(id);
            reclaimed += 1;
        }
        while reclaimed < maximum {
            let Some(id) = self.runtime.transfers.keys().next().copied() else {
                break;
            };
            self.runtime.transfers.remove(&id);
            reclaimed += 1;
        }
        for service in self.all_services() {
            if reclaimed == maximum {
                break;
            }
            let progress = service.reclaim_namespace_step(maximum - reclaimed)?;
            reclaimed += progress.reclaimed;
        }
        let owners = self.aggregate_namespace_owners();
        Ok(NamespaceReclaimProgress {
            reclaimed,
            remaining: owners.cleanable_records(),
            blocked_owners: owners.blocked_owners(),
        })
    }
    pub fn empty_successor(&self, next_generation: u64) -> Result<Self, ServiceClusterError> {
        if !self.runtime.namespace_fenced
            || self.aggregate_namespace_owners().total() != 0
            || self.generation.checked_add(1) != Some(next_generation)
        {
            return Err(MapServiceError::ActiveOwners.into());
        }
        let ids = self.node_ids();
        let borrowed = ids.iter().map(String::as_str).collect::<Vec<_>>();
        let mut next = Self::new(
            &borrowed,
            self.bounds,
            self.collection,
            self.service_limits,
            next_generation,
        )?;
        next.runtime.limits = self.runtime.limits;
        for (id, node) in &self.nodes {
            let target = next.nodes.get_mut(id).expect("same roster");
            target.generation = node.generation;
            target.live = node.live;
        }
        Ok(next)
    }

    pub fn page_source(
        &self,
        projection: ScanProjection,
        now: u64,
    ) -> Result<ServicePageSource, ServiceClusterError> {
        let _ = (projection, now);
        Err(ServiceClusterError::UnsupportedScope)
    }
    pub fn listener_source(
        &self,
        _options: ServiceListenerOptions,
    ) -> Result<ServiceListenerSource, ServiceClusterError> {
        Err(ServiceClusterError::UnsupportedScope)
    }
    pub fn resync_listener(
        &self,
        _id: u64,
        _now: u64,
        _items: usize,
        _bytes: usize,
    ) -> Result<ListenerSnapshot, ServiceClusterError> {
        Err(ServiceClusterError::UnsupportedScope)
    }

    pub fn begin_node_drain(&mut self, node: &str) -> Result<(), ServiceClusterError> {
        self.require_live(node)?;
        self.runtime.draining.insert(node.into());
        Ok(())
    }
    pub fn reclaim_drained_node_step(
        &mut self,
        node: &str,
        maximum: usize,
    ) -> Result<NamespaceReclaimProgress, ServiceClusterError> {
        if !self.runtime.draining.contains(node)
            || maximum == 0
            || maximum > self.collection.max_scan_items
        {
            return Err(ServiceClusterError::InvalidBound);
        }
        if self
            .assignments
            .iter()
            .any(|a| a.route.owner == node || a.route.backup == node)
        {
            return Err(MapServiceError::ActiveOwners.into());
        }
        let services = {
            let node = self
                .nodes
                .get(node)
                .ok_or(ServiceClusterError::UnknownNode)?;
            node.data
                .iter()
                .chain(&node.journal)
                .cloned()
                .collect::<Vec<_>>()
        };
        let mut reclaimed = 0;
        let mut owners = NamespaceOwners::default();
        for service in services {
            self.runtime
                .deferred_wakes
                .extend(service.fence_namespace_deferred());
            if reclaimed < maximum {
                reclaimed += service
                    .reclaim_namespace_step(maximum - reclaimed)?
                    .reclaimed;
            }
            add_owners(&mut owners, service.namespace_owners());
        }
        if owners.total() == 0 {
            self.nodes.get_mut(node).expect("known draining node").live = false;
        }
        Ok(NamespaceReclaimProgress {
            reclaimed,
            remaining: owners.cleanable_records(),
            blocked_owners: owners.blocked_owners(),
        })
    }

    pub fn execute_guarded(
        &mut self,
        entry: &str,
        command: ServiceCommand,
        call: ServiceClusterCall,
        guard: Arc<dyn ServiceMessageGuard>,
    ) -> Result<ServiceClusterExecution, ServiceClusterError> {
        self.validate_request(&command)?;
        let old_invocation = self.runtime.invocation.take();
        self.runtime.invocation = Some(Invocation {
            digest: command.test_request_digest(),
            identity: command.identity.clone(),
            now: command.now,
            deadline: command.deadline,
        });
        let result =
            self.with_message_guard(guard, |cluster| cluster.execute(entry, command, call));
        self.runtime.invocation = old_invocation;
        result
    }
    pub fn with_message_guard<T>(
        &mut self,
        guard: Arc<dyn ServiceMessageGuard>,
        operation: impl FnOnce(&mut Self) -> Result<T, ServiceClusterError>,
    ) -> Result<T, ServiceClusterError> {
        let previous = self.runtime.guard.replace(guard);
        let result = operation(self);
        self.runtime.guard = previous;
        result
    }
    pub(super) fn authorize_message(
        &self,
        kind: ServiceMessageKind,
        source: &str,
        target: &str,
        p: u32,
        stamp: Option<ReplicaStamp>,
        payload: &[u8],
    ) -> Result<(), ServiceClusterError> {
        let Some(guard) = &self.runtime.guard else {
            return Ok(());
        };
        let digest: [u8; 32] = Sha256::digest(payload).into();
        let invocation = self.runtime.invocation.as_ref();
        guard.authorize(&ServiceMessageContext {
            kind,
            source: source.into(),
            target: target.into(),
            source_generation: self.node_generation(source)?,
            target_generation: self.node_generation(target)?,
            partition: p,
            epoch: self.assignments[p as usize].route.epoch,
            request_digest: invocation.map_or(digest, |i| i.digest),
            mutation_identity: invocation.and_then(|i| i.identity.clone()),
            replica_stamp: stamp,
            now: invocation.map_or_else(|| self.logical_time(), |i| i.now),
            deadline: invocation.map_or(u64::MAX, |i| i.deadline),
            payload_digest: digest,
            payload_bytes: payload.len(),
        })
    }
}
