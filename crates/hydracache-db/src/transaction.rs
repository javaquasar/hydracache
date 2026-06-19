use hydracache::{CacheInvalidation, HydraCache};
use hydracache_core::CacheCodec;

use crate::{CacheEntity, InvalidationIntent, InvalidationIntentBatch};

/// Mutable invalidation collector passed to transaction companion closures.
///
/// The collector is intentionally small and database-neutral. Repository code
/// records the cache targets made stale by a write while keeping ownership of
/// SQL/ORM execution inside the transaction closure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidationCollector {
    namespace: String,
    reason: String,
    intents: Vec<InvalidationIntent>,
}

impl InvalidationCollector {
    /// Create an empty collector for one cache namespace and operator-facing reason.
    pub fn new(namespace: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            reason: reason.into(),
            intents: Vec::new(),
        }
    }

    /// Return the namespace that should receive the collected invalidations.
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Return the reason attached to durable outbox rows.
    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// Return collected intents in insertion order.
    pub fn intents(&self) -> &[InvalidationIntent] {
        &self.intents
    }

    /// Return true when no invalidations have been collected.
    pub fn is_empty(&self) -> bool {
        self.intents.is_empty()
    }

    /// Return the number of collected intents.
    pub fn len(&self) -> usize {
        self.intents.len()
    }

    /// Add one already-normalized invalidation intent.
    pub fn intent(&mut self, intent: InvalidationIntent) -> &mut Self {
        self.intents.push(intent);
        self
    }

    /// Add one physical cache-key invalidation.
    pub fn invalidate_key(&mut self, key: impl Into<String>) -> &mut Self {
        self.intent(InvalidationIntent::key(key))
    }

    /// Add one tag invalidation.
    pub fn invalidate_tag(&mut self, tag: impl Into<String>) -> &mut Self {
        self.intent(InvalidationIntent::tag(tag))
    }

    /// Add one entity-tag invalidation.
    pub fn invalidate_entity(
        &mut self,
        entity: impl Into<String>,
        key: impl Into<String>,
    ) -> &mut Self {
        self.intent(InvalidationIntent::entity(entity, key))
    }

    /// Add one collection-tag invalidation.
    pub fn invalidate_collection(&mut self, collection: impl Into<String>) -> &mut Self {
        self.intent(InvalidationIntent::collection(collection))
    }

    /// Add a cache-wide flush invalidation.
    pub fn flush(&mut self) -> &mut Self {
        self.intent(InvalidationIntent::flush())
    }

    /// Add both entity and collection invalidations for a cache entity id.
    pub fn cache_entity<E>(&mut self, id: E::Id) -> &mut Self
    where
        E: CacheEntity,
    {
        let id = id.to_string();
        self.invalidate_entity(E::ENTITY, id);
        if let Some(collection) = E::collection_tag() {
            self.invalidate_collection(collection);
        }
        self
    }

    /// Finish collection and return an immutable invalidation payload.
    pub fn into_collected(self) -> CollectedInvalidations {
        let mut batch = InvalidationIntentBatch::new(self.reason);
        for intent in self.intents {
            batch = batch.intent(intent);
        }

        CollectedInvalidations {
            namespace: self.namespace,
            batch,
        }
    }
}

impl Default for InvalidationCollector {
    fn default() -> Self {
        Self::new("db", "")
    }
}

/// Immutable invalidation payload produced by an [`InvalidationCollector`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectedInvalidations {
    namespace: String,
    batch: InvalidationIntentBatch,
}

impl CollectedInvalidations {
    /// Return the namespace that should receive the invalidation batch.
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Return the durable outbox batch.
    pub fn batch(&self) -> &InvalidationIntentBatch {
        &self.batch
    }

    /// Consume into the durable outbox batch.
    pub fn into_batch(self) -> InvalidationIntentBatch {
        self.batch
    }

    /// Return true when the batch has no invalidation intents.
    pub fn is_empty(&self) -> bool {
        self.batch.is_empty()
    }

    /// Return the number of invalidation intents.
    pub fn len(&self) -> usize {
        self.batch.len()
    }

    /// Apply collected invalidations directly to the local cache.
    ///
    /// This is useful for non-durable single-process examples. Production
    /// transaction flows should prefer a durable outbox so invalidation intent is
    /// committed atomically with the database write.
    pub async fn execute_local<C>(
        self,
        cache: &HydraCache<C>,
    ) -> hydracache::CacheResult<CollectedInvalidationReport>
    where
        C: CacheCodec,
    {
        let mut report = CollectedInvalidationReport {
            intent_count: self.batch.len(),
            ..CollectedInvalidationReport::default()
        };

        for intent in self.batch.intents() {
            match intent.to_cache_invalidation() {
                CacheInvalidation::Key { key } => {
                    if cache.remove(&key).await? {
                        report.keys_removed += 1;
                    }
                }
                CacheInvalidation::Tag { tag } => {
                    report.tags_removed += cache.invalidate_tag(&tag).await?;
                }
                CacheInvalidation::Flush => {
                    cache.flush().await?;
                    report.flushed = true;
                }
            }
        }

        Ok(report)
    }
}

/// Result of applying collected invalidations directly to a local cache.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CollectedInvalidationReport {
    /// Number of collected invalidation intents.
    pub intent_count: usize,
    /// Number of physical keys removed.
    pub keys_removed: u64,
    /// Number of entries removed through tags.
    pub tags_removed: u64,
    /// Whether a flush intent was applied.
    pub flushed: bool,
}

#[cfg(test)]
mod tests {
    use hydracache::HydraCache;

    use super::*;

    struct User;

    impl CacheEntity for User {
        type Id = i64;

        const ENTITY: &'static str = "user";
        const COLLECTION: Option<&'static str> = Some("users");
    }

    #[test]
    fn collector_preserves_namespace_reason_and_ordered_intents() {
        let mut collector = InvalidationCollector::new("tenant-a", "user-write");

        collector
            .invalidate_key("physical:user:42")
            .invalidate_tag("tenant:7")
            .cache_entity::<User>(42);

        let collected = collector.into_collected();
        assert_eq!(collected.namespace(), "tenant-a");
        assert_eq!(collected.batch().reason(), "user-write");
        assert_eq!(collected.len(), 4);
        assert_eq!(
            collected.batch().intents()[0],
            InvalidationIntent::key("physical:user:42")
        );
        assert_eq!(
            collected.batch().intents()[1],
            InvalidationIntent::tag("tenant:7")
        );
        assert_eq!(
            collected.batch().intents()[2],
            InvalidationIntent::entity("user", "42")
        );
        assert_eq!(
            collected.batch().intents()[3],
            InvalidationIntent::collection("users")
        );
    }

    #[tokio::test]
    async fn collected_invalidations_can_apply_directly_to_local_cache() {
        let cache = HydraCache::local().build();
        cache
            .get_or_insert_with(
                "user:42",
                hydracache::CacheOptions::new().tags(["users", "user:42"]),
                || async { "Ada".to_owned() },
            )
            .await
            .unwrap();

        let mut collector = InvalidationCollector::new("db", "direct");
        collector.cache_entity::<User>(42);

        let report = collector
            .into_collected()
            .execute_local(&cache)
            .await
            .unwrap();

        assert_eq!(report.intent_count, 2);
        assert_eq!(report.tags_removed, 1);
        assert!(!report.flushed);
        assert_eq!(cache.get::<String>("user:42").await.unwrap(), None);
    }
}
