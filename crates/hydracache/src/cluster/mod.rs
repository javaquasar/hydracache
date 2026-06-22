use std::collections::BTreeMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bytes::Bytes;
use hydracache_core::{CacheCodec, CacheError, CacheStats, PostcardCodec, Result};
use serde::{Deserialize, Serialize};

use crate::builder::HydraCacheBuilder;
use crate::cache::HydraCache;
use crate::invalidation_bus::{CacheInvalidationBus, InMemoryInvalidationBus};
use tokio::sync::broadcast;

mod admission;
mod control_plane;
mod diagnostics;
mod discovery;
mod ids;
mod lifecycle;
mod membership;
mod near_cache;
mod ownership;
mod peer_fetch;
mod runtime;
mod topology;

pub use admission::*;
pub use control_plane::*;
pub use diagnostics::*;
pub use discovery::*;
pub use ids::*;
pub use lifecycle::*;
pub use membership::*;
pub use near_cache::*;
pub use ownership::*;
pub use peer_fetch::*;
pub(crate) use runtime::ClusterRuntime;
pub use runtime::{HydraCacheClientBuilder, HydraCacheMemberBuilder};
pub use topology::*;

#[cfg(test)]
mod tests;
