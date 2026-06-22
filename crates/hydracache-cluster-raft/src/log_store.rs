use std::fmt;
use std::sync::{Arc, RwLock};

use raft::eraftpb::{ConfState, Entry, HardState, Snapshot};
use raft::storage::{GetEntriesContext, RaftState, Storage};
use raft::{Error as RaftError, Result as RaftResult, StorageError};

/// Result type used by [`RaftLogStore`].
pub type RaftStoreResult<T> = std::result::Result<T, RaftStoreError>;

/// Error returned by the 0.41 raft log-store seam.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaftStoreError {
    message: String,
}

impl RaftStoreError {
    /// Create a store error.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for RaftStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for RaftStoreError {}

impl From<RaftError> for RaftStoreError {
    fn from(error: RaftError) -> Self {
        Self::new(error.to_string())
    }
}

/// Durable-control-plane storage contract used by the 0.41 raft runtime.
pub trait RaftLogStore: Storage + Clone + Send + Sync + 'static {
    /// Persist term/vote/commit.
    fn save_hard_state(&self, hard_state: &HardState) -> RaftStoreResult<()>;

    /// Append entries, overwriting any existing suffix from `entries[0].index`.
    fn append(&self, entries: &[Entry]) -> RaftStoreResult<()>;

    /// Drop entries at and after `from_index`.
    fn truncate_suffix(&self, from_index: u64) -> RaftStoreResult<()>;

    /// Atomically install a snapshot and preserve optional trailing entries.
    fn save_snapshot(
        &self,
        snapshot: &Snapshot,
        preserve_log_entries: usize,
    ) -> RaftStoreResult<()>;

    /// Compact entries before `index`; must not compact past applied progress.
    fn compact_to(&self, index: u64) -> RaftStoreResult<()>;

    /// Return whether the store requires fsync before outbound messages.
    fn must_sync(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone)]
struct InMemoryRaftLogState {
    raft_state: RaftState,
    entries: Vec<Entry>,
    snapshot: Snapshot,
    applied_index: u64,
    snapshot_unavailable_once: bool,
}

impl Default for InMemoryRaftLogState {
    fn default() -> Self {
        Self {
            raft_state: RaftState::default(),
            entries: Vec::new(),
            snapshot: Snapshot::default(),
            applied_index: 0,
            snapshot_unavailable_once: false,
        }
    }
}

/// Deterministic in-memory [`RaftLogStore`] used by tests and the default
/// single-node 0.41 control-plane slice.
#[derive(Clone, Default)]
pub struct InMemoryRaftLogStore {
    state: Arc<RwLock<InMemoryRaftLogState>>,
}

impl fmt::Debug for InMemoryRaftLogStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InMemoryRaftLogStore")
            .field("first_index", &self.first_index().ok())
            .field("last_index", &self.last_index().ok())
            .finish_non_exhaustive()
    }
}

impl InMemoryRaftLogStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a store initialized with a cluster conf state.
    pub fn new_with_conf_state<T>(conf_state: T) -> Self
    where
        ConfState: From<T>,
    {
        let store = Self::new();
        store.initialize_with_conf_state(conf_state);
        store
    }

    /// Initialize raft configuration state.
    pub fn initialize_with_conf_state<T>(&self, conf_state: T)
    where
        ConfState: From<T>,
    {
        self.state
            .write()
            .expect("raft log store poisoned")
            .raft_state
            .conf_state = ConfState::from(conf_state);
    }

    /// Return all retained entries.
    pub fn all_entries(&self) -> Vec<Entry> {
        self.state
            .read()
            .expect("raft log store poisoned")
            .entries
            .clone()
    }

    /// Mark entries through `index` as applied.
    pub fn mark_applied(&self, index: u64) {
        let mut state = self.state.write().expect("raft log store poisoned");
        state.applied_index = state.applied_index.max(index);
    }

    /// Return applied index tracked by compaction guards.
    pub fn applied_index(&self) -> u64 {
        self.state
            .read()
            .expect("raft log store poisoned")
            .applied_index
    }

    /// Make the next snapshot call return `SnapshotTemporarilyUnavailable`.
    pub fn trigger_snapshot_temporarily_unavailable(&self) {
        self.state
            .write()
            .expect("raft log store poisoned")
            .snapshot_unavailable_once = true;
    }

    /// Update the hard-state commit index after raft light-ready advance.
    pub fn set_commit(&self, commit: u64) {
        let mut state = self.state.write().expect("raft log store poisoned");
        state.raft_state.hard_state.commit = commit;
    }
}

impl Storage for InMemoryRaftLogStore {
    fn initial_state(&self) -> RaftResult<RaftState> {
        Ok(self
            .state
            .read()
            .expect("raft log store poisoned")
            .raft_state
            .clone())
    }

    fn entries(
        &self,
        low: u64,
        high: u64,
        max_size: impl Into<Option<u64>>,
        _context: GetEntriesContext,
    ) -> RaftResult<Vec<Entry>> {
        let state = self.state.read().expect("raft log store poisoned");
        let first_index = first_index(&state);
        let last_index = last_index(&state);
        if low < first_index {
            return Err(RaftError::Store(StorageError::Compacted));
        }
        if high > last_index.saturating_add(1) {
            panic!("index out of bound (last: {}, high: {})", last_index + 1, high);
        }
        if low == high {
            return Ok(Vec::new());
        }
        let mut entries = state
            .entries
            .iter()
            .filter(|entry| entry.index >= low && entry.index < high)
            .cloned()
            .collect::<Vec<_>>();
        if let Some(max_size) = max_size.into() {
            limit_entries_size(&mut entries, max_size);
        }
        Ok(entries)
    }

    fn term(&self, idx: u64) -> RaftResult<u64> {
        let state = self.state.read().expect("raft log store poisoned");
        let snapshot_index = state.snapshot.get_metadata().index;
        if idx == 0 {
            return Ok(0);
        }
        if idx == snapshot_index {
            return Ok(state.snapshot.get_metadata().term);
        }
        let first_index = first_index(&state);
        let last_index = last_index(&state);
        if idx < first_index {
            return Err(RaftError::Store(StorageError::Compacted));
        }
        if idx > last_index {
            return Err(RaftError::Store(StorageError::Unavailable));
        }
        state
            .entries
            .iter()
            .find(|entry| entry.index == idx)
            .map(|entry| entry.term)
            .ok_or(RaftError::Store(StorageError::Unavailable))
    }

    fn first_index(&self) -> RaftResult<u64> {
        Ok(first_index(
            &self.state.read().expect("raft log store poisoned"),
        ))
    }

    fn last_index(&self) -> RaftResult<u64> {
        Ok(last_index(&self.state.read().expect("raft log store poisoned")))
    }

    fn snapshot(&self, request_index: u64, _to: u64) -> RaftResult<Snapshot> {
        let mut state = self.state.write().expect("raft log store poisoned");
        if state.snapshot_unavailable_once {
            state.snapshot_unavailable_once = false;
            return Err(RaftError::Store(
                StorageError::SnapshotTemporarilyUnavailable,
            ));
        }
        let mut snapshot = state.snapshot.clone();
        if snapshot.get_metadata().index < request_index {
            snapshot.mut_metadata().index = request_index;
        }
        Ok(snapshot)
    }
}

impl RaftLogStore for InMemoryRaftLogStore {
    fn save_hard_state(&self, hard_state: &HardState) -> RaftStoreResult<()> {
        self.state
            .write()
            .expect("raft log store poisoned")
            .raft_state
            .hard_state = hard_state.clone();
        Ok(())
    }

    fn append(&self, entries: &[Entry]) -> RaftStoreResult<()> {
        if entries.is_empty() {
            return Ok(());
        }
        let mut state = self.state.write().expect("raft log store poisoned");
        let start = entries[0].index;
        let first = first_index(&state);
        let last = last_index(&state);
        if start < first {
            return Err(RaftStoreError::new(format!(
                "append would overwrite compacted entries: first {first}, append {start}"
            )));
        }
        if start > last.saturating_add(1) {
            return Err(RaftStoreError::new(format!(
                "append leaves a gap: last {last}, append {start}"
            )));
        }
        state.entries.retain(|entry| entry.index < start);
        state.entries.extend_from_slice(entries);
        Ok(())
    }

    fn truncate_suffix(&self, from_index: u64) -> RaftStoreResult<()> {
        self.state
            .write()
            .expect("raft log store poisoned")
            .entries
            .retain(|entry| entry.index < from_index);
        Ok(())
    }

    fn save_snapshot(
        &self,
        snapshot: &Snapshot,
        preserve_log_entries: usize,
    ) -> RaftStoreResult<()> {
        let mut state = self.state.write().expect("raft log store poisoned");
        let snapshot_index = snapshot.get_metadata().index;
        state.snapshot = snapshot.clone();
        state.raft_state.conf_state = snapshot.get_metadata().get_conf_state().clone();
        state.raft_state.hard_state.term =
            state.raft_state.hard_state.term.max(snapshot.get_metadata().term);
        state.raft_state.hard_state.commit = state.raft_state.hard_state.commit.max(snapshot_index);
        state.entries.retain(|entry| entry.index > snapshot_index);
        if preserve_log_entries < state.entries.len() {
            let keep_from = state.entries.len() - preserve_log_entries;
            state.entries.drain(..keep_from);
        }
        Ok(())
    }

    fn compact_to(&self, index: u64) -> RaftStoreResult<()> {
        let mut state = self.state.write().expect("raft log store poisoned");
        if index > state.applied_index {
            return Err(RaftStoreError::new(format!(
                "compact index {index} is past applied index {}",
                state.applied_index
            )));
        }
        state.entries.retain(|entry| entry.index >= index);
        Ok(())
    }
}

fn first_index(state: &InMemoryRaftLogState) -> u64 {
    state
        .entries
        .first()
        .map(|entry| entry.index)
        .unwrap_or_else(|| state.snapshot.get_metadata().index.saturating_add(1))
}

fn last_index(state: &InMemoryRaftLogState) -> u64 {
    state
        .entries
        .last()
        .map(|entry| entry.index)
        .unwrap_or_else(|| state.snapshot.get_metadata().index)
}

fn limit_entries_size(entries: &mut Vec<Entry>, max_size: u64) {
    if entries.len() <= 1 {
        return;
    }
    let mut total = 0_u64;
    let mut keep = entries.len();
    for (index, entry) in entries.iter().enumerate() {
        total = total.saturating_add(entry.data.len() as u64);
        if index > 0 && total > max_size {
            keep = index;
            break;
        }
    }
    entries.truncate(keep.max(1));
}

/// Feature-gated durable-engine example placeholder.
///
/// The public type exists behind `sled-log-store` so integration and docs can
/// compile the alternate engine path without making it the default.
#[cfg(feature = "sled-log-store")]
#[derive(Clone, Debug, Default)]
pub struct SledRaftLogStore {
    inner: InMemoryRaftLogStore,
}

#[cfg(feature = "sled-log-store")]
impl SledRaftLogStore {
    /// Create the feature-gated example store.
    pub fn new_for_tests() -> Self {
        Self::default()
    }
}

#[cfg(feature = "sled-log-store")]
impl Storage for SledRaftLogStore {
    fn initial_state(&self) -> RaftResult<RaftState> {
        self.inner.initial_state()
    }

    fn entries(
        &self,
        low: u64,
        high: u64,
        max_size: impl Into<Option<u64>>,
        context: GetEntriesContext,
    ) -> RaftResult<Vec<Entry>> {
        self.inner.entries(low, high, max_size, context)
    }

    fn term(&self, idx: u64) -> RaftResult<u64> {
        self.inner.term(idx)
    }

    fn first_index(&self) -> RaftResult<u64> {
        self.inner.first_index()
    }

    fn last_index(&self) -> RaftResult<u64> {
        self.inner.last_index()
    }

    fn snapshot(&self, request_index: u64, to: u64) -> RaftResult<Snapshot> {
        self.inner.snapshot(request_index, to)
    }
}

#[cfg(feature = "sled-log-store")]
impl RaftLogStore for SledRaftLogStore {
    fn save_hard_state(&self, hard_state: &HardState) -> RaftStoreResult<()> {
        self.inner.save_hard_state(hard_state)
    }

    fn append(&self, entries: &[Entry]) -> RaftStoreResult<()> {
        self.inner.append(entries)
    }

    fn truncate_suffix(&self, from_index: u64) -> RaftStoreResult<()> {
        self.inner.truncate_suffix(from_index)
    }

    fn save_snapshot(
        &self,
        snapshot: &Snapshot,
        preserve_log_entries: usize,
    ) -> RaftStoreResult<()> {
        self.inner.save_snapshot(snapshot, preserve_log_entries)
    }

    fn compact_to(&self, index: u64) -> RaftStoreResult<()> {
        self.inner.compact_to(index)
    }
}
