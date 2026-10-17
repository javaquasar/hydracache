//! Opaque custody from this original acknowledged Starting storage only.
use super::*;
use crate::diagnostic_output_preparation::fixed_intent_storage::reserved_state::{
    original_fence::{
        reservation_fence::{parent, EmptyReservationFenceError},
        FenceCustody,
    },
    Stamp,
};
use std::os::fd::AsRawFd;

pub(crate) struct OriginalCollectorFenceCustody {
    original: FenceCustody,
}
impl CollectorReservationStorage<'_> {
    pub(crate) fn retain_original_collector_custody(
        &mut self,
    ) -> Result<OriginalCollectorFenceCustody, StorageAttemptError> {
        let result = (|| {
            if self.is_refused() || !self.starting.as_ref().is_some_and(|s| s.acknowledged) {
                return Err(StorageAttemptError::Refused);
            }
            self.revalidate()?;
            let original = &mut self.original.empty;
            let custody = original
                .fence
                .retain_custody(parent(original.storage)?)
                .map_err(EmptyReservationFenceError::from)?;
            Ok(OriginalCollectorFenceCustody { original: custody })
        })();
        if result.is_err() {
            self.latch_refusal();
        }
        result
    }
}
impl OriginalCollectorFenceCustody {
    pub(crate) fn inspect(
        &self,
        storage: &CollectorReservationStorage<'_>,
    ) -> Result<(), StorageAttemptError> {
        if storage.is_refused() || !storage.starting.as_ref().is_some_and(|s| s.acknowledged) {
            return Err(StorageAttemptError::Refused);
        }
        let fence = &storage.original.empty.fence;
        if !fence.custody_retained
            || Stamp::of(&self.original._lock.metadata()?) != fence.lock_stamp
            || Stamp::of(&fence.lock.metadata()?) != fence.lock_stamp
            || unsafe { libc::fcntl(self.original._lock.as_raw_fd(), libc::F_GETFD) }
                != libc::FD_CLOEXEC
        {
            return Err(StorageAttemptError::Document);
        }
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn tamper_for_test(&mut self, object: bool) {
        if object {
            self.original._lock = std::fs::File::open("/dev/null").unwrap();
        } else {
            assert_eq!(
                unsafe { libc::fcntl(self.original._lock.as_raw_fd(), libc::F_SETFD, 0) },
                0
            );
        }
    }
}
