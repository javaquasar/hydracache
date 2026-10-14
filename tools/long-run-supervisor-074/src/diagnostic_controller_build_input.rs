//! Reviewed builder statement bound to the original request, not run provenance/start.
use super::{SignedControllerInput, TrustInputError};
use crate::diagnostic_artifacts::{ArtifactError, VerifiedBuild};
use crate::diagnostic_builder::CheckedBuilderPolicy;
use crate::diagnostic_reviewed_trust;

#[path = "diagnostic_oidc_input.rs"]
pub mod oidc_input;

#[derive(Debug, thiserror::Error)]
pub enum ControllerBuildError {
    #[error("original controller input refused: {0}")]
    Input(#[from] TrustInputError),
    #[error("reviewed builder statement refused: {0}")]
    Build(#[from] ArtifactError),
}

impl<'c, 'b, 's, 'a, 'p> SignedControllerInput<'c, 'b, 's, 'a, 'p> {
    pub fn bind_reviewed_build<'r>(
        &'r mut self,
        policy_bytes: &[u8],
        receipt_bytes: &[u8],
    ) -> Result<ControllerBuildInput<'r, 'c, 'b, 's, 'a, 'p>, ControllerBuildError> {
        let result = (|| {
            self.revalidate()?;
            // Both verifiers enforce their budgets before decoding or copying.
            let policy = diagnostic_reviewed_trust::load_builder_policy(policy_bytes)?;
            let build = policy.verify_receipt(receipt_bytes, &self.request.identity)?;
            self.revalidate()?;
            Ok::<_, ControllerBuildError>((policy, build))
        })();
        match result {
            Ok((policy, build)) => Ok(ControllerBuildInput {
                original: self,
                policy,
                build,
                policy_bytes: policy_bytes.to_vec(),
                receipt_bytes: receipt_bytes.to_vec(),
            }),
            Err(error) => {
                self.latch_refusal();
                Err(error)
            }
        }
    }
}

/// Retains a distinct builder signature and the original request, not executable contents.
/// No revalidation supplies a new identity, receipt, key or controller packet.
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::ControllerBuildInput;
/// fn start(mut g: ControllerBuildInput<'_, '_, '_, '_, '_, '_>) { g.start_worker(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::ControllerBuildInput;
/// fn extract(g: ControllerBuildInput<'_, '_, '_, '_, '_, '_>) { g.into_verified_build(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::ControllerBuildInput;
/// fn replace(mut g: ControllerBuildInput<'_, '_, '_, '_, '_, '_>) { g.revalidate(b"new"); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::ControllerBuildInput;
/// fn send(g: ControllerBuildInput<'static, 'static, 'static, 'static, 'static, 'static>) { std::thread::spawn(move || drop(g)); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::SignedControllerInput;
/// fn refresh(original: &mut SignedControllerInput<'_, '_, '_, '_, '_>, policy: &[u8], receipt: &[u8]) {
///     let mut g = original.bind_reviewed_build(policy, receipt).unwrap();
///     original.revalidate().unwrap();
///     g.revalidate().unwrap();
/// }
/// ```
pub struct ControllerBuildInput<'r, 'c, 'b, 's, 'a, 'p> {
    original: &'r mut SignedControllerInput<'c, 'b, 's, 'a, 'p>,
    policy: CheckedBuilderPolicy,
    build: VerifiedBuild,
    policy_bytes: Vec<u8>,
    receipt_bytes: Vec<u8>,
}
impl ControllerBuildInput<'_, '_, '_, '_, '_, '_> {
    pub fn is_refused(&self) -> bool {
        self.original.is_refused()
    }
    pub fn revalidate(&mut self) -> Result<(), ControllerBuildError> {
        let result = (|| {
            self.original.revalidate()?;
            let pinned = diagnostic_reviewed_trust::load_builder_policy(&self.policy_bytes)?;
            let identity = &self.original.request.identity;
            let checked = pinned.verify_receipt(&self.receipt_bytes, identity)?;
            let retained = self.policy.verify_receipt(&self.receipt_bytes, identity)?;
            if checked.identity() != self.build.identity()
                || checked.statement() != self.build.statement()
                || retained.identity() != checked.identity()
                || retained.statement() != checked.statement()
            {
                return Err(ArtifactError::Invalid.into());
            }
            self.original.revalidate()?;
            Ok(())
        })();
        if result.is_err() {
            self.original.latch_refusal();
        }
        result
    }
}

#[cfg(test)]
#[path = "diagnostic_controller_build_input_tests.rs"]
mod tests;
