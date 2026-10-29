//! Observe the same armed root, never create or replace its kill target.
use super::*;
use crate::diagnostic_two_phase_launch::root_enrollment::FixedSlice;

impl BoundGuardians<'_> {
    pub(crate) fn pin_fixed_slice(&mut self, attempt: [u8; 32]) -> io::Result<FixedSlice> {
        let result = (|| {
            if self.binding_for_placement()?.attempt_sha256 != attempt {
                return Err(refused());
            }
            FixedSlice::pin(self.pair.root, attempt)
        })();
        if result.is_err() {
            self.refuse();
        }
        result
    }
    pub(crate) fn observe_fixed_slice(
        &mut self,
        slice: &FixedSlice,
        attempt: [u8; 32],
    ) -> io::Result<()> {
        let result = (|| {
            if self.binding_for_placement()?.attempt_sha256 != attempt {
                return Err(refused());
            }
            slice.revalidate(self.pair.root, attempt)
        })();
        if result.is_err() {
            self.refuse();
        }
        result
    }
}
