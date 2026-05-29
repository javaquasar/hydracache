//! SQLx-oriented query result caching helpers for HydraCache.
//!
//! This crate is intentionally a thin runtime adapter. It does not replace
//! SQLx compile-time checking and it does not derive cache keys from SQL
//! automatically. Callers keep SQLx as the database authority and provide an
//! explicit cache key, tags, and TTL around the query they want to cache.
//!
//! # Example
//!
//! ```rust
//! use hydracache::HydraCache;
//! use hydracache_sqlx::DbCache;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
//! struct User {
//!     id: i64,
//!     name: String,
//! }
//!
//! # #[tokio::main]
//! # async fn main() -> hydracache_sqlx::Result<()> {
//! let local = HydraCache::local().build();
//!
//! // The adapter wraps the local HydraCache instance. The namespace becomes
//! // part of the physical cache key, so key("user:42") is stored as
//! // "db:user:42".
//! let queries = DbCache::new(local, "db");
//!
//! let user = queries
//!     .cached::<User>()
//!     // Explicit key: where this one query result is stored.
//!     .key("user:42")
//!     // Explicit tag: how related query results can be invalidated together.
//!     .tag("user:42")
//!     .fetch_with(|| async {
//!         // This loader runs only on a cache miss. On a cache hit, HydraCache
//!         // returns the cached User and this SQLx code is not executed.
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

mod error;
mod query;

pub use error::{Result, SqlxCacheError};
pub use query::{DbCache, DbQuery, SqlxCache, SqlxQuery};

/// Re-export the SQLx crate used by this adapter.
///
/// This lets downstream users keep one adapter-aligned SQLx version in examples
/// and integration code without hiding SQLx behind HydraCache abstractions.
pub use sqlx;

#[cfg(test)]
mod tests;
