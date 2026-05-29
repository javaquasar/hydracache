use crate::key::CacheKeyBuilder;

/// A reusable set of cache invalidation tags.
///
/// # Example
///
/// ```rust
/// use hydracache_core::{CacheOptions, TagSet};
///
/// let tags = TagSet::new()
///     .tag("users")
///     .entity("user", 42)
///     .tenant(7);
///
/// let options = CacheOptions::new().tag_set(tags);
/// assert_eq!(options.tags_value(), &["users".to_owned(), "user:42".to_owned(), "tenant:7".to_owned()]);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TagSet {
    tags: Vec<String>,
}

impl TagSet {
    /// Create an empty tag set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a tag set with one initial tag.
    pub fn from_tag(tag: impl Into<String>) -> Self {
        Self::new().tag(tag)
    }

    /// Add one tag.
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add multiple tags.
    pub fn tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.tags.extend(tags.into_iter().map(Into::into));
        self
    }

    /// Add an entity tag such as `user:42`.
    pub fn entity(self, kind: impl ToString, id: impl ToString) -> Self {
        self.tag(
            CacheKeyBuilder::new()
                .segment(kind)
                .segment(id)
                .build_string(),
        )
    }

    /// Add a tenant tag such as `tenant:7`.
    pub fn tenant(self, id: impl ToString) -> Self {
        self.entity("tenant", id)
    }

    /// Return whether the set is empty.
    pub fn is_empty(&self) -> bool {
        self.tags.is_empty()
    }

    /// Borrow tags as strings.
    pub fn as_slice(&self) -> &[String] {
        &self.tags
    }

    /// Convert into a vector of tags.
    pub fn into_vec(self) -> Vec<String> {
        self.tags
    }
}

impl IntoIterator for TagSet {
    type Item = String;
    type IntoIter = std::vec::IntoIter<String>;

    fn into_iter(self) -> Self::IntoIter {
        self.tags.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_set_new_is_empty() {
        let tags = TagSet::new();

        assert!(tags.is_empty());
        assert!(tags.as_slice().is_empty());
        assert!(tags.into_vec().is_empty());
    }

    #[test]
    fn tag_set_from_tag_adds_initial_tag() {
        let tags = TagSet::from_tag("users");

        assert_eq!(tags.as_slice(), &["users".to_owned()]);
    }

    #[test]
    fn tag_set_tags_entity_and_tenant_preserve_order() {
        let tags = TagSet::new()
            .tags(["users", "active"])
            .entity("user", 42)
            .tenant(7);

        assert_eq!(
            tags.as_slice(),
            &[
                "users".to_owned(),
                "active".to_owned(),
                "user:42".to_owned(),
                "tenant:7".to_owned()
            ]
        );
    }

    #[test]
    fn tag_set_entity_escapes_segments() {
        let tags = TagSet::new().entity("user:type", "42%beta");

        assert_eq!(tags.as_slice(), &["user%3Atype:42%25beta".to_owned()]);
    }

    #[test]
    fn tag_set_into_iterator_yields_owned_tags() {
        let tags: Vec<_> = TagSet::new()
            .tag("users")
            .tag("admins")
            .into_iter()
            .collect();

        assert_eq!(tags, vec!["users".to_owned(), "admins".to_owned()]);
    }
}
