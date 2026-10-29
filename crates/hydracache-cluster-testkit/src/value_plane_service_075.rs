//! One local, bounded command service for the provisional IMap contract.
//! Storage, versions and retained outcomes belong to `ReferenceCollectionMap`.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::sync::{Arc, Mutex, Weak};
use std::task::{Context, Poll, Waker};

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::value_plane_collection_075::{
    BoundedMatch, CollectionBounds, CollectionError, EntryView, ReferenceCollectionMap,
    ReplacementEntry, ScanCursor, ScanProjection, ServiceRetainedOutcome,
};
use crate::value_plane_model_075::{identity_blank, MutationIdentity};
use crate::value_plane_stream_075::{BatchPoll, BoundedStreamSource};

#[path = "value_plane_service_lifecycle_075.rs"]
mod lifecycle;
pub use lifecycle::*;
#[path = "value_plane_service_cluster_hooks_075.rs"]
mod cluster_hooks;
pub use cluster_hooks::*;
#[path = "value_plane_service_replica_delta_075.rs"]
mod replica_delta;
pub use replica_delta::*;
#[path = "value_plane_service_helpers_075.rs"]
mod helpers;
pub use helpers::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceTtl {
    Preserve,
    Eternal,
    ExpireAfter(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VersionToken {
    pub namespace_generation: u64,
    pub map_id: u64,
    pub version: u64,
}

#[derive(Debug, Clone)]
pub struct ServiceCommand {
    pub operation: String,
    pub key: Vec<u8>,
    pub value: Option<Vec<u8>>,
    pub expected: Option<Vec<u8>>,
    pub keys: Vec<Vec<u8>>,
    pub entries: Vec<ReplacementEntry>,
    pub identity: Option<MutationIdentity>,
    pub ttl: ServiceTtl,
    pub now: u64,
    pub deadline: u64,
    pub token: Option<VersionToken>,
    pub cursor: Option<ScanCursor>,
    pub listener_id: u64,
    pub limit: usize,
    pub include_value: bool,
    pub lose_response: bool,
    pub cancelled: bool,
}

impl ServiceCommand {
    pub fn point(operation: impl Into<String>, key: &[u8]) -> Self {
        Self {
            operation: operation.into(),
            key: key.to_vec(),
            value: None,
            expected: None,
            keys: Vec::new(),
            entries: Vec::new(),
            identity: None,
            ttl: ServiceTtl::Eternal,
            now: 0,
            deadline: u64::MAX,
            token: None,
            cursor: None,
            listener_id: 0,
            limit: 256,
            include_value: false,
            lose_response: false,
            cancelled: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceItem {
    pub index: usize,
    pub key: Vec<u8>,
    pub kind: String,
    pub value: Option<Vec<u8>>,
    pub version: Option<u64>,
    pub remaining_ttl: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceResult {
    pub kind: String,
    pub value: Option<Vec<u8>>,
    pub version: Option<u64>,
    pub remaining_ttl: Option<u64>,
    pub items: Vec<ServiceItem>,
    pub number: u64,
    pub flag: bool,
    pub token: Option<VersionToken>,
    pub cursor: Option<ScanCursor>,
    page_revision: Option<u64>,
}

impl ServiceResult {
    pub fn new(kind: &str) -> Self {
        Self {
            kind: kind.into(),
            value: None,
            version: None,
            remaining_ttl: None,
            items: Vec::new(),
            number: 0,
            flag: false,
            token: None,
            cursor: None,
            page_revision: None,
        }
    }

    fn retained_json(&self) -> Value {
        json!({"kind":self.kind,"value":self.value,"version":self.version,"remaining_ttl":self.remaining_ttl,
            "number":self.number,"flag":self.flag,"items":self.items.iter().map(|item|json!({"index":item.index,"key":item.key,"kind":item.kind,"value":item.value,"version":item.version,"remaining_ttl":item.remaining_ttl})).collect::<Vec<_>>(),
            "token":self.token.map(|token|json!([token.namespace_generation,token.map_id,token.version]))})
    }

    fn from_retained(value: &Value) -> Result<Self, MapServiceError> {
        let mut result = Self::new(
            value["kind"]
                .as_str()
                .ok_or(MapServiceError::InvalidArgument)?,
        );
        result.value = bytes(&value["value"])?;
        result.version = value["version"].as_u64();
        result.remaining_ttl = value["remaining_ttl"].as_u64();
        result.number = value["number"].as_u64().unwrap_or(0);
        result.flag = value["flag"].as_bool().unwrap_or(false);
        if let Some(token) = value["token"].as_array() {
            result.token = Some(VersionToken {
                namespace_generation: token[0].as_u64().ok_or(MapServiceError::InvalidArgument)?,
                map_id: token[1].as_u64().ok_or(MapServiceError::InvalidArgument)?,
                version: token[2].as_u64().ok_or(MapServiceError::InvalidArgument)?,
            });
        }
        if let Some(items) = value["items"].as_array() {
            result.items = items
                .iter()
                .map(|item| {
                    Ok(ServiceItem {
                        index: item["index"]
                            .as_u64()
                            .ok_or(MapServiceError::InvalidArgument)?
                            as usize,
                        key: bytes(&item["key"])?.ok_or(MapServiceError::InvalidArgument)?,
                        kind: item["kind"]
                            .as_str()
                            .ok_or(MapServiceError::InvalidArgument)?
                            .into(),
                        value: bytes(&item["value"])?,
                        version: item["version"].as_u64(),
                        remaining_ttl: item["remaining_ttl"].as_u64(),
                    })
                })
                .collect::<Result<_, MapServiceError>>()?;
        }
        Ok(result)
    }
}

fn bytes(value: &Value) -> Result<Option<Vec<u8>>, MapServiceError> {
    if value.is_null() {
        return Ok(None);
    }
    let values = value.as_array().ok_or(MapServiceError::InvalidArgument)?;
    Ok(Some(
        values
            .iter()
            .map(|byte| {
                byte.as_u64()
                    .and_then(|n| u8::try_from(n).ok())
                    .ok_or(MapServiceError::InvalidArgument)
            })
            .collect::<Result<_, _>>()?,
    ))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MapServiceError {
    InvalidArgument,
    UnsupportedOperation,
    UnknownListener,
    ClockRegressed,
    Capacity,
    RetiredIdentity,
    PendingOperation,
    ExpiryBacklog,
    ActiveOwners,
    ResourceBudget(&'static str),
    NamespaceDeleted,
    Model(CollectionError),
}

impl MapServiceError {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::InvalidArgument => "invalid_argument",
            Self::UnsupportedOperation => "unsupported_operation",
            Self::UnknownListener => "unknown_listener",
            Self::ClockRegressed => "clock_regressed",
            Self::Capacity => "capacity",
            Self::RetiredIdentity => "retired_identity",
            Self::PendingOperation => "pending_retirement",
            Self::ExpiryBacklog => "expiry_backlog",
            Self::ActiveOwners => "active_owners",
            Self::ResourceBudget(_) => "resource_budget_exhausted",
            Self::NamespaceDeleted => "namespace_deleted",
            Self::Model(error) => match error {
                CollectionError::InvalidBound(_)
                | CollectionError::InvalidExpiry
                | CollectionError::InvalidMutationIdentity
                | CollectionError::DuplicateKey => "invalid_argument",
                CollectionError::MutationIdentityConflict => "mutation_identity_conflict",
                CollectionError::CursorMismatch => "cursor_mismatch",
                CollectionError::CursorStale { .. } => "scan_cursor_stale",
                CollectionError::MapDestroyed => "map_destroyed",
                CollectionError::VersionExhausted | CollectionError::MapIdentityExhausted => {
                    "capacity"
                }
                CollectionError::BoundExceeded { bound, .. } => match *bound {
                    "key_bytes" => "key_too_large",
                    "value_bytes" => "value_too_large",
                    "response_bytes" => "response_too_large",
                    "retained_mutations" => "idempotency_capacity_exhausted",
                    _ => "request_too_large",
                },
            },
        }
    }
}

impl From<CollectionError> for MapServiceError {
    fn from(error: CollectionError) -> Self {
        Self::Model(error)
    }
}
impl fmt::Display for MapServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.kind())
    }
}
impl std::error::Error for MapServiceError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceEvent {
    pub kind: String,
    pub key: Vec<u8>,
    pub old_value: Option<Vec<u8>>,
    pub value: Option<Vec<u8>>,
    pub version: u64,
    pub watermark: u64,
}

#[derive(Debug, Clone)]
pub struct ServiceListenerOptions {
    pub include_value: bool,
    pub max_value_bytes: usize,
    pub queue_capacity: usize,
    pub key_prefix: Vec<u8>,
    pub exact_key: Option<Vec<u8>>,
    pub kinds: BTreeSet<String>,
}

impl Default for ServiceListenerOptions {
    fn default() -> Self {
        Self {
            include_value: false,
            max_value_bytes: 4 * 1_024 * 1_024,
            queue_capacity: 1_024,
            key_prefix: Vec::new(),
            exact_key: None,
            kinds: BTreeSet::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ServiceStatistics {
    pub live_entries: usize,
    pub value_bytes: usize,
    pub retained_outcomes: usize,
    pub listeners: usize,
    pub queued_events: usize,
    pub inflight: usize,
    pub rejections: u64,
    pub cancellations: u64,
    pub requests: u64,
    pub active_streams: usize,
    pub active_cursors: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceEntry {
    pub key: Vec<u8>,
    pub value: Vec<u8>,
    pub version: u64,
    pub remaining_ttl: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationOutcomeQuery {
    Pending,
    Completed(Box<ServiceResult>),
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceRecovery {
    Unknown,
    Pending,
    Completed(Box<ServiceResult>),
}

struct ListenerState {
    options: ServiceListenerOptions,
    queue: VecDeque<ServiceEvent>,
    gap: bool,
    waker: Option<Waker>,
    delivery_epoch: u64,
}

struct ServiceState {
    map: ReferenceCollectionMap,
    namespace_generation: u64,
    now: u64,
    listeners: BTreeMap<u64, ListenerState>,
    next_listener: u64,
    watermark: u64,
    requests: u64,
    rejections: u64,
    cancellations: u64,
    pending_wakes: Vec<Waker>,
    active_streams: usize,
    active_cursors: usize,
    helper_bytes: usize,
    helper_leases: usize,
    limits: ServiceLimits,
    resync_bytes: usize,
    resync_leases: usize,
    namespace_fenced: bool,
    replica_sequence: u64,
    replica_digest: Option<[u8; 32]>,
}

#[derive(Clone)]
pub struct UnifiedIMapService {
    state: Arc<Mutex<ServiceState>>,
}

pub struct ListenerRegistration {
    service: Weak<Mutex<ServiceState>>,
    id: u64,
}

impl ListenerRegistration {
    pub const fn id(&self) -> u64 {
        self.id
    }
    pub fn drain(&self) -> Result<Vec<ServiceEvent>, MapServiceError> {
        let state = self
            .service
            .upgrade()
            .ok_or(MapServiceError::UnknownListener)?;
        let mut state = state.lock().expect("local IMap state poisoned");
        drain_listener(&mut state, self.id)
    }
    pub fn close(&mut self) {
        if let Some(state) = self.service.upgrade() {
            let wake = state
                .lock()
                .expect("local IMap state poisoned")
                .listeners
                .remove(&self.id)
                .and_then(|listener| listener.waker);
            if let Some(wake) = wake {
                wake.wake();
            }
        }
        self.service = Weak::new();
    }
}

impl Drop for ListenerRegistration {
    fn drop(&mut self) {
        self.close();
    }
}

impl UnifiedIMapService {
    pub fn new(
        bounds: CollectionBounds,
        namespace_generation: u64,
    ) -> Result<Self, MapServiceError> {
        if namespace_generation == 0 {
            return Err(MapServiceError::InvalidArgument);
        }
        Self::from_map(ReferenceCollectionMap::new(bounds)?, namespace_generation)
    }

    /// Transfer an existing collection and its retained outcomes into the service without
    /// copying either owner. Callers continue advancing its logical time explicitly.
    pub fn from_map(
        map: ReferenceCollectionMap,
        namespace_generation: u64,
    ) -> Result<Self, MapServiceError> {
        Self::from_map_with_limits(map, namespace_generation, ServiceLimits::default())
    }

    fn from_map_with_limits(
        mut map: ReferenceCollectionMap,
        namespace_generation: u64,
        limits: ServiceLimits,
    ) -> Result<Self, MapServiceError> {
        if namespace_generation == 0 {
            return Err(MapServiceError::InvalidArgument);
        }
        map.enable_service_mode();
        let service = Self {
            state: Arc::new(Mutex::new(ServiceState {
                map,
                namespace_generation,
                now: 0,
                listeners: BTreeMap::new(),
                next_listener: 1,
                watermark: 0,
                requests: 0,
                rejections: 0,
                cancellations: 0,
                pending_wakes: Vec::new(),
                active_streams: 0,
                helper_bytes: 0,
                helper_leases: 0,
                active_cursors: 0,
                limits,
                resync_bytes: 0,
                resync_leases: 0,
                namespace_fenced: false,
                replica_sequence: 0,
                replica_digest: None,
            })),
        };
        lifecycle::check_usage(&service.state.lock().expect("local IMap state poisoned"))?;
        Ok(service)
    }

    pub fn execute(&self, command: ServiceCommand) -> Result<ServiceResult, MapServiceError> {
        let mut state = self.state.lock().expect("local IMap state poisoned");
        state.requests = state.requests.saturating_add(1);
        let result = execute(&mut state, &command);
        if result.is_err() {
            state.rejections = state.rejections.saturating_add(1);
        }
        let wakes = std::mem::take(&mut state.pending_wakes);
        drop(state);
        for wake in wakes {
            wake.wake();
        }
        result
    }

    /// An admission checkpoint used by async/model tests; no value is changed by reservation.
    pub fn reserve_operation(
        &self,
        identity: MutationIdentity,
        command: &ServiceCommand,
    ) -> Result<(), MapServiceError> {
        let mut state = self.state.lock().expect("local IMap state poisoned");
        lifecycle::require_namespace(&state)?;
        if !mutating(&command.operation) {
            return Err(MapServiceError::InvalidArgument);
        }
        cluster_hooks::require_replication_capacity(&state)?;
        if command
            .identity
            .as_ref()
            .is_some_and(|command_id| command_id != &identity)
        {
            return Err(MapServiceError::InvalidArgument);
        }
        validate(&state, command)?;
        lifecycle::check_identity(&state, &identity)?;
        let result_bytes = lifecycle::preflight_command(&state, command, &identity)?;
        state
            .map
            .service_reserve(identity, command_digest(command), result_bytes)?;
        Ok(())
    }

    pub fn execute_reserved(
        &self,
        command: ServiceCommand,
    ) -> Result<ServiceResult, MapServiceError> {
        let id = command
            .identity
            .as_ref()
            .ok_or(MapServiceError::InvalidArgument)?;
        let mut state = self.state.lock().expect("local IMap state poisoned");
        lifecycle::require_namespace(&state)?;
        lifecycle::check_identity(&state, id)?;
        let original_reservation = state.map.service_pending_result_bytes(id);
        match state
            .map
            .service_lookup(id, Some(command_digest(&command)))?
        {
            ServiceRetainedOutcome::Pending => {
                cluster_hooks::require_replication_capacity(&state)?;
                validate(&state, &command)?;
                if !command.cancelled && command.now < command.deadline {
                    lifecycle::preflight_command(&state, &command, id)?;
                }
                state.map.service_abort(id);
            }
            ServiceRetainedOutcome::Unknown => return Err(MapServiceError::InvalidArgument),
            ServiceRetainedOutcome::Completed(_) => {}
        }
        state.requests = state.requests.saturating_add(1);
        let result = execute(&mut state, &command);
        if result.is_err() {
            if let Some(bytes) = original_reservation {
                state
                    .map
                    .service_reserve(id.clone(), command_digest(&command), bytes)
                    .expect("failed resume keeps its reserved ledger slot");
            }
        }
        if result.is_err() {
            state.rejections = state.rejections.saturating_add(1);
        }
        let wakes = std::mem::take(&mut state.pending_wakes);
        drop(state);
        for wake in wakes {
            wake.wake();
        }
        result
    }

    pub fn cancel_reserved(&self, identity: &MutationIdentity) -> Result<bool, MapServiceError> {
        let mut state = self.state.lock().expect("local IMap state poisoned");
        let pending = matches!(
            state.map.service_lookup(identity, None)?,
            ServiceRetainedOutcome::Pending
        );
        if pending {
            if !state.namespace_fenced {
                cluster_hooks::require_replication_capacity(&state)?;
            }
            state.map.service_abort(identity);
            state.cancellations = state.cancellations.saturating_add(1);
        }
        Ok(pending)
    }

    pub fn snapshot(&self, now: u64) -> Result<Vec<ServiceEntry>, MapServiceError> {
        let mut state = self.state.lock().expect("local IMap state poisoned");
        lifecycle::require_namespace(&state)?;
        advance(&mut state, now)?;
        let snapshot = state
            .map
            .service_snapshot(now)
            .into_iter()
            .map(|(key, view)| ServiceEntry {
                key,
                value: view.value,
                version: view.version,
                remaining_ttl: view.remaining_ttl,
            })
            .collect();
        let wakes = std::mem::take(&mut state.pending_wakes);
        drop(state);
        for wake in wakes {
            wake.wake();
        }
        Ok(snapshot)
    }

    pub fn statistics(&self) -> ServiceStatistics {
        statistics(&self.state.lock().expect("local IMap state poisoned"))
    }

    /// Observation only: does not advance the maintenance cursor or publish expiry events.
    pub fn diagnostic_snapshot(&self, now: u64) -> Result<Vec<ServiceEntry>, MapServiceError> {
        let state = self.state.lock().expect("local IMap state poisoned");
        lifecycle::require_namespace(&state)?;
        if now < state.now {
            return Err(MapServiceError::ClockRegressed);
        }
        Ok(state
            .map
            .service_snapshot(now)
            .into_iter()
            .map(|(key, view)| ServiceEntry {
                key,
                value: view.value,
                version: view.version,
                remaining_ttl: view.remaining_ttl,
            })
            .collect())
    }

    pub fn query_operation_outcome(
        &self,
        identity: &MutationIdentity,
    ) -> Result<OperationOutcomeQuery, MapServiceError> {
        let state = self.state.lock().expect("local IMap state poisoned");
        lifecycle::require_namespace(&state)?;
        Ok(match state.map.service_lookup(identity, None)? {
            ServiceRetainedOutcome::Unknown => OperationOutcomeQuery::Unknown,
            ServiceRetainedOutcome::Pending => OperationOutcomeQuery::Pending,
            ServiceRetainedOutcome::Completed(value) => {
                OperationOutcomeQuery::Completed(Box::new(ServiceResult::from_retained(&value)?))
            }
        })
    }

    pub fn recover_command(
        &self,
        command: &ServiceCommand,
        identity: &MutationIdentity,
    ) -> Result<ServiceRecovery, MapServiceError> {
        let state = self.state.lock().expect("local IMap state poisoned");
        lifecycle::require_namespace(&state)?;
        validate(&state, command)?;
        lifecycle::check_identity(&state, identity)?;
        Ok(
            match state
                .map
                .service_lookup(identity, Some(command_digest(command)))?
            {
                ServiceRetainedOutcome::Unknown => ServiceRecovery::Unknown,
                ServiceRetainedOutcome::Pending => ServiceRecovery::Pending,
                ServiceRetainedOutcome::Completed(value) => {
                    ServiceRecovery::Completed(Box::new(ServiceResult::from_retained(&value)?))
                }
            },
        )
    }

    pub fn revision(&self) -> u64 {
        self.state
            .lock()
            .expect("local IMap state poisoned")
            .map
            .revision()
    }
    pub fn logical_time(&self) -> u64 {
        self.state.lock().expect("local IMap state poisoned").now
    }

    fn open_stream(&self, cursor: bool) -> Result<(), MapServiceError> {
        let mut state = self.state.lock().expect("local IMap state poisoned");
        lifecycle::require_namespace(&state)?;
        check_stream_admission(&state)?;
        state.active_streams += 1;
        if cursor {
            state.active_cursors += 1;
        }
        Ok(())
    }
    fn close_stream(&self, cursor: bool) {
        let mut state = self.state.lock().expect("local IMap state poisoned");
        state.active_streams = state.active_streams.saturating_sub(1);
        if cursor {
            state.active_cursors = state.active_cursors.saturating_sub(1);
        }
    }

    pub fn register_listener(
        &self,
        options: ServiceListenerOptions,
    ) -> Result<ListenerRegistration, MapServiceError> {
        let mut state = self.state.lock().expect("local IMap state poisoned");
        lifecycle::require_namespace(&state)?;
        let id = register(&mut state, options, 0)?;
        Ok(ListenerRegistration {
            service: Arc::downgrade(&self.state),
            id,
        })
    }

    pub fn drain_listener(&self, id: u64) -> Result<Vec<ServiceEvent>, MapServiceError> {
        drain_listener(
            &mut self.state.lock().expect("local IMap state poisoned"),
            id,
        )
    }

    pub fn remove_listener(&self, id: u64) -> bool {
        let listener = self
            .state
            .lock()
            .expect("local IMap state poisoned")
            .listeners
            .remove(&id);
        let removed = listener.is_some();
        if let Some(wake) = listener.and_then(|listener| listener.waker) {
            wake.wake();
        }
        removed
    }

    pub fn signal_listener_gap(&self, id: u64) -> Result<(), MapServiceError> {
        let mut state = self.state.lock().expect("local IMap state poisoned");
        lifecycle::require_namespace(&state)?;
        let epoch = state
            .listeners
            .get(&id)
            .ok_or(MapServiceError::UnknownListener)?
            .delivery_epoch
            .checked_add(1)
            .ok_or(MapServiceError::Capacity)?;
        let watermark = state
            .watermark
            .checked_add(1)
            .ok_or(MapServiceError::Capacity)?;
        cluster_hooks::require_replication_capacity(&state)?;
        let version = state.map.revision();
        state.watermark = watermark;
        let listener = state
            .listeners
            .get_mut(&id)
            .ok_or(MapServiceError::UnknownListener)?;
        listener.delivery_epoch = epoch;
        listener.queue.clear();
        listener.gap = true;
        listener.queue.push_back(ServiceEvent {
            kind: "gap".into(),
            key: Vec::new(),
            old_value: None,
            value: None,
            version,
            watermark,
        });
        let wake = listener.waker.take();
        drop(state);
        if let Some(wake) = wake {
            wake.wake();
        }
        Ok(())
    }

    pub fn repair_listener(&self, id: u64) -> Result<(), MapServiceError> {
        let mut state = self.state.lock().expect("local IMap state poisoned");
        lifecycle::require_namespace(&state)?;
        let listener = state
            .listeners
            .get_mut(&id)
            .ok_or(MapServiceError::UnknownListener)?;
        listener.delivery_epoch = listener
            .delivery_epoch
            .checked_add(1)
            .ok_or(MapServiceError::Capacity)?;
        listener.queue.clear();
        listener.gap = false;
        Ok(())
    }

    pub fn poll_listener(
        &self,
        id: u64,
        max_items: usize,
        cx: &mut Context<'_>,
    ) -> Poll<Result<Option<Vec<ServiceEvent>>, MapServiceError>> {
        self.poll_listener_epoch(id, max_items, cx).1
    }

    fn poll_listener_epoch(
        &self,
        id: u64,
        max_items: usize,
        cx: &mut Context<'_>,
    ) -> EpochListenerPoll {
        if max_items == 0 {
            return (None, Poll::Ready(Err(MapServiceError::InvalidArgument)));
        }
        let mut state = self.state.lock().expect("local IMap state poisoned");
        if let Err(error) = lifecycle::require_namespace(&state) {
            return (None, Poll::Ready(Err(error)));
        }
        let destroyed = state.map.is_destroyed();
        let max_bytes = lifecycle::stream_reservation(&state);
        let Some(listener) = state.listeners.get_mut(&id) else {
            return (None, Poll::Ready(Err(MapServiceError::UnknownListener)));
        };
        let epoch = Some(listener.delivery_epoch);
        if !listener.queue.is_empty() {
            let mut bytes = 0usize;
            let mut count = 0usize;
            for event in listener.queue.iter().take(max_items) {
                let cost = lifecycle::event_bytes(event);
                if cost > max_bytes && count == 0 {
                    return (
                        epoch,
                        Poll::Ready(Err(MapServiceError::ResourceBudget("streams"))),
                    );
                }
                if bytes.saturating_add(cost) > max_bytes {
                    break;
                }
                bytes += cost;
                count += 1;
            }
            return (
                epoch,
                Poll::Ready(Ok(Some(listener.queue.drain(..count).collect()))),
            );
        }
        if destroyed {
            return (epoch, Poll::Ready(Ok(None)));
        }
        listener.waker = Some(cx.waker().clone());
        (epoch, Poll::Pending)
    }
}

type EpochListenerPoll = (Option<u64>, BatchPoll<ServiceEvent, MapServiceError>);

fn check_stream_admission(state: &ServiceState) -> Result<(), MapServiceError> {
    if state.active_streams >= state.map.service_bounds().max_entries {
        return Err(MapServiceError::Capacity);
    }
    let stream_bytes = lifecycle::stream_reservation(state);
    let total = state
        .active_streams
        .checked_add(1)
        .and_then(|count| count.checked_mul(stream_bytes))
        .and_then(|bytes| bytes.checked_add(state.helper_bytes))
        .ok_or(MapServiceError::ResourceBudget("streams"))?;
    lifecycle::budget("streams", state.limits.stream_bytes, total)
}

fn statistics(state: &ServiceState) -> ServiceStatistics {
    let (live_entries, value_bytes) = state.map.service_counts(state.now);
    ServiceStatistics {
        live_entries,
        value_bytes,
        retained_outcomes: state
            .map
            .retained_mutation_count()
            .saturating_sub(state.map.service_pending_count()),
        listeners: state.listeners.len(),
        queued_events: state
            .listeners
            .values()
            .map(|listener| listener.queue.len())
            .sum(),
        inflight: state.map.service_pending_count(),
        requests: state.requests,
        rejections: state.rejections,
        cancellations: state.cancellations,
        active_streams: state.active_streams,
        active_cursors: state.active_cursors,
    }
}

fn validate(state: &ServiceState, command: &ServiceCommand) -> Result<(), MapServiceError> {
    if !supported_operation(&command.operation) {
        return Err(MapServiceError::UnsupportedOperation);
    }
    let bounds = state.map.service_bounds();
    check_bound("key_bytes", bounds.max_key_bytes, command.key.len())?;
    if command.operation == "entry_listener" {
        if command.limit == 0
            || command.limit > 1_024
            || (command.listener_id != 0 && state.listeners.contains_key(&command.listener_id))
        {
            return Err(MapServiceError::InvalidArgument);
        }
        let id = if command.listener_id == 0 {
            state.next_listener
        } else {
            command.listener_id
        };
        id.checked_add(1).ok_or(MapServiceError::Capacity)?;
    }
    if let Some(value) = &command.value {
        check_bound("value_bytes", bounds.max_value_bytes, value.len())?;
    }
    if let Some(expected) = &command.expected {
        if command.operation == "entry_listener" {
            check_bound("key_bytes", bounds.max_key_bytes, expected.len())?;
        } else {
            check_bound("value_bytes", bounds.max_value_bytes, expected.len())?;
        }
    }
    let count = command.keys.len().max(command.entries.len());
    check_bound("request_entries", 256, count)?;
    let mut seen = BTreeSet::new();
    let mut request_bytes = command
        .key
        .len()
        .saturating_add(command.value.as_ref().map_or(0, Vec::len))
        .saturating_add(command.expected.as_ref().map_or(0, Vec::len));
    for key in &command.keys {
        check_bound("key_bytes", bounds.max_key_bytes, key.len())?;
        if !seen.insert(key) {
            return Err(MapServiceError::InvalidArgument);
        }
        request_bytes = request_bytes.saturating_add(4).saturating_add(key.len());
    }
    seen.clear();
    for entry in &command.entries {
        check_bound("key_bytes", bounds.max_key_bytes, entry.key.len())?;
        check_bound("value_bytes", bounds.max_value_bytes, entry.value.len())?;
        if !seen.insert(&entry.key) {
            return Err(MapServiceError::InvalidArgument);
        }
        request_bytes = request_bytes
            .saturating_add(8)
            .saturating_add(entry.key.len())
            .saturating_add(entry.value.len());
    }
    check_bound("request_bytes", 8 * 1_024 * 1_024, request_bytes)?;
    if matches!(
        command.operation.as_str(),
        "put"
            | "set"
            | "put_if_absent"
            | "replace_if_present"
            | "conditional_replace"
            | "get_and_put"
            | "try_put"
            | "replace_if_version"
    ) && command.value.is_none()
    {
        return Err(MapServiceError::InvalidArgument);
    }
    if matches!(
        command.operation.as_str(),
        "conditional_replace" | "conditional_remove" | "contains_entry"
    ) && command.expected.is_none()
    {
        return Err(MapServiceError::InvalidArgument);
    }
    if matches!(
        command.operation.as_str(),
        "get_all" | "remove_all_detailed"
    ) && command.keys.is_empty()
    {
        return Err(MapServiceError::InvalidArgument);
    }
    if matches!(
        command.operation.as_str(),
        "put_all_detailed" | "replace_all_detailed"
    ) && command.entries.is_empty()
    {
        return Err(MapServiceError::InvalidArgument);
    }
    if matches!(
        command.operation.as_str(),
        "replace_if_version" | "remove_if_version" | "set_ttl_if_version"
    ) && command.token.is_none()
    {
        return Err(MapServiceError::InvalidArgument);
    }
    if matches!(command.ttl, ServiceTtl::ExpireAfter(0)) {
        return Err(MapServiceError::InvalidArgument);
    }
    if matches!(command.operation.as_str(), "set_ttl" | "set_ttl_if_version")
        && command.ttl == ServiceTtl::Preserve
    {
        return Err(MapServiceError::InvalidArgument);
    }
    if let Some(identity) = &command.identity {
        if identity_blank(&identity.client) || identity.client.len() > 128 || identity.sequence == 0
        {
            return Err(MapServiceError::InvalidArgument);
        }
    }
    Ok(())
}

fn supported_operation(operation: &str) -> bool {
    matches!(
        operation,
        "get"
            | "put"
            | "delete"
            | "conditional_replace"
            | "conditional_remove"
            | "contains_key"
            | "put_if_absent"
            | "replace_if_present"
            | "get_and_remove"
            | "get_and_put"
            | "get_all"
            | "put_all_detailed"
            | "remove_all_detailed"
            | "set_ttl"
            | "remaining_ttl"
            | "entry_listener"
            | "size"
            | "is_empty"
            | "contains_value_bounded"
            | "clear_detailed"
            | "destroy"
            | "evict"
            | "evict_all_detailed"
            | "key_set_page"
            | "values_page"
            | "entry_set_page"
            | "get_or_default"
            | "contains_entry"
            | "set"
            | "get_entry_view"
            | "try_put"
            | "try_remove"
            | "replace_all_detailed"
            | "remove_entry_listener"
            | "capability_descriptor"
            | "replace_if_version"
            | "remove_if_version"
            | "set_ttl_if_version"
            | "query_operation_outcome"
            | "local_statistics"
    )
}

fn mutating(operation: &str) -> bool {
    matches!(
        operation,
        "put"
            | "set"
            | "delete"
            | "conditional_replace"
            | "conditional_remove"
            | "put_if_absent"
            | "replace_if_present"
            | "get_and_remove"
            | "get_and_put"
            | "put_all_detailed"
            | "remove_all_detailed"
            | "set_ttl"
            | "clear_detailed"
            | "destroy"
            | "evict"
            | "evict_all_detailed"
            | "try_put"
            | "try_remove"
            | "replace_all_detailed"
            | "replace_if_version"
            | "remove_if_version"
            | "set_ttl_if_version"
    )
}

pub fn is_mutating_operation(operation: &str) -> bool {
    mutating(operation)
}

fn command_digest(command: &ServiceCommand) -> [u8; 32] {
    let value = json!([
        command.operation,
        command.key,
        command.value,
        command.expected,
        command.keys,
        command
            .entries
            .iter()
            .map(|entry| (&entry.key, &entry.value))
            .collect::<Vec<_>>(),
        format!("{:?}", command.ttl),
        command
            .token
            .map(|token| [token.namespace_generation, token.map_id, token.version])
    ]);
    Sha256::digest(value.to_string().as_bytes()).into()
}

fn execute(
    state: &mut ServiceState,
    command: &ServiceCommand,
) -> Result<ServiceResult, MapServiceError> {
    lifecycle::require_namespace(state)?;
    validate(state, command)?;
    if command.operation == "query_operation_outcome" {
        let id = command
            .identity
            .as_ref()
            .ok_or(MapServiceError::InvalidArgument)?;
        return Ok(match state.map.service_lookup(id, None)? {
            ServiceRetainedOutcome::Unknown => ServiceResult::new("unknown"),
            ServiceRetainedOutcome::Pending => ServiceResult::new("pending"),
            ServiceRetainedOutcome::Completed(value) => {
                let mut result = ServiceResult::from_retained(&value)?;
                result.kind = "completed".into();
                result
            }
        });
    }
    let is_mutation = mutating(&command.operation);
    if is_mutation && command.identity.is_none() {
        return Err(MapServiceError::InvalidArgument);
    }
    let digest = command_digest(command);
    if is_mutation {
        if let Some(id) = &command.identity {
            lifecycle::check_identity(state, id)?;
            match state.map.service_lookup(id, Some(digest))? {
                ServiceRetainedOutcome::Completed(value) => {
                    return ServiceResult::from_retained(&value)
                }
                ServiceRetainedOutcome::Pending => return Ok(ServiceResult::new("pending")),
                ServiceRetainedOutcome::Unknown => {}
            }
        }
        if command.cancelled || command.now >= command.deadline {
            if command.cancelled {
                state.cancellations = state.cancellations.saturating_add(1);
            }
            return Ok(ServiceResult::new("not_applied"));
        }
    }
    if state.map.is_destroyed()
        && command.operation != "local_statistics"
        && command.operation != "capability_descriptor"
    {
        return Err(CollectionError::MapDestroyed.into());
    }
    if is_mutation {
        cluster_hooks::require_replication_capacity(state)?;
    }
    let result_bytes = if is_mutation {
        lifecycle::preflight_command(
            state,
            command,
            command
                .identity
                .as_ref()
                .ok_or(MapServiceError::InvalidArgument)?,
        )?
    } else {
        0
    };
    advance(state, command.now)?;
    if matches!(
        command.operation.as_str(),
        "clear_detailed" | "evict_all_detailed" | "destroy"
    ) && state.map.service_expiry_pending(command.now)
    {
        return Err(MapServiceError::ExpiryBacklog);
    }
    if is_mutation {
        let mut keys = command.keys.clone();
        keys.extend(command.entries.iter().map(|entry| entry.key.clone()));
        if keys.is_empty() {
            keys.push(command.key.clone());
        }
        for key in keys {
            if let Some(value) = state.map.service_expire_key(&key, command.now) {
                publish(state, "expired", key, Some(value), None);
            }
        }
    }
    let reserve = is_mutation && command.operation != "destroy";
    if reserve {
        if let Some(id) = &command.identity {
            state
                .map
                .service_reserve(id.clone(), digest, result_bytes)?;
        }
    }
    let result = dispatch(state, command);
    if reserve {
        if let Some(id) = &command.identity {
            match &result {
                Ok(value) => state.map.service_complete(id, value.retained_json()),
                Err(_) => state.map.service_abort(id),
            }
        }
    }
    if command.lose_response && is_mutation && result.is_ok() {
        return Ok(ServiceResult::new("outcome_unknown"));
    }
    result
}

fn advance(state: &mut ServiceState, now: u64) -> Result<(), MapServiceError> {
    if now < state.now {
        return Err(MapServiceError::ClockRegressed);
    }
    if now != state.now || state.map.service_expiry_pending(now) {
        cluster_hooks::require_replication_capacity(state)?;
    }
    if !state.map.is_destroyed() {
        for (key, value) in state
            .map
            .service_expire_step(now, state.limits.expiry_work)?
        {
            publish(state, "expired", key, Some(value), None);
        }
    }
    state.now = now;
    Ok(())
}

fn point_result(
    state: &ServiceState,
    kind: &str,
    view: Option<EntryView>,
    return_value: bool,
) -> ServiceResult {
    let mut result = ServiceResult::new(kind);
    if let Some(view) = view {
        if return_value {
            result.value = Some(view.value);
        }
        result.version = Some(view.version);
        result.remaining_ttl = view.remaining_ttl;
        result.token = Some(VersionToken {
            namespace_generation: state.namespace_generation,
            map_id: state.map.service_map_id(),
            version: view.version,
        });
    }
    result
}

fn expiry(
    command: &ServiceCommand,
    current: Option<&EntryView>,
) -> Result<Option<u64>, MapServiceError> {
    match command.ttl {
        ServiceTtl::Eternal => Ok(None),
        ServiceTtl::ExpireAfter(duration) => command
            .now
            .checked_add(duration)
            .map(Some)
            .ok_or(MapServiceError::InvalidArgument),
        ServiceTtl::Preserve => current
            .ok_or(MapServiceError::InvalidArgument)
            .map(|view| view.remaining_ttl.map(|ttl| command.now + ttl)),
    }
}

fn dispatch(
    state: &mut ServiceState,
    command: &ServiceCommand,
) -> Result<ServiceResult, MapServiceError> {
    let op = command.operation.as_str();
    match op {
        "entry_listener" => {
            let options = ServiceListenerOptions {
                include_value: command.include_value,
                max_value_bytes: state.map.service_bounds().max_value_bytes,
                queue_capacity: command.limit,
                key_prefix: command.key.clone(),
                exact_key: command.expected.clone(),
                kinds: BTreeSet::new(),
            };
            let id = register(state, options, command.listener_id)?;
            let mut result = ServiceResult::new("listener_added");
            result.number = id;
            result.flag = true;
            return Ok(result);
        }
        "remove_entry_listener" => {
            let removed = state.listeners.remove(&command.listener_id);
            let flag = removed.is_some();
            if let Some(wake) = removed.and_then(|listener| listener.waker) {
                state.pending_wakes.push(wake);
            }
            let mut result = ServiceResult::new(if flag { "listener_removed" } else { "absent" });
            result.flag = flag;
            return Ok(result);
        }
        "capability_descriptor" => {
            let mut result = ServiceResult::new("capabilities");
            result.number = crate::value_plane_capability_075::LocalCapabilityDescriptor::unified()
                .operations
                .len() as u64;
            return Ok(result);
        }
        "local_statistics" => {
            let stats = statistics(state);
            let mut result = ServiceResult::new("statistics");
            result.number = stats.live_entries as u64;
            result.flag = state.map.is_destroyed();
            return Ok(result);
        }
        "size" | "is_empty" => {
            let count = state.map.size(command.now)?;
            let mut result = ServiceResult::new(if op == "size" {
                "counted"
            } else if count == 0 {
                "empty"
            } else {
                "non_empty"
            });
            result.number = count as u64;
            result.flag = count == 0;
            return Ok(result);
        }
        "contains_value_bounded" => {
            let value = command
                .value
                .as_ref()
                .ok_or(MapServiceError::InvalidArgument)?;
            let found =
                state
                    .map
                    .contains_value(value, command.cursor, command.limit, command.now)?;
            let mut result = ServiceResult::new(match found {
                BoundedMatch::Present => "present",
                BoundedMatch::Absent => "absent",
                BoundedMatch::Incomplete(_) => "partial",
            });
            if let BoundedMatch::Incomplete(cursor) = found {
                result.cursor = Some(cursor);
            }
            result.flag = found == BoundedMatch::Present;
            return Ok(result);
        }
        "key_set_page" | "values_page" | "entry_set_page" => {
            let projection = match op {
                "key_set_page" => ScanProjection::Keys,
                "values_page" => ScanProjection::Values,
                _ => ScanProjection::Entries,
            };
            let page = state
                .map
                .scan(projection, command.cursor, command.limit, command.now)?;
            let mut result = ServiceResult::new(if page.complete { "page" } else { "partial" });
            result.page_revision = Some(page.snapshot_revision);
            result.flag = page.complete;
            result.cursor = page.next_cursor;
            result.items = page
                .items
                .into_iter()
                .enumerate()
                .map(|(index, item)| ServiceItem {
                    index,
                    key: item.key.unwrap_or_default(),
                    kind: "present".into(),
                    value: item.value,
                    version: None,
                    remaining_ttl: None,
                })
                .collect();
            result.number = result.items.len() as u64;
            return Ok(result);
        }
        "clear_detailed" | "evict_all_detailed" | "destroy" => {
            let before = state.map.service_snapshot(command.now);
            let receipt = match op {
                "clear_detailed" => state.map.clear(command.now)?,
                "evict_all_detailed" => state.map.evict_all(command.now)?,
                _ => state.map.destroy(command.now)?,
            };
            if op == "evict_all_detailed" {
                for (key, view) in before {
                    publish(state, "evicted", key, Some(view.value), None);
                }
            } else if receipt.removed > 0 || op == "destroy" {
                publish(
                    state,
                    if op == "destroy" {
                        "destroyed"
                    } else {
                        "cleared"
                    },
                    Vec::new(),
                    None,
                    None,
                );
            }
            let mut result = ServiceResult::new(match op {
                "clear_detailed" => "cleared",
                "evict_all_detailed" => "evicted",
                _ => "destroyed",
            });
            result.number = receipt.removed as u64;
            return Ok(result);
        }
        "get_all" | "remove_all_detailed" | "put_all_detailed" | "replace_all_detailed" => {
            return bulk(state, command)
        }
        _ => {}
    }
    let current = state.map.service_entry(&command.key, command.now);
    if matches!(
        op,
        "get_and_put"
            | "get_and_remove"
            | "conditional_replace"
            | "conditional_remove"
            | "replace_if_present"
            | "replace_if_version"
            | "put_if_absent"
    ) {
        check_bound(
            "response_bytes",
            state.map.service_bounds().max_response_bytes,
            current.as_ref().map_or(0, |entry| entry.value.len()),
        )?;
    }
    match op {
        "get" | "get_entry_view" => {
            check_bound(
                "response_bytes",
                state.map.service_bounds().max_response_bytes,
                current.as_ref().map_or(0, |entry| entry.value.len()),
            )?;
            return Ok(point_result(
                state,
                if current.is_some() {
                    if op == "get" {
                        "present"
                    } else {
                        "entry_view"
                    }
                } else {
                    "absent"
                },
                current,
                true,
            ));
        }
        "get_or_default" => {
            let value = state.map.get_or_default(
                &command.key,
                command
                    .value
                    .as_deref()
                    .ok_or(MapServiceError::InvalidArgument)?,
                command.now,
            )?;
            let mut result = point_result(
                state,
                if current.is_some() {
                    "present"
                } else {
                    "absent"
                },
                current,
                false,
            );
            result.value = Some(value);
            return Ok(result);
        }
        "contains_key" | "contains_entry" => {
            let present = current.as_ref().is_some_and(|view| {
                op == "contains_key" || Some(&view.value) == command.expected.as_ref()
            });
            let mut result = point_result(
                state,
                if current.is_none() {
                    "absent"
                } else if present {
                    "present"
                } else {
                    "mismatch"
                },
                current,
                false,
            );
            result.flag = present;
            return Ok(result);
        }
        "remaining_ttl" => {
            return Ok(point_result(
                state,
                if current.is_some() {
                    "present"
                } else {
                    "absent"
                },
                current,
                false,
            ))
        }
        _ => {}
    }
    if matches!(
        op,
        "replace_if_version" | "remove_if_version" | "set_ttl_if_version"
    ) {
        let token = command.token.ok_or(MapServiceError::InvalidArgument)?;
        if !current.as_ref().is_some_and(|view| {
            token.namespace_generation == state.namespace_generation
                && token.map_id == state.map.service_map_id()
                && token.version == view.version
        }) {
            return Ok(point_result(
                state,
                if current.is_some() {
                    "mismatch"
                } else {
                    "absent"
                },
                current,
                false,
            ));
        }
    }
    if matches!(op, "conditional_replace" | "conditional_remove")
        && current.as_ref().map(|view| &view.value) != command.expected.as_ref()
    {
        return Ok(point_result(
            state,
            if current.is_some() {
                "mismatch"
            } else {
                "absent"
            },
            current,
            true,
        ));
    }
    if op == "put_if_absent" && current.is_some() {
        return Ok(point_result(state, "present", current, true));
    }
    if matches!(
        op,
        "replace_if_present" | "set_ttl" | "get_and_remove" | "delete" | "try_remove" | "evict"
    ) && current.is_none()
    {
        return Ok(ServiceResult::new(if op == "try_remove" {
            "applied"
        } else {
            "absent"
        }));
    }
    if matches!(
        op,
        "delete"
            | "get_and_remove"
            | "conditional_remove"
            | "try_remove"
            | "evict"
            | "remove_if_version"
    ) {
        state.map.evict(&command.key, command.now)?;
        publish(
            state,
            if op == "evict" { "evicted" } else { "removed" },
            command.key.clone(),
            current.as_ref().map(|view| view.value.clone()),
            None,
        );
        let mut result = ServiceResult::new(if op == "try_remove" {
            "applied"
        } else if op == "evict" {
            "evicted"
        } else {
            "removed"
        });
        result.flag = true;
        result.version = Some(state.map.revision());
        if op == "get_and_remove" || op == "conditional_remove" {
            result.value = current.map(|view| view.value);
        }
        return Ok(result);
    }
    let value = if matches!(op, "set_ttl" | "set_ttl_if_version") {
        current
            .as_ref()
            .ok_or(MapServiceError::InvalidArgument)?
            .value
            .clone()
    } else {
        command
            .value
            .clone()
            .ok_or(MapServiceError::InvalidArgument)?
    };
    let expiry = expiry(command, current.as_ref())?;
    state
        .map
        .set(command.key.clone(), value.clone(), expiry, command.now)?;
    publish(
        state,
        if current.is_some() {
            "updated"
        } else {
            "added"
        },
        command.key.clone(),
        current.as_ref().map(|view| view.value.clone()),
        Some(value),
    );
    let updated = state.map.service_entry(&command.key, command.now);
    let mut result = point_result(
        state,
        if matches!(op, "set" | "try_put") {
            "applied"
        } else if current.is_some() {
            "replaced"
        } else {
            "inserted"
        },
        updated,
        false,
    );
    result.flag = true;
    if matches!(
        op,
        "get_and_put" | "replace_if_present" | "conditional_replace" | "replace_if_version"
    ) {
        result.value = current.map(|view| view.value);
    }
    Ok(result)
}

fn bulk(
    state: &mut ServiceState,
    command: &ServiceCommand,
) -> Result<ServiceResult, MapServiceError> {
    let mut result = ServiceResult::new("bulk");
    let mut response_bytes = 0usize;
    let keys = if command.entries.is_empty() {
        command.keys.clone()
    } else {
        command
            .entries
            .iter()
            .map(|entry| entry.key.clone())
            .collect()
    };
    for key in &keys {
        let value = state.map.service_entry(key, command.now);
        response_bytes = response_bytes.saturating_add(17).saturating_add(key.len());
        if matches!(
            command.operation.as_str(),
            "get_all" | "remove_all_detailed"
        ) {
            response_bytes =
                response_bytes.saturating_add(value.as_ref().map_or(0, |entry| entry.value.len()));
        }
    }
    check_bound(
        "response_bytes",
        state.map.service_bounds().max_response_bytes,
        response_bytes,
    )?;
    if command.operation == "put_all_detailed" {
        state
            .map
            .service_preflight_puts(&command.entries, command.now)?;
        for key in &keys {
            expiry(command, state.map.service_entry(key, command.now).as_ref())?;
        }
    }
    if command.operation == "remove_all_detailed" {
        state.map.service_preflight_removals(&keys, command.now)?;
    }
    if command.operation == "replace_all_detailed" {
        let before = state
            .map
            .service_snapshot(command.now)
            .into_iter()
            .collect::<BTreeMap<_, _>>();
        for receipt in state
            .map
            .replace_all_detailed(&command.entries, command.now)?
        {
            let current = state.map.service_entry(&receipt.key, command.now);
            if receipt.replaced {
                publish_at(
                    state,
                    "updated",
                    receipt.key.clone(),
                    before.get(&receipt.key).map(|view| view.value.clone()),
                    current.as_ref().map(|view| view.value.clone()),
                    receipt
                        .version
                        .expect("replaced item has an applied version"),
                );
            }
            result.items.push(ServiceItem {
                index: receipt.input_index,
                key: receipt.key,
                kind: if receipt.replaced {
                    "replaced"
                } else {
                    "absent"
                }
                .into(),
                value: None,
                version: receipt.version,
                remaining_ttl: current.and_then(|view| view.remaining_ttl),
            });
        }
    } else {
        for (index, key) in keys.into_iter().enumerate() {
            let current = state.map.service_entry(&key, command.now);
            let (kind, value, view) = if command.operation == "get_all" {
                (
                    if current.is_some() {
                        "present"
                    } else {
                        "absent"
                    },
                    current.as_ref().map(|view| view.value.clone()),
                    current,
                )
            } else if command.operation == "remove_all_detailed" {
                if current.is_some() {
                    state.map.evict(&key, command.now)?;
                    publish(
                        state,
                        "removed",
                        key.clone(),
                        current.as_ref().map(|view| view.value.clone()),
                        None,
                    );
                }
                (
                    if current.is_some() {
                        "removed"
                    } else {
                        "absent"
                    },
                    current.as_ref().map(|view| view.value.clone()),
                    None,
                )
            } else {
                let entry = &command.entries[index];
                let expires_at = expiry(command, current.as_ref())?;
                state
                    .map
                    .set(key.clone(), entry.value.clone(), expires_at, command.now)?;
                publish(
                    state,
                    if current.is_some() {
                        "updated"
                    } else {
                        "added"
                    },
                    key.clone(),
                    current.as_ref().map(|view| view.value.clone()),
                    Some(entry.value.clone()),
                );
                (
                    if current.is_some() {
                        "replaced"
                    } else {
                        "inserted"
                    },
                    None,
                    state.map.service_entry(&key, command.now),
                )
            };
            result.items.push(ServiceItem {
                index,
                key,
                kind: kind.into(),
                value,
                version: view.as_ref().map(|view| view.version),
                remaining_ttl: view.and_then(|view| view.remaining_ttl),
            });
        }
    }
    result.number = result.items.len() as u64;
    Ok(result)
}

fn check_bound(bound: &'static str, limit: usize, actual: usize) -> Result<(), MapServiceError> {
    if actual > limit {
        Err(CollectionError::BoundExceeded {
            bound,
            limit,
            actual,
        }
        .into())
    } else {
        Ok(())
    }
}

fn register(
    state: &mut ServiceState,
    options: ServiceListenerOptions,
    requested: u64,
) -> Result<u64, MapServiceError> {
    if state.map.is_destroyed() {
        return Err(CollectionError::MapDestroyed.into());
    }
    let bounds = state.map.service_bounds();
    if options.queue_capacity == 0
        || options.queue_capacity > 1_024
        || options.max_value_bytes
            > bounds
                .max_value_bytes
                .checked_mul(2)
                .ok_or(MapServiceError::Capacity)?
        || options.key_prefix.len() > bounds.max_key_bytes
        || options
            .exact_key
            .as_ref()
            .is_some_and(|key| key.len() > bounds.max_key_bytes)
    {
        return Err(MapServiceError::InvalidArgument);
    }
    if options.kinds.iter().any(|kind| {
        !matches!(
            kind.as_str(),
            "added" | "updated" | "removed" | "expired" | "evicted" | "cleared" | "destroyed"
        )
    }) {
        return Err(MapServiceError::InvalidArgument);
    }
    if state.listeners.len() >= bounds.max_entries {
        return Err(MapServiceError::Capacity);
    }
    lifecycle::budget(
        "listeners",
        state.limits.listener_bytes,
        lifecycle::listener_bytes(state)
            .saturating_add(128)
            .saturating_add(options.key_prefix.len())
            .saturating_add(options.exact_key.as_ref().map_or(0, Vec::len)),
    )?;
    let id = if requested == 0 {
        state.next_listener
    } else {
        requested
    };
    if state.listeners.contains_key(&id) {
        return Err(MapServiceError::InvalidArgument);
    }
    state.next_listener = state
        .next_listener
        .max(id.checked_add(1).ok_or(MapServiceError::Capacity)?);
    state.listeners.insert(
        id,
        ListenerState {
            options,
            queue: VecDeque::new(),
            gap: false,
            waker: None,
            delivery_epoch: 1,
        },
    );
    Ok(id)
}

fn drain_listener(state: &mut ServiceState, id: u64) -> Result<Vec<ServiceEvent>, MapServiceError> {
    lifecycle::require_namespace(state)?;
    let listener = state
        .listeners
        .get_mut(&id)
        .ok_or(MapServiceError::UnknownListener)?;
    let events = listener.queue.drain(..).collect();
    if state.map.is_destroyed() {
        state.listeners.remove(&id);
    }
    Ok(events)
}

fn publish(
    state: &mut ServiceState,
    kind: &str,
    key: Vec<u8>,
    old_value: Option<Vec<u8>>,
    value: Option<Vec<u8>>,
) {
    let version = state.map.revision();
    publish_at(state, kind, key, old_value, value, version);
}

fn publish_at(
    state: &mut ServiceState,
    kind: &str,
    key: Vec<u8>,
    old_value: Option<Vec<u8>>,
    value: Option<Vec<u8>>,
    version: u64,
) {
    state.watermark = state.watermark.saturating_add(1);
    let watermark = state.watermark;
    let mut queued_bytes = lifecycle::listener_bytes(state);
    let stream_maximum = lifecycle::stream_reservation(state);
    for listener in state.listeners.values_mut() {
        let global = matches!(kind, "cleared" | "destroyed");
        if listener.gap
            || (!global
                && (!key.starts_with(&listener.options.key_prefix)
                    || listener
                        .options
                        .exact_key
                        .as_ref()
                        .is_some_and(|exact| exact != &key)))
            || (!listener.options.kinds.is_empty() && !listener.options.kinds.contains(kind))
        {
            continue;
        }
        let oversized = listener.options.include_value
            && old_value
                .as_ref()
                .map_or(0, Vec::len)
                .saturating_add(value.as_ref().map_or(0, Vec::len))
                > listener.options.max_value_bytes;
        let event_bytes =
            64usize
                .saturating_add(key.len())
                .saturating_add(if listener.options.include_value {
                    old_value
                        .as_ref()
                        .map_or(0, Vec::len)
                        .saturating_add(value.as_ref().map_or(0, Vec::len))
                } else {
                    0
                });
        if oversized
            || event_bytes > stream_maximum
            || listener.queue.len() >= listener.options.queue_capacity
            || queued_bytes.saturating_add(event_bytes) > state.limits.listener_bytes
        {
            queued_bytes = queued_bytes.saturating_sub(
                listener
                    .queue
                    .iter()
                    .map(lifecycle::event_bytes)
                    .sum::<usize>(),
            );
            listener.queue.clear();
            listener.gap = true;
            listener.queue.push_back(ServiceEvent {
                kind: "gap".into(),
                key: Vec::new(),
                old_value: None,
                value: None,
                version,
                watermark,
            });
        } else {
            queued_bytes = queued_bytes.saturating_add(event_bytes);
            listener.queue.push_back(ServiceEvent {
                kind: kind.into(),
                key: key.clone(),
                old_value: listener
                    .options
                    .include_value
                    .then(|| old_value.clone())
                    .flatten(),
                value: listener
                    .options
                    .include_value
                    .then(|| value.clone())
                    .flatten(),
                version,
                watermark,
            });
        }
        if let Some(waker) = listener.waker.take() {
            state.pending_wakes.push(waker);
        }
    }
}

/// A live service page source with a revision fence, released on close or drop.
pub struct ServicePageSource {
    service: UnifiedIMapService,
    projection: ScanProjection,
    cursor: Option<ScanCursor>,
    revision: u64,
    now: u64,
    complete: bool,
    closed: bool,
}

impl ServicePageSource {
    pub fn new(
        service: UnifiedIMapService,
        projection: ScanProjection,
        now: u64,
    ) -> Result<Self, MapServiceError> {
        if now < service.logical_time() {
            return Err(MapServiceError::ClockRegressed);
        }
        service.open_stream(true)?;
        let revision = service.revision();
        Ok(Self {
            service,
            projection,
            cursor: None,
            revision,
            now,
            complete: false,
            closed: false,
        })
    }
}

impl BoundedStreamSource for ServicePageSource {
    type Item = ServiceItem;
    type Error = MapServiceError;
    fn poll_batch(
        &mut self,
        _cx: &mut Context<'_>,
        max_items: usize,
    ) -> BatchPoll<Self::Item, Self::Error> {
        if self.closed || self.complete {
            return Poll::Ready(Ok(None));
        }
        if max_items == 0 {
            return Poll::Ready(Err(MapServiceError::InvalidArgument));
        }
        if let Err(error) = self.validate_buffered() {
            return Poll::Ready(Err(error));
        }
        let op = match self.projection {
            ScanProjection::Keys => "key_set_page",
            ScanProjection::Values => "values_page",
            ScanProjection::Entries => "entry_set_page",
        };
        let page_limit = self
            .service
            .state
            .lock()
            .expect("local IMap state poisoned")
            .map
            .service_bounds()
            .max_page_items;
        let mut request = ServiceCommand::point(op, b"");
        request.cursor = self.cursor;
        request.limit = max_items.min(page_limit);
        request.now = self.now.max(self.service.logical_time());
        match self.service.execute(request) {
            Err(error) => Poll::Ready(Err(error)),
            Ok(page) => {
                self.cursor = page.cursor;
                self.complete = page.flag;
                self.revision = page
                    .page_revision
                    .expect("page dispatch carries its snapshot revision");
                if page.items.is_empty() {
                    Poll::Ready(Ok(None))
                } else {
                    Poll::Ready(Ok(Some(page.items)))
                }
            }
        }
    }
    fn validate_buffered(&mut self) -> Result<(), Self::Error> {
        if self.closed {
            return Ok(());
        }
        lifecycle::require_namespace(
            &self
                .service
                .state
                .lock()
                .expect("local IMap state poisoned"),
        )?;
        let current = self.service.revision();
        if current != self.revision {
            Err(CollectionError::CursorStale {
                cursor_revision: self.revision,
                current_revision: current,
            }
            .into())
        } else {
            Ok(())
        }
    }
    fn close(&mut self) {
        if !self.closed {
            self.closed = true;
            self.cursor = None;
            self.service.close_stream(true);
        }
    }
}

impl Drop for ServicePageSource {
    fn drop(&mut self) {
        self.close();
    }
}

/// Listener streams retain their RAII registration while waiting for demand or new events.
pub struct ServiceListenerSource {
    service: UnifiedIMapService,
    registration: Option<ListenerRegistration>,
    closed: bool,
    delivery_epoch: u64,
    reset_buffer: bool,
}

impl ServiceListenerSource {
    pub fn new(
        service: UnifiedIMapService,
        options: ServiceListenerOptions,
    ) -> Result<Self, MapServiceError> {
        let mut state = service.state.lock().expect("local IMap state poisoned");
        lifecycle::require_namespace(&state)?;
        check_stream_admission(&state)?;
        let id = register(&mut state, options, 0)?;
        let delivery_epoch = state.listeners[&id].delivery_epoch;
        state.active_streams += 1;
        drop(state);
        let registration = ListenerRegistration {
            service: Arc::downgrade(&service.state),
            id,
        };
        Ok(Self {
            service,
            registration: Some(registration),
            closed: false,
            delivery_epoch,
            reset_buffer: false,
        })
    }
    pub fn registration_id(&self) -> Option<u64> {
        self.registration.as_ref().map(ListenerRegistration::id)
    }
}

impl BoundedStreamSource for ServiceListenerSource {
    type Item = ServiceEvent;
    type Error = MapServiceError;
    fn poll_batch(
        &mut self,
        cx: &mut Context<'_>,
        max_items: usize,
    ) -> BatchPoll<Self::Item, Self::Error> {
        match self.registration.as_ref() {
            None => Poll::Ready(Ok(None)),
            Some(registration) => {
                let (epoch, result) =
                    self.service
                        .poll_listener_epoch(registration.id(), max_items, cx);
                if let Some(epoch) = epoch {
                    self.delivery_epoch = epoch;
                }
                result
            }
        }
    }
    fn close(&mut self) {
        if !self.closed {
            self.closed = true;
            drop(self.registration.take());
            self.service.close_stream(false);
        }
    }

    fn validate_buffered(&mut self) -> Result<(), Self::Error> {
        match self.registration.as_ref() {
            None => Ok(()),
            Some(registration) => {
                let state = self
                    .service
                    .state
                    .lock()
                    .expect("local IMap state poisoned");
                lifecycle::require_namespace(&state)?;
                if let Some(listener) = state.listeners.get(&registration.id()) {
                    if self.delivery_epoch != listener.delivery_epoch {
                        self.delivery_epoch = listener.delivery_epoch;
                        self.reset_buffer = true;
                    }
                    Ok(())
                } else {
                    Err(MapServiceError::UnknownListener)
                }
            }
        }
    }

    fn take_buffer_reset(&mut self) -> bool {
        std::mem::take(&mut self.reset_buffer)
    }
}

impl Drop for ServiceListenerSource {
    fn drop(&mut self) {
        self.close();
    }
}
