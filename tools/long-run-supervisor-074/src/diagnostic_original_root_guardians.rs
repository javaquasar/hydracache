//! Full original owner rounds around root enrollment; no execution capability.
use super::pending::{original_descriptors, PendingError};
use super::*;
use crate::diagnostic_two_phase_launch::root_enrollment::{
    BoundGuardians, RootBindingClaims, RootSource,
};
use sha2::{Digest, Sha256};
use std::os::fd::BorrowedFd;

pub(in super::super::super) trait RootOwner {
    fn full_round(&mut self) -> Result<(), PendingError>;
    fn descriptors(&self) -> Result<[BorrowedFd<'_>; 2], PendingError>;
    fn binding(&self, identity: [u64; 2]) -> Result<RootBindingClaims, PendingError>;
    fn refuse(&mut self);
}
pub(in super::super::super) trait RootControls {
    fn observe(&mut self) -> Result<(), PendingError>;
    fn enroll_once(&mut self) -> Result<(), PendingError>;
    fn refuse(&mut self);
}
impl RootControls for BoundGuardians<'_> {
    fn observe(&mut self) -> Result<(), PendingError> {
        Ok(self.observe()?)
    }
    fn enroll_once(&mut self) -> Result<(), PendingError> {
        Ok(self.enroll_once()?)
    }
    fn refuse(&mut self) {
        self.refuse();
    }
}
// Test probes use this exact join with the original complete frame. The kernel
// adapter is separately exercised on real root-owned cgroup2, not fake files.
pub(in super::super::super) fn join(
    owner: &mut impl RootOwner,
    controls: &mut impl RootControls,
    enroll: bool,
) -> Result<(), PendingError> {
    let result = (|| {
        controls.observe()?;
        owner.full_round()?;
        controls.observe()?;
        if enroll {
            controls.enroll_once()?;
            owner.full_round()?;
            controls.observe()?;
        }
        Ok(())
    })();
    if result.is_err() {
        owner.refuse();
        controls.refuse();
    }
    result
}
pub(super) fn derive(
    binding: &Binding,
    identity: [u64; 2],
) -> Result<RootBindingClaims, PendingError> {
    let source = &binding.source;
    let encoded = serde_json::to_vec(&(
        "hc074-original-root-context-v1",
        source.owner,
        &source.state,
        source.origins,
        source.bounds,
        source.descriptors,
        source.guardians,
        &source.names,
        source.serial.get(),
        binding.addressed,
        identity,
    ))
    .map_err(|_| MessageError::Binding)?;
    let context = Sha256::digest(encoded).into();
    let attempt = Sha256::digest(
        serde_json::to_vec(&("hc074-original-root-attempt-v1", &source.state.identity))
            .map_err(|_| MessageError::Binding)?,
    )
    .into();
    let value = RootBindingClaims {
        attempt_sha256: attempt,
        original_context_sha256: context,
        root_device: identity[0],
        root_inode: identity[1],
        deadline_boottime_ns: source.bounds[3],
    };
    // Reject zero/invalid fields with the existing packet validator.
    crate::diagnostic_two_phase_launch::root_enrollment::PacketClaims {
        kind: crate::diagnostic_two_phase_launch::root_enrollment::Kind::Enroll,
        binding: value,
    }
    .encode()
    .map_err(|_| MessageError::Binding)?;
    Ok(value)
}
fn prepare<'root>(
    owner: &mut impl RootOwner,
    root: &'root mut RootSource,
) -> Result<BoundGuardians<'root>, PendingError> {
    owner.full_round()?;
    let binding = owner.binding(root.identity())?;
    let [fence, alarm] = owner.descriptors()?;
    let mut controls = root.prepare(fence, alarm, binding)?;
    controls.observe()?;
    owner.full_round()?;
    controls.observe()?;
    Ok(controls)
}
impl RootOwner for OriginalWatchdogContext<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn full_round(&mut self) -> Result<(), PendingError> {
        Ok(self.revalidate()?)
    }
    fn descriptors(&self) -> Result<[BorrowedFd<'_>; 2], PendingError> {
        original_descriptors(&self.original.original.original, &self.binding)
    }
    fn binding(&self, identity: [u64; 2]) -> Result<RootBindingClaims, PendingError> {
        derive(&self.binding, identity)
    }
    fn refuse(&mut self) {
        self.original.refuse();
    }
}

pub(crate) struct OriginalRootGuardians<
    'root,
    'held,
    'join,
    'storage,
    'o,
    't,
    'r,
    'c,
    'b,
    's,
    'a,
    'p,
> {
    controls: BoundGuardians<'root>,
    original: OriginalWatchdogContext<'held, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
}
pub(crate) struct RootFailure<'held, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    error: Box<PendingError>,
    held: Box<OriginalWatchdogContext<'held, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>>,
}
impl std::fmt::Debug for RootFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}
impl OriginalRootGuardians<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn revalidate(&mut self) -> Result<(), PendingError> {
        join(&mut self.original, &mut self.controls, false)
    }
    pub(crate) fn enroll_once(&mut self) -> Result<(), PendingError> {
        join(&mut self.original, &mut self.controls, true)
    }
}
impl<'held, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    OriginalWatchdogContext<'held, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    pub(crate) fn prepare_root_guardians<'root>(
        mut self,
        root: &'root mut RootSource,
    ) -> Result<
        OriginalRootGuardians<'root, 'held, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        RootFailure<'held, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        match prepare(&mut self, root) {
            Ok(controls) => Ok(OriginalRootGuardians {
                controls,
                original: self,
            }),
            Err(error) => {
                RootOwner::refuse(&mut self);
                Err(RootFailure {
                    error: Box::new(error),
                    held: Box::new(self),
                })
            }
        }
    }
}

pub(in super::super::super) fn retire(
    owner: &mut impl RootOwner,
    controls: &mut impl RootControls,
) {
    // Returning the exclusive borrow must never reopen this original attempt.
    // Refusal precedes field Drop; active kernel holders still revoke and fence.
    owner.refuse();
    controls.refuse();
}
impl Drop for OriginalRootGuardians<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn drop(&mut self) {
        retire(&mut self.original, &mut self.controls);
    }
}

#[cfg(test)]
#[path = "diagnostic_original_root_retirement_tests.rs"]
mod retirement_tests;

#[path = "diagnostic_owned_root_manager.rs"]
mod owned;
