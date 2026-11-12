//! Bounded subscriptions over real owner listener registrations. There is no event replay log.
use super::*;
use crate::value_plane_stream_075::{BatchPoll, BoundedStreamSource};
use std::sync::Mutex;
use std::task::{Context, Poll};

#[derive(Debug, Clone)]
pub struct ClusterSubscriptionOptions {
    pub listener: ServiceListenerOptions,
    pub max_queued_events: usize,
    pub max_queued_bytes: usize,
}
impl Default for ClusterSubscriptionOptions {
    fn default() -> Self {
        Self {
            listener: ServiceListenerOptions::default(),
            max_queued_events: 1_024,
            max_queued_bytes: 1_024 * 1_024,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartitionSubscriptionProgress {
    pub partition: u32,
    pub epoch: u64,
    pub owner_generation: u64,
    pub owner: String,
    pub map_id: u64,
    pub watermark: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartitionSubscriptionGap {
    pub previous: PartitionSubscriptionProgress,
    pub observed: PartitionSubscriptionProgress,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClusterGapReason {
    OwnerChanged,
    OwnerUnavailable,
    SourceGap,
    QueueOverflow,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClusterSubscriptionEvent {
    Entry {
        progress: PartitionSubscriptionProgress,
        event: ServiceEvent,
    },
    Gap {
        partitions: Vec<PartitionSubscriptionGap>,
        reason: ClusterGapReason,
    },
}
impl ClusterSubscriptionEvent {
    pub fn owned_bytes(&self) -> usize {
        match self {
            Self::Entry { progress, event } => 96usize
                .saturating_add(progress.owner.len())
                .saturating_add(event.key.len())
                .saturating_add(event.old_value.as_ref().map_or(0, Vec::len))
                .saturating_add(event.value.as_ref().map_or(0, Vec::len)),
            Self::Gap { partitions, .. } => partitions.iter().fold(64usize, |sum, cut| {
                sum.saturating_add(64)
                    .saturating_add(cut.previous.owner.len())
                    .saturating_add(cut.observed.owner.len())
            }),
        }
    }
}

pub struct ClusterPartitionSnapshot {
    pub progress: PartitionSubscriptionProgress,
    pub snapshot: ListenerSnapshot,
}
pub struct ClusterSubscriptionSnapshot {
    pub cuts: Vec<ClusterPartitionSnapshot>,
    pub generation: u64,
}
impl ClusterSubscriptionSnapshot {
    pub fn close(&mut self) {
        for cut in &mut self.cuts {
            cut.snapshot.close();
        }
        self.cuts.clear();
    }
}

pub(super) struct PartitionListener {
    registration: ListenerRegistration,
    progress: PartitionSubscriptionProgress,
    delivered: PartitionSubscriptionProgress,
    gapped: bool,
    terminal: bool,
}
pub(super) struct SubscriptionState {
    pub options: ClusterSubscriptionOptions,
    pub parts: BTreeMap<u32, PartitionListener>,
    pub queue: VecDeque<ClusterSubscriptionEvent>,
    pub waker: Option<Waker>,
    pub delivery_epoch: u64,
    failed: Option<ServiceClusterError>,
}
impl SubscriptionState {
    pub(super) fn owned_bytes(&self) -> usize {
        self.queue.iter().fold(self.base_bytes(), |sum, event| {
            sum.saturating_add(event.owned_bytes())
        })
    }
    fn base_bytes(&self) -> usize {
        self.parts.values().fold(
            128usize
                .saturating_add(self.options.listener.key_prefix.len())
                .saturating_add(self.options.listener.exact_key.as_ref().map_or(0, Vec::len)),
            |sum, part| {
                sum.saturating_add(96)
                    .saturating_add(part.progress.owner.len())
                    .saturating_add(part.delivered.owner.len())
            },
        )
    }
    fn reserved_bytes(&self) -> usize {
        self.base_bytes()
            .saturating_add(self.options.max_queued_bytes)
    }
    fn queued_bytes(&self) -> usize {
        self.queue
            .iter()
            .map(ClusterSubscriptionEvent::owned_bytes)
            .sum()
    }
}

impl ServiceClusterHarness {
    pub fn subscribe(
        &mut self,
        options: ClusterSubscriptionOptions,
    ) -> Result<u64, ServiceClusterError> {
        self.ensure_namespace()?;
        if options.max_queued_events == 0
            || options.max_queued_events > 4_096
            || options.max_queued_bytes == 0
            || options.max_queued_bytes > self.runtime.limits.max_subscription_bytes
            || self.runtime.subscriptions.len() >= self.runtime.limits.max_subscriptions
        {
            return Err(ServiceClusterError::InvalidBound);
        }
        let partitions = if let Some(key) = &options.listener.exact_key {
            vec![self.partition_for(key)]
        } else {
            (0..self.bounds.partitions).collect()
        };
        let max_name = self.nodes.keys().map(String::len).max().unwrap_or(0);
        let gap_credit = 64usize.saturating_add(partitions.len().saturating_mul(64 + 2 * max_name));
        if options.max_queued_bytes < gap_credit {
            return Err(ServiceClusterError::InvalidBound);
        }
        let id = self.runtime.next_subscription;
        let next = id.checked_add(1).ok_or(ServiceClusterError::InvalidBound)?;
        let mut state = SubscriptionState {
            options,
            parts: BTreeMap::new(),
            queue: VecDeque::new(),
            waker: None,
            delivery_epoch: 1,
            failed: None,
        };
        for p in partitions {
            let listener = self.new_partition_listener(p, &state.options.listener)?;
            state.parts.insert(p, listener);
        }
        let total = self
            .runtime
            .subscriptions
            .values()
            .map(SubscriptionState::reserved_bytes)
            .sum::<usize>()
            .saturating_add(state.reserved_bytes());
        if total > self.runtime.limits.max_subscription_bytes {
            return Err(MapServiceError::ResourceBudget("cluster_subscriptions").into());
        }
        self.runtime.next_subscription = next;
        self.runtime.subscriptions.insert(id, state);
        Ok(id)
    }

    fn new_partition_listener(
        &self,
        p: u32,
        options: &ServiceListenerOptions,
    ) -> Result<PartitionListener, ServiceClusterError> {
        let route = &self.assignments[p as usize].route;
        self.require_live(&route.owner)?;
        let service = self.service(&route.owner, p, false)?;
        let registration = service.register_listener(options.clone())?;
        let (map_id, watermark) = service.listener_watermark(registration.id())?;
        let progress = PartitionSubscriptionProgress {
            partition: p,
            epoch: route.epoch,
            owner_generation: self.node_generation(&route.owner)?,
            owner: route.owner.clone(),
            map_id,
            watermark,
        };
        Ok(PartitionListener {
            registration,
            delivered: progress.clone(),
            progress,
            gapped: false,
            terminal: false,
        })
    }

    pub fn unsubscribe(&mut self, id: u64) -> bool {
        let Some(mut state) = self.runtime.subscriptions.remove(&id) else {
            return false;
        };
        if let Some(waker) = state.waker.take() {
            self.runtime.deferred_wakes.push(waker);
        }
        true
    }
    pub fn subscription_progress(
        &self,
        id: u64,
    ) -> Result<Vec<PartitionSubscriptionProgress>, ServiceClusterError> {
        Ok(self
            .runtime
            .subscriptions
            .get(&id)
            .ok_or(ServiceClusterError::UnknownSubscription)?
            .parts
            .values()
            .map(|part| part.delivered.clone())
            .collect())
    }
    pub fn subscription_delivery_epoch(&self, id: u64) -> Result<u64, ServiceClusterError> {
        let state = self
            .runtime
            .subscriptions
            .get(&id)
            .ok_or(ServiceClusterError::UnknownSubscription)?;
        if let Some(error) = &state.failed {
            return Err(error.clone());
        }
        Ok(state.delivery_epoch)
    }

    pub fn poll_subscription(
        &mut self,
        id: u64,
        maximum: usize,
    ) -> Result<Vec<ClusterSubscriptionEvent>, ServiceClusterError> {
        if self.runtime.namespace_fenced {
            self.unsubscribe(id);
            return Err(MapServiceError::NamespaceDeleted.into());
        }
        if maximum == 0 || maximum > 256 {
            return Err(ServiceClusterError::InvalidBound);
        }
        self.refresh_subscriptions();
        self.pump_subscription(id)?;
        let state = self
            .runtime
            .subscriptions
            .get_mut(&id)
            .ok_or(ServiceClusterError::UnknownSubscription)?;
        if let Some(error) = &state.failed {
            return Err(error.clone());
        }
        let events = state
            .queue
            .drain(..maximum.min(state.queue.len()))
            .collect::<Vec<_>>();
        for event in &events {
            if let ClusterSubscriptionEvent::Entry { progress, .. } = event {
                if let Some(part) = state.parts.get_mut(&progress.partition) {
                    part.delivered = progress.clone();
                }
            }
        }
        Ok(events)
    }

    pub fn poll_subscription_with_context(
        &mut self,
        id: u64,
        maximum: usize,
        context: &mut Context<'_>,
    ) -> BatchPoll<ClusterSubscriptionEvent, ServiceClusterError> {
        match self.poll_subscription(id, maximum) {
            Err(error) => Poll::Ready(Err(error)),
            Ok(events) if !events.is_empty() => Poll::Ready(Ok(Some(events))),
            Ok(_) => {
                if let Some(state) = self.runtime.subscriptions.get_mut(&id) {
                    if state.parts.values().all(|part| part.terminal) {
                        return Poll::Ready(Ok(None));
                    }
                    state.waker = Some(context.waker().clone());
                }
                Poll::Pending
            }
        }
    }

    fn pump_subscription(&mut self, id: u64) -> Result<(), ServiceClusterError> {
        let mut state = self
            .runtime
            .subscriptions
            .remove(&id)
            .ok_or(ServiceClusterError::UnknownSubscription)?;
        let result = (|| {
            let keys = state.parts.keys().copied().collect::<Vec<_>>();
            for p in keys {
                let events = {
                    let part = &state.parts[&p];
                    if part.gapped || part.terminal || !self.node_is_live(&part.progress.owner)? {
                        continue;
                    }
                    part.registration.drain()?
                };
                for event in events {
                    accept_owner_event(&mut state, p, event);
                    // An aggregate overflow marks all partitions gapped, including this one.
                    if state.parts[&p].gapped {
                        break;
                    }
                }
            }
            Ok(())
        })();
        self.runtime.subscriptions.insert(id, state);
        result
    }

    pub(super) fn refresh_subscriptions(&mut self) {
        let ids = self
            .runtime
            .subscriptions
            .keys()
            .copied()
            .collect::<Vec<_>>();
        for id in ids {
            let Some(mut state) = self.runtime.subscriptions.remove(&id) else {
                continue;
            };
            for p in state.parts.keys().copied().collect::<Vec<_>>() {
                let route = &self.assignments[p as usize].route;
                let old = &state.parts[&p];
                let changed = old.progress.owner != route.owner
                    || old.progress.epoch != route.epoch
                    || self.node_generation(&route.owner).ok()
                        != Some(old.progress.owner_generation);
                if changed {
                    if let Ok(mut replacement) =
                        self.new_partition_listener(p, &state.options.listener)
                    {
                        let previous = old.delivered.clone();
                        let observed = replacement.progress.clone();
                        replacement.gapped = true;
                        replacement.delivered = previous.clone();
                        state.parts.insert(p, replacement);
                        enqueue(
                            &mut state,
                            ClusterSubscriptionEvent::Gap {
                                partitions: vec![PartitionSubscriptionGap { previous, observed }],
                                reason: ClusterGapReason::OwnerChanged,
                            },
                        );
                    }
                } else if !self.nodes[&route.owner].live && !old.gapped {
                    let gap = PartitionSubscriptionGap {
                        previous: old.delivered.clone(),
                        observed: old.progress.clone(),
                    };
                    state.parts.get_mut(&p).expect("known partition").gapped = true;
                    enqueue(
                        &mut state,
                        ClusterSubscriptionEvent::Gap {
                            partitions: vec![gap],
                            reason: ClusterGapReason::OwnerUnavailable,
                        },
                    );
                }
            }
            if !state.queue.is_empty() {
                if let Some(waker) = state.waker.take() {
                    self.runtime.deferred_wakes.push(waker);
                }
            }
            self.runtime.subscriptions.insert(id, state);
        }
    }

    pub(super) fn signal_subscriptions(&mut self) {
        self.refresh_subscriptions();
        for state in self.runtime.subscriptions.values_mut() {
            if let Some(waker) = state.waker.take() {
                self.runtime.deferred_wakes.push(waker);
            }
        }
    }

    pub fn resync_subscription(
        &mut self,
        id: u64,
        now: u64,
        maximum_items: usize,
        maximum_bytes: usize,
    ) -> Result<ClusterSubscriptionSnapshot, ServiceClusterError> {
        self.ensure_namespace()?;
        if maximum_items == 0
            || maximum_bytes == 0
            || maximum_bytes > self.runtime.limits.max_subscription_bytes
        {
            return Err(ServiceClusterError::InvalidBound);
        }
        self.refresh_subscriptions();
        let state = self
            .runtime
            .subscriptions
            .get(&id)
            .ok_or(ServiceClusterError::UnknownSubscription)?;
        let next_epoch = state
            .delivery_epoch
            .checked_add(1)
            .ok_or(ServiceClusterError::InvalidBound)?;
        let mut plans = Vec::new();
        let mut items = 0usize;
        let mut bytes = 0usize;
        for (p, part) in &state.parts {
            self.require_live(&part.progress.owner)?;
            let service = self.service(&part.progress.owner, *p, false)?;
            let cost = service.preflight_listener_resync(
                part.registration.id(),
                now,
                maximum_items,
                maximum_bytes,
            )?;
            items = items
                .checked_add(cost.0)
                .ok_or(ServiceClusterError::InvalidBound)?;
            bytes = bytes
                .checked_add(cost.1)
                .ok_or(ServiceClusterError::InvalidBound)?;
            if items > maximum_items || bytes > maximum_bytes {
                return Err(MapServiceError::ResourceBudget("cluster_resync").into());
            }
            plans.push((*p, part.registration.id(), service, part.progress.clone()));
        }
        let mut cuts = Vec::new();
        for (p, listener, service, mut progress) in plans {
            let snapshot = service.resync_listener(
                listener,
                now,
                self.collection.max_entries,
                maximum_bytes.min(self.service_limits.resync_bytes),
            )?;
            progress.map_id = snapshot.map_id;
            progress.watermark = snapshot.watermark;
            self.assignments[p as usize].required_data = service.replica_stamp()?;
            let _ = self.replicate(p, false, ServiceLinkFaults::default());
            cuts.push(ClusterPartitionSnapshot { progress, snapshot });
        }
        let state = self
            .runtime
            .subscriptions
            .get_mut(&id)
            .expect("held subscription");
        state.queue.clear();
        state.delivery_epoch = next_epoch;
        for cut in &cuts {
            let part = state
                .parts
                .get_mut(&cut.progress.partition)
                .expect("snapshot partition");
            part.progress = cut.progress.clone();
            part.delivered = cut.progress.clone();
            part.gapped = false;
        }
        self.capture_online_changes();
        self.signal_subscriptions();
        Ok(ClusterSubscriptionSnapshot {
            cuts,
            generation: self.generation,
        })
    }
}

fn accept_owner_event(state: &mut SubscriptionState, p: u32, event: ServiceEvent) {
    let next = {
        let part = state.parts.get_mut(&p).expect("subscription partition");
        if part.gapped || part.terminal || event.watermark <= part.progress.watermark {
            return;
        }
        let previous = part.delivered.clone();
        part.progress.watermark = event.watermark;
        if event.kind == "destroyed" {
            part.terminal = true;
        }
        if event.kind == "gap" {
            part.gapped = true;
            ClusterSubscriptionEvent::Gap {
                partitions: vec![PartitionSubscriptionGap {
                    previous,
                    observed: part.progress.clone(),
                }],
                reason: ClusterGapReason::SourceGap,
            }
        } else {
            ClusterSubscriptionEvent::Entry {
                progress: part.progress.clone(),
                event,
            }
        }
    };
    enqueue(state, next);
}

fn enqueue(state: &mut SubscriptionState, event: ClusterSubscriptionEvent) {
    if state.failed.is_some() {
        return;
    }
    if let ClusterSubscriptionEvent::Gap { reason, .. } = event {
        force_gap(state, reason);
        return;
    }
    if state.queue.len() >= state.options.max_queued_events
        || state.queued_bytes().saturating_add(event.owned_bytes()) > state.options.max_queued_bytes
    {
        force_gap(state, ClusterGapReason::QueueOverflow);
    } else {
        state.queue.push_back(event);
    }
}

fn force_gap(state: &mut SubscriptionState, reason: ClusterGapReason) {
    let Some(epoch) = state.delivery_epoch.checked_add(1) else {
        state.failed = Some(ServiceClusterError::InvalidBound);
        return;
    };
    state.delivery_epoch = epoch;
    state.queue.clear();
    let mut cuts = Vec::new();
    for part in state.parts.values_mut() {
        part.gapped = true;
        cuts.push(PartitionSubscriptionGap {
            previous: part.delivered.clone(),
            observed: part.progress.clone(),
        });
    }
    state.queue.push_back(ClusterSubscriptionEvent::Gap {
        partitions: cuts,
        reason,
    });
}

/// The runtime drains the harness's deferred wakes after every mutation, outside its mutex.
pub struct ClusterListenerSource {
    cluster: Arc<Mutex<ServiceClusterHarness>>,
    id: u64,
    epoch: u64,
    reset: bool,
    closed: bool,
}
impl ClusterListenerSource {
    pub fn new(
        cluster: Arc<Mutex<ServiceClusterHarness>>,
        options: ClusterSubscriptionOptions,
    ) -> Result<Self, ServiceClusterError> {
        let (id, epoch) = {
            let mut state = cluster.lock().expect("cluster poisoned");
            let credit = options.max_queued_bytes;
            if state
                .runtime
                .subscription_streams
                .values()
                .sum::<usize>()
                .saturating_add(credit)
                > state.runtime.limits.max_subscription_bytes
            {
                return Err(MapServiceError::ResourceBudget("cluster_listener_streams").into());
            }
            let id = state.subscribe(options)?;
            state.runtime.subscription_streams.insert(id, credit);
            (id, state.subscription_delivery_epoch(id)?)
        };
        Ok(Self {
            cluster,
            id,
            epoch,
            reset: false,
            closed: false,
        })
    }
    pub const fn subscription_id(&self) -> u64 {
        self.id
    }
}
impl BoundedStreamSource for ClusterListenerSource {
    type Item = ClusterSubscriptionEvent;
    type Error = ServiceClusterError;
    fn poll_batch(
        &mut self,
        context: &mut Context<'_>,
        maximum: usize,
    ) -> BatchPoll<Self::Item, Self::Error> {
        if self.closed {
            return Poll::Ready(Ok(None));
        }
        let mut cluster = self.cluster.lock().expect("cluster poisoned");
        let result = cluster.poll_subscription_with_context(self.id, maximum, context);
        if let Ok(epoch) = cluster.subscription_delivery_epoch(self.id) {
            self.epoch = epoch;
        }
        let wakes = cluster.take_deferred_wakes();
        drop(cluster);
        for wake in wakes {
            wake.wake();
        }
        result
    }
    fn validate_buffered(&mut self) -> Result<(), Self::Error> {
        if self.closed {
            return Ok(());
        }
        let cluster = self.cluster.lock().expect("cluster poisoned");
        cluster.ensure_namespace()?;
        let epoch = cluster.subscription_delivery_epoch(self.id)?;
        if epoch != self.epoch {
            self.epoch = epoch;
            self.reset = true;
        }
        Ok(())
    }
    fn take_buffer_reset(&mut self) -> bool {
        std::mem::take(&mut self.reset)
    }
    fn close(&mut self) {
        if !self.closed {
            self.closed = true;
            let mut cluster = self.cluster.lock().expect("cluster poisoned");
            cluster.unsubscribe(self.id);
            state_remove_stream(&mut cluster, self.id);
            let wakes = cluster.take_deferred_wakes();
            drop(cluster);
            for wake in wakes {
                wake.wake();
            }
        }
    }
}
impl Drop for ClusterListenerSource {
    fn drop(&mut self) {
        self.close();
    }
}
fn state_remove_stream(cluster: &mut ServiceClusterHarness, id: u64) {
    cluster.runtime.subscription_streams.remove(&id);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_queue_duplicates_and_epoch_overflow_never_invent_continuity() {
        let mut cluster = ServiceClusterHarness::new(
            &["a", "b"],
            ServiceClusterBounds {
                partitions: 1,
                ..ServiceClusterBounds::default()
            },
            CollectionBounds::default(),
            ServiceLimits::default(),
            1,
        )
        .unwrap();
        let id = cluster
            .subscribe(ClusterSubscriptionOptions::default())
            .unwrap();
        let mut request = ServiceCommand::point("put", b"a");
        request.identity = Some(MutationIdentity::new("duplicate-events", 1));
        request.value = Some(b"v".to_vec());
        cluster
            .execute("b", request, ServiceClusterCall::default())
            .unwrap();
        let mut state = cluster.runtime.subscriptions.remove(&id).unwrap();
        let actual = state.parts[&0].registration.drain().unwrap().pop().unwrap();
        accept_owner_event(&mut state, 0, actual.clone());
        accept_owner_event(&mut state, 0, actual.clone());
        assert_eq!(state.queue.len(), 1);
        force_gap(&mut state, ClusterGapReason::SourceGap);
        let mut next = actual;
        next.watermark += 1;
        accept_owner_event(&mut state, 0, next);
        assert!(matches!(
            state.queue.front(),
            Some(ClusterSubscriptionEvent::Gap { .. })
        ));
        assert_eq!(state.queue.len(), 1);
        state.delivery_epoch = u64::MAX;
        let before = state.queue.clone();
        force_gap(&mut state, ClusterGapReason::QueueOverflow);
        assert_eq!(state.queue, before);
        cluster.runtime.subscriptions.insert(id, state);
        assert_eq!(
            cluster.subscription_delivery_epoch(id).unwrap_err(),
            ServiceClusterError::InvalidBound
        );
        cluster.unsubscribe(id);
        assert_eq!(cluster.aggregate_statistics().listeners, 0);
    }
}
