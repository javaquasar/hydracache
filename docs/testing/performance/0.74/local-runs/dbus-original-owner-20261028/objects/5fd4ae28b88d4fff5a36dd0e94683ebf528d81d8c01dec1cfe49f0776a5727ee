//! Same consumed owner constraints, not a held unit, delivery or artifact permit.
use super::*;
use crate::diagnostic_start_consent::StartConsentBody;
use crate::diagnostic_two_phase_launch::held_placement::{
    inspect_placement_signature, MAX_ENVELOPE_BYTES,
};
use crate::diagnostic_two_phase_launch::root_enrollment::{Kind, PacketClaims};
use ed25519_dalek::VerifyingKey;

pub(super) trait PlacementOwner: RootOwner {
    fn inspect(&self, raw: &[u8], root: RootBindingClaims) -> Result<(), PendingError>;
}
pub(super) trait PlacementControls: RootControls {
    fn binding_for_placement(&mut self) -> Result<RootBindingClaims, PendingError>;
}
pub(super) fn authenticate(
    raw: &[u8],
    key: &VerifyingKey,
    consent: &StartConsentBody,
    root: RootBindingClaims,
) -> Result<(), PendingError> {
    let claims = inspect_placement_signature(raw, key).map_err(|_| MessageError::Binding)?;
    let packet = PacketClaims {
        kind: Kind::Enroll,
        binding: root,
    }
    .encode()
    .map_err(|_| MessageError::Binding)?;
    let expected: String = packet.iter().map(|b| format!("{b:02x}")).collect();
    if claims.root_enrollment_hex != expected
        || crate::canonical_json(&claims.launch_claims.consent_claims)
            .map_err(|_| MessageError::Binding)?
            != crate::canonical_json(consent).map_err(|_| MessageError::Binding)?
    {
        return Err(MessageError::Binding.into());
    }
    Ok(())
}
impl PlacementOwner for OwnedOwner<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn inspect(&self, raw: &[u8], root: RootBindingClaims) -> Result<(), PendingError> {
        let material = &self
            .original
            .original
            .original
            .original
            .original
            .transport
            .material;
        let prepared = &material.outputs.pipes.starting.original.original.prepared;
        let controller = &prepared
            .original
            .original
            .reservation
            .original
            .original
            .original;
        // Select only the SAME retained policy key, never a caller-selected issuer.
        // Its live authority and original consent still pass the unchanged full frame.
        authenticate(
            raw,
            &controller.controller.source.policy.verifying_key,
            prepared.consent_to_sign(),
            root,
        )
    }
}
impl PlacementControls for BoundGuardians<'_> {
    fn binding_for_placement(&mut self) -> Result<RootBindingClaims, PendingError> {
        Ok(self.binding_for_placement()?)
    }
}
#[derive(PartialEq, Eq)]
pub(super) struct Snapshot {
    root: RootBindingClaims,
    envelope_sha256: String,
}
struct CheckedOwner<'a, O> {
    owner: &'a mut O,
    raw: &'a [u8],
    root: RootBindingClaims,
}
impl<O: PlacementOwner> CheckedOwner<'_, O> {
    fn constraints(&self) -> Result<(), PendingError> {
        if self
            .owner
            .binding([self.root.root_device, self.root.root_inode])?
            != self.root
        {
            return Err(MessageError::Binding.into());
        }
        self.owner.inspect(self.raw, self.root)
    }
}
impl<O: PlacementOwner> RootOwner for CheckedOwner<'_, O> {
    fn full_round(&mut self) -> Result<(), PendingError> {
        self.constraints()?;
        self.owner.full_round()?;
        self.constraints()
    }
    fn descriptors(&self) -> Result<[BorrowedFd<'_>; 2], PendingError> {
        self.owner.descriptors()
    }
    fn binding(&self, identity: [u64; 2]) -> Result<RootBindingClaims, PendingError> {
        self.owner.binding(identity)
    }
    fn refuse(&mut self) {
        self.owner.refuse();
    }
}
pub(super) fn placement_join(
    owner: &mut impl PlacementOwner,
    controls: &mut impl PlacementControls,
    raw: &[u8],
    retained: Option<&Snapshot>,
) -> Result<Snapshot, PendingError> {
    let result = (|| {
        if raw.is_empty() || raw.len() > MAX_ENVELOPE_BYTES {
            return Err(MessageError::Binding.into());
        }
        let root = controls.binding_for_placement()?;
        let snapshot = Snapshot {
            root,
            envelope_sha256: crate::sha256_hex(raw),
        };
        if retained.is_some_and(|old| old != &snapshot) {
            return Err(MessageError::Binding.into());
        }
        // Reuse the exact old root join and complete original full round once.
        // New pure comparisons do not replace a live observation or a deadline.
        join(&mut CheckedOwner { owner, raw, root }, controls, false)?;
        if controls.binding_for_placement()? != root {
            return Err(MessageError::Binding.into());
        }
        Ok(snapshot)
    })();
    if result.is_err() {
        retire(owner, controls);
    }
    result
}

pub(crate) struct OwnedPlacement<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    held: OwnedRootGuardians<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    raw: Vec<u8>,
    snapshot: Snapshot,
}
pub(crate) struct PlacementFailure<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    error: Box<PendingError>,
    held: Box<OwnedRootGuardians<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>>,
}
impl std::fmt::Debug for PlacementFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}
impl<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    OwnedRootGuardians<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    pub(crate) fn bind_signed_placement(
        mut self,
        raw: Vec<u8>,
    ) -> Result<
        OwnedPlacement<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        PlacementFailure<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        match placement_join(&mut self.owner, &mut self.controls, &raw, None) {
            Ok(snapshot) => Ok(OwnedPlacement {
                held: self,
                raw,
                snapshot,
            }),
            Err(error) => Err(PlacementFailure {
                error: Box::new(error),
                held: Box::new(self),
            }),
        }
    }
}
impl OwnedPlacement<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn revalidate(&mut self) -> Result<(), PendingError> {
        placement_join(
            &mut self.held.owner,
            &mut self.held.controls,
            &self.raw,
            Some(&self.snapshot),
        )
        .map(|_| ())
    }
}
// No custom Drop is needed: the complete old consuming holder retires its owner
// before root controls and destroys original writer aliases in its proven order.

#[cfg(test)]
#[path = "diagnostic_owned_placement_fixture_api.rs"]
mod fixture_api;

#[path = "diagnostic_fixed_placement.rs"]
mod fixed_slice;
