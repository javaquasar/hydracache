use sha2::{Digest, Sha256};

use hydracache::{CacheInvalidation, CacheKeyBuilder};

/// Stable SHA-256 hash of a normalized invalidation target.
pub type InvalidationTargetHash = [u8; 32];

/// Normalized, transport-neutral invalidation intent.
///
/// The intent deliberately carries no cached value. It is suitable for durable
/// database outboxes, trigger-written rows, and transport wake-ups.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InvalidationIntent {
    /// Invalidate one physical cache key.
    Key {
        /// Physical cache key.
        key: String,
    },
    /// Invalidate all entries associated with one tag.
    Tag {
        /// Invalidation tag.
        tag: String,
    },
    /// Invalidate the entity tag built from an entity kind and id/key.
    Entity {
        /// Entity name/kind.
        entity: String,
        /// Entity id/key segment.
        key: String,
    },
    /// Invalidate a collection tag.
    Collection {
        /// Collection name.
        collection: String,
    },
    /// Flush the whole cache.
    Flush,
}

impl InvalidationIntent {
    /// Create a key invalidation intent.
    pub fn key(key: impl Into<String>) -> Self {
        Self::Key { key: key.into() }
    }

    /// Create a tag invalidation intent.
    pub fn tag(tag: impl Into<String>) -> Self {
        Self::Tag { tag: tag.into() }
    }

    /// Create an entity invalidation intent.
    pub fn entity(entity: impl Into<String>, key: impl Into<String>) -> Self {
        Self::Entity {
            entity: entity.into(),
            key: key.into(),
        }
    }

    /// Create a collection invalidation intent.
    pub fn collection(collection: impl Into<String>) -> Self {
        Self::Collection {
            collection: collection.into(),
        }
    }

    /// Create a cache-wide flush intent.
    pub fn flush() -> Self {
        Self::Flush
    }

    /// Return the stable wire/storage kind for this intent.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Key { .. } => "key",
            Self::Tag { .. } => "tag",
            Self::Entity { .. } => "entity",
            Self::Collection { .. } => "collection",
            Self::Flush => "flush",
        }
    }

    /// Return the primary cache key/tag value stored in the outbox row.
    pub fn value(&self) -> Option<&str> {
        match self {
            Self::Key { key } | Self::Entity { key, .. } => Some(key),
            Self::Tag { tag } => Some(tag),
            Self::Collection { collection } => Some(collection),
            Self::Flush => None,
        }
    }

    /// Stable content hash used in the outbox idempotency key.
    ///
    /// The hash input is length-prefixed and includes the intent kind, so values
    /// containing `:`, `/`, whitespace, or empty strings cannot collide through
    /// delimiter ambiguity.
    pub fn target_hash(&self) -> InvalidationTargetHash {
        let mut hasher = Sha256::new();
        write_hash_part(&mut hasher, b"hydracache-invalidation-intent-v1");
        write_hash_part(&mut hasher, self.kind().as_bytes());

        match self {
            Self::Key { key } => write_hash_part(&mut hasher, key.as_bytes()),
            Self::Tag { tag } => write_hash_part(&mut hasher, tag.as_bytes()),
            Self::Entity { entity, key } => {
                write_hash_part(&mut hasher, entity.as_bytes());
                write_hash_part(&mut hasher, key.as_bytes());
            }
            Self::Collection { collection } => {
                write_hash_part(&mut hasher, collection.as_bytes());
            }
            Self::Flush => {}
        }

        hasher.finalize().into()
    }

    /// Hex representation of [`InvalidationIntent::target_hash`].
    pub fn target_hash_hex(&self) -> String {
        hex_encode(&self.target_hash())
    }

    /// Map this intent onto HydraCache's existing cross-process invalidation
    /// operation.
    pub fn to_cache_invalidation(&self) -> CacheInvalidation {
        match self {
            Self::Key { key } => CacheInvalidation::key(key.clone()),
            Self::Tag { tag } => CacheInvalidation::tag(tag.clone()),
            Self::Entity { entity, key } => CacheInvalidation::tag(entity_tag(entity, key)),
            Self::Collection { collection } => CacheInvalidation::tag(collection_tag(collection)),
            Self::Flush => CacheInvalidation::flush(),
        }
    }
}

/// Ordered batch of invalidation intent rows to persist with a data write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidationIntentBatch {
    reason: String,
    intents: Vec<InvalidationIntent>,
}

impl InvalidationIntentBatch {
    /// Create an empty batch with an operator-facing reason.
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
            intents: Vec::new(),
        }
    }

    /// Return the reason attached to every outbox row in this batch.
    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// Return the intents in insertion order.
    pub fn intents(&self) -> &[InvalidationIntent] {
        &self.intents
    }

    /// Return whether no intents have been added.
    pub fn is_empty(&self) -> bool {
        self.intents.is_empty()
    }

    /// Return the number of intents in this batch.
    pub fn len(&self) -> usize {
        self.intents.len()
    }

    /// Add an already-built intent.
    pub fn intent(mut self, intent: InvalidationIntent) -> Self {
        self.intents.push(intent);
        self
    }

    /// Add a key invalidation.
    pub fn invalidate_key(self, key: impl Into<String>) -> Self {
        self.intent(InvalidationIntent::key(key))
    }

    /// Add a tag invalidation.
    pub fn invalidate_tag(self, tag: impl Into<String>) -> Self {
        self.intent(InvalidationIntent::tag(tag))
    }

    /// Add an entity invalidation.
    pub fn invalidate_entity(self, entity: impl Into<String>, key: impl Into<String>) -> Self {
        self.intent(InvalidationIntent::entity(entity, key))
    }

    /// Add a collection invalidation.
    pub fn invalidate_collection(self, collection: impl Into<String>) -> Self {
        self.intent(InvalidationIntent::collection(collection))
    }

    /// Add a cache-wide flush invalidation.
    pub fn flush(self) -> Self {
        self.intent(InvalidationIntent::flush())
    }
}

impl Default for InvalidationIntentBatch {
    fn default() -> Self {
        Self::new("")
    }
}

/// Identity of a committed write used to build outbox idempotency keys.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommitPosition(String);

impl CommitPosition {
    /// Create a commit position from a database txid, LSN, or monotonic fallback.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Return the string representation persisted in the outbox table.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume this value into its owned string.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for CommitPosition {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for CommitPosition {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

fn write_hash_part(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

fn entity_tag(entity: &str, key: &str) -> String {
    CacheKeyBuilder::new()
        .segment(entity)
        .segment(key)
        .build_string()
}

fn collection_tag(collection: &str) -> String {
    CacheKeyBuilder::from_segment(collection).build_string()
}

fn hex_encode(bytes: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{CommitPosition, InvalidationIntent, InvalidationIntentBatch};

    #[test]
    fn intent_target_hash_is_stable() {
        let first = InvalidationIntent::entity("user", "42").target_hash();
        let second = InvalidationIntent::entity("user", "42").target_hash();

        assert_eq!(first, second);
        assert_eq!(
            InvalidationIntent::entity("user", "42")
                .target_hash_hex()
                .len(),
            64
        );
    }

    #[test]
    fn intent_target_hash_distinguishes_kind_and_length_prefixed_parts() {
        let key = InvalidationIntent::key("tenant:7/users");
        let tag = InvalidationIntent::tag("tenant:7/users");
        let entity = InvalidationIntent::entity("tenant:7", "users");
        let collection = InvalidationIntent::collection("tenant:7:users");

        assert_ne!(key.target_hash(), tag.target_hash());
        assert_ne!(key.target_hash(), entity.target_hash());
        assert_ne!(tag.target_hash(), collection.target_hash());
    }

    #[test]
    fn intent_to_cache_invalidation_maps_each_kind() {
        let key = InvalidationIntent::key("db:user:42").to_cache_invalidation();
        assert_eq!(key.key_value(), Some("db:user:42"));

        let tag = InvalidationIntent::tag("users").to_cache_invalidation();
        assert_eq!(tag.tag_value(), Some("users"));

        let entity = InvalidationIntent::entity("account:user", "42%beta").to_cache_invalidation();
        assert_eq!(entity.tag_value(), Some("account%3Auser:42%25beta"));

        let collection = InvalidationIntent::collection("users:active").to_cache_invalidation();
        assert_eq!(collection.tag_value(), Some("users%3Aactive"));

        assert!(InvalidationIntent::flush()
            .to_cache_invalidation()
            .is_flush());
    }

    #[test]
    fn intent_batch_preserves_reason_and_order() {
        let batch = InvalidationIntentBatch::new("user-write")
            .invalidate_key("db:user:42")
            .invalidate_tag("users")
            .invalidate_entity("user", "42")
            .invalidate_collection("users:active")
            .flush();

        assert_eq!(batch.reason(), "user-write");
        assert_eq!(batch.len(), 5);
        assert_eq!(batch.intents()[0].kind(), "key");
        assert_eq!(batch.intents()[4].kind(), "flush");
    }

    #[test]
    fn commit_position_wraps_database_identity() {
        let position = CommitPosition::new("pg:123");

        assert_eq!(position.as_str(), "pg:123");
        assert_eq!(position.clone().into_string(), "pg:123");
        assert_eq!(CommitPosition::from("pg:123"), position);
    }
}
