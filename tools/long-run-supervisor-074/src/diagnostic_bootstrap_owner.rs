//! Consumes the SAME material owner. No start, channel, owner or FD extraction.
use super::*;
use crate::diagnostic_builder::{bootstrap_build, MAX_POLICY_BYTES};
use crate::diagnostic_artifacts::{ArtifactError, MAX_RECEIPT_BYTES};
use crate::diagnostic_worker_policy::trusted_input::controller_input::held_runtime::bootstrap_enrollment::selected_build::SelectionReceipt;
use std::fs::File;

#[derive(Debug, thiserror::Error)]
pub(crate) enum BootstrapError {
    #[error("original complete material frame refused: {0}")]
    Frame(#[from] MaterialError),
    #[error("protected bootstrap trust refused: {0}")]
    Trust(#[from] crate::diagnostic_worker_policy::trusted_input::controller_input::held_runtime::HeldTrustError),
    #[error("bootstrap provenance or sealed contents refused: {0}")]
    Build(#[from] ArtifactError),
    #[error("bootstrap original context, documents or retained input changed")]
    Binding,
}
trait Checks {
    type Observation: PartialEq;
    fn refused(&self) -> bool;
    fn full_frame(&mut self) -> Result<(), BootstrapError>;
    fn inspect(&mut self) -> Result<Self::Observation, BootstrapError>;
    fn retained(&self) -> Option<&Self::Observation>;
    fn refuse(&mut self);
}
fn validate<C: Checks>(owner: &mut C) -> Result<C::Observation, BootstrapError> {
    let result = (|| {
        if owner.refused() {
            return Err(BootstrapError::Binding);
        }
        owner.full_frame()?;
        let opening = owner.inspect()?;
        if owner.retained().is_some_and(|old| old != &opening) {
            return Err(BootstrapError::Binding);
        }
        owner.full_frame()?;
        if owner.inspect()? != opening {
            return Err(BootstrapError::Binding);
        }
        owner.full_frame()?;
        Ok(opening)
    })();
    if result.is_err() {
        owner.refuse();
    }
    result
}
#[derive(PartialEq, Eq)]
struct Observation {
    context: Context,
    image: bootstrap_build::sealed::Stamp,
}
pub(crate) struct BootstrapOwnedUnit<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    // Explicit retirement and field order both keep the original before new FD close.
    owner: MaterializedHeldUnit<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    image: File,
    documents: Option<[Vec<u8>; 2]>,
    selection: Option<SelectionReceipt>,
    retained: Option<Observation>,
    refused: bool,
}
pub(crate) struct BootstrapFailure<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    error: Box<BootstrapError>,
    held: Box<BootstrapOwnedUnit<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>>,
}
impl std::fmt::Debug for BootstrapFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}
impl BootstrapFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn error(&self) -> &BootstrapError {
        &self.error
    }
    pub(crate) fn is_refused(&self) -> bool {
        self.held.refused
    }
}
impl Checks for BootstrapOwnedUnit<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    type Observation = Observation;
    fn refused(&self) -> bool {
        self.refused || self.owner.refused()
    }
    fn full_frame(&mut self) -> Result<(), BootstrapError> {
        Ok(self.owner.revalidate()?)
    }
    fn inspect(&mut self) -> Result<Observation, BootstrapError> {
        let context = self
            .owner
            .owner
            .owner
            .context()
            .map_err(MaterialError::from)?;
        if self
            .owner
            .inventory
            .as_ref()
            .is_none_or(|old| old.context != context)
        {
            return Err(BootstrapError::Binding);
        }
        let docs = self.documents.as_ref().ok_or(BootstrapError::Binding)?;
        let selected = self.owner.owner.owner.trust.inspect_bootstrap_build(
            &docs[0],
            &docs[1],
            [
                &context.documents_sha256[0],
                &context.documents_sha256[2],
                &context.host.sha256,
                &context.launcher.sha256,
            ],
            self.selection.as_ref(),
        )?;
        let image = bootstrap_build::sealed::inspect(&self.image, &selected.build)?;
        self.selection = Some(selected);
        Ok(Observation { context, image })
    }
    fn retained(&self) -> Option<&Observation> {
        self.retained.as_ref()
    }
    fn refuse(&mut self) {
        self.owner.refuse();
        self.owner.owner.owner.trust.refuse_late_join();
        self.refused = true;
    }
}
impl BootstrapOwnedUnit<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn revalidate(&mut self) -> Result<(), BootstrapError> {
        validate(self).map(|_| ())
    }
}
impl Drop for BootstrapOwnedUnit<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn drop(&mut self) {
        self.refuse();
    }
}
impl<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    MaterializedHeldUnit<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    pub(crate) fn join_bootstrap(
        self,
        image: File,
        policy: &[u8],
        receipt: &[u8],
    ) -> Result<
        BootstrapOwnedUnit<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        BootstrapFailure<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        let mut held = BootstrapOwnedUnit {
            owner: self,
            image,
            documents: None,
            selection: None,
            retained: None,
            refused: false,
        };
        let result = (|| {
            if policy.is_empty()
                || policy.len() > MAX_POLICY_BYTES
                || receipt.is_empty()
                || receipt.len() as u64 > MAX_RECEIPT_BYTES
            {
                return Err(BootstrapError::Binding);
            }
            held.documents = Some([policy.to_vec(), receipt.to_vec()]);
            held.retained = Some(validate(&mut held)?);
            Ok(())
        })();
        match result {
            Ok(()) => Ok(held),
            Err(error) => {
                held.refuse();
                Err(BootstrapFailure {
                    error: Box::new(error),
                    held: Box::new(held),
                })
            }
        }
    }
}

#[cfg(test)]
#[path = "diagnostic_bootstrap_owner_tests.rs"]
mod tests;
