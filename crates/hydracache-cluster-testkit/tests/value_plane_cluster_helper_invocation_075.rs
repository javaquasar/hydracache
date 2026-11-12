use hydracache_cluster_testkit::value_plane_cluster_admission_075::*;
use hydracache_cluster_testkit::value_plane_cluster_auth_075::*;
use hydracache_cluster_testkit::value_plane_cluster_runtime_075::*;
use hydracache_cluster_testkit::value_plane_collection_075::CollectionBounds;
use hydracache_cluster_testkit::value_plane_loopback_075::LoopbackFault;
use hydracache_cluster_testkit::value_plane_model_075::MutationIdentity;
use hydracache_cluster_testkit::value_plane_security_075::*;
use hydracache_cluster_testkit::value_plane_service_075::*;
use hydracache_cluster_testkit::value_plane_service_cluster_075::*;
use hydracache_cluster_testkit::value_plane_service_registry_075::*;
use hydracache_cluster_testkit::value_plane_surface_075::*;
use hydracache_cluster_transport_axum::NodeCredential;

fn fixture() -> (
    UnifiedServiceRegistry,
    AuthenticatedNamespace,
    RegistryHandle,
) {
    fixture_with_admission(ClusterAdmissionBounds {
        max_replica_bytes: 64 * 1024 * 1024,
        ..Default::default()
    })
}
fn fixture_with_admission(
    admission: ClusterAdmissionBounds,
) -> (
    UnifiedServiceRegistry,
    AuthenticatedNamespace,
    RegistryHandle,
) {
    let principal = IngressPrincipal {
        tenant: "tenant".into(),
    };
    let verified = VerifiedMapCommand::new(
        SurfaceId::Hc2Rust,
        &principal,
        "tenant",
        "map",
        1,
        b"k".to_vec(),
        SurfaceOperation::Read,
    )
    .unwrap();
    let security = SecurityBounds::default();
    let claims = RouteClaims {
        authenticated: true,
        authenticated_tenant: "tenant",
        claimed_tenant: "tenant",
        expected_namespace_generation: 1,
        actual_namespace_generation: 1,
        expected_topology_epoch: 1,
        actual_topology_epoch: 1,
        proxy_hops: 0,
        deadline_remaining_ticks: 10_000,
        trust_epoch: 1,
        encoded_envelope_bytes: 64,
        key_bytes: 1,
        value_bytes: 0,
    };
    let authenticated = AuthenticatedNamespace::verify(
        &principal,
        &verified,
        &claims,
        security,
        TrustEpochWindow::new(1, 3).unwrap(),
        0,
    )
    .unwrap();
    let config = ClusterRuntimeConfig {
        cluster: "test".into(),
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
            ..ServiceClusterBounds::default()
        },
        admission,
        max_replays: 4_096,
    };
    let registry = UnifiedServiceRegistry::new_clustered(
        RegistryBounds {
            partitions: 2,
            ..RegistryBounds::default()
        },
        CollectionBounds::default(),
        ServiceLimits::default(),
        security,
        TrustEpochWindow::new(1, 3).unwrap(),
        config,
    )
    .unwrap();
    let handle = registry
        .open(&authenticated, authenticated.address())
        .unwrap();
    (registry, authenticated, handle)
}
fn command(
    registry: &UnifiedServiceRegistry,
    auth: &AuthenticatedNamespace,
    handle: &RegistryHandle,
    operation: &str,
    key: &[u8],
    sequence: u64,
    ttl: ServiceTtl,
) -> ServiceClusterExecution {
    let mut command = ServiceCommand::point(operation, key);
    command.value = Some(b"old".to_vec());
    command.ttl = ttl;
    if sequence != 0 {
        command.identity = Some(MutationIdentity::new("values", sequence));
    }
    let ticket = registry
        .enqueue(handle, auth, SurfaceId::Hc2Rust, command)
        .unwrap();
    let result = registry.run_next().unwrap().unwrap();
    assert_eq!(result.ticket, ticket);
    match result.result.unwrap() {
        RegistryResult::ClusterCommand(result) => result,
        other => panic!("{other:?}"),
    }
}
fn helper(
    registry: &UnifiedServiceRegistry,
    auth: &AuthenticatedNamespace,
    handle: &RegistryHandle,
    request: ClusterHelperRequest,
    options: RuntimeCallOptions,
) -> ClusterHelperExecution {
    let ticket = registry
        .enqueue_helper(handle, auth, SurfaceId::Hc2Rust, request, options)
        .unwrap();
    let record = registry.run_next().unwrap().unwrap();
    assert_eq!(record.ticket, ticket);
    match record.result.unwrap() {
        RegistryResult::ClusterHelper(result) => result,
        other => panic!("{other:?}"),
    }
}
fn request(client: &str, sequence: u64, command: ClusterHelperCommand) -> ClusterHelperRequest {
    ClusterHelperRequest::new(MutationIdentity::new(client, sequence), command)
}

use hydracache_cluster_testkit::value_plane_async_075::{
    AsyncOutcome, CancellationDisposition, InvocationRejection,
};
use hydracache_cluster_testkit::value_plane_cluster_health_075::RuntimeMaintenance;
use std::future::Future;
use std::pin::Pin;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::task::{Context, Poll, Wake, Waker};

struct Counter(AtomicUsize);
impl Wake for Counter {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}
fn poll<F: Future + Unpin>(future: &mut F) -> Poll<F::Output> {
    Pin::new(future).poll(&mut Context::from_waker(Waker::noop()))
}
fn ready<F: Future + Unpin>(future: &mut F) -> F::Output {
    for _ in 0..8 {
        if let Poll::Ready(output) = poll(future) {
            return output;
        }
    }
    panic!("invocation failed to finish in bounded polls")
}
fn live(
    registry: &UnifiedServiceRegistry,
    auth: &AuthenticatedNamespace,
    handle: &RegistryHandle,
    node: &str,
    value: bool,
) {
    registry
        .enqueue_maintenance(
            handle,
            auth,
            RuntimeMaintenance::SetNodeLive {
                node: node.into(),
                live: value,
            },
        )
        .unwrap();
    registry.run_next().unwrap().unwrap().result.unwrap();
}
fn replacement(
    registry: &UnifiedServiceRegistry,
    auth: &AuthenticatedNamespace,
    handle: &RegistryHandle,
    sequence: u64,
) -> ClusterHelperRequest {
    command(
        registry,
        auth,
        handle,
        "put",
        b"k",
        1,
        ServiceTtl::ExpireAfter(10),
    );
    let token = command(
        registry,
        auth,
        handle,
        "get_entry_view",
        b"k",
        0,
        ServiceTtl::Preserve,
    )
    .result
    .token
    .unwrap();
    request(
        "future",
        sequence,
        ClusterHelperCommand::VersionBulk {
            operation: VersionBulkOperation::Replace,
            entries: vec![VersionBulkEntry {
                key: b"k".to_vec(),
                token,
                value: Some(b"new".to_vec()),
                ttl: ServiceTtl::Preserve,
            }],
        },
    )
}
fn no_attempt_owners(registry: &UnifiedServiceRegistry) {
    assert_eq!(registry.usage().queued_requests, 0);
    assert_eq!(registry.usage().reserved_growth, 0);
    let admission = registry.cluster_admission_observations().unwrap();
    assert_eq!(admission.queued, [0; 3]);
    assert_eq!(admission.inflight, [0; 3]);
    assert_eq!(admission.replica_bytes, 0);
}

#[test]
fn clustered_helper_future_uses_real_root_and_cancels_without_reapplying() {
    let (registry, auth, handle) = fixture();
    let request = replacement(&registry, &auth, &handle, 1);
    let before = registry.diagnostics(&handle, &auth).unwrap();
    let (mut future, control) = registry
        .cluster_helper_invocation(
            &handle,
            &auth,
            SurfaceId::Hc2Java,
            request.clone(),
            RuntimeCallOptions::default(),
        )
        .unwrap();
    assert!(
        registry.run_next().unwrap().is_none(),
        "cooperative helper cannot be dispatched by run_next"
    );
    assert!(poll(&mut future).is_pending());
    assert_eq!(control.phase(), ClusterInvocationPhase::Claimed);
    assert_eq!(
        registry.diagnostics(&handle, &auth).unwrap().owners.pending,
        before.owners.pending
    );
    assert!(poll(&mut future).is_pending());
    assert_eq!(control.phase(), ClusterInvocationPhase::Reserved);
    assert!(registry.diagnostics(&handle, &auth).unwrap().owners.pending > before.owners.pending);
    let page = registry
        .invocation_diagnostics(&handle, &auth, 4, 4096, None)
        .unwrap();
    assert_eq!(page.rows[0].class, InvocationClass::TypedHelper);
    assert_eq!(control.cancel(), CancellationDisposition::OutcomeUnknown);
    assert!(matches!(ready(&mut future), AsyncOutcome::OutcomeUnknown));
    assert_eq!(control.cancel(), CancellationDisposition::AlreadyFinished);
    no_attempt_owners(&registry);
    assert_eq!(
        registry.diagnostics(&handle, &auth).unwrap().owners.pending,
        0
    );
    assert_eq!(
        command(
            &registry,
            &auth,
            &handle,
            "get",
            b"k",
            0,
            ServiceTtl::Preserve
        )
        .result
        .value,
        Some(b"old".to_vec())
    );
    let retried = helper(
        &registry,
        &auth,
        &handle,
        request,
        RuntimeCallOptions::default(),
    );
    assert!(matches!(
        retried.result,
        ClusterHelperResult::OutcomeUnknown
    ));
}

#[test]
fn helper_futures_complete_once_and_recover_original_versions() {
    let (registry, auth, handle) = fixture();
    let request = replacement(&registry, &auth, &handle, 1);
    let (mut future, control) = registry
        .cluster_helper_invocation(
            &handle,
            &auth,
            SurfaceId::Resp,
            request.clone(),
            RuntimeCallOptions::default(),
        )
        .unwrap();
    let AsyncOutcome::Completed(Ok(result)) = ready(&mut future) else {
        panic!("helper failed: {:?}", control.last_error())
    };
    let ClusterHelperResult::Versions(versions) = result.result else {
        panic!("expected versions")
    };
    assert!(result.backup_proved);
    assert_eq!(versions.items[0].kind, "replaced");
    let version = versions.items[0].version;
    let replay = helper(
        &registry,
        &auth,
        &handle,
        request,
        RuntimeCallOptions::default(),
    );
    assert!(replay.replayed);
    assert!(
        matches!(replay.result,ClusterHelperResult::Versions(result) if result.items[0].version==version)
    );
    assert!(matches!(
        poll(&mut future),
        Poll::Ready(AsyncOutcome::Completed(Err(RegistryError::InvalidBound(
            "invocation_finished"
        ))))
    ));
    no_attempt_owners(&registry);
}

#[test]
fn dropping_helper_at_each_boundary_releases_only_actual_attempt_owners() {
    for phase in 0..3 {
        let (registry, auth, handle) = fixture();
        let request = replacement(&registry, &auth, &handle, 1);
        let (mut future, control) = registry
            .cluster_helper_invocation(
                &handle,
                &auth,
                SurfaceId::Resp,
                request,
                RuntimeCallOptions::default(),
            )
            .unwrap();
        for _ in 0..phase {
            assert!(poll(&mut future).is_pending());
        }
        drop(future);
        assert_eq!(control.phase(), ClusterInvocationPhase::Finished);
        no_attempt_owners(&registry);
        assert_eq!(
            registry.diagnostics(&handle, &auth).unwrap().owners.pending,
            0
        );
        assert_eq!(
            command(
                &registry,
                &auth,
                &handle,
                "get",
                b"k",
                0,
                ServiceTtl::Preserve
            )
            .result
            .value,
            Some(b"old".to_vec())
        );
    }
}

#[test]
fn read_retry_parks_until_capacity_changes_and_never_reserves_a_journal() {
    let (registry, auth, handle) = fixture();
    command(
        &registry,
        &auth,
        &handle,
        "put",
        b"k",
        1,
        ServiceTtl::Eternal,
    );
    let owners = registry.diagnostics(&handle, &auth).unwrap().owners;
    live(&registry, &auth, &handle, "a", false);
    live(&registry, &auth, &handle, "b", false);
    let (mut future, control) = registry
        .cluster_read_invocation_with_retry(
            &handle,
            &auth,
            SurfaceId::Resp,
            ServiceCommand::point("get", b"k"),
            RuntimeCallOptions::default(),
            ClusterRetryPolicy::new(3).unwrap(),
        )
        .unwrap();
    let counter = Arc::new(Counter(AtomicUsize::new(0)));
    let waker = Waker::from(Arc::clone(&counter));
    let mut context = Context::from_waker(&waker);
    assert!(Pin::new(&mut future).poll(&mut context).is_pending());
    assert!(Pin::new(&mut future).poll(&mut context).is_pending());
    assert_eq!(control.attempts(), 2);
    let wake_count = counter.0.load(Ordering::SeqCst);
    for _ in 0..32 {
        assert!(Pin::new(&mut future).poll(&mut context).is_pending());
    }
    assert_eq!(control.attempts(), 2);
    assert_eq!(counter.0.load(Ordering::SeqCst), wake_count);
    assert_eq!(
        registry
            .invocation_diagnostics(&handle, &auth, 4, 4096, None)
            .unwrap()
            .rows[0]
            .reason,
        InvocationWaitReason::Capacity
    );
    live(&registry, &auth, &handle, "a", true);
    live(&registry, &auth, &handle, "b", true);
    assert!(counter.0.load(Ordering::SeqCst) > wake_count);
    let AsyncOutcome::Completed(Ok(result)) = ready(&mut future) else {
        panic!("read retry failed: {:?}", control.last_error())
    };
    assert_eq!(result.result.value, Some(b"old".to_vec()));
    assert_eq!(registry.diagnostics(&handle, &auth).unwrap().owners, owners);
    no_attempt_owners(&registry);
}

#[test]
fn typed_read_helpers_have_no_reservation_and_support_capacity_wait() {
    let (registry, auth, handle) = fixture();
    let owners = registry.diagnostics(&handle, &auth).unwrap().owners;
    let request = request(
        "reads",
        1,
        ClusterHelperCommand::QueryOutcomes {
            identities: vec![MutationIdentity::new("none", 1)],
            max_bytes: 4096,
        },
    );
    live(&registry, &auth, &handle, "a", false);
    live(&registry, &auth, &handle, "b", false);
    let (mut future, control) = registry
        .cluster_helper_invocation_with_retry(
            &handle,
            &auth,
            SurfaceId::Hc2Rust,
            request,
            RuntimeCallOptions::default(),
            ClusterRetryPolicy::new(2).unwrap(),
        )
        .unwrap();
    assert!(poll(&mut future).is_pending());
    assert!(poll(&mut future).is_pending());
    for _ in 0..8 {
        assert!(poll(&mut future).is_pending())
    }
    assert_eq!(control.attempts(), 2);
    live(&registry, &auth, &handle, "a", true);
    live(&registry, &auth, &handle, "b", true);
    assert!(
        matches!(ready(&mut future),AsyncOutcome::Completed(Ok(ClusterHelperExecution {result:ClusterHelperResult::Outcomes(values),..})) if values==vec![OperationOutcomeQuery::Unknown])
    );
    assert_eq!(registry.diagnostics(&handle, &auth).unwrap().owners, owners);
    no_attempt_owners(&registry);
}

#[test]
fn helper_mutation_retry_keeps_receipt_and_does_not_reapply_after_lost_response() {
    let (registry, auth, handle) = fixture();
    let request = replacement(&registry, &auth, &handle, 1);
    let mut options = RuntimeCallOptions::default();
    options.call.faults.response = LoopbackFault::Drop;
    let (mut future, control) = registry
        .cluster_helper_invocation_with_retry(
            &handle,
            &auth,
            SurfaceId::Resp,
            request.clone(),
            options,
            ClusterRetryPolicy::new(2).unwrap(),
        )
        .unwrap();
    assert!(poll(&mut future).is_pending());
    assert!(poll(&mut future).is_pending());
    assert!(poll(&mut future).is_pending());
    // The parked mutation retains partition FIFO. Observe its actual receipt through
    // the bounded safety lane rather than attempting a later ordinary data read.
    let query = ClusterHelperRequest::new(
        MutationIdentity::new("progress", 1),
        ClusterHelperCommand::QueryProgress {
            original_digest: request.request_digest(),
            original: Box::new(request.clone()),
            max_bytes: 4096,
        },
    );
    let progress = helper(
        &registry,
        &auth,
        &handle,
        query,
        RuntimeCallOptions::default(),
    );
    let ClusterHelperResult::Progress(progress) = progress.result else {
        panic!("expected actual root progress")
    };
    let OperationOutcomeQuery::Completed(before) = progress.root else {
        panic!("lost response must retain completed root")
    };
    let version = before.items[0].version;
    assert!(version.is_some());
    assert_eq!(control.attempts(), 2);
    registry.advance_clock(3).unwrap();
    assert!(matches!(ready(&mut future), AsyncOutcome::OutcomeUnknown));
    let recovered = helper(
        &registry,
        &auth,
        &handle,
        request,
        RuntimeCallOptions::default(),
    );
    assert!(recovered.replayed);
    assert!(
        matches!(recovered.result,ClusterHelperResult::Versions(result) if result.items[0].version==version)
    );
    let after = command(
        &registry,
        &auth,
        &handle,
        "get_entry_view",
        b"k",
        0,
        ServiceTtl::Preserve,
    )
    .result;
    assert_eq!(after.version, version);
    assert_eq!(after.value, Some(b"new".to_vec()));
    assert_eq!(after.remaining_ttl, Some(7));
    no_attempt_owners(&registry);
}

#[test]
fn helper_admin_futures_expire_and_retire_actual_state() {
    let (registry, auth, handle) = fixture();
    command(
        &registry,
        &auth,
        &handle,
        "put",
        b"k",
        1,
        ServiceTtl::ExpireAfter(1),
    );
    registry.advance_clock(2).unwrap();
    for request in [
        request(
            "expiry",
            1,
            ClusterHelperCommand::ExpireStep { max_work: 1 },
        ),
        request(
            "retire",
            1,
            ClusterHelperCommand::RetireOutcomes {
                client: "values".into(),
                through: 1,
            },
        ),
    ] {
        let (mut future, control) = registry
            .cluster_helper_invocation(
                &handle,
                &auth,
                SurfaceId::Hc2Rust,
                request,
                RuntimeCallOptions::default(),
            )
            .unwrap();
        assert!(
            matches!(ready(&mut future), AsyncOutcome::Completed(Ok(_))),
            "{:?}",
            control.last_error()
        );
        no_attempt_owners(&registry);
    }
    assert_eq!(
        command(
            &registry,
            &auth,
            &handle,
            "get",
            b"k",
            0,
            ServiceTtl::Preserve
        )
        .result
        .kind,
        "absent"
    );
}

#[test]
fn helper_cancellation_and_original_deadline_are_checked_before_dispatch() {
    for steps in 0..2 {
        let (registry, auth, handle) = fixture();
        let request = request(
            "reads",
            1,
            ClusterHelperCommand::QueryOutcomes {
                identities: vec![MutationIdentity::new("none", 1)],
                max_bytes: 4096,
            },
        );
        let (mut future, control) = registry
            .cluster_helper_invocation(
                &handle,
                &auth,
                SurfaceId::Resp,
                request,
                RuntimeCallOptions::default(),
            )
            .unwrap();
        for _ in 0..steps {
            assert!(poll(&mut future).is_pending())
        }
        assert_eq!(control.cancel(), CancellationDisposition::NotApplied);
        assert!(matches!(
            ready(&mut future),
            AsyncOutcome::NotApplied(InvocationRejection::Cancelled)
        ));
        no_attempt_owners(&registry);
    }
    let (registry, auth, handle) = fixture();
    let mut request = replacement(&registry, &auth, &handle, 1);
    request.deadline = 1;
    let (mut future, control) = registry
        .cluster_helper_invocation_with_retry(
            &handle,
            &auth,
            SurfaceId::Resp,
            request,
            RuntimeCallOptions::default(),
            ClusterRetryPolicy::new(4).unwrap(),
        )
        .unwrap();
    assert!(poll(&mut future).is_pending());
    registry.advance_clock(1).unwrap();
    assert!(matches!(
        ready(&mut future),
        AsyncOutcome::NotApplied(InvocationRejection::Deadline)
    ));
    assert_eq!(control.attempts(), 1);
    no_attempt_owners(&registry);
}

#[test]
fn read_retry_never_relaxes_an_explicit_epoch_and_cancellation_is_not_an_identity_claim() {
    let (registry, auth, handle) = fixture();
    let mut read = ServiceCommand::point("get", b"k");
    read.identity = Some(MutationIdentity::new("existing", 1));
    let mut options = RuntimeCallOptions::default();
    options.call.expected_epoch = Some(99);
    let (mut future, control) = registry
        .cluster_read_invocation_with_retry(
            &handle,
            &auth,
            SurfaceId::Hc2Rust,
            read,
            options,
            ClusterRetryPolicy::new(16).unwrap(),
        )
        .unwrap();
    assert!(matches!(
        ready(&mut future),
        AsyncOutcome::Completed(Err(RegistryError::Cluster(ServiceClusterError::StaleEpoch)))
    ));
    assert_eq!(control.attempts(), 1);
    let (mut cancelled, control) = registry
        .cluster_read_invocation(
            &handle,
            &auth,
            SurfaceId::Resp,
            ServiceCommand::point("get", b"k"),
            RuntimeCallOptions::default(),
        )
        .unwrap();
    control.cancel();
    assert!(matches!(
        ready(&mut cancelled),
        AsyncOutcome::NotApplied(InvocationRejection::Cancelled)
    ));
    no_attempt_owners(&registry);
}

#[test]
fn parked_repair_helper_allows_actual_transfer_work_and_cancel_releases_its_owners() {
    use hydracache_cluster_testkit::value_plane_cluster_health_075::MaintenanceResult;
    let (registry, auth, handle) = fixture();
    let mut options = RuntimeCallOptions::default();
    options.call.faults.coordinator.ack = LoopbackFault::Drop;
    let request = request(
        "repair-future",
        1,
        ClusterHelperCommand::ExpireStep { max_work: 1 },
    );
    let (mut future, control) = registry
        .cluster_helper_invocation_with_retry(
            &handle,
            &auth,
            SurfaceId::Hc2Rust,
            request,
            options,
            ClusterRetryPolicy::new(3).unwrap(),
        )
        .unwrap();
    assert!(poll(&mut future).is_pending());
    assert!(poll(&mut future).is_pending());
    assert_eq!(
        registry.cluster_admission_observations().unwrap().inflight[2],
        0,
        "parked helper cannot monopolize the sole repair permit"
    );
    let mut work = 0;
    for (partition, target) in [(0, "c"), (1, "a")] {
        registry
            .enqueue_maintenance(
                &handle,
                &auth,
                RuntimeMaintenance::RepairStep {
                    partition,
                    backup: target.into(),
                    faults: ServiceLinkFaults::default(),
                    maximum_work: 1,
                },
            )
            .unwrap();
        let record = registry
            .run_next()
            .unwrap()
            .expect("repair must acquire actual physical permit");
        let RegistryResult::Maintenance(MaintenanceResult::Online(step)) = record.result.unwrap()
        else {
            panic!("expected actual transfer step")
        };
        work += step.work;
    }
    assert!(
        work > 0,
        "at least the partition without pending root must advance its actual transfer"
    );
    assert!(poll(&mut future).is_pending());
    assert_eq!(control.attempts(), 3);
    assert_eq!(control.cancel(), CancellationDisposition::OutcomeUnknown);
    assert!(matches!(ready(&mut future), AsyncOutcome::OutcomeUnknown));
    no_attempt_owners(&registry);
    assert_eq!(
        registry.diagnostics(&handle, &auth).unwrap().owners.pending,
        0
    );
}

#[test]
fn entry_views_and_progress_futures_share_real_tokens_without_new_journals() {
    let (registry, auth, handle) = fixture();
    let original = replacement(&registry, &auth, &handle, 1);
    helper(
        &registry,
        &auth,
        &handle,
        original.clone(),
        RuntimeCallOptions::default(),
    );
    let owners = registry.diagnostics(&handle, &auth).unwrap().owners;
    let views = request(
        "views",
        1,
        ClusterHelperCommand::EntryViews {
            keys: vec![b"k".to_vec(), b"absent".to_vec(), b"k".to_vec()],
            max_bytes: 4096,
        },
    );
    let (mut future, _) = registry
        .cluster_helper_invocation(
            &handle,
            &auth,
            SurfaceId::Hc2Java,
            views,
            RuntimeCallOptions::default(),
        )
        .unwrap();
    assert!(poll(&mut future).is_pending());
    let AsyncOutcome::Completed(Ok(ClusterHelperExecution {
        result: ClusterHelperResult::EntryViews(views),
        ..
    })) = ready(&mut future)
    else {
        panic!("expected entry views")
    };
    assert_eq!(
        views.iter().map(|view| view.index).collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert_eq!(views[0].token, views[2].token);
    assert!(views[0].token.is_some());
    assert!(views[1].token.is_none());
    assert_eq!(views[0].value, Some(b"new".to_vec()));
    let query = request(
        "progress",
        1,
        ClusterHelperCommand::QueryProgress {
            original_digest: original.request_digest(),
            original: Box::new(original),
            max_bytes: 4096,
        },
    );
    let before = registry.diagnostics(&handle, &auth).unwrap();
    let (mut future, _) = registry
        .cluster_helper_invocation(
            &handle,
            &auth,
            SurfaceId::Resp,
            query,
            RuntimeCallOptions::default(),
        )
        .unwrap();
    let AsyncOutcome::Completed(Ok(ClusterHelperExecution {
        result: ClusterHelperResult::Progress(progress),
        ..
    })) = ready(&mut future)
    else {
        panic!("expected readonly progress")
    };
    assert!(matches!(progress.root, OperationOutcomeQuery::Completed(_)));
    assert!(progress
        .groups
        .iter()
        .all(|group| matches!(group.outcome, OperationOutcomeQuery::Completed(_))));
    let after = registry.diagnostics(&handle, &auth).unwrap();
    assert_eq!(after.statistics, before.statistics);
    assert_eq!(after.owners, owners);
    no_attempt_owners(&registry);
}

#[test]
fn pending_helper_progress_future_uses_bounded_safety_lane_without_mutating_the_root() {
    let (registry, auth, handle) = fixture();
    let original = replacement(&registry, &auth, &handle, 1);
    let (mut mutation, control) = registry
        .cluster_helper_invocation(
            &handle,
            &auth,
            SurfaceId::Resp,
            original.clone(),
            RuntimeCallOptions::default(),
        )
        .unwrap();
    assert!(poll(&mut mutation).is_pending());
    assert!(poll(&mut mutation).is_pending());
    let before = registry.diagnostics(&handle, &auth).unwrap();
    assert!(before.owners.pending > 0);
    let query = request(
        "progress",
        1,
        ClusterHelperCommand::QueryProgress {
            original_digest: original.request_digest(),
            original: Box::new(original.clone()),
            max_bytes: 4096,
        },
    );
    let (mut observation, _) = registry
        .cluster_helper_invocation(
            &handle,
            &auth,
            SurfaceId::Hc2Rust,
            query,
            RuntimeCallOptions::default(),
        )
        .unwrap();
    let AsyncOutcome::Completed(Ok(ClusterHelperExecution {
        result: ClusterHelperResult::Progress(progress),
        ..
    })) = ready(&mut observation)
    else {
        panic!("progress was blocked behind the actual pending helper")
    };
    assert_eq!(progress.root, OperationOutcomeQuery::Pending);
    assert!(progress
        .groups
        .iter()
        .all(|group| matches!(group.outcome, OperationOutcomeQuery::Unknown)));
    let after = registry.diagnostics(&handle, &auth).unwrap();
    assert_eq!(before.statistics, after.statistics);
    assert_eq!(before.owners, after.owners);
    let wrong = request(
        "bad-progress",
        1,
        ClusterHelperCommand::QueryProgress {
            original_digest: [255; 32],
            original: Box::new(original),
            max_bytes: 4096,
        },
    );
    assert!(registry
        .cluster_helper_invocation(
            &handle,
            &auth,
            SurfaceId::Resp,
            wrong,
            RuntimeCallOptions::default()
        )
        .is_err());
    control.cancel();
    assert!(matches!(ready(&mut mutation), AsyncOutcome::OutcomeUnknown));
    no_attempt_owners(&registry);
}

#[test]
fn helper_key_grants_and_policy_revocation_apply_before_future_dispatch() {
    let (registry, legacy, handle) = fixture();
    let original = replacement(&registry, &legacy, &handle, 1);
    registry.advance_policy_epoch(1).unwrap();
    let full = legacy
        .clone()
        .with_grant(
            NamespaceGrant::new("tenant", "map", 1, 1, GrantRights::ALL, GrantKeys::All).unwrap(),
        )
        .unwrap();
    let rights = GrantRights {
        read: true,
        write: false,
        listen: false,
        admin: false,
    };
    let read = full
        .attenuate(rights, GrantKeys::Prefixes(vec![b"k".to_vec()]))
        .unwrap();
    assert!(registry
        .cluster_helper_invocation(
            &handle,
            &read,
            SurfaceId::Resp,
            original,
            RuntimeCallOptions::default()
        )
        .is_err());
    let outside = request(
        "views",
        1,
        ClusterHelperCommand::EntryViews {
            keys: vec![b"k".to_vec(), b"outside".to_vec()],
            max_bytes: 4096,
        },
    );
    assert!(registry
        .cluster_helper_invocation(
            &handle,
            &read,
            SurfaceId::Resp,
            outside,
            RuntimeCallOptions::default()
        )
        .is_err());
    let views = request(
        "views",
        1,
        ClusterHelperCommand::EntryViews {
            keys: vec![b"k".to_vec()],
            max_bytes: 4096,
        },
    );
    let (mut future, _) = registry
        .cluster_helper_invocation_with_retry(
            &handle,
            &read,
            SurfaceId::Resp,
            views,
            RuntimeCallOptions::default(),
            ClusterRetryPolicy::new(4).unwrap(),
        )
        .unwrap();
    assert!(poll(&mut future).is_pending());
    registry
        .advance_namespace_policy_epoch(&handle, &full, 2)
        .unwrap();
    assert!(matches!(
        ready(&mut future),
        AsyncOutcome::Completed(Err(RegistryError::Security(
            SecurityError::PolicyEpochMismatch
        )))
    ));
    no_attempt_owners(&registry);
}

#[test]
fn helper_cancellation_recovers_only_actual_completed_receipts_and_fresh_drop_is_not_applied() {
    let (registry, auth, handle) = fixture();
    let original = replacement(&registry, &auth, &handle, 1);
    let first = helper(
        &registry,
        &auth,
        &handle,
        original.clone(),
        RuntimeCallOptions::default(),
    );
    let (mut completed, control) = registry
        .cluster_helper_invocation(
            &handle,
            &auth,
            SurfaceId::Resp,
            original,
            RuntimeCallOptions::default(),
        )
        .unwrap();
    assert_eq!(control.cancel(), CancellationDisposition::NotApplied);
    let AsyncOutcome::Completed(Ok(recovered)) = ready(&mut completed) else {
        panic!("existing actual receipt must win over caller cancellation")
    };
    assert!(recovered.replayed);
    assert_eq!(recovered.result, first.result);
    no_attempt_owners(&registry);
    let request = request(
        "fresh-drop",
        1,
        ClusterHelperCommand::ExpireStep { max_work: 1 },
    );
    let before = registry.diagnostics(&handle, &auth).unwrap().owners;
    let mut options = RuntimeCallOptions::default();
    options.call.faults.coordinator.request = LoopbackFault::Drop;
    let (mut dropped, _) = registry
        .cluster_helper_invocation(&handle, &auth, SurfaceId::Resp, request, options)
        .unwrap();
    assert!(matches!(
        ready(&mut dropped),
        AsyncOutcome::NotApplied(InvocationRejection::BackendDidNotAdmit)
    ));
    assert_eq!(registry.diagnostics(&handle, &auth).unwrap().owners, before);
    no_attempt_owners(&registry);
}

#[test]
fn forged_helper_reservation_never_retries_or_allocates_a_root() {
    let (registry, auth, handle) = fixture();
    let original = replacement(&registry, &auth, &handle, 1);
    let before = registry.diagnostics(&handle, &auth).unwrap().owners;
    let options = RuntimeCallOptions {
        auth_fault: Some(AuthFault {
            stage: ServiceMessageKind::Request,
            fault: EnvelopeFault::AdmissionDigest,
        }),
        ..RuntimeCallOptions::default()
    };
    let (mut forged, control) = registry
        .cluster_helper_invocation_with_retry(
            &handle,
            &auth,
            SurfaceId::Resp,
            original,
            options,
            ClusterRetryPolicy::new(16).unwrap(),
        )
        .unwrap();
    assert!(matches!(ready(&mut forged), AsyncOutcome::OutcomeUnknown));
    assert_eq!(control.attempts(), 1);
    assert!(matches!(
        control.last_error(),
        Some(RegistryError::Cluster(ServiceClusterError::Authorization(
            _
        )))
    ));
    assert_eq!(registry.diagnostics(&handle, &auth).unwrap().owners, before);
    no_attempt_owners(&registry);
}

#[test]
fn parked_large_helper_polls_preserve_actual_payload_credits_and_service_observations() {
    let (registry, auth, handle) = fixture_with_admission(ClusterAdmissionBounds {
        max_replica_bytes: 64 * 1024 * 1024,
        max_work_bytes: 32 * 1024 * 1024,
        max_tenant_bytes: 64 * 1024 * 1024,
        max_bytes: 64 * 1024 * 1024,
        ..Default::default()
    });
    let mut original = replacement(&registry, &auth, &handle, 1);
    let ClusterHelperCommand::VersionBulk { entries, .. } = &mut original.command else {
        unreachable!()
    };
    entries[0].value = Some(vec![42; 256 * 1024]);
    let mut options = RuntimeCallOptions::default();
    options.call.faults.response = LoopbackFault::Drop;
    let (mut future, control) = registry
        .cluster_helper_invocation_with_retry(
            &handle,
            &auth,
            SurfaceId::Hc2Rust,
            original,
            options,
            ClusterRetryPolicy::new(2).unwrap(),
        )
        .unwrap();
    assert!(poll(&mut future).is_pending());
    assert!(poll(&mut future).is_pending());
    assert!(poll(&mut future).is_pending());
    let before = registry.diagnostics(&handle, &auth).unwrap();
    let credit = registry.usage();
    let admission = registry.cluster_admission_observations().unwrap();
    assert!(credit.queued_bytes >= 256 * 1024);
    assert_eq!(admission.inflight[1], 0);
    for _ in 0..32 {
        assert!(poll(&mut future).is_pending());
    }
    let after = registry.diagnostics(&handle, &auth).unwrap();
    assert_eq!(before.statistics, after.statistics);
    assert_eq!(before.resources, after.resources);
    assert_eq!(before.owners, after.owners);
    assert_eq!(registry.usage().queued_bytes, credit.queued_bytes);
    assert_eq!(registry.usage().reserved_growth, credit.reserved_growth);
    assert_eq!(
        registry
            .cluster_admission_observations()
            .unwrap()
            .owned_bytes,
        admission.owned_bytes
    );
    assert_eq!(control.attempts(), 2);
    control.cancel();
    assert!(matches!(
        ready(&mut future),
        AsyncOutcome::Completed(Ok(ClusterHelperExecution {
            result: ClusterHelperResult::Versions(_),
            ..
        }))
    ));
    no_attempt_owners(&registry);
}
