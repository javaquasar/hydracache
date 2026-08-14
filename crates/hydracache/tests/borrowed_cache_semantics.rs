use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use hydracache::{CacheError, CacheEventKind, CacheOptions, HydraCache};
use serde::Deserialize;
use tokio::sync::Barrier;

#[derive(Debug, Deserialize)]
struct Manifest {
    rows: Vec<Row>,
}

#[derive(Debug, Deserialize)]
struct Row {
    id: String,
    expected: String,
    hydracache_test: String,
}

#[derive(Debug)]
struct LoaderError;

impl std::fmt::Display for LoaderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("borrowed expectation loader failed")
    }
}

impl std::error::Error for LoaderError {}

fn manifest() -> Manifest {
    serde_json::from_str(include_str!(
        "../../../docs/integrations/cache_semantics_borrowed.json"
    ))
    .expect("borrowed cache manifest must parse")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn borrowed_cache_semantics_rows_all_execute_and_match_manifest() {
    execute_manifest().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn canary_cache_semantics_runner_skips_a_listed_row() {
    execute_manifest().await;
}

async fn execute_manifest() {
    let rows = manifest().rows;
    let expected: BTreeMap<_, _> = rows
        .iter()
        .map(|row| (row.id.clone(), row.hydracache_test.clone()))
        .collect();
    assert_eq!(expected.len(), rows.len(), "manifest IDs must be unique");

    let mut observed = BTreeMap::new();
    for row in rows {
        if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W2_SKIP")
            && row.id == "cache-put-get"
        {
            continue;
        }
        let outcome = execute(&row.id).await;
        assert_eq!(outcome, row.expected, "unexpected outcome for {}", row.id);
        observed.insert(row.id, row.hydracache_test);
    }
    assert_eq!(
        observed, expected,
        "HC-CANARY-RED:W2 a listed borrowed cache row was not executed"
    );
}

#[test]
fn no_row_is_silently_absent_from_execution() {
    let rows = manifest().rows;
    let manifest_ids: BTreeSet<_> = rows.iter().map(|row| row.id.as_str()).collect();
    let executable_ids: BTreeSet<_> = EXECUTABLE_IDS.iter().copied().collect();
    assert_eq!(manifest_ids, executable_ids);
    assert_eq!(rows.len(), EXECUTABLE_IDS.len());
}

const EXECUTABLE_IDS: [&str; 10] = [
    "cache-absent",
    "cache-put-get",
    "cache-single-flight",
    "cache-loader-error",
    "cache-invalidate",
    "cache-expiry",
    "cache-mutation-listener",
    "cache-namespaces",
    "cache-weighted-eviction",
    "cache-idle-expiry",
];

async fn execute(id: &str) -> &'static str {
    match id {
        "cache-absent" => {
            let cache = HydraCache::local().build();
            assert_eq!(cache.get::<u64>("missing").await.unwrap(), None);
            "pass"
        }
        "cache-put-get" => {
            let cache = HydraCache::local().build();
            cache.put("key", 42_u64, CacheOptions::new()).await.unwrap();
            assert_eq!(cache.get::<u64>("key").await.unwrap(), Some(42));
            "pass"
        }
        "cache-single-flight" => {
            let cache = HydraCache::local().build();
            let calls = Arc::new(AtomicUsize::new(0));
            let barrier = Arc::new(Barrier::new(8));
            let mut tasks = Vec::new();
            for _ in 0..8 {
                let cache = cache.clone();
                let calls = calls.clone();
                let barrier = barrier.clone();
                tasks.push(tokio::spawn(async move {
                    barrier.wait().await;
                    cache
                        .get_or_load("one", CacheOptions::new(), move || async move {
                            calls.fetch_add(1, Ordering::SeqCst);
                            tokio::time::sleep(Duration::from_millis(20)).await;
                            Ok::<_, LoaderError>(7_u64)
                        })
                        .await
                        .unwrap()
                }));
            }
            for task in tasks {
                assert_eq!(task.await.unwrap(), 7);
            }
            assert_eq!(calls.load(Ordering::SeqCst), 1);
            "pass"
        }
        "cache-loader-error" => {
            let cache = HydraCache::local().build();
            let failed = cache
                .get_or_load("retry", CacheOptions::new(), || async {
                    Err::<u64, _>(LoaderError)
                })
                .await;
            assert!(matches!(failed, Err(CacheError::Loader(_))));
            let loaded = cache
                .get_or_load("retry", CacheOptions::new(), || async {
                    Ok::<_, LoaderError>(9_u64)
                })
                .await
                .unwrap();
            assert_eq!(loaded, 9);
            "pass"
        }
        "cache-invalidate" => {
            let cache = HydraCache::local().build();
            cache.put("key", 1_u64, CacheOptions::new()).await.unwrap();
            assert!(cache.invalidate_key("key").await.unwrap());
            assert_eq!(cache.get::<u64>("key").await.unwrap(), None);
            "pass"
        }
        "cache-expiry" => {
            let cache = HydraCache::local().build();
            cache
                .put(
                    "short",
                    1_u64,
                    CacheOptions::new().ttl(Duration::from_millis(5)),
                )
                .await
                .unwrap();
            tokio::time::sleep(Duration::from_millis(25)).await;
            assert_eq!(cache.get::<u64>("short").await.unwrap(), None);
            "pass"
        }
        "cache-mutation-listener" => {
            let cache = HydraCache::local().build();
            let typed = cache.typed::<u64>("borrowed");
            let mut events = typed.subscribe_mutations();
            typed.put("key", 1, CacheOptions::new()).await.unwrap();
            let event = tokio::time::timeout(Duration::from_secs(1), events.recv())
                .await
                .unwrap()
                .unwrap();
            assert_eq!(event.kind(), CacheEventKind::Stored);
            assert_eq!(event.key(), Some("borrowed:key"));
            "pass"
        }
        "cache-namespaces" => {
            let cache = HydraCache::local().build();
            let first = cache.typed::<u64>("first");
            let second = cache.typed::<u64>("second");
            first.put("key", 1, CacheOptions::new()).await.unwrap();
            second.put("key", 2, CacheOptions::new()).await.unwrap();
            assert_eq!(first.get("key").await.unwrap(), Some(1));
            assert_eq!(second.get("key").await.unwrap(), Some(2));
            "pass"
        }
        "cache-weighted-eviction" => {
            let options_source = include_str!("../../hydracache-core/src/options.rs");
            assert!(!options_source.contains("pub fn weigher"));
            "unsupported-documented"
        }
        "cache-idle-expiry" => {
            let options_source = include_str!("../../hydracache-core/src/options.rs");
            assert!(!options_source.contains("pub fn time_to_idle"));
            "unsupported-documented"
        }
        unknown => panic!("manifest row {unknown} has no executable expectation"),
    }
}
