//! Private provider join beneath the consumed owner; no callback or FD export.
use super::*;
use crate::diagnostic_artifacts::linux::held_role_census::Census;
pub(super) use crate::diagnostic_artifacts::linux::held_role_census::CompleteBindings;

pub(super) fn capture(
    owner: &MaterializedHeldUnit<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    context: &Context,
) -> Result<Census, MaterialError> {
    let mut census = Census::new(
        context.worker.identity,
        &context.worker.sha256,
        context.guardians,
        context.owner[0],
    )?;
    let held = &owner.owner.owner.placement.held.held;
    let material = &held
        .owner
        .original
        .original
        .original
        .original
        .original
        .transport
        .material;
    material
        .bundle
        .as_ref()
        .ok_or(MaterialError::Binding)?
        .contribute_held_worker(&mut census)
        .map_err(HeldJoinError::from)?;
    held.controls.contribute_held_guardians(&mut census)?;
    Ok(census)
}
