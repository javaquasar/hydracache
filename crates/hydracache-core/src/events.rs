use std::time::SystemTime;

use crate::CacheKey;

/// Kind of cache event emitted by a HydraCache runtime.
///
/// Access and loader events are intentionally separate from mutation events so
/// applications can keep high-volume hit/miss reporting disabled until needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CacheEventKind {
    /// A lookup returned a cached value.
    Hit,
    /// A lookup did not find a usable cached value.
    Miss,
    /// A caller joined an already running single-flight load.
    SingleFlightJoined,
    /// The cache owner started a loader for a missing key.
    LoadStarted,
    /// A loader completed and its result was accepted into the cache.
    LoadCompleted,
    /// A loader returned an error or failed to encode the loaded value.
    LoadFailed,
    /// A value was stored.
    Stored,
    /// A key was explicitly removed.
    Removed,
    /// A key was explicitly invalidated.
    KeyInvalidated,
    /// A tag was invalidated.
    TagInvalidated,
    /// The cache was flushed.
    Flushed,
    /// A loader result was discarded because an invalidation made it stale.
    StaleLoadDiscarded,
    /// An entry expired and was removed during cache access.
    Expired,
    /// An entry was evicted by the backend.
    Evicted,
}

impl CacheEventKind {
    /// Return whether this event belongs to the high-volume access/load group.
    pub fn is_access(self) -> bool {
        matches!(
            self,
            Self::Hit
                | Self::Miss
                | Self::SingleFlightJoined
                | Self::LoadStarted
                | Self::LoadCompleted
                | Self::LoadFailed
        )
    }

    /// Return whether this event describes a cache mutation or invalidation.
    pub fn is_mutation(self) -> bool {
        !self.is_access()
    }
}

/// Logical scope affected by a cache event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheEventScope {
    /// Event for one cache key.
    Key {
        /// Physical cache key.
        key: CacheKey<'static>,
    },
    /// Event for one invalidation tag.
    Tag {
        /// Invalidated tag.
        tag: String,
        /// Number of keys affected by the operation.
        affected_keys: u64,
    },
    /// Event for the whole cache.
    Cache {
        /// Approximate number of keys affected, when known.
        affected_keys: Option<u64>,
    },
}

impl CacheEventScope {
    /// Return the event key when this is a key-scoped event.
    pub fn key(&self) -> Option<&str> {
        match self {
            Self::Key { key } => Some(key.as_str()),
            Self::Tag { .. } | Self::Cache { .. } => None,
        }
    }

    /// Return the event tag when this is a tag-scoped event.
    pub fn tag(&self) -> Option<&str> {
        match self {
            Self::Tag { tag, .. } => Some(tag),
            Self::Key { .. } | Self::Cache { .. } => None,
        }
    }

    /// Return the affected-key count when the scope carries one.
    pub fn affected_keys(&self) -> Option<u64> {
        match self {
            Self::Key { .. } => Some(1),
            Self::Tag { affected_keys, .. } => Some(*affected_keys),
            Self::Cache { affected_keys } => *affected_keys,
        }
    }
}

/// Origin of a cache event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CacheEventOrigin {
    /// Event was caused by a direct local API call.
    LocalApi,
    /// Event was caused by a loader owner.
    Loader,
    /// Event was caused by single-flight coordination.
    SingleFlight,
    /// Event was caused by backend expiration or eviction.
    Backend,
    /// Event was received from a future distributed bus.
    DistributedBus,
}

/// Value payload mode requested by event subscribers.
///
/// The first implementation emits metadata-only events. The enum exists so the
/// public filter shape can grow toward encoded-value delivery without changing
/// subscription options later.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum CacheEventValueMode {
    /// Do not include cached values in events.
    #[default]
    MetadataOnly,
    /// Reserve space for a future encoded-value event mode.
    EncodedBytes,
}

/// Metadata-only cache event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheEvent {
    kind: CacheEventKind,
    scope: CacheEventScope,
    origin: CacheEventOrigin,
    tags: Vec<String>,
    timestamp: SystemTime,
}

impl CacheEvent {
    /// Create a key-scoped event.
    pub fn for_key<I, S>(
        kind: CacheEventKind,
        key: impl Into<String>,
        origin: CacheEventOrigin,
        tags: I,
    ) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            kind,
            scope: CacheEventScope::Key {
                key: CacheKey::from(key.into()),
            },
            origin,
            tags: tags.into_iter().map(Into::into).collect(),
            timestamp: SystemTime::now(),
        }
    }

    /// Create a tag-scoped event.
    pub fn for_tag(
        kind: CacheEventKind,
        tag: impl Into<String>,
        affected_keys: u64,
        origin: CacheEventOrigin,
    ) -> Self {
        let tag = tag.into();
        Self {
            kind,
            scope: CacheEventScope::Tag {
                tag: tag.clone(),
                affected_keys,
            },
            origin,
            tags: vec![tag],
            timestamp: SystemTime::now(),
        }
    }

    /// Create a cache-wide event.
    pub fn for_cache(
        kind: CacheEventKind,
        affected_keys: Option<u64>,
        origin: CacheEventOrigin,
    ) -> Self {
        Self {
            kind,
            scope: CacheEventScope::Cache { affected_keys },
            origin,
            tags: Vec::new(),
            timestamp: SystemTime::now(),
        }
    }

    /// Return the event kind.
    pub fn kind(&self) -> CacheEventKind {
        self.kind
    }

    /// Return the event scope.
    pub fn scope(&self) -> &CacheEventScope {
        &self.scope
    }

    /// Return the event origin.
    pub fn origin(&self) -> CacheEventOrigin {
        self.origin
    }

    /// Return the event key when this is a key-scoped event.
    pub fn key(&self) -> Option<&str> {
        self.scope.key()
    }

    /// Return the event tag when this is a tag-scoped event.
    pub fn tag(&self) -> Option<&str> {
        self.scope.tag()
    }

    /// Return event tags associated with the key or invalidation.
    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    /// Return the affected-key count when known.
    pub fn affected_keys(&self) -> Option<u64> {
        self.scope.affected_keys()
    }

    /// Return the event timestamp.
    pub fn timestamp(&self) -> SystemTime {
        self.timestamp
    }
}

/// Subscription filters for cache events.
///
/// Filters are intentionally metadata-only. They are cheap enough to apply in
/// the subscriber wrapper and avoid coupling event publication to decoded
/// values.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CacheEventOptions {
    include_kinds: Option<Vec<CacheEventKind>>,
    exclude_kinds: Vec<CacheEventKind>,
    key: Option<String>,
    key_prefix: Option<String>,
    tag: Option<String>,
    origin: Option<CacheEventOrigin>,
    value_mode: CacheEventValueMode,
}

impl CacheEventOptions {
    /// Create event options that accept all published events.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create event options for mutation/invalidation events.
    pub fn mutations() -> Self {
        Self::new().include_kinds([
            CacheEventKind::Stored,
            CacheEventKind::Removed,
            CacheEventKind::KeyInvalidated,
            CacheEventKind::TagInvalidated,
            CacheEventKind::Flushed,
            CacheEventKind::StaleLoadDiscarded,
            CacheEventKind::Expired,
            CacheEventKind::Evicted,
        ])
    }

    /// Create event options for high-volume access/load events.
    pub fn access() -> Self {
        Self::new().include_kinds([
            CacheEventKind::Hit,
            CacheEventKind::Miss,
            CacheEventKind::SingleFlightJoined,
            CacheEventKind::LoadStarted,
            CacheEventKind::LoadCompleted,
            CacheEventKind::LoadFailed,
        ])
    }

    /// Include one event kind.
    pub fn include_kind(self, kind: CacheEventKind) -> Self {
        self.include_kinds([kind])
    }

    /// Include several event kinds.
    pub fn include_kinds<I>(mut self, kinds: I) -> Self
    where
        I: IntoIterator<Item = CacheEventKind>,
    {
        self.include_kinds
            .get_or_insert_with(Vec::new)
            .extend(kinds);
        self
    }

    /// Exclude one event kind.
    pub fn exclude_kind(self, kind: CacheEventKind) -> Self {
        self.exclude_kinds([kind])
    }

    /// Exclude several event kinds.
    pub fn exclude_kinds<I>(mut self, kinds: I) -> Self
    where
        I: IntoIterator<Item = CacheEventKind>,
    {
        self.exclude_kinds.extend(kinds);
        self
    }

    /// Restrict events to one exact key.
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Restrict events to key-scoped events whose key starts with the prefix.
    pub fn key_prefix(mut self, key_prefix: impl Into<String>) -> Self {
        self.key_prefix = Some(key_prefix.into());
        self
    }

    /// Restrict events to a tag.
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = Some(tag.into());
        self
    }

    /// Restrict events to one origin.
    pub fn origin(mut self, origin: CacheEventOrigin) -> Self {
        self.origin = Some(origin);
        self
    }

    /// Set the value mode requested by this subscription.
    pub fn value_mode(mut self, value_mode: CacheEventValueMode) -> Self {
        self.value_mode = value_mode;
        self
    }

    /// Return the requested value mode.
    pub fn value_mode_value(&self) -> CacheEventValueMode {
        self.value_mode
    }

    /// Return whether this event passes all filters.
    pub fn matches(&self, event: &CacheEvent) -> bool {
        if let Some(include_kinds) = &self.include_kinds {
            if !include_kinds.contains(&event.kind()) {
                return false;
            }
        }

        if self.exclude_kinds.contains(&event.kind()) {
            return false;
        }

        if let Some(key) = &self.key {
            if event.key() != Some(key.as_str()) {
                return false;
            }
        }

        if let Some(key_prefix) = &self.key_prefix {
            let Some(key) = event.key() else {
                return false;
            };
            if !key.starts_with(key_prefix) {
                return false;
            }
        }

        if let Some(tag) = &self.tag {
            let scope_matches = event.tag() == Some(tag.as_str());
            let tags_match = event.tags().iter().any(|event_tag| event_tag == tag);
            if !scope_matches && !tags_match {
                return false;
            }
        }

        if let Some(origin) = self.origin {
            if event.origin() != origin {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CacheEvent, CacheEventKind, CacheEventOptions, CacheEventOrigin, CacheEventScope,
        CacheEventValueMode,
    };

    #[test]
    fn event_kind_groups_access_and_mutation_events() {
        assert!(CacheEventKind::Hit.is_access());
        assert!(CacheEventKind::LoadFailed.is_access());
        assert!(!CacheEventKind::Stored.is_access());
        assert!(CacheEventKind::Stored.is_mutation());
    }

    #[test]
    fn event_constructors_capture_scope_tags_and_origin() {
        let key_event = CacheEvent::for_key(
            CacheEventKind::Stored,
            "user:42",
            CacheEventOrigin::LocalApi,
            ["users", "user:42"],
        );
        let tag_event = CacheEvent::for_tag(
            CacheEventKind::TagInvalidated,
            "users",
            3,
            CacheEventOrigin::LocalApi,
        );
        let cache_event = CacheEvent::for_cache(
            CacheEventKind::Flushed,
            Some(10),
            CacheEventOrigin::LocalApi,
        );

        assert_eq!(key_event.key(), Some("user:42"));
        assert_eq!(
            key_event.tags(),
            &["users".to_owned(), "user:42".to_owned()]
        );
        assert_eq!(key_event.origin(), CacheEventOrigin::LocalApi);
        assert_eq!(tag_event.tag(), Some("users"));
        assert_eq!(tag_event.affected_keys(), Some(3));
        assert_eq!(
            cache_event.scope(),
            &CacheEventScope::Cache {
                affected_keys: Some(10)
            }
        );
    }

    #[test]
    fn event_options_filter_by_kind_key_prefix_tag_and_origin() {
        let stored = CacheEvent::for_key(
            CacheEventKind::Stored,
            "users:42",
            CacheEventOrigin::LocalApi,
            ["users"],
        );
        let removed = CacheEvent::for_key(
            CacheEventKind::Removed,
            "orders:7",
            CacheEventOrigin::LocalApi,
            ["orders"],
        );

        let options = CacheEventOptions::new()
            .include_kind(CacheEventKind::Stored)
            .key_prefix("users:")
            .tag("users")
            .origin(CacheEventOrigin::LocalApi);

        assert!(options.matches(&stored));
        assert!(!options.matches(&removed));
        assert!(!options
            .clone()
            .exclude_kind(CacheEventKind::Stored)
            .matches(&stored));
    }

    #[test]
    fn event_options_mutations_and_access_presets_are_distinct() {
        let stored = CacheEvent::for_key(
            CacheEventKind::Stored,
            "k",
            CacheEventOrigin::LocalApi,
            ["t"],
        );
        let hit = CacheEvent::for_key(CacheEventKind::Hit, "k", CacheEventOrigin::LocalApi, ["t"]);

        assert!(CacheEventOptions::mutations().matches(&stored));
        assert!(!CacheEventOptions::mutations().matches(&hit));
        assert!(CacheEventOptions::access().matches(&hit));
        assert!(!CacheEventOptions::access().matches(&stored));
    }

    #[test]
    fn event_options_keep_requested_value_mode() {
        let options = CacheEventOptions::new().value_mode(CacheEventValueMode::EncodedBytes);

        assert_eq!(
            options.value_mode_value(),
            CacheEventValueMode::EncodedBytes
        );
    }
}
