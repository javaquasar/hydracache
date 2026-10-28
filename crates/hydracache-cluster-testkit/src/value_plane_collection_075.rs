//! Test-only bounded collection and lifecycle semantics for the provisional 0.75 IMap API.
//!
//! The model deliberately owns no production cursor, wire, durable, or partition identity.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

use sha2::{Digest, Sha256};

use crate::value_plane_model_075::MutationIdentity;

#[path = "value_plane_collection_delta_075.rs"]
mod replica_delta;
pub(crate) use replica_delta::{
    CollectionDeltaMeasure, CollectionReplicaCursor, CollectionReplicaPatch,
};

static NEXT_MAP_ID: AtomicU64 = AtomicU64::new(1);

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
    map_id: u64,
    purpose: [u8; 32],
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

/// One atomic observation of a live entry; TTL is relative to the supplied logical time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryView {
    pub value: Vec<u8>,
    pub version: u64,
    pub remaining_ttl: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimedMutationOptions {
    pub now: u64,
    pub deadline: u64,
    pub cancelled: bool,
    /// Deterministic response-loss checkpoint after application; retries retain their identity.
    pub lose_response: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimedRejection {
    Deadline,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimedMutationOutcome {
    Applied { changed: bool, version: Option<u64> },
    NotApplied(TimedRejection),
    OutcomeUnknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplacementEntry {
    pub key: Vec<u8>,
    pub value: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplacementReceipt {
    pub input_index: usize,
    pub key: Vec<u8>,
    pub replaced: bool,
    pub version: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PointCommand {
    Put {
        key: Vec<u8>,
        value: Vec<u8>,
        expires_at: Option<u64>,
    },
    Remove {
        key: Vec<u8>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RetainedCommand {
    Point(PointCommand),
    ReplaceAll(Vec<ReplacementEntry>),
    Service([u8; 32]),
}

#[derive(Debug, Clone)]
enum RetainedOutcome {
    Point(TimedMutationOutcome),
    Replacements(Vec<ReplacementReceipt>),
    ServicePending { result_bytes: usize },
    Service(serde_json::Value),
}

pub(crate) enum ServiceRetainedOutcome {
    Unknown,
    Pending,
    Completed(serde_json::Value),
}

pub(crate) type ExpiredServiceEntries = Vec<(Vec<u8>, Vec<u8>)>;

/// Pulls at most one bounded page at a time, including revision validation for buffered items.
#[derive(Debug)]
pub struct CollectionCursorIterator {
    projection: ScanProjection,
    page_items: usize,
    map_id: u64,
    revision: u64,
    cursor: Option<ScanCursor>,
    buffered: VecDeque<ScanItem>,
    complete: bool,
    failed: bool,
}

impl CollectionCursorIterator {
    pub fn next(
        &mut self,
        map: &mut ReferenceCollectionMap,
        now: u64,
    ) -> Option<Result<ScanItem, CollectionError>> {
        if self.failed {
            return None;
        }
        let result = self.next_item(map, now);
        match result {
            Ok(item) => item.map(Ok),
            Err(error) => {
                self.failed = true;
                self.buffered.clear();
                Some(Err(error))
            }
        }
    }

    fn next_item(
        &mut self,
        map: &mut ReferenceCollectionMap,
        now: u64,
    ) -> Result<Option<ScanItem>, CollectionError> {
        map.require_active()?;
        map.purge_expired(now)?;
        if self.map_id != map.map_id {
            return Err(CollectionError::CursorMismatch);
        }
        if self.revision != map.revision {
            return Err(CollectionError::CursorStale {
                cursor_revision: self.revision,
                current_revision: map.revision,
            });
        }
        if self.buffered.is_empty() && !self.complete {
            let page = map.scan(self.projection, self.cursor, self.page_items, now)?;
            self.cursor = page.next_cursor;
            self.complete = page.complete;
            self.buffered = page.items.into();
        }
        Ok(self.buffered.pop_front())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollectionError {
    InvalidBound(&'static str),
    InvalidExpiry,
    InvalidMutationIdentity,
    MutationIdentityConflict,
    DuplicateKey,
    CursorMismatch,
    VersionExhausted,
    MapIdentityExhausted,
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
    version: u64,
}

#[derive(Debug)]
pub struct ReferenceCollectionMap {
    bounds: CollectionBounds,
    entries: BTreeMap<Vec<u8>, StoredValue>,
    revision: u64,
    destroyed: bool,
    map_id: u64,
    retained: BTreeMap<MutationIdentity, (RetainedCommand, RetainedOutcome)>,
    service_mode: bool,
    expiry_index: BTreeSet<(u64, Vec<u8>)>,
    observed_time: u64,
    retirement_fences: BTreeMap<String, u64>,
}

impl ReferenceCollectionMap {
    pub fn new(bounds: CollectionBounds) -> Result<Self, CollectionError> {
        Ok(Self {
            bounds: bounds.validate()?,
            entries: BTreeMap::new(),
            revision: 1,
            destroyed: false,
            map_id: allocate_map_id(&NEXT_MAP_ID)?,
            retained: BTreeMap::new(),
            service_mode: false,
            expiry_index: BTreeSet::new(),
            observed_time: 0,
            retirement_fences: BTreeMap::new(),
        })
    }

    pub const fn revision(&self) -> u64 {
        self.revision
    }

    pub const fn is_destroyed(&self) -> bool {
        self.destroyed
    }

    pub fn retained_mutation_count(&self) -> usize {
        self.retained.len()
    }

    pub(crate) const fn service_map_id(&self) -> u64 {
        self.map_id
    }

    pub(crate) const fn service_bounds(&self) -> CollectionBounds {
        self.bounds
    }

    pub(crate) fn enable_service_mode(&mut self) {
        self.service_mode = true;
        self.expiry_index = self
            .entries
            .iter()
            .filter_map(|(key, value)| value.expires_at.map(|at| (at, key.clone())))
            .collect();
    }

    pub(crate) fn service_observe_time(&mut self, now: u64) -> Result<(), CollectionError> {
        if self.observed_time.checked_add(1).is_some_and(|start| {
            self.expiry_index
                .range((start, Vec::new())..)
                .next()
                .is_some_and(|(at, _)| *at <= now)
        }) {
            self.bump_revision()?;
        }
        self.observed_time = now;
        Ok(())
    }

    pub(crate) fn service_expire_step(
        &mut self,
        now: u64,
        maximum: usize,
    ) -> Result<ExpiredServiceEntries, CollectionError> {
        self.service_observe_time(now)?;
        let mut expired = Vec::new();
        for _ in 0..maximum {
            if self.expiry_index.first().is_none_or(|(at, _)| *at > now) {
                break;
            }
            let (at, key) = self.expiry_index.pop_first().expect("checked expiry head");
            if self
                .entries
                .get(&key)
                .is_some_and(|value| value.expires_at == Some(at))
            {
                let value = self.entries.remove(&key).expect("indexed entry exists");
                expired.push((key, value.value));
            }
        }
        Ok(expired)
    }

    pub(crate) fn service_expire_key(&mut self, key: &[u8], now: u64) -> Option<Vec<u8>> {
        if self
            .entries
            .get(key)
            .is_none_or(|value| is_live(value, now))
        {
            return None;
        }
        let value = self.entries.remove(key)?;
        if let Some(at) = value.expires_at {
            self.expiry_index.remove(&(at, key.to_vec()));
        }
        Some(value.value)
    }

    pub(crate) fn service_expiry_pending(&self, now: u64) -> bool {
        self.expiry_index.first().is_some_and(|(at, _)| *at <= now)
    }

    pub(crate) fn service_value_bytes(&self) -> usize {
        self.entries.iter().fold(0usize, |sum, (key, value)| {
            sum.saturating_add(24)
                .saturating_add(key.len())
                .saturating_add(value.value.len())
        })
    }

    pub(crate) fn service_physical_entry_bytes(&self, key: &[u8]) -> usize {
        self.entries.get(key).map_or(0, |value| {
            24usize
                .saturating_add(key.len())
                .saturating_add(value.value.len())
        })
    }

    pub(crate) fn service_retained_bytes(&self) -> (usize, usize) {
        let mut completed = self.retirement_fences.keys().fold(0usize, |sum, client| {
            sum.saturating_add(client.len()).saturating_add(24)
        });
        let mut pending = 0usize;
        for (id, (_, outcome)) in &self.retained {
            let base = id.client.len().saturating_add(48);
            let result = match outcome {
                RetainedOutcome::ServicePending { result_bytes } => {
                    pending = pending.saturating_add(base);
                    *result_bytes
                }
                RetainedOutcome::Service(value) => {
                    let value_len = value["value"].as_array().map_or(0, Vec::len);
                    value["items"]
                        .as_array()
                        .map_or(64usize.saturating_add(value_len), |items| {
                            items
                                .iter()
                                .fold(64usize.saturating_add(value_len), |sum, item| {
                                    sum.saturating_add(64)
                                        .saturating_add(item["key"].as_array().map_or(0, Vec::len))
                                        .saturating_add(
                                            item["value"].as_array().map_or(0, Vec::len),
                                        )
                                })
                        })
                }
                RetainedOutcome::Point(_) => 64,
                RetainedOutcome::Replacements(items) => items.iter().fold(64usize, |sum, item| {
                    sum.saturating_add(64).saturating_add(item.key.len())
                }),
            };
            completed = completed.saturating_add(base).saturating_add(result);
        }
        (completed, pending)
    }

    pub(crate) fn service_query_result_bytes(
        &self,
        identity: &MutationIdentity,
    ) -> Result<usize, CollectionError> {
        self.validate_mutation_identity(identity)?;
        let Some((_, outcome)) = self.retained.get(identity) else {
            return Ok(0);
        };
        Ok(match outcome {
            RetainedOutcome::ServicePending { .. } => 0,
            RetainedOutcome::Point(_) => 64,
            RetainedOutcome::Replacements(items) => items.iter().fold(64usize, |sum, item| {
                sum.saturating_add(64).saturating_add(item.key.len())
            }),
            RetainedOutcome::Service(value) => value["items"].as_array().map_or(
                64usize.saturating_add(value["value"].as_array().map_or(0, Vec::len)),
                |items| {
                    items.iter().fold(
                        64usize.saturating_add(value["value"].as_array().map_or(0, Vec::len)),
                        |sum, item| {
                            sum.saturating_add(64)
                                .saturating_add(item["key"].as_array().map_or(0, Vec::len))
                                .saturating_add(item["value"].as_array().map_or(0, Vec::len))
                        },
                    )
                },
            ),
        })
    }

    pub(crate) fn service_pending_before(&self, client: &str, through: u64) -> bool {
        self.retained.iter().any(|(id, (_, outcome))| {
            id.client == client
                && id.sequence <= through
                && matches!(outcome, RetainedOutcome::ServicePending { .. })
        })
    }

    pub(crate) fn service_retirement_credit(&self, client: &str, through: u64) -> usize {
        self.retained
            .iter()
            .filter(|(id, _)| id.client == client && id.sequence <= through)
            .fold(0usize, |sum, (id, (_, outcome))| {
                let result = match outcome {
                    RetainedOutcome::ServicePending { result_bytes } => *result_bytes,
                    RetainedOutcome::Service(value) => value["items"].as_array().map_or(
                        64usize.saturating_add(value["value"].as_array().map_or(0, Vec::len)),
                        |items| {
                            items.iter().fold(
                                64usize
                                    .saturating_add(value["value"].as_array().map_or(0, Vec::len)),
                                |sum, item| {
                                    sum.saturating_add(64)
                                        .saturating_add(item["key"].as_array().map_or(0, Vec::len))
                                        .saturating_add(
                                            item["value"].as_array().map_or(0, Vec::len),
                                        )
                                },
                            )
                        },
                    ),
                    RetainedOutcome::Point(_) => 64,
                    RetainedOutcome::Replacements(items) => {
                        items.iter().fold(64usize, |sum, item| {
                            sum.saturating_add(64).saturating_add(item.key.len())
                        })
                    }
                };
                sum.saturating_add(id.client.len())
                    .saturating_add(48)
                    .saturating_add(result)
            })
    }

    pub(crate) fn service_fences(&self) -> &BTreeMap<String, u64> {
        &self.retirement_fences
    }

    pub(crate) fn service_retired(&self, identity: &MutationIdentity) -> bool {
        self.retirement_fences
            .get(&identity.client)
            .is_some_and(|through| identity.sequence <= *through)
    }

    pub(crate) fn service_retire(&mut self, client: String, through: u64) -> usize {
        let before = self.retained.len();
        self.retained
            .retain(|id, _| id.client != client || id.sequence > through);
        self.retirement_fences.insert(client, through);
        before - self.retained.len()
    }

    pub(crate) fn service_checkpoint_clone(&self) -> Self {
        Self {
            bounds: self.bounds,
            entries: self.entries.clone(),
            revision: self.revision,
            destroyed: self.destroyed,
            map_id: self.map_id,
            retained: self.retained.clone(),
            service_mode: true,
            expiry_index: self.expiry_index.clone(),
            observed_time: self.observed_time,
            retirement_fences: self.retirement_fences.clone(),
        }
    }

    pub(crate) fn service_restore_identity(&mut self) -> Result<(), CollectionError> {
        self.map_id = allocate_map_id(&NEXT_MAP_ID)?;
        Ok(())
    }

    pub(crate) fn service_reserve_versions(&self, count: usize) -> Result<(), CollectionError> {
        self.reserve_revisions(count as u64)
    }

    pub(crate) fn service_counts(&self, now: u64) -> (usize, usize) {
        self.entries
            .values()
            .filter(|stored| is_live(stored, now))
            .fold((0, 0), |(count, bytes), stored| {
                (count + 1, bytes + stored.value.len())
            })
    }

    pub(crate) fn service_snapshot(&self, now: u64) -> Vec<(Vec<u8>, EntryView)> {
        self.entries
            .iter()
            .filter(|(_, stored)| is_live(stored, now))
            .map(|(key, stored)| {
                (
                    key.clone(),
                    EntryView {
                        value: stored.value.clone(),
                        version: stored.version,
                        remaining_ttl: stored.expires_at.map(|deadline| deadline - now),
                    },
                )
            })
            .collect()
    }

    pub(crate) fn service_snapshot_cost(
        &self,
        prefix: &[u8],
        exact: Option<&[u8]>,
        include_value: bool,
        now: u64,
    ) -> (usize, usize) {
        self.entries
            .iter()
            .filter(|(key, value)| {
                key.starts_with(prefix)
                    && exact.is_none_or(|exact| key.as_slice() == exact)
                    && is_live(value, now)
            })
            .fold((0usize, 0usize), |(count, bytes), (key, value)| {
                (
                    count.saturating_add(1),
                    bytes
                        .saturating_add(32)
                        .saturating_add(key.len())
                        .saturating_add(if include_value { value.value.len() } else { 0 }),
                )
            })
    }

    pub(crate) fn service_snapshot_value_exceeds(
        &self,
        prefix: &[u8],
        exact: Option<&[u8]>,
        now: u64,
        maximum: usize,
    ) -> bool {
        self.entries.iter().any(|(key, value)| {
            key.starts_with(prefix)
                && exact.is_none_or(|exact| key.as_slice() == exact)
                && is_live(value, now)
                && value.value.len() > maximum
        })
    }

    pub(crate) fn service_filtered_snapshot(
        &self,
        prefix: &[u8],
        exact: Option<&[u8]>,
        include_value: bool,
        now: u64,
    ) -> Vec<(Vec<u8>, EntryView)> {
        self.entries
            .iter()
            .filter(|(key, value)| {
                key.starts_with(prefix)
                    && exact.is_none_or(|exact| key.as_slice() == exact)
                    && is_live(value, now)
            })
            .map(|(key, value)| {
                (
                    key.clone(),
                    EntryView {
                        value: if include_value {
                            value.value.clone()
                        } else {
                            Vec::new()
                        },
                        version: value.version,
                        remaining_ttl: value.expires_at.map(|at| at - now),
                    },
                )
            })
            .collect()
    }

    pub(crate) fn service_entry(&self, key: &[u8], now: u64) -> Option<EntryView> {
        self.entries
            .get(key)
            .filter(|stored| is_live(stored, now))
            .map(|stored| EntryView {
                value: stored.value.clone(),
                version: stored.version,
                remaining_ttl: stored.expires_at.map(|deadline| deadline - now),
            })
    }

    pub(crate) fn service_live_value_len(&self, key: &[u8], now: u64) -> Option<usize> {
        self.entries
            .get(key)
            .filter(|value| is_live(value, now))
            .map(|value| value.value.len())
    }

    pub(crate) fn service_preflight_puts(
        &self,
        entries: &[ReplacementEntry],
        now: u64,
    ) -> Result<(), CollectionError> {
        let additional = entries
            .iter()
            .filter(|entry| {
                !self
                    .entries
                    .get(&entry.key)
                    .is_some_and(|stored| self.service_mode || is_live(stored, now))
            })
            .count();
        self.check_bound(
            "entries",
            self.bounds.max_entries,
            (if self.service_mode {
                self.entries.len()
            } else {
                self.live_count(now)
            })
            .saturating_add(additional),
        )?;
        self.reserve_revisions(entries.len() as u64)
    }

    pub(crate) fn service_preflight_removals(
        &self,
        keys: &[Vec<u8>],
        now: u64,
    ) -> Result<(), CollectionError> {
        let removed = keys
            .iter()
            .filter(|key| {
                self.entries
                    .get(key.as_slice())
                    .is_some_and(|stored| is_live(stored, now))
            })
            .count();
        self.reserve_revisions(removed as u64)
    }

    pub(crate) fn service_lookup(
        &self,
        identity: &MutationIdentity,
        digest: Option<[u8; 32]>,
    ) -> Result<ServiceRetainedOutcome, CollectionError> {
        self.validate_mutation_identity(identity)?;
        let Some((command, outcome)) = self.retained.get(identity) else {
            return Ok(ServiceRetainedOutcome::Unknown);
        };
        if let Some(digest) = digest {
            if command != &RetainedCommand::Service(digest) {
                return Err(CollectionError::MutationIdentityConflict);
            }
        }
        Ok(match outcome {
            RetainedOutcome::ServicePending { .. } => ServiceRetainedOutcome::Pending,
            RetainedOutcome::Service(value) => ServiceRetainedOutcome::Completed(value.clone()),
            RetainedOutcome::Point(outcome) => ServiceRetainedOutcome::Completed(match outcome {
                TimedMutationOutcome::Applied { changed, version } => {
                    serde_json::json!({"kind":"applied","flag":changed,"version":version})
                }
                TimedMutationOutcome::NotApplied(_) => serde_json::json!({"kind":"not_applied"}),
                TimedMutationOutcome::OutcomeUnknown => {
                    serde_json::json!({"kind":"outcome_unknown"})
                }
            }),
            RetainedOutcome::Replacements(receipts) => ServiceRetainedOutcome::Completed(
                serde_json::json!({"kind":"bulk","number":receipts.len(),"items":receipts.iter().map(|receipt|serde_json::json!({
                    "index":receipt.input_index,"key":receipt.key,"kind":if receipt.replaced{"replaced"}else{"absent"},
                    "value":null,"version":receipt.version,"remaining_ttl":null
                })).collect::<Vec<_>>()}),
            ),
        })
    }

    pub(crate) fn service_reserve(
        &mut self,
        identity: MutationIdentity,
        digest: [u8; 32],
        result_bytes: usize,
    ) -> Result<(), CollectionError> {
        self.require_active()?;
        self.validate_mutation_identity(&identity)?;
        if self.retained.contains_key(&identity) {
            return Err(CollectionError::MutationIdentityConflict);
        }
        self.check_bound(
            "retained_mutations",
            self.bounds.max_entries,
            self.retained.len().saturating_add(1),
        )?;
        self.retained.insert(
            identity,
            (
                RetainedCommand::Service(digest),
                RetainedOutcome::ServicePending { result_bytes },
            ),
        );
        Ok(())
    }

    pub(crate) fn service_complete(
        &mut self,
        identity: &MutationIdentity,
        value: serde_json::Value,
    ) {
        if let Some((_, outcome)) = self.retained.get_mut(identity) {
            *outcome = RetainedOutcome::Service(value);
        }
    }

    pub(crate) fn service_abort(&mut self, identity: &MutationIdentity) {
        self.retained.remove(identity);
    }

    pub(crate) fn service_pending_count(&self) -> usize {
        self.retained
            .values()
            .filter(|(_, outcome)| matches!(outcome, RetainedOutcome::ServicePending { .. }))
            .count()
    }

    pub(crate) fn service_pending_result_bytes(
        &self,
        identity: &MutationIdentity,
    ) -> Option<usize> {
        self.retained
            .get(identity)
            .and_then(|(_, outcome)| match outcome {
                RetainedOutcome::ServicePending { result_bytes } => Some(*result_bytes),
                _ => None,
            })
    }

    pub(crate) fn service_owner_counts(&self) -> (usize, usize, usize, usize) {
        let pending = self.service_pending_count();
        (
            self.entries.len(),
            self.retained.len() - pending,
            self.retirement_fences.len(),
            pending,
        )
    }

    pub(crate) fn service_pending_identities(&self) -> Vec<MutationIdentity> {
        self.retained
            .iter()
            .filter(|(_, (_, outcome))| matches!(outcome, RetainedOutcome::ServicePending { .. }))
            .map(|(identity, _)| identity.clone())
            .collect()
    }

    pub(crate) fn service_reclaim(&mut self, maximum: usize) -> usize {
        let mut removed = 0;
        while removed < maximum {
            if let Some((key, value)) = self.entries.pop_first() {
                if let Some(at) = value.expires_at {
                    self.expiry_index.remove(&(at, key));
                }
            } else if let Some(identity) = self
                .retained
                .iter()
                .find(|(_, (_, outcome))| {
                    !matches!(outcome, RetainedOutcome::ServicePending { .. })
                })
                .map(|(id, _)| id.clone())
            {
                self.retained.remove(&identity);
            } else if self.retirement_fences.pop_first().is_none() {
                break;
            }
            removed += 1;
        }
        if self.entries.is_empty() && self.retained.is_empty() && self.retirement_fences.is_empty()
        {
            self.destroyed = true;
        }
        removed
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
        let live_entries = if self.service_mode {
            self.entries.len()
        } else {
            self.live_count(now)
        };
        let key_is_live = self
            .entries
            .get(&key)
            .is_some_and(|stored| self.service_mode || is_live(stored, now));
        self.check_bound(
            "entries",
            self.bounds.max_entries,
            live_entries + usize::from(!key_is_live),
        )?;
        self.reserve_revisions(u64::from(self.has_expired(now)) + 1)?;
        self.purge_expired(now)?;
        self.bump_revision()?;
        if self.service_mode {
            if let Some(old) = self.entries.get(&key).and_then(|value| value.expires_at) {
                self.expiry_index.remove(&(old, key.clone()));
            }
            if let Some(at) = expires_at {
                self.expiry_index.insert((at, key.clone()));
            }
        }
        self.entries.insert(
            key,
            StoredValue {
                value,
                expires_at,
                version: self.revision,
            },
        );
        Ok(())
    }

    pub fn get_or_default(
        &mut self,
        key: &[u8],
        default: &[u8],
        now: u64,
    ) -> Result<Vec<u8>, CollectionError> {
        self.check_bound("value_bytes", self.bounds.max_value_bytes, default.len())?;
        match self.get_entry_view(key, now)? {
            Some(view) => Ok(view.value),
            None => {
                self.check_bound(
                    "response_bytes",
                    self.bounds.max_response_bytes,
                    default.len(),
                )?;
                Ok(default.to_vec())
            }
        }
    }

    pub fn contains_entry(
        &mut self,
        key: &[u8],
        value: &[u8],
        now: u64,
    ) -> Result<bool, CollectionError> {
        self.require_active()?;
        self.check_bound("key_bytes", self.bounds.max_key_bytes, key.len())?;
        self.check_bound("value_bytes", self.bounds.max_value_bytes, value.len())?;
        self.purge_expired(now)?;
        Ok(self
            .entries
            .get(key)
            .is_some_and(|stored| is_live(stored, now) && stored.value == value))
    }

    pub fn set(
        &mut self,
        key: Vec<u8>,
        value: Vec<u8>,
        expires_at: Option<u64>,
        now: u64,
    ) -> Result<(), CollectionError> {
        self.put(key, value, expires_at, now)
    }

    pub fn get_entry_view(
        &mut self,
        key: &[u8],
        now: u64,
    ) -> Result<Option<EntryView>, CollectionError> {
        self.require_active()?;
        self.check_bound("key_bytes", self.bounds.max_key_bytes, key.len())?;
        self.purge_expired(now)?;
        if let Some(stored) = self.entries.get(key).filter(|stored| is_live(stored, now)) {
            self.check_bound(
                "response_bytes",
                self.bounds.max_response_bytes,
                stored.value.len(),
            )?;
        }
        Ok(self
            .entries
            .get(key)
            .filter(|stored| is_live(stored, now))
            .map(|stored| EntryView {
                value: stored.value.clone(),
                version: stored.version,
                remaining_ttl: stored.expires_at.map(|deadline| deadline - now),
            }))
    }

    pub fn try_put(
        &mut self,
        identity: MutationIdentity,
        key: Vec<u8>,
        value: Vec<u8>,
        expires_at: Option<u64>,
        options: TimedMutationOptions,
    ) -> Result<TimedMutationOutcome, CollectionError> {
        let command = PointCommand::Put {
            key,
            value,
            expires_at,
        };
        self.execute_timed(identity, command, options)
    }

    pub fn try_remove(
        &mut self,
        identity: MutationIdentity,
        key: &[u8],
        options: TimedMutationOptions,
    ) -> Result<TimedMutationOutcome, CollectionError> {
        self.require_active()?;
        self.check_bound("key_bytes", self.bounds.max_key_bytes, key.len())?;
        self.execute_timed(
            identity,
            PointCommand::Remove { key: key.to_vec() },
            options,
        )
    }

    /// Recover a retained point result without dispatching new work or consulting a deadline.
    pub fn recover_timed_put(
        &self,
        identity: &MutationIdentity,
        key: &[u8],
        value: &[u8],
        expires_at: Option<u64>,
    ) -> Result<Option<TimedMutationOutcome>, CollectionError> {
        self.require_active()?;
        self.validate_mutation_identity(identity)?;
        self.check_bound("key_bytes", self.bounds.max_key_bytes, key.len())?;
        self.check_bound("value_bytes", self.bounds.max_value_bytes, value.len())?;
        match self.retained.get(identity) {
            None => Ok(None),
            Some((
                RetainedCommand::Point(PointCommand::Put {
                    key: retained_key,
                    value: retained_value,
                    expires_at: retained_expiry,
                }),
                RetainedOutcome::Point(outcome),
            )) if retained_key == key
                && retained_value == value
                && retained_expiry == &expires_at =>
            {
                Ok(Some(outcome.clone()))
            }
            _ => Err(CollectionError::MutationIdentityConflict),
        }
    }

    pub fn recover_timed_remove(
        &self,
        identity: &MutationIdentity,
        key: &[u8],
    ) -> Result<Option<TimedMutationOutcome>, CollectionError> {
        self.require_active()?;
        self.validate_mutation_identity(identity)?;
        self.check_bound("key_bytes", self.bounds.max_key_bytes, key.len())?;
        match self.retained.get(identity) {
            None => Ok(None),
            Some((
                RetainedCommand::Point(PointCommand::Remove { key: retained_key }),
                RetainedOutcome::Point(outcome),
            )) if retained_key == key => Ok(Some(outcome.clone())),
            _ => Err(CollectionError::MutationIdentityConflict),
        }
    }

    pub fn recover_replacements(
        &self,
        identity: &MutationIdentity,
        replacements: &[ReplacementEntry],
    ) -> Result<Option<Vec<ReplacementReceipt>>, CollectionError> {
        self.require_active()?;
        self.validate_mutation_identity(identity)?;
        self.validate_replacements(replacements)?;
        match self.retained.get(identity) {
            None => Ok(None),
            Some((
                RetainedCommand::ReplaceAll(retained_entries),
                RetainedOutcome::Replacements(receipts),
            )) if retained_entries == replacements => Ok(Some(receipts.clone())),
            _ => Err(CollectionError::MutationIdentityConflict),
        }
    }

    fn execute_timed(
        &mut self,
        identity: MutationIdentity,
        command: PointCommand,
        options: TimedMutationOptions,
    ) -> Result<TimedMutationOutcome, CollectionError> {
        self.require_active()?;
        self.validate_mutation_identity(&identity)?;
        match &command {
            PointCommand::Put { key, value, .. } => {
                self.check_bound("key_bytes", self.bounds.max_key_bytes, key.len())?;
                self.check_bound("value_bytes", self.bounds.max_value_bytes, value.len())?;
            }
            PointCommand::Remove { key } => {
                self.check_bound("key_bytes", self.bounds.max_key_bytes, key.len())?;
            }
        }
        let proposed_command = RetainedCommand::Point(command.clone());
        if let Some((retained_command, outcome)) = self.retained.get(&identity) {
            return match outcome {
                RetainedOutcome::Point(outcome) if retained_command == &proposed_command => {
                    Ok(outcome.clone())
                }
                _ => Err(CollectionError::MutationIdentityConflict),
            };
        }
        if options.cancelled {
            return Ok(TimedMutationOutcome::NotApplied(TimedRejection::Cancelled));
        }
        if options.now >= options.deadline {
            return Ok(TimedMutationOutcome::NotApplied(TimedRejection::Deadline));
        }
        self.check_bound(
            "retained_mutations",
            self.bounds.max_entries,
            self.retained.len().saturating_add(1),
        )?;
        let outcome = match &command {
            PointCommand::Put {
                key,
                value,
                expires_at,
            } => {
                self.put(key.clone(), value.clone(), *expires_at, options.now)?;
                TimedMutationOutcome::Applied {
                    changed: true,
                    version: Some(self.revision),
                }
            }
            PointCommand::Remove { key } => {
                let changed = self.evict(key, options.now)?;
                TimedMutationOutcome::Applied {
                    changed,
                    version: changed.then_some(self.revision),
                }
            }
        };
        self.retained.insert(
            identity,
            (proposed_command, RetainedOutcome::Point(outcome.clone())),
        );
        Ok(if options.lose_response {
            TimedMutationOutcome::OutcomeUnknown
        } else {
            outcome
        })
    }

    pub fn replace_all_detailed_idempotent(
        &mut self,
        identity: MutationIdentity,
        replacements: &[ReplacementEntry],
        now: u64,
    ) -> Result<Vec<ReplacementReceipt>, CollectionError> {
        self.require_active()?;
        self.validate_mutation_identity(&identity)?;
        self.validate_replacements(replacements)?;
        let command = RetainedCommand::ReplaceAll(replacements.to_vec());
        if let Some((retained_command, outcome)) = self.retained.get(&identity) {
            return match outcome {
                RetainedOutcome::Replacements(receipts) if retained_command == &command => {
                    Ok(receipts.clone())
                }
                _ => Err(CollectionError::MutationIdentityConflict),
            };
        }
        self.check_bound(
            "retained_mutations",
            self.bounds.max_entries,
            self.retained.len().saturating_add(1),
        )?;
        let receipts = self.replace_all_detailed(replacements, now)?;
        self.retained.insert(
            identity,
            (command, RetainedOutcome::Replacements(receipts.clone())),
        );
        Ok(receipts)
    }

    fn validate_mutation_identity(
        &self,
        identity: &MutationIdentity,
    ) -> Result<(), CollectionError> {
        if identity.client.trim().is_empty() || identity.sequence == 0 {
            return Err(CollectionError::InvalidMutationIdentity);
        }
        self.check_bound("client_identity_bytes", 128, identity.client.len())
    }

    /// Explicit-key nontransactional replacement; this local model preflights all inputs.
    pub fn replace_all_detailed(
        &mut self,
        replacements: &[ReplacementEntry],
        now: u64,
    ) -> Result<Vec<ReplacementReceipt>, CollectionError> {
        self.require_active()?;
        self.validate_replacements(replacements)?;
        let replacements_live = replacements
            .iter()
            .filter(|replacement| {
                self.entries
                    .get(&replacement.key)
                    .is_some_and(|stored| is_live(stored, now))
            })
            .count();
        self.reserve_revisions(u64::from(self.has_expired(now)) + replacements_live as u64)?;
        self.purge_expired(now)?;
        let mut receipts = Vec::with_capacity(replacements.len());
        for (input_index, replacement) in replacements.iter().enumerate() {
            let expiry = self
                .entries
                .get(&replacement.key)
                .map(|stored| stored.expires_at);
            let version = if let Some(expires_at) = expiry {
                self.put(
                    replacement.key.clone(),
                    replacement.value.clone(),
                    expires_at,
                    now,
                )?;
                Some(self.revision)
            } else {
                None
            };
            receipts.push(ReplacementReceipt {
                input_index,
                key: replacement.key.clone(),
                replaced: version.is_some(),
                version,
            });
        }
        Ok(receipts)
    }

    fn validate_replacements(
        &self,
        replacements: &[ReplacementEntry],
    ) -> Result<(), CollectionError> {
        if replacements.is_empty() {
            return Err(CollectionError::BoundExceeded {
                bound: "bulk_mutation_items",
                limit: self.bounds.max_bulk_mutation_items.min(256),
                actual: 0,
            });
        }
        self.check_bound(
            "bulk_mutation_items",
            self.bounds.max_bulk_mutation_items.min(256),
            replacements.len(),
        )?;
        let mut seen = BTreeSet::new();
        let mut response_bytes = 0_usize;
        let mut request_bytes = 0_usize;
        for replacement in replacements {
            self.check_bound(
                "key_bytes",
                self.bounds.max_key_bytes,
                replacement.key.len(),
            )?;
            self.check_bound(
                "value_bytes",
                self.bounds.max_value_bytes,
                replacement.value.len(),
            )?;
            if !seen.insert(&replacement.key) {
                return Err(CollectionError::DuplicateKey);
            }
            // Conservative domain accounting; no protocol or durable envelope is assigned.
            request_bytes = request_bytes
                .saturating_add(8)
                .saturating_add(replacement.key.len())
                .saturating_add(replacement.value.len());
            self.check_bound("request_bytes", 8 * 1_024 * 1_024, request_bytes)?;
            response_bytes = response_bytes
                .saturating_add(17)
                .saturating_add(replacement.key.len());
            self.check_bound(
                "response_bytes",
                self.bounds.max_response_bytes,
                response_bytes,
            )?;
        }
        Ok(())
    }

    pub fn iterator(
        &mut self,
        projection: ScanProjection,
        page_items: usize,
        now: u64,
    ) -> Result<CollectionCursorIterator, CollectionError> {
        self.require_active()?;
        if page_items == 0 {
            return Err(CollectionError::InvalidBound("page_items"));
        }
        self.check_bound("page_items", self.bounds.max_page_items, page_items)?;
        self.purge_expired(now)?;
        Ok(CollectionCursorIterator {
            projection,
            page_items,
            map_id: self.map_id,
            revision: self.revision,
            cursor: None,
            buffered: VecDeque::new(),
            complete: false,
            failed: false,
        })
    }

    pub fn size(&mut self, now: u64) -> Result<usize, CollectionError> {
        self.require_active()?;
        self.purge_expired(now)?;
        Ok(self.live_count(now))
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
        self.purge_expired(now)?;
        let purpose = search_purpose(value);
        let offset = self.cursor_offset(cursor, purpose)?;
        for stored in self
            .entries
            .values()
            .filter(|stored| is_live(stored, now))
            .skip(offset)
            .take(scan_budget)
        {
            if stored.value == value {
                return Ok(BoundedMatch::Present);
            }
        }
        let live = self.live_count(now);
        let next_offset = offset.saturating_add(scan_budget).min(live);
        if next_offset == live {
            Ok(BoundedMatch::Absent)
        } else {
            Ok(BoundedMatch::Incomplete(ScanCursor {
                map_id: self.map_id,
                purpose,
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
        self.purge_expired(now)?;
        let purpose = projection_purpose(projection);
        let offset = self.cursor_offset(cursor, purpose)?;
        let mut items = Vec::with_capacity(page_items.min(self.entries.len()));
        let mut response_bytes = 0_usize;
        let mut consumed = 0_usize;
        for (key, stored) in self
            .entries
            .iter()
            .filter(|(_, stored)| is_live(stored, now))
            .skip(offset)
            .take(page_items)
        {
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
        let complete = next_offset == self.live_count(now);
        Ok(ScanPage {
            items,
            next_cursor: (!complete).then_some(ScanCursor {
                map_id: self.map_id,
                purpose,
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
        let removed = self
            .entries
            .get(key)
            .is_some_and(|stored| is_live(stored, now));
        self.reserve_revisions(u64::from(self.has_expired(now)) + u64::from(removed))?;
        self.purge_expired(now)?;
        if removed {
            self.bump_revision()?;
            if let Some(at) = self.entries.get(key).and_then(|value| value.expires_at) {
                self.expiry_index.remove(&(at, key.to_vec()));
            }
            self.entries.remove(key);
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
        self.require_active()?;
        let removed = self.live_count(now);
        self.check_bound(
            "bulk_mutation_items",
            self.bounds.max_bulk_mutation_items,
            removed,
        )?;
        self.bump_revision()?;
        self.entries.clear();
        self.expiry_index.clear();
        self.retained.clear();
        self.retirement_fences.clear();
        self.destroyed = true;
        Ok(RemovalReceipt {
            cause: RemovalCause::Destroy,
            removed,
            revision: self.revision,
        })
    }

    fn remove_all(
        &mut self,
        cause: RemovalCause,
        now: u64,
    ) -> Result<RemovalReceipt, CollectionError> {
        self.require_active()?;
        let removed = self.live_count(now);
        self.check_bound(
            "bulk_mutation_items",
            self.bounds.max_bulk_mutation_items,
            removed,
        )?;
        self.reserve_revisions(u64::from(self.has_expired(now)) + u64::from(removed > 0))?;
        self.purge_expired(now)?;
        if removed > 0 {
            self.bump_revision()?;
            self.entries.clear();
            self.expiry_index.clear();
        }
        Ok(RemovalReceipt {
            cause,
            removed,
            revision: self.revision,
        })
    }

    fn cursor_offset(
        &self,
        cursor: Option<ScanCursor>,
        purpose: [u8; 32],
    ) -> Result<usize, CollectionError> {
        let Some(cursor) = cursor else {
            return Ok(0);
        };
        if cursor.map_id != self.map_id || cursor.purpose != purpose {
            return Err(CollectionError::CursorMismatch);
        }
        if cursor.revision != self.revision {
            return Err(CollectionError::CursorStale {
                cursor_revision: cursor.revision,
                current_revision: self.revision,
            });
        }
        Ok(cursor.offset.min(self.entries.len()))
    }

    fn purge_expired(&mut self, now: u64) -> Result<(), CollectionError> {
        if self.service_mode {
            return Ok(());
        }
        if self.has_expired(now) {
            self.bump_revision()?;
            self.entries.retain(|_, stored| is_live(stored, now));
        }
        Ok(())
    }

    fn live_count(&self, now: u64) -> usize {
        self.entries
            .values()
            .filter(|stored| is_live(stored, now))
            .count()
    }

    fn has_expired(&self, now: u64) -> bool {
        !self.service_mode && self.entries.values().any(|stored| !is_live(stored, now))
    }

    fn reserve_revisions(&self, count: u64) -> Result<(), CollectionError> {
        self.revision
            .checked_add(count)
            .map(|_| ())
            .ok_or(CollectionError::VersionExhausted)
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

    fn bump_revision(&mut self) -> Result<(), CollectionError> {
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or(CollectionError::VersionExhausted)?;
        Ok(())
    }
}

fn is_live(stored: &StoredValue, now: u64) -> bool {
    stored.expires_at.is_none_or(|deadline| deadline > now)
}

fn allocate_map_id(counter: &AtomicU64) -> Result<u64, CollectionError> {
    counter
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current.checked_add(1)
        })
        .map_err(|_| CollectionError::MapIdentityExhausted)
}

fn projection_purpose(projection: ScanProjection) -> [u8; 32] {
    let tag = match projection {
        ScanProjection::Keys => 1,
        ScanProjection::Values => 2,
        ScanProjection::Entries => 3,
    };
    Sha256::digest([tag]).into()
}

fn search_purpose(value: &[u8]) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update([4]);
    digest.update(value);
    digest.finalize().into()
}

#[cfg(test)]
mod counter_exhaustion_tests {
    use super::*;

    #[test]
    fn exposed_versions_never_saturate_or_partially_mutate() {
        let mut map = ReferenceCollectionMap::new(CollectionBounds::default()).unwrap();
        map.set(b"a".to_vec(), b"old".to_vec(), None, 0).unwrap();
        map.revision = u64::MAX;
        let before = map.entries.clone();
        assert_eq!(
            map.set(b"a".to_vec(), b"new".to_vec(), None, 0),
            Err(CollectionError::VersionExhausted)
        );
        assert_eq!(map.evict(b"a", 0), Err(CollectionError::VersionExhausted));
        assert_eq!(map.clear(0), Err(CollectionError::VersionExhausted));
        assert_eq!(map.evict_all(0), Err(CollectionError::VersionExhausted));
        assert_eq!(map.destroy(0), Err(CollectionError::VersionExhausted));
        assert_eq!(map.entries, before);
        assert!(!map.is_destroyed());
        assert_eq!(map.evict(b"absent", 0), Ok(false));

        let mut expiring = ReferenceCollectionMap::new(CollectionBounds::default()).unwrap();
        expiring
            .set(b"a".to_vec(), b"old".to_vec(), Some(1), 0)
            .unwrap();
        expiring.revision = u64::MAX;
        let before = expiring.entries.clone();
        assert_eq!(expiring.size(1), Err(CollectionError::VersionExhausted));
        assert_eq!(expiring.entries, before);
    }

    #[test]
    fn expiry_plus_mutation_and_multi_replacement_reserve_all_versions_first() {
        let mut map = ReferenceCollectionMap::new(CollectionBounds::default()).unwrap();
        map.set(b"a".to_vec(), b"live".to_vec(), None, 0).unwrap();
        map.set(b"b".to_vec(), b"expires".to_vec(), Some(1), 0)
            .unwrap();
        map.revision = u64::MAX - 1;
        let before = map.entries.clone();
        assert_eq!(
            map.set(b"a".to_vec(), b"new".to_vec(), None, 1),
            Err(CollectionError::VersionExhausted)
        );
        assert_eq!(map.evict(b"a", 1), Err(CollectionError::VersionExhausted));
        assert_eq!(map.clear(1), Err(CollectionError::VersionExhausted));
        let replacements = [ReplacementEntry {
            key: b"a".to_vec(),
            value: b"new".to_vec(),
        }];
        assert_eq!(
            map.replace_all_detailed(&replacements, 1),
            Err(CollectionError::VersionExhausted)
        );
        assert_eq!(map.entries, before);
        assert_eq!(map.revision, u64::MAX - 1);

        let mut multiple = ReferenceCollectionMap::new(CollectionBounds::default()).unwrap();
        multiple
            .set(b"a".to_vec(), b"one".to_vec(), None, 0)
            .unwrap();
        multiple
            .set(b"b".to_vec(), b"two".to_vec(), None, 0)
            .unwrap();
        multiple.revision = u64::MAX - 1;
        let before = multiple.entries.clone();
        let replacements = [
            ReplacementEntry {
                key: b"a".to_vec(),
                value: b"new".to_vec(),
            },
            ReplacementEntry {
                key: b"b".to_vec(),
                value: b"new".to_vec(),
            },
        ];
        assert_eq!(
            multiple.replace_all_detailed(&replacements, 0),
            Err(CollectionError::VersionExhausted)
        );
        assert_eq!(multiple.entries, before);
    }

    #[test]
    fn cursor_instance_identity_never_wraps() {
        let counter = AtomicU64::new(1);
        assert_eq!(allocate_map_id(&counter), Ok(1));
        assert_eq!(counter.load(Ordering::Relaxed), 2);
        counter.store(u64::MAX, Ordering::Relaxed);
        assert_eq!(
            allocate_map_id(&counter),
            Err(CollectionError::MapIdentityExhausted)
        );
        assert_eq!(counter.load(Ordering::Relaxed), u64::MAX);
    }

    #[test]
    fn unified_bulk_remove_reserves_every_version_before_mutation() {
        use crate::value_plane_service_075::{
            OperationOutcomeQuery, ServiceCommand, UnifiedIMapService,
        };
        let mut map = ReferenceCollectionMap::new(CollectionBounds::default()).unwrap();
        map.put(b"a".to_vec(), b"one".to_vec(), None, 0).unwrap();
        map.put(b"b".to_vec(), b"two".to_vec(), None, 0).unwrap();
        map.revision = u64::MAX - 1;
        let service = UnifiedIMapService::from_map(map, 1).unwrap();
        let identity = MutationIdentity::new("bulk-remove", 1);
        let mut command = ServiceCommand::point("remove_all_detailed", b"");
        command.keys = vec![b"a".to_vec(), b"b".to_vec()];
        command.identity = Some(identity.clone());
        assert_eq!(service.execute(command).unwrap_err().kind(), "capacity");
        let entries = service.snapshot(0).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].value, b"one");
        assert_eq!(entries[1].value, b"two");
        assert_eq!(service.revision(), u64::MAX - 1);
        assert_eq!(
            service.query_operation_outcome(&identity).unwrap(),
            OperationOutcomeQuery::Unknown
        );
        assert_eq!(service.statistics().retained_outcomes, 0);
    }

    #[test]
    fn legacy_expiry_index_stays_bounded_and_service_refresh_removes_old_deadlines() {
        let mut map = ReferenceCollectionMap::new(CollectionBounds {
            max_entries: 1,
            ..CollectionBounds::default()
        })
        .unwrap();
        for now in 0..128 {
            map.put(b"a".to_vec(), b"v".to_vec(), Some(now + 1), now)
                .unwrap();
            assert_eq!(map.size(now + 1).unwrap(), 0);
            assert!(map.expiry_index.is_empty());
        }
        map.enable_service_mode();
        map.put(b"a".to_vec(), b"v".to_vec(), Some(200), 128)
            .unwrap();
        map.put(b"a".to_vec(), b"v".to_vec(), Some(300), 128)
            .unwrap();
        assert_eq!(map.expiry_index.len(), 1);
        assert!(map.service_expire_step(200, 1).unwrap().is_empty());
        map.clear(200).unwrap();
        assert!(map.expiry_index.is_empty());
    }

    #[test]
    fn version_bulk_exhaustion_has_no_partial_mutations_or_retained_receipt() {
        use crate::value_plane_service_075::{
            OperationOutcomeQuery, ServiceCommand, ServiceTtl, UnifiedIMapService,
            VersionBulkEntry, VersionBulkOperation,
        };
        let mut map = ReferenceCollectionMap::new(CollectionBounds::default()).unwrap();
        map.put(b"a".to_vec(), b"one".to_vec(), None, 0).unwrap();
        map.put(b"b".to_vec(), b"two".to_vec(), None, 0).unwrap();
        map.revision = u64::MAX - 1;
        let service = UnifiedIMapService::from_map(map, 1).unwrap();
        let entries = [b"a", b"b"]
            .iter()
            .map(|key| VersionBulkEntry {
                key: key.to_vec(),
                token: service
                    .execute(ServiceCommand::point("get_entry_view", *key))
                    .unwrap()
                    .token
                    .unwrap(),
                value: Some(b"new".to_vec()),
                ttl: ServiceTtl::Eternal,
            })
            .collect::<Vec<_>>();
        let identity = MutationIdentity::new("version-bulk", 1);
        assert_eq!(
            service
                .bulk_version_conditional(
                    VersionBulkOperation::Replace,
                    &entries,
                    identity.clone(),
                    0,
                    1,
                    false
                )
                .unwrap_err()
                .kind(),
            "capacity"
        );
        assert_eq!(
            service
                .diagnostic_snapshot(0)
                .unwrap()
                .iter()
                .map(|entry| entry.value.as_slice())
                .collect::<Vec<_>>(),
            [b"one".as_slice(), b"two".as_slice()]
        );
        assert_eq!(
            service.query_operation_outcome(&identity).unwrap(),
            OperationOutcomeQuery::Unknown
        );
    }
}
