#![cfg(not(target_arch = "wasm32"))]

use std::collections::{BTreeMap, BTreeSet};

use hydracache::{CacheOptions, HydraCache, MemoryInstrumentationMode, MemorySnapshotRequest};

async fn exact_snapshot(cache: &HydraCache) -> hydracache::MemoryFootprintSnapshot {
    cache.diagnostics().await;
    let barrier = cache.memory_snapshot_barrier().expect("barrier");
    cache
        .memory_footprint_snapshot(MemorySnapshotRequest::Exact {
            acknowledged_epoch: barrier.epoch,
        })
        .await
        .expect("snapshot")
}

#[tokio::test]
async fn deterministic_interleavings_match_a_reference_membership_model() {
    let cache = HydraCache::local()
        .memory_instrumentation_mode(MemoryInstrumentationMode::Production)
        .build();
    let mut model = BTreeMap::<String, BTreeSet<String>>::new();

    for step in 0..512 {
        let key = format!("key-{}", step % 31);
        let tag = format!("tag-{}", step % 7);
        match step % 4 {
            0 | 1 => {
                cache
                    .put(&key, step, CacheOptions::new().tag(tag.clone()))
                    .await
                    .expect("put");
                model.insert(key, BTreeSet::from([tag]));
            }
            2 => {
                cache.remove(&key).await.expect("remove");
                model.remove(&key);
            }
            _ => {
                cache.invalidate_tag(&tag).await.expect("invalidate tag");
                model.retain(|_, tags| !tags.contains(&tag));
            }
        }
    }

    for index in 0..31 {
        let key = format!("key-{index}");
        assert_eq!(cache.contains_key(&key).await, model.contains_key(&key));
    }
    let snapshot = exact_snapshot(&cache).await;
    assert_eq!(snapshot.live_entries as usize, model.len());
    assert_eq!(
        snapshot.tag_memberships as usize,
        model.values().map(BTreeSet::len).sum::<usize>()
    );
}

#[tokio::test]
async fn high_fanout_membership_is_reclaimed_by_tag_invalidation_and_flush() {
    let cache = HydraCache::local()
        .memory_instrumentation_mode(MemoryInstrumentationMode::Production)
        .build();
    for index in 0..1_000 {
        let key = format!("fanout-{index}");
        cache
            .put(&key, index, CacheOptions::new().tag("fanout"))
            .await
            .expect("put");
    }
    assert_eq!(
        cache.invalidate_tag("fanout").await.expect("invalidate"),
        1_000
    );
    let after_invalidation = exact_snapshot(&cache).await;
    assert_eq!(after_invalidation.live_entries, 0);
    assert_eq!(after_invalidation.tag_memberships, 0);
    cache.flush().await.expect("flush");
    assert_eq!(exact_snapshot(&cache).await.tag_generation_records, 0);
}
