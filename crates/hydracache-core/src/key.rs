use std::borrow::Cow;
use std::fmt;

/// A logical cache key.
///
/// v0 treats keys as application-provided strings. Query adapters may later derive
/// these keys from SQL text and typed arguments.
///
/// # Example
///
/// ```rust
/// use hydracache_core::CacheKey;
///
/// let key = CacheKey::new("users:42");
/// assert_eq!(key.as_str(), "users:42");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey<'a>(Cow<'a, str>);

impl<'a> CacheKey<'a> {
    /// Create a new cache key.
    pub fn new(value: impl Into<Cow<'a, str>>) -> Self {
        Self(value.into())
    }

    /// Start building an owned cache key from escaped segments.
    ///
    /// # Example
    ///
    /// ```rust
    /// use hydracache_core::CacheKey;
    ///
    /// let key = CacheKey::builder()
    ///     .segment("tenant:7")
    ///     .segment("users")
    ///     .segment(42)
    ///     .build();
    ///
    /// assert_eq!(key.as_str(), "tenant%3A7:users:42");
    /// ```
    pub fn builder() -> CacheKeyBuilder {
        CacheKeyBuilder::new()
    }

    /// Return the string representation of the key.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert this key into an owned key.
    pub fn into_owned(self) -> CacheKey<'static> {
        CacheKey(Cow::Owned(self.0.into_owned()))
    }
}

/// Builder for cache keys made of escaped `:`-separated segments.
///
/// `segment` escapes `:` and `%`, which keeps a single logical segment from
/// being confused with multiple key segments.
///
/// # Example
///
/// ```rust
/// use hydracache_core::CacheKeyBuilder;
///
/// let key = CacheKeyBuilder::new()
///     .segment("tenant")
///     .segment(7)
///     .entity("user", 42)
///     .build_string();
///
/// assert_eq!(key, "tenant:7:user:42");
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CacheKeyBuilder {
    segments: Vec<String>,
}

impl CacheKeyBuilder {
    /// Create an empty key builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a key builder with one initial segment.
    pub fn from_segment(segment: impl ToString) -> Self {
        Self::new().segment(segment)
    }

    /// Append one escaped key segment.
    pub fn segment(mut self, segment: impl ToString) -> Self {
        self.segments.push(escape_segment(&segment.to_string()));
        self
    }

    /// Append multiple escaped key segments.
    pub fn segments<I, S>(mut self, segments: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: ToString,
    {
        self.segments.extend(
            segments
                .into_iter()
                .map(|segment| escape_segment(&segment.to_string())),
        );
        self
    }

    /// Append an escaped entity kind and id pair.
    pub fn entity(self, kind: impl ToString, id: impl ToString) -> Self {
        self.segment(kind).segment(id)
    }

    /// Append a `tenant:{id}` prefix.
    pub fn tenant(self, id: impl ToString) -> Self {
        self.segment("tenant").segment(id)
    }

    /// Return whether no segments have been added.
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    /// Build an owned [`CacheKey`].
    pub fn build(self) -> CacheKey<'static> {
        CacheKey::new(self.build_string())
    }

    /// Build an owned key string.
    pub fn build_string(self) -> String {
        self.segments.join(":")
    }
}

impl<'a> From<&'a str> for CacheKey<'a> {
    fn from(value: &'a str) -> Self {
        Self::new(value)
    }
}

impl From<String> for CacheKey<'static> {
    fn from(value: String) -> Self {
        Self::new(Cow::Owned(value))
    }
}

impl fmt::Display for CacheKey<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

pub(crate) fn escape_segment(segment: &str) -> String {
    let mut escaped = String::with_capacity(segment.len());
    for ch in segment.chars() {
        match ch {
            '%' => escaped.push_str("%25"),
            ':' => escaped.push_str("%3A"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_builder_new_is_empty() {
        let builder = CacheKeyBuilder::new();

        assert!(builder.is_empty());
        assert_eq!(builder.clone().build_string(), "");
        assert_eq!(builder.build().as_str(), "");
    }

    #[test]
    fn key_builder_from_segment_adds_initial_segment() {
        let key = CacheKeyBuilder::from_segment("users").build_string();

        assert_eq!(key, "users");
    }

    #[test]
    fn key_builder_segment_escapes_colon_and_percent() {
        let key = CacheKeyBuilder::new()
            .segment("tenant:7")
            .segment("percent%value")
            .build_string();

        assert_eq!(key, "tenant%3A7:percent%25value");
    }

    #[test]
    fn key_builder_segments_preserve_order() {
        let key = CacheKeyBuilder::new()
            .segments(["tenant", "7", "users"])
            .build_string();

        assert_eq!(key, "tenant:7:users");
    }

    #[test]
    fn key_builder_entity_and_tenant_append_pairs() {
        let key = CacheKeyBuilder::new()
            .tenant(7)
            .entity("user", 42)
            .build_string();

        assert_eq!(key, "tenant:7:user:42");
    }

    #[test]
    fn cache_key_builder_constructor_matches_direct_builder() {
        let key = CacheKey::builder().entity("user", 42).build();

        assert_eq!(key.as_str(), "user:42");
        assert_eq!(key.to_string(), "user:42");
    }

    #[test]
    fn cache_key_conversions_preserve_owned_and_borrowed_values() {
        let borrowed = CacheKey::from("users:1");
        let owned = CacheKey::from(String::from("users:2"));
        let promoted = borrowed.clone().into_owned();

        assert_eq!(borrowed.as_str(), "users:1");
        assert_eq!(owned.as_str(), "users:2");
        assert_eq!(promoted.as_str(), "users:1");
    }
}
