use futures_util::Stream;
use hydracache_cluster_testkit::value_plane_admission_075::{
    AdmissionLimits, DataWork, FairAdmissionQueue, WorkItem,
};
use hydracache_cluster_testkit::value_plane_async_075::{
    AsyncOutcome, CancellationDisposition, InvocationRejection,
};
use hydracache_cluster_testkit::value_plane_cluster_admission_075::ClusterAdmissionBounds;
use hydracache_cluster_testkit::value_plane_cluster_auth_075::RuntimeMember;
use hydracache_cluster_testkit::value_plane_cluster_health_075::RuntimeMaintenance;
use hydracache_cluster_testkit::value_plane_cluster_runtime_075::{
    ClusterRuntimeConfig, RuntimeCallOptions,
};
use hydracache_cluster_testkit::value_plane_collection_075::CollectionBounds;
use hydracache_cluster_testkit::value_plane_model_075::MutationIdentity;
use hydracache_cluster_testkit::value_plane_security_075::*;
use hydracache_cluster_testkit::value_plane_service_075::*;
use hydracache_cluster_testkit::value_plane_service_cluster_075::*;
use hydracache_cluster_testkit::value_plane_service_registry_075::*;
use hydracache_cluster_testkit::value_plane_stream_075::{DemandStream, DemandStreamError};
use hydracache_cluster_testkit::value_plane_surface_075::*;
use hydracache_cluster_transport_axum::NodeCredential;
use std::future::Future;
use std::pin::Pin;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::task::{Context, Poll, Waker};

fn bounds() -> CollectionBounds {
    CollectionBounds {
        max_entries: 64,
        max_key_bytes: 128,
        max_value_bytes: 128,
        max_page_items: 16,
        max_scan_items: 128,
        max_response_bytes: 4096,
        max_bulk_mutation_items: 32,
    }
}
fn security() -> SecurityBounds {
    SecurityBounds {
        max_key_bytes: 128,
        max_value_bytes: 128,
        ..SecurityBounds::default()
    }
}
fn registry(cluster: bool) -> UnifiedServiceRegistry {
    if !cluster {
        return UnifiedServiceRegistry::new(
            RegistryBounds::default(),
            bounds(),
            ServiceLimits::default(),
            security(),
            TrustEpochWindow::new(1, 3).unwrap(),
        )
        .unwrap();
    }
    UnifiedServiceRegistry::new_clustered(
        RegistryBounds::default(),
        bounds(),
        ServiceLimits::default(),
        security(),
        TrustEpochWindow::new(1, 3).unwrap(),
        ClusterRuntimeConfig {
            cluster: "secret-cluster".into(),
            members: ["a", "b", "c"]
                .into_iter()
                .map(|node| RuntimeMember {
                    node: node.into(),
                    generation: 1,
                    credential: NodeCredential::new("key", format!("secret-{node}")),
                })
                .collect(),
            entry_node: "c".into(),
            bounds: ServiceClusterBounds {
                partitions: 2,
                max_frame_bytes: 1024 * 1024,
                ..ServiceClusterBounds::default()
            },
            admission: ClusterAdmissionBounds {
                max_replica_bytes: 64 * 1024 * 1024,
                max_repair_bytes: 64 * 1024 * 1024,
                ..ClusterAdmissionBounds::default()
            },
            max_replays: 128,
        },
    )
    .unwrap()
}
fn context(tenant: &str, namespace: &str, epoch: Option<u64>) -> AuthenticatedNamespace {
    let principal = IngressPrincipal {
        tenant: tenant.into(),
    };
    let verified = VerifiedMapCommand::new(
        SurfaceId::Hc2Rust,
        &principal,
        tenant,
        namespace,
        1,
        b"key".to_vec(),
        SurfaceOperation::Read,
    )
    .unwrap();
    let claims = RouteClaims {
        authenticated: true,
        authenticated_tenant: tenant,
        claimed_tenant: tenant,
        expected_namespace_generation: 1,
        actual_namespace_generation: 1,
        expected_topology_epoch: 1,
        actual_topology_epoch: 1,
        proxy_hops: 0,
        deadline_remaining_ticks: 1000,
        trust_epoch: 1,
        encoded_envelope_bytes: 64,
        key_bytes: 3,
        value_bytes: 0,
    };
    let auth = AuthenticatedNamespace::verify(
        &principal,
        &verified,
        &claims,
        security(),
        TrustEpochWindow::new(1, 3).unwrap(),
        0,
    )
    .unwrap();
    if let Some(epoch) = epoch {
        auth.with_grant(
            NamespaceGrant::new(
                tenant,
                namespace,
                1,
                epoch,
                GrantRights::ALL,
                GrantKeys::All,
            )
            .unwrap(),
        )
        .unwrap()
    } else {
        auth
    }
}
fn open(
    registry: &UnifiedServiceRegistry,
    tenant: &str,
    namespace: &str,
    epoch: Option<u64>,
) -> (AuthenticatedNamespace, RegistryHandle) {
    let auth = context(tenant, namespace, epoch);
    let handle = registry.open(&auth, auth.address()).unwrap();
    (auth, handle)
}
fn command(operation: &str, sequence: u64) -> ServiceCommand {
    let mut command = ServiceCommand::point(operation, &[0, 255]);
    if is_mutating_operation(operation) {
        command.value = Some(b"private-value".to_vec());
        command.identity = Some(MutationIdentity::new("private-client", sequence));
    }
    command
}
fn run(
    registry: &UnifiedServiceRegistry,
    handle: &RegistryHandle,
    auth: &AuthenticatedNamespace,
    command: ServiceCommand,
) -> RegistryResult {
    registry
        .enqueue(handle, auth, SurfaceId::Hc2Rust, command)
        .unwrap();
    registry.run_next().unwrap().unwrap().result.unwrap()
}
fn maintain(
    registry: &UnifiedServiceRegistry,
    handle: &RegistryHandle,
    auth: &AuthenticatedNamespace,
    node: &str,
    live: bool,
) {
    registry
        .enqueue_maintenance(
            handle,
            auth,
            RuntimeMaintenance::SetNodeLive {
                node: node.into(),
                live,
            },
        )
        .unwrap();
    registry.run_next().unwrap().unwrap().result.unwrap();
}

#[test]
fn clustered_read_futures_never_reserve_journals_and_preserve_bulk_input_order() {
    let registry = registry(true);
    let (auth, handle) = open(&registry, "secret-tenant", "secret-map", None);
    run(&registry, &handle, &auth, command("put", 1));
    let mut other = command("put", 2);
    other.key = vec![0, 0];
    other.value = Some(b"other".to_vec());
    run(&registry, &handle, &auth, other);
    let before = registry.diagnostics(&handle, &auth).unwrap();
    let mut get = command("get_all", 0);
    get.keys = vec![vec![0, 0], vec![0, 255], vec![99]];
    let (mut future, control) = registry
        .cluster_read_invocation(
            &handle,
            &auth,
            SurfaceId::Resp,
            get,
            RuntimeCallOptions::default(),
        )
        .unwrap();
    let mut cx = Context::from_waker(Waker::noop());
    assert!(Pin::new(&mut future).poll(&mut cx).is_pending());
    assert_eq!(control.phase(), ClusterInvocationPhase::Claimed);
    let Poll::Ready(AsyncOutcome::Completed(Ok(result))) = Pin::new(&mut future).poll(&mut cx)
    else {
        panic!("read must complete without reservation");
    };
    assert_eq!(
        result
            .result
            .items
            .iter()
            .map(|item| item.key.clone())
            .collect::<Vec<_>>(),
        vec![vec![0, 0], vec![0, 255], vec![99]]
    );
    assert_eq!(
        result.result.items[0].value.as_deref(),
        Some(b"other".as_slice())
    );
    let after = registry.diagnostics(&handle, &auth).unwrap();
    assert_eq!(
        after.owners.completed_outcomes,
        before.owners.completed_outcomes
    );
    assert_eq!(after.owners.pending, before.owners.pending);
    assert_eq!(after.owners.values, before.owners.values);
    assert_eq!(registry.usage().reserved_growth, 0);
    assert!(registry
        .cluster_invocation(
            &handle,
            &auth,
            SurfaceId::Resp,
            command("get", 0),
            RuntimeCallOptions::default()
        )
        .is_err());
    assert!(registry
        .cluster_read_invocation(
            &handle,
            &auth,
            SurfaceId::Resp,
            command("put", 3),
            RuntimeCallOptions::default()
        )
        .is_err());
}

#[test]
fn read_cancellation_deadline_and_namespace_revocation_precede_dispatch() {
    let registry = registry(true);
    registry.advance_policy_epoch(1).unwrap();
    let (auth, handle) = open(&registry, "a", "map", Some(1));
    let before = registry.diagnostics(&handle, &auth).unwrap();
    let mut cx = Context::from_waker(Waker::noop());
    let (mut cancelled, control) = registry
        .cluster_read_invocation(
            &handle,
            &auth,
            SurfaceId::Resp,
            command("get", 0),
            RuntimeCallOptions::default(),
        )
        .unwrap();
    assert!(Pin::new(&mut cancelled).poll(&mut cx).is_pending());
    assert_eq!(control.cancel(), CancellationDisposition::NotApplied);
    assert!(matches!(
        Pin::new(&mut cancelled).poll(&mut cx),
        Poll::Ready(AsyncOutcome::NotApplied(InvocationRejection::Cancelled))
    ));
    assert_eq!(registry.diagnostics(&handle, &auth).unwrap(), before);
    let mut read = command("get", 0);
    read.deadline = 1;
    let (mut late, _) = registry
        .cluster_read_invocation(
            &handle,
            &auth,
            SurfaceId::Resp,
            read,
            RuntimeCallOptions::default(),
        )
        .unwrap();
    registry.advance_clock(1).unwrap();
    assert!(matches!(
        Pin::new(&mut late).poll(&mut cx),
        Poll::Ready(AsyncOutcome::NotApplied(InvocationRejection::Deadline))
    ));
    let (mut revoked, _) = registry
        .cluster_read_invocation(
            &handle,
            &auth,
            SurfaceId::Resp,
            command("get", 0),
            RuntimeCallOptions::default(),
        )
        .unwrap();
    assert!(Pin::new(&mut revoked).poll(&mut cx).is_pending());
    registry
        .advance_namespace_policy_epoch(&handle, &auth, 2)
        .unwrap();
    assert!(matches!(
        Pin::new(&mut revoked).poll(&mut cx),
        Poll::Ready(AsyncOutcome::Completed(Err(RegistryError::Security(
            SecurityError::PolicyEpochMismatch
        ))))
    ));
    assert_eq!(registry.usage().queued_requests, 0);
    assert_eq!(registry.usage().reserved_growth, 0);
}

#[test]
fn opt_in_retry_preserves_identity_and_completes_after_actual_backup_recovery() {
    let registry = registry(true);
    let (auth, handle) = open(&registry, "a", "map", None);
    maintain(&registry, &handle, &auth, "b", false);
    let mut cx = Context::from_waker(Waker::noop());
    let (mut future, control) = registry
        .cluster_invocation_with_retry(
            &handle,
            &auth,
            SurfaceId::Hc2Java,
            command("put", 1),
            RuntimeCallOptions::default(),
            ClusterRetryPolicy::new(3).unwrap(),
        )
        .unwrap();
    assert!(Pin::new(&mut future).poll(&mut cx).is_pending());
    assert!(Pin::new(&mut future).poll(&mut cx).is_pending());
    assert_eq!(control.attempts(), 2);
    maintain(&registry, &handle, &auth, "b", true);
    assert!(Pin::new(&mut future).poll(&mut cx).is_pending());
    let Poll::Ready(AsyncOutcome::Completed(Ok(result))) = Pin::new(&mut future).poll(&mut cx)
    else {
        panic!("retry failed: {:?}", control.last_error());
    };
    assert!(result.backup_proved);
    assert_eq!(result.result.kind, "inserted");
    assert_eq!(control.attempts(), 2);
    assert_eq!(
        registry.diagnostics(&handle, &auth).unwrap().owners.values,
        2
    );
    assert_eq!(registry.usage().reserved_growth, 0);
    let RegistryResult::ClusterCommand(replay) = run(&registry, &handle, &auth, command("put", 1))
    else {
        panic!("expected cluster replay");
    };
    assert!(replay.replayed);
}

#[test]
fn retry_budget_original_deadline_and_explicit_epoch_are_never_relaxed() {
    let registry = registry(true);
    let (auth, handle) = open(&registry, "a", "map", None);
    let mut cx = Context::from_waker(Waker::noop());
    let call = ServiceClusterCall {
        expected_epoch: Some(99),
        ..ServiceClusterCall::default()
    };
    let (mut fenced, control) = registry
        .cluster_invocation_with_retry(
            &handle,
            &auth,
            SurfaceId::Resp,
            command("put", 1),
            RuntimeCallOptions {
                call,
                ..RuntimeCallOptions::default()
            },
            ClusterRetryPolicy::new(16).unwrap(),
        )
        .unwrap();
    assert!(Pin::new(&mut fenced).poll(&mut cx).is_pending());
    assert!(Pin::new(&mut fenced).poll(&mut cx).is_pending());
    assert!(matches!(
        Pin::new(&mut fenced).poll(&mut cx),
        Poll::Ready(AsyncOutcome::OutcomeUnknown)
    ));
    assert_eq!(control.attempts(), 1);
    assert_eq!(
        control.last_error(),
        Some(RegistryError::Cluster(ServiceClusterError::StaleEpoch))
    );
    maintain(&registry, &handle, &auth, "b", false);
    let mut put = command("put", 2);
    put.deadline = 1;
    let (mut future, control) = registry
        .cluster_invocation_with_retry(
            &handle,
            &auth,
            SurfaceId::Resp,
            put,
            RuntimeCallOptions::default(),
            ClusterRetryPolicy::new(2).unwrap(),
        )
        .unwrap();
    assert!(Pin::new(&mut future).poll(&mut cx).is_pending());
    assert!(Pin::new(&mut future).poll(&mut cx).is_pending());
    registry.advance_clock(1).unwrap();
    assert!(matches!(
        Pin::new(&mut future).poll(&mut cx),
        Poll::Ready(AsyncOutcome::OutcomeUnknown)
    ));
    assert_eq!(control.attempts(), 2);
    assert_eq!(
        registry.diagnostics(&handle, &auth).unwrap().owners.values,
        0
    );
    assert_eq!(registry.usage().reserved_growth, 0);
    assert!(ClusterRetryPolicy::new(0).is_err());
    assert!(ClusterRetryPolicy::new(17).is_err());
}

#[test]
fn targeted_policy_revocation_preserves_other_namespaces_and_global_revoke_remains_global() {
    for clustered in [false, true] {
        let registry = registry(clustered);
        registry.advance_policy_epoch(1).unwrap();
        let (a, ah) = open(&registry, "a", "map", Some(1));
        let (b, bh) = open(&registry, "b", "map", Some(1));
        registry
            .enqueue(&ah, &a, SurfaceId::Resp, command("put", 1))
            .unwrap();
        let keep = registry
            .enqueue(&bh, &b, SurfaceId::Resp, command("put", 1))
            .unwrap();
        registry.advance_namespace_policy_epoch(&ah, &a, 2).unwrap();
        assert_eq!(registry.usage().queued_requests, 1);
        let result = registry.run_next().unwrap().unwrap();
        assert_eq!(result.ticket, keep);
        result.result.unwrap();
        assert!(registry
            .enqueue(&ah, &a, SurfaceId::Resp, command("get", 0))
            .is_err());
        assert!(registry
            .enqueue(&bh, &b, SurfaceId::Resp, command("get", 0))
            .is_ok());
        let fresh = context("a", "map", Some(2));
        assert!(registry
            .advance_namespace_policy_epoch(&ah, &fresh, 2)
            .is_err());
        assert!(registry.advance_policy_epoch(2).is_err());
        registry.advance_policy_epoch(3).unwrap();
        assert_eq!(registry.usage().queued_requests, 0);
        assert!(registry
            .enqueue(&bh, &b, SurfaceId::Resp, command("get", 0))
            .is_err());
        assert!(registry
            .enqueue(&ah, &fresh, SurfaceId::Resp, command("get", 0))
            .is_err());
        assert_eq!(registry.usage().reserved_growth, 0);
    }
}

#[test]
fn fair_local_targeted_cancellation_preserves_round_robin_and_safety_order() {
    let mut queue = FairAdmissionQueue::new(AdmissionLimits {
        total: 8,
        per_tenant: 4,
        per_partition: 4,
        safety: 2,
    })
    .unwrap();
    for (tenant, id) in [("a", 1), ("b", 2), ("a", 3), ("c", 4)] {
        queue
            .admit_data(DataWork {
                tenant: tenant.into(),
                partition: 0,
                request_id: id,
            })
            .unwrap();
    }
    assert!(queue.cancel_request(3));
    assert!(!queue.cancel_request(99));
    assert!(matches!(
        queue.pop(),
        Some(WorkItem::Data(DataWork { request_id: 1, .. }))
    ));
    assert!(queue.cancel_request(2));
    queue.admit_safety("7").unwrap();
    assert!(queue.cancel_request(7));
    assert!(matches!(
        queue.pop(),
        Some(WorkItem::Data(DataWork { request_id: 4, .. }))
    ));
    assert!(queue.is_empty());
    assert_eq!(queue.active_tenants(), 0);
}

#[test]
fn invocation_diagnostics_are_paginated_redacted_and_bound_to_real_phase_and_credit_owners() {
    let registry = registry(true);
    registry.advance_policy_epoch(1).unwrap();
    let (auth, handle) = open(&registry, "secret-tenant", "secret-map", Some(1));
    let (other, other_handle) = open(&registry, "other", "map", Some(1));
    let (mut first, _) = registry
        .cluster_invocation(
            &handle,
            &auth,
            SurfaceId::Resp,
            command("put", 1),
            RuntimeCallOptions::default(),
        )
        .unwrap();
    let (_second, _) = registry
        .cluster_read_invocation(
            &handle,
            &auth,
            SurfaceId::Resp,
            command("get", 0),
            RuntimeCallOptions::default(),
        )
        .unwrap();
    let (_third, _) = registry
        .cluster_invocation(
            &handle,
            &auth,
            SurfaceId::Resp,
            command("put", 2),
            RuntimeCallOptions::default(),
        )
        .unwrap();
    let mut cx = Context::from_waker(Waker::noop());
    assert!(Pin::new(&mut first).poll(&mut cx).is_pending());
    let page = registry
        .invocation_diagnostics(&handle, &auth, 1, 416, None)
        .unwrap();
    assert_eq!(page.rows.len(), 1);
    assert_eq!(page.total_invocations, 3);
    assert!(page.encoded_bytes <= 416);
    assert_eq!(page.rows[0].phase, ClusterInvocationPhase::Claimed);
    assert!(page.rows[0].replica_bytes > 0);
    let cursor = page.next.clone().unwrap();
    let other_ticket = registry
        .enqueue(&other_handle, &other, SurfaceId::Resp, command("get", 0))
        .unwrap();
    let next = registry
        .invocation_diagnostics(&handle, &auth, 2, 512, Some(&cursor))
        .unwrap();
    assert_eq!(next.admission_watermark, page.admission_watermark);
    assert_eq!(next.rows.len(), 2);
    assert!(next.rows.iter().all(|row| row.ticket > page.rows[0].ticket));
    assert!(next.next.is_none());
    assert!(registry
        .cancel_cluster_queued(&other_handle, &other, other_ticket)
        .unwrap());
    for secret in [
        "secret-tenant",
        "secret-map",
        "private-client",
        "private-value",
        "secret-cluster",
    ] {
        assert!(!format!("{page:?}{next:?}").contains(secret));
    }
    assert!(registry
        .invocation_diagnostics(&other_handle, &other, 1, 416, Some(&cursor))
        .is_err());
    assert!(registry
        .invocation_diagnostics(&handle, &auth, 0, 416, None)
        .is_err());
    assert!(registry
        .invocation_diagnostics(&handle, &auth, 1, 415, None)
        .is_err());
    assert!(Pin::new(&mut first).poll(&mut cx).is_pending());
    assert!(registry
        .invocation_diagnostics(&handle, &auth, 1, 416, Some(&cursor))
        .is_err());
    let reserved = registry
        .invocation_diagnostics(&handle, &auth, 1, 416, None)
        .unwrap();
    assert_eq!(
        reserved.rows[0].reason,
        InvocationWaitReason::RootReservation
    );
    assert!(reserved
        .blocked_by
        .contains(&LeaseBlockReason::PendingOperation));
    let reader = auth
        .attenuate(
            GrantRights {
                read: true,
                ..GrantRights::default()
            },
            GrantKeys::All,
        )
        .unwrap();
    assert!(registry
        .invocation_diagnostics(&handle, &reader, 1, 416, None)
        .is_err());
    drop(first);
}

#[test]
fn namespace_revocation_fences_only_its_own_stream_and_keeps_other_buffered_values() {
    let registry = registry(true);
    registry.advance_policy_epoch(1).unwrap();
    let (a, ah) = open(&registry, "a", "map", Some(1));
    let (b, bh) = open(&registry, "b", "map", Some(1));
    let options = ClusterSubscriptionOptions {
        listener: ServiceListenerOptions {
            max_value_bytes: 128,
            queue_capacity: 8,
            include_value: true,
            ..ServiceListenerOptions::default()
        },
        max_queued_events: 16,
        max_queued_bytes: 1024,
    };
    let mut astream = DemandStream::new(
        registry
            .cluster_listener_source(&ah, &a, options.clone())
            .unwrap(),
        2,
    )
    .unwrap();
    let mut bstream = DemandStream::new(
        registry.cluster_listener_source(&bh, &b, options).unwrap(),
        2,
    )
    .unwrap();
    let mut cx = Context::from_waker(Waker::noop());
    assert!(Pin::new(&mut astream).poll_next(&mut cx).is_pending());
    run(&registry, &bh, &b, command("put", 1));
    let mut second = command("put", 2);
    second.key = vec![0, 0];
    run(&registry, &bh, &b, second);
    assert!(matches!(
        Pin::new(&mut bstream).poll_next(&mut cx),
        Poll::Ready(Some(Ok(_)))
    ));
    assert_eq!(bstream.retained_items(), 1);
    registry.advance_namespace_policy_epoch(&ah, &a, 2).unwrap();
    assert!(matches!(
        Pin::new(&mut astream).poll_next(&mut cx),
        Poll::Ready(Some(Err(DemandStreamError::Source(
            RegistryError::Security(SecurityError::PolicyEpochMismatch)
        ))))
    ));
    assert!(matches!(
        Pin::new(&mut bstream).poll_next(&mut cx),
        Poll::Ready(Some(Ok(ClusterSubscriptionEvent::Entry { .. })))
    ));
    bstream.cancel();
    assert_eq!(registry.diagnostics(&bh, &b).unwrap().owners.streams, 0);
    let current = context("a", "map", Some(2));
    assert_eq!(
        registry.diagnostics(&ah, &current).unwrap().owners.streams,
        0
    );
}

#[test]
fn registry_typed_helper_dispatch_uses_actual_admission_and_does_not_create_query_outcomes() {
    use hydracache_cluster_testkit::value_plane_cluster_runtime_075::{
        ClusterHelperCommand, ClusterHelperRequest, ClusterHelperResult,
    };
    let registry = registry(true);
    registry.advance_policy_epoch(1).unwrap();
    let (auth, handle) = open(&registry, "a", "map", Some(1));
    run(&registry, &handle, &auth, command("put", 1));
    let before = registry.diagnostics(&handle, &auth).unwrap();
    let request = ClusterHelperRequest::new(
        MutationIdentity::new("query", 1),
        ClusterHelperCommand::QueryOutcomes {
            identities: vec![
                MutationIdentity::new("private-client", 1),
                MutationIdentity::new("unknown", 1),
            ],
            max_bytes: 4096,
        },
    );
    registry
        .enqueue_helper(
            &handle,
            &auth,
            SurfaceId::Hc2Java,
            request,
            RuntimeCallOptions::default(),
        )
        .unwrap();
    let metadata = registry
        .invocation_diagnostics(&handle, &auth, 1, 416, None)
        .unwrap();
    assert_eq!(metadata.rows[0].class, InvocationClass::TypedHelper);
    let RegistryResult::ClusterHelper(result) =
        registry.run_next().unwrap().unwrap().result.unwrap()
    else {
        panic!("expected typed helper");
    };
    let ClusterHelperResult::Outcomes(outcomes) = result.result else {
        panic!("expected outcomes");
    };
    assert!(matches!(outcomes[0], OperationOutcomeQuery::Completed(_)));
    assert_eq!(outcomes[1], OperationOutcomeQuery::Unknown);
    let after = registry.diagnostics(&handle, &auth).unwrap();
    assert_eq!(
        before.owners.completed_outcomes,
        after.owners.completed_outcomes
    );
    assert_eq!(before.owners.pending, after.owners.pending);
    assert_eq!(registry.usage().reserved_growth, 0);
    assert_eq!(
        registry
            .cluster_admission_observations()
            .unwrap()
            .replica_bytes,
        0
    );
}

#[test]
fn read_future_returns_owner_errors_without_claiming_a_mutation_unknown() {
    let registry = registry(true);
    let (auth, handle) = open(&registry, "a", "map", None);
    maintain(&registry, &handle, &auth, "b", false);
    let before = registry.diagnostics(&handle, &auth).unwrap();
    let (mut future, _) = registry
        .cluster_read_invocation(
            &handle,
            &auth,
            SurfaceId::Hc1,
            command("get", 0),
            RuntimeCallOptions::default(),
        )
        .unwrap();
    let mut cx = Context::from_waker(Waker::noop());
    assert!(Pin::new(&mut future).poll(&mut cx).is_pending());
    assert!(matches!(
        Pin::new(&mut future).poll(&mut cx),
        Poll::Ready(AsyncOutcome::Completed(Err(RegistryError::Cluster(
            ServiceClusterError::NodeUnavailable
        ))))
    ));
    let after = registry.diagnostics(&handle, &auth).unwrap();
    assert_eq!(
        before.owners.completed_outcomes,
        after.owners.completed_outcomes
    );
    assert_eq!(before.owners.pending, after.owners.pending);
    assert_eq!(registry.usage().reserved_growth, 0);
}

#[test]
fn read_grants_cover_every_bulk_key_before_admission_without_copying_the_payload() {
    let registry = registry(true);
    registry.advance_policy_epoch(1).unwrap();
    let (admin, handle) = open(&registry, "a", "map", Some(1));
    let reader = admin
        .attenuate(
            GrantRights {
                read: true,
                ..GrantRights::default()
            },
            GrantKeys::Prefixes(vec![b"a".to_vec()]),
        )
        .unwrap();
    let before = registry.diagnostics(&handle, &admin).unwrap();
    let mut read = command("get_all", 0);
    read.keys = vec![b"allowed".to_vec(), b"denied".to_vec()];
    assert!(matches!(
        registry.cluster_read_invocation(
            &handle,
            &reader,
            SurfaceId::Resp,
            read,
            RuntimeCallOptions::default()
        ),
        Err(RegistryError::Security(SecurityError::PermissionDenied))
    ));
    let mut malformed = command("get_all", 0);
    malformed.keys = vec![vec![b'a'; 129]; 257];
    assert!(registry
        .cluster_read_invocation(
            &handle,
            &admin,
            SurfaceId::Resp,
            malformed,
            RuntimeCallOptions::default()
        )
        .is_err());
    assert_eq!(registry.diagnostics(&handle, &admin).unwrap(), before);
    assert_eq!(registry.usage().queued_requests, 0);
}

#[test]
fn retrying_a_partial_bulk_preserves_confirmed_items_without_claiming_unproved_success() {
    use hydracache_cluster_testkit::value_plane_collection_075::ReplacementEntry;
    use hydracache_cluster_testkit::value_plane_loopback_075::LoopbackFault;
    let registry = registry(true);
    let (auth, handle) = open(&registry, "a", "map", None);
    let mut bulk = command("put_all_detailed", 1);
    bulk.ttl = ServiceTtl::ExpireAfter(10);
    bulk.entries = vec![
        ReplacementEntry {
            key: vec![0, 0],
            value: b"zero".to_vec(),
        },
        ReplacementEntry {
            key: vec![0, 255],
            value: b"one".to_vec(),
        },
    ];
    let mut call = ServiceClusterCall::default();
    call.faults.partitions.insert(
        0,
        ServiceLinkFaults {
            request: LoopbackFault::Drop,
            ..ServiceLinkFaults::default()
        },
    );
    let (mut future, control) = registry
        .cluster_invocation_with_retry(
            &handle,
            &auth,
            SurfaceId::Resp,
            bulk,
            RuntimeCallOptions {
                call,
                ..RuntimeCallOptions::default()
            },
            ClusterRetryPolicy::new(2).unwrap(),
        )
        .unwrap();
    let mut cx = Context::from_waker(Waker::noop());
    assert!(Pin::new(&mut future).poll(&mut cx).is_pending());
    assert!(Pin::new(&mut future).poll(&mut cx).is_pending());
    assert!(Pin::new(&mut future).poll(&mut cx).is_pending());
    let applied = registry.diagnostics(&handle, &auth).unwrap();
    assert_eq!(applied.owners.values, 2);
    registry.advance_clock(3).unwrap();
    assert!(matches!(
        Pin::new(&mut future).poll(&mut cx),
        Poll::Ready(AsyncOutcome::OutcomeUnknown)
    ));
    assert_eq!(
        control.last_error(),
        Some(RegistryError::Cluster(
            ServiceClusterError::TransportTimeout
        ))
    );
    assert_eq!(
        registry.diagnostics(&handle, &auth).unwrap().owners.values,
        2
    );
    assert_eq!(control.attempts(), 2);
    assert_eq!(registry.usage().reserved_growth, 0);
    let RegistryResult::ClusterCommand(view) =
        run(&registry, &handle, &auth, command("get_entry_view", 0))
    else {
        panic!("expected actual entry view");
    };
    assert_eq!(view.result.version, Some(2));
    assert_eq!(view.result.remaining_ttl, Some(7));
    let mut missing = command("get", 0);
    missing.key = vec![0, 0];
    let RegistryResult::ClusterCommand(missing) = run(&registry, &handle, &auth, missing) else {
        panic!("expected actual read");
    };
    assert_eq!(missing.result.kind, "absent");
}

#[test]
fn retry_exhaustion_and_authorization_failures_release_each_actual_admission_once() {
    use hydracache_cluster_testkit::value_plane_cluster_auth_075::{AuthFault, EnvelopeFault};
    let registry = registry(true);
    let (auth, handle) = open(&registry, "a", "map", None);
    let mut cx = Context::from_waker(Waker::noop());
    maintain(&registry, &handle, &auth, "b", false);
    let (mut exhausted, control) = registry
        .cluster_invocation_with_retry(
            &handle,
            &auth,
            SurfaceId::Resp,
            command("put", 1),
            RuntimeCallOptions::default(),
            ClusterRetryPolicy::new(2).unwrap(),
        )
        .unwrap();
    assert!(Pin::new(&mut exhausted).poll(&mut cx).is_pending());
    assert!(Pin::new(&mut exhausted).poll(&mut cx).is_pending());
    assert!(matches!(
        Pin::new(&mut exhausted).poll(&mut cx),
        Poll::Ready(AsyncOutcome::OutcomeUnknown)
    ));
    assert_eq!(control.attempts(), 2);
    maintain(&registry, &handle, &auth, "b", true);
    let before = registry.diagnostics(&handle, &auth).unwrap().owners;
    let (mut forged, control) = registry
        .cluster_invocation_with_retry(
            &handle,
            &auth,
            SurfaceId::Resp,
            command("put", 2),
            RuntimeCallOptions {
                auth_fault: Some(AuthFault {
                    stage: ServiceMessageKind::Request,
                    fault: EnvelopeFault::AdmissionDigest,
                }),
                ..RuntimeCallOptions::default()
            },
            ClusterRetryPolicy::new(16).unwrap(),
        )
        .unwrap();
    assert!(Pin::new(&mut forged).poll(&mut cx).is_pending());
    assert!(matches!(
        Pin::new(&mut forged).poll(&mut cx),
        Poll::Ready(AsyncOutcome::OutcomeUnknown)
    ));
    assert_eq!(control.attempts(), 1);
    assert_eq!(registry.diagnostics(&handle, &auth).unwrap().owners, before);
    assert_eq!(registry.usage().queued_requests, 0);
    assert_eq!(registry.usage().reserved_growth, 0);
    assert_eq!(
        registry.cluster_admission_observations().unwrap().inflight,
        [0; 3]
    );
}

fn owned_payload(usage: ServiceResourceUsage) -> usize {
    usage.values + usage.outcomes + usage.pending + usage.listeners + usage.streams + usage.resync
}
fn seed_empty_values(
    registry: &UnifiedServiceRegistry,
    handle: &RegistryHandle,
    auth: &AuthenticatedNamespace,
) {
    for sequence in 1..49 {
        let mut put = command("put", sequence);
        put.key = vec![(sequence * 2) as u8, 0];
        put.value = Some(Vec::new());
        run(registry, handle, auth, put);
    }
}

#[test]
fn stepped_transfer_reserves_serialized_snapshot_and_lease_growth_before_global_admission() {
    let baseline = registry(true);
    let (auth, handle) = open(&baseline, "a", "map", None);
    seed_empty_values(&baseline, &handle, &auth);
    let before = owned_payload(baseline.diagnostics(&handle, &auth).unwrap().resources);
    let old_limit = before * 2 + 512;
    let step = RuntimeMaintenance::RepairStep {
        partition: 0,
        backup: "c".into(),
        faults: ServiceLinkFaults::default(),
        maximum_work: 1,
    };
    baseline
        .enqueue_maintenance(&handle, &auth, step.clone())
        .unwrap();
    baseline.run_next().unwrap().unwrap().result.unwrap();
    assert!(
        owned_payload(baseline.diagnostics(&handle, &auth).unwrap().resources) > old_limit,
        "fixture must exercise retained serialized growth beyond the old reservation"
    );
    let capped = UnifiedServiceRegistry::new_clustered(
        RegistryBounds {
            max_owned_bytes: old_limit,
            max_tenant_owned_bytes: old_limit,
            ..RegistryBounds::default()
        },
        bounds(),
        ServiceLimits::default(),
        security(),
        TrustEpochWindow::new(1, 3).unwrap(),
        ClusterRuntimeConfig {
            cluster: "secret-cluster".into(),
            members: ["a", "b", "c"]
                .into_iter()
                .map(|node| RuntimeMember {
                    node: node.into(),
                    generation: 1,
                    credential: NodeCredential::new("key", format!("secret-{node}")),
                })
                .collect(),
            entry_node: "c".into(),
            bounds: ServiceClusterBounds {
                partitions: 2,
                max_frame_bytes: 1024 * 1024,
                ..ServiceClusterBounds::default()
            },
            admission: ClusterAdmissionBounds {
                max_replica_bytes: 64 * 1024 * 1024,
                max_repair_bytes: 64 * 1024 * 1024,
                ..ClusterAdmissionBounds::default()
            },
            max_replays: 4096,
        },
    )
    .unwrap();
    let (auth, handle) = open(&capped, "a", "map", None);
    seed_empty_values(&capped, &handle, &auth);
    let unchanged = capped.diagnostics(&handle, &auth).unwrap();
    assert!(matches!(
        capped.enqueue_maintenance(&handle, &auth, step),
        Err(RegistryError::Quota { .. })
    ));
    assert_eq!(capped.diagnostics(&handle, &auth).unwrap(), unchanged);
    assert_eq!(capped.usage().queued_requests, 0);
    assert_eq!(capped.usage().reserved_growth, 0);
    assert!(capped
        .cluster_health(&handle, &auth)
        .unwrap()
        .reasons
        .is_empty());
}

struct AdvanceClockOnAdmission {
    registry: UnifiedServiceRegistry,
    armed: AtomicBool,
}
impl std::task::Wake for AdvanceClockOnAdmission {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        if self.armed.swap(false, Ordering::SeqCst) {
            self.registry.advance_clock(1000).unwrap();
        }
    }
}
#[test]
fn invocation_control_is_attached_before_reentrant_admission_expiry_can_orphan_a_job() {
    let registry = registry(true);
    let (auth, handle) = open(&registry, "a", "map", None);
    let notice = Arc::new(AdvanceClockOnAdmission {
        registry: registry.clone(),
        armed: AtomicBool::new(false),
    });
    let waker = Waker::from(Arc::clone(&notice));
    let mut cx = Context::from_waker(&waker);
    let (mut first, _) = registry
        .cluster_invocation(
            &handle,
            &auth,
            SurfaceId::Resp,
            command("put", 1),
            RuntimeCallOptions::default(),
        )
        .unwrap();
    assert!(Pin::new(&mut first).poll(&mut cx).is_pending());
    notice.armed.store(true, Ordering::SeqCst);
    let (mut admitted, _) = registry
        .cluster_read_invocation(
            &handle,
            &auth,
            SurfaceId::Hc2Rust,
            command("get", 0),
            RuntimeCallOptions::default(),
        )
        .expect("admission returns the owning future after an out-of-lock clock wake");
    assert!(matches!(
        Pin::new(&mut admitted).poll(&mut cx),
        Poll::Ready(AsyncOutcome::Completed(Err(RegistryError::Security(
            SecurityError::DeadlineExpired
        ))))
    ));
    drop(first);
    drop(admitted);
    assert_eq!(registry.usage().queued_requests, 0);
    assert_eq!(registry.usage().reserved_growth, 0);
    assert_eq!(
        registry.cluster_admission_observations().unwrap().inflight,
        [0; 3]
    );
    assert!(registry.run_next().unwrap().is_none());
}
