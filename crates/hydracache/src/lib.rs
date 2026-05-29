//! User-facing HydraCache local runtime.
//!
//! v0 is intentionally local-only: no SQLx adapter, no distributed coordination,
//! and no cluster membership. The goal is a small async cache with TTL, tags,
//! local single-flight, and pleasant loader ergonomics.
//!
//! # Quick start
//!
//! ```rust
//! use std::time::Duration;
//!
//! use hydracache::{CacheOptions, HydraCache};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
//! struct User {
//!     id: u64,
//!     name: String,
//! }
//!
//! # #[tokio::main]
//! # async fn main() -> hydracache::CacheResult<()> {
//! let cache = HydraCache::local()
//!     .default_ttl(Duration::from_secs(300))
//!     .max_capacity(10_000)
//!     .build();
//!
//! let user = cache
//!     .get_or_insert_with("user:42", CacheOptions::new().tag("user:42"), || async {
//!         User {
//!             id: 42,
//!             name: "Ada".to_owned(),
//!         }
//!     })
//!     .await?;
//!
//! assert_eq!(user.id, 42);
//! cache.invalidate_tag("user:42").await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Typed local cache
//!
//! ```rust
//! use hydracache::{CacheOptions, HydraCache};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
//! struct User {
//!     id: u64,
//!     name: String,
//! }
//!
//! # #[tokio::main]
//! # async fn main() -> hydracache::CacheResult<()> {
//! let cache = HydraCache::local().build();
//! let users = cache.typed::<User>("users");
//!
//! users
//!     .put(
//!         "42",
//!         User {
//!             id: 42,
//!             name: "Ada".to_owned(),
//!         },
//!         CacheOptions::new(),
//!     )
//!     .await?;
//!
//! let cached = users.get("42").await?;
//! assert_eq!(cached.map(|user| user.id), Some(42));
//! # Ok(())
//! # }
//! ```

mod builder;
mod cache;
mod entry;
mod inflight;
mod stats;
mod tag_index;
mod typed;

pub use builder::HydraCacheBuilder;
pub use cache::HydraCache;
pub use hydracache_core::{
    CacheError, CacheKey, CacheKeyBuilder, CacheOptions, CacheStats, PostcardCodec, TagSet,
};
pub use typed::TypedCache;

pub use hydracache_core::{CacheOptions as Options, CacheStats as Stats, Result as CacheResult};

#[cfg(test)]
mod tests;
