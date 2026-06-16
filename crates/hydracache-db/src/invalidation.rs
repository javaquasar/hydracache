use std::collections::BTreeSet;

use hydracache::HydraCache;
use hydracache_core::CacheCodec;

use crate::CacheEntity;

/// A database-neutral list of cache invalidations staged by repository code.
///
/// `InvalidationPlan` deliberately does not know about SQLx, Diesel, SeaORM, or
/// any transaction type. Build it while preparing a write, execute the database
/// transaction in the ORM/client you already use, and call [`execute`] only
/// after commit succeeds. Dropping the plan on rollback leaves cached values
/// untouched.
///
/// [`execute`]: InvalidationPlan::execute
///
/// # Example
///
/// ```rust
/// use hydracache::HydraCache;
/// use hydracache_db::{HydraCacheEntity, InvalidationPlan};
///
/// #[derive(HydraCacheEntity)]
/// #[hydracache(entity = "user", collection = "users")]
/// struct User {
///     #[hydracache(id)]
///     id: i64,
/// }
///
/// # async fn example(cache: HydraCache) -> hydracache::CacheResult<()> {
/// let pending = InvalidationPlan::new().cache_entity::<User>(42);
///
/// // tx.update_user(42).await?;
/// // tx.commit().await?;
///
/// let report = pending.execute(&cache).await?;
/// assert_eq!(report.tag_count, 2);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InvalidationPlan {
    keys: BTreeSet<String>,
    tags: BTreeSet<String>,
}

impl InvalidationPlan {
    /// Create an empty staged invalidation plan.
    pub fn new() -> Self {
        Self::default()
    }

    /// Stage one physical cache key for removal.
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.keys.insert(key.into());
        self
    }

    /// Stage several physical cache keys for removal.
    pub fn keys<I, S>(mut self, keys: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.keys.extend(keys.into_iter().map(Into::into));
        self
    }

    /// Stage one invalidation tag.
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.insert(tag.into());
        self
    }

    /// Stage several invalidation tags.
    pub fn tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.tags.extend(tags.into_iter().map(Into::into));
        self
    }

    /// Stage the entity tag for a [`CacheEntity`] id.
    pub fn entity<E>(mut self, id: E::Id) -> Self
    where
        E: CacheEntity,
    {
        self.tags.insert(E::entity_tag_for(&id));
        self
    }

    /// Stage the collection tag for a [`CacheEntity`], if it has one.
    pub fn collection<E>(mut self) -> Self
    where
        E: CacheEntity,
    {
        if let Some(tag) = E::collection_tag() {
            self.tags.insert(tag);
        }
        self
    }

    /// Stage both entity and collection tags for a [`CacheEntity`] id.
    pub fn cache_entity<E>(self, id: E::Id) -> Self
    where
        E: CacheEntity,
    {
        self.entity::<E>(id).collection::<E>()
    }

    /// Return true when no key or tag invalidations have been staged.
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty() && self.tags.is_empty()
    }

    /// Number of staged key invalidations after de-duplication.
    pub fn key_count(&self) -> usize {
        self.keys.len()
    }

    /// Number of staged tag invalidations after de-duplication.
    pub fn tag_count(&self) -> usize {
        self.tags.len()
    }

    /// Staged keys in deterministic order.
    pub fn key_values(&self) -> impl Iterator<Item = &str> {
        self.keys.iter().map(String::as_str)
    }

    /// Staged tags in deterministic order.
    pub fn tag_values(&self) -> impl Iterator<Item = &str> {
        self.tags.iter().map(String::as_str)
    }

    /// Execute all staged invalidations against the local cache after commit.
    pub async fn execute<C>(
        self,
        cache: &HydraCache<C>,
    ) -> hydracache::CacheResult<InvalidationReport>
    where
        C: CacheCodec,
    {
        let key_count = self.keys.len();
        let tag_count = self.tags.len();
        let mut keys_removed = 0;
        let mut tags_removed = 0;

        for key in self.keys {
            if cache.remove(&key).await? {
                keys_removed += 1;
            }
        }

        for tag in self.tags {
            tags_removed += cache.invalidate_tag(&tag).await?;
        }

        Ok(InvalidationReport {
            key_count,
            tag_count,
            keys_removed,
            tags_removed,
        })
    }
}

/// Result of executing a staged [`InvalidationPlan`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InvalidationReport {
    /// Number of distinct staged keys.
    pub key_count: usize,
    /// Number of distinct staged tags.
    pub tag_count: usize,
    /// Number of key removals that found an entry.
    pub keys_removed: u64,
    /// Number of entries removed by tag invalidations.
    pub tags_removed: u64,
}

impl InvalidationReport {
    /// Total entries removed by key and tag invalidation.
    pub fn removed_entries(self) -> u64 {
        self.keys_removed + self.tags_removed
    }
}
