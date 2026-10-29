//! Borrow original bundle/process objects, never bootstrap or start authority.
use super::AssertedSignerBundle;
use crate::diagnostic_artifacts::ArtifactError;
use crate::diagnostic_process::{ProcessError, ProcessExecutableRead, ProcessRead};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SignerExecutableError {
    #[error("original asserted signer bundle refused: {0}")]
    Bundle(#[from] ArtifactError),
    #[error("original process executable observation refused: {0}")]
    Process(#[from] ProcessError),
}

/// Point-in-time /proc/exe object match, NOT original invocation, memory,
/// interpreter/loader/runtime integrity, job custody or permission to sign.
/// Same-inode re-exec can match; process generation alone does not rule it out.
///
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_signer_bundle::AssertedSignerExecutableRead;
/// fn duplicate(read: AssertedSignerExecutableRead<'_>) { let _ = read.clone(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_signer_bundle::AssertedSignerExecutableRead;
/// fn decode<T: serde::de::DeserializeOwned>() {}
/// decode::<AssertedSignerExecutableRead<'static>>();
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_signer_bundle::AssertedSignerExecutableRead;
/// use std::os::fd::{AsRawFd, RawFd};
/// fn export(read: &AssertedSignerExecutableRead<'_>) -> RawFd { read.as_raw_fd() }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::{diagnostic_process::ProcessRead,
///     diagnostic_signer_bundle::AssertedSignerExecutableRead};
/// fn replace(read: &mut AssertedSignerExecutableRead<'_>, other: &ProcessRead) {
///     read.revalidate(other);
/// }
/// ```
pub struct AssertedSignerExecutableRead<'a> {
    bundle: &'a mut AssertedSignerBundle,
    executable: ProcessExecutableRead<'a>,
    refused: bool,
}

impl AssertedSignerBundle {
    /// Borrow the same file-input owner and original process observation. The
    /// caller's origin/selection remain asserted; this is not original start.
    pub fn bind_asserted_process_executable<'a>(
        &'a mut self,
        process: &'a ProcessRead,
    ) -> Result<AssertedSignerExecutableRead<'a>, SignerExecutableError> {
        self.check_original()?;
        let result = process.pin_executable(&self.files[0].file);
        let executable = match result {
            Ok(observed) => observed,
            Err(error) => {
                self.refused = true;
                return Err(error.into());
            }
        };
        self.check_original()?;
        let mut observed = AssertedSignerExecutableRead {
            bundle: self,
            executable,
            refused: false,
        };
        observed.revalidate()?;
        Ok(observed)
    }
}

impl AssertedSignerExecutableRead<'_> {
    pub fn is_refused(&self) -> bool {
        self.refused || self.bundle.is_refused()
    }

    /// Original owners only. Observed process or file-input loss permanently
    /// refuses this guard and its borrowed bundle, even after restoration/drop.
    pub fn revalidate(&mut self) -> Result<&str, SignerExecutableError> {
        if self.is_refused() {
            self.refuse();
            return Err(ArtifactError::Contents.into());
        }
        let result = self.inspect();
        if result.is_err() {
            self.refuse();
        }
        result?;
        Ok(self.bundle.checked.receipt_sha256())
    }
    fn refuse(&mut self) {
        self.refused = true;
        self.bundle.refused = true;
    }
    fn inspect(&mut self) -> Result<(), SignerExecutableError> {
        self.bundle.check_original()?;
        self.executable.revalidate()?;
        self.bundle.check_original()?;
        self.executable.revalidate()?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "diagnostic_signer_executable_tests.rs"]
mod tests;
