#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

#[cfg(not(target_arch = "wasm32"))]
use std::collections::BTreeMap;
use std::fmt;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::{Arc, Mutex};
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use bytes::Bytes;
#[cfg(not(target_arch = "wasm32"))]
use hydracache_core::{CacheCodec, CacheError, CacheStats, PostcardCodec, Result};
use serde::{Deserialize, Serialize};

#[cfg(not(target_arch = "wasm32"))]
use crate::builder::HydraCacheBuilder;
#[cfg(not(target_arch = "wasm32"))]
use crate::cache::HydraCache;
#[cfg(not(target_arch = "wasm32"))]
use crate::invalidation_bus::{CacheInvalidationBus, InMemoryInvalidationBus};
#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::broadcast;

#[cfg(not(target_arch = "wasm32"))]
mod admission;
#[cfg(not(target_arch = "wasm32"))]
mod control_plane;
#[cfg(not(target_arch = "wasm32"))]
mod diagnostics;
#[cfg(not(target_arch = "wasm32"))]
mod discovery;
mod ids;
#[cfg(not(target_arch = "wasm32"))]
mod lifecycle;
#[cfg(not(target_arch = "wasm32"))]
mod membership;
#[cfg(not(target_arch = "wasm32"))]
mod near_cache;
mod node;
#[cfg(not(target_arch = "wasm32"))]
mod ownership;
#[cfg(not(target_arch = "wasm32"))]
mod peer_fetch;
#[cfg(not(target_arch = "wasm32"))]
mod runtime;
#[cfg(not(target_arch = "wasm32"))]
mod topology;

#[cfg(not(target_arch = "wasm32"))]
pub use admission::*;
#[cfg(not(target_arch = "wasm32"))]
pub use control_plane::*;
#[cfg(not(target_arch = "wasm32"))]
pub use diagnostics::*;
#[cfg(not(target_arch = "wasm32"))]
pub use discovery::*;
pub use ids::*;
#[cfg(not(target_arch = "wasm32"))]
pub use lifecycle::*;
#[cfg(not(target_arch = "wasm32"))]
pub use membership::*;
#[cfg(not(target_arch = "wasm32"))]
pub use near_cache::*;
pub use node::*;
#[cfg(not(target_arch = "wasm32"))]
pub use ownership::*;
#[cfg(not(target_arch = "wasm32"))]
pub use peer_fetch::*;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use runtime::ClusterRuntime;
#[cfg(not(target_arch = "wasm32"))]
pub use runtime::{HydraCacheClientBuilder, HydraCacheMemberBuilder};
#[cfg(not(target_arch = "wasm32"))]
pub use topology::*;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;
