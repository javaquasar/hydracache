use std::collections::BTreeSet;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use futures_util::{Stream, StreamExt};

use hydracache_cluster_testkit::value_plane_collection_075::CollectionError;
use hydracache_cluster_testkit::value_plane_collection_075::ScanProjection;
use hydracache_cluster_testkit::value_plane_collection_075::{
    CollectionBounds, ReferenceCollectionMap, ReplacementEntry, TimedMutationOptions,
    TimedMutationOutcome,
};
use hydracache_cluster_testkit::value_plane_model_075::MutationIdentity;
use hydracache_cluster_testkit::value_plane_service_075::*;
use hydracache_cluster_testkit::value_plane_stream_075::{DemandStream, DemandStreamError};

fn service() -> UnifiedIMapService {
    UnifiedIMapService::new(CollectionBounds::default(), 1).unwrap()
}
fn id(sequence: u64) -> MutationIdentity {
    MutationIdentity::new("service-client", sequence)
}
fn command(op: &str, sequence: u64) -> ServiceCommand {
    let mut command = ServiceCommand::point(op, b"a");
    command.identity = Some(id(sequence));
    command.value = Some(b"v".to_vec());
    command.expected = Some(b"v".to_vec());
    command
}
fn put(service: &UnifiedIMapService, sequence: u64, value: &[u8]) -> ServiceResult {
    let mut request = command("put", sequence);
    request.value = Some(value.to_vec());
    service.execute(request).unwrap()
}
fn view(service: &UnifiedIMapService) -> ServiceResult {
    service
        .execute(ServiceCommand::point("get_entry_view", b"a"))
        .unwrap()
}

#[test]
fn existing_collection_values_and_retained_receipts_share_the_service_owner() {
    let mut collection = ReferenceCollectionMap::new(CollectionBounds::default()).unwrap();
    collection
        .put(b"preloaded".to_vec(), b"seed".to_vec(), None, 0)
        .unwrap();
    let lost = collection
        .try_put(
            id(1),
            b"a".to_vec(),
            b"old".to_vec(),
            Some(10),
            TimedMutationOptions {
                now: 0,
                deadline: 10,
                cancelled: false,
                lose_response: true,
            },
        )
        .unwrap();
    assert_eq!(lost, TimedMutationOutcome::OutcomeUnknown);
    let point_version = collection.get_entry_view(b"a", 0).unwrap().unwrap().version;
    let replacements = [
        ReplacementEntry {
            key: b"a".to_vec(),
            value: b"new".to_vec(),
        },
        ReplacementEntry {
            key: b"missing".to_vec(),
            value: b"ignored".to_vec(),
        },
    ];
    let original_receipts = collection
        .replace_all_detailed_idempotent(id(2), &replacements, 0)
        .unwrap();
    let service = UnifiedIMapService::from_map(collection, 1).unwrap();
    assert_eq!(view(&service).value, Some(b"new".to_vec()));
    assert_eq!(service.statistics().live_entries, 2);
    assert_eq!(service.statistics().retained_outcomes, 2);
    let point = match service.query_operation_outcome(&id(1)).unwrap() {
        OperationOutcomeQuery::Completed(result) => result,
        _ => panic!("legacy point outcome must remain retained"),
    };
    assert_eq!(point.kind, "applied");
    assert!(point.flag);
    assert_eq!(point.version, Some(point_version));
    let bulk = match service.query_operation_outcome(&id(2)).unwrap() {
        OperationOutcomeQuery::Completed(result) => result,
        _ => panic!("legacy bulk receipts must remain retained"),
    };
    assert_eq!(bulk.kind, "bulk");
    assert_eq!(bulk.items.len(), original_receipts.len());
    for (item, receipt) in bulk.items.iter().zip(&original_receipts) {
        assert_eq!(item.index, receipt.input_index);
        assert_eq!(item.key, receipt.key);
        assert_eq!(
            item.kind,
            if receipt.replaced {
                "replaced"
            } else {
                "absent"
            }
        );
        assert_eq!(item.version, receipt.version);
    }
    assert_eq!(
        service.execute(command("put", 1)).unwrap_err().kind(),
        "mutation_identity_conflict"
    );
    put(&service, 3, b"shared");
    assert_eq!(service.clone().statistics().retained_outcomes, 3);
    assert_eq!(view(&service).value, Some(b"shared".to_vec()));
    assert!(UnifiedIMapService::from_map(
        ReferenceCollectionMap::new(CollectionBounds::default()).unwrap(),
        0
    )
    .is_err());
}

#[test]
fn unified_dispatch_covers_all_contract_operations() {
    let contract: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/testing/imap/0.75/operation-contract.json"
    ))
    .unwrap();
    let mut observed = BTreeSet::new();
    for operation in contract["operations"].as_array().unwrap() {
        let op = operation["id"].as_str().unwrap();
        let service = service();
        put(&service, 1, b"v");
        let mut request = command(op, 2);
        request.keys = vec![b"a".to_vec(), b"b".to_vec()];
        request.entries = vec![
            ReplacementEntry {
                key: b"a".to_vec(),
                value: b"new".to_vec(),
            },
            ReplacementEntry {
                key: b"b".to_vec(),
                value: b"other".to_vec(),
            },
        ];
        request.token = view(&service).token;
        request.limit = 2;
        request.listener_id = 7;
        let result = service
            .execute(request)
            .unwrap_or_else(|error| panic!("{op}: {error}"));
        let outcomes = operation["outcomes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|outcome| outcome.as_str().unwrap())
            .collect::<BTreeSet<_>>();
        if matches!(
            op,
            "get_all" | "put_all_detailed" | "remove_all_detailed" | "replace_all_detailed"
        ) {
            assert_eq!(result.kind, "bulk", "{op}: wrong aggregate projection");
            assert_eq!(result.items.len(), 2, "{op}: incomplete item coverage");
            for item in &result.items {
                assert!(
                    outcomes.contains(item.kind.as_str()),
                    "{op}: item outcome {} is absent from {outcomes:?}",
                    item.kind
                );
            }
        } else {
            assert!(
                outcomes.contains(result.kind.as_str()),
                "{op}: outcome {} is absent from {outcomes:?}",
                result.kind
            );
        }
        observed.insert(op);
    }
    assert_eq!(
        observed.len(),
        contract["operations"].as_array().unwrap().len()
    );
}

#[test]
fn version_conditionals_reject_aba_and_foreign_tokens() {
    let service = service();
    put(&service, 1, b"v");
    let first = view(&service).token.unwrap();
    put(&service, 2, b"different");
    put(&service, 3, b"v");
    let mut stale = command("replace_if_version", 4);
    stale.token = Some(first);
    stale.value = Some(b"bad".to_vec());
    assert_eq!(service.execute(stale.clone()).unwrap().kind, "mismatch");
    assert_eq!(view(&service).value, Some(b"v".to_vec()));
    let foreign = UnifiedIMapService::new(CollectionBounds::default(), 1).unwrap();
    put(&foreign, 1, b"v");
    stale.identity = Some(id(5));
    stale.token = view(&foreign).token;
    assert_eq!(service.execute(stale.clone()).unwrap().kind, "mismatch");
    stale.identity = Some(id(6));
    stale.token = Some(VersionToken {
        namespace_generation: 2,
        ..view(&service).token.unwrap()
    });
    assert_eq!(service.execute(stale).unwrap().kind, "mismatch");
    let mut replace = command("replace_if_version", 7);
    replace.token = view(&service).token;
    replace.value = Some(b"next".to_vec());
    assert_eq!(service.execute(replace).unwrap().kind, "replaced");
    let mut ttl = command("set_ttl_if_version", 8);
    ttl.token = view(&service).token;
    ttl.ttl = ServiceTtl::ExpireAfter(5);
    assert_eq!(service.execute(ttl).unwrap().remaining_ttl, Some(5));
    let mut remove = command("remove_if_version", 9);
    remove.token = view(&service).token;
    assert_eq!(service.execute(remove).unwrap().kind, "removed");
    assert_eq!(view(&service).kind, "absent");
    let mut absent = command("remove_if_version", 10);
    absent.token = Some(first);
    assert_eq!(service.execute(absent).unwrap().kind, "absent");
}

#[test]
fn public_outcome_query_never_reexecutes_or_invents_absence() {
    let service = service();
    assert_eq!(
        service.query_operation_outcome(&id(1)).unwrap(),
        OperationOutcomeQuery::Unknown
    );
    assert_eq!(
        service
            .reserve_operation(id(99), &ServiceCommand::point("get", b"a"))
            .unwrap_err()
            .kind(),
        "invalid_argument"
    );
    let mut request = command("put", 1);
    request.lose_response = true;
    assert_eq!(
        service.execute(request.clone()).unwrap().kind,
        "outcome_unknown"
    );
    let revision = service.revision();
    let original = match service.query_operation_outcome(&id(1)).unwrap() {
        OperationOutcomeQuery::Completed(result) => *result,
        _ => panic!("missing retained outcome"),
    };
    assert_eq!(original.kind, "inserted");
    request.lose_response = false;
    request.now = 10;
    request.deadline = 0;
    assert_eq!(service.execute(request.clone()).unwrap(), original);
    assert_eq!(service.revision(), revision);
    request.value = Some(b"conflict".to_vec());
    assert_eq!(
        service.execute(request).unwrap_err().kind(),
        "mutation_identity_conflict"
    );
    let mut pending = command("put", 2);
    pending.value = Some(b"later".to_vec());
    service.reserve_operation(id(2), &pending).unwrap();
    assert_eq!(service.statistics().inflight, 1);
    assert_eq!(
        service.query_operation_outcome(&id(2)).unwrap(),
        OperationOutcomeQuery::Pending
    );
    assert_eq!(service.execute(pending.clone()).unwrap().kind, "pending");
    assert_eq!(service.execute_reserved(pending).unwrap().kind, "replaced");
    assert_eq!(service.statistics().inflight, 0);
    let pending = command("put", 3);
    service.reserve_operation(id(3), &pending).unwrap();
    assert!(service.cancel_reserved(&id(3)).unwrap());
    assert!(!service.cancel_reserved(&id(3)).unwrap());
    assert_eq!(
        service.query_operation_outcome(&id(3)).unwrap(),
        OperationOutcomeQuery::Unknown
    );
    assert_eq!(service.statistics().cancellations, 1);
    let mut cancelled = command("put", 4);
    cancelled.cancelled = true;
    assert_eq!(service.execute(cancelled).unwrap().kind, "not_applied");
    assert_eq!(service.statistics().cancellations, 2);
    let mut timed = command("put", 5);
    timed.deadline = 0;
    assert_eq!(service.execute(timed).unwrap().kind, "not_applied");
}

#[test]
fn listener_values_lifecycle_and_drop_are_bounded() {
    let service = service();
    let metadata = service
        .register_listener(ServiceListenerOptions::default())
        .unwrap();
    let values = service
        .register_listener(ServiceListenerOptions {
            include_value: true,
            queue_capacity: 16,
            ..ServiceListenerOptions::default()
        })
        .unwrap();
    put(&service, 1, b"one");
    put(&service, 2, b"two");
    let metadata_events = metadata.drain().unwrap();
    assert_eq!(metadata_events.len(), 2);
    assert!(metadata_events
        .iter()
        .all(|event| event.value.is_none() && event.old_value.is_none()));
    let value_events = values.drain().unwrap();
    assert_eq!(value_events[0].value, Some(b"one".to_vec()));
    assert_eq!(value_events[1].old_value, Some(b"one".to_vec()));
    assert_eq!(value_events[1].value, Some(b"two".to_vec()));
    let limited = service
        .register_listener(ServiceListenerOptions {
            include_value: true,
            max_value_bytes: 1,
            queue_capacity: 1,
            ..ServiceListenerOptions::default()
        })
        .unwrap();
    put(&service, 3, b"large");
    put(&service, 4, b"larger");
    let gap = limited.drain().unwrap();
    assert_eq!(gap.len(), 1);
    assert_eq!(gap[0].kind, "gap");
    assert!(gap[0].value.is_none());
    assert!(limited.drain().unwrap().is_empty());
    service.repair_listener(limited.id()).unwrap();
    put(&service, 5, b"x");
    assert_eq!(limited.drain().unwrap()[0].kind, "gap");
    let mut expiring = command("put", 6);
    expiring.ttl = ServiceTtl::ExpireAfter(1);
    service.execute(expiring).unwrap();
    service.snapshot(1).unwrap();
    assert!(values
        .drain()
        .unwrap()
        .iter()
        .any(|event| event.kind == "expired"));
    let mut key = command("put", 7);
    key.now = 1;
    key.key = b"b".to_vec();
    service.execute(key).unwrap();
    let mut evict = command("evict", 8);
    evict.now = 1;
    evict.key = b"b".to_vec();
    service.execute(evict).unwrap();
    assert_eq!(values.drain().unwrap().last().unwrap().kind, "evicted");
    let mut put_after = command("put", 9);
    put_after.now = 1;
    service.execute(put_after).unwrap();
    let mut clear = command("clear_detailed", 10);
    clear.now = 1;
    service.execute(clear).unwrap();
    assert_eq!(values.drain().unwrap().last().unwrap().kind, "cleared");
    drop(metadata);
    drop(limited);
    assert_eq!(service.statistics().listeners, 1);
    let mut destroy = command("destroy", 11);
    destroy.now = 1;
    service.execute(destroy).unwrap();
    assert_eq!(values.drain().unwrap()[0].kind, "destroyed");
    assert_eq!(service.statistics().listeners, 0);
    assert!(service.snapshot(1).unwrap().is_empty());
}

#[test]
fn statistics_snapshot_tracks_live_and_retained_owners() {
    let service = service();
    let mut request = command("put", 1);
    request.value = Some(b"12345".to_vec());
    service.execute(request).unwrap();
    let listener = service
        .register_listener(ServiceListenerOptions::default())
        .unwrap();
    put(&service, 2, b"123");
    let stats = service.statistics();
    assert_eq!(stats.live_entries, 1);
    assert_eq!(stats.value_bytes, 3);
    assert_eq!(stats.retained_outcomes, 2);
    assert_eq!(stats.listeners, 1);
    assert_eq!(stats.queued_events, 1);
    assert_eq!(stats.requests, 2);
    assert!(service
        .execute(ServiceCommand::point("missing", b""))
        .is_err());
    assert_eq!(service.statistics().rejections, 1);
    listener.drain().unwrap();
    drop(listener);
    assert_eq!(service.statistics().queued_events, 0);
    assert_eq!(service.statistics().listeners, 0);
    let mut destroy = command("destroy", 3);
    destroy.value = None;
    service.execute(destroy).unwrap();
    let stats = service.statistics();
    assert_eq!(stats.live_entries, 0);
    assert_eq!(stats.value_bytes, 0);
    assert_eq!(stats.retained_outcomes, 0);
}

#[test]
fn typed_errors_preserve_domain_failure_reasons() {
    assert_eq!(
        MapServiceError::Model(CollectionError::CursorMismatch).kind(),
        "cursor_mismatch"
    );
    assert_eq!(
        MapServiceError::Model(CollectionError::CursorStale {
            cursor_revision: 1,
            current_revision: 2
        })
        .kind(),
        "scan_cursor_stale"
    );
    assert_eq!(
        UnifiedIMapService::new(CollectionBounds::default(), 0)
            .err()
            .unwrap()
            .kind(),
        "invalid_argument"
    );
    let small = UnifiedIMapService::new(
        CollectionBounds {
            max_key_bytes: 1,
            max_value_bytes: 2,
            max_entries: 2,
            max_response_bytes: 64,
            ..CollectionBounds::default()
        },
        1,
    )
    .unwrap();
    let mut bad = command("put", 1);
    bad.key = b"long".to_vec();
    assert_eq!(small.execute(bad).unwrap_err().kind(), "key_too_large");
    let mut bad = command("put", 1);
    bad.value = Some(b"long".to_vec());
    assert_eq!(small.execute(bad).unwrap_err().kind(), "value_too_large");
    assert_eq!(
        small
            .execute(ServiceCommand::point("put", b"a"))
            .unwrap_err()
            .kind(),
        "invalid_argument"
    );
    assert_eq!(
        small
            .execute(ServiceCommand::point("unknown", b"a"))
            .unwrap_err()
            .kind(),
        "unsupported_operation"
    );
    assert_eq!(
        small.drain_listener(99).unwrap_err().kind(),
        "unknown_listener"
    );
    put(&small, 1, b"v");
    put(&small, 2, b"v");
    assert_eq!(
        small.execute(command("put", 3)).unwrap_err().kind(),
        "idempotency_capacity_exhausted"
    );
    assert_eq!(
        small
            .register_listener(ServiceListenerOptions {
                queue_capacity: 0,
                ..ServiceListenerOptions::default()
            })
            .err()
            .unwrap()
            .kind(),
        "invalid_argument"
    );
    assert_eq!(
        service()
            .register_listener(ServiceListenerOptions {
                kinds: BTreeSet::from(["arbitrary".into()]),
                ..ServiceListenerOptions::default()
            })
            .err()
            .unwrap()
            .kind(),
        "invalid_argument"
    );
    let mut read = ServiceCommand::point("get", b"a");
    read.now = 2;
    small.execute(read.clone()).unwrap();
    read.now = 1;
    assert_eq!(small.execute(read).unwrap_err().kind(), "clock_regressed");
    let mut destroy = command("destroy", 4);
    destroy.now = 2;
    small.execute(destroy).unwrap();
    assert_eq!(
        small
            .execute(ServiceCommand::point("get", b"a"))
            .unwrap_err()
            .kind(),
        "map_destroyed"
    );
}

#[test]
fn conditional_bulk_ttl_and_read_errors_share_the_single_store() {
    let service = service();
    let mut missing = command("get_or_default", 1);
    missing.value = Some(b"fallback".to_vec());
    assert_eq!(
        service.execute(missing).unwrap().value,
        Some(b"fallback".to_vec())
    );
    let mut absent = command("put_if_absent", 1);
    absent.ttl = ServiceTtl::ExpireAfter(10);
    service.execute(absent).unwrap();
    let mut mismatch = command("conditional_replace", 2);
    mismatch.expected = Some(b"wrong".to_vec());
    assert_eq!(service.execute(mismatch).unwrap().kind, "mismatch");
    let mut replace = command("conditional_replace", 3);
    replace.value = Some(b"new".to_vec());
    replace.ttl = ServiceTtl::Preserve;
    assert_eq!(service.execute(replace).unwrap().remaining_ttl, Some(10));
    let mut remove = command("conditional_remove", 4);
    remove.expected = Some(b"new".to_vec());
    assert_eq!(service.execute(remove).unwrap().kind, "removed");
    let mut batch = command("put_all_detailed", 5);
    batch.entries = vec![
        ReplacementEntry {
            key: b"a".to_vec(),
            value: b"x".to_vec(),
        },
        ReplacementEntry {
            key: b"b".to_vec(),
            value: b"y".to_vec(),
        },
    ];
    assert_eq!(service.execute(batch).unwrap().number, 2);
    let mut read = ServiceCommand::point("get_all", b"");
    read.keys = vec![b"b".to_vec(), b"missing".to_vec(), b"a".to_vec()];
    let items = service.execute(read).unwrap().items;
    assert_eq!(
        items
            .iter()
            .map(|item| item.kind.as_str())
            .collect::<Vec<_>>(),
        ["present", "absent", "present"]
    );
    let mut replace = command("replace_all_detailed", 6);
    replace.entries = vec![ReplacementEntry {
        key: b"b".to_vec(),
        value: b"z".to_vec(),
    }];
    assert_eq!(
        service.execute(replace.clone()).unwrap().items[0].kind,
        "replaced"
    );
    let revision = service.revision();
    service.execute(replace).unwrap();
    assert_eq!(service.revision(), revision);
    let mut duplicate = command("put_all_detailed", 7);
    duplicate.entries = vec![
        ReplacementEntry {
            key: b"a".to_vec(),
            value: b"1".to_vec()
        };
        2
    ];
    assert_eq!(
        service.execute(duplicate).unwrap_err().kind(),
        "invalid_argument"
    );
    let small = UnifiedIMapService::new(
        CollectionBounds {
            max_entries: 1,
            ..CollectionBounds::default()
        },
        1,
    )
    .unwrap();
    let mut batch = command("put_all_detailed", 1);
    batch.entries = vec![
        ReplacementEntry {
            key: b"a".to_vec(),
            value: b"1".to_vec(),
        },
        ReplacementEntry {
            key: b"b".to_vec(),
            value: b"2".to_vec(),
        },
    ];
    assert_eq!(
        small.execute(batch).unwrap_err().kind(),
        "request_too_large"
    );
    assert!(small.snapshot(0).unwrap().is_empty());
}

#[test]
fn listeners_wake_on_events_and_removal_without_running_under_service_lock() {
    let service = service();
    let listener = service
        .register_listener(ServiceListenerOptions::default())
        .unwrap();
    let mut cx = Context::from_waker(Waker::noop());
    assert!(service
        .poll_listener(listener.id(), 1, &mut cx)
        .is_pending());
    put(&service, 1, b"v");
    assert!(
        matches!(service.poll_listener(listener.id(),1,&mut cx),Poll::Ready(Ok(Some(events)))if events.len()==1)
    );
    assert!(service
        .poll_listener(listener.id(), 1, &mut cx)
        .is_pending());
    assert!(service.remove_listener(listener.id()));
    assert!(matches!(
        service.poll_listener(listener.id(), 1, &mut cx),
        Poll::Ready(Err(MapServiceError::UnknownListener))
    ));
    assert!(matches!(
        service.poll_listener(listener.id(), 0, &mut cx),
        Poll::Ready(Err(MapServiceError::InvalidArgument))
    ));
}

#[tokio::test]
async fn service_page_streams_fetch_on_demand_and_release_stale_buffers() {
    let service = service();
    for (index, key) in [b"a", b"b", b"c"].into_iter().enumerate() {
        let mut request = command("put", index as u64 + 1);
        request.key = key.to_vec();
        service.execute(request).unwrap();
    }
    let source = ServicePageSource::new(service.clone(), ScanProjection::Entries, 0).unwrap();
    let requests = service.statistics().requests;
    let mut stream = DemandStream::new(source, 2).unwrap();
    assert_eq!(service.statistics().active_cursors, 1);
    assert_eq!(service.statistics().requests, requests);
    assert_eq!(stream.next().await.unwrap().unwrap().key, b"a");
    assert_eq!(stream.retained_items(), 1);
    assert_eq!(service.statistics().requests, requests + 1);
    put(&service, 4, b"changed");
    assert!(matches!(
        stream.next().await,
        Some(Err(DemandStreamError::Source(MapServiceError::Model(_))))
    ));
    assert_eq!(stream.retained_items(), 0);
    assert_eq!(service.statistics().active_streams, 0);
    assert!(stream.next().await.is_none());
    let mut complete = DemandStream::new(
        ServicePageSource::new(service.clone(), ScanProjection::Keys, 0).unwrap(),
        1,
    )
    .unwrap();
    let mut keys = Vec::new();
    while let Some(item) = complete.next().await {
        keys.push(item.unwrap().key);
    }
    assert_eq!(keys, [b"a".to_vec(), b"b".to_vec(), b"c".to_vec()]);
    assert_eq!(service.statistics().active_cursors, 0);
    let mut cancelled = DemandStream::new(
        ServicePageSource::new(service.clone(), ScanProjection::Values, 0).unwrap(),
        2,
    )
    .unwrap();
    cancelled.cancel();
    assert_eq!(service.statistics().active_cursors, 0);
}

#[test]
fn service_listener_streams_wait_and_release_registration_on_cancel() {
    let service = service();
    let source =
        ServiceListenerSource::new(service.clone(), ServiceListenerOptions::default()).unwrap();
    let mut stream = DemandStream::new(source, 2).unwrap();
    let mut cx = Context::from_waker(Waker::noop());
    assert!(Pin::new(&mut stream).poll_next(&mut cx).is_pending());
    assert_eq!(service.statistics().listeners, 1);
    assert_eq!(service.statistics().active_streams, 1);
    put(&service, 1, b"v");
    assert!(
        matches!(Pin::new(&mut stream).poll_next(&mut cx),Poll::Ready(Some(Ok(event)))if event.kind=="added")
    );
    stream.cancel();
    assert_eq!(service.statistics().listeners, 0);
    assert_eq!(service.statistics().active_streams, 0);
    assert!(matches!(
        Pin::new(&mut stream).poll_next(&mut cx),
        Poll::Ready(None)
    ));
    let mut terminal = DemandStream::new(
        ServiceListenerSource::new(service.clone(), ServiceListenerOptions::default()).unwrap(),
        1,
    )
    .unwrap();
    service.execute(command("destroy", 2)).unwrap();
    assert!(
        matches!(Pin::new(&mut terminal).poll_next(&mut cx),Poll::Ready(Some(Ok(event)))if event.kind=="destroyed")
    );
    assert!(matches!(
        Pin::new(&mut terminal).poll_next(&mut cx),
        Poll::Ready(None)
    ));
    assert_eq!(service.statistics().listeners, 0);
    assert_eq!(service.statistics().active_streams, 0);
}

#[test]
fn service_aliases_linearize_conditional_races_and_emit_one_event() {
    let service = service();
    let listener = service
        .register_listener(ServiceListenerOptions::default())
        .unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
    let threads = (1..=8)
        .map(|sequence| {
            let service = service.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let mut request = command("put_if_absent", sequence);
                request.value = Some(vec![sequence as u8]);
                barrier.wait();
                service.execute(request).unwrap()
            })
        })
        .collect::<Vec<_>>();
    let results = threads
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        results
            .iter()
            .filter(|result| result.kind == "inserted")
            .count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .filter(|result| result.kind == "present")
            .count(),
        7
    );
    assert_eq!(listener.drain().unwrap().len(), 1);
    let token = view(&service).token;
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
    let threads = (9..=16)
        .map(|sequence| {
            let service = service.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let mut request = command("replace_if_version", sequence);
                request.token = token;
                request.value = Some(vec![sequence as u8]);
                barrier.wait();
                service.execute(request).unwrap()
            })
        })
        .collect::<Vec<_>>();
    let results = threads
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        results
            .iter()
            .filter(|result| result.kind == "replaced")
            .count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .filter(|result| result.kind == "mismatch")
            .count(),
        7
    );
    assert_eq!(listener.drain().unwrap().len(), 1);
    assert_eq!(service.statistics().retained_outcomes, 16);
}

#[test]
fn bulk_replacement_events_preserve_each_applied_version_and_input_order() {
    let service = service();
    let mut b = command("put", 1);
    b.key = b"b".to_vec();
    b.value = Some(b"old-b".to_vec());
    service.execute(b).unwrap();
    put(&service, 2, b"old-a");
    let listener = service
        .register_listener(ServiceListenerOptions {
            include_value: true,
            ..ServiceListenerOptions::default()
        })
        .unwrap();
    let mut replacement = command("replace_all_detailed", 3);
    replacement.entries = vec![
        ReplacementEntry {
            key: b"b".to_vec(),
            value: b"new-b".to_vec(),
        },
        ReplacementEntry {
            key: b"a".to_vec(),
            value: b"new-a".to_vec(),
        },
    ];
    let result = service.execute(replacement).unwrap();
    let events = listener.drain().unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(
        events
            .iter()
            .map(|event| event.key.as_slice())
            .collect::<Vec<_>>(),
        [b"b".as_slice(), b"a".as_slice()]
    );
    assert_eq!(events[0].version, result.items[0].version.unwrap());
    assert_eq!(events[1].version, result.items[1].version.unwrap());
    assert!(events[0].version < events[1].version);
    assert_eq!(events[1].version, service.revision());
    assert_eq!(events[0].old_value.as_deref(), Some(b"old-b".as_slice()));
    assert_eq!(events[0].value.as_deref(), Some(b"new-b".as_slice()));
    assert_eq!(events[1].old_value.as_deref(), Some(b"old-a".as_slice()));
    assert_eq!(events[1].value.as_deref(), Some(b"new-a".as_slice()));
}

#[test]
fn external_listener_removal_discards_prefetched_events() {
    let service = service();
    let source =
        ServiceListenerSource::new(service.clone(), ServiceListenerOptions::default()).unwrap();
    let registration = source.registration_id().unwrap();
    let mut stream = DemandStream::new(source, 2).unwrap();
    put(&service, 1, b"one");
    put(&service, 2, b"two");
    let mut cx = Context::from_waker(Waker::noop());
    assert!(
        matches!(Pin::new(&mut stream).poll_next(&mut cx), Poll::Ready(Some(Ok(event))) if event.kind == "added")
    );
    assert_eq!(stream.retained_items(), 1);
    assert!(service.remove_listener(registration));
    assert!(matches!(
        Pin::new(&mut stream).poll_next(&mut cx),
        Poll::Ready(Some(Err(DemandStreamError::Source(
            MapServiceError::UnknownListener
        ))))
    ));
    assert_eq!(stream.retained_items(), 0);
    assert_eq!(service.statistics().listeners, 0);
    assert_eq!(service.statistics().active_streams, 0);
    assert!(matches!(
        Pin::new(&mut stream).poll_next(&mut cx),
        Poll::Ready(None)
    ));
}
