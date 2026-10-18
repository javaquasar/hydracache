//! Actual retained receiver context only, never principal or execution authority.
use super::*;

pub(crate) struct OriginalReceiverBinding {
    origin: std::sync::Arc<Origin>,
    refused: bool,
}
impl KernelPacket {
    pub(crate) fn retain_original_receiver_binding(
        &mut self,
    ) -> Result<OriginalReceiverBinding, KernelError> {
        let result = (|| {
            if self.refused {
                return Err(KernelError::Refused);
            }
            self.sender.revalidate(&self.origin)?;
            valid_hold(&self.received, &self.origin.sample()?)?;
            self.sender.revalidate(&self.origin)?;
            Ok(OriginalReceiverBinding {
                origin: std::sync::Arc::clone(&self.origin),
                refused: false,
            })
        })();
        if result.is_err() {
            self.latch_refusal();
        }
        result
    }
}
impl OriginalReceiverBinding {
    // Candidate handles are compared with retained originals, never adopted.
    pub(crate) fn require_context(
        &mut self,
        boot: &str,
        files: [&File; 4],
    ) -> Result<(), KernelError> {
        if self.refused {
            return Err(KernelError::Refused);
        }
        let result = (|| {
            self.origin.revalidate()?;
            if boot != self.origin.boot {
                return Err(KernelError::Invalid);
            }
            for ((file, (_, retained)), kind) in
                files.into_iter().zip(&self.origin.namespaces).zip([
                    libc::CLONE_NEWUSER,
                    libc::CLONE_NEWNS,
                    libc::CLONE_NEWPID,
                    libc::CLONE_NEWTIME,
                ])
            {
                if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFD) } != libc::FD_CLOEXEC {
                    return Err(KernelError::Invalid);
                }
                require_namespace(file, *retained, kind)?;
            }
            self.origin.revalidate()
        })();
        if result.is_err() {
            self.refused = true;
        }
        result
    }
}

#[cfg(test)]
#[path = "diagnostic_kernel_manager_binding_tests.rs"]
mod tests;
