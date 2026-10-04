//! Test-only bounded collection and lifecycle semantics for the provisional 0.75 IMap API.
//!
//! The model deliberately owns no production cursor, wire, durable, or partition identity.

use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollectionBounds {
    pub max_key_bytes: usize,
    pub max_value_bytes: usize,
    pub max_entries: usize,
    pub max_page_items: usize,
    pub max_scan_items: usize,
    pub max_response_bytes: usize,
    pub max_bulk_mutation_items: usize,
}

impl Default for CollectionBounds {
    fn default() -> Self {
        Self {
            max_key_bytes: 1_024 * 1_024,
            max_value_bytes: 4 * 1_024 * 1_024,
            max_entries: 4_096,
            max_page_items: 256,
            max_scan_items: 4_096,
            max_response_bytes: 8 * 1_024 * 1_024,
            max_bulk_mutation_items: 4_096,
        }
    }
}

impl CollectionBounds {
    fn validate(self) -> Result<Self, CollectionError> {
        for (name, value) in [
            ("key_bytes", self.max_key_bytes),
            ("value_bytes", self.max_value_bytes),
            ("entries", self.max_entries),
            ("page_items", self.max_page_items),
            ("scan_items", self.max_scan_items),
            ("response_bytes", self.max_response_bytes),
            ("bulk_mutation_items", self.max_bulk_mutation_items),
        ] {
            if value == 0 {
                return Err(CollectionError::InvalidBound(name));
            }
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanProjection {
    Keys,
    Values,
    Entries,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanItem {
    pub key: Option<Vec<u8>>,
    pub value: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanCursor {
    revision: u64,
    offset: usize,
}

impl ScanCursor {
    pub const fn revision(self) -> u64 {
        self.revision
    }

    pub const fn offset(self) -> usize {
        self.offset
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanPage {
    pub items: Vec<ScanItem>,
    pub next_cursor: Option<ScanCursor>,
    pub complete: bool,
    pub snapshot_revision: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundedMatch {
    Present,
    Absent,
    Incomplete(ScanCursor),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemovalCause {
    Clear,
    Evict,
    Destroy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemovalReceipt {
    pub cause: RemovalCause,
    pub removed: usize,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollectionError {
    InvalidBound(&'static str),
    InvalidExpiry,
    BoundExceeded {
        bound: &'static str,
        limit: usize,
        actual: usize,
    },
    CursorStale {
        cursor_revision: u64,
        current_revision: u64,
    },
    MapDestroyed,
}

impl fmt::Display for CollectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for CollectionError {}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StoredValue {
    value: Vec<u8>,
    expires_at: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct ReferenceCollectionMap {
    bounds: CollectionBounds,
    entries: BTreeMap<Vec<u8>, StoredValue>,
    revision: u64,
    destroyed: bool,
}

impl ReferenceCollectionMap {
    pub fn new(bounds: CollectionBounds) -> Result<Self, CollectionError> {
        Ok(Self {
            bounds: bounds.validate()?,
            entries: BTreeMap::new(),
            revision: 1,
            destroyed: false,
        })
    }

    pub const fn revision(&self) -> u64 {
        self.revision
    }

    pub const fn is_destroyed(&self) -> bool {
        self.destroyed
    }

    pub fn put(
        &mut self,
        key: Vec<u8>,
        value: Vec<u8>,
        expires_at: Option<u64>,
        now: u64,
    ) -> Result<(), CollectionError> {
        self.require_active()?;
        self.check_bound("key_bytes", self.bounds.max_key_bytes, key.len())?;
        self.check_bound("value_bytes", self.bounds.max_value_bytes, value.len())?;
        if expires_at.is_some_and(|deadline| deadline <= now) {
            return Err(CollectionError::InvalidExpiry);
        }
        self.purge_expired(now);
        if !self.entries.contains_key(&key) {
            self.check_bound(
                "entries",
                self.bounds.max_entries,
                self.entries.len().saturating_add(1),
            )?;
        }
        self.entries.insert(key, StoredValue { value, expires_at });
        self.bump_revision();
        Ok(())
    }

    pub fn size(&mut self, now: u64) -> Result<usize, CollectionError> {
        self.require_active()?;
        self.purge_expired(now);
        Ok(self.entries.len())
    }

    pub fn is_empty(&mut self, now: u64) -> Result<bool, CollectionError> {
        Ok(self.size(now)? == 0)
    }

    pub fn contains_value(
        &mut self,
        value: &[u8],
        cursor: Option<ScanCursor>,
        scan_budget: usize,
        now: u64,
    ) -> Result<BoundedMatch, CollectionError> {
        self.require_active()?;
        self.check_bound("value_bytes", self.bounds.max_value_bytes, value.len())?;
        if scan_budget == 0 {
            return Err(CollectionError::BoundExceeded {
                bound: "scan_items",
                limit: self.bounds.max_scan_items,
                actual: 0,
            });
        }
        self.check_bound("scan_items", self.bounds.max_scan_items, scan_budget)?;
        self.purge_expired(now);
        let offset = self.cursor_offset(cursor)?;
        for stored in self.entries.values().skip(offset).take(scan_budget) {
            if stored.value == value {
                return Ok(BoundedMatch::Present);
            }
        }
        let next_offset = offset.saturating_add(scan_budget).min(self.entries.len());
        if next_offset == self.entries.len() {
            Ok(BoundedMatch::Absent)
        } else {
            Ok(BoundedMatch::Incomplete(ScanCursor {
                revision: self.revision,
                offset: next_offset,
            }))
        }
    }

    pub fn scan(
        &mut self,
        projection: ScanProjection,
        cursor: Option<ScanCursor>,
        page_items: usize,
        now: u64,
    ) -> Result<ScanPage, CollectionError> {
        self.require_active()?;
        if page_items == 0 {
            return Err(CollectionError::BoundExceeded {
                bound: "page_items",
                limit: self.bounds.max_page_items,
                actual: 0,
            });
        }
        self.check_bound("page_items", self.bounds.max_page_items, page_items)?;
        self.purge_expired(now);
        let offset = self.cursor_offset(cursor)?;
        let mut items = Vec::with_capacity(page_items.min(self.entries.len()));
        let mut response_bytes = 0_usize;
        let mut consumed = 0_usize;
        for (key, stored) in self.entries.iter().skip(offset).take(page_items) {
            let encoded_bytes = match projection {
                ScanProjection::Keys => key.len(),
                ScanProjection::Values => stored.value.len(),
                ScanProjection::Entries => key.len().saturating_add(stored.value.len()),
            };
            self.check_bound(
                "response_bytes",
                self.bounds.max_response_bytes,
                response_bytes.saturating_add(encoded_bytes),
            )?;
            response_bytes = response_bytes.saturating_add(encoded_bytes);
            items.push(match projection {
                ScanProjection::Keys => ScanItem {
                    key: Some(key.clone()),
                    value: None,
                },
                ScanProjection::Values => ScanItem {
                    key: None,
                    value: Some(stored.value.clone()),
                },
                ScanProjection::Entries => ScanItem {
                    key: Some(key.clone()),
                    value: Some(stored.value.clone()),
                },
            });
            consumed = consumed.saturating_add(1);
        }
        let next_offset = offset.saturating_add(consumed);
        let complete = next_offset == self.entries.len();
        Ok(ScanPage {
            items,
            next_cursor: (!complete).then_some(ScanCursor {
                revision: self.revision,
                offset: next_offset,
            }),
            complete,
            snapshot_revision: self.revision,
        })
    }

    pub fn evict(&mut self, key: &[u8], now: u64) -> Result<bool, CollectionError> {
        self.require_active()?;
        self.check_bound("key_bytes", self.bounds.max_key_bytes, key.len())?;
        self.purge_expired(now);
        let removed = self.entries.remove(key).is_some();
        if removed {
            self.bump_revision();
        }
        Ok(removed)
    }

    pub fn clear(&mut self, now: u64) -> Result<RemovalReceipt, CollectionError> {
        self.remove_all(RemovalCause::Clear, now)
    }

    pub fn evict_all(&mut self, now: u64) -> Result<RemovalReceipt, CollectionError> {
        self.remove_all(RemovalCause::Evict, now)
    }

    pub fn destroy(&mut self, now: u64) -> Result<RemovalReceipt, CollectionError> {
        let receipt = self.remove_all(RemovalCause::Destroy, now)?;
        self.destroyed = true;
        self.bump_revision();
        Ok(RemovalReceipt {
            revision: self.revision,
            ..receipt
        })
    }

    fn remove_all(
        &mut self,
        cause: RemovalCause,
        now: u64,
    ) -> Result<RemovalReceipt, CollectionError> {
        self.require_active()?;
        self.purge_expired(now);
        self.check_bound(
            "bulk_mutation_items",
            self.bounds.max_bulk_mutation_items,
            self.entries.len(),
        )?;
        let removed = self.entries.len();
        if removed > 0 {
            self.entries.clear();
            self.bump_revision();
        }
        Ok(RemovalReceipt {
            cause,
            removed,
            revision: self.revision,
        })
    }

    fn cursor_offset(&self, cursor: Option<ScanCursor>) -> Result<usize, CollectionError> {
        let Some(cursor) = cursor else {
            return Ok(0);
        };
        if cursor.revision != self.revision {
            return Err(CollectionError::CursorStale {
                cursor_revision: cursor.revision,
                current_revision: self.revision,
            });
        }
        Ok(cursor.offset.min(self.entries.len()))
    }

    fn purge_expired(&mut self, now: u64) {
        let before = self.entries.len();
        self.entries
            .retain(|_, stored| stored.expires_at.is_none_or(|deadline| deadline > now));
        if self.entries.len() != before {
            self.bump_revision();
        }
    }

    fn require_active(&self) -> Result<(), CollectionError> {
        if self.destroyed {
            Err(CollectionError::MapDestroyed)
        } else {
            Ok(())
        }
    }

    fn check_bound(
        &self,
        bound: &'static str,
        limit: usize,
        actual: usize,
    ) -> Result<(), CollectionError> {
        if actual > limit {
            Err(CollectionError::BoundExceeded {
                bound,
                limit,
                actual,
            })
        } else {
            Ok(())
        }
    }

    fn bump_revision(&mut self) {
        self.revision = self.revision.saturating_add(1);
    }
}
