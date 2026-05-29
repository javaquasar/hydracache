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
//! use hydracache_db::DbCache;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
//! let user = queries
//!     .cached::<User>()
//!     // Explicit key: where this one query result is stored.
//!     .key("user:42")
//!     // Explicit tag: how related query results can be invalidated together.
//!     .tag("user:42")
//!     .fetch_with(|| async {
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

mod error;
mod query;

pub use error::{DbCacheError, Result};
pub use query::{DbCache, DbQuery};

#[cfg(test)]
mod tests;
