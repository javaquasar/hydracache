//! Sparse, runtime-only installed decisions. A cursor owns keys and fingerprints, never values.
use super::cluster_hooks::{current_stamp, BoundedDebug, CountingDebug};
use super::lifecycle::{budget, require_namespace, ByteLease};
use super::*;
use crate::value_plane_collection_075::{
    CollectionDeltaMeasure, CollectionReplicaCursor, CollectionReplicaPatch,
};

#[derive(Debug)]
pub struct ServiceReplicaCursor {
    base: ReplicaStamp,
    records: CollectionReplicaCursor,
    lease: ByteLease,
}
impl ServiceReplicaCursor {
    pub const fn stamp(&self) -> ReplicaStamp {
        self.base
    }
    pub fn owned_payload_bytes(&self) -> usize {
        self.lease.bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplicaDeltaMeasure {
    pub base: ReplicaStamp,
    pub target: ReplicaStamp,
    pub owned_bytes: usize,
    pub payload_bytes: usize,
    pub cursor_bytes: usize,
    pub cursor_growth: usize,
    pub entry_upserts: usize,
    pub entry_deletions: usize,
    pub outcome_upserts: usize,
    pub outcome_deletions: usize,
    pub fence_upserts: usize,
    pub fence_deletions: usize,
}

#[derive(Debug)]
pub struct ServiceReplicaDelta {
    base: ReplicaStamp,
    target: ReplicaStamp,
    patch: Option<CollectionReplicaPatch>,
    now: u64,
    watermark: u64,
    measure: ReplicaDeltaMeasure,
    lease: ByteLease,
}
impl ServiceReplicaDelta {
    pub const fn base(&self) -> ReplicaStamp {
        self.base
    }
    pub const fn target(&self) -> ReplicaStamp {
        self.target
    }
    pub const fn measure(&self) -> ReplicaDeltaMeasure {
        self.measure
    }
    pub fn owned_payload_bytes(&self) -> usize {
        self.lease.bytes
    }
    pub fn transport_payload(&self, maximum: usize) -> Result<Vec<u8>, MapServiceError> {
        let mut writer = BoundedDebug {
            bytes: Vec::new(),
            maximum,
        };
        fmt::Write::write_fmt(
            &mut writer,
            format_args!(
                "{:?}:{:?}:{}:{}:{:?}",
                self.base,
                self.target,
                self.now,
                self.watermark,
                self.patch
                    .as_ref()
                    .ok_or(MapServiceError::InvalidArgument)?
            ),
        )
        .map_err(|_| MapServiceError::ResourceBudget("transport_payload"))?;
        Ok(writer.bytes)
    }
}

impl ServiceReplicaImage {
    /// Build a bounded metadata cursor at the image's exact cut, even if its owner advanced.
    pub fn replication_cursor(&self) -> Result<ServiceReplicaCursor, MapServiceError> {
        let map = self.map.as_ref().ok_or(MapServiceError::InvalidArgument)?;
        // Pending decisions are replicated only through the explicit journal image port.
        // Sparse streams start from a quiescent cut and never replace a live reservation.
        if map.service_pending_count() != 0 {
            return Err(MapServiceError::ActiveOwners);
        }
        let owner = self
            .lease
            .service
            .upgrade()
            .ok_or(MapServiceError::NamespaceDeleted)?;
        let mut state = owner.lock().expect("local IMap state poisoned");
        require_namespace(&state)?;
        let bytes = map.service_cursor_bytes();
        check_lease(&state, bytes, 1)?;
        state.resync_bytes += bytes;
        state.resync_leases += 1;
        Ok(ServiceReplicaCursor {
            base: self.stamp,
            records: map.service_replica_cursor(),
            lease: ByteLease {
                service: Arc::downgrade(&owner),
                bytes,
                active: true,
            },
        })
    }
}

fn check_lease(state: &ServiceState, bytes: usize, slots: usize) -> Result<(), MapServiceError> {
    if state
        .resync_leases
        .checked_add(slots)
        .is_none_or(|n| n > state.map.service_bounds().max_entries)
    {
        return Err(MapServiceError::Capacity);
    }
    budget(
        "resync",
        state.limits.resync_bytes,
        state
            .resync_bytes
            .checked_add(bytes)
            .ok_or(MapServiceError::Capacity)?,
    )
}

fn measure_delta(
    state: &mut ServiceState,
    cursor: &ServiceReplicaCursor,
    maximum: usize,
) -> Result<ReplicaDeltaMeasure, MapServiceError> {
    require_namespace(state)?;
    if state.map.service_pending_count() != 0 {
        return Err(MapServiceError::ActiveOwners);
    }
    let target = current_stamp(state)?;
    if cursor.base.map_id != target.map_id
        || cursor.base.namespace_generation != target.namespace_generation
        || cursor.base.sequence > target.sequence
    {
        return Err(MapServiceError::InvalidArgument);
    }
    let CollectionDeltaMeasure {
        owned_bytes,
        cursor_bytes,
        entry_upserts,
        entry_deletions,
        outcome_upserts,
        outcome_deletions,
        fence_upserts,
        fence_deletions,
    } = state.map.service_delta_measure(&cursor.records);
    let mut writer = CountingDebug { bytes: 0, maximum };
    fmt::Write::write_fmt(
        &mut writer,
        format_args!(
            "{:?}:{:?}:{}:{}:{:?}",
            cursor.base,
            target,
            state.now,
            state.watermark,
            state.map.service_delta_debug(&cursor.records)
        ),
    )
    .map_err(|_| MapServiceError::ResourceBudget("transport_payload"))?;
    let cursor_growth = cursor_bytes.saturating_sub(cursor.lease.bytes);
    check_lease(
        state,
        owned_bytes
            .checked_add(cursor_growth)
            .ok_or(MapServiceError::Capacity)?,
        1,
    )?;
    Ok(ReplicaDeltaMeasure {
        base: cursor.base,
        target,
        owned_bytes,
        payload_bytes: writer.bytes,
        cursor_bytes,
        cursor_growth,
        entry_upserts,
        entry_deletions,
        outcome_upserts,
        outcome_deletions,
        fence_upserts,
        fence_deletions,
    })
}

impl UnifiedIMapService {
    pub fn retirement_delta_credit(
        &self,
        client: &str,
        through: u64,
    ) -> Result<usize, MapServiceError> {
        let state = self.state.lock().expect("local IMap state poisoned");
        require_namespace(&state)?;
        let bytes = state.map.service_retirement_delta_bytes(client, through);
        if bytes == 0 {
            return Ok(0);
        }
        bytes
            .checked_mul(8)
            .and_then(|n| n.checked_add(2_048))
            .ok_or(MapServiceError::Capacity)
    }
    /// Conservative sparse decision credit, measured without cloning values or advancing time.
    pub fn command_delta_credit(
        &self,
        command: &ServiceCommand,
        maximum: usize,
    ) -> Result<usize, MapServiceError> {
        let state = self.state.lock().expect("local IMap state poisoned");
        require_namespace(&state)?;
        validate(&state, command)?;
        let mutation = mutating(&command.operation);
        if mutation {
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
            ) || command.cancelled
                || command.now >= command.deadline
            {
                return Ok(0);
            }
        } else if command.now == state.now && !state.map.service_expiry_pending(command.now) {
            return Ok(0);
        }
        let request = command.test_transport_size(maximum)?;
        let response = if mutation {
            lifecycle::preflight_command(
                &state,
                command,
                command
                    .identity
                    .as_ref()
                    .expect("validated mutation identity"),
            )?
        } else {
            0
        };
        let expired_keys = state.map.service_expiry_delta_key_bytes(
            command.now,
            state.limits.expiry_work,
            command,
        );
        let map_wide = if matches!(
            command.operation.as_str(),
            "destroy" | "clear_detailed" | "evict_all_detailed"
        ) {
            let used = lifecycle::usage(&state);
            used.values.saturating_add(used.outcomes)
        } else {
            0
        };
        let credit = 2_048usize
            .checked_add(request.checked_mul(2).ok_or(MapServiceError::Capacity)?)
            .and_then(|n| n.checked_add(response.checked_mul(16)?))
            .and_then(|n| n.checked_add(expired_keys.checked_mul(8)?))
            .and_then(|n| n.checked_add(map_wide.checked_mul(16)?))
            .ok_or(MapServiceError::Capacity)?;
        Ok(credit)
    }
    /// Check both image and metadata-cursor leases before either snapshot allocation.
    pub fn preflight_replica_snapshot_with_cursor(
        &self,
    ) -> Result<(usize, usize), MapServiceError> {
        let state = self.state.lock().expect("local IMap state poisoned");
        require_namespace(&state)?;
        if state.map.service_pending_count() != 0 {
            return Err(MapServiceError::ActiveOwners);
        }
        let used = lifecycle::usage(&state);
        let image = used
            .values
            .checked_add(used.outcomes)
            .and_then(|n| n.checked_add(256))
            .ok_or(MapServiceError::Capacity)?;
        let cursor = state.map.service_cursor_bytes();
        check_lease(
            &state,
            image.checked_add(cursor).ok_or(MapServiceError::Capacity)?,
            2,
        )?;
        Ok((image, cursor))
    }

    pub fn preflight_replica_delta(
        &self,
        cursor: &ServiceReplicaCursor,
        maximum_payload: usize,
    ) -> Result<ReplicaDeltaMeasure, MapServiceError> {
        if !cursor.lease.active
            || !Weak::ptr_eq(&cursor.lease.service, &Arc::downgrade(&self.state))
        {
            return Err(MapServiceError::InvalidArgument);
        }
        let mut state = self.state.lock().expect("local IMap state poisoned");
        measure_delta(&mut state, cursor, maximum_payload)
    }

    pub fn export_replica_delta(
        &self,
        cursor: &mut ServiceReplicaCursor,
        maximum_payload: usize,
    ) -> Result<ServiceReplicaDelta, MapServiceError> {
        if !cursor.lease.active
            || !Weak::ptr_eq(&cursor.lease.service, &Arc::downgrade(&self.state))
        {
            return Err(MapServiceError::InvalidArgument);
        }
        let mut state = self.state.lock().expect("local IMap state poisoned");
        let measure = measure_delta(&mut state, cursor, maximum_payload)?;
        if measure.base == measure.target {
            return Err(MapServiceError::InvalidArgument);
        }
        // Allocation begins only after both the sparse patch and metadata growth are admitted.
        let patch = state.map.service_capture_delta(&mut cursor.records);
        state.resync_bytes =
            state.resync_bytes - cursor.lease.bytes + measure.cursor_bytes + measure.owned_bytes;
        state.resync_leases += 1;
        cursor.lease.bytes = measure.cursor_bytes;
        cursor.base = measure.target;
        Ok(ServiceReplicaDelta {
            base: measure.base,
            target: measure.target,
            patch: Some(patch),
            now: state.now,
            watermark: state.watermark,
            measure,
            lease: ByteLease {
                service: Arc::downgrade(&self.state),
                bytes: measure.owned_bytes,
                active: true,
            },
        })
    }

    /// Install exact records, tombstone decisions and retained receipts; never execute a request.
    pub fn install_replica_delta(
        &self,
        mut delta: ServiceReplicaDelta,
    ) -> Result<ReplicaStamp, MapServiceError> {
        let mut state = self.state.lock().expect("local IMap state poisoned");
        require_namespace(&state)?;
        if state.map.service_pending_count() != 0
            || state.active_streams != 0
            || state.resync_leases != 0
            || !state.listeners.is_empty()
        {
            return Err(MapServiceError::ActiveOwners);
        }
        let current = current_stamp(&mut state)?;
        if current != delta.base
            || delta.target.sequence <= current.sequence
            || delta.target.map_id != current.map_id
            || delta.target.namespace_generation != current.namespace_generation
            || delta.now < state.now
            || delta.watermark < state.watermark
        {
            return Err(MapServiceError::InvalidArgument);
        }
        let patch = delta
            .patch
            .as_ref()
            .ok_or(MapServiceError::InvalidArgument)?;
        if patch.revision() != delta.target.revision {
            return Err(MapServiceError::InvalidArgument);
        }
        state.map.service_validate_delta(patch)?;
        let counts = state.map.service_projected_counts(patch);
        budget("values", state.limits.value_bytes, counts.3)?;
        budget("outcomes", state.limits.outcome_bytes, counts.4)?;
        budget("pending", state.limits.pending_bytes, counts.5)?;
        budget("retirement_clients", state.limits.max_clients, counts.2)?;
        struct HashWriter(Sha256);
        impl fmt::Write for HashWriter {
            fn write_str(&mut self, value: &str) -> fmt::Result {
                self.0.update(value.as_bytes());
                Ok(())
            }
        }
        let mut writer = HashWriter(Sha256::new());
        fmt::Write::write_fmt(
            &mut writer,
            format_args!(
                "{}:{}:{}:{:?}",
                state.namespace_generation,
                delta.now,
                delta.watermark,
                state.map.service_projected_debug(patch)
            ),
        )
        .expect("hash writer");
        let digest: [u8; 32] = writer.0.finalize().into();
        if digest != delta.target.digest {
            return Err(MapServiceError::InvalidArgument);
        }
        state
            .map
            .service_install_delta(delta.patch.take().expect("validated sparse decisions"));
        state.now = delta.now;
        state.watermark = delta.watermark;
        state.replica_sequence = delta.target.sequence;
        state.replica_digest = Some(digest);
        let result = delta.target;
        drop(state);
        delta.lease.release();
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(
        service: &UnifiedIMapService,
        key: &[u8],
        value: &[u8],
        sequence: u64,
    ) -> ServiceResult {
        let mut command = ServiceCommand::point("put", key);
        command.value = Some(value.to_vec());
        command.identity = Some(MutationIdentity::new("delta", sequence));
        service.execute(command).unwrap()
    }
    fn pair() -> (UnifiedIMapService, UnifiedIMapService, ServiceReplicaCursor) {
        let owner = UnifiedIMapService::new(CollectionBounds::default(), 9).unwrap();
        let backup = UnifiedIMapService::new(CollectionBounds::default(), 9).unwrap();
        write(&owner, b"large", &vec![197; 16 * 1_024], 1);
        let image = owner.export_replica_image().unwrap();
        let cursor = image.replication_cursor().unwrap();
        backup.install_replica_image(image).unwrap();
        (owner, backup, cursor)
    }

    #[test]
    fn sparse_decision_bounds_are_checked_before_cursor_or_payload_allocation() {
        let (owner, backup, mut cursor) = pair();
        write(&owner, b"small", b"new", 2);
        let base = cursor.stamp();
        let before = owner.resource_usage();
        assert!(matches!(
            owner.export_replica_delta(&mut cursor, 1),
            Err(MapServiceError::ResourceBudget("transport_payload"))
        ));
        assert_eq!(cursor.stamp(), base);
        assert_eq!(owner.resource_usage(), before);
        let measure = owner.preflight_replica_delta(&cursor, 32 * 1_024).unwrap();
        assert_eq!(measure.entry_upserts, 1);
        assert!(measure.owned_bytes < 1_024);
        assert!(measure.cursor_bytes < 1_024);
        let delta = owner
            .export_replica_delta(&mut cursor, measure.payload_bytes)
            .unwrap();
        assert_eq!(
            delta
                .transport_payload(measure.payload_bytes)
                .unwrap()
                .len(),
            measure.payload_bytes
        );
        assert_eq!(
            backup.install_replica_delta(delta).unwrap(),
            owner.replica_stamp().unwrap()
        );
        assert_eq!(
            backup.diagnostic_snapshot(0).unwrap(),
            owner.diagnostic_snapshot(0).unwrap()
        );
        drop(cursor);
        assert_eq!(owner.resource_usage().resync, 0);
    }

    #[test]
    fn sparse_decisions_reject_wrong_base_target_digest_and_revision_without_effects() {
        for fault in 0..3 {
            let (owner, backup, mut cursor) = pair();
            write(&owner, b"small", b"new", 2);
            let mut delta = owner.export_replica_delta(&mut cursor, 32 * 1_024).unwrap();
            match fault {
                0 => delta.base.sequence += 1,
                1 => delta.target.digest[0] ^= 1,
                _ => delta.target.revision += 1,
            }
            let before = backup.replica_stamp().unwrap();
            let values = backup.diagnostic_snapshot(0).unwrap();
            assert_eq!(
                backup.install_replica_delta(delta).unwrap_err(),
                MapServiceError::InvalidArgument
            );
            assert_eq!(backup.replica_stamp().unwrap(), before);
            assert_eq!(backup.diagnostic_snapshot(0).unwrap(), values);
            drop(cursor);
            assert_eq!(owner.resource_usage().resync, 0);
        }
    }

    #[test]
    fn sparse_deletions_outcomes_fences_and_absolute_ttl_install_without_reexecution() {
        let (owner, backup, mut cursor) = pair();
        let mut put = ServiceCommand::point("put", b"ttl");
        put.value = Some(b"kept".to_vec());
        put.ttl = ServiceTtl::ExpireAfter(20);
        put.identity = Some(MutationIdentity::new("delta", 2));
        let original = owner.execute(put.clone()).unwrap();
        let delta = owner.export_replica_delta(&mut cursor, 32 * 1_024).unwrap();
        backup.install_replica_delta(delta).unwrap();
        assert_eq!(backup.execute(put).unwrap(), original);
        assert_eq!(
            backup
                .diagnostic_snapshot(5)
                .unwrap()
                .iter()
                .find(|entry| entry.key == b"ttl")
                .unwrap()
                .remaining_ttl,
            Some(15)
        );
        let mut delete = ServiceCommand::point("delete", b"large");
        delete.identity = Some(MutationIdentity::new("delta", 3));
        owner.execute(delete).unwrap();
        owner.retire_outcomes("delta", 9, 1).unwrap();
        let measure = owner.preflight_replica_delta(&cursor, 32 * 1_024).unwrap();
        assert_eq!(
            (
                measure.entry_deletions,
                measure.outcome_upserts,
                measure.outcome_deletions,
                measure.fence_upserts
            ),
            (1, 1, 1, 1)
        );
        let delta = owner.export_replica_delta(&mut cursor, 32 * 1_024).unwrap();
        backup.install_replica_delta(delta).unwrap();
        assert_eq!(
            backup.replica_stamp().unwrap(),
            owner.replica_stamp().unwrap()
        );
        assert_eq!(
            backup
                .query_operation_outcome(&MutationIdentity::new("delta", 3))
                .unwrap(),
            owner
                .query_operation_outcome(&MutationIdentity::new("delta", 3))
                .unwrap()
        );
        let mut retry = ServiceCommand::point("put", b"large");
        retry.value = Some(b"retry".to_vec());
        retry.identity = Some(MutationIdentity::new("delta", 1));
        assert_eq!(
            backup.execute(retry).unwrap_err(),
            MapServiceError::RetiredIdentity
        );
    }

    #[test]
    fn pending_journal_images_preserve_reservation_credit_and_never_admit_data_images() {
        let owner = UnifiedIMapService::new(CollectionBounds::default(), 2).unwrap();
        let backup = UnifiedIMapService::new(CollectionBounds::default(), 2).unwrap();
        let identity = MutationIdentity::new("journal", 1);
        owner
            .admit_external_operation(identity.clone(), [7; 32], 128)
            .unwrap();
        assert_eq!(
            owner.replica_stamp().unwrap_err(),
            MapServiceError::ActiveOwners
        );
        assert_eq!(
            owner.export_replica_image().unwrap_err(),
            MapServiceError::ActiveOwners
        );
        let stamp = owner.journal_replica_stamp().unwrap();
        let pending_bytes = owner.resource_usage();
        let image = owner.export_journal_image().unwrap();
        let held = owner.resource_usage();
        assert_eq!(
            image.replication_cursor().unwrap_err(),
            MapServiceError::ActiveOwners
        );
        assert_eq!(owner.resource_usage(), held);
        assert_eq!(
            image.owned_payload_bytes(),
            pending_bytes.values + pending_bytes.outcomes + pending_bytes.pending + 256
        );
        assert_eq!(backup.install_journal_image(image).unwrap(), stamp);
        assert_eq!(
            backup.query_operation_outcome(&identity).unwrap(),
            OperationOutcomeQuery::Pending
        );
        assert_eq!(
            backup.resource_usage().pending,
            owner.resource_usage().pending
        );
        assert_eq!(
            backup.checkpoint().unwrap_err(),
            MapServiceError::ActiveOwners
        );
        let result = ServiceResult::new("applied");
        owner
            .complete_external_operation(&identity, [7; 32], &result)
            .unwrap();
        backup
            .install_journal_image(owner.export_journal_image().unwrap())
            .unwrap();
        assert_eq!(
            backup.query_operation_outcome(&identity).unwrap(),
            OperationOutcomeQuery::Completed(Box::new(result))
        );
        assert_eq!(backup.resource_usage().pending, 0);
    }

    #[test]
    fn journal_image_cannot_overwrite_another_incarnations_pending_reservation() {
        let incoming = UnifiedIMapService::new(CollectionBounds::default(), 2).unwrap();
        let target = UnifiedIMapService::new(CollectionBounds::default(), 2).unwrap();
        let identity = MutationIdentity::new("occupied", 1);
        target
            .admit_external_operation(identity.clone(), [3; 32], 128)
            .unwrap();
        let before = target.journal_replica_stamp().unwrap();
        let usage = target.resource_usage();
        assert_eq!(
            target
                .install_journal_image(incoming.export_journal_image().unwrap())
                .unwrap_err(),
            MapServiceError::InvalidArgument
        );
        assert_eq!(target.journal_replica_stamp().unwrap(), before);
        assert_eq!(target.resource_usage(), usage);
        assert_eq!(
            target.query_operation_outcome(&identity).unwrap(),
            OperationOutcomeQuery::Pending
        );
    }

    #[test]
    fn full_decision_image_respects_the_receivers_retirement_client_bound() {
        let owner = UnifiedIMapService::new(CollectionBounds::default(), 2).unwrap();
        owner.retire_outcomes("one", 2, 1).unwrap();
        owner.retire_outcomes("two", 2, 1).unwrap();
        for journal in [false, true] {
            let target = UnifiedIMapService::with_limits(
                CollectionBounds::default(),
                2,
                ServiceLimits {
                    max_clients: 1,
                    ..ServiceLimits::default()
                },
            )
            .unwrap();
            let before = target.replica_stamp().unwrap();
            let usage = target.resource_usage();
            let image = owner.export_replica_image().unwrap();
            let result = if journal {
                target.install_journal_image(image)
            } else {
                target.install_replica_image(image)
            };
            assert_eq!(
                result.unwrap_err(),
                MapServiceError::ResourceBudget("retirement_clients")
            );
            assert_eq!(target.replica_stamp().unwrap(), before);
            assert_eq!(target.resource_usage(), usage);
            assert_eq!(owner.resource_usage().resync, 0);
        }
    }
}
