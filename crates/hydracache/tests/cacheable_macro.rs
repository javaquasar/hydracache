use std::error::Error;
use std::fmt::{Display, Formatter};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use hydracache::{cacheable_infallible, cacheable_loader, HydraCache, TagSet};
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

#[hydracache::cacheable(
    cache = cache,
    key_segments = ["attribute", value_id],
    tag_segments = [["attribute", value_id], ["attribute-values"]],
    ttl_secs = 60
)]
async fn load_attribute_value(
    cache: &HydraCache,
    calls: Arc<AtomicUsize>,
    value_id: u64,
) -> Result<ExpensiveValue, LoadError> {
    calls.fetch_add(1, Ordering::SeqCst);
    Ok(ExpensiveValue { id: value_id })
}

#[tokio::test]
async fn cacheable_macro_caches_loader_result() {
    let cache = HydraCache::local().build();
    let calls = Arc::new(AtomicUsize::new(0));

    let first: ExpensiveValue = cacheable_loader!(
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

    let second: ExpensiveValue = cacheable_loader!(
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

    let first: ExpensiveValue = cacheable_loader!(
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

    let second: ExpensiveValue = cacheable_loader!(
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

#[tokio::test]
async fn cacheable_macro_accepts_tags_expression() {
    let cache = HydraCache::local().build();
    let calls = Arc::new(AtomicUsize::new(0));

    let first: ExpensiveValue = cacheable_loader!(
        cache = cache,
        key = "tag-list:1",
        tags = ["tag-list", "tag-list:1"],
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

    let second: ExpensiveValue = cacheable_loader!(
        cache = cache,
        key = "tag-list:1",
        tags = ["tag-list", "tag-list:1"],
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

    cache.invalidate_tag("tag-list").await.unwrap();
    assert!(!cache.contains_key("tag-list:1").await);
}

#[tokio::test]
async fn cacheable_macro_accepts_tag_set_expression() {
    let cache = HydraCache::local().build();

    let value: ExpensiveValue = cacheable_loader!(
        cache = cache,
        key = "tag-set:1",
        tags = TagSet::new().tag("tag-set").entity("value", 1),
        load = || async { Ok::<_, LoadError>(ExpensiveValue { id: 1 }) },
    )
    .await
    .unwrap();

    assert_eq!(value, ExpensiveValue { id: 1 });

    cache.invalidate_tag("value:1").await.unwrap();
    assert!(!cache.contains_key("tag-set:1").await);
}

#[tokio::test]
async fn cacheable_attribute_caches_function_result() {
    let cache = HydraCache::local().build();
    let calls = Arc::new(AtomicUsize::new(0));

    let first = load_attribute_value(&cache, Arc::clone(&calls), 7)
        .await
        .unwrap();
    let second = load_attribute_value(&cache, Arc::clone(&calls), 7)
        .await
        .unwrap();

    assert_eq!(first, ExpensiveValue { id: 7 });
    assert_eq!(second, ExpensiveValue { id: 7 });
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    cache.invalidate_tag("attribute:7").await.unwrap();
    assert!(!cache.contains_key("attribute:7").await);
}

#[tokio::test]
async fn cacheable_infallible_macro_caches_loader_result() {
    let cache = HydraCache::local().build();
    let calls = Arc::new(AtomicUsize::new(0));

    let first: ExpensiveValue = cacheable_infallible!(
        cache = cache,
        key = "infallible:1",
        tags = ["infallible", "infallible:1"],
        ttl_secs = 60,
        load = {
            let calls = Arc::clone(&calls);
            move || async move {
                calls.fetch_add(1, Ordering::SeqCst);
                ExpensiveValue { id: 1 }
            }
        },
    )
    .await
    .unwrap();

    let second: ExpensiveValue = cacheable_infallible!(
        cache = cache,
        key = "infallible:1",
        tags = ["infallible", "infallible:1"],
        ttl_secs = 60,
        load = {
            let calls = Arc::clone(&calls);
            move || async move {
                calls.fetch_add(1, Ordering::SeqCst);
                ExpensiveValue { id: 2 }
            }
        },
    )
    .await
    .unwrap();

    assert_eq!(first, ExpensiveValue { id: 1 });
    assert_eq!(second, ExpensiveValue { id: 1 });
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
