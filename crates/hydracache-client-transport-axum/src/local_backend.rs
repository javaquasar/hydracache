//! Production local data owner; edge policy and CP lock ownership stay in the dispatcher.
use super::*;

/// The existing local cache, expiry and retained-outcome owners extracted without changing semantics.
#[derive(Debug, Default)]
pub struct LocalClientDataBackend {
    pub(super) store: Mutex<BTreeMap<StoreKey, StoredValue>>,
    cache_now_ms_for_tests: Mutex<Option<u64>>,
    cache_time_floor_ms: AtomicU64,
    next_expiry_sweep_ms: AtomicU64,
    pub(super) expiry_sweep_cursor: Mutex<Option<StoreKey>>,
    pub(super) idempotency_keys: Mutex<BTreeMap<IdempotencyKey, IdempotencyRecord>>,
}

impl LocalClientDataBackend {
    /// Construct an empty local owner.
    pub fn new() -> Self {
        Self::default()
    }

    pub(super) fn execute_inline(
        &self,
        request: VerifiedClientRequest<'_>,
    ) -> ClientResponseEnvelope {
        LocalBackendExecution::new(self, request.edge).handle_request(
            request.identity,
            request.envelope,
            request.profiling,
        )
    }
}

impl ClientDataBackend for LocalClientDataBackend {
    fn execute<'a>(&'a self, request: VerifiedClientRequest<'a>) -> ClientBackendFuture<'a> {
        Box::pin(async move { self.execute_inline(request).result })
    }
}

pub(super) struct LocalBackendExecution<'a> {
    backend: &'a LocalClientDataBackend,
    edge: &'a ClientSurfaceEdgeState,
}

impl<'a> LocalBackendExecution<'a> {
    pub(super) fn new(
        backend: &'a LocalClientDataBackend,
        edge: &'a ClientSurfaceEdgeState,
    ) -> Self {
        Self { backend, edge }
    }
    pub(super) fn set_cache_time_for_tests(&self, now_ms: Option<u64>) {
        let now_ms = now_ms.map(|candidate| {
            candidate.max(self.backend.cache_time_floor_ms.load(Ordering::SeqCst))
        });
        *self
            .backend
            .cache_now_ms_for_tests
            .lock()
            .expect("cache clock mutex") = now_ms;
    }

    pub(super) fn advance_cache_time_for_tests(&self, millis: u64) {
        let mut now = self
            .backend
            .cache_now_ms_for_tests
            .lock()
            .expect("cache clock mutex");
        let base = now.unwrap_or_else(system_time_millis);
        *now = Some(base.saturating_add(millis));
    }

    pub(super) fn retained_state_for_diagnostics(&self) -> ClientSurfaceRetainedState {
        self.sweep_expired_entries(self.now_ms(), true);
        let idempotency = self
            .backend
            .idempotency_keys
            .lock()
            .expect("idempotency mutex");
        let store = self.lock_store();
        let lock_service = self.edge.lock_service.lock().expect("lock service mutex");
        self.retained_state_locked(&idempotency, &store, &lock_service)
    }

    pub(super) fn reap_expired_entries_for_maintenance(&self) -> usize {
        self.sweep_expired_entries(self.now_ms(), false)
    }

    pub(super) fn reset_retained_state_for_diagnostics(
        &self,
    ) -> Result<ClientSurfaceDiagnosticReset, ClientSurfaceError> {
        if self.edge.isolation.is_some() {
            return Err(ClientSurfaceError::DiagnosticResetUnavailable);
        }
        let mut idempotency = self
            .backend
            .idempotency_keys
            .lock()
            .expect("idempotency mutex");
        let mut store = self.lock_store();
        let mut lock_service = self.edge.lock_service.lock().expect("lock service mutex");
        let before = self.retained_state_locked(&idempotency, &store, &lock_service);
        idempotency.clear();
        store.clear();
        lock_service.store.clear_for_diagnostic_reset();
        let after = self.retained_state_locked(&idempotency, &store, &lock_service);
        Ok(ClientSurfaceDiagnosticReset { before, after })
    }

    pub(super) fn retained_state_locked(
        &self,
        idempotency: &BTreeMap<IdempotencyKey, IdempotencyRecord>,
        store: &BTreeMap<StoreKey, StoredValue>,
        lock_service: &ClientLockService,
    ) -> ClientSurfaceRetainedState {
        ClientSurfaceRetainedState {
            store_entries: store.len(),
            value_bytes: store.values().map(|entry| entry.value.len()).sum(),
            store_identity_bytes: store
                .keys()
                .map(|(tenant, namespace, key)| tenant.len() + namespace.len() + key.len())
                .sum(),
            idempotency_outcomes: idempotency.len(),
            idempotency_identity_bytes: idempotency
                .keys()
                .map(|(tenant, key)| tenant.len() + key.len())
                .sum(),
            audit_events: self.edge.audit_sink.len(),
            conditional: lock_service.store.retained_state(),
        }
    }

    pub(super) fn now_ms(&self) -> u64 {
        let profiling = self.edge.profile_instrumentation.enabled();
        let started = profiling.then(Instant::now);
        if profiling {
            self.edge
                .profile_instrumentation
                .clock_reads
                .fetch_add(1, Ordering::Relaxed);
        }
        let candidate = self
            .backend
            .cache_now_ms_for_tests
            .lock()
            .expect("cache clock mutex")
            .unwrap_or_else(system_time_millis);
        if profiling {
            self.edge
                .profile_instrumentation
                .clock_floor_seqcst_operations
                .fetch_add(1, Ordering::Relaxed);
        }
        let previous = self
            .backend
            .cache_time_floor_ms
            .fetch_max(candidate, Ordering::SeqCst);
        let now_ms = candidate.max(previous);
        if let Some(started) = started {
            self.edge
                .profile_instrumentation
                .clock_nanoseconds
                .fetch_add(elapsed_nanoseconds(started), Ordering::Relaxed);
        }
        now_ms
    }

    pub(super) fn materialize_store_key(
        &self,
        identity: &ClientIdentity,
        ns: &Namespace,
        key: &StructuredKey,
    ) -> StoreKey {
        let stable_key = key.stable_key();
        if self.edge.profile_instrumentation.enabled() {
            self.edge
                .profile_instrumentation
                .structured_key_materializations
                .fetch_add(1, Ordering::Relaxed);
            self.edge
                .profile_instrumentation
                .structured_key_bytes_copied
                .fetch_add(
                    identity
                        .tenant()
                        .len()
                        .saturating_add(ns.as_str().len())
                        .saturating_add(stable_key.len()) as u64,
                    Ordering::Relaxed,
                );
        }
        (
            identity.tenant().to_owned(),
            ns.as_str().to_owned(),
            stable_key,
        )
    }

    pub(super) fn sweep_expired_entries(&self, now_ms: u64, force: bool) -> usize {
        let profiling = self.edge.profile_instrumentation.enabled();
        if profiling {
            self.edge
                .profile_instrumentation
                .expiry_sweep_checks
                .fetch_add(1, Ordering::Relaxed);
        }
        if profiling {
            self.edge
                .profile_instrumentation
                .expiry_gate_seqcst_operations
                .fetch_add(1, Ordering::Relaxed);
        }
        let next = self.backend.next_expiry_sweep_ms.load(Ordering::SeqCst);
        if !force && now_ms < next {
            return 0;
        }
        if profiling && !force {
            self.edge
                .profile_instrumentation
                .expiry_gate_seqcst_operations
                .fetch_add(1, Ordering::Relaxed);
        }
        if !force
            && self
                .backend
                .next_expiry_sweep_ms
                .compare_exchange(
                    next,
                    now_ms.saturating_add(CLIENT_SURFACE_EXPIRY_SWEEP_INTERVAL_MS),
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                )
                .is_err()
        {
            return 0;
        }
        if force {
            if profiling {
                self.edge
                    .profile_instrumentation
                    .expiry_gate_seqcst_operations
                    .fetch_add(1, Ordering::Relaxed);
            }
            self.backend.next_expiry_sweep_ms.store(
                now_ms.saturating_add(CLIENT_SURFACE_EXPIRY_SWEEP_INTERVAL_MS),
                Ordering::SeqCst,
            );
        }
        if profiling {
            self.edge
                .profile_instrumentation
                .expiry_sweeps_claimed
                .fetch_add(1, Ordering::Relaxed);
        }

        let mut cursor = self
            .backend
            .expiry_sweep_cursor
            .lock()
            .expect("expiry cursor mutex");
        let removed = if let Some(isolation) = &self.edge.isolation {
            let mut isolation = isolation.lock().expect("isolation mutex");
            let mut store = self.lock_store();
            let candidates = expiry_sweep_candidates(
                &store,
                now_ms,
                if force { None } else { cursor.as_ref() },
                if force {
                    usize::MAX
                } else {
                    CLIENT_SURFACE_EXPIRY_SWEEP_SCAN_LIMIT
                },
                !force,
            );
            if profiling {
                self.edge
                    .profile_instrumentation
                    .expiry_sweep_entries_examined
                    .fetch_add(candidates.examined as u64, Ordering::Relaxed);
                self.edge
                    .profile_instrumentation
                    .expiry_sweep_entries_removed
                    .fetch_add(candidates.expired.len() as u64, Ordering::Relaxed);
            }
            for expired_key @ (tenant, namespace, key) in &candidates.expired {
                let tenant_id =
                    TenantId::new(tenant).expect("stored tenant identity was validated");
                isolation.remove_entry_for_tenant(&tenant_id, namespace, key);
                store.remove(expired_key);
            }
            *cursor = candidates.next_cursor;
            candidates.expired.len()
        } else {
            let mut store = self.lock_store();
            let candidates = expiry_sweep_candidates(
                &store,
                now_ms,
                if force { None } else { cursor.as_ref() },
                if force {
                    usize::MAX
                } else {
                    CLIENT_SURFACE_EXPIRY_SWEEP_SCAN_LIMIT
                },
                !force,
            );
            if profiling {
                self.edge
                    .profile_instrumentation
                    .expiry_sweep_entries_examined
                    .fetch_add(candidates.examined as u64, Ordering::Relaxed);
                self.edge
                    .profile_instrumentation
                    .expiry_sweep_entries_removed
                    .fetch_add(candidates.expired.len() as u64, Ordering::Relaxed);
            }
            for key in &candidates.expired {
                store.remove(key);
            }
            *cursor = candidates.next_cursor;
            candidates.expired.len()
        };
        if removed != 0 {
            self.edge.record_state_mutations(removed as u64);
        }
        removed
    }

    pub(super) fn lock_store(&self) -> ProfiledStoreGuard<'_> {
        let profiling = self.edge.profile_instrumentation.enabled();
        let wait_started = profiling.then(Instant::now);
        let guard = self.backend.store.lock().expect("store mutex");
        if let Some(started) = wait_started {
            self.edge
                .profile_instrumentation
                .store_lock_acquisitions
                .fetch_add(1, Ordering::Relaxed);
            self.edge
                .profile_instrumentation
                .store_lock_wait_nanoseconds
                .fetch_add(elapsed_nanoseconds(started), Ordering::Relaxed);
        }
        ProfiledStoreGuard {
            guard,
            hold_started: profiling.then(Instant::now),
            instrumentation: &self.edge.profile_instrumentation,
        }
    }

    pub(super) fn handle_compare_and_set(
        &self,
        identity: &ClientIdentity,
        request_id: String,
        command: CompareAndSetCommand,
    ) -> ClientResponseEnvelope {
        let CompareAndSetCommand {
            ns,
            key,
            expected,
            new_value,
            level,
        } = command;
        if new_value.len() > self.edge.limits.max_value_bytes {
            return ClientResponseEnvelope::error(
                request_id,
                ClientErrorEnvelope::new(ClientErrorCode::TooLarge, false, "value too large"),
            );
        }
        let value_bytes = new_value.len() as u64;
        let map_key = self.materialize_store_key(identity, &ns, &key);
        let cas_key = lock_key(identity, &ns, &key);
        let stable_key = key.stable_key();
        let now_ms = self.now_ms();
        let level = lock_consistency(level);
        let result = if let Some(isolation) = &self.edge.isolation {
            let mut isolation = isolation.lock().expect("isolation mutex");
            let mut store = self.lock_store();
            let (live_value, expired) = live_value(&mut store, &map_key, now_ms);
            let condition_holds = match &expected {
                CasExpectation::Exact(expected) => {
                    live_value.as_deref() == Some(expected.as_slice())
                }
                CasExpectation::Present => live_value.is_some(),
            };

            if condition_holds {
                let value_for_store = new_value.clone();
                let mut command_result = None;
                let admitted = isolation
                    .admit_put_if_committed(
                        identity.client_id(),
                        ns.as_str(),
                        &stable_key,
                        value_bytes,
                        true,
                        || {
                            let result = apply_compare_and_set(
                                &mut self.edge.lock_service.lock().expect("lock service mutex"),
                                &cas_key,
                                live_value.as_deref(),
                                &expected,
                                new_value,
                                level,
                            );
                            let applied = matches!(&result, Ok(ClientResponse::CasApplied { .. }));
                            if applied {
                                store.insert(map_key, StoredValue::persistent(value_for_store));
                            }
                            command_result = Some(result);
                            applied
                        },
                    )
                    .map_err(admission_error);
                let committed = matches!(admitted, Ok(true));
                if expired && !committed {
                    let _ = isolation.remove_entry(identity.client_id(), ns.as_str(), &stable_key);
                }
                match admitted {
                    Err(error) => Err(error),
                    Ok(_) => command_result.expect("true CAS condition executes command"),
                }
            } else {
                let admitted = isolation
                    .admit_put_if_committed(
                        identity.client_id(),
                        ns.as_str(),
                        &stable_key,
                        value_bytes,
                        false,
                        || unreachable!("false CAS condition skips commit callback"),
                    )
                    .map_err(admission_error);
                if expired {
                    let _ = isolation.remove_entry(identity.client_id(), ns.as_str(), &stable_key);
                }
                match admitted {
                    Err(error) => Err(error),
                    Ok(false) => apply_compare_and_set(
                        &mut self.edge.lock_service.lock().expect("lock service mutex"),
                        &cas_key,
                        live_value.as_deref(),
                        &expected,
                        new_value,
                        level,
                    ),
                    Ok(true) => unreachable!("false CAS condition cannot commit"),
                }
            }
        } else {
            let mut store = self.lock_store();
            let (live_value, _) = live_value(&mut store, &map_key, now_ms);
            let value_for_store = new_value.clone();
            let result = apply_compare_and_set(
                &mut self.edge.lock_service.lock().expect("lock service mutex"),
                &cas_key,
                live_value.as_deref(),
                &expected,
                new_value,
                level,
            );
            if matches!(&result, Ok(ClientResponse::CasApplied { .. })) {
                store.insert(map_key, StoredValue::persistent(value_for_store));
            }
            result
        };

        let applied = matches!(&result, Ok(ClientResponse::CasApplied { .. }));
        if applied {
            self.edge.record_state_mutations(1);
            self.edge
                .record_invalidation(identity, ns, key, ClientSurfaceMutationKind::Stored);
        }
        lock_response(request_id, result)
    }

    pub(super) fn handle_remove_if_value(
        &self,
        identity: &ClientIdentity,
        request_id: String,
        ns: Namespace,
        key: StructuredKey,
        expected: Vec<u8>,
        level: LockConsistency,
    ) -> ClientResponseEnvelope {
        let map_key = self.materialize_store_key(identity, &ns, &key);
        let cas_key = lock_key(identity, &ns, &key);
        let level = lock_consistency(level);
        let now_ms = self.now_ms();
        let result = match self.with_admitted_store(identity, |value_store, isolation| {
            let (live_value, expired) = live_value(value_store, &map_key, now_ms);
            let result = self
                .edge
                .lock_service
                .lock()
                .expect("lock service mutex")
                .apply_lock_command(|store, _now| {
                    sync_conditional_record(store, &cas_key, live_value.as_deref())?;
                    let result = store.remove_if_value(&cas_key, &expected, level)?;
                    Ok(cas_response(result))
                });
            let applied = matches!(&result, Ok(ClientResponse::CasApplied { .. }));
            if applied {
                value_store.remove(&map_key);
            }
            if applied || expired {
                if let Some(isolation) = isolation {
                    let _ = isolation.remove_entry(
                        identity.client_id(),
                        ns.as_str(),
                        &key.stable_key(),
                    );
                }
            }
            result
        }) {
            Ok(result) => result,
            Err(error) => Err(error),
        };

        let applied = matches!(&result, Ok(ClientResponse::CasApplied { .. }));
        if applied {
            self.edge.record_state_mutations(1);
            self.edge
                .record_invalidation(identity, ns, key, ClientSurfaceMutationKind::Removed);
        }
        lock_response(request_id, result)
    }

    pub(super) fn handle_put(
        &self,
        identity: &ClientIdentity,
        command: PutCommand,
    ) -> ClientResponseEnvelope {
        let PutCommand {
            request_id,
            idempotency_key,
            ns,
            key,
            value,
            ttl_ms,
        } = command;
        if value.len() > self.edge.limits.max_value_bytes {
            return ClientResponseEnvelope::error(
                request_id,
                ClientErrorEnvelope::new(ClientErrorCode::TooLarge, false, "value too large"),
            );
        }
        let now_ms = self.now_ms();
        let scoped_idempotency_key =
            idempotency_key.map(|idempotency_key| (identity.tenant().to_owned(), idempotency_key));
        let mut idempotency_keys = scoped_idempotency_key.as_ref().map(|_| {
            self.backend
                .idempotency_keys
                .lock()
                .expect("idempotency mutex")
        });
        if let (Some(idempotency_key), Some(keys)) =
            (scoped_idempotency_key.as_ref(), idempotency_keys.as_mut())
        {
            keys.retain(|_, record| record.expires_at_ms > now_ms);
            if keys.contains_key(idempotency_key) {
                return ClientResponseEnvelope::ok(request_id, ClientResponse::Stored);
            }
            if keys.len() >= CLIENT_SURFACE_IDEMPOTENCY_CAPACITY {
                let retry_after_ms = keys
                    .values()
                    .map(|record| record.expires_at_ms.saturating_sub(now_ms).max(1))
                    .min()
                    .unwrap_or(1);
                return ClientResponseEnvelope::error(
                    request_id,
                    ClientErrorEnvelope::new(
                        ClientErrorCode::RateLimited,
                        true,
                        "idempotency outcome capacity exhausted",
                    )
                    .with_retry_after_ms(retry_after_ms),
                );
            }
        }
        let value_bytes = value.len() as u64;
        if self.edge.profile_instrumentation.enabled() {
            self.edge
                .profile_instrumentation
                .put_value_bytes_moved
                .fetch_add(value_bytes, Ordering::Relaxed);
        }
        let stored = match ttl_ms {
            Some(ttl_ms) => StoredValue::with_ttl(value, now_ms, ttl_ms),
            None => StoredValue::persistent(value),
        };
        let map_key = self.materialize_store_key(identity, &ns, &key);
        let admitted = if let Some(isolation) = &self.edge.isolation {
            let mut isolation = isolation.lock().expect("isolation mutex");
            let mut store = self.lock_store();
            isolation
                .admit_put_if_committed(
                    identity.client_id(),
                    ns.as_str(),
                    &key.stable_key(),
                    value_bytes,
                    true,
                    || {
                        store.insert(map_key, stored);
                        true
                    },
                )
                .map_err(admission_error)
        } else {
            self.lock_store().insert(map_key, stored);
            Ok(true)
        };
        if let Err(error) = admitted {
            return ClientResponseEnvelope::error(request_id, error);
        }
        if let (Some(idempotency_key), Some(keys)) =
            (scoped_idempotency_key, idempotency_keys.as_mut())
        {
            keys.insert(
                idempotency_key,
                IdempotencyRecord {
                    expires_at_ms: now_ms.saturating_add(CLIENT_SURFACE_IDEMPOTENCY_TTL_MS),
                },
            );
        }
        drop(idempotency_keys);
        self.edge.record_state_mutations(1);
        self.edge
            .record_invalidation(identity, ns, key, ClientSurfaceMutationKind::Stored);
        ClientResponseEnvelope::ok(request_id, ClientResponse::Stored)
    }

    pub(super) fn handle_expire(
        &self,
        identity: &ClientIdentity,
        request_id: String,
        ns: Namespace,
        key: StructuredKey,
        ttl_ms: u64,
    ) -> ClientResponseEnvelope {
        let now_ms = self.now_ms();
        let applied = match self.with_admitted_store(identity, |store, isolation| {
            let (entry, expired) = live_entry_mut(
                store,
                &self.materialize_store_key(identity, &ns, &key),
                now_ms,
            );
            let applied = entry
                .map(|entry| {
                    entry.expires_at_ms = Some(now_ms.saturating_add(ttl_ms));
                    true
                })
                .unwrap_or(false);
            if expired {
                if let Some(isolation) = isolation {
                    let _ = isolation.remove_entry(
                        identity.client_id(),
                        ns.as_str(),
                        &key.stable_key(),
                    );
                }
            }
            applied
        }) {
            Ok(applied) => applied,
            Err(error) => return ClientResponseEnvelope::error(request_id, error),
        };
        if applied {
            self.edge.record_state_mutations(1);
            self.edge
                .record_invalidation(identity, ns, key, ClientSurfaceMutationKind::Expire);
        }
        ClientResponseEnvelope::ok(request_id, ClientResponse::Expiry { applied })
    }

    pub(super) fn handle_persist(
        &self,
        identity: &ClientIdentity,
        request_id: String,
        ns: Namespace,
        key: StructuredKey,
    ) -> ClientResponseEnvelope {
        let now_ms = self.now_ms();
        let applied = match self.with_admitted_store(identity, |store, isolation| {
            let (entry, expired) = live_entry_mut(
                store,
                &self.materialize_store_key(identity, &ns, &key),
                now_ms,
            );
            let applied = entry
                .map(|entry| entry.expires_at_ms.take().is_some())
                .unwrap_or(false);
            if expired {
                if let Some(isolation) = isolation {
                    let _ = isolation.remove_entry(
                        identity.client_id(),
                        ns.as_str(),
                        &key.stable_key(),
                    );
                }
            }
            applied
        }) {
            Ok(applied) => applied,
            Err(error) => return ClientResponseEnvelope::error(request_id, error),
        };
        if applied {
            self.edge.record_state_mutations(1);
            self.edge
                .record_invalidation(identity, ns, key, ClientSurfaceMutationKind::Persist);
        }
        ClientResponseEnvelope::ok(request_id, ClientResponse::Expiry { applied })
    }

    pub(super) fn handle_get_ttl(
        &self,
        identity: &ClientIdentity,
        request_id: String,
        ns: Namespace,
        key: StructuredKey,
    ) -> ClientResponseEnvelope {
        let now_ms = self.now_ms();
        let state = match self.with_admitted_store(identity, |store, isolation| {
            let (state, expired) = ttl_state(
                store,
                &self.materialize_store_key(identity, &ns, &key),
                now_ms,
            );
            if expired {
                if let Some(isolation) = isolation {
                    let _ = isolation.remove_entry(
                        identity.client_id(),
                        ns.as_str(),
                        &key.stable_key(),
                    );
                }
            }
            state
        }) {
            Ok(state) => state,
            Err(error) => return ClientResponseEnvelope::error(request_id, error),
        };
        ClientResponseEnvelope::ok(request_id, ClientResponse::Ttl { state })
    }

    pub(super) fn handle_conditional_put(
        &self,
        identity: &ClientIdentity,
        args: ConditionalPutArgs,
    ) -> ClientResponseEnvelope {
        let ConditionalPutArgs {
            request_id,
            ns,
            key,
            value,
            ttl_ms,
            condition,
        } = args;
        if value.len() > self.edge.limits.max_value_bytes {
            return ClientResponseEnvelope::error(
                request_id,
                ClientErrorEnvelope::new(ClientErrorCode::TooLarge, false, "value too large"),
            );
        }
        if let ConditionalPutCondition::IfPresentValue(expected) = &condition {
            if expected.len() > self.edge.limits.max_value_bytes {
                return ClientResponseEnvelope::error(
                    request_id,
                    ClientErrorEnvelope::new(
                        ClientErrorCode::TooLarge,
                        false,
                        "expected value too large",
                    ),
                );
            }
        }
        let now_ms = self.now_ms();
        let map_key = self.materialize_store_key(identity, &ns, &key);
        let stable_key = key.stable_key();
        let value_bytes = value.len() as u64;
        let stored = if let Some(isolation) = &self.edge.isolation {
            let mut isolation = isolation.lock().expect("isolation mutex");
            let mut store = self.lock_store();
            let (entry, expired) = live_entry_mut(&mut store, &map_key, now_ms);
            let condition_holds = match &condition {
                ConditionalPutCondition::IfAbsent => entry.is_none(),
                ConditionalPutCondition::IfPresentValue(expected) => {
                    entry.is_some_and(|entry| entry.value == *expected)
                }
            };
            let admitted = isolation
                .admit_put_if_committed(
                    identity.client_id(),
                    ns.as_str(),
                    &stable_key,
                    value_bytes,
                    condition_holds,
                    || {
                        store.insert(map_key, stored_value(value, now_ms, ttl_ms));
                        true
                    },
                )
                .map_err(admission_error);
            let stored = matches!(admitted, Ok(true));
            if expired && !stored {
                let _ = isolation.remove_entry(identity.client_id(), ns.as_str(), &stable_key);
            }
            if let Err(error) = admitted {
                return ClientResponseEnvelope::error(request_id, error);
            }
            stored
        } else {
            let mut store = self.lock_store();
            let (entry, _) = live_entry_mut(&mut store, &map_key, now_ms);
            let condition_holds = match &condition {
                ConditionalPutCondition::IfAbsent => entry.is_none(),
                ConditionalPutCondition::IfPresentValue(expected) => {
                    entry.is_some_and(|entry| entry.value == *expected)
                }
            };
            if condition_holds {
                store.insert(map_key, stored_value(value, now_ms, ttl_ms));
            }
            condition_holds
        };
        if stored {
            self.edge.record_state_mutations(1);
            self.edge
                .record_invalidation(identity, ns, key, ClientSurfaceMutationKind::Stored);
        }
        ClientResponseEnvelope::ok(request_id, ClientResponse::ConditionalStored { stored })
    }

    pub(super) fn handle_compare_value_invalidate(
        &self,
        identity: &ClientIdentity,
        request_id: String,
        ns: Namespace,
        key: StructuredKey,
        expected_value: Vec<u8>,
    ) -> ClientResponseEnvelope {
        if expected_value.len() > self.edge.limits.max_value_bytes {
            return ClientResponseEnvelope::error(
                request_id,
                ClientErrorEnvelope::new(
                    ClientErrorCode::TooLarge,
                    false,
                    "expected value too large",
                ),
            );
        }
        let now_ms = self.now_ms();
        let map_key = self.materialize_store_key(identity, &ns, &key);
        let applied = match self.with_admitted_store(identity, |store, isolation| {
            let (entry, expired) = live_entry_mut(store, &map_key, now_ms);
            let applied = entry.is_some_and(|entry| entry.value == expected_value);
            if applied {
                store.remove(&map_key);
            }
            if expired || applied {
                if let Some(isolation) = isolation {
                    let _ = isolation.remove_entry(
                        identity.client_id(),
                        ns.as_str(),
                        &key.stable_key(),
                    );
                }
            }
            applied
        }) {
            Ok(applied) => applied,
            Err(error) => return ClientResponseEnvelope::error(request_id, error),
        };
        if applied {
            self.edge.record_state_mutations(1);
            self.edge
                .record_invalidation(identity, ns, key, ClientSurfaceMutationKind::Removed);
        }
        ClientResponseEnvelope::ok(request_id, ClientResponse::CompareValueApplied { applied })
    }

    pub(super) fn handle_compare_value_expire(
        &self,
        identity: &ClientIdentity,
        args: CompareValueExpireArgs,
    ) -> ClientResponseEnvelope {
        let CompareValueExpireArgs {
            request_id,
            ns,
            key,
            expected_value,
            ttl_ms,
            mode,
        } = args;
        if expected_value.len() > self.edge.limits.max_value_bytes {
            return ClientResponseEnvelope::error(
                request_id,
                ClientErrorEnvelope::new(
                    ClientErrorCode::TooLarge,
                    false,
                    "expected value too large",
                ),
            );
        }
        let now_ms = self.now_ms();
        let applied = match self.with_admitted_store(identity, |store, isolation| {
            let (entry, expired) = live_entry_mut(
                store,
                &self.materialize_store_key(identity, &ns, &key),
                now_ms,
            );
            let applied = entry
                .map(|entry| {
                    if entry.value == expected_value {
                        match mode {
                            CompareValueExpireMode::Replace => {
                                entry.expires_at_ms = Some(now_ms.saturating_add(ttl_ms));
                                true
                            }
                            CompareValueExpireMode::ReplaceIfExpiring => {
                                if entry.expires_at_ms.is_some() {
                                    entry.expires_at_ms = Some(now_ms.saturating_add(ttl_ms));
                                    true
                                } else {
                                    false
                                }
                            }
                            CompareValueExpireMode::AddToRemaining => {
                                if let Some(expires_at_ms) = entry.expires_at_ms {
                                    entry.expires_at_ms =
                                        Some(expires_at_ms.saturating_add(ttl_ms));
                                    true
                                } else {
                                    false
                                }
                            }
                        }
                    } else {
                        false
                    }
                })
                .unwrap_or(false);
            if expired {
                if let Some(isolation) = isolation {
                    let _ = isolation.remove_entry(
                        identity.client_id(),
                        ns.as_str(),
                        &key.stable_key(),
                    );
                }
            }
            applied
        }) {
            Ok(applied) => applied,
            Err(error) => return ClientResponseEnvelope::error(request_id, error),
        };
        if applied {
            self.edge.record_state_mutations(1);
            self.edge
                .record_invalidation(identity, ns, key, ClientSurfaceMutationKind::Expire);
        }
        ClientResponseEnvelope::ok(request_id, ClientResponse::CompareValueApplied { applied })
    }

    pub(super) fn with_admitted_store<T>(
        &self,
        identity: &ClientIdentity,
        apply: impl FnOnce(&mut BTreeMap<StoreKey, StoredValue>, Option<&mut ConsumerIsolation>) -> T,
    ) -> Result<T, ClientErrorEnvelope> {
        if let Some(isolation) = &self.edge.isolation {
            let mut isolation = isolation.lock().expect("isolation mutex");
            isolation
                .admit_request(identity.client_id())
                .map_err(admission_error)?;
            let mut store = self.lock_store();
            Ok(apply(&mut store, Some(&mut isolation)))
        } else {
            let mut store = self.lock_store();
            Ok(apply(&mut store, None))
        }
    }

    fn handle_request(
        &self,
        identity: &ClientIdentity,
        envelope: ClientRequestEnvelope,
        profiling: bool,
    ) -> ClientResponseEnvelope {
        self.sweep_expired_entries(self.now_ms(), false);
        let response = match envelope.request {
            ClientRequest::Get { ns, key } => {
                let now_ms = self.now_ms();
                match self.with_admitted_store(identity, |store, isolation| {
                    let (value, expired) = live_value(
                        store,
                        &self.materialize_store_key(identity, &ns, &key),
                        now_ms,
                    );
                    if expired {
                        if let Some(isolation) = isolation {
                            let _ = isolation.remove_entry(
                                identity.client_id(),
                                ns.as_str(),
                                &key.stable_key(),
                            );
                        }
                    }
                    (value, expired)
                }) {
                    Ok((value, expired)) => {
                        if profiling {
                            let counter = if expired {
                                &self.edge.profile_instrumentation.expired_get_removals
                            } else if value.is_some() {
                                &self.edge.profile_instrumentation.live_get_hits
                            } else {
                                &self.edge.profile_instrumentation.missing_gets
                            };
                            counter.fetch_add(1, Ordering::Relaxed);
                            self.edge
                                .profile_instrumentation
                                .get_value_bytes_cloned
                                .fetch_add(
                                    value.as_ref().map_or(0, Vec::len) as u64,
                                    Ordering::Relaxed,
                                );
                        }
                        ClientResponseEnvelope::ok(
                            envelope.request_id,
                            ClientResponse::Value { value },
                        )
                    }
                    Err(error) => ClientResponseEnvelope::error(envelope.request_id, error),
                }
            }
            ClientRequest::Put {
                ns,
                key,
                value,
                ttl_ms,
                dimensions: _,
            } => self.handle_put(
                identity,
                PutCommand {
                    request_id: envelope.request_id,
                    idempotency_key: envelope.idempotency_key,
                    ns,
                    key,
                    value,
                    ttl_ms,
                },
            ),
            ClientRequest::Invalidate { ns, key } => {
                match self.with_admitted_store(identity, |store, isolation| {
                    let removed = store
                        .remove(&self.materialize_store_key(identity, &ns, &key))
                        .is_some();
                    if removed {
                        if let Some(isolation) = isolation {
                            let _ = isolation.remove_entry(
                                identity.client_id(),
                                ns.as_str(),
                                &key.stable_key(),
                            );
                        }
                    }
                }) {
                    Err(error) => ClientResponseEnvelope::error(envelope.request_id, error),
                    Ok(()) => {
                        self.edge.record_state_mutations(1);
                        self.edge.record_invalidation(
                            identity,
                            ns,
                            key,
                            ClientSurfaceMutationKind::Invalidated,
                        );
                        ClientResponseEnvelope::ok(envelope.request_id, ClientResponse::Invalidated)
                    }
                }
            }
            ClientRequest::BatchGet { ns, keys } => {
                self.edge.record_batch_shape(keys.len());
                let batch_bytes = keys.iter().fold(0usize, |total, key| {
                    total.saturating_add(key.stable_key().len())
                });
                if keys.len() > self.edge.limits.max_batch_entries {
                    ClientResponseEnvelope::error(
                        envelope.request_id,
                        ClientErrorEnvelope::new(
                            ClientErrorCode::TooLarge,
                            false,
                            "batch contains too many entries",
                        ),
                    )
                } else if batch_bytes > self.edge.limits.max_batch_bytes {
                    ClientResponseEnvelope::error(
                        envelope.request_id,
                        ClientErrorEnvelope::new(
                            ClientErrorCode::TooLarge,
                            false,
                            "batch exceeds max_batch_bytes",
                        ),
                    )
                } else {
                    let now_ms = self.now_ms();
                    match self.with_admitted_store(identity, |store, isolation| {
                        let mut expired_keys = Vec::new();
                        let mut items = Vec::with_capacity(keys.len());
                        for (index, key) in keys.iter().enumerate() {
                            let (value, expired) = live_value(
                                store,
                                &self.materialize_store_key(identity, &ns, key),
                                now_ms,
                            );
                            if expired {
                                expired_keys.push(key);
                            }
                            items.push(BatchItemStatus {
                                index,
                                result: Ok(value),
                            });
                        }
                        if let Some(isolation) = isolation {
                            for key in expired_keys {
                                let _ = isolation.remove_entry(
                                    identity.client_id(),
                                    ns.as_str(),
                                    &key.stable_key(),
                                );
                            }
                        }
                        items
                    }) {
                        Ok(items) => ClientResponseEnvelope::ok(
                            envelope.request_id,
                            ClientResponse::Batch { items },
                        ),
                        Err(error) => ClientResponseEnvelope::error(envelope.request_id, error),
                    }
                }
            }
            ClientRequest::BatchPut { ns, entries } => {
                self.edge.record_batch_shape(entries.len());
                let batch_bytes = entries.iter().fold(0usize, |total, entry| {
                    total
                        .saturating_add(entry.key.stable_key().len())
                        .saturating_add(entry.value.len())
                });
                if entries.len() > self.edge.limits.max_batch_entries {
                    ClientResponseEnvelope::error(
                        envelope.request_id,
                        ClientErrorEnvelope::new(
                            ClientErrorCode::TooLarge,
                            false,
                            "batch contains too many entries",
                        ),
                    )
                } else if batch_bytes > self.edge.limits.max_batch_bytes {
                    ClientResponseEnvelope::error(
                        envelope.request_id,
                        ClientErrorEnvelope::new(
                            ClientErrorCode::TooLarge,
                            false,
                            "batch exceeds max_batch_bytes",
                        ),
                    )
                } else if entries
                    .iter()
                    .any(|entry| entry.value.len() > self.edge.limits.max_value_bytes)
                {
                    ClientResponseEnvelope::error(
                        envelope.request_id,
                        ClientErrorEnvelope::new(
                            ClientErrorCode::TooLarge,
                            false,
                            "batch item value too large",
                        ),
                    )
                } else {
                    let quota_entries = entries
                        .iter()
                        .map(|entry| (entry.key.stable_key(), entry.value.len() as u64))
                        .collect::<Vec<_>>();
                    let commit = || {
                        let mut store = self.lock_store();
                        for entry in &entries {
                            store.insert(
                                self.materialize_store_key(identity, &ns, &entry.key),
                                StoredValue::persistent(entry.value.clone()),
                            );
                        }
                        true
                    };
                    let admitted = if let Some(isolation) = &self.edge.isolation {
                        isolation
                            .lock()
                            .expect("isolation mutex")
                            .admit_batch_put_if_committed(
                                identity.client_id(),
                                ns.as_str(),
                                &quota_entries,
                                commit,
                            )
                            .map_err(admission_error)
                    } else {
                        Ok(commit())
                    };
                    match admitted {
                        Err(error) => ClientResponseEnvelope::error(envelope.request_id, error),
                        Ok(_) => {
                            self.edge.record_state_mutations(entries.len() as u64);
                            for entry in &entries {
                                self.edge.record_mutation(
                                    identity,
                                    ns.clone(),
                                    Some(entry.key.clone()),
                                    ClientSurfaceMutationKind::Stored,
                                );
                            }
                            let items = (0..entries.len())
                                .map(|index| BatchItemStatus {
                                    index,
                                    result: Ok(None),
                                })
                                .collect();
                            ClientResponseEnvelope::ok(
                                envelope.request_id,
                                ClientResponse::Batch { items },
                            )
                        }
                    }
                }
            }
            ClientRequest::Expire { ns, key, ttl_ms } => {
                self.handle_expire(identity, envelope.request_id, ns, key, ttl_ms)
            }
            ClientRequest::Persist { ns, key } => {
                self.handle_persist(identity, envelope.request_id, ns, key)
            }
            ClientRequest::GetTtl { ns, key } => {
                self.handle_get_ttl(identity, envelope.request_id, ns, key)
            }
            ClientRequest::ConditionalPut {
                ns,
                key,
                value,
                ttl_ms,
                condition,
            } => self.handle_conditional_put(
                identity,
                ConditionalPutArgs {
                    request_id: envelope.request_id,
                    ns,
                    key,
                    value,
                    ttl_ms,
                    condition,
                },
            ),
            ClientRequest::CompareValueAndInvalidate {
                ns,
                key,
                expected_value,
            } => self.handle_compare_value_invalidate(
                identity,
                envelope.request_id,
                ns,
                key,
                expected_value,
            ),
            ClientRequest::CompareValueAndExpire {
                ns,
                key,
                expected_value,
                ttl_ms,
                mode,
            } => self.handle_compare_value_expire(
                identity,
                CompareValueExpireArgs {
                    request_id: envelope.request_id,
                    ns,
                    key,
                    expected_value,
                    ttl_ms,
                    mode,
                },
            ),
            ClientRequest::EvictRegion { ns } => {
                let evict_store = |store: &mut BTreeMap<StoreKey, StoredValue>| {
                    store.retain(|(tenant, stored_ns, _), _| {
                        tenant != identity.tenant() || stored_ns != ns.as_str()
                    });
                };
                let admitted = if let Some(isolation) = &self.edge.isolation {
                    let mut isolation = isolation.lock().expect("isolation mutex");
                    isolation
                        .evict_namespace(identity.client_id(), ns.as_str())
                        .map_err(admission_error)
                        .map(|_| {
                            let mut store = self.lock_store();
                            evict_store(&mut store);
                        })
                } else {
                    let mut store = self.lock_store();
                    evict_store(&mut store);
                    Ok(())
                };
                if let Err(error) = admitted {
                    ClientResponseEnvelope::error(envelope.request_id, error)
                } else {
                    self.edge.record_state_mutations(1);
                    self.edge.record_namespace_mutation(
                        identity,
                        ns,
                        ClientSurfaceMutationKind::NamespaceEvicted,
                    );
                    ClientResponseEnvelope::ok(envelope.request_id, ClientResponse::Evicted)
                }
            }
            ClientRequest::CompareAndSet {
                ns,
                key,
                expected,
                new_value,
                level,
            } => self.handle_compare_and_set(
                identity,
                envelope.request_id,
                CompareAndSetCommand {
                    ns,
                    key,
                    expected,
                    new_value,
                    level,
                },
            ),
            ClientRequest::RemoveIfValue {
                ns,
                key,
                expected,
                level,
            } => {
                self.handle_remove_if_value(identity, envelope.request_id, ns, key, expected, level)
            }
            request @ (ClientRequest::SubscribeInvalidations { .. }
            | ClientRequest::SubscribeEntryEvents { .. }
            | ClientRequest::TryLock { .. }
            | ClientRequest::Unlock { .. }
            | ClientRequest::RenewLockLease { .. }
            | ClientRequest::ForceUnlock { .. }
            | ClientRequest::GetLockOwnership { .. }) => self.edge.handle_control_request(
                identity,
                ClientRequestEnvelope {
                    request,
                    ..envelope
                },
            ),
        };
        response
    }
}
