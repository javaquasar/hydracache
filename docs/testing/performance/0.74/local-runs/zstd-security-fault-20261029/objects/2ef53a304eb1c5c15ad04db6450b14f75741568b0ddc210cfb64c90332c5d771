//! Complete current buffer core; only the File adapter is explicitly omitted.
use crate::{KernelError, DOC_BYTES};

// BEGIN PURE DOCUMENT CORE
pub(super) struct DocumentBuffer {
    bytes: Vec<u8>,
}
impl DocumentBuffer {
    pub(super) fn new() -> Self {
        Self {
            bytes: vec![0; DOC_BYTES + 1],
        }
    }
    fn read_with(
        &mut self,
        read: impl FnOnce(&mut [u8]) -> std::io::Result<usize>,
    ) -> Result<&[u8], KernelError> {
        #[cfg(test)]
        let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(
            crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Document,
        );
        let count = read(&mut self.bytes)?;
        if count == 0 || count > DOC_BYTES {
            return Err(kernel_invalid!("proc-document-budget"));
        }
        Ok(&self.bytes[..count])
    }
}
// END PURE DOCUMENT CORE

#[path = "controls.rs"]
mod controls;
