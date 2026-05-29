use std::time::Duration;

use crate::TagSet;

/// Per-entry cache behavior.
///
/// Options are passed to `put`, `get_or_load`, and loader helper methods.
///
/// # Example
///
/// ```rust
/// use std::time::Duration;
///
/// use hydracache_core::CacheOptions;
///
/// let options = CacheOptions::new()
///     .ttl(Duration::from_secs(60))
///     .tags(["users", "user:42"]);
///
/// assert_eq!(options.ttl_value(), Some(Duration::from_secs(60)));
/// assert_eq!(options.tags_value(), &["users".to_owned(), "user:42".to_owned()]);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CacheOptions {
    ttl: Option<Duration>,
    tags: Vec<String>,
}

impl CacheOptions {
    /// Create empty cache options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a per-entry TTL.
    pub fn ttl(mut self, ttl: Duration) -> Self {
        self.ttl = Some(ttl);
        self
    }

    /// Attach one tag used by `invalidate_tag`.
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Attach tags used by `invalidate_tag`.
    pub fn tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.tags = tags.into_iter().map(Into::into).collect();
        self
    }

    /// Replace tags from a [`TagSet`].
    pub fn tag_set(mut self, tags: TagSet) -> Self {
        self.tags = tags.into_vec();
        self
    }

    /// Return the configured TTL, if any.
    pub fn ttl_value(&self) -> Option<Duration> {
        self.ttl
    }

    /// Return tags attached to this entry.
    pub fn tags_value(&self) -> &[String] {
        &self.tags
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TagSet;

    #[test]
    fn cache_options_tag_set_replaces_existing_tags() {
        let options = CacheOptions::new()
            .tag("old")
            .tag_set(TagSet::new().tag("new").entity("user", 42));

        assert_eq!(
            options.tags_value(),
            &["new".to_owned(), "user:42".to_owned()]
        );
    }
}
