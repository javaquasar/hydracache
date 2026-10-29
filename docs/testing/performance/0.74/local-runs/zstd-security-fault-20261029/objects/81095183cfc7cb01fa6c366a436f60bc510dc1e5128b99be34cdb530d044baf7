//! Keep the original checked worker, active authority and context around progress.
use super::*;

impl KernelWorkerPolicyInput<'_, '_, '_> {
    pub(in crate::diagnostic_worker_policy::trusted_input) fn authenticate_original_progress(
        &mut self,
        next: &mut KernelPacket,
        controller: &crate::diagnostic_ipc::DiagnosticPolicy,
        after: &crate::diagnostic_kernel_admission::KernelTimePoint,
    ) -> Result<crate::diagnostic_ipc::DiagnosticRequest, TrustInputError> {
        let result = (|| {
            self.revalidate()?;
            let request = self
                .packet
                .authenticate_original_progress(next, controller, after)?;
            self.revalidate()?;
            Ok(request)
        })();
        if result.is_err() {
            self.latch_refusal();
            next.latch_refusal();
        }
        result
    }
}
