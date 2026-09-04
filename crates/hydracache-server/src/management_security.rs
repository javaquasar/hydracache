//! Independent admission budget for read-only management requests.

use std::sync::Arc;

use tokio::sync::{OwnedSemaphorePermit, Semaphore, TryAcquireError};

/// Maximum management reads executing concurrently on one admin surface.
pub const MANAGEMENT_READ_MAX_CONCURRENCY: usize = 16;

/// Cloneable limiter whose permits are independent from data-plane and write-admin paths.
#[derive(Debug, Clone)]
pub struct ManagementReadLimiter {
    permits: Arc<Semaphore>,
}

impl Default for ManagementReadLimiter {
    fn default() -> Self {
        Self::new(MANAGEMENT_READ_MAX_CONCURRENCY)
    }
}

impl ManagementReadLimiter {
    /// Create a non-zero concurrency budget.
    pub fn new(max_concurrency: usize) -> Self {
        Self {
            permits: Arc::new(Semaphore::new(max_concurrency.max(1))),
        }
    }

    /// Acquire immediately or fail without queueing behind an overloaded diagnostic surface.
    pub fn try_acquire(&self) -> Result<OwnedSemaphorePermit, ManagementReadLimitError> {
        Arc::clone(&self.permits)
            .try_acquire_owned()
            .map_err(ManagementReadLimitError::from)
    }

    /// Number of currently free permits, exposed for resource-cleanup tests.
    pub fn available_permits(&self) -> usize {
        self.permits.available_permits()
    }
}

/// Stable overload outcome; a closed pool is treated as unavailable, never as capacity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagementReadLimitError {
    Saturated,
    Closed,
}

impl From<TryAcquireError> for ManagementReadLimitError {
    fn from(error: TryAcquireError) -> Self {
        match error {
            TryAcquireError::NoPermits => Self::Saturated,
            TryAcquireError::Closed => Self::Closed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saturation_is_bounded_and_dropped_requests_release_capacity() {
        let limiter = ManagementReadLimiter::new(2);
        let first = limiter.try_acquire().unwrap();
        let second = limiter.try_acquire().unwrap();
        assert_eq!(limiter.available_permits(), 0);
        assert_eq!(
            limiter.try_acquire().unwrap_err(),
            ManagementReadLimitError::Saturated
        );
        drop(first);
        assert_eq!(limiter.available_permits(), 1);
        let replacement = limiter.try_acquire().unwrap();
        drop((second, replacement));
        assert_eq!(limiter.available_permits(), 2);
    }
}
