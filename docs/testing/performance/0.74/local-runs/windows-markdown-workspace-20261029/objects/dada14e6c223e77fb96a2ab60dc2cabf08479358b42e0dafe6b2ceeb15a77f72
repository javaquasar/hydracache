//! Caller mount pin completes the existing channel's PID/user/time view.
use super::*;
use crate::diagnostic_kernel_admission::{KernelError, OriginalReceiverBinding};

pub(in super::super) struct ReceiverPin {
    mount: File,
}
impl ReceiverPin {
    pub(in super::super) fn capture() -> Result<Self> {
        let proc = open_at(
            libc::AT_FDCWD,
            b"/proc",
            libc::O_PATH | libc::O_DIRECTORY | libc::O_NOFOLLOW,
        )?;
        filesystem(&proc, libc::PROC_SUPER_MAGIC as _)?;
        Ok(Self {
            mount: namespace(&proc, "thread-self", "mnt", libc::CLONE_NEWNS)?,
        })
    }
    pub(in super::super) fn require(
        &self,
        channel: &super::super::FixedManagerChannel,
        original: &mut OriginalReceiverBinding,
    ) -> std::result::Result<(), KernelError> {
        let invalid = |_| KernelError::Invalid;
        channel.inspect().map_err(invalid)?;
        let current = Self::capture().map_err(invalid)?;
        if stamp(&current.mount).map_err(invalid)? != stamp(&self.mount).map_err(invalid)? {
            return Err(KernelError::Invalid);
        }
        let ns = &channel.kernel.namespaces;
        if ns.len() != 7 {
            return Err(KernelError::Invalid);
        }
        original.require_context(&channel.boot, [&ns[5], &self.mount, &ns[2], &ns[6]])?;
        channel.inspect().map_err(invalid)
    }
}
