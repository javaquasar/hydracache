use std::sync::Arc;

use hydracache_cluster_testkit::value_plane_async_075::{
    AsyncIMapFacade, AsyncOutcome, InvocationClock, InvocationRejection,
};
use hydracache_cluster_testkit::value_plane_collection_075::CollectionBounds;
use hydracache_cluster_testkit::value_plane_model_075::MutationIdentity;
use hydracache_cluster_testkit::value_plane_service_075::{
    MapServiceError, OperationOutcomeQuery, ServiceLimits, ServiceListenerOptions,
    VersionBulkEntry, VersionBulkOperation,
};
use hydracache_cluster_testkit::value_plane_service_075::{
    ServiceCommand, ServiceTtl, UnifiedIMapService,
};
use hydracache_cluster_testkit::value_plane_service_async_075::UnifiedServiceAsyncBackend;
use hydracache_cluster_testkit::value_plane_service_async_075::{
    LocalHelperAsyncFacade, LocalHelperCommand, LocalHelperResult, LocalHelperStartError,
};

#[tokio::test]
async fn unified_async_dispatch_shares_values_events_versions_and_retained_outcomes() {
    let service = UnifiedIMapService::new(CollectionBounds::default(), 7).unwrap();
    let clock = Arc::new(InvocationClock::default());
    let facade = AsyncIMapFacade::new(
        Arc::new(UnifiedServiceAsyncBackend {
            service: service.clone(),
        }),
        Arc::clone(&clock),
        4,
    )
    .unwrap();
    let identity = MutationIdentity::new("unified-async", 1);
    let mut command = ServiceCommand::point("put", b"a");
    command.value = Some(b"v".to_vec());
    command.ttl = ServiceTtl::ExpireAfter(50);
    let (_, future) = facade
        .invoke(identity.clone(), 10, command.clone())
        .unwrap();
    let original = future.await;
    assert!(
        matches!(&original.outcome, AsyncOutcome::Completed(Ok(result)) if result.kind == "inserted")
    );
    assert_eq!(service.snapshot(0).unwrap()[0].value, b"v");
    let revision = service.revision();
    clock.advance_to(10).unwrap();
    let (cancel, future) = facade.invoke(identity.clone(), 1, command.clone()).unwrap();
    cancel.cancel();
    assert_eq!(future.await, original);
    assert_eq!(service.revision(), revision);
    assert_eq!(facade.inflight(), 0);

    let mut query = ServiceCommand::point("query_operation_outcome", b"");
    query.identity = Some(identity.clone());
    assert_eq!(service.execute(query).unwrap().kind, "completed");
    command.value = Some(b"conflict".to_vec());
    let (_, future) = facade.invoke(identity, 20, command).unwrap();
    assert!(
        matches!(future.await.outcome, AsyncOutcome::Completed(Err(error)) if error.kind() == "mutation_identity_conflict")
    );
}

#[tokio::test]
async fn unified_async_cancellation_and_public_pending_query_do_not_apply_values() {
    let service = UnifiedIMapService::new(CollectionBounds::default(), 1).unwrap();
    let facade = AsyncIMapFacade::new(
        Arc::new(UnifiedServiceAsyncBackend {
            service: service.clone(),
        }),
        Arc::new(InvocationClock::default()),
        2,
    )
    .unwrap();
    let identity = MutationIdentity::new("pending", 1);
    let mut command = ServiceCommand::point("set", b"a");
    command.value = Some(b"v".to_vec());
    let (handle, future) = facade
        .invoke(identity.clone(), 10, command.clone())
        .unwrap();
    handle.cancel();
    assert_eq!(
        future.await.outcome,
        AsyncOutcome::NotApplied(InvocationRejection::Cancelled)
    );
    assert!(service.snapshot(0).unwrap().is_empty());
    command.identity = Some(identity.clone());
    service
        .reserve_operation(identity.clone(), &command)
        .unwrap();
    let mut query = ServiceCommand::point("query_operation_outcome", b"");
    query.identity = Some(identity.clone());
    assert_eq!(service.execute(query.clone()).unwrap().kind, "pending");
    let (_, retry) = facade
        .invoke(identity.clone(), 10, command.clone())
        .unwrap();
    assert!(
        matches!(retry.await.outcome, AsyncOutcome::Completed(Ok(result)) if result.kind == "pending")
    );
    assert!(service.snapshot(0).unwrap().is_empty());
    assert_eq!(service.statistics().inflight, 1);
    assert_eq!(service.execute(query.clone()).unwrap().kind, "pending");
    let (handle, future) = facade.invoke(identity.clone(), 0, command.clone()).unwrap();
    handle.cancel();
    assert_eq!(future.await.outcome, AsyncOutcome::OutcomeUnknown);
    service.execute_reserved(command).unwrap();
    assert_eq!(service.execute(query).unwrap().kind, "completed");
    assert_eq!(service.statistics().inflight, 0);
}

#[tokio::test]
async fn typed_helpers_share_version_receipts_retirement_expiry_and_query_owners() {
    let service = UnifiedIMapService::new(CollectionBounds::default(), 7).unwrap();
    let clock = Arc::new(InvocationClock::default());
    let facade = LocalHelperAsyncFacade::new(service.clone(), clock.clone(), 4).unwrap();
    let mut put = ServiceCommand::point("put", b"a");
    put.value = Some(b"before".to_vec());
    put.ttl = ServiceTtl::ExpireAfter(5);
    put.identity = Some(MutationIdentity::new("values", 1));
    service.execute(put).unwrap();
    let token = service
        .execute(ServiceCommand::point("get_entry_view", b"a"))
        .unwrap()
        .token
        .unwrap();
    let entry = VersionBulkEntry {
        key: b"a".to_vec(),
        token,
        value: Some(b"after".to_vec()),
        ttl: ServiceTtl::Preserve,
    };
    let id = MutationIdentity::new("versions", 1);
    let (_, future) = facade
        .invoke(
            id.clone(),
            10,
            LocalHelperCommand::VersionBulk {
                operation: VersionBulkOperation::Replace,
                entries: vec![entry.clone()],
            },
        )
        .unwrap();
    let result = match future.await.outcome {
        AsyncOutcome::Completed(Ok(LocalHelperResult::Versions(result))) => result,
        other => panic!("{other:?}"),
    };
    assert_eq!(
        service
            .bulk_version_conditional(
                VersionBulkOperation::Replace,
                std::slice::from_ref(&entry),
                id.clone(),
                0,
                10,
                false
            )
            .unwrap(),
        result
    );
    let (_, query) = facade
        .invoke(
            MutationIdentity::new("query", 1),
            10,
            LocalHelperCommand::QueryOutcomes {
                identities: vec![id],
                max_bytes: 1_024,
            },
        )
        .unwrap();
    assert!(
        matches!(query.await.outcome, AsyncOutcome::Completed(Ok(LocalHelperResult::Outcomes(items))) if items == vec![OperationOutcomeQuery::Completed(Box::new(result))])
    );
    let (_, retire) = facade
        .invoke(
            MutationIdentity::new("helper-retire", 1),
            10,
            LocalHelperCommand::RetireOutcomes {
                client: "values".into(),
                generation: 7,
                through: 1,
            },
        )
        .unwrap();
    assert!(
        matches!(retire.await.outcome, AsyncOutcome::Completed(Ok(LocalHelperResult::Retirement(receipt))) if receipt.removed == 1)
    );
    assert!(
        matches!(service.query_operation_outcome(&MutationIdentity::new("helper-retire", 1)).unwrap(), OperationOutcomeQuery::Completed(result) if result.kind == "helper_retired")
    );
    clock.advance_to(5).unwrap();
    let expiry_id = MutationIdentity::new("helper-expiry", 1);
    let (_, expiry) = facade
        .invoke(
            expiry_id.clone(),
            10,
            LocalHelperCommand::ExpireStep { max_work: 1 },
        )
        .unwrap();
    assert!(
        matches!(expiry.await.outcome, AsyncOutcome::Completed(Ok(LocalHelperResult::Expiry(progress))) if progress.expired == 1)
    );
    let (cancel, replay) = facade
        .invoke(expiry_id, 0, LocalHelperCommand::ExpireStep { max_work: 1 })
        .unwrap();
    cancel.cancel();
    assert!(
        matches!(replay.await.outcome, AsyncOutcome::Completed(Ok(LocalHelperResult::Expiry(progress))) if progress.expired == 1)
    );
    assert!(service.diagnostic_snapshot(5).unwrap().is_empty());
    assert_eq!(service.resource_usage().streams, 0);
    assert_eq!(facade.inflight(), 0);
}

#[tokio::test]
async fn typed_snapshot_recovery_keeps_original_cut_privacy_and_later_queue() {
    let service = UnifiedIMapService::new(CollectionBounds::default(), 3).unwrap();
    let listener = service
        .register_listener(ServiceListenerOptions {
            include_value: true,
            ..ServiceListenerOptions::default()
        })
        .unwrap();
    let mut put = ServiceCommand::point("put", b"a");
    put.value = Some(b"old".to_vec());
    put.identity = Some(MutationIdentity::new("writer", 1));
    service.execute(put.clone()).unwrap();
    let facade =
        LocalHelperAsyncFacade::new(service.clone(), Arc::new(InvocationClock::default()), 2)
            .unwrap();
    let id = MutationIdentity::new("snapshot", 1);
    let command = || LocalHelperCommand::ListenerSnapshot {
        listener_id: listener.id(),
        max_items: 8,
        max_bytes: 1_024,
    };
    let (_, future) = facade.invoke(id.clone(), 10, command()).unwrap();
    let first = match future.await.outcome {
        AsyncOutcome::Completed(Ok(LocalHelperResult::Snapshot(snapshot))) => snapshot,
        other => panic!("{other:?}"),
    };
    let original = (
        first.entries.clone(),
        first.watermark,
        first.revision,
        first.include_value,
    );
    assert!(original.3);
    drop(first);
    put.identity = Some(MutationIdentity::new("writer", 2));
    put.value = Some(b"new".to_vec());
    service.execute(put).unwrap();
    let (cancel, retry) = facade.invoke(id, 0, command()).unwrap();
    cancel.cancel();
    let recovered = match retry.await.outcome {
        AsyncOutcome::Completed(Ok(LocalHelperResult::Snapshot(snapshot))) => snapshot,
        other => panic!("{other:?}"),
    };
    assert_eq!(
        (
            recovered.entries.clone(),
            recovered.watermark,
            recovered.revision,
            recovered.include_value
        ),
        original
    );
    assert_eq!(listener.drain().unwrap().len(), 1);
    assert!(service.resource_usage().resync > 0);
    drop(recovered);
    assert_eq!(service.resource_usage().resync, 0);
    let private = service
        .register_listener(ServiceListenerOptions::default())
        .unwrap();
    let (_, private_future) = facade
        .invoke(
            MutationIdentity::new("private", 1),
            10,
            LocalHelperCommand::ListenerSnapshot {
                listener_id: private.id(),
                max_items: 8,
                max_bytes: 1_024,
            },
        )
        .unwrap();
    assert!(
        matches!(private_future.await.outcome, AsyncOutcome::Completed(Ok(LocalHelperResult::Snapshot(snapshot))) if !snapshot.include_value && snapshot.entries[0].value.is_empty())
    );
}

#[tokio::test]
async fn typed_helpers_are_lazy_byte_bounded_and_cancelled_leases_are_released() {
    let service = UnifiedIMapService::new(CollectionBounds::default(), 1).unwrap();
    let facade =
        LocalHelperAsyncFacade::new(service.clone(), Arc::new(InvocationClock::default()), 1)
            .unwrap();
    let baseline = service.resource_usage();
    let (cancel, future) = facade
        .invoke(
            MutationIdentity::new("lazy", 1),
            10,
            LocalHelperCommand::Checkpoint,
        )
        .unwrap();
    assert!(service.resource_usage().streams > baseline.streams);
    cancel.cancel();
    assert!(matches!(
        future.await.outcome,
        AsyncOutcome::NotApplied(InvocationRejection::Cancelled)
    ));
    assert_eq!(service.resource_usage(), baseline);
    let (_, checkpoint) = facade
        .invoke(
            MutationIdentity::new("checkpoint", 1),
            10,
            LocalHelperCommand::Checkpoint,
        )
        .unwrap();
    let checkpoint = match checkpoint.await.outcome {
        AsyncOutcome::Completed(Ok(LocalHelperResult::Checkpoint(checkpoint))) => checkpoint,
        other => panic!("{other:?}"),
    };
    assert!(service.resource_usage().resync > 0);
    drop(checkpoint);
    assert_eq!(service.resource_usage(), baseline);
    let (_, dropped) = facade
        .invoke(
            MutationIdentity::new("drop", 1),
            10,
            LocalHelperCommand::ExpireStep { max_work: 1 },
        )
        .unwrap();
    drop(dropped);
    assert_eq!(facade.inflight(), 0);
    assert_eq!(service.resource_usage(), baseline);
    let small = UnifiedIMapService::with_limits(
        CollectionBounds::default(),
        1,
        ServiceLimits {
            stream_bytes: 367,
            ..ServiceLimits::default()
        },
    )
    .unwrap();
    let bounded =
        LocalHelperAsyncFacade::new(small.clone(), Arc::new(InvocationClock::default()), 1)
            .unwrap();
    assert!(matches!(
        bounded.invoke(
            MutationIdentity::new("reject", 1),
            10,
            LocalHelperCommand::ExpireStep { max_work: 1 }
        ),
        Err(LocalHelperStartError::Domain(
            MapServiceError::ResourceBudget("streams")
        ))
    ));
    assert_eq!(small.statistics().retained_outcomes, 0);
    assert_eq!(small.resource_usage().streams, 0);
    let (_, own_retirement) = facade
        .invoke(
            MutationIdentity::new("same", 1),
            10,
            LocalHelperCommand::RetireOutcomes {
                client: "same".into(),
                generation: 1,
                through: 1,
            },
        )
        .unwrap();
    assert!(matches!(
        own_retirement.await.outcome,
        AsyncOutcome::Completed(Err(MapServiceError::PendingOperation))
    ));
    assert_eq!(
        service
            .query_operation_outcome(&MutationIdentity::new("same", 1))
            .unwrap(),
        OperationOutcomeQuery::Unknown
    );
}

#[tokio::test]
async fn typed_snapshot_cancelled_after_cut_retains_receipt_and_closes_undelivered_lease() {
    use hydracache_cluster_testkit::value_plane_async_075::InvocationHandle;
    use std::sync::Mutex;
    use std::task::{Context, Wake, Waker};
    struct CancelOnWake(Mutex<Option<InvocationHandle>>);
    impl Wake for CancelOnWake {
        fn wake(self: Arc<Self>) {
            if let Some(handle) = self.0.lock().unwrap().as_ref() {
                handle.cancel();
            }
        }
    }
    let service = UnifiedIMapService::new(CollectionBounds::default(), 1).unwrap();
    let mut put = ServiceCommand::point("put", b"expires");
    put.value = Some(b"v".to_vec());
    put.identity = Some(MutationIdentity::new("writer", 1));
    put.ttl = ServiceTtl::ExpireAfter(1);
    service.execute(put).unwrap();
    let listener = service
        .register_listener(ServiceListenerOptions::default())
        .unwrap();
    let clock = Arc::new(InvocationClock::default());
    clock.advance_to(1).unwrap();
    let facade = LocalHelperAsyncFacade::new(service.clone(), clock, 2).unwrap();
    let id = MutationIdentity::new("cut", 1);
    let command = || LocalHelperCommand::ListenerSnapshot {
        listener_id: listener.id(),
        max_items: 8,
        max_bytes: 1_024,
    };
    let (handle, future) = facade.invoke(id.clone(), 10, command()).unwrap();
    let notice = Arc::new(CancelOnWake(Mutex::new(Some(handle))));
    let waker = Waker::from(notice);
    let mut context = Context::from_waker(&waker);
    assert!(service
        .poll_listener(listener.id(), 1, &mut context)
        .is_pending());
    assert!(matches!(future.await.outcome, AsyncOutcome::OutcomeUnknown));
    assert_eq!(service.resource_usage().resync, 0);
    assert_eq!(service.resource_usage().streams, 0);
    let (_, retry) = facade.invoke(id, 0, command()).unwrap();
    assert!(
        matches!(retry.await.outcome, AsyncOutcome::Completed(Ok(LocalHelperResult::Snapshot(snapshot))) if snapshot.entries.is_empty() && !snapshot.include_value && snapshot.watermark > 0)
    );
    assert_eq!(service.resource_usage().resync, 0);
}
