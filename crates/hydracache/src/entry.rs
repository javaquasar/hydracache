use std::time::{Duration, Instant};

use bytes::Bytes;

#[derive(Debug, Clone)]
pub(crate) struct CacheEntry {
    pub(crate) value: Bytes,
    pub(crate) tags: Vec<String>,
    pub(crate) expires_at: Option<Instant>,
}

impl CacheEntry {
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
