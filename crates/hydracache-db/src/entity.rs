use hydracache::CacheKeyBuilder;

/// Static cache metadata for a domain entity.
///
/// Derive or implement this trait when you want domain-shaped cache calls
/// without repeating entity and collection names at every query site.
///
/// # Example
///
/// ```rust
/// use hydracache_db::{CacheEntity, HydraCacheEntity};
///
/// #[derive(HydraCacheEntity)]
/// #[hydracache(entity = "user", collection = "users", id = i64)]
/// struct User;
///
/// assert_eq!(User::cache_key_for(&42), "user:42");
/// assert_eq!(User::entity_tag_for(&42), "user:42");
/// assert_eq!(User::collection_tag(), Some("users".to_owned()));
/// ```
pub trait CacheEntity {
    /// Identifier type used to build entity keys and tags.
    type Id: ToString;

    /// Stable entity segment used in keys and entity tags.
    const ENTITY: &'static str;

    /// Optional collection tag for broader invalidation groups.
    const COLLECTION: Option<&'static str>;

    /// Build the logical cache key for this entity id.
    fn cache_key_for(id: &Self::Id) -> String {
        CacheKeyBuilder::new()
            .entity(Self::ENTITY, id.to_string())
            .build_string()
    }

    /// Build the entity invalidation tag for this id.
    fn entity_tag_for(id: &Self::Id) -> String {
        Self::cache_key_for(id)
    }

    /// Build the optional collection invalidation tag.
    fn collection_tag() -> Option<String> {
        Self::COLLECTION.map(|collection| CacheKeyBuilder::from_segment(collection).build_string())
    }
}
