use futures_util::Stream;
use hydracache_cluster_testkit::value_plane_collection_075::CollectionBounds;
use hydracache_cluster_testkit::value_plane_model_075::MutationIdentity;
use hydracache_cluster_testkit::value_plane_service_075::*;
use hydracache_cluster_testkit::value_plane_service_cluster_075::*;
use hydracache_cluster_testkit::value_plane_stream_075::DemandStream;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

fn cluster() -> ServiceClusterHarness {
    ServiceClusterHarness::new(
        &["a", "b", "c"],
        ServiceClusterBounds::default(),
        CollectionBounds::default(),
        ServiceLimits::default(),
        1,
    )
    .unwrap()
}
fn key(cluster: &ServiceClusterHarness, p: u32) -> Vec<u8> {
    (0..10_000)
        .map(|n| format!("prefix-{n}").into_bytes())
        .find(|key| cluster.partition_for(key) == p)
        .unwrap()
}
fn put(cluster: &mut ServiceClusterHarness, key: &[u8], sequence: u64) {
    let mut command = ServiceCommand::point("put", key);
    command.value = Some(b"secret".to_vec());
    command.identity = Some(MutationIdentity::new("subscription", sequence));
    cluster
        .execute("c", command, ServiceClusterCall::default())
        .unwrap();
}

#[test]
fn subscription_covers_real_owners_and_preserves_partition_progress() {
    let mut cluster = cluster();
    let id = cluster
        .subscribe(ClusterSubscriptionOptions::default())
        .unwrap();
    for p in 0..3 {
        let key = key(&cluster, p);
        put(&mut cluster, &key, p as u64 + 1);
    }
    let events = cluster.poll_subscription(id, 256).unwrap();
    assert_eq!(events.len(), 3);
    for event in events {
        let ClusterSubscriptionEvent::Entry { progress, event } = event else {
            panic!("unexpected gap")
        };
        assert_eq!(progress.partition, cluster.partition_for(&event.key));
        assert_eq!(
            progress.epoch,
            cluster.assignment(progress.partition).unwrap().epoch
        );
        assert_eq!(progress.watermark, event.watermark);
        assert_eq!(event.kind, "added");
        assert!(event.value.is_none());
    }
    assert!(cluster.poll_subscription(id, 256).unwrap().is_empty());
    assert!(cluster.unsubscribe(id));
    assert!(!cluster.unsubscribe(id));
    assert_eq!(cluster.aggregate_statistics().listeners, 0);
}

#[test]
fn subscription_filters_values_and_empty_keys_share_local_semantics() {
    let mut cluster = cluster();
    let selected = key(&cluster, 1);
    let id = cluster
        .subscribe(ClusterSubscriptionOptions {
            listener: ServiceListenerOptions {
                key_prefix: b"prefix-".to_vec(),
                exact_key: Some(selected.clone()),
                include_value: true,
                ..ServiceListenerOptions::default()
            },
            ..ClusterSubscriptionOptions::default()
        })
        .unwrap();
    put(&mut cluster, b"", 1);
    let other = key(&cluster, 0);
    put(&mut cluster, &other, 2);
    put(&mut cluster, &selected, 3);
    let events = cluster.poll_subscription(id, 256).unwrap();
    assert_eq!(events.len(), 1);
    assert!(
        matches!(&events[0], ClusterSubscriptionEvent::Entry { event, .. } if event.key == selected && event.value.as_deref() == Some(b"secret".as_slice()))
    );
    let snapshot = cluster.resync_subscription(id, 0, 1, 1_024).unwrap();
    assert_eq!(snapshot.cuts.len(), 1);
    assert_eq!(snapshot.cuts[0].snapshot.entries[0].value, b"secret");
    drop(snapshot);
    cluster.unsubscribe(id);
    assert_eq!(cluster.aggregate_namespace_owners().resync_leases, 0);
}

#[test]
fn promotion_and_rebalance_emit_gap_before_new_owner_continuation() {
    let mut cluster = cluster();
    let id = cluster
        .subscribe(ClusterSubscriptionOptions::default())
        .unwrap();
    let selected = key(&cluster, 0);
    put(&mut cluster, &selected, 1);
    cluster.poll_subscription(id, 256).unwrap();
    let old = cluster.assignment(0).unwrap();
    cluster.set_node_live(&old.owner, false).unwrap();
    let promoted = cluster.promote_backup(0).unwrap();
    cluster
        .repair_backup(0, &promoted.backup, ServiceLinkFaults::default())
        .unwrap();
    put(&mut cluster, &selected, 2);
    let events = cluster.poll_subscription(id, 256).unwrap();
    assert!(!events.is_empty());
    assert!(matches!(
        &events[0],
        ClusterSubscriptionEvent::Gap {
            reason: ClusterGapReason::OwnerChanged | ClusterGapReason::OwnerUnavailable,
            ..
        }
    ));
    assert!(events
        .iter()
        .all(|event| matches!(event, ClusterSubscriptionEvent::Gap { .. })));
    let snapshot = cluster.resync_subscription(id, 0, 16, 4_096).unwrap();
    let cut = snapshot
        .cuts
        .iter()
        .find(|cut| cut.progress.partition == 0)
        .unwrap()
        .progress
        .clone();
    put(&mut cluster, &selected, 3);
    assert!(cluster.poll_subscription(id, 256).unwrap().iter().any(|event| matches!(event, ClusterSubscriptionEvent::Entry { progress, .. } if progress.partition == 0 && progress.epoch == promoted.epoch && progress.watermark > cut.watermark)));
    cluster.set_node_live(&old.owner, true).unwrap();
    cluster
        .rebalance(0, &old.owner, ServiceLinkFaults::default())
        .unwrap();
    assert!(matches!(
        cluster.poll_subscription(id, 256).unwrap().first(),
        Some(ClusterSubscriptionEvent::Gap { .. })
    ));
    drop(snapshot);
    cluster.unsubscribe(id);
    assert_eq!(cluster.aggregate_statistics().listeners, 0);
}

#[test]
fn slow_subscription_overflow_is_explicit_and_does_not_delay_backup_ack() {
    let mut cluster = cluster();
    let slow = cluster
        .subscribe(ClusterSubscriptionOptions {
            max_queued_events: 1,
            max_queued_bytes: 1_024,
            ..ClusterSubscriptionOptions::default()
        })
        .unwrap();
    let fast = cluster
        .subscribe(ClusterSubscriptionOptions::default())
        .unwrap();
    for p in 0..3 {
        let selected = key(&cluster, p);
        put(&mut cluster, &selected, p as u64 + 1);
        assert!(cluster.backup_proved(p));
    }
    assert!(matches!(
        cluster.poll_subscription(slow, 256).unwrap().first(),
        Some(ClusterSubscriptionEvent::Gap {
            reason: ClusterGapReason::QueueOverflow,
            ..
        })
    ));
    assert_eq!(cluster.poll_subscription(fast, 256).unwrap().len(), 3);
    let snapshot = cluster.resync_subscription(slow, 0, 16, 4_096).unwrap();
    drop(snapshot);
    cluster.unsubscribe(slow);
    cluster.unsubscribe(fast);
    assert_eq!(cluster.aggregate_statistics().listeners, 0);
}

#[test]
fn cluster_resync_discards_prefetch_and_releases_real_registration_owners() {
    let cluster = Arc::new(Mutex::new(cluster()));
    let source =
        ClusterListenerSource::new(cluster.clone(), ClusterSubscriptionOptions::default()).unwrap();
    let id = source.subscription_id();
    let mut stream = DemandStream::new(source, 2).unwrap();
    {
        let mut state = cluster.lock().unwrap();
        for p in 0..3 {
            let selected = key(&state, p);
            put(&mut state, &selected, p as u64 + 1);
        }
    }
    let mut cx = Context::from_waker(Waker::noop());
    assert!(matches!(
        Pin::new(&mut stream).poll_next(&mut cx),
        Poll::Ready(Some(Ok(_)))
    ));
    assert_eq!(stream.retained_items(), 1);
    let snapshot = cluster
        .lock()
        .unwrap()
        .resync_subscription(id, 0, 16, 4_096)
        .unwrap();
    assert!(Pin::new(&mut stream).poll_next(&mut cx).is_pending());
    assert_eq!(stream.retained_items(), 0);
    drop(snapshot);
    drop(stream);
    let state = cluster.lock().unwrap();
    assert_eq!(state.aggregate_statistics().listeners, 0);
    assert_eq!(state.aggregate_namespace_owners().resync_leases, 0);
}

#[test]
fn subscription_failure_bounds_and_namespace_fence_cleanup_are_explicit() {
    let mut cluster = cluster();
    assert!(cluster
        .subscribe(ClusterSubscriptionOptions {
            max_queued_events: 0,
            ..ClusterSubscriptionOptions::default()
        })
        .is_err());
    assert!(cluster
        .subscribe(ClusterSubscriptionOptions {
            max_queued_bytes: 1,
            ..ClusterSubscriptionOptions::default()
        })
        .is_err());
    let id = cluster
        .subscribe(ClusterSubscriptionOptions::default())
        .unwrap();
    assert!(cluster.poll_subscription(id, 0).is_err());
    assert!(cluster.poll_subscription(999, 1).is_err());
    assert!(cluster.resync_subscription(id, 0, 0, 1).is_err());
    let wakes = cluster.fence_namespace_deferred();
    for wake in wakes {
        wake.wake();
    }
    assert!(cluster.poll_subscription(id, 1).is_err());
    assert_eq!(cluster.aggregate_statistics().listeners, 0);
    while cluster.aggregate_namespace_owners().total() != 0 {
        cluster.reclaim_namespace_step(256).unwrap();
    }
    assert!(cluster.empty_successor(2).is_ok());
}

#[test]
fn cluster_source_credit_survives_external_unsubscribe_until_buffer_drop() {
    let cluster = Arc::new(Mutex::new(cluster()));
    let source =
        ClusterListenerSource::new(cluster.clone(), ClusterSubscriptionOptions::default()).unwrap();
    let id = source.subscription_id();
    let mut stream = DemandStream::new(source, 2).unwrap();
    {
        let mut state = cluster.lock().unwrap();
        for p in 0..2 {
            let selected = key(&state, p);
            put(&mut state, &selected, p as u64 + 1);
        }
    }
    let mut cx = Context::from_waker(Waker::noop());
    assert!(matches!(
        Pin::new(&mut stream).poll_next(&mut cx),
        Poll::Ready(Some(Ok(_)))
    ));
    assert_eq!(stream.retained_items(), 1);
    {
        let mut state = cluster.lock().unwrap();
        state.unsubscribe(id);
        assert_eq!(state.aggregate_namespace_owners().streams, 1);
        assert!(state.aggregate_resource_usage().streams > 0);
    }
    assert!(matches!(
        Pin::new(&mut stream).poll_next(&mut cx),
        Poll::Ready(Some(Err(
            hydracache_cluster_testkit::value_plane_stream_075::DemandStreamError::Source(
                ServiceClusterError::UnknownSubscription
            )
        )))
    ));
    drop(stream);
    assert_eq!(
        cluster.lock().unwrap().aggregate_namespace_owners().streams,
        0
    );
}

#[test]
fn local_filter_and_ttl_validation_precede_logical_time_or_expiry() {
    let bounds = CollectionBounds {
        max_key_bytes: 8,
        max_value_bytes: 1,
        ..CollectionBounds::default()
    };
    let service = UnifiedIMapService::new(bounds, 1).unwrap();
    let mut value = ServiceCommand::point("put", b"a");
    value.identity = Some(MutationIdentity::new("validation", 1));
    value.value = Some(b"v".to_vec());
    value.ttl = ServiceTtl::ExpireAfter(1);
    service.execute(value).unwrap();
    let before = service.resource_usage();
    let mut invalid = ServiceCommand::point("entry_listener", b"");
    invalid.expected = Some(vec![0; 9]);
    invalid.now = 1;
    assert_eq!(
        service.execute(invalid).unwrap_err().kind(),
        "key_too_large"
    );
    assert_eq!(service.logical_time(), 0);
    assert_eq!(service.resource_usage(), before);
    let mut valid = ServiceCommand::point("entry_listener", b"");
    valid.expected = Some(vec![0; 8]);
    valid.limit = 1;
    let exact = service.execute(valid).unwrap();
    assert_eq!(exact.kind, "listener_added");
    let mut prefix = ServiceCommand::point("entry_listener", b"pre");
    prefix.limit = 1;
    let prefix = service.execute(prefix).unwrap().number;
    for operation in ["set_ttl", "set_ttl_if_version"] {
        let mut command = ServiceCommand::point(operation, b"a");
        command.identity = Some(MutationIdentity::new("validation", 2));
        command.token = service
            .execute(ServiceCommand::point("get_entry_view", b"a"))
            .unwrap()
            .token;
        command.now = 1;
        command.ttl = ServiceTtl::Preserve;
        assert_eq!(
            service.execute(command).unwrap_err(),
            MapServiceError::InvalidArgument
        );
        assert_eq!(service.logical_time(), 0);
        assert_eq!(
            service
                .query_operation_outcome(&MutationIdentity::new("validation", 2))
                .unwrap(),
            OperationOutcomeQuery::Unknown
        );
    }
    let mut empty = ServiceCommand::point("put", b"");
    empty.value = Some(b"v".to_vec());
    empty.identity = Some(MutationIdentity::new("validation", 3));
    service.execute(empty).unwrap();
    assert!(service.drain_listener(exact.number).unwrap().is_empty());
    assert!(service.drain_listener(prefix).unwrap().is_empty());
    let mut clear = ServiceCommand::point("clear_detailed", b"");
    clear.identity = Some(MutationIdentity::new("validation", 4));
    service.execute(clear).unwrap();
    assert_eq!(
        service.drain_listener(exact.number).unwrap()[0].kind,
        "cleared"
    );
    assert_eq!(service.drain_listener(prefix).unwrap()[0].kind, "cleared");
}

#[test]
fn destroyed_event_precedes_eof_and_prefetch_drop_releases_all_subscription_owners() {
    let cluster = Arc::new(Mutex::new(
        ServiceClusterHarness::new(
            &["a", "b", "c"],
            ServiceClusterBounds {
                partitions: 1,
                ..ServiceClusterBounds::default()
            },
            CollectionBounds::default(),
            ServiceLimits::default(),
            1,
        )
        .unwrap(),
    ));
    let source =
        ClusterListenerSource::new(cluster.clone(), ClusterSubscriptionOptions::default()).unwrap();
    let mut stream = DemandStream::new(source, 2).unwrap();
    {
        let mut state = cluster.lock().unwrap();
        put(&mut state, b"a", 1);
        let mut destroy = ServiceCommand::point("destroy", b"");
        destroy.identity = Some(MutationIdentity::new("subscription", 2));
        state
            .execute("c", destroy, ServiceClusterCall::default())
            .unwrap();
    }
    let mut context = Context::from_waker(Waker::noop());
    assert!(
        matches!(Pin::new(&mut stream).poll_next(&mut context), Poll::Ready(Some(Ok(ClusterSubscriptionEvent::Entry { event, .. }))) if event.kind == "added")
    );
    assert_eq!(stream.retained_items(), 1);
    assert_eq!(
        cluster.lock().unwrap().aggregate_namespace_owners().streams,
        1
    );
    assert!(
        matches!(Pin::new(&mut stream).poll_next(&mut context), Poll::Ready(Some(Ok(ClusterSubscriptionEvent::Entry { event, .. }))) if event.kind == "destroyed")
    );
    assert!(matches!(
        Pin::new(&mut stream).poll_next(&mut context),
        Poll::Ready(None)
    ));
    assert!(matches!(
        Pin::new(&mut stream).poll_next(&mut context),
        Poll::Ready(None)
    ));
    let state = cluster.lock().unwrap();
    assert_eq!(state.aggregate_statistics().listeners, 0);
    assert_eq!(state.aggregate_namespace_owners().streams, 0);
    assert_eq!(state.aggregate_resource_usage().streams, 0);
    drop(state);
    let cancelled = Arc::new(Mutex::new(
        ServiceClusterHarness::new(
            &["a", "b", "c"],
            ServiceClusterBounds {
                partitions: 1,
                ..ServiceClusterBounds::default()
            },
            CollectionBounds::default(),
            ServiceLimits::default(),
            1,
        )
        .unwrap(),
    ));
    let mut cancelled_stream = DemandStream::new(
        ClusterListenerSource::new(cancelled.clone(), ClusterSubscriptionOptions::default())
            .unwrap(),
        2,
    )
    .unwrap();
    {
        let mut state = cancelled.lock().unwrap();
        put(&mut state, b"a", 1);
        let mut destroy = ServiceCommand::point("destroy", b"");
        destroy.identity = Some(MutationIdentity::new("subscription", 2));
        state
            .execute("c", destroy, ServiceClusterCall::default())
            .unwrap();
    }
    assert!(matches!(
        Pin::new(&mut cancelled_stream).poll_next(&mut context),
        Poll::Ready(Some(Ok(_)))
    ));
    assert_eq!(cancelled_stream.retained_items(), 1);
    drop(cancelled_stream);
    let state = cancelled.lock().unwrap();
    assert_eq!(state.aggregate_statistics().listeners, 0);
    assert_eq!(state.aggregate_namespace_owners().streams, 0);
}
