#![cfg(not(target_arch = "wasm32"))]

use std::time::Duration;

use hydracache::{CacheOptions, HydraCache, MemoryInstrumentationMode, MemorySnapshotRequest};

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

fn assert_empty(snapshot: &hydracache::MemoryFootprintSnapshot) {
    assert_eq!(snapshot.live_entries, 0);
    assert_eq!(snapshot.logical_key_bytes, 0);
    assert_eq!(snapshot.logical_value_bytes, 0);
    assert_eq!(snapshot.tag_memberships, 0);
    assert_eq!(snapshot.estimated_retained_bytes, 0);
    assert_eq!(snapshot.pending_loads, 0);
}

#[tokio::test]
async fn hundred_ttl_delete_and_flush_cycles_return_logical_owners_to_zero() {
    let cache = HydraCache::local()
        .memory_instrumentation_mode(MemoryInstrumentationMode::Production)
        .max_capacity(4 * 1024 * 1024)
        .build();

    for cycle in 0..100 {
        let ttl_key = format!("ttl-{cycle}");
        cache
            .put(
                &ttl_key,
                vec![1_u8; 32],
                CacheOptions::new()
                    .tag(format!("tag-{cycle}"))
                    .ttl(Duration::from_millis(1)),
            )
            .await
            .expect("ttl put");
        tokio::time::sleep(Duration::from_millis(2)).await;
        assert!(!cache.contains_key(&ttl_key).await);

        let delete_key = format!("delete-{cycle}");
        cache
            .put(
                &delete_key,
                vec![2_u8; 32],
                CacheOptions::new().tag("delete"),
            )
            .await
            .expect("delete put");
        assert!(cache.remove(&delete_key).await.expect("delete"));

        let flush_key = format!("flush-{cycle}");
        cache
            .put(&flush_key, vec![3_u8; 32], CacheOptions::new().tag("flush"))
            .await
            .expect("flush put");
        cache.flush().await.expect("flush");
        assert_empty(&exact_snapshot(&cache).await);
        assert!(
            cache
                .reconcile_memory_footprint()
                .await
                .expect("reconcile")
                .matched
        );
    }
}

#[tokio::test]
async fn invalidation_during_load_does_not_publish_a_stale_value() {
    let cache = HydraCache::local()
        .memory_instrumentation_mode(MemoryInstrumentationMode::Production)
        .build();
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = tokio::sync::oneshot::channel();
    let worker_cache = cache.clone();
    let worker = tokio::spawn(async move {
        worker_cache
            .get_or_insert_with(
                "stale",
                CacheOptions::new().tag("fence"),
                move || async move {
                    let _ = started_tx.send(());
                    let _ = release_rx.await;
                    7_u64
                },
            )
            .await
    });
    started_rx.await.expect("loader started");
    cache.invalidate_tag("fence").await.expect("invalidate");
    release_tx.send(()).expect("release loader");
    let _ = worker.await.expect("worker");
    assert!(!cache.contains_key("stale").await);
    assert_empty(&exact_snapshot(&cache).await);
}
