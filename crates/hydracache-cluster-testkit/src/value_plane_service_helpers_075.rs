//! Typed local helpers share the collection's original retained ledger and resource owners.
use super::lifecycle::{budget, check_identity, require_namespace, ByteLease};
use super::*;

#[derive(Debug)]
pub enum LocalHelperCommand {
    VersionBulk {
        operation: VersionBulkOperation,
        entries: Vec<VersionBulkEntry>,
    },
    QueryOutcomes {
        identities: Vec<MutationIdentity>,
        max_bytes: usize,
    },
    ListenerSnapshot {
        listener_id: u64,
        max_items: usize,
        max_bytes: usize,
    },
    RetireOutcomes {
        client: String,
        generation: u64,
        through: u64,
    },
    ExpireStep {
        max_work: usize,
    },
    Checkpoint,
}
#[derive(Debug)]
pub enum LocalHelperResult {
    Versions(ServiceResult),
    Outcomes(Vec<OperationOutcomeQuery>),
    Snapshot(ListenerSnapshot),
    Retirement(RetirementReceipt),
    Expiry(ExpiryProgress),
    Checkpoint(ServiceCheckpoint),
}

/// Request and maximum-response credit survives until its invocation future is dropped.
#[derive(Debug)]
pub struct LocalHelperCredit {
    owner: Weak<Mutex<ServiceState>>,
    bytes: usize,
}
impl Drop for LocalHelperCredit {
    fn drop(&mut self) {
        if let Some(owner) = self.owner.upgrade() {
            let mut state = owner.lock().expect("local IMap state poisoned");
            state.helper_bytes = state.helper_bytes.saturating_sub(self.bytes);
            state.helper_leases = state.helper_leases.saturating_sub(1);
        }
    }
}

fn helper_bound(
    state: &ServiceState,
    command: &LocalHelperCommand,
) -> Result<(usize, usize), MapServiceError> {
    require_namespace(state)?;
    let bounds = state.map.service_bounds();
    let mut request = 304usize;
    let response = match command {
        LocalHelperCommand::VersionBulk { operation, entries } => {
            if entries.is_empty() || entries.len() > state.limits.max_batch {
                return Err(MapServiceError::InvalidArgument);
            }
            let mut seen = BTreeSet::new();
            let mut response = 64usize;
            for entry in entries {
                if !seen.insert(&entry.key)
                    || (*operation == VersionBulkOperation::Replace) != entry.value.is_some()
                    || (*operation == VersionBulkOperation::SetTtl
                        && entry.ttl == ServiceTtl::Preserve)
                    || matches!(entry.ttl, ServiceTtl::ExpireAfter(0))
                {
                    return Err(MapServiceError::InvalidArgument);
                }
                check_bound("key_bytes", bounds.max_key_bytes, entry.key.len())?;
                check_bound(
                    "value_bytes",
                    bounds.max_value_bytes,
                    entry.value.as_ref().map_or(0, Vec::len),
                )?;
                request = request
                    .checked_add(64)
                    .and_then(|n| n.checked_add(entry.key.len()))
                    .and_then(|n| n.checked_add(entry.value.as_ref().map_or(0, Vec::len)))
                    .ok_or(MapServiceError::Capacity)?;
                response = response
                    .checked_add(64)
                    .and_then(|n| n.checked_add(entry.key.len()))
                    .ok_or(MapServiceError::Capacity)?;
            }
            check_bound("request_bytes", 8 * 1_024 * 1_024, request - 304)?;
            response
        }
        LocalHelperCommand::QueryOutcomes {
            identities,
            max_bytes,
        } => {
            if identities.is_empty()
                || identities.len() > state.limits.max_batch
                || *max_bytes == 0
                || *max_bytes > state.limits.query_bytes
            {
                return Err(MapServiceError::InvalidArgument);
            }
            for id in identities {
                state.map.service_query_result_bytes(id)?;
                request = request
                    .checked_add(48)
                    .and_then(|n| n.checked_add(id.client.len()))
                    .ok_or(MapServiceError::Capacity)?;
            }
            *max_bytes
        }
        LocalHelperCommand::ListenerSnapshot {
            max_items,
            max_bytes,
            ..
        } => {
            if *max_items == 0
                || *max_items > bounds.max_entries
                || *max_bytes == 0
                || *max_bytes > state.limits.resync_bytes
            {
                return Err(MapServiceError::InvalidArgument);
            }
            max_items
                .checked_mul(32)
                .and_then(|n| n.checked_add(*max_bytes))
                .and_then(|n| n.checked_add(64))
                .ok_or(MapServiceError::Capacity)?
        }
        LocalHelperCommand::RetireOutcomes {
            client,
            generation,
            through,
        } => {
            if client.trim().is_empty()
                || client.len() > 128
                || *generation != state.namespace_generation
                || *through == 0
            {
                return Err(MapServiceError::InvalidArgument);
            }
            request += client.len();
            64
        }
        LocalHelperCommand::ExpireStep { max_work } => {
            if *max_work == 0 || *max_work > bounds.max_scan_items {
                return Err(MapServiceError::InvalidArgument);
            }
            64
        }
        LocalHelperCommand::Checkpoint => {
            let used = lifecycle::usage(state);
            used.values
                .checked_add(used.outcomes)
                .and_then(|n| n.checked_add(256))
                .ok_or(MapServiceError::Capacity)?
        }
    };
    Ok((request, response))
}

fn helper_digest(command: &LocalHelperCommand) -> [u8; 32] {
    if let LocalHelperCommand::VersionBulk { operation, entries } = command {
        return lifecycle::version_digest(*operation, entries);
    }
    // All variable input has been bounded before this allocation; clock/deadline are absent.
    Sha256::digest(format!("local_helper:{command:?}").as_bytes()).into()
}

fn mutating_helper(command: &LocalHelperCommand) -> bool {
    !matches!(
        command,
        LocalHelperCommand::QueryOutcomes { .. } | LocalHelperCommand::Checkpoint
    )
}

fn recover_projection(
    state: &mut ServiceState,
    source: Weak<Mutex<ServiceState>>,
    command: &LocalHelperCommand,
    result: ServiceResult,
) -> Result<LocalHelperResult, MapServiceError> {
    Ok(match command {
        LocalHelperCommand::VersionBulk { .. } => LocalHelperResult::Versions(result),
        LocalHelperCommand::RetireOutcomes { through, .. } if result.kind == "helper_retired" => {
            LocalHelperResult::Retirement(RetirementReceipt {
                removed: result
                    .number
                    .try_into()
                    .map_err(|_| MapServiceError::Capacity)?,
                through: *through,
            })
        }
        LocalHelperCommand::ExpireStep { .. } if result.kind == "helper_expiry" => {
            LocalHelperResult::Expiry(ExpiryProgress {
                examined: result
                    .number
                    .try_into()
                    .map_err(|_| MapServiceError::Capacity)?,
                expired: result
                    .version
                    .ok_or(MapServiceError::InvalidArgument)?
                    .try_into()
                    .map_err(|_| MapServiceError::Capacity)?,
                pending: result.flag,
            })
        }
        LocalHelperCommand::ListenerSnapshot { .. }
            if result.kind == "helper_listener_snapshot" =>
        {
            let token = result.token.ok_or(MapServiceError::InvalidArgument)?;
            let bytes = result.items.iter().try_fold(0usize, |sum, item| {
                sum.checked_add(32)
                    .and_then(|n| n.checked_add(item.key.len()))
                    .and_then(|n| n.checked_add(item.value.as_ref().map_or(0, Vec::len)))
                    .ok_or(MapServiceError::Capacity)
            })?;
            if state.resync_leases >= state.map.service_bounds().max_entries {
                return Err(MapServiceError::Capacity);
            }
            budget(
                "resync",
                state.limits.resync_bytes,
                state
                    .resync_bytes
                    .checked_add(bytes)
                    .ok_or(MapServiceError::Capacity)?,
            )?;
            state.resync_bytes += bytes;
            state.resync_leases += 1;
            LocalHelperResult::Snapshot(ListenerSnapshot {
                entries: result
                    .items
                    .into_iter()
                    .map(|item| ServiceEntry {
                        key: item.key,
                        value: item.value.unwrap_or_default(),
                        version: item.version.unwrap_or(0),
                        remaining_ttl: item.remaining_ttl,
                    })
                    .collect(),
                watermark: result.number,
                namespace_generation: token.namespace_generation,
                map_id: token.map_id,
                revision: token.version,
                include_value: result.flag,
                lease: ByteLease {
                    service: source,
                    bytes,
                    active: true,
                },
            })
        }
        _ => return Err(MapServiceError::InvalidArgument),
    })
}

impl UnifiedIMapService {
    pub fn reserve_local_helper_credit(
        &self,
        command: &LocalHelperCommand,
    ) -> Result<LocalHelperCredit, MapServiceError> {
        let mut state = self.state.lock().expect("local IMap state poisoned");
        let (request, response) = helper_bound(&state, command)?;
        let bytes = request
            .checked_add(response)
            .ok_or(MapServiceError::Capacity)?;
        if state.helper_leases >= state.map.service_bounds().max_entries {
            return Err(MapServiceError::Capacity);
        }
        budget(
            "streams",
            state.limits.stream_bytes,
            lifecycle::usage(&state)
                .streams
                .checked_add(bytes)
                .ok_or(MapServiceError::Capacity)?,
        )?;
        state.helper_bytes += bytes;
        state.helper_leases += 1;
        Ok(LocalHelperCredit {
            owner: Arc::downgrade(&self.state),
            bytes,
        })
    }

    pub(crate) fn recover_local_helper(
        &self,
        command: &LocalHelperCommand,
        identity: &MutationIdentity,
    ) -> Result<crate::value_plane_async_075::BackendRecovery<LocalHelperResult>, MapServiceError>
    {
        use crate::value_plane_async_075::BackendRecovery;
        let mut state = self.state.lock().expect("local IMap state poisoned");
        helper_bound(&state, command)?;
        if !mutating_helper(command) {
            return Ok(BackendRecovery::NotRetained);
        }
        check_identity(&state, identity)?;
        Ok(
            match state
                .map
                .service_lookup(identity, Some(helper_digest(command)))?
            {
                ServiceRetainedOutcome::Unknown => BackendRecovery::NotRetained,
                ServiceRetainedOutcome::Pending => BackendRecovery::Unavailable,
                ServiceRetainedOutcome::Completed(value) => {
                    BackendRecovery::Retained(recover_projection(
                        &mut state,
                        Arc::downgrade(&self.state),
                        command,
                        ServiceResult::from_retained(&value)?,
                    )?)
                }
            },
        )
    }

    pub(crate) fn execute_local_helper(
        &self,
        command: &LocalHelperCommand,
        identity: MutationIdentity,
        now: u64,
        deadline: u64,
    ) -> Result<LocalHelperResult, MapServiceError> {
        match command {
            LocalHelperCommand::QueryOutcomes {
                identities,
                max_bytes,
            } => {
                return self
                    .query_operation_outcomes(identities, *max_bytes)
                    .map(LocalHelperResult::Outcomes)
            }
            LocalHelperCommand::Checkpoint => {
                return self.checkpoint().map(LocalHelperResult::Checkpoint)
            }
            _ => {}
        }
        let mut state = self.state.lock().expect("local IMap state poisoned");
        let result = (|| {
            let (_, response) = helper_bound(&state, command)?;
            if let LocalHelperCommand::VersionBulk { operation, entries } = command {
                return lifecycle::bulk_version(
                    &mut state, *operation, entries, identity, now, deadline, false,
                )
                .map(LocalHelperResult::Versions);
            }
            check_identity(&state, &identity)?;
            let digest = helper_digest(command);
            match state.map.service_lookup(&identity, Some(digest))? {
                ServiceRetainedOutcome::Completed(value) => {
                    return recover_projection(
                        &mut state,
                        Arc::downgrade(&self.state),
                        command,
                        ServiceResult::from_retained(&value)?,
                    )
                }
                ServiceRetainedOutcome::Pending => return Err(MapServiceError::PendingOperation),
                ServiceRetainedOutcome::Unknown => {}
            }
            if now >= deadline {
                return Err(MapServiceError::InvalidArgument);
            }
            cluster_hooks::require_replication_capacity(&state)?;
            let used = lifecycle::usage(&state);
            let base = identity.client.len().saturating_add(48);
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
                    .and_then(|n| n.checked_add(response))
                    .ok_or(MapServiceError::Capacity)?,
            )?;
            state
                .map
                .service_reserve(identity.clone(), digest, response)?;
            let applied = (|| {
                let mut receipt;
                let output = match command {
                    LocalHelperCommand::RetireOutcomes {
                        client,
                        generation,
                        through,
                    } => {
                        let result =
                            lifecycle::retire_locked(&mut state, client, *generation, *through)?;
                        receipt = ServiceResult::new("helper_retired");
                        receipt.number = result.removed as u64;
                        LocalHelperResult::Retirement(result)
                    }
                    LocalHelperCommand::ExpireStep { max_work } => {
                        let result = lifecycle::expire_locked(&mut state, now, *max_work)?;
                        receipt = ServiceResult::new("helper_expiry");
                        receipt.number = result.examined as u64;
                        receipt.version = Some(result.expired as u64);
                        receipt.flag = result.pending;
                        LocalHelperResult::Expiry(result)
                    }
                    LocalHelperCommand::ListenerSnapshot {
                        listener_id,
                        max_items,
                        max_bytes,
                    } => {
                        let result = lifecycle::resync_locked(
                            &mut state,
                            Arc::downgrade(&self.state),
                            *listener_id,
                            now,
                            *max_items,
                            *max_bytes,
                        )?;
                        receipt = ServiceResult::new("helper_listener_snapshot");
                        receipt.number = result.watermark;
                        receipt.flag = result.include_value;
                        receipt.token = Some(VersionToken {
                            namespace_generation: result.namespace_generation,
                            map_id: result.map_id,
                            version: result.revision,
                        });
                        receipt.items = result
                            .entries
                            .iter()
                            .enumerate()
                            .map(|(index, entry)| ServiceItem {
                                index,
                                key: entry.key.clone(),
                                kind: "present".into(),
                                value: result.include_value.then(|| entry.value.clone()),
                                version: Some(entry.version),
                                remaining_ttl: entry.remaining_ttl,
                            })
                            .collect();
                        LocalHelperResult::Snapshot(result)
                    }
                    _ => return Err(MapServiceError::InvalidArgument),
                };
                state
                    .map
                    .service_complete(&identity, receipt.retained_json());
                Ok(output)
            })();
            if applied.is_err() {
                state.map.service_abort(&identity);
            }
            applied
        })();
        let wakes = std::mem::take(&mut state.pending_wakes);
        drop(state);
        for wake in wakes {
            wake.wake();
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value_plane_async_075::{AsyncOutcome, InvocationClock};
    use crate::value_plane_service_async_075::LocalHelperAsyncFacade;

    #[tokio::test]
    async fn conflicting_helper_cancellation_and_deadline_preserve_original_pending_credit() {
        let service = UnifiedIMapService::new(CollectionBounds::default(), 1).unwrap();
        let original = LocalHelperCommand::ExpireStep { max_work: 1 };
        let identity = MutationIdentity::new("helper-owner", 1);
        service
            .admit_external_operation(identity.clone(), helper_digest(&original), 64)
            .unwrap();
        let listener = service
            .register_listener(ServiceListenerOptions::default())
            .unwrap();
        let baseline = service.resource_usage();
        let clock = Arc::new(InvocationClock::default());
        clock.advance_to(1).unwrap();
        let facade = LocalHelperAsyncFacade::new(service.clone(), clock, 2).unwrap();
        for cancelled in [false, true] {
            let (handle, future) = facade
                .invoke(
                    identity.clone(),
                    if cancelled { 10 } else { 1 },
                    LocalHelperCommand::ExpireStep { max_work: 2 },
                )
                .unwrap();
            if cancelled {
                handle.cancel();
            }
            assert!(
                matches!(future.await.outcome, AsyncOutcome::Completed(Err(error)) if error.kind() == "mutation_identity_conflict")
            );
            assert_eq!(service.resource_usage(), baseline);
            assert_eq!(service.logical_time(), 0);
            assert_eq!(
                service.query_operation_outcome(&identity).unwrap(),
                OperationOutcomeQuery::Pending
            );
            assert!(listener.drain().unwrap().is_empty());
        }
        assert_eq!(
            service
                .execute_local_helper(
                    &LocalHelperCommand::ExpireStep { max_work: 2 },
                    identity.clone(),
                    1,
                    1
                )
                .unwrap_err()
                .kind(),
            "mutation_identity_conflict"
        );
        assert_eq!(service.resource_usage(), baseline);
        assert_eq!(
            service.query_operation_outcome(&identity).unwrap(),
            OperationOutcomeQuery::Pending
        );
        assert!(service.cancel_reserved(&identity).unwrap());
    }
}
