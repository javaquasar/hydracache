//! Same consumed owner plus fixed-path observations, not a held-unit permission.
use super::*;
use crate::diagnostic_two_phase_launch::root_enrollment::FixedSlice;

#[path = "diagnostic_consuming_held_join.rs"]
mod held_join;

trait SliceControls: PlacementControls {
    type Slice;
    fn pin(&mut self, attempt: [u8; 32]) -> Result<Self::Slice, PendingError>;
    fn observe_slice(&mut self, slice: &Self::Slice, attempt: [u8; 32])
        -> Result<(), PendingError>;
}
impl SliceControls for BoundGuardians<'_> {
    type Slice = FixedSlice;
    fn pin(&mut self, attempt: [u8; 32]) -> Result<Self::Slice, PendingError> {
        Ok(self.pin_fixed_slice(attempt)?)
    }
    fn observe_slice(
        &mut self,
        slice: &Self::Slice,
        attempt: [u8; 32],
    ) -> Result<(), PendingError> {
        Ok(self.observe_fixed_slice(slice, attempt)?)
    }
}
fn round<C: SliceControls>(
    owner: &mut impl PlacementOwner,
    controls: &mut C,
    raw: &[u8],
    snapshot: &Snapshot,
    slice: &C::Slice,
) -> Result<(), PendingError> {
    let result = (|| {
        controls.observe_slice(slice, snapshot.root.attempt_sha256)?;
        placement_join(owner, controls, raw, Some(snapshot))?;
        controls.observe_slice(slice, snapshot.root.attempt_sha256)
    })();
    if result.is_err() {
        retire(owner, controls);
    }
    result
}
fn pin<C: SliceControls>(
    owner: &mut impl PlacementOwner,
    controls: &mut C,
    raw: &[u8],
    snapshot: &Snapshot,
) -> Result<C::Slice, PendingError> {
    let result = (|| {
        let slice = controls.pin(snapshot.root.attempt_sha256)?;
        round(owner, controls, raw, snapshot, &slice)?;
        Ok(slice)
    })();
    if result.is_err() {
        retire(owner, controls);
    }
    result
}

pub(crate) struct FixedPlacement<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    held: OwnedPlacement<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    slice: FixedSlice,
}
pub(crate) struct SliceFailure<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    error: Box<PendingError>,
    held: Box<OwnedPlacement<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>>,
}
impl std::fmt::Debug for SliceFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}
impl<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    OwnedPlacement<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    pub(crate) fn bind_fixed_slice(
        mut self,
    ) -> Result<
        FixedPlacement<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        SliceFailure<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        match pin(
            &mut self.held.owner,
            &mut self.held.controls,
            &self.raw,
            &self.snapshot,
        ) {
            Ok(slice) => Ok(FixedPlacement { held: self, slice }),
            Err(error) => Err(SliceFailure {
                error: Box::new(error),
                held: Box::new(self),
            }),
        }
    }
}
impl FixedPlacement<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn revalidate(&mut self) -> Result<(), PendingError> {
        round(
            &mut self.held.held.owner,
            &mut self.held.held.controls,
            &self.held.raw,
            &self.held.snapshot,
            &self.slice,
        )
    }
}

#[cfg(test)]
#[path = "diagnostic_fixed_placement_tests.rs"]
mod tests;
