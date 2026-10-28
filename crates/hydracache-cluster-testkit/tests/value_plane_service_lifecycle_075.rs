use futures_util::Stream;
use hydracache_cluster_testkit::value_plane_collection_075::{
    CollectionBounds, ReferenceCollectionMap, ScanProjection, TimedMutationOptions,
};
use hydracache_cluster_testkit::value_plane_model_075::MutationIdentity;
use hydracache_cluster_testkit::value_plane_service_075::*;
use hydracache_cluster_testkit::value_plane_stream_075::DemandStream;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

fn id(sequence: u64) -> MutationIdentity {
    MutationIdentity::new("c", sequence)
}
fn service() -> UnifiedIMapService {
    UnifiedIMapService::new(CollectionBounds::default(), 1).unwrap()
}
fn request(op: &str, key: &[u8], sequence: u64) -> ServiceCommand {
    let mut command = ServiceCommand::point(op, key);
    command.identity = Some(id(sequence));
    command.value = Some(b"v".to_vec());
    command
}
fn put(service: &UnifiedIMapService, key: &[u8], sequence: u64) -> ServiceResult {
    service.execute(request("put", key, sequence)).unwrap()
}
fn token(service: &UnifiedIMapService, key: &[u8], now: u64) -> VersionToken {
    let mut command = ServiceCommand::point("get_entry_view", key);
    command.now = now;
    service.execute(command).unwrap().token.unwrap()
}

#[test]
fn retirement_fences_reclaim_completed_and_reject_old_retries() {
    let service = service();
    put(&service, b"a", 1);
    let pending = request("put", b"b", 2);
    service.reserve_operation(id(2), &pending).unwrap();
    assert_eq!(
        service.retire_outcomes("c", 1, 2).unwrap_err().kind(),
        "pending_retirement"
    );
    assert!(matches!(
        service.query_operation_outcome(&id(1)).unwrap(),
        OperationOutcomeQuery::Completed(_)
    ));
    assert_eq!(service.retire_outcomes("c", 1, 1).unwrap().removed, 1);
    assert_eq!(
        service.execute(request("put", b"a", 1)).unwrap_err().kind(),
        "retired_identity"
    );
    assert_eq!(
        service
            .reserve_operation(id(1), &request("put", b"a", 1))
            .unwrap_err()
            .kind(),
        "retired_identity"
    );
    assert_eq!(
        service
            .execute_reserved(request("put", b"a", 1))
            .unwrap_err()
            .kind(),
        "retired_identity"
    );
    assert_eq!(
        service
            .recover_command(&request("put", b"a", 1), &id(1))
            .unwrap_err()
            .kind(),
        "retired_identity"
    );
    assert_eq!(
        service.query_operation_outcome(&id(1)).unwrap(),
        OperationOutcomeQuery::Unknown
    );
    assert!(service.cancel_reserved(&id(2)).unwrap());
    assert_eq!(service.retire_outcomes("c", 1, 2).unwrap().removed, 0);
    assert_eq!(
        service.retire_outcomes("c", 2, 3).unwrap_err(),
        MapServiceError::InvalidArgument
    );
    assert_eq!(
        service.retire_outcomes("c", 1, 1).unwrap_err(),
        MapServiceError::InvalidArgument
    );
    assert_eq!(
        service.retire_outcomes("", 1, 1).unwrap_err(),
        MapServiceError::InvalidArgument
    );
    assert_eq!(
        service.retire_outcomes(&"é".repeat(65), 1, 1).unwrap_err(),
        MapServiceError::InvalidArgument
    );
    put(&service, b"a", 3);
    let limited = UnifiedIMapService::with_limits(
        CollectionBounds::default(),
        1,
        ServiceLimits {
            max_clients: 1,
            ..ServiceLimits::default()
        },
    )
    .unwrap();
    limited.retire_outcomes("c", 1, 1).unwrap();
    assert_eq!(
        limited.retire_outcomes("d", 1, 1).unwrap_err(),
        MapServiceError::ResourceBudget("retirement_clients")
    );
    assert_eq!(limited.resource_usage().outcomes, 25);
    limited.execute(request("destroy", b"", 2)).unwrap();
    assert_eq!(limited.resource_usage().outcomes, 0);
    assert_eq!(limited.resource_usage().values, 0);
}

#[test]
fn aggregate_payload_budgets_admit_before_mutation_and_release_owners() {
    let limits = ServiceLimits {
        value_bytes: 26,
        ..ServiceLimits::default()
    };
    let service = UnifiedIMapService::with_limits(CollectionBounds::default(), 1, limits).unwrap();
    put(&service, b"a", 1);
    let revision = service.revision();
    assert_eq!(
        service.execute(request("put", b"b", 2)).unwrap_err(),
        MapServiceError::ResourceBudget("values")
    );
    assert_eq!(service.revision(), revision);
    assert_eq!(service.statistics().live_entries, 1);
    assert_eq!(
        service.query_operation_outcome(&id(2)).unwrap(),
        OperationOutcomeQuery::Unknown
    );
    let outcomes = UnifiedIMapService::with_limits(
        CollectionBounds::default(),
        1,
        ServiceLimits {
            outcome_bytes: 200,
            ..ServiceLimits::default()
        },
    )
    .unwrap();
    put(&outcomes, b"a", 1);
    assert_eq!(
        outcomes.execute(request("put", b"b", 2)).unwrap_err(),
        MapServiceError::ResourceBudget("outcomes")
    );
    outcomes.retire_outcomes("c", 1, 1).unwrap();
    put(&outcomes, b"b", 2);
    let pending = UnifiedIMapService::with_limits(
        CollectionBounds::default(),
        1,
        ServiceLimits {
            pending_bytes: 49,
            ..ServiceLimits::default()
        },
    )
    .unwrap();
    pending
        .reserve_operation(id(1), &request("put", b"a", 1))
        .unwrap();
    assert_eq!(pending.resource_usage().pending, 49);
    assert_eq!(pending.resource_usage().outcomes, 113);
    assert_eq!(
        pending
            .reserve_operation(id(2), &request("put", b"b", 2))
            .unwrap_err(),
        MapServiceError::ResourceBudget("pending")
    );
    pending.cancel_reserved(&id(1)).unwrap();
    assert_eq!(pending.resource_usage().pending, 0);
    assert_eq!(pending.resource_usage().outcomes, 0);
    let listener = UnifiedIMapService::with_limits(
        CollectionBounds::default(),
        1,
        ServiceLimits {
            listener_bytes: 128,
            ..ServiceLimits::default()
        },
    )
    .unwrap();
    let registration = listener
        .register_listener(ServiceListenerOptions::default())
        .unwrap();
    assert_eq!(
        listener
            .register_listener(ServiceListenerOptions::default())
            .err(),
        Some(MapServiceError::ResourceBudget("listeners"))
    );
    put(&listener, b"a", 1);
    assert_eq!(registration.drain().unwrap()[0].kind, "gap");
    assert_eq!(listener.resource_usage().listeners, 128);
    drop(registration);
    assert_eq!(listener.resource_usage().listeners, 0);
    let bounds = CollectionBounds {
        max_response_bytes: 64,
        ..CollectionBounds::default()
    };
    let streamed = UnifiedIMapService::with_limits(
        bounds,
        1,
        ServiceLimits {
            stream_bytes: 128,
            ..ServiceLimits::default()
        },
    )
    .unwrap();
    let source = ServicePageSource::new(streamed.clone(), ScanProjection::Keys, 0).unwrap();
    assert_eq!(streamed.resource_usage().streams, 128);
    assert_eq!(
        ServicePageSource::new(streamed.clone(), ScanProjection::Keys, 0).err(),
        Some(MapServiceError::ResourceBudget("streams"))
    );
    drop(source);
    assert_eq!(streamed.resource_usage().streams, 0);
    assert!(UnifiedIMapService::with_limits(
        bounds,
        1,
        ServiceLimits {
            query_bytes: 0,
            ..ServiceLimits::default()
        }
    )
    .is_err());
    let mut legacy = ReferenceCollectionMap::new(bounds).unwrap();
    legacy.put(b"a".to_vec(), b"v".to_vec(), None, 0).unwrap();
    let adopted = UnifiedIMapService::from_map(legacy, 1).unwrap();
    assert_eq!(adopted.resource_usage().values, 26);
    let timed = UnifiedIMapService::with_limits(CollectionBounds::default(), 1, limits).unwrap();
    let registration = timed
        .register_listener(ServiceListenerOptions::default())
        .unwrap();
    let mut first = request("put", b"a", 1);
    first.ttl = ServiceTtl::ExpireAfter(1);
    timed.execute(first).unwrap();
    registration.drain().unwrap();
    let revision = timed.revision();
    let mut rejected = request("put", b"b", 2);
    rejected.now = 1;
    assert_eq!(
        timed.execute(rejected.clone()).unwrap_err(),
        MapServiceError::ResourceBudget("values")
    );
    assert_eq!(timed.logical_time(), 0);
    assert_eq!(timed.revision(), revision);
    assert_eq!(timed.statistics().live_entries, 1);
    assert_eq!(timed.resource_usage().values, 26);
    assert!(registration.drain().unwrap().is_empty());
    assert_eq!(
        timed.query_operation_outcome(&id(2)).unwrap(),
        OperationOutcomeQuery::Unknown
    );
    timed.expire_step(1, 1).unwrap();
    timed.execute(rejected).unwrap();
    let reserved = UnifiedIMapService::with_limits(CollectionBounds::default(), 1, limits).unwrap();
    let original = request("put", b"a", 1);
    reserved.reserve_operation(id(1), &original).unwrap();
    put(&reserved, b"b", 2);
    assert_eq!(
        reserved.execute_reserved(original.clone()).unwrap_err(),
        MapServiceError::ResourceBudget("values")
    );
    assert_eq!(
        reserved.query_operation_outcome(&id(1)).unwrap(),
        OperationOutcomeQuery::Pending
    );
    reserved.execute(request("delete", b"b", 3)).unwrap();
    assert_eq!(
        reserved.execute_reserved(original).unwrap().kind,
        "inserted"
    );
    assert_eq!(reserved.resource_usage().pending, 0);
    let mut invalid_ttl = request("put", b"a", 4);
    invalid_ttl.now = 1;
    invalid_ttl.ttl = ServiceTtl::ExpireAfter(u64::MAX);
    reserved.reserve_operation(id(4), &invalid_ttl).unwrap();
    assert_eq!(
        reserved.execute_reserved(invalid_ttl).unwrap_err(),
        MapServiceError::InvalidArgument
    );
    assert_eq!(
        reserved.query_operation_outcome(&id(4)).unwrap(),
        OperationOutcomeQuery::Pending
    );
    reserved.cancel_reserved(&id(4)).unwrap();
}

#[test]
fn listener_resync_is_atomic_bounded_and_privacy_preserving() {
    let service = UnifiedIMapService::with_limits(
        CollectionBounds::default(),
        1,
        ServiceLimits {
            resync_bytes: 64,
            ..ServiceLimits::default()
        },
    )
    .unwrap();
    let registration = service
        .register_listener(ServiceListenerOptions {
            queue_capacity: 1,
            key_prefix: b"a".to_vec(),
            ..ServiceListenerOptions::default()
        })
        .unwrap();
    put(&service, b"a", 1);
    put(&service, b"a", 2);
    put(&service, b"b", 3);
    assert!(matches!(
        service.resync_listener(registration.id(), 0, 1, 1),
        Err(MapServiceError::ResourceBudget("resync"))
    ));
    assert_eq!(registration.drain().unwrap()[0].kind, "gap");
    let mut snapshot = service
        .resync_listener(registration.id(), 0, 1, 64)
        .unwrap();
    assert_eq!(snapshot.entries.len(), 1);
    assert_eq!(snapshot.entries[0].key, b"a");
    assert!(snapshot.entries[0].value.is_empty());
    assert_eq!(service.resource_usage().resync, 33);
    assert!(matches!(
        service.resync_listener(registration.id(), 0, 1, 64),
        Err(MapServiceError::ResourceBudget("resync"))
    ));
    put(&service, b"a", 4);
    let events = registration.drain().unwrap();
    assert_eq!(events.len(), 1);
    assert!(events[0].watermark > snapshot.watermark);
    snapshot.close();
    snapshot.close();
    assert_eq!(service.resource_usage().resync, 0);
    let values = service
        .register_listener(ServiceListenerOptions {
            include_value: true,
            ..ServiceListenerOptions::default()
        })
        .unwrap();
    assert!(matches!(
        service.resync_listener(values.id(), 0, 1, 64),
        Err(MapServiceError::ResourceBudget("resync_items"))
    ));
    drop(registration);
    assert!(matches!(
        service.resync_listener(999, 0, 1, 64),
        Err(MapServiceError::UnknownListener)
    ));
}

#[test]
fn listener_resync_discards_prefetched_events_and_releases_stream_owners() {
    let service = service();
    let source = ServiceListenerSource::new(
        service.clone(),
        ServiceListenerOptions {
            include_value: true,
            ..ServiceListenerOptions::default()
        },
    )
    .unwrap();
    let listener_id = source.registration_id().unwrap();
    let mut stream = DemandStream::new(source, 2).unwrap();
    let mut context = Context::from_waker(Waker::noop());
    put(&service, b"a", 1);
    put(&service, b"b", 2);
    put(&service, b"c", 3);
    assert!(
        matches!(Pin::new(&mut stream).poll_next(&mut context), Poll::Ready(Some(Ok(event))) if event.key == b"a")
    );
    assert_eq!(stream.retained_items(), 1);
    assert_eq!(service.statistics().queued_events, 1);
    let snapshot = service.resync_listener(listener_id, 0, 4, 1_024).unwrap();
    assert_eq!(snapshot.entries.len(), 3);
    assert!(Pin::new(&mut stream).poll_next(&mut context).is_pending());
    assert_eq!(stream.retained_items(), 0);
    assert_eq!(service.statistics().queued_events, 0);
    put(&service, b"d", 4);
    assert!(
        matches!(Pin::new(&mut stream).poll_next(&mut context), Poll::Ready(Some(Ok(event))) if event.key == b"d" && event.watermark > snapshot.watermark)
    );
    put(&service, b"e", 5);
    put(&service, b"f", 6);
    assert!(
        matches!(Pin::new(&mut stream).poll_next(&mut context), Poll::Ready(Some(Ok(event))) if event.key == b"e")
    );
    assert_eq!(stream.retained_items(), 1);
    service.repair_listener(listener_id).unwrap();
    assert!(Pin::new(&mut stream).poll_next(&mut context).is_pending());
    assert_eq!(stream.retained_items(), 0);
    put(&service, b"g", 7);
    assert!(
        matches!(Pin::new(&mut stream).poll_next(&mut context), Poll::Ready(Some(Ok(event))) if event.key == b"g")
    );
    drop(snapshot);
    drop(stream);
    assert_eq!(service.resource_usage().resync, 0);
    assert_eq!(service.resource_usage().listeners, 0);
    assert_eq!(service.resource_usage().streams, 0);
    assert_eq!(service.statistics().active_streams, 0);
    let source =
        ServiceListenerSource::new(service.clone(), ServiceListenerOptions::default()).unwrap();
    let abandoned_id = source.registration_id().unwrap();
    let mut abandoned = DemandStream::new(source, 2).unwrap();
    put(&service, b"h", 8);
    put(&service, b"i", 9);
    assert!(matches!(
        Pin::new(&mut abandoned).poll_next(&mut context),
        Poll::Ready(Some(Ok(_)))
    ));
    assert_eq!(abandoned.retained_items(), 1);
    drop(abandoned);
    assert!(matches!(
        service.resync_listener(abandoned_id, 0, 16, 1_024),
        Err(MapServiceError::UnknownListener)
    ));
    assert_eq!(service.resource_usage().listeners, 0);
    assert_eq!(service.resource_usage().streams, 0);
    assert_eq!(service.resource_usage().resync, 0);
}

#[test]
fn bulk_version_conditionals_keep_order_receipts_and_retry_identity() {
    let service = service();
    put(&service, b"a", 1);
    put(&service, b"b", 2);
    let a = token(&service, b"a", 0);
    let b = token(&service, b"b", 0);
    let listener = service
        .register_listener(ServiceListenerOptions::default())
        .unwrap();
    let entries = vec![
        VersionBulkEntry {
            key: b"b".to_vec(),
            token: b,
            value: Some(b"B".to_vec()),
            ttl: ServiceTtl::ExpireAfter(10),
        },
        VersionBulkEntry {
            key: b"a".to_vec(),
            token: VersionToken { version: 0, ..a },
            value: Some(b"ignored".to_vec()),
            ttl: ServiceTtl::Eternal,
        },
        VersionBulkEntry {
            key: b"missing".to_vec(),
            token: a,
            value: Some(b"ignored".to_vec()),
            ttl: ServiceTtl::Preserve,
        },
    ];
    assert_eq!(
        service
            .bulk_version_conditional(VersionBulkOperation::Replace, &entries, id(3), 0, 1, true)
            .unwrap()
            .kind,
        "outcome_unknown"
    );
    let result = service
        .bulk_version_conditional(VersionBulkOperation::Replace, &entries, id(3), 99, 0, false)
        .unwrap();
    assert_eq!(
        result
            .items
            .iter()
            .map(|i| i.kind.as_str())
            .collect::<Vec<_>>(),
        ["replaced", "mismatch", "absent"]
    );
    assert_eq!(
        result.items.iter().map(|i| i.index).collect::<Vec<_>>(),
        [0, 1, 2]
    );
    assert_eq!(result.items[0].remaining_ttl, Some(10));
    assert_eq!(listener.drain().unwrap().len(), 1);
    let mut conflict = entries.clone();
    conflict[0].value = Some(b"different".to_vec());
    assert_eq!(
        service
            .bulk_version_conditional(VersionBulkOperation::Replace, &conflict, id(3), 0, 1, false)
            .unwrap_err()
            .kind(),
        "mutation_identity_conflict"
    );
    let ttl = [VersionBulkEntry {
        key: b"b".to_vec(),
        token: token(&service, b"b", 0),
        value: None,
        ttl: ServiceTtl::Eternal,
    }];
    assert_eq!(
        service
            .bulk_version_conditional(VersionBulkOperation::SetTtl, &ttl, id(4), 0, 1, false)
            .unwrap()
            .items[0]
            .kind,
        "replaced"
    );
    let removes = [
        VersionBulkEntry {
            key: b"b".to_vec(),
            token: token(&service, b"b", 0),
            value: None,
            ttl: ServiceTtl::Preserve,
        },
        VersionBulkEntry {
            key: b"a".to_vec(),
            token: a,
            value: None,
            ttl: ServiceTtl::Preserve,
        },
    ];
    let result = service
        .bulk_version_conditional(VersionBulkOperation::Remove, &removes, id(5), 0, 1, false)
        .unwrap();
    assert_eq!(result.items[0].kind, "removed");
    assert_eq!(result.items[1].kind, "removed");
    assert!(result.items[0].version < result.items[1].version);
    assert_eq!(service.statistics().live_entries, 0);
    assert_eq!(
        service
            .bulk_version_conditional(VersionBulkOperation::Replace, &[], id(6), 0, 1, false)
            .unwrap_err(),
        MapServiceError::InvalidArgument
    );
    let mut duplicate = vec![entries[0].clone(), entries[0].clone()];
    duplicate[0].ttl = ServiceTtl::Eternal;
    assert_eq!(
        service
            .bulk_version_conditional(
                VersionBulkOperation::Replace,
                &duplicate,
                id(6),
                0,
                1,
                false
            )
            .unwrap_err(),
        MapServiceError::InvalidArgument
    );
    assert_eq!(
        service
            .bulk_version_conditional(VersionBulkOperation::Replace, &entries, id(6), 0, 0, false)
            .unwrap()
            .kind,
        "not_applied"
    );
}

#[test]
fn batch_outcome_queries_preserve_pending_completed_and_unknown() {
    let service = service();
    put(&service, b"a", 1);
    service
        .reserve_operation(id(2), &request("put", b"b", 2))
        .unwrap();
    let before = service.statistics();
    let results = service
        .query_operation_outcomes(&[id(1), id(2), id(3), id(1)], 1_024)
        .unwrap();
    assert!(
        matches!(&results[0], OperationOutcomeQuery::Completed(result) if result.kind == "inserted")
    );
    assert_eq!(results[1], OperationOutcomeQuery::Pending);
    assert_eq!(results[2], OperationOutcomeQuery::Unknown);
    assert_eq!(results[0], results[3]);
    assert_eq!(service.statistics(), before);
    assert_eq!(
        service.query_operation_outcomes(&[id(1)], 1).unwrap_err(),
        MapServiceError::ResourceBudget("query_response")
    );
    assert!(service.query_operation_outcomes(&[], 1).is_err());
    assert!(service.query_operation_outcomes(&[id(0)], 1_024).is_err());
    service.cancel_reserved(&id(2)).unwrap();
    service.retire_outcomes("c", 1, 1).unwrap();
    assert_eq!(
        service
            .query_operation_outcomes(&[id(1), id(2)], 1_024)
            .unwrap(),
        [
            OperationOutcomeQuery::Unknown,
            OperationOutcomeQuery::Unknown
        ]
    );
}

#[test]
fn incremental_expiry_bounds_work_hides_expired_and_emits_once() {
    let service = UnifiedIMapService::with_limits(
        CollectionBounds::default(),
        1,
        ServiceLimits {
            expiry_work: 1,
            ..ServiceLimits::default()
        },
    )
    .unwrap();
    let listener = service
        .register_listener(ServiceListenerOptions {
            include_value: true,
            ..ServiceListenerOptions::default()
        })
        .unwrap();
    for (index, key) in [b"a", b"b", b"c", b"d"].iter().enumerate() {
        let mut put = request("put", *key, index as u64 + 1);
        put.ttl = ServiceTtl::ExpireAfter(5);
        service.execute(put).unwrap();
    }
    listener.drain().unwrap();
    let mut page = ServiceCommand::point("key_set_page", b"");
    page.limit = 1;
    let cursor = service.execute(page).unwrap().cursor.unwrap();
    let progress = service.expire_step(5, 1).unwrap();
    assert_eq!(progress.examined, 1);
    assert_eq!(progress.expired, 1);
    assert!(progress.pending);
    assert_eq!(service.statistics().live_entries, 0);
    assert_eq!(service.resource_usage().values, 3 * 26);
    assert!(service.diagnostic_snapshot(5).unwrap().is_empty());
    assert_eq!(service.resource_usage().values, 3 * 26);
    let mut page = ServiceCommand::point("key_set_page", b"");
    page.cursor = Some(cursor);
    page.now = 5;
    page.limit = 1;
    assert_eq!(
        service.execute(page).unwrap_err().kind(),
        "scan_cursor_stale"
    );
    let mut get = ServiceCommand::point("get", b"d");
    get.now = 5;
    assert_eq!(service.execute(get).unwrap().kind, "absent");
    let progress = service.expire_step(5, 1).unwrap();
    assert_eq!(progress.expired, 1);
    assert!(!progress.pending);
    let events = listener.drain().unwrap();
    assert_eq!(events.iter().filter(|e| e.kind == "expired").count(), 4);
    assert_eq!(service.expire_step(5, 1).unwrap().expired, 0);
    assert!(listener.drain().unwrap().is_empty());
    assert_eq!(
        service.expire_step(4, 1).unwrap_err(),
        MapServiceError::ClockRegressed
    );
    assert_eq!(
        service.expire_step(5, 0).unwrap_err(),
        MapServiceError::InvalidArgument
    );
    assert_eq!(service.resource_usage().values, 0);
    let targeted = UnifiedIMapService::with_limits(
        CollectionBounds::default(),
        1,
        ServiceLimits {
            expiry_work: 1,
            ..ServiceLimits::default()
        },
    )
    .unwrap();
    let registration = targeted
        .register_listener(ServiceListenerOptions::default())
        .unwrap();
    for (index, key) in [b"a", b"b", b"z"].iter().enumerate() {
        let mut command = request("put", *key, index as u64 + 1);
        command.ttl = ServiceTtl::ExpireAfter(1);
        targeted.execute(command).unwrap();
    }
    registration.drain().unwrap();
    let mut command = request("put", b"z", 4);
    command.now = 1;
    targeted.execute(command).unwrap();
    let events = registration.drain().unwrap();
    assert_eq!(
        events.iter().map(|e| e.kind.as_str()).collect::<Vec<_>>(),
        ["expired", "expired", "added"]
    );
    assert_eq!(targeted.expire_step(1, 1).unwrap().expired, 1);
}

#[test]
fn in_memory_checkpoint_restores_receipts_fences_ttl_and_fresh_identity() {
    let service = service();
    put(&service, b"a", 1);
    service.retire_outcomes("c", 1, 1).unwrap();
    let mut command = request("put", b"b", 2);
    command.ttl = ServiceTtl::ExpireAfter(10);
    service.execute(command.clone()).unwrap();
    let old_token = token(&service, b"b", 0);
    let mut page = ServiceCommand::point("key_set_page", b"");
    page.limit = 1;
    let old_cursor = service.execute(page).unwrap().cursor.unwrap();
    let checkpoint = service.checkpoint().unwrap();
    assert!(service.resource_usage().resync > 0);
    let restored = UnifiedIMapService::restore(checkpoint).unwrap();
    assert_eq!(service.resource_usage().resync, 0);
    assert_eq!(
        restored.diagnostic_snapshot(0).unwrap(),
        service.diagnostic_snapshot(0).unwrap()
    );
    assert_eq!(restored.execute(command).unwrap().kind, "inserted");
    assert_eq!(
        restored
            .execute(request("put", b"a", 1))
            .unwrap_err()
            .kind(),
        "retired_identity"
    );
    let mut stale = request("replace_if_version", b"b", 3);
    stale.token = Some(old_token);
    assert_eq!(restored.execute(stale).unwrap().kind, "mismatch");
    let mut stale_page = ServiceCommand::point("key_set_page", b"");
    stale_page.limit = 1;
    stale_page.cursor = Some(old_cursor);
    assert_eq!(
        restored.execute(stale_page).unwrap_err().kind(),
        "cursor_mismatch"
    );
    assert_ne!(token(&restored, b"b", 0).map_id, old_token.map_id);
    let mut original_change = request("put", b"a", 4);
    original_change.value = Some(b"changed-original".to_vec());
    service.execute(original_change).unwrap();
    let mut original_ttl = request("set_ttl", b"b", 5);
    original_ttl.ttl = ServiceTtl::Eternal;
    service.execute(original_ttl).unwrap();
    service.retire_outcomes("c", 1, 2).unwrap();
    let restored_entries = restored.diagnostic_snapshot(0).unwrap();
    assert_eq!(restored_entries[0].value, b"v");
    assert_eq!(restored_entries[1].remaining_ttl, Some(10));
    assert_eq!(
        service.query_operation_outcome(&id(2)).unwrap(),
        OperationOutcomeQuery::Unknown
    );
    assert!(matches!(
        restored.query_operation_outcome(&id(2)).unwrap(),
        OperationOutcomeQuery::Completed(_)
    ));
    let registration = restored
        .register_listener(ServiceListenerOptions::default())
        .unwrap();
    assert!(matches!(
        restored.checkpoint(),
        Err(MapServiceError::ActiveOwners)
    ));
    drop(registration);
    restored
        .reserve_operation(id(5), &request("put", b"c", 5))
        .unwrap();
    assert!(matches!(
        restored.checkpoint(),
        Err(MapServiceError::ActiveOwners)
    ));
    restored.cancel_reserved(&id(5)).unwrap();
    let source = ServicePageSource::new(restored.clone(), ScanProjection::Keys, 0).unwrap();
    assert!(matches!(
        restored.checkpoint(),
        Err(MapServiceError::ActiveOwners)
    ));
    drop(source);
    let mut checkpoint = restored.checkpoint().unwrap();
    checkpoint.close();
    checkpoint.close();
    assert_eq!(restored.resource_usage().resync, 0);
    assert!(UnifiedIMapService::restore(checkpoint).is_err());
    assert_eq!(restored.expire_step(10, 1).unwrap().expired, 1);
    // Old collection receipts are transferred and charged by the same owner as new ones.
    let mut legacy = ReferenceCollectionMap::new(CollectionBounds::default()).unwrap();
    legacy
        .try_put(
            id(1),
            b"legacy".to_vec(),
            b"v".to_vec(),
            None,
            TimedMutationOptions {
                now: 0,
                deadline: 1,
                cancelled: false,
                lose_response: true,
            },
        )
        .unwrap();
    let legacy = UnifiedIMapService::from_map(legacy, 1).unwrap();
    let legacy = UnifiedIMapService::restore(legacy.checkpoint().unwrap()).unwrap();
    assert!(matches!(
        legacy.query_operation_outcome(&id(1)).unwrap(),
        OperationOutcomeQuery::Completed(_)
    ));
}
