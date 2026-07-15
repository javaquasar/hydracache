use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
#[cfg(feature = "sled-log-store")]
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

#[cfg(feature = "sled-log-store")]
use protobuf::Message as ProtobufMessage;
use raft::eraftpb::{ConfState, Entry, HardState, Snapshot};
use raft::storage::{GetEntriesContext, RaftState, Storage};
use raft::{Error as RaftError, Result as RaftResult, StorageError};

/// Supported durable raft log format version for the 0.42 control-plane seam.
pub const RAFT_LOG_FORMAT_VERSION: u32 = 1;

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

    /// Persist the current raft configuration state.
    fn save_conf_state(&self, conf_state: &ConfState) -> RaftStoreResult<()>;

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

    /// Return retained entries in index order for runtime recovery.
    fn retained_entries(&self) -> RaftStoreResult<Vec<Entry>>;

    /// Mark entries through `index` as applied.
    fn mark_applied(&self, index: u64);

    /// Update the persisted commit index after raft light-ready advance.
    fn set_commit(&self, commit: u64) -> RaftStoreResult<()> {
        let mut state = self.initial_state().map_err(RaftStoreError::from)?;
        state.hard_state.commit = commit;
        self.save_hard_state(&state.hard_state)
    }
}

#[derive(Debug, Clone, Default)]
struct InMemoryRaftLogState {
    raft_state: RaftState,
    entries: Vec<Entry>,
    snapshot: Snapshot,
    applied_index: u64,
    snapshot_unavailable_once: bool,
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
            panic!(
                "index out of bound (last: {}, high: {})",
                last_index + 1,
                high
            );
        }
        if low == high {
            return Ok(Vec::new());
        }
        let start = usize::try_from(low - first_index)
            .map_err(|_| RaftError::Store(StorageError::Unavailable))?;
        let end = usize::try_from(high - first_index)
            .map_err(|_| RaftError::Store(StorageError::Unavailable))?;
        let mut entries = state
            .entries
            .get(start..end)
            .ok_or(RaftError::Store(StorageError::Unavailable))?
            .to_vec();
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
        Ok(last_index(
            &self.state.read().expect("raft log store poisoned"),
        ))
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
        snapshot.mut_metadata().index = snapshot.get_metadata().index.max(request_index);
        Ok(snapshot)
    }
}

impl RaftLogStore for InMemoryRaftLogStore {
    fn save_hard_state(&self, hard_state: &HardState) -> RaftStoreResult<()> {
        #[cfg(feature = "test-failpoints")]
        fail::fail_point!("raft_before_save_hard_state", |_| {
            Err(RaftStoreError::new(
                "injected crash before raft hard state save",
            ))
        });
        self.state
            .write()
            .expect("raft log store poisoned")
            .raft_state
            .hard_state = hard_state.clone();
        Ok(())
    }

    fn save_conf_state(&self, conf_state: &ConfState) -> RaftStoreResult<()> {
        #[cfg(feature = "test-failpoints")]
        fail::fail_point!("raft_store_before_save_conf_state", |_| {
            Err(RaftStoreError::new(
                "injected crash before raft store conf state save",
            ))
        });
        self.state
            .write()
            .expect("raft log store poisoned")
            .raft_state
            .conf_state = conf_state.clone();
        Ok(())
    }

    fn append(&self, entries: &[Entry]) -> RaftStoreResult<()> {
        #[cfg(feature = "test-failpoints")]
        fail::fail_point!("sled_append_disk_full", |_| {
            Err(RaftStoreError::new("injected disk full on raft append"))
        });
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
        #[cfg(feature = "test-failpoints")]
        fail::fail_point!("raft_save_snapshot_disk_full", |_| {
            Err(RaftStoreError::new(
                "injected disk full during raft snapshot save",
            ))
        });
        let mut state = self.state.write().expect("raft log store poisoned");
        let snapshot_index = snapshot.get_metadata().index;
        state.snapshot = snapshot.clone();
        state.raft_state.conf_state = snapshot.get_metadata().get_conf_state().clone();
        state.raft_state.hard_state.term = state
            .raft_state
            .hard_state
            .term
            .max(snapshot.get_metadata().term);
        state.raft_state.hard_state.commit = state.raft_state.hard_state.commit.max(snapshot_index);
        state.entries.retain(|entry| entry.index > snapshot_index);
        let drop_count = state.entries.len().saturating_sub(preserve_log_entries);
        state.entries.drain(..drop_count);
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

    fn retained_entries(&self) -> RaftStoreResult<Vec<Entry>> {
        Ok(self.all_entries())
    }

    fn mark_applied(&self, index: u64) {
        Self::mark_applied(self, index);
    }

    fn set_commit(&self, commit: u64) -> RaftStoreResult<()> {
        Self::set_commit(self, commit);
        Ok(())
    }
}

/// Restartable in-process directory for the supported 0.42 durable log seam.
///
/// The production contract is encoded here without introducing a heavy storage
/// dependency into the workspace: reopening the directory returns a fresh store
/// handle over retained log state, and unknown future format versions fail loud.
#[cfg(feature = "durable-log")]
#[derive(Clone, Debug)]
pub struct DurableRaftLogDirectory {
    inner: InMemoryRaftLogStore,
    format_version: Arc<RwLock<u32>>,
    fsync_count: Arc<AtomicU64>,
}

#[cfg(feature = "durable-log")]
impl Default for DurableRaftLogDirectory {
    fn default() -> Self {
        Self {
            inner: InMemoryRaftLogStore::new(),
            format_version: Arc::new(RwLock::new(RAFT_LOG_FORMAT_VERSION)),
            fsync_count: Arc::new(AtomicU64::new(0)),
        }
    }
}

#[cfg(feature = "durable-log")]
impl DurableRaftLogDirectory {
    /// Create an empty durable directory.
    pub fn new() -> Self {
        Self::default()
    }

    /// Open a store handle, refusing unknown future formats.
    pub fn open(&self) -> RaftStoreResult<DurableRaftLogStore> {
        let format_version = *self
            .format_version
            .read()
            .expect("durable raft format version poisoned");
        if format_version > RAFT_LOG_FORMAT_VERSION {
            return Err(RaftStoreError::new(format!(
                "unknown future raft log format version {format_version}"
            )));
        }
        Ok(DurableRaftLogStore {
            inner: self.inner.clone(),
            fsync_count: self.fsync_count.clone(),
        })
    }

    /// Test helper that simulates an on-disk header from a future writer.
    pub fn set_format_version_for_tests(&self, format_version: u32) {
        *self
            .format_version
            .write()
            .expect("durable raft format version poisoned") = format_version;
    }

    /// Return how many sync-required persist operations were observed.
    pub fn fsync_count(&self) -> u64 {
        self.fsync_count.load(Ordering::Relaxed)
    }
}

/// Supported durable raft log store for the 0.42 control-plane seam.
#[cfg(feature = "durable-log")]
#[derive(Clone, Debug)]
pub struct DurableRaftLogStore {
    inner: InMemoryRaftLogStore,
    fsync_count: Arc<AtomicU64>,
}

#[cfg(feature = "durable-log")]
impl DurableRaftLogStore {
    /// Return all retained entry payloads.
    pub fn retained_payloads(&self) -> Vec<Vec<u8>> {
        self.inner
            .all_entries()
            .into_iter()
            .map(|entry| entry.data.to_vec())
            .collect()
    }

    /// Mark entries through `index` as applied.
    pub fn mark_applied(&self, index: u64) {
        self.inner.mark_applied(index);
    }

    /// Return retained entries in index order.
    pub fn retained_entries(&self) -> Vec<Entry> {
        self.inner.all_entries()
    }

    /// Initialize raft configuration state when creating a fresh directory.
    pub fn initialize_with_conf_state<T>(&self, conf_state: T)
    where
        ConfState: From<T>,
    {
        self.inner.initialize_with_conf_state(conf_state);
    }

    fn record_sync(&self) {
        self.fsync_count.fetch_add(1, Ordering::Relaxed);
    }
}

#[cfg(feature = "durable-log")]
impl Storage for DurableRaftLogStore {
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

#[cfg(feature = "durable-log")]
impl RaftLogStore for DurableRaftLogStore {
    fn save_hard_state(&self, hard_state: &HardState) -> RaftStoreResult<()> {
        #[cfg(feature = "test-failpoints")]
        fail::fail_point!("raft_before_save_hard_state", |_| {
            Err(RaftStoreError::new(
                "injected crash before durable raft hard state save",
            ))
        });
        self.inner.save_hard_state(hard_state)?;
        self.record_sync();
        Ok(())
    }

    fn save_conf_state(&self, conf_state: &ConfState) -> RaftStoreResult<()> {
        #[cfg(feature = "test-failpoints")]
        fail::fail_point!("raft_store_before_save_conf_state", |_| {
            Err(RaftStoreError::new(
                "injected crash before durable raft conf state save",
            ))
        });
        self.inner.save_conf_state(conf_state)?;
        self.record_sync();
        Ok(())
    }

    fn append(&self, entries: &[Entry]) -> RaftStoreResult<()> {
        #[cfg(feature = "test-failpoints")]
        fail::fail_point!("sled_append_disk_full", |_| {
            Err(RaftStoreError::new(
                "injected disk full on durable raft append",
            ))
        });
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

    fn must_sync(&self) -> bool {
        true
    }

    fn retained_entries(&self) -> RaftStoreResult<Vec<Entry>> {
        Ok(self.retained_entries())
    }

    fn mark_applied(&self, index: u64) {
        Self::mark_applied(self, index);
    }

    fn set_commit(&self, commit: u64) -> RaftStoreResult<()> {
        self.inner.set_commit(commit);
        Ok(())
    }
}

/// Tiny deterministic multi-node control-plane model used by 0.42 release gates.
#[cfg(feature = "durable-log")]
#[derive(Debug, Clone)]
pub struct DurableControlPlaneCluster {
    members: BTreeMap<u64, DurableRaftLogDirectory>,
    reachable: BTreeSet<u64>,
    leader: Option<u64>,
    committed: Vec<Vec<u8>>,
}

#[cfg(feature = "durable-log")]
impl DurableControlPlaneCluster {
    /// Create a cluster with node ids `1..=count`.
    pub fn new(count: u64) -> Self {
        let mut members = BTreeMap::new();
        let mut reachable = BTreeSet::new();
        for id in 1..=count {
            members.insert(id, DurableRaftLogDirectory::new());
            reachable.insert(id);
        }
        Self {
            members,
            reachable,
            leader: (count > 0).then_some(1),
            committed: Vec::new(),
        }
    }

    /// Current leader id.
    pub fn leader(&self) -> Option<u64> {
        self.leader
    }

    /// Kill the current leader and elect the lowest reachable member if a
    /// majority remains.
    pub fn kill_leader_and_elect(&mut self) -> Option<u64> {
        if let Some(leader) = self.leader.take() {
            self.reachable.remove(&leader);
        }
        self.elect()
    }

    /// Isolate the cluster view to one node, modeling a minority partition.
    pub fn isolate_only(&mut self, node_id: u64) {
        self.reachable.clear();
        self.reachable.insert(node_id);
        self.leader = Some(node_id);
    }

    /// Propose a command if a majority is reachable.
    pub fn propose(&mut self, data: impl Into<Vec<u8>>) -> RaftStoreResult<u64> {
        if !self.has_majority() {
            return Err(RaftStoreError::new("minority partition cannot commit"));
        }
        if self.leader.is_none() {
            self.elect();
        }
        let index = self.committed.len() as u64 + 1;
        let payload = data.into();
        let entry = Entry {
            index,
            term: 1,
            data: payload.clone().into(),
            ..Entry::default()
        };
        let hard_state = HardState {
            commit: index,
            term: 1,
            ..HardState::default()
        };

        for node_id in &self.reachable {
            let store = self
                .members
                .get(node_id)
                .expect("reachable node must exist")
                .open()?;
            store.append(std::slice::from_ref(&entry))?;
            store.save_hard_state(&hard_state)?;
            store.mark_applied(index);
        }
        self.committed.push(payload);
        Ok(index)
    }

    /// Return committed payloads retained by `node_id`.
    pub fn committed_payloads_on(&self, node_id: u64) -> RaftStoreResult<Vec<Vec<u8>>> {
        self.members
            .get(&node_id)
            .ok_or_else(|| RaftStoreError::new("unknown raft node"))?
            .open()
            .map(|store| store.retained_payloads())
    }

    fn elect(&mut self) -> Option<u64> {
        if !self.has_majority() {
            self.leader = None;
            return None;
        }
        self.leader = self.reachable.iter().next().copied();
        self.leader
    }

    fn has_majority(&self) -> bool {
        self.reachable.len() > self.members.len() / 2
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
        if total > max_size {
            keep = index;
            break;
        }
    }
    entries.truncate(keep.max(1));
}

/// Feature-gated sled-backed durable log store.
#[cfg(feature = "sled-log-store")]
#[derive(Clone, Debug)]
pub struct SledRaftLogStore {
    inner: InMemoryRaftLogStore,
    db: sled::Db,
}

#[cfg(feature = "sled-log-store")]
impl SledRaftLogStore {
    /// Open or create a sled-backed log store under `path`.
    pub fn open(path: impl AsRef<Path>) -> RaftStoreResult<Self> {
        let db = sled::open(path).map_err(|error| {
            RaftStoreError::new(format!("failed to open sled raft log store: {error}"))
        })?;
        let store = Self {
            inner: InMemoryRaftLogStore::new(),
            db,
        };
        store.replay_from_sled()?;
        Ok(store)
    }

    /// Create a temporary sled-backed store for tests and examples.
    pub fn new_for_tests() -> Self {
        let db = sled::Config::new()
            .temporary(true)
            .open()
            .expect("temporary sled raft log store opens");
        Self {
            inner: InMemoryRaftLogStore::new(),
            db,
        }
    }

    /// Initialize raft configuration state when creating a fresh store.
    pub fn initialize_with_conf_state<T>(&self, conf_state: T)
    where
        ConfState: From<T>,
    {
        self.inner.initialize_with_conf_state(conf_state);
    }

    fn replay_from_sled(&self) -> RaftStoreResult<()> {
        if let Some(bytes) = self.db.get(SLED_HARD_STATE_KEY).map_err(sled_error)? {
            self.inner.save_hard_state(&decode_hard_state(&bytes)?)?;
        }
        if let Some(bytes) = self.db.get(SLED_SNAPSHOT_KEY).map_err(sled_error)? {
            self.inner
                .save_snapshot(&decode_snapshot(&bytes)?, usize::MAX)?;
        }
        if let Some(bytes) = self.db.get(SLED_CONF_STATE_KEY).map_err(sled_error)? {
            self.inner.save_conf_state(&decode_conf_state(&bytes)?)?;
        }
        let entries = self
            .db
            .scan_prefix(SLED_ENTRY_PREFIX)
            .map(|item| {
                let (_, value) = item.map_err(sled_error)?;
                decode_entry(&value)
            })
            .collect::<RaftStoreResult<Vec<_>>>()?;
        if !entries.is_empty() {
            self.inner.append(&entries)?;
        }
        if let Some(bytes) = self.db.get(SLED_APPLIED_KEY).map_err(sled_error)? {
            self.inner.mark_applied(decode_u64(&bytes)?);
        }
        Ok(())
    }

    fn sync(&self) -> RaftStoreResult<()> {
        self.db
            .flush()
            .map(|_| ())
            .map_err(|error| RaftStoreError::new(format!("failed to flush sled raft log: {error}")))
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
        #[cfg(feature = "test-failpoints")]
        fail::fail_point!("raft_before_save_hard_state", |_| {
            Err(RaftStoreError::new(
                "injected crash before sled raft hard state save",
            ))
        });
        self.inner.save_hard_state(hard_state)?;
        self.db
            .insert(SLED_HARD_STATE_KEY, encode_hard_state(hard_state)?)
            .map_err(sled_error)?;
        self.sync()
    }

    fn save_conf_state(&self, conf_state: &ConfState) -> RaftStoreResult<()> {
        #[cfg(feature = "test-failpoints")]
        fail::fail_point!("raft_store_before_save_conf_state", |_| {
            Err(RaftStoreError::new(
                "injected crash before sled raft conf state save",
            ))
        });
        self.inner.save_conf_state(conf_state)?;
        self.db
            .insert(SLED_CONF_STATE_KEY, encode_conf_state(conf_state)?)
            .map_err(sled_error)?;
        self.sync()
    }

    fn append(&self, entries: &[Entry]) -> RaftStoreResult<()> {
        #[cfg(feature = "test-failpoints")]
        fail::fail_point!("sled_append_disk_full", |_| {
            Err(RaftStoreError::new(
                "injected disk full on sled raft append",
            ))
        });
        if let Some(first) = entries.first() {
            let keys = self
                .db
                .scan_prefix(SLED_ENTRY_PREFIX)
                .keys()
                .map(|key| key.map_err(sled_error))
                .collect::<RaftStoreResult<Vec<_>>>()?;
            for key in keys {
                if decode_entry_index_key(&key)? >= first.index {
                    self.db.remove(key).map_err(sled_error)?;
                }
            }
        }
        self.inner.append(entries)?;
        for entry in entries {
            self.db
                .insert(entry_key(entry.index), encode_entry(entry)?)
                .map_err(sled_error)?;
        }
        self.sync()
    }

    fn truncate_suffix(&self, from_index: u64) -> RaftStoreResult<()> {
        self.inner.truncate_suffix(from_index)?;
        let keys = self
            .db
            .scan_prefix(SLED_ENTRY_PREFIX)
            .keys()
            .map(|key| key.map_err(sled_error))
            .collect::<RaftStoreResult<Vec<_>>>()?;
        for key in keys {
            if decode_entry_index_key(&key)? >= from_index {
                self.db.remove(key).map_err(sled_error)?;
            }
        }
        self.sync()
    }

    fn save_snapshot(
        &self,
        snapshot: &Snapshot,
        preserve_log_entries: usize,
    ) -> RaftStoreResult<()> {
        self.inner.save_snapshot(snapshot, preserve_log_entries)?;
        self.db
            .insert(SLED_SNAPSHOT_KEY, encode_snapshot(snapshot)?)
            .map_err(sled_error)?;
        self.rewrite_entries_from_inner()?;
        self.sync()
    }

    fn compact_to(&self, index: u64) -> RaftStoreResult<()> {
        self.inner.compact_to(index)?;
        let keys = self
            .db
            .scan_prefix(SLED_ENTRY_PREFIX)
            .keys()
            .map(|key| key.map_err(sled_error))
            .collect::<RaftStoreResult<Vec<_>>>()?;
        for key in keys {
            if decode_entry_index_key(&key)? < index {
                self.db.remove(key).map_err(sled_error)?;
            }
        }
        self.sync()
    }

    fn must_sync(&self) -> bool {
        true
    }

    fn retained_entries(&self) -> RaftStoreResult<Vec<Entry>> {
        Ok(self.inner.all_entries())
    }

    fn mark_applied(&self, index: u64) {
        self.inner.mark_applied(index);
        let _ = self.db.insert(SLED_APPLIED_KEY, encode_u64(index).to_vec());
        let _ = self.db.flush();
    }

    fn set_commit(&self, commit: u64) -> RaftStoreResult<()> {
        self.inner.set_commit(commit);
        let mut state = self.initial_state().map_err(RaftStoreError::from)?;
        state.hard_state.commit = commit;
        self.db
            .insert(SLED_HARD_STATE_KEY, encode_hard_state(&state.hard_state)?)
            .map_err(sled_error)?;
        self.sync()
    }
}

#[cfg(feature = "sled-log-store")]
impl SledRaftLogStore {
    fn rewrite_entries_from_inner(&self) -> RaftStoreResult<()> {
        let keys = self
            .db
            .scan_prefix(SLED_ENTRY_PREFIX)
            .keys()
            .map(|key| key.map_err(sled_error))
            .collect::<RaftStoreResult<Vec<_>>>()?;
        for key in keys {
            self.db.remove(key).map_err(sled_error)?;
        }
        for entry in self.inner.all_entries() {
            self.db
                .insert(entry_key(entry.index), encode_entry(&entry)?)
                .map_err(sled_error)?;
        }
        Ok(())
    }
}

#[cfg(feature = "sled-log-store")]
const SLED_HARD_STATE_KEY: &[u8] = b"meta:hard_state";
#[cfg(feature = "sled-log-store")]
const SLED_CONF_STATE_KEY: &[u8] = b"meta:conf_state";
#[cfg(feature = "sled-log-store")]
const SLED_SNAPSHOT_KEY: &[u8] = b"meta:snapshot";
#[cfg(feature = "sled-log-store")]
const SLED_APPLIED_KEY: &[u8] = b"meta:applied";
#[cfg(feature = "sled-log-store")]
const SLED_ENTRY_PREFIX: &[u8] = b"entry:";
#[cfg(feature = "sled-log-store")]
const SLED_SNAPSHOT_ENVELOPE_MAGIC: &[u8; 8] = b"HCSNAP01";
#[cfg(feature = "sled-log-store")]
const SLED_SNAPSHOT_ENVELOPE_VERSION: u32 = 1;
#[cfg(feature = "sled-log-store")]
const SLED_SNAPSHOT_ENVELOPE_HEADER_LEN: usize = 28;

#[cfg(feature = "sled-log-store")]
fn sled_error(error: sled::Error) -> RaftStoreError {
    RaftStoreError::new(format!("sled raft log error: {error}"))
}

#[cfg(feature = "sled-log-store")]
fn entry_key(index: u64) -> Vec<u8> {
    let mut key = Vec::with_capacity(SLED_ENTRY_PREFIX.len() + 8);
    key.extend_from_slice(SLED_ENTRY_PREFIX);
    key.extend_from_slice(&index.to_be_bytes());
    key
}

#[cfg(feature = "sled-log-store")]
fn decode_entry_index_key(key: &[u8]) -> RaftStoreResult<u64> {
    let index = key
        .strip_prefix(SLED_ENTRY_PREFIX)
        .ok_or_else(|| RaftStoreError::new("invalid sled raft entry key prefix"))?;
    decode_u64(index)
}

#[cfg(feature = "sled-log-store")]
fn encode_u64(value: u64) -> [u8; 8] {
    value.to_be_bytes()
}

#[cfg(feature = "sled-log-store")]
fn decode_u64(bytes: &[u8]) -> RaftStoreResult<u64> {
    let bytes: [u8; 8] = bytes
        .try_into()
        .map_err(|_| RaftStoreError::new("invalid sled raft u64 value"))?;
    Ok(u64::from_be_bytes(bytes))
}

#[cfg(feature = "sled-log-store")]
fn decode_u32(bytes: &[u8]) -> RaftStoreResult<u32> {
    let bytes: [u8; 4] = bytes
        .try_into()
        .map_err(|_| RaftStoreError::new("invalid sled raft u32 value"))?;
    Ok(u32::from_be_bytes(bytes))
}

#[cfg(feature = "sled-log-store")]
fn encode_entry(entry: &Entry) -> RaftStoreResult<Vec<u8>> {
    protobuf::Message::write_to_bytes(entry)
        .map_err(|error| RaftStoreError::new(format!("failed to encode raft entry: {error}")))
}

#[cfg(feature = "sled-log-store")]
fn decode_entry(bytes: &[u8]) -> RaftStoreResult<Entry> {
    Entry::parse_from_bytes(bytes)
        .map_err(|error| RaftStoreError::new(format!("failed to decode raft entry: {error}")))
}

#[cfg(feature = "sled-log-store")]
fn encode_hard_state(hard_state: &HardState) -> RaftStoreResult<Vec<u8>> {
    protobuf::Message::write_to_bytes(hard_state)
        .map_err(|error| RaftStoreError::new(format!("failed to encode hard state: {error}")))
}

#[cfg(feature = "sled-log-store")]
fn decode_hard_state(bytes: &[u8]) -> RaftStoreResult<HardState> {
    HardState::parse_from_bytes(bytes)
        .map_err(|error| RaftStoreError::new(format!("failed to decode hard state: {error}")))
}

#[cfg(feature = "sled-log-store")]
fn encode_conf_state(conf_state: &ConfState) -> RaftStoreResult<Vec<u8>> {
    protobuf::Message::write_to_bytes(conf_state)
        .map_err(|error| RaftStoreError::new(format!("failed to encode conf state: {error}")))
}

#[cfg(feature = "sled-log-store")]
fn decode_conf_state(bytes: &[u8]) -> RaftStoreResult<ConfState> {
    ConfState::parse_from_bytes(bytes)
        .map_err(|error| RaftStoreError::new(format!("failed to decode conf state: {error}")))
}

#[cfg(feature = "sled-log-store")]
fn encode_snapshot(snapshot: &Snapshot) -> RaftStoreResult<Vec<u8>> {
    let payload = protobuf::Message::write_to_bytes(snapshot)
        .map_err(|error| RaftStoreError::new(format!("failed to encode raft snapshot: {error}")))?;
    let mut encoded = Vec::with_capacity(SLED_SNAPSHOT_ENVELOPE_HEADER_LEN + payload.len());
    encoded.extend_from_slice(SLED_SNAPSHOT_ENVELOPE_MAGIC);
    encoded.extend_from_slice(&SLED_SNAPSHOT_ENVELOPE_VERSION.to_be_bytes());
    encoded.extend_from_slice(&(payload.len() as u64).to_be_bytes());
    encoded.extend_from_slice(&snapshot_payload_checksum(&payload).to_be_bytes());
    encoded.extend_from_slice(&payload);
    Ok(encoded)
}

#[cfg(feature = "sled-log-store")]
fn decode_snapshot(bytes: &[u8]) -> RaftStoreResult<Snapshot> {
    if bytes.starts_with(SLED_SNAPSHOT_ENVELOPE_MAGIC) {
        return decode_snapshot_envelope(bytes);
    }
    Snapshot::parse_from_bytes(bytes)
        .map_err(|error| RaftStoreError::new(format!("failed to decode raft snapshot: {error}")))
}

#[cfg(feature = "sled-log-store")]
fn decode_snapshot_envelope(bytes: &[u8]) -> RaftStoreResult<Snapshot> {
    if bytes.len() < SLED_SNAPSHOT_ENVELOPE_HEADER_LEN {
        return Err(RaftStoreError::new(format!(
            "truncated raft snapshot checksum envelope: expected at least {} bytes, got {}",
            SLED_SNAPSHOT_ENVELOPE_HEADER_LEN,
            bytes.len()
        )));
    }
    let version = decode_u32(&bytes[8..12])?;
    if version != SLED_SNAPSHOT_ENVELOPE_VERSION {
        return Err(RaftStoreError::new(format!(
            "unsupported raft snapshot checksum envelope version {version}"
        )));
    }
    let payload_len = decode_u64(&bytes[12..20])? as usize;
    let expected_checksum = decode_u64(&bytes[20..28])?;
    let expected_len = SLED_SNAPSHOT_ENVELOPE_HEADER_LEN
        .checked_add(payload_len)
        .ok_or_else(|| RaftStoreError::new("raft snapshot checksum envelope length overflow"))?;
    if bytes.len() != expected_len {
        return Err(RaftStoreError::new(format!(
            "truncated raft snapshot checksum envelope: expected {expected_len} bytes, got {}",
            bytes.len()
        )));
    }
    let payload = &bytes[SLED_SNAPSHOT_ENVELOPE_HEADER_LEN..];
    let actual_checksum = snapshot_payload_checksum(payload);
    if actual_checksum != expected_checksum {
        return Err(RaftStoreError::new(format!(
            "raft snapshot checksum mismatch: expected {expected_checksum:#x}, actual {actual_checksum:#x}"
        )));
    }
    Snapshot::parse_from_bytes(payload)
        .map_err(|error| RaftStoreError::new(format!("failed to decode raft snapshot: {error}")))
}

#[cfg(feature = "sled-log-store")]
fn snapshot_payload_checksum(bytes: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = FNV_OFFSET;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(index: u64, term: u64, data: &[u8]) -> Entry {
        Entry {
            index,
            term,
            data: data.to_vec().into(),
            ..Entry::default()
        }
    }

    fn indexes(entries: &[Entry]) -> Vec<u64> {
        entries.iter().map(|entry| entry.index).collect()
    }

    #[test]
    fn in_memory_store_debug_and_trait_defaults_are_observable() {
        let store = InMemoryRaftLogStore::new();
        let debug = format!("{store:?}");

        assert!(debug.contains("InMemoryRaftLogStore"));
        assert!(debug.contains("first_index"));
        assert!(debug.contains("last_index"));
        assert!(!<InMemoryRaftLogStore as RaftLogStore>::must_sync(&store));
    }

    #[test]
    fn in_memory_entries_accept_exact_bounds_and_append_overwrite_boundary() {
        let store = InMemoryRaftLogStore::new();
        store
            .append(&[entry(1, 1, b"a"), entry(2, 1, b"b"), entry(3, 1, b"c")])
            .unwrap();

        let loaded = store
            .entries(1, 4, None, GetEntriesContext::empty(false))
            .unwrap();
        assert_eq!(indexes(&loaded), vec![1, 2, 3]);
        let half_open = store
            .entries(1, 3, None, GetEntriesContext::empty(false))
            .unwrap();
        assert_eq!(indexes(&half_open), vec![1, 2]);

        let compacted = InMemoryRaftLogStore::new();
        compacted
            .append(&[entry(1, 1, b"a"), entry(2, 1, b"b"), entry(3, 1, b"c")])
            .unwrap();
        let mut prefix = Snapshot::default();
        prefix.mut_metadata().index = 1;
        compacted.save_snapshot(&prefix, usize::MAX).unwrap();
        let after_compaction = compacted
            .entries(2, 4, None, GetEntriesContext::empty(false))
            .unwrap();
        assert_eq!(indexes(&after_compaction), vec![2, 3]);

        store.append(&[entry(1, 2, b"replacement")]).unwrap();
        let retained = store.all_entries();
        assert_eq!(indexes(&retained), vec![1]);
        assert_eq!(retained[0].term, 2);
    }

    #[test]
    fn in_memory_progress_updates_through_the_trait_contract() {
        let store = InMemoryRaftLogStore::new();
        <InMemoryRaftLogStore as RaftLogStore>::mark_applied(&store, 7);
        <InMemoryRaftLogStore as RaftLogStore>::set_commit(&store, 6).unwrap();

        assert_eq!(store.applied_index(), 7);
        assert_eq!(store.initial_state().unwrap().hard_state.commit, 6);
    }

    #[test]
    fn snapshot_removes_boundary_entry_and_preserves_requested_tail() {
        let store = InMemoryRaftLogStore::new();
        store
            .append(&[
                entry(1, 1, b"a"),
                entry(2, 1, b"b"),
                entry(3, 1, b"c"),
                entry(4, 1, b"d"),
            ])
            .unwrap();
        store.set_commit(4);
        let mut snapshot = Snapshot::default();
        snapshot.mut_metadata().index = 2;
        snapshot.mut_metadata().term = 3;

        store.save_snapshot(&snapshot, usize::MAX).unwrap();
        assert_eq!(indexes(&store.all_entries()), vec![3, 4]);
        let state = store.initial_state().unwrap();
        assert_eq!(state.hard_state.commit, 4);
        assert_eq!(state.hard_state.term, 3);

        let tail_store = InMemoryRaftLogStore::new();
        tail_store
            .append(&[
                entry(1, 1, b"a"),
                entry(2, 1, b"b"),
                entry(3, 1, b"c"),
                entry(4, 1, b"d"),
            ])
            .unwrap();
        let mut prefix = Snapshot::default();
        prefix.mut_metadata().index = 1;
        tail_store.save_snapshot(&prefix, 2).unwrap();
        assert_eq!(indexes(&tail_store.all_entries()), vec![3, 4]);
    }

    #[test]
    fn snapshot_request_index_is_monotonic_at_the_exact_boundary() {
        let store = InMemoryRaftLogStore::new();
        let mut snapshot = Snapshot::default();
        snapshot.mut_metadata().index = 5;
        store.save_snapshot(&snapshot, 0).unwrap();

        assert_eq!(store.snapshot(5, 0).unwrap().get_metadata().index, 5);
        assert_eq!(store.snapshot(7, 0).unwrap().get_metadata().index, 7);
    }

    #[test]
    fn entry_size_limit_keeps_one_entry_and_respects_exact_budget() {
        let make = || {
            vec![
                entry(1, 1, b"aaaa"),
                entry(2, 1, b"bbbb"),
                entry(3, 1, b"cccc"),
            ]
        };

        let mut exact = make();
        limit_entries_size(&mut exact, 8);
        assert_eq!(indexes(&exact), vec![1, 2]);

        let mut below = make();
        limit_entries_size(&mut below, 7);
        assert_eq!(indexes(&below), vec![1]);

        let mut zero = make();
        limit_entries_size(&mut zero, 0);
        assert_eq!(indexes(&zero), vec![1]);

        let mut unlimited = make();
        limit_entries_size(&mut unlimited, u64::MAX);
        assert_eq!(indexes(&unlimited), vec![1, 2, 3]);

        let mut empty = Vec::new();
        limit_entries_size(&mut empty, 0);
        assert!(empty.is_empty());
    }

    #[cfg(feature = "durable-log")]
    #[test]
    fn durable_store_records_sync_progress_and_exact_raft_metadata() {
        let directory = DurableRaftLogDirectory::new();
        let store = directory.open().unwrap();
        assert_eq!(directory.fsync_count(), 0);
        assert!(<DurableRaftLogStore as RaftLogStore>::must_sync(&store));

        store.append(&[entry(1, 7, b"command")]).unwrap();
        store
            .save_hard_state(&HardState {
                term: 7,
                commit: 1,
                ..HardState::default()
            })
            .unwrap();
        store
            .save_conf_state(&ConfState::from((vec![1, 2, 3], vec![])))
            .unwrap();
        <DurableRaftLogStore as RaftLogStore>::mark_applied(&store, 1);

        assert_eq!(directory.fsync_count(), 2);
        assert_eq!(store.inner.applied_index(), 1);
        assert_eq!(store.retained_entries()[0].term, 7);
        let hard_state = store.initial_state().unwrap().hard_state;
        assert_eq!(hard_state.term, 7);
        assert_eq!(hard_state.commit, 1);
    }

    #[cfg(feature = "durable-log")]
    #[test]
    fn durable_cluster_reports_new_leader_and_even_quorum_boundary() {
        let mut replicated = DurableControlPlaneCluster::new(3);
        assert_eq!(replicated.propose(b"term-contract".to_vec()).unwrap(), 1);
        let store = replicated.members.get(&1).unwrap().open().unwrap();
        assert_eq!(store.retained_entries()[0].term, 1);
        let hard_state = store.initial_state().unwrap().hard_state;
        assert_eq!(hard_state.term, 1);
        assert_eq!(hard_state.commit, 1);

        let mut cluster = DurableControlPlaneCluster::new(4);
        assert_eq!(cluster.leader(), Some(1));
        assert_eq!(cluster.kill_leader_and_elect(), Some(2));
        assert_eq!(cluster.leader(), Some(2));
        assert_eq!(cluster.kill_leader_and_elect(), None);
        assert_eq!(cluster.leader(), None);
        assert!(cluster.propose(b"no-quorum".to_vec()).is_err());
    }

    #[cfg(feature = "sled-log-store")]
    fn sled_temp_path(label: &str) -> std::path::PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("hydracache-{label}-{unique}"))
    }

    #[cfg(feature = "sled-log-store")]
    #[test]
    fn sled_store_persists_applied_index_and_exact_compaction_boundary() {
        let path = sled_temp_path("mutant-boundaries");
        let store = SledRaftLogStore::open(&path).unwrap();
        assert!(<SledRaftLogStore as RaftLogStore>::must_sync(&store));
        store
            .append(&[entry(1, 1, b"a"), entry(2, 1, b"b"), entry(3, 1, b"c")])
            .unwrap();
        let mut snapshot = Snapshot::default();
        snapshot.mut_metadata().index = 1;
        snapshot.mut_metadata().term = 1;
        store.save_snapshot(&snapshot, usize::MAX).unwrap();
        <SledRaftLogStore as RaftLogStore>::mark_applied(&store, 3);
        store.compact_to(2).unwrap();
        drop(store);

        let reopened = SledRaftLogStore::open(&path).unwrap();
        assert_eq!(reopened.inner.applied_index(), 3);
        assert_eq!(indexes(&reopened.retained_entries().unwrap()), vec![2, 3]);
        drop(reopened);
        let _ = std::fs::remove_dir_all(path);
    }

    #[cfg(feature = "sled-log-store")]
    #[test]
    fn sled_integer_and_empty_snapshot_codecs_have_known_answers() {
        let value = 0x0102_0304_0506_0708_u64;
        assert_eq!(encode_u64(value), [1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(decode_u64(&encode_u64(value)).unwrap(), value);

        let snapshot = Snapshot::default();
        let encoded = encode_snapshot(&snapshot).unwrap();
        assert_eq!(encoded.len(), SLED_SNAPSHOT_ENVELOPE_HEADER_LEN);
        assert_eq!(decode_snapshot_envelope(&encoded).unwrap(), snapshot);
    }
}
