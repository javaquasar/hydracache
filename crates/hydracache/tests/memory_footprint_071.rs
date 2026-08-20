#![cfg(not(target_arch = "wasm32"))]

use std::time::Duration;

use hydracache::{
    CacheOptions, HydraCache, MemoryInstrumentationMode, MemorySnapshotConsistency,
    MemorySnapshotRequest,
};

async fn exact_snapshot(cache: &HydraCache) -> hydracache::MemoryFootprintSnapshot {
    cache.diagnostics().await;
    let barrier = cache.memory_snapshot_barrier().expect("quiescent barrier");
    cache
        .memory_footprint_snapshot(MemorySnapshotRequest::Exact {
            acknowledged_epoch: barrier.epoch,
        })
        .await
        .expect("exact snapshot")
}

#[tokio::test]
async fn insert_replace_delete_tag_expire_and_flush_update_bounded_aggregates() {
    let cache = HydraCache::local().max_capacity(1024 * 1024).build();
    cache
        .put(
            "secret-user-key",
            "secret-payload",
            CacheOptions::new().tags(["group", "tenant"]),
        )
        .await
        .expect("insert");

    let inserted = exact_snapshot(&cache).await;
    assert_eq!(inserted.live_entries, 1);
    assert_eq!(inserted.logical_key_bytes, 15);
    assert_eq!(inserted.tag_memberships, 2);
    assert_eq!(inserted.consistency, MemorySnapshotConsistency::Exact);
    let encoded = serde_json::to_string(&inserted).expect("serialize");
    assert!(!encoded.contains("secret-user-key"));
    assert!(!encoded.contains("secret-payload"));

    cache
        .put(
            "secret-user-key",
            vec![7_u8; 64],
            CacheOptions::new().tag("replacement"),
        )
        .await
        .expect("replace");
    let replaced = exact_snapshot(&cache).await;
    assert_eq!(replaced.live_entries, 1);
    assert_eq!(replaced.tag_memberships, 1);

    assert!(cache.remove("secret-user-key").await.expect("remove"));
    let removed = exact_snapshot(&cache).await;
    assert_eq!(removed.live_entries, 0);
    assert_eq!(removed.tag_memberships, 0);

    cache
        .put(
            "expiring",
            42_u64,
            CacheOptions::new()
                .tag("short-lived")
                .ttl(Duration::from_millis(1)),
        )
        .await
        .expect("expiring insert");
    tokio::time::sleep(Duration::from_millis(5)).await;
    assert!(!cache.contains_key("expiring").await);
    let expired = exact_snapshot(&cache).await;
    assert_eq!(expired.live_entries, 0);
    assert_eq!(expired.tag_generation_records, 0);

    cache
        .put("flush-a", 1_u64, CacheOptions::new().tag("flush"))
        .await
        .expect("flush-a");
    cache
        .put("flush-b", 2_u64, CacheOptions::new().tag("flush"))
        .await
        .expect("flush-b");
    assert_eq!(
        cache.invalidate_tag("flush").await.expect("tag invalidate"),
        2
    );
    let tag_invalidated = exact_snapshot(&cache).await;
    assert_eq!(tag_invalidated.live_entries, 0);
    assert_eq!(tag_invalidated.tag_generation_records, 1);

    cache
        .put("flush-c", 3_u64, CacheOptions::new())
        .await
        .expect("flush-c");
    cache.flush().await.expect("flush");
    let flushed = exact_snapshot(&cache).await;
    assert_eq!(flushed.live_entries, 0);
    assert!(
        cache
            .reconcile_memory_footprint()
            .await
            .expect("reconcile")
            .matched
    );
}

#[tokio::test]
async fn snapshot_reports_subscriptions_and_off_mode_without_labels() {
    let cache = HydraCache::local().event_buffer_capacity(8).build();
    let subscriber = cache.subscribe_mutations();
    cache
        .put("redacted-key", 1_u64, CacheOptions::new())
        .await
        .expect("put");
    let snapshot = cache
        .memory_footprint_snapshot(MemorySnapshotRequest::Admin)
        .await
        .expect("admin snapshot");
    assert_eq!(snapshot.event_subscribers, 1);
    assert!(snapshot.event_ring_occupancy <= 8);
    assert!(snapshot.owners.len() <= 8);
    drop(subscriber);
    assert_eq!(
        cache
            .memory_footprint_snapshot(MemorySnapshotRequest::Admin)
            .await
            .expect("closed subscription snapshot")
            .event_subscribers,
        0
    );

    let off = HydraCache::local()
        .memory_instrumentation_mode(MemoryInstrumentationMode::Off)
        .build();
    off.put("invisible", 1_u64, CacheOptions::new())
        .await
        .expect("off put");
    let off_snapshot = off
        .memory_footprint_snapshot(MemorySnapshotRequest::Admin)
        .await
        .expect("off snapshot");
    assert_eq!(off_snapshot.live_entries, 0);
    assert!(!off_snapshot.owners[0].available);
}

#[tokio::test]
async fn cancelled_load_releases_the_inflight_owner() {
    let cache = HydraCache::local().build();
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (_release_tx, release_rx) = tokio::sync::oneshot::channel::<()>();
    let worker_cache = cache.clone();
    let worker = tokio::spawn(async move {
        worker_cache
            .get_or_insert_with("cancelled", CacheOptions::new(), move || async move {
                let _ = started_tx.send(());
                let _ = release_rx.await;
                1_u64
            })
            .await
    });
    started_rx.await.expect("loader started");
    assert_eq!(
        cache
            .memory_footprint_snapshot(MemorySnapshotRequest::Admin)
            .await
            .expect("pending snapshot")
            .pending_loads,
        1
    );
    worker.abort();
    let _ = worker.await;
    for _ in 0..32 {
        if cache
            .memory_footprint_snapshot(MemorySnapshotRequest::Admin)
            .await
            .expect("cancel snapshot")
            .pending_loads
            == 0
        {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert_eq!(exact_snapshot(&cache).await.pending_loads, 0);
    assert!(
        cache
            .reconcile_memory_footprint()
            .await
            .expect("cancel reconciliation")
            .matched
    );
}

#[tokio::test]
async fn capacity_eviction_is_accounted_by_the_removal_listener() {
    let cache = HydraCache::local().max_capacity(16).build();
    cache
        .put("first", vec![1_u8; 16], CacheOptions::new())
        .await
        .expect("first");
    cache
        .put("second", vec![2_u8; 16], CacheOptions::new())
        .await
        .expect("second");
    cache.diagnostics().await;
    let snapshot = exact_snapshot(&cache).await;
    assert!(snapshot.live_entries <= 1);
    assert!(
        cache
            .reconcile_memory_footprint()
            .await
            .expect("reconcile")
            .matched
    );
}
