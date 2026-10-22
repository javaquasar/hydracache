//! Owned kernel material only. No delivery, release, execution or FD extraction.
use super::*;
#[path = "diagnostic_held_unit_material_kernel.rs"]
mod kernel;
use kernel::{Bindings, Resources, Stamp};

#[derive(Debug, thiserror::Error)]
pub(crate) enum MaterialError {
    #[error("original held projection refused: {0}")]
    Frame(#[from] HeldJoinError),
    #[error("held kernel material refused: {0}")]
    Kernel(#[from] std::io::Error),
    #[error("held material or expanded descriptor inventory differs")]
    Binding,
    #[error("held material owner previously refused")]
    Refused,
}
fn bindings(c: &Context) -> Bindings {
    Bindings {
        owner: c.owner,
        deadline: c.bounds[3],
        guardians: c.guardians,
        worker: (c.worker.identity, c.worker.sha256.clone()),
        config: (c.config.identity, c.config.sha256.clone()),
        streams: c.streams,
    }
}
#[derive(PartialEq, Eq)]
struct Inventory {
    context: Context,
    descriptors: Vec<Stamp>,
    roles: kernel::RoleBindings,
}
trait MaterialOwner {
    fn refused(&self) -> bool;
    fn full_frame(&mut self) -> Result<(), MaterialError>;
    fn prepare(&mut self) -> Result<(), MaterialError>;
    fn inspect(&self) -> Result<Inventory, MaterialError>;
    fn retained(&self) -> Option<&Inventory>;
    fn refuse(&mut self);
}
fn validate_material(owner: &mut impl MaterialOwner) -> Result<Inventory, MaterialError> {
    let result = (|| {
        if owner.refused() {
            return Err(MaterialError::Refused);
        }
        owner.full_frame()?;
        owner.prepare()?;
        let opening = owner.inspect()?;
        if owner.retained().is_some_and(|old| old != &opening) {
            return Err(MaterialError::Binding);
        }
        owner.full_frame()?;
        if owner.inspect()? != opening {
            return Err(MaterialError::Binding);
        }
        Ok(opening)
    })();
    if result.is_err() {
        owner.refuse();
    }
    result
}
pub(crate) struct MaterializedHeldUnit<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    // Explicit retirement in Drop precedes closing NEW objects, then old owner.
    resources: Option<Resources>,
    owner: ProjectedHeldUnit<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    inventory: Option<Inventory>,
    refused: bool,
}
pub(crate) struct MaterialFailure<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    error: Box<MaterialError>,
    held: Box<MaterializedHeldUnit<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>>,
}
impl std::fmt::Debug for MaterialFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}
impl MaterialFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn error(&self) -> &MaterialError {
        &self.error
    }
    pub(crate) fn is_refused(&self) -> bool {
        self.held.refused
    }
}
impl MaterialOwner for MaterializedHeldUnit<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn refused(&self) -> bool {
        self.refused || self.owner.owner.is_refused()
    }
    fn full_frame(&mut self) -> Result<(), MaterialError> {
        Ok(self.owner.revalidate()?)
    }
    fn prepare(&mut self) -> Result<(), MaterialError> {
        if self.resources.is_some() {
            return Ok(());
        }
        if self.inventory.is_some() {
            return Err(MaterialError::Binding);
        }
        let context = self.owner.owner.context()?;
        if context != self.owner.projection.context {
            return Err(MaterialError::Binding);
        }
        let material = &self
            .owner
            .owner
            .placement
            .held
            .held
            .owner
            .original
            .original
            .original
            .original
            .original
            .transport
            .material;
        let surface = cell_intent(&context.state)
            .map_err(|_| MaterialError::Binding)?
            .surface;
        let config = material
            .bundle
            .as_ref()
            .ok_or(MaterialError::Binding)?
            .copy_material_config(&surface, context.config.identity, &context.config.sha256)
            .map_err(HeldJoinError::from)?;
        self.resources = Some(Resources::create(&bindings(&context), &config)?);
        Ok(())
    }
    fn inspect(&self) -> Result<Inventory, MaterialError> {
        let context = self.owner.owner.context()?;
        if context != self.owner.projection.context {
            return Err(MaterialError::Binding);
        }
        let original = std::array::from_fn(|i| match i {
            0..=9 => context.inventory[i],
            10..=11 => context.addressed[i - 10],
            _ => context.custody[i - 12],
        });
        let descriptors = self
            .resources
            .as_ref()
            .ok_or(MaterialError::Binding)?
            .inspect(&bindings(&context), original)?;
        // Pure binding of the SAME fresh census. No new FD, bus/time read or
        // authority is introduced; the context pins the original channel names.
        let roles = kernel::bind_roles(&bindings(&context), &descriptors)?;
        Ok(Inventory {
            context,
            descriptors,
            roles,
        })
    }
    fn retained(&self) -> Option<&Inventory> {
        self.inventory.as_ref()
    }
    fn refuse(&mut self) {
        self.owner.owner.refuse();
        self.refused = true;
    }
}
impl MaterializedHeldUnit<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn revalidate(&mut self) -> Result<(), MaterialError> {
        if self.refused() {
            return Err(MaterialError::Refused);
        }
        if self.inventory.is_none() {
            self.refuse();
            return Err(MaterialError::Binding);
        }
        validate_material(self).map(|_| ())
    }
}
impl Drop for MaterializedHeldUnit<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn drop(&mut self) {
        self.refuse();
    }
}
impl<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    ProjectedHeldUnit<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    pub(crate) fn materialize_held_unit(
        self,
    ) -> Result<
        MaterializedHeldUnit<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        MaterialFailure<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        let mut held = MaterializedHeldUnit {
            resources: None,
            owner: self,
            inventory: None,
            refused: false,
        };
        // Every old full predicate remains, with fresh extended checks on both sides.
        match validate_material(&mut held) {
            Ok(inventory) => {
                held.inventory = Some(inventory);
                Ok(held)
            }
            Err(error) => {
                held.refuse();
                Err(MaterialFailure {
                    error: Box::new(error),
                    held: Box::new(held),
                })
            }
        }
    }
}

#[cfg(test)]
#[path = "diagnostic_held_unit_material_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/support/held_material_live_api.rs"]
mod live_api;
