//! Private fresh checks for the consuming holder, never detached live authority.
use super::*;
use held::role_files::PinnedRoles;

pub(crate) struct LateCheck {
    observed: Observed,
    documents: [String; 6],
    build: held::VerifiedBuild,
    artifacts: crate::diagnostic_two_phase_launch::LaunchArtifacts,
}
impl LateCheck {
    pub(crate) fn same_original(&self, retained: &Self) -> bool {
        self.observed == retained.observed && self.documents == retained.documents
    }
    pub(crate) fn revalidate_roles(&self, roles: &mut PinnedRoles) -> Result<(), ArtifactError> {
        roles.revalidate_build_join(&self.build, &self.artifacts)
    }
}
impl HeldTrustInput {
    pub(crate) fn refuse_late_join(&mut self) {
        self.refused = true;
    }
    pub(crate) fn inspect_late_join(
        &mut self,
        original: &mut SignedControllerInput<'_, '_, '_, '_, '_>,
        documents: [&[u8]; 4],
        placement: &[u8],
    ) -> Result<LateCheck, HeldTrustError> {
        let [held_policy, worker_policy, receipt, binding] = documents;
        let mut guard = self.bind_original(original, held_policy, worker_policy)?;
        let result = (|| {
            let inspection = guard.inspect_request_binding(binding, receipt)?;
            inspection.matches_placement_bytes(placement)?;
            let policy = guard.checked_policy()?;
            let build = held::verify_receipt(receipt, &policy)?;
            let doc: role_binding::SignedBinding =
                serde_json::from_slice(binding).map_err(|_| HeldTrustError::ContextMismatch)?;
            let observed = Observed::capture(guard.original)?;
            let documents = [
                crate::sha256_hex(held_policy),
                crate::sha256_hex(worker_policy),
                crate::sha256_hex(receipt),
                crate::sha256_hex(binding),
                crate::sha256_hex(placement),
                crate::sha256_hex(&guard.source.source.snapshot.raw),
            ];
            guard.revalidate()?;
            if Observed::capture(guard.original)? != observed {
                return Err(HeldTrustError::ContextMismatch);
            }
            Ok(LateCheck {
                observed,
                documents,
                build,
                artifacts: doc.binding.placement.placement.launch_claims.artifacts,
            })
        })();
        if result.is_err() {
            guard.poison();
        }
        result
    }
}
