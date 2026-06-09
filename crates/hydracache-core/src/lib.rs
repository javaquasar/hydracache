//! Core types for HydraCache.
//!
//! This crate intentionally contains no database adapter and no distributed runtime.
//! It defines the small set of types shared by the v0 local cache.

mod codec;
mod error;
mod events;
mod key;
mod options;
mod stats;
mod tags;

pub use codec::{CacheCodec, PostcardCodec};
pub use error::CacheError;
pub use events::{
    CacheEvent, CacheEventKind, CacheEventOptions, CacheEventOrigin, CacheEventScope,
    CacheEventValueMode,
};
pub use key::{CacheKey, CacheKeyBuilder};
pub use options::CacheOptions;
pub use stats::{CacheDiagnostics, CacheStats};
pub use tags::TagSet;

/// HydraCache result type.
pub type Result<T> = std::result::Result<T, CacheError>;
