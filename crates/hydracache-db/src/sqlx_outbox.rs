use std::fmt;
use std::time::Duration;

use async_trait::async_trait;
use sqlx::postgres::{PgListener, PgRow};
use sqlx::sqlite::SqliteRow;
use sqlx::{PgPool, Postgres, Row, Sqlite, SqlitePool, Transaction};

use crate::{
    CommitPosition, DbCacheError, InvalidationIntent, InvalidationIntentBatch, InvalidationOutbox,
    OutboxRow, OutboxState, OutboxStatus, Result,
};

/// Current durable schema version for `hydracache_invalidation_outbox`.
pub const OUTBOX_SCHEMA_VERSION: i64 = 1;

const SCHEMA_ARTIFACT: &str = "hydracache_invalidation_outbox";

/// One Postgres LISTEN/NOTIFY wake-up payload.
///
/// This is a latency hint only. Durable correctness must still come from the
/// invalidation outbox because Postgres notifications are at-most-once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgNotifyIntent {
    channel: String,
    payload: String,
}

impl PgNotifyIntent {
    /// Create a notification intent from channel and payload strings.
    pub fn new(channel: impl Into<String>, payload: impl Into<String>) -> Self {
        Self {
            channel: channel.into(),
            payload: payload.into(),
        }
    }

    /// Return the Postgres notification channel.
    pub fn channel(&self) -> &str {
        &self.channel
    }

    /// Return the Postgres notification payload.
    pub fn payload(&self) -> &str {
        &self.payload
    }
}

/// Thin sqlx `PgListener` wrapper for invalidation wake-ups.
///
/// The source intentionally does not apply invalidation by itself. It lets a
/// worker wake up and drain the durable outbox sooner; missed notifications are
/// recovered by the normal polling path.
pub struct PgNotifyIntentSource {
    listener: PgListener,
    channel: String,
}

impl PgNotifyIntentSource {
    /// Connect a listener and subscribe to one channel.
    pub async fn connect(database_url: &str, channel: &str) -> Result<Self> {
        let mut listener = PgListener::connect(database_url)
            .await
            .map_err(sqlx_error)?;
        listener.listen(channel).await.map_err(sqlx_error)?;
        Ok(Self {
            listener,
            channel: channel.to_owned(),
        })
    }

    /// Return the subscribed channel.
    pub fn channel(&self) -> &str {
        &self.channel
    }

    /// Receive the next notification intent.
    pub async fn recv(&mut self) -> Result<PgNotifyIntent> {
        let notification = self.listener.recv().await.map_err(sqlx_error)?;
        Ok(PgNotifyIntent::new(
            notification.channel(),
            notification.payload(),
        ))
    }
}

impl fmt::Debug for PgNotifyIntentSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PgNotifyIntentSource")
            .field("channel", &self.channel)
            .finish_non_exhaustive()
    }
}

/// SQLx-backed invalidation outbox.
#[derive(Clone)]
pub struct SqlxInvalidationOutbox {
    pool: SqlxOutboxPool,
}

#[derive(Clone)]
enum SqlxOutboxPool {
    Sqlite(SqlitePool),
    Postgres(PgPool),
}

impl SqlxInvalidationOutbox {
    /// Build a SQLite-backed outbox.
    pub fn sqlite(pool: SqlitePool) -> Self {
        Self {
            pool: SqlxOutboxPool::Sqlite(pool),
        }
    }

    /// Build a Postgres-backed outbox.
    pub fn postgres(pool: PgPool) -> Self {
        Self {
            pool: SqlxOutboxPool::Postgres(pool),
        }
    }

    /// Install the outbox schema if it is missing.
    ///
    /// Migrations are copyable and idempotent. Applications may run their own
    /// migration system instead; this helper exists for tests and small demos.
    pub async fn install_schema(&self) -> Result<()> {
        match &self.pool {
            SqlxOutboxPool::Sqlite(pool) => {
                for statement in SQLITE_SCHEMA {
                    sqlx::query(statement)
                        .execute(pool)
                        .await
                        .map_err(sqlx_error)?;
                }
            }
            SqlxOutboxPool::Postgres(pool) => {
                for statement in POSTGRES_SCHEMA {
                    sqlx::query(statement)
                        .execute(pool)
                        .await
                        .map_err(sqlx_error)?;
                }
            }
        }

        Ok(())
    }

    /// Refuse to operate against missing or unknown future schema versions.
    pub async fn check_schema(&self) -> Result<()> {
        let version = match &self.pool {
            SqlxOutboxPool::Sqlite(pool) => {
                sqlx::query("select version from hydracache_schema_version where artifact = ?")
                    .bind(SCHEMA_ARTIFACT)
                    .fetch_optional(pool)
                    .await
                    .map_err(sqlx_error)?
                    .map(|row| row.get::<i64, _>("version"))
            }
            SqlxOutboxPool::Postgres(pool) => {
                sqlx::query("select version from hydracache_schema_version where artifact = $1")
                    .bind(SCHEMA_ARTIFACT)
                    .fetch_optional(pool)
                    .await
                    .map_err(sqlx_error)?
                    .map(|row| row.get::<i64, _>("version"))
            }
        };

        match version {
            Some(OUTBOX_SCHEMA_VERSION) => Ok(()),
            Some(version) if version > OUTBOX_SCHEMA_VERSION => Err(backend_error(format!(
                "unknown future {SCHEMA_ARTIFACT} schema version {version}; supported version is {OUTBOX_SCHEMA_VERSION}"
            ))),
            Some(version) => Err(backend_error(format!(
                "unsupported {SCHEMA_ARTIFACT} schema version {version}; expected {OUTBOX_SCHEMA_VERSION}"
            ))),
            None => Err(backend_error(format!(
                "missing {SCHEMA_ARTIFACT} schema version row"
            ))),
        }
    }

    /// Enqueue intent inside an existing SQLite transaction.
    pub async fn enqueue_in_sqlite_tx(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        namespace: &str,
        commit_position: &CommitPosition,
        batch: &InvalidationIntentBatch,
    ) -> Result<usize> {
        match self.pool {
            SqlxOutboxPool::Sqlite(_) => {}
            SqlxOutboxPool::Postgres(_) => {
                return Err(backend_error(
                    "enqueue_in_sqlite_tx called on a Postgres outbox",
                ));
            }
        }

        let now = now_ms_i64();
        let mut inserted = 0;
        for intent in batch.intents() {
            let fields = IntentFields::from_intent(intent);
            let result = sqlx::query(SQLITE_INSERT_OUTBOX)
                .bind(outbox_row_id(
                    namespace,
                    commit_position.as_str(),
                    &fields.target_hash,
                ))
                .bind(namespace)
                .bind(commit_position.as_str())
                .bind(&fields.target_hash)
                .bind(fields.intent_kind)
                .bind(fields.cache_key)
                .bind(fields.cache_tag)
                .bind(fields.entity_name)
                .bind(fields.collection_name)
                .bind(batch.reason())
                .bind(now)
                .bind(now)
                .execute(&mut **tx)
                .await
                .map_err(sqlx_error)?;
            inserted += usize::try_from(result.rows_affected()).unwrap_or(usize::MAX);
        }
        Ok(inserted)
    }

    /// Enqueue intent inside an existing Postgres transaction.
    pub async fn enqueue_in_postgres_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        namespace: &str,
        commit_position: &CommitPosition,
        batch: &InvalidationIntentBatch,
    ) -> Result<usize> {
        match self.pool {
            SqlxOutboxPool::Postgres(_) => {}
            SqlxOutboxPool::Sqlite(_) => {
                return Err(backend_error(
                    "enqueue_in_postgres_tx called on a SQLite outbox",
                ));
            }
        }

        let now = now_ms_i64();
        let mut inserted = 0;
        for intent in batch.intents() {
            let fields = IntentFields::from_intent(intent);
            let result = sqlx::query(POSTGRES_INSERT_OUTBOX)
                .bind(outbox_row_id(
                    namespace,
                    commit_position.as_str(),
                    &fields.target_hash,
                ))
                .bind(namespace)
                .bind(commit_position.as_str())
                .bind(&fields.target_hash)
                .bind(fields.intent_kind)
                .bind(fields.cache_key)
                .bind(fields.cache_tag)
                .bind(fields.entity_name)
                .bind(fields.collection_name)
                .bind(batch.reason())
                .bind(now)
                .bind(now)
                .execute(&mut **tx)
                .await
                .map_err(sqlx_error)?;
            inserted += usize::try_from(result.rows_affected()).unwrap_or(usize::MAX);
        }
        Ok(inserted)
    }

    /// Read the current Postgres transaction id as a commit-position fallback.
    pub async fn postgres_commit_position(
        &self,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<CommitPosition> {
        let row = sqlx::query("select pg_current_xact_id()::text as position")
            .fetch_one(&mut **tx)
            .await
            .map_err(sqlx_error)?;
        Ok(CommitPosition::new(row.get::<String, _>("position")))
    }
}

impl fmt::Debug for SqlxInvalidationOutbox {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let backend = match self.pool {
            SqlxOutboxPool::Sqlite(_) => "sqlite",
            SqlxOutboxPool::Postgres(_) => "postgres",
        };
        formatter
            .debug_struct("SqlxInvalidationOutbox")
            .field("backend", &backend)
            .finish()
    }
}

#[async_trait]
impl InvalidationOutbox for SqlxInvalidationOutbox {
    async fn enqueue(
        &self,
        namespace: &str,
        commit_position: &CommitPosition,
        batch: &InvalidationIntentBatch,
    ) -> Result<usize> {
        match &self.pool {
            SqlxOutboxPool::Sqlite(pool) => {
                let mut tx = pool.begin().await.map_err(sqlx_error)?;
                let inserted = self
                    .enqueue_in_sqlite_tx(&mut tx, namespace, commit_position, batch)
                    .await?;
                tx.commit().await.map_err(sqlx_error)?;
                Ok(inserted)
            }
            SqlxOutboxPool::Postgres(pool) => {
                let mut tx = pool.begin().await.map_err(sqlx_error)?;
                let inserted = self
                    .enqueue_in_postgres_tx(&mut tx, namespace, commit_position, batch)
                    .await?;
                tx.commit().await.map_err(sqlx_error)?;
                Ok(inserted)
            }
        }
    }

    async fn claim(
        &self,
        namespace: &str,
        owner: &str,
        limit: usize,
        claim_ttl: Duration,
    ) -> Result<Vec<OutboxRow>> {
        match &self.pool {
            SqlxOutboxPool::Sqlite(pool) => {
                claim_sqlite(pool, namespace, owner, limit, claim_ttl).await
            }
            SqlxOutboxPool::Postgres(pool) => {
                claim_postgres(pool, namespace, owner, limit, claim_ttl).await
            }
        }
    }

    async fn mark_published(&self, ids: &[String]) -> Result<()> {
        match &self.pool {
            SqlxOutboxPool::Sqlite(pool) => {
                let now = now_ms_i64();
                for id in ids {
                    sqlx::query(
                        "update hydracache_invalidation_outbox \
                         set state = 'published', published_at_ms = ?, claimed_at_ms = null, \
                             claim_owner = null, last_error = null \
                         where id = ?",
                    )
                    .bind(now)
                    .bind(id)
                    .execute(pool)
                    .await
                    .map_err(sqlx_error)?;
                }
            }
            SqlxOutboxPool::Postgres(pool) => {
                let now = now_ms_i64();
                for id in ids {
                    sqlx::query(
                        "update hydracache_invalidation_outbox \
                         set state = 'published', published_at_ms = $1, claimed_at_ms = null, \
                             claim_owner = null, last_error = null \
                         where id = $2",
                    )
                    .bind(now)
                    .bind(id)
                    .execute(pool)
                    .await
                    .map_err(sqlx_error)?;
                }
            }
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
        let state = if dead {
            OutboxState::Dead
        } else {
            OutboxState::Pending
        };
        let available_at = now_ms_i64().saturating_add(duration_ms_i64(backoff));

        match &self.pool {
            SqlxOutboxPool::Sqlite(pool) => {
                sqlx::query(
                    "update hydracache_invalidation_outbox \
                     set state = ?, available_at_ms = ?, attempts = attempts + 1, \
                         claimed_at_ms = null, claim_owner = null, last_error = ? \
                     where id = ?",
                )
                .bind(state.as_str())
                .bind(available_at)
                .bind(error)
                .bind(id)
                .execute(pool)
                .await
                .map_err(sqlx_error)?;
            }
            SqlxOutboxPool::Postgres(pool) => {
                sqlx::query(
                    "update hydracache_invalidation_outbox \
                     set state = $1, available_at_ms = $2, attempts = attempts + 1, \
                         claimed_at_ms = null, claim_owner = null, last_error = $3 \
                     where id = $4",
                )
                .bind(state.as_str())
                .bind(available_at)
                .bind(error)
                .bind(id)
                .execute(pool)
                .await
                .map_err(sqlx_error)?;
            }
        }
        Ok(())
    }

    async fn reset_dead_letters(&self, namespace: &str) -> Result<u64> {
        let now = now_ms_i64();
        let affected = match &self.pool {
            SqlxOutboxPool::Sqlite(pool) => sqlx::query(
                "update hydracache_invalidation_outbox \
                 set state = 'pending', available_at_ms = ?, claimed_at_ms = null, \
                     claim_owner = null, attempts = 0, last_error = null \
                 where namespace = ? and state = 'dead'",
            )
            .bind(now)
            .bind(namespace)
            .execute(pool)
            .await
            .map_err(sqlx_error)?
            .rows_affected(),
            SqlxOutboxPool::Postgres(pool) => sqlx::query(
                "update hydracache_invalidation_outbox \
                 set state = 'pending', available_at_ms = $1, claimed_at_ms = null, \
                     claim_owner = null, attempts = 0, last_error = null \
                 where namespace = $2 and state = 'dead'",
            )
            .bind(now)
            .bind(namespace)
            .execute(pool)
            .await
            .map_err(sqlx_error)?
            .rows_affected(),
        };
        Ok(affected)
    }

    async fn status(&self, namespace: &str) -> Result<OutboxStatus> {
        match &self.pool {
            SqlxOutboxPool::Sqlite(pool) => status_sqlite(pool, namespace).await,
            SqlxOutboxPool::Postgres(pool) => status_postgres(pool, namespace).await,
        }
    }
}

async fn claim_sqlite(
    pool: &SqlitePool,
    namespace: &str,
    owner: &str,
    limit: usize,
    claim_ttl: Duration,
) -> Result<Vec<OutboxRow>> {
    if limit == 0 {
        return Ok(Vec::new());
    }

    let mut tx = pool.begin().await.map_err(sqlx_error)?;
    let now = now_ms_i64();
    let claim_expired_before = now.saturating_sub(duration_ms_i64(claim_ttl));
    let rows = sqlx::query(
        "select * from hydracache_invalidation_outbox \
         where namespace = ? and state = 'pending' and available_at_ms <= ? \
           and (claimed_at_ms is null or claimed_at_ms <= ?) \
         order by available_at_ms asc, created_at_ms asc \
         limit ?",
    )
    .bind(namespace)
    .bind(now)
    .bind(claim_expired_before)
    .bind(i64::try_from(limit).unwrap_or(i64::MAX))
    .fetch_all(&mut *tx)
    .await
    .map_err(sqlx_error)?;

    let claimed = rows
        .into_iter()
        .map(row_from_sqlite)
        .collect::<Result<Vec<_>>>()?;

    for row in &claimed {
        sqlx::query(
            "update hydracache_invalidation_outbox \
             set claimed_at_ms = ?, claim_owner = ? \
             where id = ?",
        )
        .bind(now)
        .bind(owner)
        .bind(&row.id)
        .execute(&mut *tx)
        .await
        .map_err(sqlx_error)?;
    }

    tx.commit().await.map_err(sqlx_error)?;
    Ok(claimed
        .into_iter()
        .map(|mut row| {
            row.claimed_at_ms = Some(i64_to_u64(now));
            row.claim_owner = Some(owner.to_owned());
            row
        })
        .collect())
}

async fn claim_postgres(
    pool: &PgPool,
    namespace: &str,
    owner: &str,
    limit: usize,
    claim_ttl: Duration,
) -> Result<Vec<OutboxRow>> {
    if limit == 0 {
        return Ok(Vec::new());
    }

    let mut tx = pool.begin().await.map_err(sqlx_error)?;
    let now = now_ms_i64();
    let claim_expired_before = now.saturating_sub(duration_ms_i64(claim_ttl));
    let rows = sqlx::query(
        "select * from hydracache_invalidation_outbox \
         where namespace = $1 and state = 'pending' and available_at_ms <= $2 \
           and (claimed_at_ms is null or claimed_at_ms <= $3) \
         order by available_at_ms asc, created_at_ms asc \
         limit $4 \
         for update skip locked",
    )
    .bind(namespace)
    .bind(now)
    .bind(claim_expired_before)
    .bind(i64::try_from(limit).unwrap_or(i64::MAX))
    .fetch_all(&mut *tx)
    .await
    .map_err(sqlx_error)?;

    let claimed = rows
        .into_iter()
        .map(row_from_postgres)
        .collect::<Result<Vec<_>>>()?;

    for row in &claimed {
        sqlx::query(
            "update hydracache_invalidation_outbox \
             set claimed_at_ms = $1, claim_owner = $2 \
             where id = $3",
        )
        .bind(now)
        .bind(owner)
        .bind(&row.id)
        .execute(&mut *tx)
        .await
        .map_err(sqlx_error)?;
    }

    tx.commit().await.map_err(sqlx_error)?;
    Ok(claimed
        .into_iter()
        .map(|mut row| {
            row.claimed_at_ms = Some(i64_to_u64(now));
            row.claim_owner = Some(owner.to_owned());
            row
        })
        .collect())
}

async fn status_sqlite(pool: &SqlitePool, namespace: &str) -> Result<OutboxStatus> {
    let now = now_ms_i64();
    let row = sqlx::query(
        "select \
           coalesce(sum(case when state = 'pending' then 1 else 0 end), 0) as pending, \
           min(case when state = 'pending' then created_at_ms else null end) as oldest_pending, \
           coalesce(sum(case when state = 'dead' then 1 else 0 end), 0) as dead_lettered, \
           max(published_at_ms) as last_published_at_ms, \
           coalesce(sum(attempts), 0) as failed_attempts \
         from hydracache_invalidation_outbox \
         where namespace = ?",
    )
    .bind(namespace)
    .fetch_one(pool)
    .await
    .map_err(sqlx_error)?;
    status_from_sqlite_row(row, now)
}

async fn status_postgres(pool: &PgPool, namespace: &str) -> Result<OutboxStatus> {
    let now = now_ms_i64();
    let row = sqlx::query(
        "select \
           coalesce(sum(case when state = 'pending' then 1 else 0 end), 0)::bigint as pending, \
           min(case when state = 'pending' then created_at_ms else null end) as oldest_pending, \
           coalesce(sum(case when state = 'dead' then 1 else 0 end), 0)::bigint as dead_lettered, \
           max(published_at_ms) as last_published_at_ms, \
           coalesce(sum(attempts), 0)::bigint as failed_attempts \
         from hydracache_invalidation_outbox \
         where namespace = $1",
    )
    .bind(namespace)
    .fetch_one(pool)
    .await
    .map_err(sqlx_error)?;
    status_from_postgres_row(row, now)
}

fn row_from_sqlite(row: SqliteRow) -> Result<OutboxRow> {
    row_from_parts(RowParts {
        id: row.get("id"),
        namespace: row.get("namespace"),
        commit_position: row.get("commit_position"),
        target_hash: row.get("target_hash"),
        intent_kind: row.get("intent_kind"),
        cache_key: row.get("cache_key"),
        cache_tag: row.get("cache_tag"),
        entity_name: row.get("entity_name"),
        collection_name: row.get("collection_name"),
        reason: row.get::<Option<String>, _>("reason").unwrap_or_default(),
        created_at_ms: row.get("created_at_ms"),
        available_at_ms: row.get("available_at_ms"),
        claimed_at_ms: row.get("claimed_at_ms"),
        claim_owner: row.get("claim_owner"),
        published_at_ms: row.get("published_at_ms"),
        attempts: row.get("attempts"),
        state: row.get("state"),
        last_error: row.get("last_error"),
    })
}

fn row_from_postgres(row: PgRow) -> Result<OutboxRow> {
    row_from_parts(RowParts {
        id: row.get("id"),
        namespace: row.get("namespace"),
        commit_position: row.get("commit_position"),
        target_hash: row.get("target_hash"),
        intent_kind: row.get("intent_kind"),
        cache_key: row.get("cache_key"),
        cache_tag: row.get("cache_tag"),
        entity_name: row.get("entity_name"),
        collection_name: row.get("collection_name"),
        reason: row.get::<Option<String>, _>("reason").unwrap_or_default(),
        created_at_ms: row.get("created_at_ms"),
        available_at_ms: row.get("available_at_ms"),
        claimed_at_ms: row.get("claimed_at_ms"),
        claim_owner: row.get("claim_owner"),
        published_at_ms: row.get("published_at_ms"),
        attempts: row.get("attempts"),
        state: row.get("state"),
        last_error: row.get("last_error"),
    })
}

struct RowParts {
    id: String,
    namespace: String,
    commit_position: String,
    target_hash: String,
    intent_kind: String,
    cache_key: Option<String>,
    cache_tag: Option<String>,
    entity_name: Option<String>,
    collection_name: Option<String>,
    reason: String,
    created_at_ms: i64,
    available_at_ms: i64,
    claimed_at_ms: Option<i64>,
    claim_owner: Option<String>,
    published_at_ms: Option<i64>,
    attempts: i64,
    state: String,
    last_error: Option<String>,
}

fn row_from_parts(parts: RowParts) -> Result<OutboxRow> {
    let intent = intent_from_columns(
        &parts.intent_kind,
        parts.cache_key,
        parts.cache_tag,
        parts.entity_name,
        parts.collection_name,
    )?;
    let state = OutboxState::from_storage(&parts.state)
        .ok_or_else(|| backend_error("invalid outbox row state"))?;

    Ok(OutboxRow {
        id: parts.id,
        namespace: parts.namespace,
        commit_position: CommitPosition::new(parts.commit_position),
        target_hash: parts.target_hash,
        intent,
        reason: parts.reason,
        created_at_ms: i64_to_u64(parts.created_at_ms),
        available_at_ms: i64_to_u64(parts.available_at_ms),
        claimed_at_ms: parts.claimed_at_ms.map(i64_to_u64),
        claim_owner: parts.claim_owner,
        published_at_ms: parts.published_at_ms.map(i64_to_u64),
        attempts: i64_to_u32(parts.attempts),
        state,
        last_error: parts.last_error,
    })
}

fn status_from_sqlite_row(row: SqliteRow, now: i64) -> Result<OutboxStatus> {
    let oldest = row.get::<Option<i64>, _>("oldest_pending");
    Ok(OutboxStatus {
        pending: i64_to_u64(row.get("pending")),
        oldest_pending_age_ms: oldest
            .map(|created_at| i64_to_u64(now.saturating_sub(created_at)))
            .unwrap_or_default(),
        dead_lettered: i64_to_u64(row.get("dead_lettered")),
        last_published_at_ms: row
            .get::<Option<i64>, _>("last_published_at_ms")
            .map(i64_to_u64),
        failed_attempts: i64_to_u64(row.get("failed_attempts")),
    })
}

fn status_from_postgres_row(row: PgRow, now: i64) -> Result<OutboxStatus> {
    let oldest = row.get::<Option<i64>, _>("oldest_pending");
    Ok(OutboxStatus {
        pending: i64_to_u64(row.get("pending")),
        oldest_pending_age_ms: oldest
            .map(|created_at| i64_to_u64(now.saturating_sub(created_at)))
            .unwrap_or_default(),
        dead_lettered: i64_to_u64(row.get("dead_lettered")),
        last_published_at_ms: row
            .get::<Option<i64>, _>("last_published_at_ms")
            .map(i64_to_u64),
        failed_attempts: i64_to_u64(row.get("failed_attempts")),
    })
}

struct IntentFields {
    target_hash: String,
    intent_kind: &'static str,
    cache_key: Option<String>,
    cache_tag: Option<String>,
    entity_name: Option<String>,
    collection_name: Option<String>,
}

impl IntentFields {
    fn from_intent(intent: &InvalidationIntent) -> Self {
        let mut fields = Self {
            target_hash: intent.target_hash_hex(),
            intent_kind: intent.kind(),
            cache_key: None,
            cache_tag: None,
            entity_name: None,
            collection_name: None,
        };

        match intent {
            InvalidationIntent::Key { key } => fields.cache_key = Some(key.clone()),
            InvalidationIntent::Tag { tag } => fields.cache_tag = Some(tag.clone()),
            InvalidationIntent::Entity { entity, key } => {
                fields.entity_name = Some(entity.clone());
                fields.cache_key = Some(key.clone());
            }
            InvalidationIntent::Collection { collection } => {
                fields.collection_name = Some(collection.clone());
            }
            InvalidationIntent::Flush => {}
        }

        fields
    }
}

fn intent_from_columns(
    intent_kind: &str,
    cache_key: Option<String>,
    cache_tag: Option<String>,
    entity_name: Option<String>,
    collection_name: Option<String>,
) -> Result<InvalidationIntent> {
    match intent_kind {
        "key" => cache_key
            .map(InvalidationIntent::key)
            .ok_or_else(|| backend_error("key outbox row is missing cache_key")),
        "tag" => cache_tag
            .map(InvalidationIntent::tag)
            .ok_or_else(|| backend_error("tag outbox row is missing cache_tag")),
        "entity" => match (entity_name, cache_key) {
            (Some(entity), Some(key)) => Ok(InvalidationIntent::entity(entity, key)),
            _ => Err(backend_error(
                "entity outbox row is missing entity_name or cache_key",
            )),
        },
        "collection" => collection_name
            .map(InvalidationIntent::collection)
            .ok_or_else(|| backend_error("collection outbox row is missing collection_name")),
        "flush" => Ok(InvalidationIntent::flush()),
        other => Err(backend_error(format!(
            "unsupported outbox intent kind {other}"
        ))),
    }
}

fn outbox_row_id(namespace: &str, commit_position: &str, target_hash: &str) -> String {
    let intent = InvalidationIntent::entity(namespace, format!("{commit_position}:{target_hash}"));
    intent.target_hash_hex()
}

fn i64_to_u64(value: i64) -> u64 {
    u64::try_from(value).unwrap_or_default()
}

fn i64_to_u32(value: i64) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

fn duration_ms_i64(duration: Duration) -> i64 {
    i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
}

fn now_ms_i64() -> i64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    i64::try_from(now).unwrap_or(i64::MAX)
}

fn sqlx_error(error: sqlx::Error) -> DbCacheError {
    backend_error(format!("sqlx invalidation outbox error: {error}"))
}

fn backend_error(message: impl Into<String>) -> DbCacheError {
    hydracache::CacheError::Backend(message.into()).into()
}

const SQLITE_INSERT_OUTBOX: &str = "\
insert or ignore into hydracache_invalidation_outbox (
    id, namespace, commit_position, target_hash, intent_kind,
    cache_key, cache_tag, entity_name, collection_name, reason,
    created_at_ms, available_at_ms
) values (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";

const POSTGRES_INSERT_OUTBOX: &str = "\
insert into hydracache_invalidation_outbox (
    id, namespace, commit_position, target_hash, intent_kind,
    cache_key, cache_tag, entity_name, collection_name, reason,
    created_at_ms, available_at_ms
) values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
on conflict (namespace, commit_position, target_hash) do nothing";

const SQLITE_SCHEMA: &[&str] = &[
    "create table if not exists hydracache_schema_version (
        artifact text primary key,
        version integer not null
    )",
    "insert or ignore into hydracache_schema_version (artifact, version)
     values ('hydracache_invalidation_outbox', 1)",
    "create table if not exists hydracache_invalidation_outbox (
        id text primary key,
        namespace text not null,
        commit_position text not null,
        target_hash text not null,
        intent_kind text not null,
        cache_key text null,
        cache_tag text null,
        entity_name text null,
        collection_name text null,
        reason text null,
        payload_json text null,
        created_at_ms integer not null,
        available_at_ms integer not null,
        claimed_at_ms integer null,
        claim_owner text null,
        published_at_ms integer null,
        attempts integer not null default 0,
        state text not null default 'pending',
        last_error text null,
        unique (namespace, commit_position, target_hash)
    )",
    "create index if not exists idx_hydracache_outbox_available
     on hydracache_invalidation_outbox (namespace, state, available_at_ms, created_at_ms)",
    "create index if not exists idx_hydracache_outbox_claim
     on hydracache_invalidation_outbox (claim_owner, claimed_at_ms)",
    "create index if not exists idx_hydracache_outbox_published
     on hydracache_invalidation_outbox (namespace, published_at_ms)",
];

const POSTGRES_SCHEMA: &[&str] = &[
    "create table if not exists hydracache_schema_version (
        artifact text primary key,
        version bigint not null
    )",
    "insert into hydracache_schema_version (artifact, version)
     values ('hydracache_invalidation_outbox', 1)
     on conflict (artifact) do nothing",
    "create table if not exists hydracache_invalidation_outbox (
        id text primary key,
        namespace text not null,
        commit_position text not null,
        target_hash text not null,
        intent_kind text not null,
        cache_key text null,
        cache_tag text null,
        entity_name text null,
        collection_name text null,
        reason text null,
        payload_json text null,
        created_at_ms bigint not null,
        available_at_ms bigint not null,
        claimed_at_ms bigint null,
        claim_owner text null,
        published_at_ms bigint null,
        attempts bigint not null default 0,
        state text not null default 'pending',
        last_error text null,
        unique (namespace, commit_position, target_hash)
    )",
    "create index if not exists idx_hydracache_outbox_available
     on hydracache_invalidation_outbox (namespace, state, available_at_ms, created_at_ms)",
    "create index if not exists idx_hydracache_outbox_claim
     on hydracache_invalidation_outbox (claim_owner, claimed_at_ms)",
    "create index if not exists idx_hydracache_outbox_published
     on hydracache_invalidation_outbox (namespace, published_at_ms)",
];
