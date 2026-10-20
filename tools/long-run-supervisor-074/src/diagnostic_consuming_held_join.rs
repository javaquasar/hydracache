//! Consumed original fixed placement plus exact retained role provenance.
//! No original message, descriptor, role, source or retry owner can be extracted.
#[cfg(test)]
#[path = "../tests/support/consuming_held_live_api.rs"]
mod live_api;
use super::*;
use crate::diagnostic_artifacts::{ArtifactError, MAX_RECEIPT_BYTES};
use crate::diagnostic_builder::{
    held_build::{role_binding, role_files::PinnedRoles},
    MAX_POLICY_BYTES,
};
use crate::diagnostic_worker_policy::trusted_input::controller_input::held_runtime::{
    late_join::LateCheck, HeldTrustError, HeldTrustInput,
};

#[derive(Debug, thiserror::Error)]
pub(crate) enum HeldJoinError {
    #[error("full original placement frame refused: {0}")]
    Frame(#[from] PendingError),
    #[error("held source/provenance refused: {0}")]
    Trust(#[from] HeldTrustError),
    #[error("retained role content refused: {0}")]
    Roles(#[from] ArtifactError),
    #[error("held join documents are missing, changed or over budget")]
    Binding,
    #[error("consuming held join previously refused")]
    Refused,
}
pub(crate) struct JoinDocuments<'a> {
    pub(crate) held_policy: &'a [u8],
    pub(crate) worker_policy: &'a [u8],
    pub(crate) receipt: &'a [u8],
    pub(crate) binding: &'a [u8],
}
struct Documents {
    values: [Vec<u8>; 4],
}
impl Documents {
    fn copy(input: JoinDocuments<'_>) -> Result<Self, HeldJoinError> {
        let values = [
            input.held_policy,
            input.worker_policy,
            input.receipt,
            input.binding,
        ];
        for (raw, max) in values.iter().zip([
            MAX_POLICY_BYTES,
            MAX_POLICY_BYTES,
            MAX_RECEIPT_BYTES as usize,
            role_binding::MAX_ENVELOPE_BYTES,
        ]) {
            if raw.is_empty() || raw.len() > max {
                return Err(HeldJoinError::Binding);
            }
        }
        Ok(Self {
            values: values.map(<[u8]>::to_vec),
        })
    }
    fn slices(&self) -> [&[u8]; 4] {
        self.values.each_ref().map(Vec::as_slice)
    }
}
trait Checks {
    fn is_refused(&self) -> bool;
    fn frame(&mut self) -> Result<(), HeldJoinError>;
    fn trust(&mut self) -> Result<(), HeldJoinError>;
    fn roles(&mut self) -> Result<(), HeldJoinError>;
    fn refuse(&mut self);
}
fn validate(checks: &mut impl Checks) -> Result<(), HeldJoinError> {
    let result = (|| {
        if checks.is_refused() {
            return Err(HeldJoinError::Refused);
        }
        for _ in 0..2 {
            checks.frame()?;
            checks.trust()?;
            checks.roles()?;
            // Unchanged complete frame: its closing clocks follow all hashing.
            checks.frame()?;
        }
        Ok(())
    })();
    if result.is_err() {
        checks.refuse();
    }
    result
}

pub(crate) struct JoinedHeldPlacement<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    // Field drop order is deliberate, in addition to explicit retirement in Drop.
    placement: FixedPlacement<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    roles: PinnedRoles,
    trust: HeldTrustInput,
    documents: Option<Documents>,
    evidence: Option<LateCheck>,
    refused: bool,
}
pub(crate) struct HeldJoinFailure<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    error: Box<HeldJoinError>,
    held: Box<JoinedHeldPlacement<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>>,
}
impl std::fmt::Debug for HeldJoinFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}
impl HeldJoinFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn error(&self) -> &HeldJoinError {
        &self.error
    }
    pub(crate) fn is_refused(&self) -> bool {
        self.held.refused
    }
}
impl Checks for JoinedHeldPlacement<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn is_refused(&self) -> bool {
        self.refused || self.trust.is_refused()
    }
    fn frame(&mut self) -> Result<(), HeldJoinError> {
        Ok(self.placement.revalidate()?)
    }
    fn trust(&mut self) -> Result<(), HeldJoinError> {
        let documents = self.documents.as_ref().ok_or(HeldJoinError::Binding)?;
        let owner = &mut self.placement.held.held.owner;
        let material = &mut owner
            .original
            .original
            .original
            .original
            .original
            .transport
            .material;
        let prepared = &mut material.outputs.pipes.starting.original.original.prepared;
        let controller = &mut prepared
            .original
            .original
            .reservation
            .original
            .original
            .original;
        let current = self.trust.inspect_late_join(
            controller,
            documents.slices(),
            &self.placement.held.raw,
        )?;
        if self
            .evidence
            .as_ref()
            .is_some_and(|old| !current.same_original(old))
        {
            return Err(HeldJoinError::Binding);
        }
        self.evidence = Some(current);
        Ok(())
    }
    fn roles(&mut self) -> Result<(), HeldJoinError> {
        Ok(self
            .evidence
            .as_ref()
            .ok_or(HeldJoinError::Binding)?
            .revalidate_roles(&mut self.roles)?)
    }
    fn refuse(&mut self) {
        retire(
            &mut self.placement.held.held.owner,
            &mut self.placement.held.held.controls,
        );
        self.roles.refuse_build_join();
        self.trust.refuse_late_join();
        self.refused = true;
    }
}
impl JoinedHeldPlacement<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn revalidate(&mut self) -> Result<(), HeldJoinError> {
        validate(self)
    }
}
impl Drop for JoinedHeldPlacement<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn drop(&mut self) {
        self.refuse();
    }
}
impl<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    FixedPlacement<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    pub(crate) fn join_held_roles(
        self,
        roles: PinnedRoles,
        trust: HeldTrustInput,
        documents: JoinDocuments<'_>,
    ) -> Result<
        JoinedHeldPlacement<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        HeldJoinFailure<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        let mut held = JoinedHeldPlacement {
            placement: self,
            roles,
            trust,
            documents: None,
            evidence: None,
            refused: false,
        };
        let result = Documents::copy(documents).and_then(|documents| {
            held.documents = Some(documents);
            held.revalidate()
        });
        match result {
            Ok(()) => Ok(held),
            Err(error) => {
                held.refuse();
                Err(HeldJoinFailure {
                    error: Box::new(error),
                    held: Box::new(held),
                })
            }
        }
    }
}

#[cfg(test)]
#[path = "../tests/support/consuming_held_join.rs"]
mod consuming_held_tests;
