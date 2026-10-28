//! The async locks a connection holds.
//!
//! These belong to no particular runtime: any of them can be taken from a future polled
//! anywhere. They come from zruntime, except where the `tokio` feature is on: Tokio's locks are
//! used there instead. Only the object server takes readers-writer locks, so those come along
//! with the `service` feature.

#[cfg(feature = "tokio")]
pub(crate) use tokio::sync::Mutex;
#[cfg(all(feature = "tokio", feature = "service"))]
pub(crate) use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
#[cfg(not(feature = "tokio"))]
pub(crate) use zruntime::lock::Mutex;
#[cfg(all(not(feature = "tokio"), feature = "service"))]
pub(crate) use zruntime::lock::{RwLock, RwLockReadGuard, RwLockWriteGuard};
