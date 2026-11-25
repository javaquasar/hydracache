use super::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_boundary_refuses_data_operations_before_effects() {
        let edge = ClientSurfaceEdgeState::new(Default::default(), None).unwrap();
        let response = edge.handle_control_request(
            &ClientIdentity::new("client", "tenant").unwrap(),
            ClientRequestEnvelope::new(
                "data",
                ClientRequest::Get {
                    ns: Namespace::new("data").unwrap(),
                    key: StructuredKey::new(vec!["key".to_owned()]).unwrap(),
                },
            ),
        );
        assert_eq!(
            response.result.unwrap_err().code,
            ClientErrorCode::BackendUnavailable
        );
        assert_eq!(edge.state_mutations(), 0);
    }
}

/// Admission, audit, subscription lifecycle and the separate CP lock engine.
#[derive(Debug)]
pub(super) struct ClientSurfaceEdgeState {
    pub(super) limits: ClientSurfaceLimits,
    dispatch_attempts: AtomicU64,
    state_mutations: AtomicU64,
    rejected_anonymous: AtomicU64,
    rejected_oversized: AtomicU64,
    active_subscriptions: AtomicU64,
    pub(super) next_message_id: AtomicU64,
    mutation_events: broadcast::Sender<ClientSurfaceMutationEvent>,
    pub(super) lock_service: Mutex<ClientLockService>,
    pub(super) audit_sink: Arc<InMemoryAuditSink>,
    audit: Mutex<AuditRecorder<Arc<InMemoryAuditSink>>>,
    pub(super) isolation: Option<Mutex<ConsumerIsolation>>,
    pub(super) profile_instrumentation: ClientSurfaceProfileInstrumentation,
}

impl ClientSurfaceEdgeState {
    pub(super) fn new(
        limits: ClientSurfaceLimits,
        isolation: Option<ConsumerIsolation>,
    ) -> Result<Self, ClientSurfaceError> {
        limits.validate()?;
        let audit_sink = Arc::new(InMemoryAuditSink::new());
        let (mutation_events, _) = broadcast::channel(CLIENT_SURFACE_MUTATION_EVENT_CAPACITY);
        Ok(Self {
            limits,
            dispatch_attempts: AtomicU64::new(0),
            state_mutations: AtomicU64::new(0),
            rejected_anonymous: AtomicU64::new(0),
            rejected_oversized: AtomicU64::new(0),
            active_subscriptions: AtomicU64::new(0),
            next_message_id: AtomicU64::new(1),
            mutation_events,
            lock_service: Mutex::new(ClientLockService::new()),
            audit: Mutex::new(AuditRecorder::new(Arc::clone(&audit_sink))),
            audit_sink,
            isolation: isolation.map(Mutex::new),
            profile_instrumentation: ClientSurfaceProfileInstrumentation::default(),
        })
    }
}

impl ClientSurfaceEdgeState {
    pub(super) fn limits(&self) -> ClientSurfaceLimits {
        self.limits
    }

    pub(super) fn dispatch_attempts(&self) -> u64 {
        self.dispatch_attempts.load(Ordering::SeqCst)
    }

    pub(super) fn state_mutations(&self) -> u64 {
        self.state_mutations.load(Ordering::SeqCst)
    }

    pub(super) fn rejected_anonymous(&self) -> u64 {
        self.rejected_anonymous.load(Ordering::SeqCst)
    }

    pub(super) fn rejected_oversized(&self) -> u64 {
        self.rejected_oversized.load(Ordering::SeqCst)
    }

    pub(super) fn active_subscriptions(&self) -> u64 {
        self.active_subscriptions.load(Ordering::SeqCst)
    }

    pub(super) fn set_profile_instrumentation_enabled(&self, enabled: bool) {
        self.profile_instrumentation
            .enabled
            .store(enabled, Ordering::Release);
    }

    pub(super) fn reset_profile_metrics(&self) {
        self.profile_instrumentation.reset();
    }

    pub(super) fn profile_metrics(&self) -> ClientSurfaceProfileMetrics {
        self.profile_instrumentation.snapshot()
    }

    pub(super) fn subscribe_mutations(
        &self,
        identity: &ClientIdentity,
    ) -> Result<ClientSurfaceMutationSubscriber, ClientSurfaceError> {
        self.validate_tenant_identity(identity, CLIENT_SUBSCRIPTIONS_PATH, None)?;
        Ok(ClientSurfaceMutationSubscriber {
            tenant: identity.tenant().to_owned(),
            receiver: self.mutation_events.subscribe(),
        })
    }

    pub(super) fn set_lock_leader_for_tests(&self, local_node_id: u64, leader_node_id: u64) {
        self.lock_service
            .lock()
            .expect("lock service mutex")
            .set_leader_for_tests(local_node_id, leader_node_id);
    }

    pub(super) fn advance_lock_logical_time_for_tests(&self, millis: u64) {
        self.lock_service
            .lock()
            .expect("lock service mutex")
            .advance_time_for_tests(millis);
    }

    pub(super) fn audit_events_for_tests(&self) -> Vec<AuditEvent> {
        self.audit_sink.events()
    }

    pub(super) fn tenant_status(
        &self,
        identity: &ClientIdentity,
    ) -> Result<TenantStatus, ClientSurfaceError> {
        self.validate_tenant_identity(identity, CLIENT_STATUS_PATH, None)?;
        let metrics = if let Some(isolation) = &self.isolation {
            let tenant_id =
                TenantId::new(identity.tenant()).map_err(|_| ClientSurfaceError::Unauthorized)?;
            isolation
                .lock()
                .expect("isolation mutex")
                .metrics_snapshot_for_tenant(&tenant_id)
                .ok_or(ClientSurfaceError::Unauthorized)?
        } else {
            TenantMetricsSnapshot::default()
        };
        let active_subscriptions = if self.isolation.is_some() {
            metrics
                .tenant_subscriptions
                .get(identity.tenant())
                .copied()
                .unwrap_or_default()
        } else {
            self.active_subscriptions()
        };
        Ok(TenantStatus::from_metrics(
            identity.tenant(),
            &metrics,
            active_subscriptions,
            0,
        ))
    }

    pub(super) fn validate_tenant_identity(
        &self,
        identity: &ClientIdentity,
        route: &str,
        request_id: Option<&str>,
    ) -> Result<(), ClientSurfaceError> {
        if let Some(isolation) = &self.isolation {
            let tenant = isolation
                .lock()
                .expect("isolation mutex")
                .resolve_tenant(identity.client_id())
                .map_err(|_| ClientSurfaceError::Unauthorized);
            let authorized = tenant
                .as_ref()
                .is_ok_and(|tenant| tenant.as_str() == identity.tenant());
            if !authorized {
                let _ = self.record_audit(AuditEvent::AuthFailure {
                    tenant: Some(identity.tenant().to_owned()),
                    route: route.to_owned(),
                    request_id: request_id.map(ToOwned::to_owned),
                });
                return Err(ClientSurfaceError::Unauthorized);
            }
        }
        Ok(())
    }

    pub(super) fn reject_anonymous(&self) {
        self.rejected_anonymous.fetch_add(1, Ordering::SeqCst);
    }

    pub(super) fn reject_oversized(&self) {
        self.rejected_oversized.fetch_add(1, Ordering::SeqCst);
    }

    pub(super) fn record_dispatch(&self) {
        if self.profile_instrumentation.enabled() {
            self.profile_instrumentation
                .dispatch_counter_seqcst_operations
                .fetch_add(1, Ordering::Relaxed);
        }
        self.dispatch_attempts.fetch_add(1, Ordering::SeqCst);
        if self.profile_instrumentation.enabled() {
            self.profile_instrumentation
                .dispatches
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    pub(super) fn record_batch_shape(&self, items: usize) {
        if self.profile_instrumentation.enabled() {
            self.profile_instrumentation
                .batch_operations
                .fetch_add(1, Ordering::Relaxed);
            self.profile_instrumentation
                .batch_items
                .fetch_add(items as u64, Ordering::Relaxed);
        }
    }

    pub(super) fn record_state_mutations(&self, mutations: u64) {
        if self.profile_instrumentation.enabled() {
            self.profile_instrumentation
                .mutation_counter_seqcst_operations
                .fetch_add(1, Ordering::Relaxed);
        }
        self.state_mutations.fetch_add(mutations, Ordering::SeqCst);
    }

    pub(super) fn begin_subscription(&self) {
        self.active_subscriptions.fetch_add(1, Ordering::SeqCst);
    }

    pub(super) fn drain_subscriptions(&self) -> u64 {
        self.active_subscriptions.swap(0, Ordering::SeqCst)
    }

    pub(super) fn handle_try_lock(
        &self,
        identity: &ClientIdentity,
        request_id: String,
        ns: Namespace,
        key: StructuredKey,
        lease_ms: u64,
        level: LockConsistency,
    ) -> ClientResponseEnvelope {
        let lock_key = lock_key(identity, &ns, &key);
        let owner = lock_owner(identity);
        let lease = LogicalDuration::from_millis(lease_ms.max(1));
        let level = lock_consistency(level);
        let result = self
            .lock_service
            .lock()
            .expect("lock service mutex")
            .apply_lock_command(|store, now| {
                let acquired = store.try_acquire_lock(&lock_key, level, owner, lease, now)?;
                Ok(match acquired {
                    Some(fence) => ClientResponse::LockAcquired {
                        fence: fence.value(),
                    },
                    None => ClientResponse::LockBusy,
                })
            });
        lock_response(request_id, result)
    }

    pub(super) fn handle_unlock(
        &self,
        identity: &ClientIdentity,
        request_id: String,
        ns: Namespace,
        key: StructuredKey,
        fence: u64,
    ) -> ClientResponseEnvelope {
        let lock_key = lock_key(identity, &ns, &key);
        let owner = lock_owner(identity);
        let result = self
            .lock_service
            .lock()
            .expect("lock service mutex")
            .apply_lock_command(|store, _now| {
                store.release_lock(&lock_key, &owner, FenceToken::new(fence))?;
                Ok(ClientResponse::LockReleased)
            });
        lock_response(request_id, result)
    }

    pub(super) fn handle_renew_lock(
        &self,
        identity: &ClientIdentity,
        request_id: String,
        ns: Namespace,
        key: StructuredKey,
        fence: u64,
        lease_ms: u64,
    ) -> ClientResponseEnvelope {
        let lock_key = lock_key(identity, &ns, &key);
        let owner = lock_owner(identity);
        let lease = LogicalDuration::from_millis(lease_ms.max(1));
        let result = self
            .lock_service
            .lock()
            .expect("lock service mutex")
            .apply_lock_command(|store, now| {
                store.renew_lease(
                    &lock_key,
                    &owner,
                    FenceToken::new(fence),
                    now.saturating_add(lease),
                )?;
                Ok(ClientResponse::LockLeaseRenewed)
            });
        lock_response(request_id, result)
    }

    pub(super) fn handle_force_unlock(
        &self,
        identity: &ClientIdentity,
        request_id: String,
        ns: Namespace,
        key: StructuredKey,
    ) -> ClientResponseEnvelope {
        if !identity.is_admin() {
            let _ = self.record_audit(AuditEvent::AuthFailure {
                tenant: Some(identity.tenant().to_owned()),
                route: CLIENT_DATA_PATH.to_owned(),
                request_id: Some(request_id.clone()),
            });
            return ClientResponseEnvelope::error(
                request_id,
                ClientErrorEnvelope::new(
                    ClientErrorCode::Unauthorized,
                    false,
                    "force_unlock requires admin privileges",
                ),
            );
        }
        if let Err(error) = self.record_audit(AuditEvent::PolicyChanged {
            namespace: ns.as_str().to_owned(),
            policy_epoch: ClusterEpoch::new(1),
            summary: "force_unlock".to_owned(),
        }) {
            return ClientResponseEnvelope::error(request_id, error);
        }
        let lock_key = lock_key(identity, &ns, &key);
        let result = self
            .lock_service
            .lock()
            .expect("lock service mutex")
            .apply_lock_command(|store, _now| {
                store.force_unlock(&lock_key);
                Ok(ClientResponse::LockReleased)
            });
        lock_response(request_id, result)
    }

    pub(super) fn handle_lock_ownership(
        &self,
        identity: &ClientIdentity,
        request_id: String,
        ns: Namespace,
        key: StructuredKey,
    ) -> ClientResponseEnvelope {
        let lock_key = lock_key(identity, &ns, &key);
        let mut service = self.lock_service.lock().expect("lock service mutex");
        if !service.is_leader() {
            return ClientResponseEnvelope::error(
                request_id,
                ClientErrorEnvelope::new(
                    ClientErrorCode::BackendUnavailable,
                    true,
                    format!(
                        "not leader for lock partition; leader={}",
                        service.leader_node_id
                    ),
                )
                .with_retry_after_ms(1),
            );
        }
        ClientResponseEnvelope::ok(request_id, service.current_ownership(&lock_key))
    }

    pub(super) fn record_audit(&self, event: AuditEvent) -> Result<(), ClientErrorEnvelope> {
        self.audit
            .lock()
            .expect("audit mutex")
            .record(&event)
            .map(|_| ())
            .map_err(|error| {
                ClientErrorEnvelope::new(
                    ClientErrorCode::BackendUnavailable,
                    false,
                    format!("mandatory audit unavailable: {error}"),
                )
            })
    }

    pub(super) fn record_rejection_audit(
        &self,
        identity: &ClientIdentity,
        namespace: &str,
        request_id: &str,
        response: &ClientResponseEnvelope,
        audit_auth_failure: bool,
    ) {
        let Err(error) = &response.result else {
            return;
        };
        match error.code {
            ClientErrorCode::TenantQuota | ClientErrorCode::RateLimited => {
                let _ = self.record_audit(AuditEvent::QuotaRejected {
                    tenant: identity.tenant().to_owned(),
                    namespace: namespace.to_owned(),
                    request_id: Some(request_id.to_owned()),
                });
            }
            ClientErrorCode::Unauthorized if audit_auth_failure => {
                let _ = self.record_audit(AuditEvent::AuthFailure {
                    tenant: Some(identity.tenant().to_owned()),
                    route: CLIENT_DATA_PATH.to_owned(),
                    request_id: Some(request_id.to_owned()),
                });
            }
            _ => {}
        }
    }

    pub(super) fn begin_tenant_subscription(
        &self,
        identity: &ClientIdentity,
    ) -> Result<(), ClientErrorEnvelope> {
        if let Some(isolation) = &self.isolation {
            isolation
                .lock()
                .expect("isolation mutex")
                .begin_subscription(identity.client_id())
                .map_err(admission_error)
        } else {
            Ok(())
        }
    }

    pub(super) fn record_invalidation(
        &self,
        identity: &ClientIdentity,
        ns: Namespace,
        key: StructuredKey,
        kind: ClientSurfaceMutationKind,
    ) {
        self.record_mutation(identity, ns, Some(key), kind);
    }

    pub(super) fn record_mutation(
        &self,
        identity: &ClientIdentity,
        namespace: Namespace,
        key: Option<StructuredKey>,
        kind: ClientSurfaceMutationKind,
    ) {
        if self.mutation_events.receiver_count() == 0 {
            return;
        }
        if self.profile_instrumentation.enabled() {
            self.profile_instrumentation
                .message_id_seqcst_operations
                .fetch_add(1, Ordering::Relaxed);
        }
        let message_id = self.next_message_id.fetch_add(1, Ordering::SeqCst);
        self.publish_mutation(identity, namespace, key, kind, message_id);
    }

    pub(super) fn publish_mutation(
        &self,
        identity: &ClientIdentity,
        namespace: Namespace,
        key: Option<StructuredKey>,
        kind: ClientSurfaceMutationKind,
        message_id: u64,
    ) {
        if self.mutation_events.receiver_count() > 0 {
            let _ = self.mutation_events.send(ClientSurfaceMutationEvent {
                tenant: identity.tenant().to_owned(),
                namespace,
                key,
                kind,
                message_id,
            });
        }
    }

    pub(super) fn record_namespace_mutation(
        &self,
        identity: &ClientIdentity,
        namespace: Namespace,
        kind: ClientSurfaceMutationKind,
    ) {
        self.record_mutation(identity, namespace, None, kind);
    }

    pub(super) fn handle_control_request(
        &self,
        identity: &ClientIdentity,
        envelope: ClientRequestEnvelope,
    ) -> ClientResponseEnvelope {
        match envelope.request {
            ClientRequest::SubscribeInvalidations {
                ns: _,
                region: _,
                from,
                include_value: _,
            } => {
                if let Err(error) = self.begin_tenant_subscription(identity) {
                    ClientResponseEnvelope::error(envelope.request_id, error)
                } else {
                    ClientResponseEnvelope::ok(
                        envelope.request_id,
                        ClientResponse::Subscribed { from },
                    )
                }
            }
            ClientRequest::SubscribeEntryEvents {
                ns: _,
                region: _,
                from,
                include_value: _,
                projection: _,
            } => {
                if let Err(error) = self.begin_tenant_subscription(identity) {
                    ClientResponseEnvelope::error(envelope.request_id, error)
                } else {
                    ClientResponseEnvelope::ok(
                        envelope.request_id,
                        ClientResponse::Subscribed { from },
                    )
                }
            }
            ClientRequest::TryLock {
                ns,
                key,
                lease_ms,
                wait_ms: _,
                level,
            } => self.handle_try_lock(identity, envelope.request_id, ns, key, lease_ms, level),
            ClientRequest::Unlock { ns, key, fence } => {
                self.handle_unlock(identity, envelope.request_id, ns, key, fence)
            }
            ClientRequest::RenewLockLease {
                ns,
                key,
                fence,
                lease_ms,
            } => self.handle_renew_lock(identity, envelope.request_id, ns, key, fence, lease_ms),
            ClientRequest::ForceUnlock { ns, key } => {
                self.handle_force_unlock(identity, envelope.request_id, ns, key)
            }
            ClientRequest::GetLockOwnership { ns, key } => {
                self.handle_lock_ownership(identity, envelope.request_id, ns, key)
            }
            _ => ClientResponseEnvelope::error(
                envelope.request_id,
                ClientErrorEnvelope::new(
                    ClientErrorCode::BackendUnavailable,
                    false,
                    "data operation is not an edge control request",
                ),
            ),
        }
    }
}
