//! Linear nonexecuting join. Original refusal precedes the new sealed FD close.
use super::*;
use crate::diagnostic_artifacts::linux::held_role_census::bootstrap_images::entry::installed::entry_material::InstalledEntryBindings;
use kernel::bootstrap_entry::installed_material::InstalledEntryResource;

pub(crate) struct InstalledEntryUnit<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    owner: InstalledBootstrapUnit<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    resource: Option<InstalledEntryResource>,
    retained: Option<InstalledEntryBindings>,
    refused: bool,
}
pub(crate) struct InstalledEntryFailure<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    error: Box<BootstrapError>,
    held: Box<InstalledEntryUnit<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>>,
}
impl std::fmt::Debug for InstalledEntryFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}
impl InstalledEntryFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn error(&self) -> &BootstrapError {
        &self.error
    }
    pub(crate) fn is_refused(&self) -> bool {
        self.held.refused
    }
}
impl Checks for InstalledEntryUnit<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    type Observation = InstalledEntryBindings;
    fn refused(&self) -> bool {
        self.refused || self.owner.refused()
    }
    fn full_frame(&mut self) -> Result<(), BootstrapError> {
        self.owner.revalidate()
    }
    fn inspect(&mut self) -> Result<InstalledEntryBindings, BootstrapError> {
        let original = &self.owner.owner;
        let before = original.retained.as_ref().ok_or(BootstrapError::Binding)?;
        let entry = original.entry.as_ref().ok_or(BootstrapError::Binding)?;
        let installed = self
            .owner
            .retained
            .as_ref()
            .ok_or(BootstrapError::Binding)?;
        if self.resource.is_none() {
            if self.retained.is_some() {
                return Err(BootstrapError::Binding);
            }
            self.resource = Some(
                InstalledEntryResource::create(entry, &before.census, &before.entry, installed)
                    .map_err(ArtifactError::from)?,
            );
        }
        Ok(self
            .resource
            .as_mut()
            .ok_or(BootstrapError::Binding)?
            .inspect(entry, &before.census, &before.entry, installed)
            .map_err(ArtifactError::from)?)
    }
    fn retained(&self) -> Option<&InstalledEntryBindings> {
        self.retained.as_ref()
    }
    fn refuse(&mut self) {
        self.owner.refuse();
        self.refused = true;
    }
}
impl InstalledEntryUnit<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn revalidate(&mut self) -> Result<(), BootstrapError> {
        validate(self).map(|_| ())
    }
}
impl Drop for InstalledEntryUnit<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn drop(&mut self) {
        self.refuse();
    }
}
impl<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    InstalledBootstrapUnit<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    pub(crate) fn materialize_installed_entry(
        self,
    ) -> Result<
        InstalledEntryUnit<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        InstalledEntryFailure<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        let mut held = InstalledEntryUnit {
            owner: self,
            resource: None,
            retained: None,
            refused: false,
        };
        match validate(&mut held) {
            Ok(value) => {
                held.retained = Some(value);
                Ok(held)
            }
            Err(error) => {
                held.refuse();
                Err(InstalledEntryFailure {
                    error: Box::new(error),
                    held: Box::new(held),
                })
            }
        }
    }
}

#[cfg(test)]
#[path = "diagnostic_bootstrap_installed_entry_owner_tests.rs"]
mod tests;

#[path = "diagnostic_bootstrap_sandbox_owner.rs"]
mod sandbox_owner;
