//! Retain the active worker authority around original kernel consent authentication.
use super::*;

impl KernelWorkerPolicyInput<'_, '_, '_> {
    pub(in crate::diagnostic_worker_policy::trusted_input) fn authenticate_original_start_consent(
        &mut self,
        next: &mut KernelPacket,
        controller: &crate::diagnostic_ipc::DiagnosticPolicy,
        after: &crate::diagnostic_kernel_admission::KernelTimePoint,
    ) -> Result<crate::diagnostic_start_consent::StartConsentBody, TrustInputError> {
        let result = (|| {
            self.revalidate()?;
            let consent = self
                .packet
                .authenticate_original_start_consent(next, controller, after)?;
            self.revalidate()?;
            Ok(consent)
        })();
        if result.is_err() {
            self.latch_refusal();
            next.latch_refusal();
        }
        result
    }
}
