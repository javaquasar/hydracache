use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use hydracache_core::CacheCodec;
use hydracache_db::{
    CollectedInvalidationReport, CommitPosition, DbCache, InvalidationCollector,
    SqlxInvalidationOutbox,
};
use sqlx::{Sqlite, SqlitePool, Transaction};

use crate::{SqlxTransactionError, TransactionResult};

/// Boxed future returned by SQLx transaction companion closures.
pub type SqlxTransactionFuture<'tx, E> =
    Pin<Box<dyn Future<Output = std::result::Result<(), E>> + Send + 'tx>>;

/// Extension trait that creates SQLx transaction companions from [`DbCache`].
pub trait SqlxTransactionExt<C>
where
    C: CacheCodec,
{
    /// Create a transaction companion using this database cache namespace.
    fn sqlx_transactions(&self) -> SqlxTransactionCompanion<C>;
}

impl<C> SqlxTransactionExt<C> for DbCache<C>
where
    C: CacheCodec,
{
    fn sqlx_transactions(&self) -> SqlxTransactionCompanion<C> {
        SqlxTransactionCompanion::new(self.clone())
    }
}

/// SQLx transaction companion.
///
/// The companion begins a transaction, gives user code explicit access to the
/// SQLx transaction and an invalidation collector, then either enqueues durable
/// outbox rows before commit or applies local-only invalidation after commit.
#[derive(Debug)]
pub struct SqlxTransactionCompanion<C = hydracache::PostcardCodec>
where
    C: CacheCodec,
{
    queries: DbCache<C>,
    outbox: Option<SqlxInvalidationOutbox>,
    counters: Arc<SqlxTransactionCounters>,
}

impl<C> Clone for SqlxTransactionCompanion<C>
where
    C: CacheCodec,
{
    fn clone(&self) -> Self {
        Self {
            queries: self.queries.clone(),
            outbox: self.outbox.clone(),
            counters: self.counters.clone(),
        }
    }
}

impl<C> SqlxTransactionCompanion<C>
where
    C: CacheCodec,
{
    /// Create a companion for the given query cache namespace.
    pub fn new(queries: DbCache<C>) -> Self {
        Self {
            queries,
            outbox: None,
            counters: Arc::new(SqlxTransactionCounters::default()),
        }
    }

    /// Attach a durable SQLx invalidation outbox.
    pub fn with_outbox(mut self, outbox: SqlxInvalidationOutbox) -> Self {
        self.outbox = Some(outbox);
        self
    }

    /// Return transaction companion diagnostics.
    pub fn diagnostics(&self) -> SqlxTransactionDiagnostics {
        self.counters.snapshot()
    }

    /// Execute a SQLite transaction and enqueue invalidation intent before commit.
    pub async fn sqlite_durable<F, E>(
        &self,
        pool: &SqlitePool,
        reason: impl Into<String>,
        body: F,
    ) -> TransactionResult<SqlxTransactionReport, E>
    where
        F: for<'tx> FnOnce(
                &'tx mut Transaction<'_, Sqlite>,
                &'tx mut InvalidationCollector,
            ) -> SqlxTransactionFuture<'tx, E>
            + Send,
        E: Error + Send + Sync + 'static,
    {
        let Some(outbox) = self.outbox.as_ref() else {
            self.counters
                .enqueue_failures
                .fetch_add(1, Ordering::Relaxed);
            return Err(SqlxTransactionError::MissingOutbox);
        };

        let mut tx = pool.begin().await.map_err(SqlxTransactionError::Sqlx)?;
        let mut collector = InvalidationCollector::new(self.queries.namespace(), reason);

        if let Err(error) = body(&mut tx, &mut collector).await {
            self.counters.body_errors.fetch_add(1, Ordering::Relaxed);
            rollback_and_count(tx, &self.counters).await;
            return Err(SqlxTransactionError::Body(error));
        }

        let collected = collector.into_collected();
        let intent_count = collected.len();
        let inserted = if collected.is_empty() {
            0
        } else {
            match outbox
                .enqueue_in_sqlite_tx(
                    &mut tx,
                    collected.namespace(),
                    &sqlite_commit_position(),
                    collected.batch(),
                )
                .await
            {
                Ok(inserted) => inserted,
                Err(error) => {
                    self.counters
                        .enqueue_failures
                        .fetch_add(1, Ordering::Relaxed);
                    rollback_and_count(tx, &self.counters).await;
                    return Err(SqlxTransactionError::Outbox(error));
                }
            }
        };

        match tx.commit().await {
            Ok(()) => {
                self.counters.commits.fetch_add(1, Ordering::Relaxed);
                Ok(SqlxTransactionReport {
                    intent_count,
                    durable_rows: inserted,
                    local_report: None,
                })
            }
            Err(error) => {
                self.counters
                    .commit_failures
                    .fetch_add(1, Ordering::Relaxed);
                Err(SqlxTransactionError::Sqlx(error))
            }
        }
    }

    /// Execute a SQLite transaction and apply invalidation directly after commit.
    ///
    /// This mode is intentionally non-durable. It is useful for local demos and
    /// single-process tests, while production writes should use
    /// [`SqlxTransactionCompanion::sqlite_durable`].
    pub async fn sqlite_local<F, E>(
        &self,
        pool: &SqlitePool,
        reason: impl Into<String>,
        body: F,
    ) -> TransactionResult<SqlxTransactionReport, E>
    where
        F: for<'tx> FnOnce(
                &'tx mut Transaction<'_, Sqlite>,
                &'tx mut InvalidationCollector,
            ) -> SqlxTransactionFuture<'tx, E>
            + Send,
        E: Error + Send + Sync + 'static,
    {
        let mut tx = pool.begin().await.map_err(SqlxTransactionError::Sqlx)?;
        let mut collector = InvalidationCollector::new(self.queries.namespace(), reason);

        if let Err(error) = body(&mut tx, &mut collector).await {
            self.counters.body_errors.fetch_add(1, Ordering::Relaxed);
            rollback_and_count(tx, &self.counters).await;
            return Err(SqlxTransactionError::Body(error));
        }

        let collected = collector.into_collected();
        let intent_count = collected.len();

        match tx.commit().await {
            Ok(()) => {
                self.counters.commits.fetch_add(1, Ordering::Relaxed);
            }
            Err(error) => {
                self.counters
                    .commit_failures
                    .fetch_add(1, Ordering::Relaxed);
                return Err(SqlxTransactionError::Sqlx(error));
            }
        }

        let local_report = collected
            .execute_local(self.queries.cache())
            .await
            .map_err(SqlxTransactionError::LocalInvalidation)?;
        self.counters
            .local_invalidations
            .fetch_add(1, Ordering::Relaxed);

        Ok(SqlxTransactionReport {
            intent_count,
            durable_rows: 0,
            local_report: Some(local_report),
        })
    }
}

/// Result of a SQLx transaction companion run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SqlxTransactionReport {
    /// Number of invalidation intents collected by user code.
    pub intent_count: usize,
    /// Number of durable outbox rows inserted.
    pub durable_rows: usize,
    /// Local invalidation report for non-durable mode.
    pub local_report: Option<CollectedInvalidationReport>,
}

/// Diagnostics for SQLx transaction companion runs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SqlxTransactionDiagnostics {
    /// Successful transaction commits.
    pub commits: u64,
    /// Rollbacks attempted after body/enqueue failures.
    pub rollbacks: u64,
    /// User closure failures.
    pub body_errors: u64,
    /// Outbox enqueue failures.
    pub enqueue_failures: u64,
    /// Commit failures after a successful user body.
    pub commit_failures: u64,
    /// Local non-durable invalidation applications.
    pub local_invalidations: u64,
}

#[derive(Debug, Default)]
struct SqlxTransactionCounters {
    commits: AtomicU64,
    rollbacks: AtomicU64,
    body_errors: AtomicU64,
    enqueue_failures: AtomicU64,
    commit_failures: AtomicU64,
    local_invalidations: AtomicU64,
}

impl SqlxTransactionCounters {
    fn snapshot(&self) -> SqlxTransactionDiagnostics {
        SqlxTransactionDiagnostics {
            commits: self.commits.load(Ordering::Relaxed),
            rollbacks: self.rollbacks.load(Ordering::Relaxed),
            body_errors: self.body_errors.load(Ordering::Relaxed),
            enqueue_failures: self.enqueue_failures.load(Ordering::Relaxed),
            commit_failures: self.commit_failures.load(Ordering::Relaxed),
            local_invalidations: self.local_invalidations.load(Ordering::Relaxed),
        }
    }
}

async fn rollback_and_count(tx: Transaction<'_, Sqlite>, counters: &SqlxTransactionCounters) {
    if tx.rollback().await.is_ok() {
        counters.rollbacks.fetch_add(1, Ordering::Relaxed);
    }
}

fn sqlite_commit_position() -> CommitPosition {
    static NEXT: AtomicU64 = AtomicU64::new(1);

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    CommitPosition::new(format!("sqlite:{now}:{sequence}"))
}
