use hydracache_cluster_testkit::value_plane_async_075::{
    AsyncOutcome, InvocationClock, InvocationRejection,
};
use hydracache_cluster_testkit::value_plane_collection_075::CollectionBounds;
use hydracache_cluster_testkit::value_plane_model_075::MutationIdentity;
use hydracache_cluster_testkit::value_plane_service_075::*;
use hydracache_cluster_testkit::value_plane_service_async_075::{
    LocalHelperAsyncFacade, LocalHelperCommand, LocalHelperResult,
};
use std::sync::Arc;

fn owner() -> UnifiedIMapService {
    UnifiedIMapService::new(CollectionBounds::default(), 7).unwrap()
}
fn put(owner: &UnifiedIMapService, key: &[u8], sequence: u64, ttl: ServiceTtl) {
    let mut request = ServiceCommand::point("put", key);
    request.value = Some(b"v".to_vec());
    request.identity = Some(MutationIdentity::new("writer", sequence));
    request.ttl = ttl;
    owner.execute(request).unwrap();
}

#[test]
fn batch_entry_views_preserve_order_absence_duplicates_ttl_and_full_tokens() {
    let owner = owner();
    put(&owner, b"a", 1, ServiceTtl::ExpireAfter(10));
    let keys = vec![b"a".to_vec(), b"miss".to_vec(), b"a".to_vec()];
    assert_eq!(
        owner.entry_views_response_bytes(&keys, 0, 248).unwrap(),
        248
    );
    let views = owner.entry_views(&keys, 0, 248).unwrap();
    assert_eq!(
        views.iter().map(|view| view.index).collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert_eq!(views[0].key, b"a");
    assert_eq!(views[0].value.as_deref(), Some(b"v".as_slice()));
    assert_eq!(views[0].remaining_ttl, Some(10));
    assert_eq!(views[0].token, views[2].token);
    assert_eq!(views[0].token.unwrap().namespace_generation, 7);
    assert!(
        views[1].value.is_none() && views[1].token.is_none() && views[1].remaining_ttl.is_none()
    );
    let token = views[0].token.unwrap();
    let mut replace = ServiceCommand::point("replace_if_version", b"a");
    replace.value = Some(b"next".to_vec());
    replace.token = Some(token);
    replace.identity = Some(MutationIdentity::new("cas", 1));
    assert_eq!(owner.execute(replace.clone()).unwrap().kind, "replaced");
    let mut old = replace;
    old.identity = Some(MutationIdentity::new("cas", 2));
    assert_eq!(owner.execute(old).unwrap().kind, "mismatch");
    let other = UnifiedIMapService::new(CollectionBounds::default(), 7).unwrap();
    put(&other, b"a", 1, ServiceTtl::Eternal);
    assert_ne!(
        other.entry_views(&[b"a".to_vec()], 0, 1_024).unwrap()[0]
            .token
            .unwrap()
            .map_id,
        token.map_id
    );
}

#[test]
fn batch_entry_view_bounds_reject_before_clock_expiry_events_or_receipts_change() {
    let owner = owner();
    put(&owner, b"a", 1, ServiceTtl::ExpireAfter(1));
    let listener = owner
        .register_listener(ServiceListenerOptions::default())
        .unwrap();
    let before = (
        owner.resource_usage(),
        owner.revision(),
        owner.listener_watermark(listener.id()).unwrap(),
    );
    assert_eq!(
        owner.entry_views(&[b"a".to_vec()], 1, 1).unwrap_err(),
        MapServiceError::ResourceBudget("entry_views_response")
    );
    assert_eq!(owner.logical_time(), 0);
    assert_eq!(
        (
            owner.resource_usage(),
            owner.revision(),
            owner.listener_watermark(listener.id()).unwrap()
        ),
        before
    );
    assert!(listener.drain().unwrap().is_empty());
    assert_eq!(
        owner.entry_views(&[], 0, 1_024).unwrap_err(),
        MapServiceError::InvalidArgument
    );
    assert_eq!(
        owner
            .entry_views(&vec![b"a".to_vec(); 257], 0, 1_024)
            .unwrap_err(),
        MapServiceError::InvalidArgument
    );
    assert_eq!(
        owner.entry_views(&[b"a".to_vec()], 0, 0).unwrap_err(),
        MapServiceError::InvalidArgument
    );
    let oversized = vec![0; CollectionBounds::default().max_key_bytes + 1];
    assert_eq!(
        owner
            .entry_views(&[oversized], 0, 1_024)
            .unwrap_err()
            .kind(),
        "key_too_large"
    );
    owner.entry_views(&[b"a".to_vec()], 1, 1_024).unwrap();
    assert_eq!(
        owner.entry_views(&[b"a".to_vec()], 0, 1_024).unwrap_err(),
        MapServiceError::ClockRegressed
    );
    let mut destroy = ServiceCommand::point("destroy", b"");
    destroy.now = 1;
    destroy.identity = Some(MutationIdentity::new("destroy", 1));
    owner.execute(destroy).unwrap();
    assert_eq!(
        owner
            .entry_views(&[b"a".to_vec()], 1, 1_024)
            .unwrap_err()
            .kind(),
        "map_destroyed"
    );
}

#[test]
fn batch_entry_views_keep_expired_data_invisible_with_bounded_maintenance() {
    let owner = UnifiedIMapService::with_limits(
        CollectionBounds::default(),
        1,
        ServiceLimits {
            expiry_work: 1,
            ..ServiceLimits::default()
        },
    )
    .unwrap();
    let keys = vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()];
    for (index, key) in keys.iter().enumerate() {
        put(&owner, key, index as u64 + 1, ServiceTtl::ExpireAfter(1));
    }
    let listener = owner
        .register_listener(ServiceListenerOptions::default())
        .unwrap();
    for remaining in [2, 1, 0] {
        let views = owner.entry_views(&keys, 1, 1_024).unwrap();
        assert!(views
            .iter()
            .all(|view| view.value.is_none() && view.token.is_none()));
        let events = listener.drain().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, "expired");
        assert_eq!(owner.namespace_owners().values, remaining);
    }
    owner.entry_views(&keys, 1, 1_024).unwrap();
    assert!(listener.drain().unwrap().is_empty());
}

#[tokio::test]
async fn async_entry_views_are_lazy_bounded_read_helpers_without_retained_receipts() {
    let owner = owner();
    put(&owner, b"a", 1, ServiceTtl::ExpireAfter(1));
    let clock = Arc::new(InvocationClock::default());
    clock.advance_to(1).unwrap();
    let facade = LocalHelperAsyncFacade::new(owner.clone(), clock, 2).unwrap();
    let command = || LocalHelperCommand::EntryViews {
        keys: vec![b"a".to_vec()],
        max_bytes: 1_024,
    };
    let identity = MutationIdentity::new("read", 1);
    let (handle, future) = facade.invoke(identity.clone(), 10, command()).unwrap();
    handle.cancel();
    assert!(matches!(
        future.await.outcome,
        AsyncOutcome::NotApplied(InvocationRejection::Cancelled)
    ));
    assert_eq!(owner.logical_time(), 0);
    assert_eq!(owner.resource_usage().streams, 0);
    let (_, future) = facade.invoke(identity.clone(), 10, command()).unwrap();
    assert!(
        matches!(future.await.outcome, AsyncOutcome::Completed(Ok(LocalHelperResult::EntryViews(views))) if views[0].token.is_none())
    );
    assert_eq!(
        owner.query_operation_outcome(&identity).unwrap(),
        OperationOutcomeQuery::Unknown
    );
    assert_eq!(facade.inflight(), 0);
    assert_eq!(owner.resource_usage().streams, 0);
}
