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
//! # Cacheable functions
//!
//! Use [`cacheable!`] when an ordinary async function or expensive operation
//! should be cached without introducing database-result-cache concepts.
//! `cacheable!` wraps fallible loaders. [`cacheable_infallible!`] wraps loaders
//! that return a value directly.
//!
//! ```rust
//! use std::time::Duration;
//!
//! use hydracache::{cacheable, cacheable_infallible, HydraCache};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
//! struct Report {
//!     id: u64,
//! }
//!
//! #[derive(Debug)]
//! struct LoadError;
//!
//! impl std::fmt::Display for LoadError {
//!     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//!         f.write_str("load failed")
//!     }
//! }
//!
//! impl std::error::Error for LoadError {}
//!
//! # #[tokio::main]
//! # async fn main() -> hydracache::CacheResult<()> {
//! let cache = HydraCache::local().build();
//!
//! let report = cacheable!(
//!     cache = cache,
//!     key = "report:42",
//!     tags = ["reports", "report:42"],
//!     ttl = Duration::from_secs(60),
//!     load = || async { Ok::<_, LoadError>(Report { id: 42 }) },
//! )
//! .await?;
//!
//! assert_eq!(report.id, 42);
//!
//! let total = cacheable_infallible!(
//!     cache = cache,
//!     key = "report-total:42",
//!     tags = ["reports", "report:42"],
//!     ttl_secs = 60,
//!     load = || async { 42_u64 },
//! )
//! .await?;
//!
//! assert_eq!(total, 42);
//! # Ok(())
//! # }
//! ```
//!
//! Use [`CacheKeyBuilder`] and [`TagSet`] when the key and invalidation tags are
//! generated from the same domain metadata:
//!
//! ```rust
//! use hydracache::{cacheable, CacheKeyBuilder, HydraCache, TagSet};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
//! struct Profile {
//!     id: u64,
//! }
//!
//! #[derive(Debug)]
//! struct LoadError;
//!
//! impl std::fmt::Display for LoadError {
//!     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//!         f.write_str("load failed")
//!     }
//! }
//!
//! impl std::error::Error for LoadError {}
//!
//! # #[tokio::main]
//! # async fn main() -> hydracache::CacheResult<()> {
//! let cache = HydraCache::local().build();
//! let profile_id = 42_u64;
//! let key = CacheKeyBuilder::new()
//!     .entity("profile", profile_id)
//!     .build_string();
//!
//! let profile = cacheable!(
//!     cache = cache,
//!     key = key.as_str(),
//!     tags = TagSet::new().tag("profiles").entity("profile", profile_id),
//!     ttl_secs = 60,
//!     load = move || async move {
//!         Ok::<_, LoadError>(Profile { id: profile_id })
//!     },
//! )
//! .await?;
//!
//! assert_eq!(profile.id, 42);
//! cache.invalidate_tag("profile:42").await?;
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
//!
//! # Cache events
//!
//! Use [`HydraCache::subscribe`] when an application, actuator, or sandbox
//! wants to observe cache mutations without wrapping every call manually.
//! Access/load events are opt-in because hit/miss streams can be noisy.
//!
//! ```rust
//! use hydracache::{CacheEventKind, CacheOptions, HydraCache};
//!
//! # #[tokio::main]
//! # async fn main() -> hydracache::CacheResult<()> {
//! let cache = HydraCache::local().build();
//! let mut events = cache.subscribe_mutations();
//!
//! cache
//!     .put("user:42", 42_u64, CacheOptions::new().tag("users"))
//!     .await?;
//!
//! let event = events.recv().await.expect("stored event");
//! assert_eq!(event.kind(), CacheEventKind::Stored);
//! assert_eq!(event.key(), Some("user:42"));
//! assert_eq!(event.tags(), &["users".to_owned()]);
//! # Ok(())
//! # }
//! ```
//!
//! Callback listeners are adapters over the same subscription stream:
//!
//! ```rust
//! use hydracache::{CacheOptions, HydraCache};
//!
//! # #[tokio::main]
//! # async fn main() -> hydracache::CacheResult<()> {
//! let cache = HydraCache::local().build();
//! let listener = cache.on_mutation(|event| {
//!     println!("cache changed: {event:?}");
//! });
//!
//! cache.put("user:42", 42_u64, CacheOptions::new()).await?;
//! listener.unsubscribe();
//! # Ok(())
//! # }
//! ```
//!
//! # Distributed invalidation bus
//!
//! Use [`InMemoryInvalidationBus`] when several cache instances in one process
//! should propagate invalidation intent to each other. The bus only sends
//! `invalidate_key`, `invalidate_tag`, `remove`, and `flush` operations; cached
//! values are not replicated.
//!
//! ```rust
//! use std::sync::Arc;
//! use std::time::Duration;
//!
//! use hydracache::{CacheEventOrigin, CacheOptions, HydraCache, InMemoryInvalidationBus};
//!
//! # #[tokio::main]
//! # async fn main() -> hydracache::CacheResult<()> {
//! let bus = Arc::new(InMemoryInvalidationBus::default());
//! let first = HydraCache::local()
//!     .shared_invalidation_bus(bus.clone())
//!     .invalidation_node_id("first")
//!     .build();
//! let second = HydraCache::local()
//!     .shared_invalidation_bus(bus)
//!     .invalidation_node_id("second")
//!     .build();
//!
//! first
//!     .put("user:42", 42_u64, CacheOptions::new().tag("users"))
//!     .await?;
//! second
//!     .put("user:42", 42_u64, CacheOptions::new().tag("users"))
//!     .await?;
//!
//! let mut events = second.subscribe_tag("users");
//! first.invalidate_tag("users").await?;
//!
//! // Remote invalidation is applied by a background task, so applications that
//! // need to observe it immediately should wait on events or diagnostics.
//! let event = tokio::time::timeout(Duration::from_millis(500), events.recv())
//!     .await
//!     .expect("remote invalidation event")
//!     .expect("subscription stays open");
//!
//! assert_eq!(event.origin(), CacheEventOrigin::DistributedBus);
//! assert!(!second.contains_key("user:42").await);
//! assert_eq!(first.stats().distributed_invalidations_published, 1);
//! assert_eq!(second.stats().distributed_invalidations_applied, 1);
//! # Ok(())
//! # }
//! ```
//!
//! # Observability
//!
//! Use [`HydraCache::diagnostics`] for quick local smoke checks. It combines
//! lightweight stats with the approximate local backend entry count.
//!
//! ```rust
//! use hydracache::{CacheOptions, HydraCache};
//!
//! # #[tokio::main]
//! # async fn main() -> hydracache::CacheResult<()> {
//! let cache = HydraCache::local().build();
//!
//! let first = cache
//!     .get_or_insert_with("answer", CacheOptions::new(), || async { 42_u64 })
//!     .await?;
//! let second = cache
//!     .get_or_insert_with("answer", CacheOptions::new(), || async { 7_u64 })
//!     .await?;
//!
//! let diagnostics = cache.diagnostics().await;
//! assert_eq!((first, second), (42, 42));
//! assert_eq!(diagnostics.stats.loads, 1);
//! assert_eq!(diagnostics.stats.hits, 1);
//! assert_eq!(diagnostics.hit_ratio(), Some(0.5));
//! # Ok(())
//! # }
//! ```

extern crate self as hydracache;

mod builder;
mod cache;
mod entry;
mod events;
mod inflight;
mod invalidation_bus;
mod stats;
mod tag_index;
mod typed;

pub use builder::HydraCacheBuilder;
pub use cache::HydraCache;
pub use events::{CacheEventListenerHandle, CacheEventRecvError, CacheEventSubscriber};
pub use hydracache_core::{
    CacheDiagnostics, CacheError, CacheEvent, CacheEventKind, CacheEventOptions, CacheEventOrigin,
    CacheEventScope, CacheEventValueMode, CacheKey, CacheKeyBuilder, CacheOptions, CacheStats,
    PostcardCodec, TagSet,
};
pub use hydracache_macros::{cacheable, cacheable_infallible};
pub use invalidation_bus::{
    CacheInvalidation, CacheInvalidationBus, CacheInvalidationMessage, CacheInvalidationReceiver,
    InMemoryInvalidationBus,
};
pub use typed::TypedCache;

pub use hydracache_core::{
    CacheDiagnostics as Diagnostics, CacheOptions as Options, CacheStats as Stats,
    Result as CacheResult,
};

#[cfg(test)]
mod tests;
