use std::sync::Arc;
use std::time::{Duration, Instant};

use bytes::Bytes;

#[derive(Debug, Clone)]
pub(crate) struct CacheEntry {
    pub(crate) value: Bytes,
    pub(crate) tags: Arc<[String]>,
    pub(crate) expires_at: Option<Instant>,
    pub(crate) version: u64,
}

impl CacheEntry {
    pub(crate) fn new(
        value: Bytes,
        tags: Vec<String>,
        expires_at: Option<Instant>,
        version: u64,
    ) -> Self {
        Self {
            value,
            tags: tags.into(),
            expires_at,
            version,
        }
    }

    pub(crate) fn is_expired(&self) -> bool {
        self.expires_at
            .map(|expires_at| Instant::now() >= expires_at)
            .unwrap_or(false)
    }

    pub(crate) fn stale_window_contains_now(&self, window: Duration) -> bool {
        self.expires_at
            .and_then(|expires_at| expires_at.checked_add(window))
            .map(|stale_until| Instant::now() < stale_until)
            .unwrap_or(false)
    }

    pub(crate) fn refresh_ahead_due(&self, threshold: Duration) -> bool {
        self.expires_at
            .map(|expires_at| {
                let now = Instant::now();
                expires_at <= now || expires_at.duration_since(now) <= threshold
            })
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clone_shares_immutable_tags() {
        let entry = CacheEntry::new(
            Bytes::from_static(b"value"),
            vec!["blue".to_owned(), "tenant:1".to_owned()],
            None,
            1,
        );

        let cloned = entry.clone();

        assert!(Arc::ptr_eq(&entry.tags, &cloned.tags));
        assert_eq!(&*cloned.tags, &["blue".to_owned(), "tenant:1".to_owned()]);
    }
}
