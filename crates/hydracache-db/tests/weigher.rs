use std::error::Error;
use std::fmt;

use hydracache::HydraCache;
use hydracache_core::{CacheCodec, PostcardCodec};
use hydracache_db::{DbCache, QueryCachePolicy};

#[derive(Debug)]
struct LoaderError;

impl fmt::Display for LoaderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("loader failed")
    }
}

impl Error for LoaderError {}

fn encoded_len(value: &[u8]) -> usize {
    PostcardCodec.encode(&value).unwrap().len()
}

async fn load_vec(queries: &DbCache, key: &'static str, value: Vec<u8>) -> Vec<u8> {
    queries
        .cached_with::<Vec<u8>>(QueryCachePolicy::named(key).key(key))
        .load(move || async move { Ok::<_, LoaderError>(value) })
        .await
        .unwrap()
}

#[tokio::test]
async fn weigher_uses_encoded_byte_length() {
    let first = vec![1_u8; 64];
    let second = vec![2_u8; 64];
    let max_bytes = encoded_len(&first) + encoded_len(&second) - 1;
    let cache = HydraCache::local()
        .max_capacity(max_bytes as u64)
        .max_entry_bytes(1024)
        .build();
    let queries = DbCache::new(cache.clone(), "db");

    load_vec(&queries, "first", first).await;
    load_vec(&queries, "second", second).await;

    let diagnostics = cache.diagnostics().await;
    assert!(
        diagnostics.estimated_entries < 2,
        "combined encoded byte weight should exceed max_capacity"
    );
    assert_eq!(cache.stats().oversize_rejections, 0);
}

#[tokio::test]
async fn oversize_entry_is_rejected_before_insert() {
    let cache = HydraCache::local().max_entry_bytes(8).build();
    let queries = DbCache::new(cache.clone(), "db");
    let value = vec![7_u8; 64];

    let loaded = load_vec(&queries, "too-large", value.clone()).await;

    assert_eq!(loaded, value);
    assert_eq!(cache.get_encoded("db:too-large").await.unwrap(), None);
    assert_eq!(cache.stats().oversize_rejections, 1);
}

#[tokio::test]
async fn rejected_oversize_counter_is_distinct_from_evictions() {
    let cache = HydraCache::local().max_entry_bytes(8).build();
    let queries = DbCache::new(cache.clone(), "db");

    load_vec(&queries, "too-large", vec![7_u8; 64]).await;
    let stats = cache.stats();

    assert_eq!(stats.oversize_rejections, 1);
    assert_eq!(stats.evictions, 0);
}

#[tokio::test]
async fn byte_budget_evicts_when_total_exceeds_max_bytes() {
    let first = vec![1_u8; 80];
    let second = vec![2_u8; 80];
    let cache = HydraCache::local()
        .max_capacity(encoded_len(&first) as u64)
        .max_entry_bytes(1024)
        .build();
    let queries = DbCache::new(cache.clone(), "db");

    load_vec(&queries, "first", first).await;
    load_vec(&queries, "second", second).await;

    assert!(cache.diagnostics().await.estimated_entries <= 1);
}
