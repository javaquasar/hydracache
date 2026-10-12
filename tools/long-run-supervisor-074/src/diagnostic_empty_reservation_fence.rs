//! Existing common fence before initial state; no publication/write capability.
use super::super::super::{
    campaign_admission::leaf_present, FixedIntentStorageError, FixedIntentStorageRead, StorageLeaf,
};
use super::{Fence, OriginalHostFenceError};
use crate::host_execution::HostExecutionError;
use std::fs::File;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EmptyReservationFenceError {
    #[error("original intent storage refused: {0}")]
    Storage(#[from] FixedIntentStorageError),
    #[error("original common fence refused: {0}")]
    Fence(#[from] OriginalHostFenceError),
    #[error("fixed leaf has retained diagnostic context")]
    Conflict,
    #[error("fixed storage observation refused: {0}")]
    Scan(#[from] HostExecutionError),
    #[error("storage is not the complete original intent origin")]
    Origin,
    #[error("original empty reservation fence permanently refused")]
    Refused,
}
fn parent(storage: &FixedIntentStorageRead) -> Result<&File, EmptyReservationFenceError> {
    Ok(&storage
        .components
        .get(2)
        .ok_or(EmptyReservationFenceError::Origin)?
        .file)
}

/// Exclusively retains original storage and its existing trusted common fence.
/// An empty observation is neither state publication nor execution permission.
///
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::OriginalEmptyReservationFence;
/// fn copy(read: OriginalEmptyReservationFence<'_>) { let _ = read.clone(); }
/// ```
///
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_output_preparation::fixed_intent_storage::{FixedIntentStorageRead, reserved_state::original_fence::reservation_fence::bind_original_empty_reservation_fence};
/// fn reuse(storage: &mut FixedIntentStorageRead) {
///     let mut fence = bind_original_empty_reservation_fence(storage).unwrap();
///     storage.revalidate().unwrap();
///     fence.revalidate().unwrap();
/// }
/// ```
///
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::OriginalEmptyReservationFence;
/// fn start(read: &mut OriginalEmptyReservationFence<'_>) { read.start_once(); }
/// ```
///
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::bind_original_empty_reservation_fence;
/// fn select() { bind_original_empty_reservation_fence(std::path::Path::new("/tmp/fixture")); }
/// ```
pub struct OriginalEmptyReservationFence<'storage> {
    storage: &'storage mut FixedIntentStorageRead,
    fence: Fence,
}
impl OriginalEmptyReservationFence<'_> {
    pub(crate) fn latch_refusal(&mut self) {
        self.storage.refused = true;
    }
    pub fn is_refused(&self) -> bool {
        self.storage.is_refused()
    }
    pub fn revalidate(&mut self) -> Result<(), EmptyReservationFenceError> {
        self.revalidate_with(|| {})
    }
    fn revalidate_with(
        &mut self,
        checkpoint: impl FnOnce(),
    ) -> Result<(), EmptyReservationFenceError> {
        if self.is_refused() {
            return Err(EmptyReservationFenceError::Refused);
        }
        let result = (|| {
            self.storage.revalidate()?;
            self.fence.inspect(parent(self.storage)?)?;
            if leaf_present(self.storage)? {
                return Err(EmptyReservationFenceError::Conflict);
            }
            // Private logical test checkpoint; production invokes only the no-op.
            checkpoint();
            self.fence.inspect(parent(self.storage)?)?;
            self.storage.revalidate()?;
            if leaf_present(self.storage)? {
                return Err(EmptyReservationFenceError::Conflict);
            }
            self.fence.inspect(parent(self.storage)?)?;
            self.storage.revalidate()?;
            Ok(())
        })();
        if result.is_err() {
            self.storage.refused = true;
        }
        result
    }
}
pub fn bind_original_empty_reservation_fence(
    storage: &mut FixedIntentStorageRead,
) -> Result<OriginalEmptyReservationFence<'_>, EmptyReservationFenceError> {
    if storage.is_refused() {
        return Err(EmptyReservationFenceError::Refused);
    }
    if !matches!(storage.kind, StorageLeaf::Intent) {
        return Err(EmptyReservationFenceError::Origin);
    }
    storage.revalidate()?;
    let result = (|| {
        let leaf = storage
            .components
            .get(3)
            .ok_or(EmptyReservationFenceError::Origin)?;
        Ok(Fence::acquire(
            parent(storage)?,
            leaf.stamp.uid,
            leaf.stamp.gid,
        )?)
    })();
    let fence = match result {
        Ok(fence) => fence,
        Err(error) => {
            if !matches!(
                error,
                EmptyReservationFenceError::Fence(OriginalHostFenceError::Busy)
            ) {
                storage.refused = true;
            }
            return Err(error);
        }
    };
    let mut read = OriginalEmptyReservationFence { storage, fence };
    read.revalidate()?;
    Ok(read)
}

#[cfg(test)]
#[path = "diagnostic_empty_reservation_fence_tests.rs"]
mod tests;

#[cfg(test)]
pub(crate) use tests::with_empty_fence_for_test;
