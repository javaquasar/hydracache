//! Consumes original aliases; no original delivery, completion or unlock.
use super::*;

#[cfg(test)]
#[path = "diagnostic_owned_root_manager_test_api.rs"]
mod test_api;

struct OwnedOwner<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    original: OriginalManagerMessage<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    binding: Option<Binding>,
}
impl RootOwner for OwnedOwner<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn full_round(&mut self) -> Result<(), PendingError> {
        let binding = self.binding.as_ref().ok_or(MessageError::Refused)?;
        self.original.watchdog_context_round(Some(binding))?;
        Ok(())
    }
    fn descriptors(&self) -> Result<[BorrowedFd<'_>; 2], PendingError> {
        original_descriptors(
            &self.original.original.original,
            self.binding.as_ref().ok_or(MessageError::Refused)?,
        )
    }
    fn binding(&self, identity: [u64; 2]) -> Result<RootBindingClaims, PendingError> {
        derive(
            self.binding.as_ref().ok_or(MessageError::Refused)?,
            identity,
        )
    }
    fn refuse(&mut self) {
        self.original.refuse();
    }
}

pub(crate) struct OwnedRootGuardians<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    // Original message aliases and collector/custodian cancellation precede
    // returning the root borrow. Independent enrolled guards retain the fence.
    owner: OwnedOwner<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    controls: BoundGuardians<'root>,
}
pub(crate) struct OwnedRootFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    error: Box<PendingError>,
    held: Box<OwnedOwner<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>>,
}
impl std::fmt::Debug for OwnedRootFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}
impl OwnedRootGuardians<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn revalidate(&mut self) -> Result<(), PendingError> {
        join(&mut self.owner, &mut self.controls, false)
    }
    pub(crate) fn enroll_once(&mut self) -> Result<(), PendingError> {
        join(&mut self.owner, &mut self.controls, true)
    }
}
impl<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    OriginalManagerMessage<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    pub(crate) fn prepare_owned_root_guardians<'root>(
        self,
        root: &'root mut RootSource,
    ) -> Result<
        OwnedRootGuardians<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        OwnedRootFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        let mut owner = OwnedOwner {
            original: self,
            binding: None,
        };
        let result = match owner.original.watchdog_context_round(None) {
            Ok(binding) => {
                owner.binding = Some(binding);
                prepare(&mut owner, root)
            }
            Err(error) => Err(error.into()),
        };
        match result {
            Ok(controls) => Ok(OwnedRootGuardians { owner, controls }),
            Err(error) => {
                owner.refuse();
                Err(OwnedRootFailure {
                    error: Box::new(error),
                    held: Box::new(owner),
                })
            }
        }
    }
}
impl Drop for OwnedRootGuardians<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn drop(&mut self) {
        retire(&mut self.owner, &mut self.controls);
    }
}

#[cfg(test)]
#[test]
fn owned_root_manager_requires_consumed_original_message_and_root_borrow() {
    fn signature(
        original: OriginalManagerMessage<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
        root: &mut RootSource,
    ) {
        if let Ok(mut held) = original.prepare_owned_root_guardians(root) {
            let _ = held.revalidate();
            let _ = held.enroll_once();
        }
    }
    let _contract = signature;
}

#[cfg(test)]
#[path = "diagnostic_owned_placement_tests.rs"]
mod placement_tests;

#[path = "diagnostic_owned_placement.rs"]
mod placement;
