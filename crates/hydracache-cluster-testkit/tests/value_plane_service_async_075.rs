use std::sync::Arc;

use hydracache_cluster_testkit::value_plane_async_075::{
    AsyncIMapFacade, AsyncOutcome, InvocationClock, InvocationRejection,
};
use hydracache_cluster_testkit::value_plane_collection_075::CollectionBounds;
use hydracache_cluster_testkit::value_plane_model_075::MutationIdentity;
use hydracache_cluster_testkit::value_plane_service_075::{
    ServiceCommand, ServiceTtl, UnifiedIMapService,
};
use hydracache_cluster_testkit::value_plane_service_async_075::UnifiedServiceAsyncBackend;

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
