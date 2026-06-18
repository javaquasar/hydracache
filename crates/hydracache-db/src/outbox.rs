use std::collections::BTreeMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use sha2::{Digest, Sha256};

use hydracache::{CacheInvalidation, CacheKeyBuilder, HydraCache};
use hydracache_core::CacheCodec;

use crate::{DbCacheError, Result};

/// Stable SHA-256 hash of a normalized invalidation target.
pub type InvalidationTargetHash = [u8; 32];

/// Normalized, transport-neutral invalidation intent.
///
/// The intent deliberately carries no cached value. It is suitable for durable
/// database outboxes, trigger-written rows, and transport wake-ups.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InvalidationIntent {
    /// Invalidate one physical cache key.
    Key {
        /// Physical cache key.
        key: String,
    },
    /// Invalidate all entries associated with one tag.
    Tag {
        /// Invalidation tag.
        tag: String,
    },
    /// Invalidate the entity tag built from an entity kind and id/key.
    Entity {
        /// Entity name/kind.
        entity: String,
        /// Entity id/key segment.
        key: String,
    },
    /// Invalidate a collection tag.
    Collection {
        /// Collection name.
        collection: String,
    },
    /// Flush the whole cache.
    Flush,
}

impl InvalidationIntent {
    /// Create a key invalidation intent.
    pub fn key(key: impl Into<String>) -> Self {
        Self::Key { key: key.into() }
    }

    /// Create a tag invalidation intent.
    pub fn tag(tag: impl Into<String>) -> Self {
        Self::Tag { tag: tag.into() }
    }

    /// Create an entity invalidation intent.
    pub fn entity(entity: impl Into<String>, key: impl Into<String>) -> Self {
        Self::Entity {
            entity: entity.into(),
            key: key.into(),
        }
    }

    /// Create a collection invalidation intent.
    pub fn collection(collection: impl Into<String>) -> Self {
        Self::Collection {
            collection: collection.into(),
        }
    }

    /// Create a cache-wide flush intent.
    pub fn flush() -> Self {
        Self::Flush
    }

    /// Return the stable wire/storage kind for this intent.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Key { .. } => "key",
            Self::Tag { .. } => "tag",
            Self::Entity { .. } => "entity",
            Self::Collection { .. } => "collection",
            Self::Flush => "flush",
        }
    }

    /// Return the primary cache key/tag value stored in the outbox row.
    pub fn value(&self) -> Option<&str> {
        match self {
            Self::Key { key } | Self::Entity { key, .. } => Some(key),
            Self::Tag { tag } => Some(tag),
            Self::Collection { collection } => Some(collection),
            Self::Flush => None,
        }
    }

    /// Stable content hash used in the outbox idempotency key.
    ///
    /// The hash input is length-prefixed and includes the intent kind, so values
    /// containing `:`, `/`, whitespace, or empty strings cannot collide through
    /// delimiter ambiguity.
    pub fn target_hash(&self) -> InvalidationTargetHash {
        let mut hasher = Sha256::new();
        write_hash_part(&mut hasher, b"hydracache-invalidation-intent-v1");
        write_hash_part(&mut hasher, self.kind().as_bytes());

        match self {
            Self::Key { key } => write_hash_part(&mut hasher, key.as_bytes()),
            Self::Tag { tag } => write_hash_part(&mut hasher, tag.as_bytes()),
            Self::Entity { entity, key } => {
                write_hash_part(&mut hasher, entity.as_bytes());
                write_hash_part(&mut hasher, key.as_bytes());
            }
            Self::Collection { collection } => {
                write_hash_part(&mut hasher, collection.as_bytes());
            }
            Self::Flush => {}
        }

        hasher.finalize().into()
    }

    /// Hex representation of [`InvalidationIntent::target_hash`].
    pub fn target_hash_hex(&self) -> String {
        hex_encode(&self.target_hash())
    }

    /// Map this intent onto HydraCache's existing cross-process invalidation
    /// operation.
    pub fn to_cache_invalidation(&self) -> CacheInvalidation {
        match self {
            Self::Key { key } => CacheInvalidation::key(key.clone()),
            Self::Tag { tag } => CacheInvalidation::tag(tag.clone()),
            Self::Entity { entity, key } => CacheInvalidation::tag(entity_tag(entity, key)),
            Self::Collection { collection } => CacheInvalidation::tag(collection_tag(collection)),
            Self::Flush => CacheInvalidation::flush(),
        }
    }
}

/// Ordered batch of invalidation intent rows to persist with a data write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidationIntentBatch {
    reason: String,
    intents: Vec<InvalidationIntent>,
}

impl InvalidationIntentBatch {
    /// Create an empty batch with an operator-facing reason.
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
            intents: Vec::new(),
        }
    }

    /// Return the reason attached to every outbox row in this batch.
    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// Return the intents in insertion order.
    pub fn intents(&self) -> &[InvalidationIntent] {
        &self.intents
    }

    /// Return whether no intents have been added.
    pub fn is_empty(&self) -> bool {
        self.intents.is_empty()
    }

    /// Return the number of intents in this batch.
    pub fn len(&self) -> usize {
        self.intents.len()
    }

    /// Add an already-built intent.
    pub fn intent(mut self, intent: InvalidationIntent) -> Self {
        self.intents.push(intent);
        self
    }

    /// Add a key invalidation.
    pub fn invalidate_key(self, key: impl Into<String>) -> Self {
        self.intent(InvalidationIntent::key(key))
    }

    /// Add a tag invalidation.
    pub fn invalidate_tag(self, tag: impl Into<String>) -> Self {
        self.intent(InvalidationIntent::tag(tag))
    }

    /// Add an entity invalidation.
    pub fn invalidate_entity(self, entity: impl Into<String>, key: impl Into<String>) -> Self {
        self.intent(InvalidationIntent::entity(entity, key))
    }

    /// Add a collection invalidation.
    pub fn invalidate_collection(self, collection: impl Into<String>) -> Self {
        self.intent(InvalidationIntent::collection(collection))
    }

    /// Add a cache-wide flush invalidation.
    pub fn flush(self) -> Self {
        self.intent(InvalidationIntent::flush())
    }
}

impl Default for InvalidationIntentBatch {
    fn default() -> Self {
        Self::new("")
    }
}

/// Identity of a committed write used to build outbox idempotency keys.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommitPosition(String);

impl CommitPosition {
    /// Create a commit position from a database txid, LSN, or monotonic fallback.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Return the string representation persisted in the outbox table.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume this value into its owned string.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for CommitPosition {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for CommitPosition {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Durable lifecycle state of one outbox row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OutboxState {
    /// Row is eligible for claim once `available_at_ms` is reached.
    Pending,
    /// Row was applied and marked published.
    Published,
    /// Row exceeded the retry budget and needs operator attention.
    Dead,
}

impl OutboxState {
    /// Return the stable storage representation.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Published => "published",
            Self::Dead => "dead",
        }
    }

    /// Parse a storage representation.
    pub fn from_storage(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "published" => Some(Self::Published),
            "dead" => Some(Self::Dead),
            _ => None,
        }
    }
}

/// One durable invalidation intent row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboxRow {
    /// Stable row id.
    pub id: String,
    /// Cache namespace this row belongs to.
    pub namespace: String,
    /// Database commit identity used for idempotency.
    pub commit_position: CommitPosition,
    /// Hex SHA-256 of the normalized target.
    pub target_hash: String,
    /// Invalidation intent to apply.
    pub intent: InvalidationIntent,
    /// Operator-facing write reason.
    pub reason: String,
    /// Creation timestamp in Unix milliseconds.
    pub created_at_ms: u64,
    /// Earliest claim timestamp in Unix milliseconds.
    pub available_at_ms: u64,
    /// Last claim timestamp in Unix milliseconds.
    pub claimed_at_ms: Option<u64>,
    /// Last claim owner.
    pub claim_owner: Option<String>,
    /// Publish timestamp in Unix milliseconds.
    pub published_at_ms: Option<u64>,
    /// Failed publish attempts.
    pub attempts: u32,
    /// Current durable state.
    pub state: OutboxState,
    /// Last publish/apply error.
    pub last_error: Option<String>,
}

impl OutboxRow {
    /// Return whether this row has been marked published.
    pub fn is_published(&self) -> bool {
        self.state == OutboxState::Published
    }

    /// Return whether this row is dead-lettered.
    pub fn is_dead_lettered(&self) -> bool {
        self.state == OutboxState::Dead
    }
}

/// Read-only worker/backlog status.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OutboxStatus {
    /// Pending rows in the namespace.
    pub pending: u64,
    /// Age of the oldest pending row.
    pub oldest_pending_age_ms: u64,
    /// Dead-lettered rows in the namespace.
    pub dead_lettered: u64,
    /// Most recent publish timestamp in the namespace.
    pub last_published_at_ms: Option<u64>,
    /// Sum of failed publish attempts in the namespace.
    pub failed_attempts: u64,
}

/// Cumulative in-process outbox worker counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OutboxWorkerDiagnostics {
    /// Completed worker iterations.
    pub iterations: u64,
    /// Rows claimed by the worker.
    pub claimed: u64,
    /// Rows successfully published.
    pub published: u64,
    /// Rows returned to pending with backoff.
    pub retried: u64,
    /// Rows moved to dead-letter state.
    pub dead_lettered: u64,
}

#[derive(Debug, Default)]
struct OutboxWorkerCounters {
    iterations: AtomicU64,
    claimed: AtomicU64,
    published: AtomicU64,
    retried: AtomicU64,
    dead_lettered: AtomicU64,
}

impl OutboxWorkerCounters {
    fn snapshot(&self) -> OutboxWorkerDiagnostics {
        OutboxWorkerDiagnostics {
            iterations: self.iterations.load(Ordering::Relaxed),
            claimed: self.claimed.load(Ordering::Relaxed),
            published: self.published.load(Ordering::Relaxed),
            retried: self.retried.load(Ordering::Relaxed),
            dead_lettered: self.dead_lettered.load(Ordering::Relaxed),
        }
    }

    fn record(&self, report: OutboxPublishReport) {
        self.iterations.fetch_add(1, Ordering::Relaxed);
        self.claimed
            .fetch_add(report.claimed as u64, Ordering::Relaxed);
        self.published
            .fetch_add(report.published as u64, Ordering::Relaxed);
        self.retried
            .fetch_add(report.retried as u64, Ordering::Relaxed);
        self.dead_lettered
            .fetch_add(report.dead_lettered as u64, Ordering::Relaxed);
    }
}

/// Read-after-write wait mode for durable invalidation outbox users.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ConsistencyMode {
    /// Do not wait for local invalidation publishing.
    NoWait,
    /// Wait for the local namespace outbox to drain.
    Local,
    /// Wait locally until timeout, then report degraded instead of hiding it.
    BestEffort,
}

/// Receipt returned by write paths after enqueueing durable invalidation intent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidationReceipt {
    namespace: String,
    commit_position: CommitPosition,
    created_at_ms: u64,
}

impl InvalidationReceipt {
    /// Create a receipt for a committed write.
    pub fn new(namespace: impl Into<String>, commit_position: CommitPosition) -> Self {
        Self {
            namespace: namespace.into(),
            commit_position,
            created_at_ms: now_ms(),
        }
    }

    /// Cache namespace of the invalidation write.
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Database commit identity associated with this invalidation write.
    pub fn commit_position(&self) -> &CommitPosition {
        &self.commit_position
    }

    /// Receipt creation timestamp in Unix milliseconds.
    pub fn created_at_ms(&self) -> u64 {
        self.created_at_ms
    }
}

/// Outcome of a read-after-write invalidation wait.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidationWaitOutcome {
    /// Requested wait mode.
    pub mode: ConsistencyMode,
    /// Whether the outbox was drained before returning.
    pub satisfied: bool,
    /// Whether the caller should treat the result as degraded.
    pub degraded: bool,
    /// Whether waiting stopped because the timeout elapsed.
    pub timed_out: bool,
    /// Pending rows still visible when the wait completed.
    pub pending: u64,
    /// Elapsed wait time in milliseconds.
    pub elapsed_ms: u64,
}

/// Cumulative in-process read-after-write wait counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InvalidationWaitDiagnostics {
    /// Wait calls.
    pub waits: u64,
    /// Waits that observed a drained outbox.
    pub satisfied: u64,
    /// Waits that elapsed their timeout.
    pub timed_out: u64,
    /// Waits that returned a degraded outcome.
    pub degraded: u64,
}

#[derive(Debug, Default)]
struct InvalidationWaitCounters {
    waits: AtomicU64,
    satisfied: AtomicU64,
    timed_out: AtomicU64,
    degraded: AtomicU64,
}

impl InvalidationWaitCounters {
    fn snapshot(&self) -> InvalidationWaitDiagnostics {
        InvalidationWaitDiagnostics {
            waits: self.waits.load(Ordering::Relaxed),
            satisfied: self.satisfied.load(Ordering::Relaxed),
            timed_out: self.timed_out.load(Ordering::Relaxed),
            degraded: self.degraded.load(Ordering::Relaxed),
        }
    }

    fn record(&self, outcome: InvalidationWaitOutcome) {
        self.waits.fetch_add(1, Ordering::Relaxed);
        if outcome.satisfied {
            self.satisfied.fetch_add(1, Ordering::Relaxed);
        }
        if outcome.timed_out {
            self.timed_out.fetch_add(1, Ordering::Relaxed);
        }
        if outcome.degraded {
            self.degraded.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// Read-after-write helper that waits for durable invalidation publishing.
#[derive(Debug, Clone)]
pub struct InvalidationWait {
    mode: ConsistencyMode,
    timeout: Duration,
    poll_interval: Duration,
    counters: Arc<InvalidationWaitCounters>,
}

impl InvalidationWait {
    /// Create a wait helper that returns immediately.
    pub fn no_wait() -> Self {
        Self {
            mode: ConsistencyMode::NoWait,
            timeout: Duration::ZERO,
            poll_interval: Duration::from_millis(10),
            counters: Arc::default(),
        }
    }

    /// Create a local wait helper with a timeout.
    pub fn local(timeout: Duration) -> Self {
        Self {
            mode: ConsistencyMode::Local,
            timeout,
            poll_interval: Duration::from_millis(10),
            counters: Arc::default(),
        }
    }

    /// Create a best-effort wait helper with a timeout.
    pub fn best_effort(timeout: Duration) -> Self {
        Self {
            mode: ConsistencyMode::BestEffort,
            timeout,
            poll_interval: Duration::from_millis(10),
            counters: Arc::default(),
        }
    }

    /// Override polling interval.
    pub fn poll_interval(mut self, poll_interval: Duration) -> Self {
        self.poll_interval = poll_interval;
        self
    }

    /// Return configured wait mode.
    pub fn mode(&self) -> ConsistencyMode {
        self.mode
    }

    /// Return cumulative wait diagnostics.
    pub fn diagnostics(&self) -> InvalidationWaitDiagnostics {
        self.counters.snapshot()
    }

    /// Wait until the namespace outbox is drained, timeout is reached, or mode
    /// is [`ConsistencyMode::NoWait`].
    pub async fn wait<O>(
        &self,
        outbox: &O,
        receipt: &InvalidationReceipt,
    ) -> Result<InvalidationWaitOutcome>
    where
        O: InvalidationOutbox,
    {
        let start = tokio::time::Instant::now();

        if self.mode == ConsistencyMode::NoWait {
            let outcome = InvalidationWaitOutcome {
                mode: self.mode,
                satisfied: true,
                degraded: false,
                timed_out: false,
                pending: 0,
                elapsed_ms: elapsed_ms(start),
            };
            self.counters.record(outcome);
            return Ok(outcome);
        }

        loop {
            let status = outbox.status(receipt.namespace()).await?;
            if status.pending == 0 {
                let outcome = InvalidationWaitOutcome {
                    mode: self.mode,
                    satisfied: true,
                    degraded: false,
                    timed_out: false,
                    pending: 0,
                    elapsed_ms: elapsed_ms(start),
                };
                self.counters.record(outcome);
                return Ok(outcome);
            }

            let elapsed = start.elapsed();
            if elapsed >= self.timeout {
                let outcome = InvalidationWaitOutcome {
                    mode: self.mode,
                    satisfied: false,
                    degraded: true,
                    timed_out: true,
                    pending: status.pending,
                    elapsed_ms: duration_ms(elapsed),
                };
                self.counters.record(outcome);
                return Ok(outcome);
            }

            let remaining = self.timeout.saturating_sub(elapsed);
            tokio::time::sleep(self.poll_interval.min(remaining)).await;
        }
    }
}

/// Durable outbox storage abstraction.
#[async_trait]
pub trait InvalidationOutbox: fmt::Debug + Send + Sync + 'static {
    /// Enqueue a batch for a namespace and commit position.
    async fn enqueue(
        &self,
        namespace: &str,
        commit_position: &CommitPosition,
        batch: &InvalidationIntentBatch,
    ) -> Result<usize>;

    /// Claim up to `limit` rows for a worker owner.
    async fn claim(
        &self,
        namespace: &str,
        owner: &str,
        limit: usize,
        claim_ttl: Duration,
    ) -> Result<Vec<OutboxRow>>;

    /// Mark rows published after invalidation was applied.
    async fn mark_published(&self, ids: &[String]) -> Result<()>;

    /// Mark one row failed, with retry backoff or dead-letter state.
    async fn mark_failed(&self, id: &str, error: &str, backoff: Duration, dead: bool)
        -> Result<()>;

    /// Re-enable all dead-lettered rows for a namespace.
    async fn reset_dead_letters(&self, namespace: &str) -> Result<u64>;

    /// Read-only status snapshot for operators.
    async fn status(&self, namespace: &str) -> Result<OutboxStatus>;
}

/// In-memory outbox adapter for tests, demos, and custom-adapter examples.
#[derive(Clone, Default)]
pub struct InMemoryInvalidationOutbox {
    inner: Arc<Mutex<InMemoryOutboxInner>>,
}

impl InMemoryInvalidationOutbox {
    /// Create an empty in-memory outbox.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return one row by id.
    pub fn row(&self, id: &str) -> Option<OutboxRow> {
        self.inner.lock().ok()?.rows.get(id).cloned()
    }

    /// Return all rows for diagnostics and tests.
    pub fn rows(&self) -> Vec<OutboxRow> {
        self.inner
            .lock()
            .map(|inner| inner.rows.values().cloned().collect())
            .unwrap_or_default()
    }
}

impl fmt::Debug for InMemoryInvalidationOutbox {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InMemoryInvalidationOutbox")
            .field("rows", &self.rows().len())
            .finish()
    }
}

#[derive(Default)]
struct InMemoryOutboxInner {
    rows: BTreeMap<String, OutboxRow>,
}

#[async_trait]
impl InvalidationOutbox for InMemoryInvalidationOutbox {
    async fn enqueue(
        &self,
        namespace: &str,
        commit_position: &CommitPosition,
        batch: &InvalidationIntentBatch,
    ) -> Result<usize> {
        let mut inner = self.lock_inner()?;
        let now = now_ms();
        let mut inserted = 0;

        for intent in batch.intents() {
            let target_hash = intent.target_hash_hex();
            let id = outbox_row_id(namespace, commit_position.as_str(), &target_hash);
            if inner.rows.contains_key(&id) {
                continue;
            }

            inner.rows.insert(
                id.clone(),
                OutboxRow {
                    id,
                    namespace: namespace.to_owned(),
                    commit_position: commit_position.clone(),
                    target_hash,
                    intent: intent.clone(),
                    reason: batch.reason().to_owned(),
                    created_at_ms: now,
                    available_at_ms: now,
                    claimed_at_ms: None,
                    claim_owner: None,
                    published_at_ms: None,
                    attempts: 0,
                    state: OutboxState::Pending,
                    last_error: None,
                },
            );
            inserted += 1;
        }

        Ok(inserted)
    }

    async fn claim(
        &self,
        namespace: &str,
        owner: &str,
        limit: usize,
        claim_ttl: Duration,
    ) -> Result<Vec<OutboxRow>> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let mut inner = self.lock_inner()?;
        let now = now_ms();
        let claim_ttl_ms = duration_ms(claim_ttl);
        let mut candidates = inner
            .rows
            .values()
            .filter(|row| {
                row.namespace == namespace
                    && row.state == OutboxState::Pending
                    && row.available_at_ms <= now
                    && claim_is_available(row, now, claim_ttl_ms)
            })
            .map(|row| (row.available_at_ms, row.created_at_ms, row.id.clone()))
            .collect::<Vec<_>>();
        candidates.sort();

        let ids = candidates
            .into_iter()
            .take(limit)
            .map(|(_, _, id)| id)
            .collect::<Vec<_>>();
        let mut claimed = Vec::with_capacity(ids.len());

        for id in ids {
            if let Some(row) = inner.rows.get_mut(&id) {
                row.claimed_at_ms = Some(now);
                row.claim_owner = Some(owner.to_owned());
                claimed.push(row.clone());
            }
        }

        Ok(claimed)
    }

    async fn mark_published(&self, ids: &[String]) -> Result<()> {
        let mut inner = self.lock_inner()?;
        let now = now_ms();

        for id in ids {
            let Some(row) = inner.rows.get_mut(id) else {
                continue;
            };
            row.state = OutboxState::Published;
            row.published_at_ms = Some(now);
            row.claimed_at_ms = None;
            row.claim_owner = None;
            row.last_error = None;
        }

        Ok(())
    }

    async fn mark_failed(
        &self,
        id: &str,
        error: &str,
        backoff: Duration,
        dead: bool,
    ) -> Result<()> {
        let mut inner = self.lock_inner()?;
        let Some(row) = inner.rows.get_mut(id) else {
            return Ok(());
        };
        let now = now_ms();

        row.attempts = row.attempts.saturating_add(1);
        row.last_error = Some(error.to_owned());
        row.claimed_at_ms = None;
        row.claim_owner = None;
        if dead {
            row.state = OutboxState::Dead;
        } else {
            row.state = OutboxState::Pending;
            row.available_at_ms = now.saturating_add(duration_ms(backoff));
        }

        Ok(())
    }

    async fn reset_dead_letters(&self, namespace: &str) -> Result<u64> {
        let mut inner = self.lock_inner()?;
        let now = now_ms();
        let mut reset = 0;

        for row in inner.rows.values_mut() {
            if row.namespace == namespace && row.state == OutboxState::Dead {
                row.state = OutboxState::Pending;
                row.available_at_ms = now;
                row.claimed_at_ms = None;
                row.claim_owner = None;
                row.attempts = 0;
                row.last_error = None;
                reset += 1;
            }
        }

        Ok(reset)
    }

    async fn status(&self, namespace: &str) -> Result<OutboxStatus> {
        let inner = self.lock_inner()?;
        let now = now_ms();
        let mut status = OutboxStatus::default();
        let mut oldest_pending = None::<u64>;

        for row in inner.rows.values().filter(|row| row.namespace == namespace) {
            status.failed_attempts += u64::from(row.attempts);
            match row.state {
                OutboxState::Pending => {
                    status.pending += 1;
                    oldest_pending = Some(
                        oldest_pending
                            .map_or(row.created_at_ms, |oldest| oldest.min(row.created_at_ms)),
                    );
                }
                OutboxState::Published => {
                    status.last_published_at_ms =
                        match (status.last_published_at_ms, row.published_at_ms) {
                            (Some(current), Some(candidate)) => Some(current.max(candidate)),
                            (None, Some(candidate)) => Some(candidate),
                            (current, None) => current,
                        };
                }
                OutboxState::Dead => {
                    status.dead_lettered += 1;
                }
            }
        }

        status.oldest_pending_age_ms = oldest_pending
            .map(|created_at| now.saturating_sub(created_at))
            .unwrap_or_default();

        Ok(status)
    }
}

impl InMemoryInvalidationOutbox {
    fn lock_inner(&self) -> Result<std::sync::MutexGuard<'_, InMemoryOutboxInner>> {
        self.inner
            .lock()
            .map_err(|_| backend_error("in-memory invalidation outbox mutex was poisoned"))
    }
}

/// Applies one outbox invalidation intent.
#[async_trait]
pub trait InvalidationApplier: Send + Sync + 'static {
    /// Apply an invalidation intent after it has been claimed.
    async fn apply_invalidation(&self, intent: &InvalidationIntent) -> hydracache::CacheResult<()>;
}

#[async_trait]
impl<C> InvalidationApplier for HydraCache<C>
where
    C: CacheCodec,
{
    async fn apply_invalidation(&self, intent: &InvalidationIntent) -> hydracache::CacheResult<()> {
        match intent.to_cache_invalidation() {
            CacheInvalidation::Key { key } => {
                self.invalidate_key(&key).await?;
            }
            CacheInvalidation::Tag { tag } => {
                self.invalidate_tag(&tag).await?;
            }
            CacheInvalidation::Flush => {
                self.flush().await?;
            }
        }
        Ok(())
    }
}

/// Result of one outbox worker drain iteration.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OutboxPublishReport {
    /// Rows claimed for this iteration.
    pub claimed: usize,
    /// Rows successfully applied and marked published.
    pub published: usize,
    /// Rows returned to pending with backoff.
    pub retried: usize,
    /// Rows moved to dead-letter state.
    pub dead_lettered: usize,
}

/// Circuit-breaker drain worker for invalidation outbox rows.
#[derive(Debug, Clone)]
pub struct InvalidationOutboxWorker<O, A> {
    outbox: O,
    applier: A,
    namespace: String,
    owner: String,
    batch_size: usize,
    claim_ttl: Duration,
    backoff: Duration,
    max_attempts: u32,
    counters: Arc<OutboxWorkerCounters>,
}

impl<O, A> InvalidationOutboxWorker<O, A> {
    /// Create a worker with conservative defaults.
    pub fn new(outbox: O, applier: A, namespace: impl Into<String>) -> Self {
        Self {
            outbox,
            applier,
            namespace: namespace.into(),
            owner: "hydracache-outbox-worker".to_owned(),
            batch_size: 64,
            claim_ttl: Duration::from_secs(30),
            backoff: Duration::from_secs(1),
            max_attempts: 5,
            counters: Arc::default(),
        }
    }

    /// Override the claim owner written to rows.
    pub fn owner(mut self, owner: impl Into<String>) -> Self {
        self.owner = owner.into();
        self
    }

    /// Override the maximum rows claimed in one iteration.
    pub fn batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size.max(1);
        self
    }

    /// Override the claim timeout used to recover abandoned claims.
    pub fn claim_ttl(mut self, claim_ttl: Duration) -> Self {
        self.claim_ttl = claim_ttl;
        self
    }

    /// Override retry backoff after an apply/publish failure.
    pub fn backoff(mut self, backoff: Duration) -> Self {
        self.backoff = backoff;
        self
    }

    /// Override maximum failed attempts before dead-lettering.
    pub fn max_attempts(mut self, max_attempts: u32) -> Self {
        self.max_attempts = max_attempts.max(1);
        self
    }

    /// Return cumulative in-process worker diagnostics.
    pub fn diagnostics(&self) -> OutboxWorkerDiagnostics {
        self.counters.snapshot()
    }

    /// Run one claim -> apply -> mark-published iteration.
    pub async fn run_once(&self) -> Result<OutboxPublishReport>
    where
        O: InvalidationOutbox,
        A: InvalidationApplier,
    {
        let rows = self
            .outbox
            .claim(
                &self.namespace,
                &self.owner,
                self.batch_size,
                self.claim_ttl,
            )
            .await?;
        let mut report = OutboxPublishReport {
            claimed: rows.len(),
            ..OutboxPublishReport::default()
        };

        for row in rows {
            match self.applier.apply_invalidation(&row.intent).await {
                Ok(()) => {
                    self.outbox
                        .mark_published(std::slice::from_ref(&row.id))
                        .await?;
                    report.published += 1;
                }
                Err(error) => {
                    let dead = row.attempts.saturating_add(1) >= self.max_attempts;
                    self.outbox
                        .mark_failed(&row.id, &error.to_string(), self.backoff, dead)
                        .await?;
                    if dead {
                        report.dead_lettered += 1;
                    } else {
                        report.retried += 1;
                    }
                }
            }
        }

        self.counters.record(report);
        Ok(report)
    }

    /// Operator reset; re-enables all dead-lettered rows for this namespace.
    pub async fn reset_dead_letters(&self) -> Result<u64>
    where
        O: InvalidationOutbox,
    {
        self.outbox.reset_dead_letters(&self.namespace).await
    }
}

fn write_hash_part(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

fn entity_tag(entity: &str, key: &str) -> String {
    CacheKeyBuilder::new()
        .segment(entity)
        .segment(key)
        .build_string()
}

fn collection_tag(collection: &str) -> String {
    CacheKeyBuilder::from_segment(collection).build_string()
}

fn hex_encode(bytes: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn outbox_row_id(namespace: &str, commit_position: &str, target_hash: &str) -> String {
    let mut hasher = Sha256::new();
    write_hash_part(&mut hasher, b"hydracache-outbox-row-id-v1");
    write_hash_part(&mut hasher, namespace.as_bytes());
    write_hash_part(&mut hasher, commit_position.as_bytes());
    write_hash_part(&mut hasher, target_hash.as_bytes());
    hex_encode(&hasher.finalize().into())
}

fn claim_is_available(row: &OutboxRow, now: u64, claim_ttl_ms: u64) -> bool {
    match row.claimed_at_ms {
        Some(claimed_at) => claimed_at.saturating_add(claim_ttl_ms) <= now,
        None => true,
    }
}

fn duration_ms(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

fn elapsed_ms(start: tokio::time::Instant) -> u64 {
    duration_ms(start.elapsed())
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn backend_error(message: impl Into<String>) -> DbCacheError {
    hydracache::CacheError::Backend(message.into()).into()
}

#[cfg(test)]
mod tests {
    use super::{
        CommitPosition, InMemoryInvalidationOutbox, InvalidationIntent, InvalidationIntentBatch,
        InvalidationOutbox, OutboxState,
    };

    #[test]
    fn intent_target_hash_is_stable() {
        let first = InvalidationIntent::entity("user", "42").target_hash();
        let second = InvalidationIntent::entity("user", "42").target_hash();

        assert_eq!(first, second);
        assert_eq!(
            InvalidationIntent::entity("user", "42")
                .target_hash_hex()
                .len(),
            64
        );
    }

    #[test]
    fn intent_target_hash_distinguishes_kind_and_length_prefixed_parts() {
        let key = InvalidationIntent::key("tenant:7/users");
        let tag = InvalidationIntent::tag("tenant:7/users");
        let entity = InvalidationIntent::entity("tenant:7", "users");
        let collection = InvalidationIntent::collection("tenant:7:users");

        assert_ne!(key.target_hash(), tag.target_hash());
        assert_ne!(key.target_hash(), entity.target_hash());
        assert_ne!(tag.target_hash(), collection.target_hash());
    }

    #[test]
    fn intent_to_cache_invalidation_maps_each_kind() {
        let key = InvalidationIntent::key("db:user:42").to_cache_invalidation();
        assert_eq!(key.key_value(), Some("db:user:42"));

        let tag = InvalidationIntent::tag("users").to_cache_invalidation();
        assert_eq!(tag.tag_value(), Some("users"));

        let entity = InvalidationIntent::entity("account:user", "42%beta").to_cache_invalidation();
        assert_eq!(entity.tag_value(), Some("account%3Auser:42%25beta"));

        let collection = InvalidationIntent::collection("users:active").to_cache_invalidation();
        assert_eq!(collection.tag_value(), Some("users%3Aactive"));

        assert!(InvalidationIntent::flush()
            .to_cache_invalidation()
            .is_flush());
    }

    #[test]
    fn intent_batch_preserves_reason_and_order() {
        let batch = InvalidationIntentBatch::new("user-write")
            .invalidate_key("db:user:42")
            .invalidate_tag("users")
            .invalidate_entity("user", "42")
            .invalidate_collection("users:active")
            .flush();

        assert_eq!(batch.reason(), "user-write");
        assert_eq!(batch.len(), 5);
        assert_eq!(batch.intents()[0].kind(), "key");
        assert_eq!(batch.intents()[4].kind(), "flush");
    }

    #[test]
    fn commit_position_wraps_database_identity() {
        let position = CommitPosition::new("pg:123");

        assert_eq!(position.as_str(), "pg:123");
        assert_eq!(position.clone().into_string(), "pg:123");
        assert_eq!(CommitPosition::from("pg:123"), position);
    }

    #[tokio::test]
    async fn in_memory_outbox_enqueue_is_idempotent_for_same_commit_and_target() {
        let outbox = InMemoryInvalidationOutbox::new();
        let commit = CommitPosition::new("sqlite:1");
        let batch = InvalidationIntentBatch::new("write").invalidate_tag("users");

        assert_eq!(outbox.enqueue("db", &commit, &batch).await.unwrap(), 1);
        assert_eq!(outbox.enqueue("db", &commit, &batch).await.unwrap(), 0);

        let rows = outbox.rows();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].state, OutboxState::Pending);
        assert_eq!(rows[0].namespace, "db");
    }
}
