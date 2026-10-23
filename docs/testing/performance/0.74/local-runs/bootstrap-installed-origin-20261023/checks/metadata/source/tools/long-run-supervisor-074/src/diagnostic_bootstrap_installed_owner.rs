//! Consumes the same bootstrap owner; no runnable spec, descriptor or permit export.
use super::*;
use crate::diagnostic_artifacts::linux::held_role_census::bootstrap_images::entry::installed::InstalledBindings;
use bootstrap_build::installed::InstalledSource;
pub(crate) struct InstalledBootstrapUnit<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    // Original retirement occurs before the installed image/ancestry FDs close.
    owner: BootstrapOwnedUnit<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    source: Option<InstalledSource>,
    retained: Option<InstalledBindings>,
    refused: bool,
}
pub(crate) struct InstalledFailure<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    error: Box<BootstrapError>,
    held: Box<InstalledBootstrapUnit<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>>,
}
impl std::fmt::Debug for InstalledFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}
impl InstalledFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn error(&self) -> &BootstrapError {
        &self.error
    }
    pub(crate) fn is_refused(&self) -> bool {
        self.held.refused
    }
}
impl Checks for InstalledBootstrapUnit<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    type Observation = InstalledBindings;
    fn refused(&self) -> bool {
        self.refused || self.owner.refused()
    }
    fn full_frame(&mut self) -> Result<(), BootstrapError> {
        self.owner.revalidate()
    }
    fn inspect(&mut self) -> Result<InstalledBindings, BootstrapError> {
        let build = &self
            .owner
            .selection
            .as_ref()
            .ok_or(BootstrapError::Binding)?
            .build;
        if self.source.is_none() {
            if self.retained.is_some() {
                return Err(BootstrapError::Binding);
            }
            self.source = Some(InstalledSource::open_fixed(build)?);
        }
        let original = self
            .owner
            .retained
            .as_ref()
            .ok_or(BootstrapError::Binding)?;
        Ok(self
            .source
            .as_mut()
            .ok_or(BootstrapError::Binding)?
            .inspect(build, &original.entry)?)
    }
    fn retained(&self) -> Option<&InstalledBindings> {
        self.retained.as_ref()
    }
    fn refuse(&mut self) {
        self.owner.refuse();
        self.refused = true;
    }
}
impl InstalledBootstrapUnit<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn revalidate(&mut self) -> Result<(), BootstrapError> {
        validate(self).map(|_| ())
    }
}
impl Drop for InstalledBootstrapUnit<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn drop(&mut self) {
        self.refuse();
    }
}
impl<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    BootstrapOwnedUnit<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    pub(crate) fn bind_installed_origin(
        self,
    ) -> Result<
        InstalledBootstrapUnit<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        InstalledFailure<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        let mut held = InstalledBootstrapUnit {
            owner: self,
            source: None,
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
                Err(InstalledFailure {
                    error: Box::new(error),
                    held: Box::new(held),
                })
            }
        }
    }
}

#[cfg(test)]
#[path = "diagnostic_bootstrap_installed_owner_tests.rs"]
mod tests;
