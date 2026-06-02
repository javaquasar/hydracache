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
//! use std::time::Duration;
//!
//! use hydracache::HydraCache;
//! use hydracache_db::{DbCache, HydraCacheEntity, QueryCachePolicy};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, HydraCacheEntity)]
//! #[hydracache(entity = "user", collection = "users", id = i64)]
//! struct User {
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
//! let policy = QueryCachePolicy::new()
//!     // Metadata helper: key "user:42", tag "user:42", and tag "users".
//!     .for_cache_entity::<User>(42)
//!     .ttl(Duration::from_secs(60));
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
//!     name = "load-user",
//!     entity = User,
//!     id = user_id,
//!     ttl_secs = 60,
//! );
//!
//! assert_eq!(policy.name(), Some("load-user"));
//! assert_eq!(policy.key_value(), Some("user:42"));
//! ```

extern crate self as hydracache_db;

mod entity;
mod error;
mod policy;
mod query;

pub use entity::CacheEntity;
pub use error::{DbCacheError, Result};
pub use hydracache_macros::{query_cache_policy, HydraCacheEntity};
pub use policy::QueryCachePolicy;
pub use query::{DbCache, DbQuery};

#[cfg(test)]
mod tests;
