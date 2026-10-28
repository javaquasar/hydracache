//! Administrative, runtime-only hooks for the test cluster and namespace registry.
//! Replica images are typed owner decisions, not commands to reevaluate on a backup.

use super::lifecycle::{budget, require_namespace, ByteLease};
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplicaStamp {
    pub sequence: u64,
    pub map_id: u64,
    pub namespace_generation: u64,
    pub revision: u64,
    pub digest: [u8; 32],
}

#[derive(Debug)]
pub struct ServiceReplicaImage {
    map: Option<ReferenceCollectionMap>,
    generation: u64,
    now: u64,
    watermark: u64,
    stamp: ReplicaStamp,
    lease: ByteLease,
}

impl ServiceReplicaImage {
    pub fn owned_payload_bytes(&self) -> usize {
        self.lease.bytes
    }
    pub const fn stamp(&self) -> ReplicaStamp {
        self.stamp
    }

    /// A bounded diagnostic payload used only by the logical fault transport.
    /// There is deliberately no reader, schema version, or production wire identity.
    pub fn transport_payload(&self, maximum: usize) -> Result<Vec<u8>, MapServiceError> {
        let mut writer = BoundedDebug {
            bytes: Vec::new(),
            maximum,
        };
        fmt::Write::write_fmt(
            &mut writer,
            format_args!(
                "{}:{}:{}:{}:{:?}",
                self.stamp.sequence,
                self.generation,
                self.now,
                self.watermark,
                self.map.as_ref().ok_or(MapServiceError::InvalidArgument)?
            ),
        )
        .map_err(|_| MapServiceError::ResourceBudget("transport_payload"))?;
        Ok(writer.bytes)
    }
}

struct BoundedDebug {
    bytes: Vec<u8>,
    maximum: usize,
}

struct CountingDebug {
    bytes: usize,
    maximum: usize,
}
impl fmt::Write for CountingDebug {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        let next = self.bytes.checked_add(value.len()).ok_or(fmt::Error)?;
        if next > self.maximum {
            return Err(fmt::Error);
        }
        self.bytes = next;
        Ok(())
    }
}
impl fmt::Write for BoundedDebug {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        if self
            .bytes
            .len()
            .checked_add(value.len())
            .is_none_or(|n| n > self.maximum)
        {
            return Err(fmt::Error);
        }
        self.bytes.extend_from_slice(value.as_bytes());
        Ok(())
    }
}

struct HashDebug(Sha256);
impl fmt::Write for HashDebug {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        self.0.update(value.as_bytes());
        Ok(())
    }
}

fn stamp(map: &ReferenceCollectionMap, generation: u64, now: u64, watermark: u64) -> ReplicaStamp {
    let mut writer = HashDebug(Sha256::new());
    fmt::Write::write_fmt(
        &mut writer,
        format_args!("{generation}:{now}:{watermark}:{map:?}"),
    )
    .expect("hash writer cannot fail");
    ReplicaStamp {
        sequence: 0,
        map_id: map.service_map_id(),
        namespace_generation: generation,
        revision: map.revision(),
        digest: writer.0.finalize().into(),
    }
}

pub(super) fn require_replication_capacity(state: &ServiceState) -> Result<(), MapServiceError> {
    if state.replica_sequence == u64::MAX {
        Err(MapServiceError::Capacity)
    } else {
        Ok(())
    }
}

fn current_stamp(state: &mut ServiceState) -> Result<ReplicaStamp, MapServiceError> {
    let mut result = stamp(
        &state.map,
        state.namespace_generation,
        state.now,
        state.watermark,
    );
    if state.replica_digest != Some(result.digest) {
        let next = state
            .replica_sequence
            .checked_add(1)
            .ok_or(MapServiceError::Capacity)?;
        state.replica_sequence = next;
        state.replica_digest = Some(result.digest);
    }
    result.sequence = state.replica_sequence;
    Ok(result)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExternalAdmission {
    Reserved,
    Pending,
    Completed(Box<ServiceResult>),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NamespaceOwners {
    pub values: usize,
    pub completed_outcomes: usize,
    pub retirement_fences: usize,
    pub pending: usize,
    pub listeners: usize,
    pub streams: usize,
    pub resync_leases: usize,
    pub resync_bytes: usize,
}

impl NamespaceOwners {
    pub fn total(self) -> usize {
        self.cleanable_records()
            .saturating_add(self.blocked_owners())
    }
    pub fn cleanable_records(self) -> usize {
        self.values + self.completed_outcomes + self.retirement_fences + self.listeners
    }
    pub fn blocked_owners(self) -> usize {
        self.pending + self.streams + self.resync_leases
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NamespaceReclaimProgress {
    pub reclaimed: usize,
    pub remaining: usize,
    pub blocked_owners: usize,
}

fn owners(state: &ServiceState) -> NamespaceOwners {
    let (values, completed_outcomes, retirement_fences, pending) = state.map.service_owner_counts();
    NamespaceOwners {
        values,
        completed_outcomes,
        retirement_fences,
        pending,
        listeners: state.listeners.len(),
        streams: state.active_streams,
        resync_leases: state.resync_leases,
        resync_bytes: state.resync_bytes,
    }
}

fn result_bytes(result: &ServiceResult) -> usize {
    result.items.iter().fold(
        64usize.saturating_add(result.value.as_ref().map_or(0, Vec::len)),
        |sum, item| {
            sum.saturating_add(64)
                .saturating_add(item.key.len())
                .saturating_add(item.value.as_ref().map_or(0, Vec::len))
        },
    )
}

impl ServiceCommand {
    pub fn test_transport_size(&self, maximum: usize) -> Result<usize, MapServiceError> {
        let mut writer = CountingDebug { bytes: 0, maximum };
        fmt::Write::write_fmt(&mut writer, format_args!("{self:?}"))
            .map_err(|_| MapServiceError::ResourceBudget("transport_payload"))?;
        Ok(writer.bytes)
    }
    pub fn test_transport_payload(&self, maximum: usize) -> Result<Vec<u8>, MapServiceError> {
        let mut writer = BoundedDebug {
            bytes: Vec::new(),
            maximum,
        };
        fmt::Write::write_fmt(&mut writer, format_args!("{self:?}"))
            .map_err(|_| MapServiceError::ResourceBudget("transport_payload"))?;
        Ok(writer.bytes)
    }

    pub fn test_request_digest(&self) -> [u8; 32] {
        let mut writer = HashDebug(Sha256::new());
        fmt::Write::write_fmt(
            &mut writer,
            format_args!(
                "{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{}:{}:{}",
                self.operation,
                self.key,
                self.value,
                self.expected,
                self.keys,
                self.entries,
                self.ttl,
                self.token,
                self.cursor,
                self.listener_id,
                self.limit,
                self.include_value
            ),
        )
        .expect("hash writer cannot fail");
        writer.0.finalize().into()
    }
}

impl ServiceResult {
    pub fn test_transport_payload(&self, maximum: usize) -> Result<Vec<u8>, MapServiceError> {
        let mut writer = BoundedDebug {
            bytes: Vec::new(),
            maximum,
        };
        fmt::Write::write_fmt(&mut writer, format_args!("{self:?}"))
            .map_err(|_| MapServiceError::ResourceBudget("transport_payload"))?;
        Ok(writer.bytes)
    }
}

impl UnifiedIMapService {
    pub fn replica_transport_size(&self, maximum: usize) -> Result<usize, MapServiceError> {
        let state = self.state.lock().expect("local IMap state poisoned");
        require_namespace(&state)?;
        let mut writer = CountingDebug { bytes: 0, maximum };
        fmt::Write::write_fmt(
            &mut writer,
            format_args!(
                "{}:{}:{}:{}:{:?}",
                u64::MAX,
                state.namespace_generation,
                u64::MAX,
                u64::MAX,
                state.map
            ),
        )
        .map_err(|_| MapServiceError::ResourceBudget("transport_payload"))?;
        Ok(writer.bytes)
    }

    pub fn preflight_replica_image(&self) -> Result<usize, MapServiceError> {
        let state = self.state.lock().expect("local IMap state poisoned");
        require_namespace(&state)?;
        if state.map.service_pending_count() != 0 {
            return Err(MapServiceError::ActiveOwners);
        }
        if state.resync_leases >= state.map.service_bounds().max_entries {
            return Err(MapServiceError::Capacity);
        }
        let used = lifecycle::usage(&state);
        let bytes = used
            .values
            .checked_add(used.outcomes)
            .and_then(|n| n.checked_add(256))
            .ok_or(MapServiceError::Capacity)?;
        budget(
            "resync",
            state.limits.resync_bytes,
            state
                .resync_bytes
                .checked_add(bytes)
                .ok_or(MapServiceError::Capacity)?,
        )?;
        Ok(bytes)
    }
    pub fn listener_watermark(&self, id: u64) -> Result<(u64, u64), MapServiceError> {
        let state = self.state.lock().expect("local IMap state poisoned");
        require_namespace(&state)?;
        if !state.listeners.contains_key(&id) {
            return Err(MapServiceError::UnknownListener);
        }
        Ok((state.map.service_map_id(), state.watermark))
    }

    pub fn preflight_listener_resync(
        &self,
        id: u64,
        now: u64,
        maximum_items: usize,
        maximum_bytes: usize,
    ) -> Result<(usize, usize), MapServiceError> {
        let state = self.state.lock().expect("local IMap state poisoned");
        require_namespace(&state)?;
        if state.map.is_destroyed() {
            return Err(CollectionError::MapDestroyed.into());
        }
        if now < state.now {
            return Err(MapServiceError::ClockRegressed);
        }
        let listener = state
            .listeners
            .get(&id)
            .ok_or(MapServiceError::UnknownListener)?;
        if listener.options.include_value
            && state.map.service_snapshot_value_exceeds(
                &listener.options.key_prefix,
                listener.options.exact_key.as_deref(),
                now,
                listener.options.max_value_bytes,
            )
        {
            return Err(MapServiceError::ResourceBudget("resync_value_bytes"));
        }
        listener
            .delivery_epoch
            .checked_add(1)
            .ok_or(MapServiceError::Capacity)?;
        if state.resync_leases >= state.map.service_bounds().max_entries {
            return Err(MapServiceError::Capacity);
        }
        let cost = state.map.service_snapshot_cost(
            &listener.options.key_prefix,
            listener.options.exact_key.as_deref(),
            listener.options.include_value,
            now,
        );
        budget("resync_items", maximum_items, cost.0)?;
        budget("resync", maximum_bytes, cost.1)?;
        budget(
            "resync",
            state.limits.resync_bytes,
            state
                .resync_bytes
                .checked_add(cost.1)
                .ok_or(MapServiceError::Capacity)?,
        )?;
        Ok(cost)
    }

    pub fn validate_request(&self, command: &ServiceCommand) -> Result<(), MapServiceError> {
        let state = self.state.lock().expect("local IMap state poisoned");
        require_namespace(&state)?;
        validate(&state, command)?;
        if mutating(&command.operation) && command.identity.is_none() {
            return Err(MapServiceError::InvalidArgument);
        }
        Ok(())
    }

    pub fn encoded_response_bound(
        &self,
        command: &ServiceCommand,
    ) -> Result<usize, MapServiceError> {
        let state = self.state.lock().expect("local IMap state poisoned");
        require_namespace(&state)?;
        validate(&state, command)?;
        let value = || {
            state
                .map
                .service_live_value_len(&command.key, command.now)
                .unwrap_or(0)
        };
        let bytes = match command.operation.as_str() {
            "get_all" | "remove_all_detailed" => command.keys.iter().fold(0usize, |sum, key| {
                sum.saturating_add(17)
                    .saturating_add(key.len())
                    .saturating_add(
                        state
                            .map
                            .service_live_value_len(key, command.now)
                            .unwrap_or(0),
                    )
            }),
            "put_all_detailed" | "replace_all_detailed" => {
                command.entries.iter().fold(0usize, |sum, entry| {
                    sum.saturating_add(17).saturating_add(entry.key.len())
                })
            }
            "get"
            | "get_entry_view"
            | "get_and_put"
            | "get_and_remove"
            | "conditional_replace"
            | "conditional_remove"
            | "replace_if_present"
            | "replace_if_version"
            | "put_if_absent" => value(),
            "get_or_default" => state
                .map
                .service_live_value_len(&command.key, command.now)
                .unwrap_or_else(|| command.value.as_ref().map_or(0, Vec::len)),
            _ => 0,
        };
        if bytes == usize::MAX {
            return Err(MapServiceError::Capacity);
        }
        Ok(bytes)
    }

    pub fn command_result_bound(&self, command: &ServiceCommand) -> Result<usize, MapServiceError> {
        let state = self.state.lock().expect("local IMap state poisoned");
        require_namespace(&state)?;
        validate(&state, command)?;
        let id = command
            .identity
            .as_ref()
            .ok_or(MapServiceError::InvalidArgument)?;
        lifecycle::preflight_command(&state, command, id)
    }

    pub fn replica_stamp(&self) -> Result<ReplicaStamp, MapServiceError> {
        let mut state = self.state.lock().expect("local IMap state poisoned");
        require_namespace(&state)?;
        if state.map.service_pending_count() != 0 {
            return Err(MapServiceError::ActiveOwners);
        }
        current_stamp(&mut state)
    }

    /// The image retains a charged clone until installed or dropped. Live registrations are
    /// deliberately excluded; a new owner must register and resynchronize its own listeners.
    pub fn export_replica_image(&self) -> Result<ServiceReplicaImage, MapServiceError> {
        let mut state = self.state.lock().expect("local IMap state poisoned");
        require_namespace(&state)?;
        if state.resync_leases >= state.map.service_bounds().max_entries {
            return Err(MapServiceError::Capacity);
        }
        if state.map.service_pending_count() != 0 {
            return Err(MapServiceError::ActiveOwners);
        }
        let used = lifecycle::usage(&state);
        let bytes = used
            .values
            .checked_add(used.outcomes)
            .and_then(|n| n.checked_add(256))
            .ok_or(MapServiceError::ResourceBudget("resync"))?;
        budget(
            "resync",
            state.limits.resync_bytes,
            state
                .resync_bytes
                .checked_add(bytes)
                .ok_or(MapServiceError::ResourceBudget("resync"))?,
        )?;
        let image_stamp = current_stamp(&mut state)?;
        state.resync_bytes += bytes;
        state.resync_leases += 1;
        Ok(ServiceReplicaImage {
            map: Some(state.map.service_checkpoint_clone()),
            generation: state.namespace_generation,
            now: state.now,
            watermark: state.watermark,
            stamp: image_stamp,
            lease: ByteLease {
                service: Arc::downgrade(&self.state),
                bytes,
                active: true,
            },
        })
    }

    /// Installation moves the validated image into the actual backup collection. No conditional,
    /// TTL directive, or mutation identity is evaluated again on the backup.
    pub fn install_replica_image(
        &self,
        mut image: ServiceReplicaImage,
    ) -> Result<ReplicaStamp, MapServiceError> {
        let mut state = self.state.lock().expect("local IMap state poisoned");
        require_namespace(&state)?;
        let map = image.map.as_ref().ok_or(MapServiceError::InvalidArgument)?;
        let actual = stamp(map, image.generation, image.now, image.watermark);
        if image.generation != state.namespace_generation
            || map.service_bounds() != state.map.service_bounds()
            || actual.digest != image.stamp.digest
            || actual.map_id != image.stamp.map_id
            || actual.namespace_generation != image.stamp.namespace_generation
            || actual.revision != image.stamp.revision
            || image.stamp.sequence == 0
            || map.service_pending_count() != 0
        {
            return Err(MapServiceError::InvalidArgument);
        }
        if owners(&state).blocked_owners() != 0 || !state.listeners.is_empty() {
            return Err(MapServiceError::ActiveOwners);
        }
        let occupied = state.map.service_owner_counts();
        if state.map.service_map_id() != image.stamp.map_id {
            if occupied.0 != 0 || occupied.1 != 0 || occupied.2 != 0 {
                return Err(MapServiceError::InvalidArgument);
            }
        } else {
            let current = current_stamp(&mut state)?;
            if image.stamp.sequence < current.sequence
                || (image.stamp.sequence == current.sequence
                    && image.stamp.digest != current.digest)
                || map.revision() < state.map.revision()
                || image.now < state.now
            {
                return Err(MapServiceError::InvalidArgument);
            }
        }
        let (outcomes, pending) = map.service_retained_bytes();
        budget(
            "values",
            state.limits.value_bytes,
            map.service_value_bytes(),
        )?;
        budget("outcomes", state.limits.outcome_bytes, outcomes)?;
        budget("pending", state.limits.pending_bytes, pending)?;
        state.map = image.map.take().expect("validated replica image");
        state.now = image.now;
        state.watermark = image.watermark;
        state.replica_sequence = image.stamp.sequence;
        state.replica_digest = Some(image.stamp.digest);
        let result = image.stamp;
        drop(state);
        image.lease.release();
        Ok(result)
    }

    pub fn recover_external_operation(
        &self,
        identity: &MutationIdentity,
        digest: [u8; 32],
    ) -> Result<ServiceRecovery, MapServiceError> {
        let state = self.state.lock().expect("local IMap state poisoned");
        require_namespace(&state)?;
        lifecycle::check_identity(&state, identity)?;
        Ok(match state.map.service_lookup(identity, Some(digest))? {
            ServiceRetainedOutcome::Unknown => ServiceRecovery::Unknown,
            ServiceRetainedOutcome::Pending => ServiceRecovery::Pending,
            ServiceRetainedOutcome::Completed(value) => {
                ServiceRecovery::Completed(Box::new(ServiceResult::from_retained(&value)?))
            }
        })
    }

    /// Coordinator receipts use the same bounded ledger as ordinary service mutations.
    pub fn admit_external_operation(
        &self,
        identity: MutationIdentity,
        digest: [u8; 32],
        maximum_result_bytes: usize,
    ) -> Result<ExternalAdmission, MapServiceError> {
        let mut state = self.state.lock().expect("local IMap state poisoned");
        require_namespace(&state)?;
        lifecycle::check_identity(&state, &identity)?;
        match state.map.service_lookup(&identity, Some(digest))? {
            ServiceRetainedOutcome::Pending => return Ok(ExternalAdmission::Pending),
            ServiceRetainedOutcome::Completed(value) => {
                return Ok(ExternalAdmission::Completed(Box::new(
                    ServiceResult::from_retained(&value)?,
                )))
            }
            ServiceRetainedOutcome::Unknown => {}
        }
        if maximum_result_bytes < 64 {
            return Err(MapServiceError::InvalidArgument);
        }
        require_replication_capacity(&state)?;
        let used = lifecycle::usage(&state);
        let base = identity
            .client
            .len()
            .checked_add(48)
            .ok_or(MapServiceError::Capacity)?;
        budget(
            "pending",
            state.limits.pending_bytes,
            used.pending
                .checked_add(base)
                .ok_or(MapServiceError::Capacity)?,
        )?;
        budget(
            "outcomes",
            state.limits.outcome_bytes,
            used.outcomes
                .checked_add(base)
                .and_then(|n| n.checked_add(maximum_result_bytes))
                .ok_or(MapServiceError::Capacity)?,
        )?;
        state
            .map
            .service_reserve(identity, digest, maximum_result_bytes)?;
        Ok(ExternalAdmission::Reserved)
    }

    pub fn complete_external_operation(
        &self,
        identity: &MutationIdentity,
        digest: [u8; 32],
        result: &ServiceResult,
    ) -> Result<(), MapServiceError> {
        let mut state = self.state.lock().expect("local IMap state poisoned");
        require_namespace(&state)?;
        lifecycle::check_identity(&state, identity)?;
        match state.map.service_lookup(identity, Some(digest))? {
            ServiceRetainedOutcome::Pending => {
                require_replication_capacity(&state)?;
                let maximum = state
                    .map
                    .service_pending_result_bytes(identity)
                    .expect("pending reservation has credit");
                budget("outcomes", maximum, result_bytes(result))?;
                state.map.service_complete(identity, result.retained_json());
                Ok(())
            }
            ServiceRetainedOutcome::Completed(value) if value == result.retained_json() => Ok(()),
            _ => Err(MapServiceError::InvalidArgument),
        }
    }

    pub fn namespace_owners(&self) -> NamespaceOwners {
        owners(&self.state.lock().expect("local IMap state poisoned"))
    }
    pub fn is_namespace_fenced(&self) -> bool {
        self.state
            .lock()
            .expect("local IMap state poisoned")
            .namespace_fenced
    }
    pub fn pending_identities(&self) -> Vec<MutationIdentity> {
        self.state
            .lock()
            .expect("local IMap state poisoned")
            .map
            .service_pending_identities()
    }

    pub fn fence_namespace(&self) -> Result<(), MapServiceError> {
        for wake in self.fence_namespace_deferred() {
            wake.wake();
        }
        Ok(())
    }

    pub fn fence_namespace_deferred(&self) -> Vec<Waker> {
        let mut state = self.state.lock().expect("local IMap state poisoned");
        state.namespace_fenced = true;
        let wakes = state
            .listeners
            .values_mut()
            .filter_map(|listener| listener.waker.take())
            .collect::<Vec<_>>();
        wakes
    }

    /// Auth/trust clocks can make a parked listener ready with an error. Its owner calls
    /// this after releasing registry locks so the consumer can recheck its guard.
    pub fn wake_listener_waiters(&self) {
        for wake in self.wake_listener_waiters_deferred() {
            wake.wake();
        }
    }

    pub fn wake_listener_waiters_deferred(&self) -> Vec<Waker> {
        let mut state = self.state.lock().expect("local IMap state poisoned");
        let wakes = state
            .listeners
            .values_mut()
            .filter_map(|listener| listener.waker.take())
            .collect::<Vec<_>>();
        wakes
    }

    pub fn cancel_admitted_identity(
        &self,
        identity: &MutationIdentity,
    ) -> Result<bool, MapServiceError> {
        let mut state = self.state.lock().expect("local IMap state poisoned");
        if !matches!(
            state.map.service_lookup(identity, None)?,
            ServiceRetainedOutcome::Pending
        ) {
            return Ok(false);
        }
        if state.namespace_fenced {
            state.map.service_abort(identity);
        } else {
            require_replication_capacity(&state)?;
            state.map.service_complete(
                identity,
                ServiceResult::new("outcome_unknown").retained_json(),
            );
        }
        Ok(true)
    }

    pub fn reclaim_namespace_step(
        &self,
        maximum: usize,
    ) -> Result<NamespaceReclaimProgress, MapServiceError> {
        let mut state = self.state.lock().expect("local IMap state poisoned");
        if !state.namespace_fenced
            || maximum == 0
            || maximum > state.map.service_bounds().max_scan_items
        {
            return Err(MapServiceError::InvalidArgument);
        }
        let mut reclaimed = 0;
        let mut wakes = Vec::new();
        while reclaimed < maximum {
            let Some((_, listener)) = state.listeners.pop_first() else {
                break;
            };
            if let Some(waker) = listener.waker {
                wakes.push(waker);
            }
            reclaimed += 1;
        }
        reclaimed += state.map.service_reclaim(maximum - reclaimed);
        let owned = owners(&state);
        let progress = NamespaceReclaimProgress {
            reclaimed,
            remaining: owned.cleanable_records(),
            blocked_owners: owned.blocked_owners(),
        };
        drop(state);
        for wake in wakes {
            wake.wake();
        }
        Ok(progress)
    }

    /// Conservative extra owned payload for a registry admission. Replays reserve no new
    /// resources; listener growth is capped by the actual aggregate queue budget.
    pub fn preflight_registry_growth(
        &self,
        command: &ServiceCommand,
    ) -> Result<usize, MapServiceError> {
        let state = self.state.lock().expect("local IMap state poisoned");
        require_namespace(&state)?;
        validate(&state, command)?;
        let used = lifecycle::usage(&state);
        let mut growth = 0usize;
        if mutating(&command.operation) {
            let identity = command
                .identity
                .as_ref()
                .ok_or(MapServiceError::InvalidArgument)?;
            lifecycle::check_identity(&state, identity)?;
            if !matches!(
                state
                    .map
                    .service_lookup(identity, Some(command_digest(command)))?,
                ServiceRetainedOutcome::Unknown
            ) {
                return Ok(0);
            }
            let result = lifecycle::preflight_command(&state, command, identity)?;
            growth = result
                .saturating_add(2usize.saturating_mul(identity.client.len().saturating_add(48)));
            if let Some(value) = &command.value {
                growth = growth.saturating_add(
                    24usize
                        .saturating_add(command.key.len())
                        .saturating_add(value.len())
                        .saturating_sub(state.map.service_physical_entry_bytes(&command.key)),
                );
            }
            for entry in &command.entries {
                growth = growth.saturating_add(
                    24usize
                        .saturating_add(entry.key.len())
                        .saturating_add(entry.value.len())
                        .saturating_sub(state.map.service_physical_entry_bytes(&entry.key)),
                );
            }
        }
        if command.operation == "entry_listener" {
            growth = growth
                .saturating_add(128)
                .saturating_add(command.key.len())
                .saturating_add(command.expected.as_ref().map_or(0, Vec::len));
        }
        let mutation = mutating(&command.operation);
        let expiry = state.map.service_expiry_pending(command.now);
        if mutation || expiry {
            let bounds = state.map.service_bounds();
            let explicit = command.keys.len().max(command.entries.len()).max(1);
            let count = if command.operation == "evict_all_detailed" {
                state.map.service_counts(command.now).0
            } else if mutation {
                explicit
            } else {
                0
            };
            let key_bytes = if command.operation == "evict_all_detailed" {
                bounds.max_key_bytes
            } else {
                std::iter::once(command.key.len())
                    .chain(command.entries.iter().map(|entry| entry.key.len()))
                    .chain(command.keys.iter().map(Vec::len))
                    .max()
                    .unwrap_or(0)
            };
            let mutation_bytes = count.saturating_mul(
                64usize
                    .saturating_add(key_bytes)
                    .saturating_add(bounds.max_value_bytes.saturating_mul(2)),
            );
            // Maintenance and targeted expiry refer to stored keys, which can be much
            // larger than the incoming request's key (including an empty observation).
            let expiry_count = if expiry {
                state
                    .limits
                    .expiry_work
                    .saturating_add(if mutation { explicit } else { 0 })
            } else {
                0
            };
            let expiry_bytes = expiry_count.saturating_mul(
                64usize
                    .saturating_add(bounds.max_key_bytes)
                    .saturating_add(bounds.max_value_bytes),
            );
            let potential = mutation_bytes
                .saturating_add(expiry_bytes)
                .saturating_mul(state.listeners.len());
            growth = growth.saturating_add(
                potential.min(state.limits.listener_bytes.saturating_sub(used.listeners)),
            );
        }
        if growth == usize::MAX {
            return Err(MapServiceError::Capacity);
        }
        Ok(growth)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_payload_snapshot_leases_and_negotiated_value_bounds_are_finite() {
        let service = UnifiedIMapService::new(
            CollectionBounds {
                max_entries: 1,
                ..CollectionBounds::default()
            },
            1,
        )
        .unwrap();
        let listener = service
            .register_listener(ServiceListenerOptions::default())
            .unwrap();
        let snapshot = service.resync_listener(listener.id(), 0, 1, 1).unwrap();
        assert_eq!(service.namespace_owners().resync_leases, 1);
        assert_eq!(service.resource_usage().resync, 0);
        assert_eq!(
            service.resync_listener(listener.id(), 0, 1, 1).unwrap_err(),
            MapServiceError::Capacity
        );
        assert_eq!(
            service.export_replica_image().unwrap_err(),
            MapServiceError::Capacity
        );
        drop(snapshot);
        let image = service.export_replica_image().unwrap();
        assert_eq!(
            service.export_replica_image().unwrap_err(),
            MapServiceError::Capacity
        );
        drop(image);
        assert_eq!(service.namespace_owners().resync_leases, 0);
        let service = UnifiedIMapService::new(CollectionBounds::default(), 1).unwrap();
        let mut command = ServiceCommand::point("put", b"a");
        command.value = Some(b"large".to_vec());
        command.identity = Some(MutationIdentity::new("resync-bound", 1));
        service.execute(command).unwrap();
        let listener = service
            .register_listener(ServiceListenerOptions {
                include_value: true,
                max_value_bytes: 1,
                ..ServiceListenerOptions::default()
            })
            .unwrap();
        assert_eq!(
            service
                .preflight_listener_resync(listener.id(), 0, 1, 1_024)
                .unwrap_err(),
            MapServiceError::ResourceBudget("resync_value_bytes")
        );
        assert_eq!(
            service
                .resync_listener(listener.id(), 0, 1, 1_024)
                .unwrap_err(),
            MapServiceError::ResourceBudget("resync_value_bytes")
        );
        assert_eq!(service.namespace_owners().resync_leases, 0);
    }

    #[test]
    fn waiter_notifications_release_the_mutex_and_preserve_domain_state() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::task::{Context, Wake, Waker};
        struct Notice {
            service: UnifiedIMapService,
            wakes: AtomicUsize,
        }
        impl Wake for Notice {
            fn wake(self: Arc<Self>) {
                assert!(self.service.state.try_lock().is_ok());
                self.wakes.fetch_add(1, Ordering::SeqCst);
            }
        }
        let service = UnifiedIMapService::new(CollectionBounds::default(), 1).unwrap();
        let registration = service
            .register_listener(ServiceListenerOptions::default())
            .unwrap();
        let notice = Arc::new(Notice {
            service: service.clone(),
            wakes: AtomicUsize::new(0),
        });
        let waker = Waker::from(notice.clone());
        let mut context = Context::from_waker(&waker);
        assert!(service
            .poll_listener(registration.id(), 1, &mut context)
            .is_pending());
        let before = service.replica_stamp().unwrap();
        service.wake_listener_waiters();
        assert_eq!(notice.wakes.load(Ordering::SeqCst), 1);
        assert_eq!(service.replica_stamp().unwrap(), before);
        service.wake_listener_waiters();
        assert_eq!(notice.wakes.load(Ordering::SeqCst), 1);
        assert!(registration.drain().unwrap().is_empty());
    }

    #[test]
    fn registry_growth_covers_large_expired_keys_target_touches_and_eviction_fanout() {
        let options = ServiceListenerOptions {
            max_value_bytes: 1,
            ..ServiceListenerOptions::default()
        };
        let bounds = CollectionBounds {
            max_key_bytes: 1_024,
            max_value_bytes: 1,
            max_entries: 512,
            ..CollectionBounds::default()
        };
        let service = UnifiedIMapService::with_limits(
            bounds,
            1,
            ServiceLimits {
                expiry_work: 1,
                ..ServiceLimits::default()
            },
        )
        .unwrap();
        let key = vec![b'k'; 1_024];
        let mut put = ServiceCommand::point("put", &key);
        put.value = Some(b"v".to_vec());
        put.identity = Some(MutationIdentity::new("growth", 1));
        put.ttl = ServiceTtl::ExpireAfter(1);
        service.execute(put).unwrap();
        let registrations = (0..3)
            .map(|_| service.register_listener(options.clone()).unwrap())
            .collect::<Vec<_>>();
        let mut observation = ServiceCommand::point("get", b"");
        observation.now = 1;
        let before = lifecycle::usage(&service.state.lock().unwrap());
        let growth = service.preflight_registry_growth(&observation).unwrap();
        service.execute(observation).unwrap();
        let after = lifecycle::usage(&service.state.lock().unwrap());
        assert!(after.listeners.saturating_sub(before.listeners) <= growth);
        assert!(growth >= 3 * (64 + key.len()));
        drop(registrations);
        let targeted = UnifiedIMapService::with_limits(
            CollectionBounds {
                max_key_bytes: 128,
                max_value_bytes: 1,
                ..CollectionBounds::default()
            },
            1,
            ServiceLimits {
                expiry_work: 1,
                ..ServiceLimits::default()
            },
        )
        .unwrap();
        let keys = [b"a".to_vec(), vec![b'b'; 128], vec![b'c'; 128]];
        for (index, key) in keys.iter().enumerate() {
            let mut command = ServiceCommand::point("put", key);
            command.value = Some(b"v".to_vec());
            command.ttl = ServiceTtl::ExpireAfter(1);
            command.identity = Some(MutationIdentity::new("target", index as u64 + 1));
            targeted.execute(command).unwrap();
        }
        let listener = targeted.register_listener(options.clone()).unwrap();
        let mut command = ServiceCommand::point("put_all_detailed", b"");
        command.entries = keys[1..]
            .iter()
            .map(|key| ReplacementEntry {
                key: key.clone(),
                value: b"v".to_vec(),
            })
            .collect();
        command.identity = Some(MutationIdentity::new("target", 4));
        command.now = 1;
        let total = |usage: ServiceResourceUsage| {
            [
                usage.values,
                usage.outcomes,
                usage.pending,
                usage.listeners,
                usage.streams,
                usage.resync,
            ]
            .into_iter()
            .sum::<usize>()
        };
        let before = total(targeted.resource_usage());
        let growth = targeted.preflight_registry_growth(&command).unwrap();
        targeted.execute(command).unwrap();
        let after = total(targeted.resource_usage());
        assert!(after.saturating_sub(before) <= growth);
        assert_eq!(listener.drain().unwrap().len(), 5);
        let service = UnifiedIMapService::with_limits(
            CollectionBounds {
                max_key_bytes: 16,
                max_value_bytes: 1,
                max_entries: 512,
                ..CollectionBounds::default()
            },
            1,
            ServiceLimits {
                expiry_work: 1,
                ..ServiceLimits::default()
            },
        )
        .unwrap();
        for n in 0..100 {
            let mut command = ServiceCommand::point("put", format!("k{n}").as_bytes());
            command.value = Some(b"v".to_vec());
            command.identity = Some(MutationIdentity::new("eviction", n + 1));
            service.execute(command).unwrap();
        }
        let registration = service.register_listener(options).unwrap();
        let mut command = ServiceCommand::point("evict_all_detailed", b"");
        command.identity = Some(MutationIdentity::new("eviction", 101));
        let before = service.resource_usage();
        let growth = service.preflight_registry_growth(&command).unwrap();
        service.execute(command).unwrap();
        let after = service.resource_usage();
        assert!(after.listeners.saturating_sub(before.listeners) <= growth);
        assert_eq!(registration.drain().unwrap().len(), 100);
    }

    #[test]
    fn replica_integrity_and_sequence_exhaustion_fail_before_state_changes() {
        let owner = UnifiedIMapService::new(CollectionBounds::default(), 1).unwrap();
        let backup = UnifiedIMapService::new(CollectionBounds::default(), 1).unwrap();
        let mut command = ServiceCommand::point("put", b"a");
        command.identity = Some(MutationIdentity::new("counter", 1));
        command.value = Some(b"value".to_vec());
        owner.execute(command.clone()).unwrap();
        let mut corrupt = owner.export_replica_image().unwrap();
        corrupt
            .map
            .as_mut()
            .unwrap()
            .put(b"a".to_vec(), b"tampered".to_vec(), None, 0)
            .unwrap();
        assert_eq!(
            backup.install_replica_image(corrupt).unwrap_err(),
            MapServiceError::InvalidArgument
        );
        assert!(backup.diagnostic_snapshot(0).unwrap().is_empty());
        assert_eq!(owner.namespace_owners().resync_leases, 0);
        let current = owner.replica_stamp().unwrap();
        {
            let mut state = owner.state.lock().unwrap();
            state.replica_sequence = u64::MAX;
            state.replica_digest = Some(current.digest);
        }
        let before = owner.diagnostic_snapshot(0).unwrap();
        let mut next = command.clone();
        next.identity = Some(MutationIdentity::new("counter", 2));
        next.value = Some(b"new".to_vec());
        assert_eq!(
            owner.execute(next.clone()).unwrap_err(),
            MapServiceError::Capacity
        );
        assert_eq!(
            owner
                .reserve_operation(next.identity.clone().unwrap(), &next)
                .unwrap_err(),
            MapServiceError::Capacity
        );
        assert_eq!(
            owner.retire_outcomes("counter", 1, 1).unwrap_err(),
            MapServiceError::Capacity
        );
        assert_eq!(
            owner
                .admit_external_operation(MutationIdentity::new("admin", 1), [1; 32], 64)
                .unwrap_err(),
            MapServiceError::Capacity
        );
        let mut future = ServiceCommand::point("get", b"a");
        future.now = 1;
        assert_eq!(
            owner.execute(future).unwrap_err(),
            MapServiceError::Capacity
        );
        assert_eq!(owner.logical_time(), 0);
        assert_eq!(owner.diagnostic_snapshot(0).unwrap(), before);
        assert_eq!(owner.execute(command).unwrap().kind, "inserted");
        assert_eq!(owner.replica_stamp().unwrap().sequence, u64::MAX);
        {
            let mut state = owner.state.lock().unwrap();
            state.replica_digest = Some([0; 32]);
        }
        assert_eq!(
            owner.replica_stamp().unwrap_err(),
            MapServiceError::Capacity
        );
        assert_eq!(owner.state.lock().unwrap().replica_digest, Some([0; 32]));
    }
}
