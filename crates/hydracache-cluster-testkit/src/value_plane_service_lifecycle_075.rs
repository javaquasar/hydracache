//! Runtime-only lifecycle helpers over the service's existing collection owner.
//! Byte accounting covers logical owned payload, not allocator overhead or caller buffers.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServiceLimits {
    pub value_bytes: usize,
    pub outcome_bytes: usize,
    pub pending_bytes: usize,
    pub listener_bytes: usize,
    pub stream_bytes: usize,
    pub resync_bytes: usize,
    pub max_clients: usize,
    pub max_batch: usize,
    pub expiry_work: usize,
    pub query_bytes: usize,
}

impl Default for ServiceLimits {
    fn default() -> Self {
        Self {
            value_bytes: 64 * 1_024 * 1_024,
            outcome_bytes: 32 * 1_024 * 1_024,
            pending_bytes: 1_024 * 1_024,
            listener_bytes: 16 * 1_024 * 1_024,
            stream_bytes: 64 * 1_024 * 1_024,
            resync_bytes: 8 * 1_024 * 1_024,
            max_clients: 4_096,
            max_batch: 256,
            expiry_work: 64,
            query_bytes: 8 * 1_024 * 1_024,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ServiceResourceUsage {
    pub values: usize,
    pub outcomes: usize,
    pub pending: usize,
    pub listeners: usize,
    pub streams: usize,
    pub resync: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetirementReceipt {
    pub removed: usize,
    pub through: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpiryProgress {
    pub examined: usize,
    pub expired: usize,
    pub pending: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionBulkOperation {
    Replace,
    Remove,
    SetTtl,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionBulkEntry {
    pub key: Vec<u8>,
    pub token: VersionToken,
    pub value: Option<Vec<u8>>,
    pub ttl: ServiceTtl,
}

/// A bounded snapshot lease. Dropping or closing returns its aggregate resync credit.
#[derive(Debug)]
pub struct ListenerSnapshot {
    pub entries: Vec<ServiceEntry>,
    pub watermark: u64,
    pub namespace_generation: u64,
    pub map_id: u64,
    lease: ByteLease,
}

impl ListenerSnapshot {
    pub fn close(&mut self) {
        self.entries.clear();
        self.lease.release();
    }
}

#[derive(Debug)]
pub(super) struct ByteLease {
    pub(super) service: Weak<Mutex<ServiceState>>,
    pub(super) bytes: usize,
    pub(super) active: bool,
}
impl ByteLease {
    pub(super) fn release(&mut self) {
        if self.active {
            if let Some(service) = self.service.upgrade() {
                let mut state = service.lock().expect("local IMap state poisoned");
                state.resync_bytes = state.resync_bytes.saturating_sub(self.bytes);
                state.resync_leases = state.resync_leases.saturating_sub(1);
            }
            self.bytes = 0;
            self.active = false;
        }
    }
}
impl Drop for ByteLease {
    fn drop(&mut self) {
        self.release();
    }
}

/// In-memory only; it has no serialization or durable compatibility identity.
#[derive(Debug)]
pub struct ServiceCheckpoint {
    map: Option<ReferenceCollectionMap>,
    generation: u64,
    now: u64,
    watermark: u64,
    limits: ServiceLimits,
    lease: ByteLease,
}

impl ServiceCheckpoint {
    pub fn close(&mut self) {
        self.map.take();
        self.lease.release();
    }
}

pub(super) fn budget(
    class: &'static str,
    maximum: usize,
    actual: usize,
) -> Result<(), MapServiceError> {
    if actual > maximum {
        Err(MapServiceError::ResourceBudget(class))
    } else {
        Ok(())
    }
}

fn checked_add(a: usize, b: usize, class: &'static str) -> Result<usize, MapServiceError> {
    a.checked_add(b)
        .ok_or(MapServiceError::ResourceBudget(class))
}

pub(super) fn event_bytes(event: &ServiceEvent) -> usize {
    if event.kind == "gap" {
        0
    } else {
        64usize
            .saturating_add(event.key.len())
            .saturating_add(event.old_value.as_ref().map_or(0, Vec::len))
            .saturating_add(event.value.as_ref().map_or(0, Vec::len))
    }
}

pub(super) fn listener_bytes(state: &ServiceState) -> usize {
    state.listeners.values().fold(0usize, |sum, listener| {
        listener.queue.iter().fold(
            sum.saturating_add(128)
                .saturating_add(listener.options.key_prefix.len())
                .saturating_add(listener.options.exact_key.as_ref().map_or(0, Vec::len)),
            |sum, event| sum.saturating_add(event_bytes(event)),
        )
    })
}

pub(super) fn stream_reservation(state: &ServiceState) -> usize {
    state
        .map
        .service_bounds()
        .max_response_bytes
        .saturating_add(64)
}

pub(super) fn usage(state: &ServiceState) -> ServiceResourceUsage {
    let (outcomes, pending) = state.map.service_retained_bytes();
    ServiceResourceUsage {
        values: state.map.service_value_bytes(),
        outcomes,
        pending,
        listeners: listener_bytes(state),
        streams: state
            .active_streams
            .saturating_mul(stream_reservation(state)),
        resync: state.resync_bytes,
    }
}

pub(super) fn check_usage(state: &ServiceState) -> Result<(), MapServiceError> {
    let used = usage(state);
    for (class, maximum, actual) in [
        ("values", state.limits.value_bytes, used.values),
        ("outcomes", state.limits.outcome_bytes, used.outcomes),
        ("pending", state.limits.pending_bytes, used.pending),
        ("listeners", state.limits.listener_bytes, used.listeners),
        ("streams", state.limits.stream_bytes, used.streams),
        ("resync", state.limits.resync_bytes, used.resync),
    ] {
        budget(class, maximum, actual)?;
    }
    Ok(())
}

pub(super) fn check_identity(
    state: &ServiceState,
    id: &MutationIdentity,
) -> Result<(), MapServiceError> {
    if state.map.service_retired(id) {
        Err(MapServiceError::RetiredIdentity)
    } else {
        Ok(())
    }
}

pub(super) fn require_namespace(state: &ServiceState) -> Result<(), MapServiceError> {
    if state.namespace_fenced {
        Err(MapServiceError::NamespaceDeleted)
    } else {
        Ok(())
    }
}

/// Reserve a conservative result bound before changing values or admitting a pending owner.
pub(super) fn preflight_command(
    state: &ServiceState,
    command: &ServiceCommand,
    id: &MutationIdentity,
) -> Result<usize, MapServiceError> {
    let mut used = usage(state);
    if let Some(reserved) = state.map.service_pending_result_bytes(id) {
        let identity_bytes = id.client.len().saturating_add(48);
        used.pending = used.pending.saturating_sub(identity_bytes);
        used.outcomes = used
            .outcomes
            .saturating_sub(identity_bytes.saturating_add(reserved));
    }
    let mut result_bytes = 64usize;
    let mut values = used.values;
    let op = command.operation.as_str();
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
        result_bytes = checked_add(
            result_bytes,
            state
                .map
                .service_entry(&command.key, command.now)
                .map_or(0, |v| v.value.len()),
            "outcomes",
        )?;
    }
    if !command.keys.is_empty() || !command.entries.is_empty() {
        let keys: Vec<&[u8]> = if command.entries.is_empty() {
            command.keys.iter().map(Vec::as_slice).collect()
        } else {
            command.entries.iter().map(|e| e.key.as_slice()).collect()
        };
        for key in keys {
            result_bytes =
                checked_add(result_bytes, 64usize.saturating_add(key.len()), "outcomes")?;
            if op == "remove_all_detailed" {
                result_bytes = checked_add(
                    result_bytes,
                    state
                        .map
                        .service_entry(key, command.now)
                        .map_or(0, |v| v.value.len()),
                    "outcomes",
                )?;
            }
        }
    }
    let current = state.map.service_entry(&command.key, command.now);
    let would_apply = match op {
        "put_if_absent" => current.is_none(),
        "replace_if_present" => current.is_some(),
        "conditional_replace" => current.as_ref().map(|v| &v.value) == command.expected.as_ref(),
        "replace_if_version" => current.as_ref().is_some_and(|v| {
            command.token.is_some_and(|token| {
                token.namespace_generation == state.namespace_generation
                    && token.map_id == state.map.service_map_id()
                    && token.version == v.version
            })
        }),
        _ => true,
    };
    if would_apply
        && matches!(
            op,
            "put"
                | "set"
                | "try_put"
                | "get_and_put"
                | "put_if_absent"
                | "replace_if_present"
                | "conditional_replace"
                | "replace_if_version"
        )
    {
        if let Some(value) = &command.value {
            values = values.saturating_sub(state.map.service_physical_entry_bytes(&command.key));
            values = checked_add(
                values,
                24usize
                    .saturating_add(command.key.len())
                    .saturating_add(value.len()),
                "values",
            )?;
        }
    }
    if matches!(op, "put_all_detailed" | "replace_all_detailed") {
        for entry in &command.entries {
            if op == "replace_all_detailed"
                && state.map.service_entry(&entry.key, command.now).is_none()
            {
                continue;
            }
            values = values.saturating_sub(state.map.service_physical_entry_bytes(&entry.key));
            values = checked_add(
                values,
                24usize
                    .saturating_add(entry.key.len())
                    .saturating_add(entry.value.len()),
                "values",
            )?;
            budget("values", state.limits.value_bytes, values)?;
        }
    }
    budget("values", state.limits.value_bytes, values)?;
    let identity_bytes = 48usize.saturating_add(id.client.len());
    budget(
        "pending",
        state.limits.pending_bytes,
        checked_add(used.pending, identity_bytes, "pending")?,
    )?;
    budget(
        "outcomes",
        state.limits.outcome_bytes,
        checked_add(
            used.outcomes,
            checked_add(identity_bytes, result_bytes, "outcomes")?,
            "outcomes",
        )?,
    )?;
    Ok(result_bytes)
}

impl UnifiedIMapService {
    pub fn with_limits(
        bounds: CollectionBounds,
        generation: u64,
        limits: ServiceLimits,
    ) -> Result<Self, MapServiceError> {
        if [
            limits.value_bytes,
            limits.outcome_bytes,
            limits.pending_bytes,
            limits.listener_bytes,
            limits.stream_bytes,
            limits.resync_bytes,
            limits.max_clients,
            limits.max_batch,
            limits.expiry_work,
            limits.query_bytes,
        ]
        .contains(&0)
            || limits.max_batch > 256
        {
            return Err(MapServiceError::InvalidArgument);
        }
        Self::from_map_with_limits(ReferenceCollectionMap::new(bounds)?, generation, limits)
    }

    pub fn resource_usage(&self) -> ServiceResourceUsage {
        usage(&self.state.lock().expect("local IMap state poisoned"))
    }

    pub fn retire_outcomes(
        &self,
        client: &str,
        generation: u64,
        through: u64,
    ) -> Result<RetirementReceipt, MapServiceError> {
        let mut state = self.state.lock().expect("local IMap state poisoned");
        require_namespace(&state)?;
        if state.map.is_destroyed() {
            return Err(CollectionError::MapDestroyed.into());
        }
        if generation != state.namespace_generation
            || client.trim().is_empty()
            || client.len() > 128
            || through == 0
            || state
                .map
                .service_fences()
                .get(client)
                .is_some_and(|old| through < *old)
        {
            return Err(MapServiceError::InvalidArgument);
        }
        if state.map.service_pending_before(client, through) {
            return Err(MapServiceError::PendingOperation);
        }
        cluster_hooks::require_replication_capacity(&state)?;
        if !state.map.service_fences().contains_key(client) {
            if state.map.service_fences().len() >= state.limits.max_clients {
                return Err(MapServiceError::ResourceBudget("retirement_clients"));
            }
            budget(
                "outcomes",
                state.limits.outcome_bytes,
                checked_add(
                    usage(&state)
                        .outcomes
                        .saturating_sub(state.map.service_retirement_credit(client, through)),
                    client.len().saturating_add(24),
                    "outcomes",
                )?,
            )?;
        }
        let removed = state.map.service_retire(client.into(), through);
        Ok(RetirementReceipt { removed, through })
    }

    pub fn query_operation_outcomes(
        &self,
        identities: &[MutationIdentity],
        max_bytes: usize,
    ) -> Result<Vec<OperationOutcomeQuery>, MapServiceError> {
        let state = self.state.lock().expect("local IMap state poisoned");
        require_namespace(&state)?;
        if identities.is_empty()
            || identities.len() > state.limits.max_batch
            || max_bytes == 0
            || max_bytes > state.limits.query_bytes
        {
            return Err(MapServiceError::InvalidArgument);
        }
        let mut total = 0usize;
        for id in identities {
            let result_bytes = state.map.service_query_result_bytes(id)?;
            total = checked_add(
                total,
                checked_add(
                    64usize.saturating_add(id.client.len()),
                    result_bytes,
                    "query_response",
                )?,
                "query_response",
            )?;
            budget("query_response", max_bytes, total)?;
        }
        let mut outcomes = Vec::with_capacity(identities.len());
        for id in identities {
            let outcome = match state.map.service_lookup(id, None)? {
                ServiceRetainedOutcome::Unknown => OperationOutcomeQuery::Unknown,
                ServiceRetainedOutcome::Pending => OperationOutcomeQuery::Pending,
                ServiceRetainedOutcome::Completed(value) => OperationOutcomeQuery::Completed(
                    Box::new(ServiceResult::from_retained(&value)?),
                ),
            };
            outcomes.push(outcome);
        }
        Ok(outcomes)
    }

    pub fn expire_step(
        &self,
        now: u64,
        max_work: usize,
    ) -> Result<ExpiryProgress, MapServiceError> {
        let mut state = self.state.lock().expect("local IMap state poisoned");
        require_namespace(&state)?;
        if max_work == 0 || max_work > state.map.service_bounds().max_scan_items {
            return Err(MapServiceError::InvalidArgument);
        }
        if now < state.now {
            return Err(MapServiceError::ClockRegressed);
        }
        if state.map.is_destroyed() {
            return Err(CollectionError::MapDestroyed.into());
        }
        if now != state.now || state.map.service_expiry_pending(now) {
            cluster_hooks::require_replication_capacity(&state)?;
        }
        let expired = state.map.service_expire_step(now, max_work)?;
        let count = expired.len();
        state.now = now;
        for (key, value) in expired {
            publish(&mut state, "expired", key, Some(value), None);
        }
        let progress = ExpiryProgress {
            examined: count,
            expired: count,
            pending: state.map.service_expiry_pending(now),
        };
        let wakes = std::mem::take(&mut state.pending_wakes);
        drop(state);
        for wake in wakes {
            wake.wake();
        }
        Ok(progress)
    }

    pub fn resync_listener(
        &self,
        id: u64,
        now: u64,
        max_items: usize,
        max_bytes: usize,
    ) -> Result<ListenerSnapshot, MapServiceError> {
        let mut state = self.state.lock().expect("local IMap state poisoned");
        require_namespace(&state)?;
        if state.resync_leases >= state.map.service_bounds().max_entries {
            return Err(MapServiceError::Capacity);
        }
        if state.map.is_destroyed() {
            return Err(CollectionError::MapDestroyed.into());
        }
        if max_items == 0
            || max_items > state.map.service_bounds().max_entries
            || max_bytes == 0
            || max_bytes > state.limits.resync_bytes
        {
            return Err(MapServiceError::InvalidArgument);
        }
        let options = state
            .listeners
            .get(&id)
            .ok_or(MapServiceError::UnknownListener)?
            .options
            .clone();
        if options.include_value
            && state.map.service_snapshot_value_exceeds(
                &options.key_prefix,
                options.exact_key.as_deref(),
                now,
                options.max_value_bytes,
            )
        {
            return Err(MapServiceError::ResourceBudget("resync_value_bytes"));
        }
        let next_epoch = state.listeners[&id]
            .delivery_epoch
            .checked_add(1)
            .ok_or(MapServiceError::Capacity)?;
        if now < state.now {
            return Err(MapServiceError::ClockRegressed);
        }
        // Check size before advancing expiry or clearing a sticky gap.
        let (count, bytes) = state.map.service_snapshot_cost(
            &options.key_prefix,
            options.exact_key.as_deref(),
            options.include_value,
            now,
        );
        budget("resync_items", max_items, count)?;
        budget("resync", max_bytes, bytes)?;
        budget(
            "resync",
            state.limits.resync_bytes,
            checked_add(state.resync_bytes, bytes, "resync")?,
        )?;
        advance(&mut state, now)?;
        let entries = state
            .map
            .service_filtered_snapshot(
                &options.key_prefix,
                options.exact_key.as_deref(),
                options.include_value,
                now,
            )
            .into_iter()
            .map(|(key, view)| ServiceEntry {
                key,
                value: view.value,
                version: view.version,
                remaining_ttl: view.remaining_ttl,
            })
            .collect();
        let listener = state
            .listeners
            .get_mut(&id)
            .expect("registration held under lock");
        listener.delivery_epoch = next_epoch;
        listener.queue.clear();
        listener.gap = false;
        state.resync_bytes += bytes;
        state.resync_leases += 1;
        let snapshot = ListenerSnapshot {
            entries,
            watermark: state.watermark,
            namespace_generation: state.namespace_generation,
            map_id: state.map.service_map_id(),
            lease: ByteLease {
                service: Arc::downgrade(&self.state),
                bytes,
                active: true,
            },
        };
        let wakes = std::mem::take(&mut state.pending_wakes);
        drop(state);
        for wake in wakes {
            wake.wake();
        }
        Ok(snapshot)
    }

    pub fn checkpoint(&self) -> Result<ServiceCheckpoint, MapServiceError> {
        let mut state = self.state.lock().expect("local IMap state poisoned");
        require_namespace(&state)?;
        if state.map.service_pending_count() != 0
            || !state.listeners.is_empty()
            || state.active_streams != 0
            || state.resync_leases != 0
        {
            return Err(MapServiceError::ActiveOwners);
        }
        let used = usage(&state);
        let bytes = checked_add(
            checked_add(used.values, used.outcomes, "resync")?,
            256,
            "resync",
        )?;
        budget("resync", state.limits.resync_bytes, bytes)?;
        state.resync_bytes = bytes;
        state.resync_leases += 1;
        Ok(ServiceCheckpoint {
            map: Some(state.map.service_checkpoint_clone()),
            generation: state.namespace_generation,
            now: state.now,
            watermark: state.watermark,
            limits: state.limits,
            lease: ByteLease {
                service: Arc::downgrade(&self.state),
                bytes,
                active: true,
            },
        })
    }

    pub fn restore(mut checkpoint: ServiceCheckpoint) -> Result<Self, MapServiceError> {
        let mut map = checkpoint
            .map
            .take()
            .ok_or(MapServiceError::InvalidArgument)?;
        map.service_restore_identity()?;
        let service = Self::from_map_with_limits(map, checkpoint.generation, checkpoint.limits)?;
        {
            let mut state = service.state.lock().expect("local IMap state poisoned");
            state.now = checkpoint.now;
            state.watermark = checkpoint.watermark;
            state.limits = checkpoint.limits;
            check_usage(&state)?;
        }
        checkpoint.lease.release();
        Ok(service)
    }

    pub fn bulk_version_conditional(
        &self,
        operation: VersionBulkOperation,
        entries: &[VersionBulkEntry],
        identity: MutationIdentity,
        now: u64,
        deadline: u64,
        lose_response: bool,
    ) -> Result<ServiceResult, MapServiceError> {
        let mut state = self.state.lock().expect("local IMap state poisoned");
        let result = bulk_version(
            &mut state,
            operation,
            entries,
            identity,
            now,
            deadline,
            lose_response,
        );
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
}

fn bulk_version(
    state: &mut ServiceState,
    operation: VersionBulkOperation,
    entries: &[VersionBulkEntry],
    identity: MutationIdentity,
    now: u64,
    deadline: u64,
    lose_response: bool,
) -> Result<ServiceResult, MapServiceError> {
    require_namespace(state)?;
    state.requests = state.requests.saturating_add(1);
    if entries.is_empty()
        || entries.len() > state.limits.max_batch
        || identity.client.trim().is_empty()
        || identity.client.len() > 128
        || identity.sequence == 0
    {
        return Err(MapServiceError::InvalidArgument);
    }
    let mut seen = BTreeSet::new();
    let bounds = state.map.service_bounds();
    let mut request_bytes = 0usize;
    for entry in entries {
        if !seen.insert(&entry.key)
            || (operation == VersionBulkOperation::Replace && entry.value.is_none())
            || (operation != VersionBulkOperation::Replace && entry.value.is_some())
            || (operation == VersionBulkOperation::SetTtl && entry.ttl == ServiceTtl::Preserve)
            || matches!(entry.ttl, ServiceTtl::ExpireAfter(0))
        {
            return Err(MapServiceError::InvalidArgument);
        }
        check_bound("key_bytes", bounds.max_key_bytes, entry.key.len())?;
        if let Some(value) = &entry.value {
            check_bound("value_bytes", bounds.max_value_bytes, value.len())?;
        }
        request_bytes = checked_add(
            request_bytes,
            64usize
                .saturating_add(entry.key.len())
                .saturating_add(entry.value.as_ref().map_or(0, Vec::len)),
            "request",
        )?;
    }
    check_bound("request_bytes", 8 * 1_024 * 1_024, request_bytes)?;
    let digest: [u8; 32] = Sha256::digest(
        json!([
            format!("{:?}", operation),
            entries
                .iter()
                .map(|e| json!([
                    e.key,
                    e.token.namespace_generation,
                    e.token.map_id,
                    e.token.version,
                    e.value,
                    format!("{:?}", e.ttl)
                ]))
                .collect::<Vec<_>>()
        ])
        .to_string()
        .as_bytes(),
    )
    .into();
    check_identity(state, &identity)?;
    match state.map.service_lookup(&identity, Some(digest))? {
        ServiceRetainedOutcome::Completed(value) => return ServiceResult::from_retained(&value),
        ServiceRetainedOutcome::Pending => return Ok(ServiceResult::new("pending")),
        ServiceRetainedOutcome::Unknown => {}
    }
    if now >= deadline {
        return Ok(ServiceResult::new("not_applied"));
    }
    cluster_hooks::require_replication_capacity(state)?;
    if state.map.is_destroyed() {
        return Err(CollectionError::MapDestroyed.into());
    }
    if now < state.now {
        return Err(MapServiceError::ClockRegressed);
    }
    let mut values = state.map.service_value_bytes();
    let mut result_bytes = 64usize;
    let mut changed = 0usize;
    for entry in entries {
        result_bytes = checked_add(
            result_bytes,
            64usize.saturating_add(entry.key.len()),
            "outcomes",
        )?;
        let current = state.map.service_entry(&entry.key, now);
        if !current.as_ref().is_some_and(|v| {
            entry.token.namespace_generation == state.namespace_generation
                && entry.token.map_id == state.map.service_map_id()
                && entry.token.version == v.version
        }) {
            continue;
        }
        changed += 1;
        if operation == VersionBulkOperation::Replace {
            values = values.saturating_sub(state.map.service_physical_entry_bytes(&entry.key));
            values = checked_add(
                values,
                24usize
                    .saturating_add(entry.key.len())
                    .saturating_add(entry.value.as_ref().expect("validated replacement").len()),
                "values",
            )?;
            budget("values", state.limits.value_bytes, values)?;
        }
        if operation != VersionBulkOperation::Remove {
            let mut command = ServiceCommand::point("set_ttl", &entry.key);
            command.now = now;
            command.ttl = entry.ttl;
            expiry(&command, current.as_ref())?;
        }
    }
    budget("values", state.limits.value_bytes, values)?;
    check_bound(
        "response_bytes",
        bounds.max_response_bytes,
        entries.iter().fold(0usize, |sum, entry| {
            sum.saturating_add(17).saturating_add(entry.key.len())
        }),
    )?;
    let used = usage(state);
    let owner = identity.client.len().saturating_add(48);
    budget(
        "pending",
        state.limits.pending_bytes,
        checked_add(used.pending, owner, "pending")?,
    )?;
    budget(
        "outcomes",
        state.limits.outcome_bytes,
        checked_add(
            used.outcomes,
            checked_add(owner, result_bytes, "outcomes")?,
            "outcomes",
        )?,
    )?;
    state.map.service_reserve_versions(
        changed + usize::from(now > state.now && state.map.service_expiry_pending(now)),
    )?;
    advance(state, now)?;
    state
        .map
        .service_reserve(identity.clone(), digest, result_bytes)?;
    let mut result = ServiceResult::new("bulk");
    for (index, entry) in entries.iter().enumerate() {
        if let Some(value) = state.map.service_expire_key(&entry.key, now) {
            publish(state, "expired", entry.key.clone(), Some(value), None);
        }
        let current = state.map.service_entry(&entry.key, now);
        let matched = current.as_ref().is_some_and(|v| {
            entry.token.namespace_generation == state.namespace_generation
                && entry.token.map_id == state.map.service_map_id()
                && entry.token.version == v.version
        });
        let (kind, version, remaining_ttl) = if !matched {
            (
                if current.is_some() {
                    "mismatch"
                } else {
                    "absent"
                },
                current.as_ref().map(|view| view.version),
                current.as_ref().and_then(|view| view.remaining_ttl),
            )
        } else if operation == VersionBulkOperation::Remove {
            state.map.evict(&entry.key, now)?;
            publish(
                state,
                "removed",
                entry.key.clone(),
                current.map(|view| view.value),
                None,
            );
            ("removed", Some(state.map.revision()), None)
        } else {
            let mut command = ServiceCommand::point("set_ttl", &entry.key);
            command.now = now;
            command.ttl = entry.ttl;
            let expires_at = expiry(&command, current.as_ref())?;
            let value = if operation == VersionBulkOperation::Replace {
                entry.value.clone().expect("validated replacement")
            } else {
                current.as_ref().expect("matched entry").value.clone()
            };
            state
                .map
                .set(entry.key.clone(), value.clone(), expires_at, now)?;
            publish(
                state,
                "updated",
                entry.key.clone(),
                current.map(|view| view.value),
                Some(value),
            );
            (
                "replaced",
                Some(state.map.revision()),
                expires_at.map(|at| at - now),
            )
        };
        result.items.push(ServiceItem {
            index,
            key: entry.key.clone(),
            kind: kind.into(),
            value: None,
            version,
            remaining_ttl,
        });
    }
    result.number = result.items.len() as u64;
    state
        .map
        .service_complete(&identity, result.retained_json());
    Ok(if lose_response {
        ServiceResult::new("outcome_unknown")
    } else {
        result
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listener_delivery_epoch_exhaustion_preserves_queue_and_gap() {
        let service = UnifiedIMapService::new(CollectionBounds::default(), 1).unwrap();
        let registration = service
            .register_listener(ServiceListenerOptions::default())
            .unwrap();
        let mut command = ServiceCommand::point("put", b"a");
        command.value = Some(b"v".to_vec());
        command.identity = Some(MutationIdentity::new("epoch", 1));
        service.execute(command).unwrap();
        {
            let mut state = service.state.lock().unwrap();
            let listener = state.listeners.get_mut(&registration.id()).unwrap();
            listener.delivery_epoch = u64::MAX;
            listener.gap = true;
        }
        assert_eq!(
            service.repair_listener(registration.id()).unwrap_err(),
            MapServiceError::Capacity
        );
        assert_eq!(
            service
                .resync_listener(registration.id(), 0, 1, 1_024)
                .unwrap_err(),
            MapServiceError::Capacity
        );
        let state = service.state.lock().unwrap();
        let listener = &state.listeners[&registration.id()];
        assert_eq!(listener.queue.len(), 1);
        assert!(listener.gap);
        assert_eq!(state.resync_bytes, 0);
    }
}
