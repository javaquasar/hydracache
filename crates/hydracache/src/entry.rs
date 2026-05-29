use std::time::Instant;

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
}
