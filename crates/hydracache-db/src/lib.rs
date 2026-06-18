//! Database-neutral query result caching helpers for HydraCache.
//!
//! This crate is intentionally a thin runtime adapter. It does not replace a
//! database client, ORM, or query builder. Callers keep their database library
//! as the query authority and provide an explicit cache key, tags, and TTL
//! around the operation they want to cache.
//!
//! # Example
//!
//! ```rust
//! use hydracache::HydraCache;
//! use hydracache_db::{
//!     DbCache, HydraCacheEntity, PreparedQueryPolicy, QueryCachePolicy, RefreshPolicy,
//! };
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, HydraCacheEntity)]
//! #[hydracache(entity = "user", collection = "users")]
//! struct User {
//!     #[hydracache(id)]
//!     id: i64,
//!     name: String,
//! }
//!
//! # #[tokio::main]
//! # async fn main() -> hydracache_db::Result<()> {
//! let local = HydraCache::local().build();
//!
//! // The adapter wraps the local HydraCache instance. The namespace becomes
//! // part of the physical cache key, so key("user:42") is stored as
//! // "db:user:42".
//! let queries = DbCache::new(local, "db");
//!
//! let policy = QueryCachePolicy::read_mostly()
//!     // Metadata helper: key "user:42", tag "user:42", and tag "users".
//!     .for_cache_entity::<User>(42)
//!     .with_name("load-user")
//!     .refresh_policy(
//!         RefreshPolicy::new()
//!             .refresh_ahead(std::time::Duration::from_secs(10))
//!             .stale_while_revalidate(std::time::Duration::from_secs(300)),
//!     );
//!
//! let user = queries
//!     .cached_with::<User>(policy)
//!     .load(|| async {
//!         // This loader runs only on a cache miss. On a cache hit, HydraCache
//!         // returns the cached User and this database code is not executed.
//!         Ok::<_, std::io::Error>(User {
//!             id: 42,
//!             name: "Ada".to_owned(),
//!         })
//!     })
//!     .await?;
//!
//! assert_eq!(user.id, 42);
//! # Ok(())
//! # }
//! ```
//!
//! For hot repository methods, prepare stable metadata once and bind only the
//! dynamic id on each call:
//!
//! ```rust
//! use hydracache::HydraCache;
//! use hydracache_db::{DbCache, HydraCacheEntity, PreparedQueryPolicy};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, HydraCacheEntity)]
//! #[hydracache(entity = "user", collection = "users")]
//! struct User {
//!     #[hydracache(id)]
//!     id: i64,
//!     name: String,
//! }
//!
//! # #[tokio::main]
//! # async fn main() -> hydracache_db::Result<()> {
//! let queries = DbCache::new(HydraCache::local().build(), "db");
//! let load_user = queries.prepare::<User>(
//!     PreparedQueryPolicy::per_entity()
//!         .cache_entity::<User>()
//!         .with_name("load-user"),
//! );
//!
//! let user = load_user
//!     .load_id(42, || async {
//!         Ok::<_, std::io::Error>(User {
//!             id: 42,
//!             name: "Ada".to_owned(),
//!         })
//!     })
//!     .await?;
//!
//! assert_eq!(user.id, 42);
//! # Ok(())
//! # }
//! ```
//!
//! For compact policy construction, use [`query_cache_policy!`]:
//!
//! ```rust
//! use hydracache_db::{query_cache_policy, CacheEntity};
//!
//! struct User;
//!
//! impl CacheEntity for User {
//!     type Id = i64;
//!
//!     const ENTITY: &'static str = "user";
//!     const COLLECTION: Option<&'static str> = Some("users");
//! }
//!
//! let user_id = 42_i64;
//! let policy = query_cache_policy!(
//!     preset = read_mostly,
//!     name = "load-user",
//!     entity = User,
//!     id = user_id,
//!     refresh_ahead_secs = 10,
//!     stale_while_revalidate_secs = 300,
//! );
//!
//! assert_eq!(policy.name(), Some("load-user"));
//! assert_eq!(policy.key_value(), Some("user:42"));
//! assert!(policy.refresh_policy_value().is_some());
//!
//! let search = query_cache_policy!(
//!     name = "search-users",
//!     key_segments = ["tenant", 7_u64, "q", "ada:lovelace", "page", 1_u32],
//!     tag_segments = [["tenant", 7_u64], ["users"]],
//!     ttl_secs = 30,
//! );
//!
//! assert_eq!(
//!     search.key_value(),
//!     Some("tenant:7:q:ada%3Alovelace:page:1")
//! );
//! assert_eq!(search.tags_value(), &["tenant:7".to_owned(), "users".to_owned()]);
//! ```
//!
//! For write paths, stage invalidations during repository work and execute them
//! only after the database transaction commits:
//!
//! ```rust
//! use hydracache::HydraCache;
//! use hydracache_db::{HydraCacheEntity, InvalidationPlan};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Clone, Serialize, Deserialize, HydraCacheEntity)]
//! #[hydracache(entity = "user", collection = "users")]
//! struct User {
//!     #[hydracache(id)]
//!     id: i64,
//! }
//!
//! # #[tokio::main]
//! # async fn main() -> hydracache::CacheResult<()> {
//! let cache = HydraCache::local().build();
//! let pending = InvalidationPlan::new().cache_entity::<User>(42);
//!
//! // tx.update_user(42).await?;
//! // tx.commit().await?;
//!
//! let report = pending.execute(&cache).await?;
//! assert_eq!(report.tag_count, 2);
//! # Ok(())
//! # }
//! ```

extern crate self as hydracache_db;

mod entity;
mod error;
mod invalidation;
mod outbox;
mod policy;
mod prepared;
mod query;
#[cfg(feature = "sqlx-outbox")]
mod sqlx_outbox;

pub use entity::CacheEntity;
pub use error::{DbAdapterKind, DbCacheError, DbOperationContext, DbResultShape, Result};
pub use hydracache::CacheKeyBuilder;
pub use hydracache_macros::{prepared_query_policy, query_cache_policy, HydraCacheEntity};
pub use invalidation::{InvalidationPlan, InvalidationReport};
pub use outbox::{
    CommitPosition, ConsistencyMode, InMemoryInvalidationOutbox, InvalidationApplier,
    InvalidationIntent, InvalidationIntentBatch, InvalidationOutbox, InvalidationOutboxWorker,
    InvalidationReceipt, InvalidationTargetHash, InvalidationWait, InvalidationWaitDiagnostics,
    InvalidationWaitOutcome, OutboxPublishReport, OutboxRow, OutboxState, OutboxStatus,
    OutboxWorkerDiagnostics,
};
pub use policy::QueryCachePolicy;
pub use prepared::PreparedQueryPolicy;
pub use query::{DbCache, DbQuery, PreparedDbQuery};
#[cfg(feature = "sqlx-outbox")]
pub use sqlx_outbox::{
    PgNotifyIntent, PgNotifyIntentSource, SqlxInvalidationOutbox, OUTBOX_SCHEMA_VERSION,
};

/// Database-facing alias for local cache refresh/stale behavior.
pub type RefreshPolicy = hydracache::RefreshOptions;

#[cfg(test)]
mod tests;
