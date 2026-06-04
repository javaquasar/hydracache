use std::error::Error;
use std::fmt::{Display, Formatter};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use hydracache::{cacheable, HydraCache};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ExpensiveValue {
    id: u64,
}

#[derive(Debug)]
struct LoadError;

impl Display for LoadError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("load failed")
    }
}

impl Error for LoadError {}

#[tokio::test]
async fn cacheable_macro_caches_loader_result() {
    let cache = HydraCache::local().build();
    let calls = Arc::new(AtomicUsize::new(0));

    let first: ExpensiveValue = cacheable!(
        cache = cache,
        key = "expensive:1",
        tag = "expensive",
        ttl_secs = 60,
        load = {
            let calls = Arc::clone(&calls);
            move || async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, LoadError>(ExpensiveValue { id: 1 })
            }
        },
    )
    .await
    .unwrap();

    let second: ExpensiveValue = cacheable!(
        cache = cache,
        key = "expensive:1",
        tag = "expensive",
        ttl_secs = 60,
        load = {
            let calls = Arc::clone(&calls);
            move || async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, LoadError>(ExpensiveValue { id: 2 })
            }
        },
    )
    .await
    .unwrap();

    assert_eq!(first, ExpensiveValue { id: 1 });
    assert_eq!(second, ExpensiveValue { id: 1 });
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn cacheable_macro_applies_tags_and_ttl() {
    let cache = HydraCache::local().build();
    let ttl_calls = Arc::new(AtomicUsize::new(0));

    let first: ExpensiveValue = cacheable!(
        cache = cache,
        key = "expiring:1",
        tag = "expiring",
        ttl = Duration::from_millis(20),
        load = {
            let ttl_calls = Arc::clone(&ttl_calls);
            move || async move {
                ttl_calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, LoadError>(ExpensiveValue { id: 1 })
            }
        },
    )
    .await
    .unwrap();

    tokio::time::sleep(Duration::from_millis(30)).await;

    let second: ExpensiveValue = cacheable!(
        cache = cache,
        key = "expiring:1",
        tag = "expiring",
        ttl = Duration::from_millis(20),
        load = {
            let ttl_calls = Arc::clone(&ttl_calls);
            move || async move {
                ttl_calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, LoadError>(ExpensiveValue { id: 2 })
            }
        },
    )
    .await
    .unwrap();

    assert_eq!(first, ExpensiveValue { id: 1 });
    assert_eq!(second, ExpensiveValue { id: 2 });
    assert_eq!(ttl_calls.load(Ordering::SeqCst), 2);

    cache.invalidate_tag("expiring").await.unwrap();
    assert!(!cache.contains_key("expiring:1").await);
}
