//! Original numeric-root bytes joined to GNU declarations and an ASSERTED receipt.
//! No independent host/job/runtime enrollment, writer revocation or launch authority.
use crate::diagnostic_artifacts::{ArtifactDigest, ArtifactError};
use crate::diagnostic_root_owned_input::{inspect_root_owned_input, RootOwnedInputRead};
use crate::diagnostic_signer_build::{AssertedSignerBuildSelection, SignerBuildContents};
use crate::diagnostic_signer_gnu::inspect_gnu_declarations;
use std::path::Path;

const MAX_IMAGE: u64 = 8 * 1024 * 1024;

/// Other build assertions. Binary and runtime manifest cannot be supplied here.
pub struct SignerRuntimeCompanions<'a> {
    pub root_lock: &'a [u8],
    pub build_log: &'a [u8],
    pub bootstrap_context: &'a [u8],
}

/// Retain exactly signer/loader/libc/libgcc in their original reading context.
/// Expected issuer, statement, paths and digests remain explicit local assertions.
/// Numeric-root/current-root acquisition cannot select a trusted initial host.
pub fn retain_asserted_gnu_runtime(
    inputs: [(&Path, ArtifactDigest); 4],
    selection: AssertedSignerBuildSelection,
    receipt: &[u8],
    companions: &SignerRuntimeCompanions<'_>,
) -> Result<AssertedGnuRuntimeRead, ArtifactError> {
    // All role budgets and the exact expected signed statement precede file IO.
    for (_, digest) in &inputs {
        if !(64..=MAX_IMAGE).contains(&digest.bytes) || !crate::is_hash(&digest.sha256) {
            return Err(ArtifactError::Contents);
        }
    }
    selection.check_receipt(receipt)?;
    let mut originals = Vec::with_capacity(4);
    for (path, digest) in inputs {
        match inspect_root_owned_input(path, digest) {
            Ok(original) => originals.push(original),
            Err(error) => {
                refuse_all(&mut originals);
                return Err(error.into());
            }
        }
    }
    join(originals, selection, receipt, companions)
}

fn refuse_all(originals: &mut [RootOwnedInputRead]) {
    for original in originals {
        original.refuse();
    }
}

/// Stronger optional declared-context check, not launcher/job or runtime admission.
pub fn retain_asserted_bootstrap_runtime(
    inputs: [(&Path, ArtifactDigest); 4],
    selection: AssertedSignerBuildSelection,
    receipt: &[u8],
    companions: &SignerRuntimeCompanions<'_>,
) -> Result<AssertedBootstrapRuntimeRead, ArtifactError> {
    selection.check_receipt(receipt)?;
    selection.check_bootstrap_context(companions.bootstrap_context)?;
    let original = retain_asserted_gnu_runtime(inputs, selection, receipt, companions)?;
    Ok(AssertedBootstrapRuntimeRead { original })
}

/// Owns the stronger context+runtime consistency result, without guard extraction.
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_signer_runtime_input::{AssertedBootstrapRuntimeRead, AssertedGnuRuntimeRead};
/// fn weaken(r: AssertedBootstrapRuntimeRead) -> AssertedGnuRuntimeRead { r.into() }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_signer_runtime_input::AssertedBootstrapRuntimeRead;
/// fn start(r: AssertedBootstrapRuntimeRead) { r.start_worker(); }
/// ```
pub struct AssertedBootstrapRuntimeRead {
    original: AssertedGnuRuntimeRead,
}
impl AssertedBootstrapRuntimeRead {
    pub fn is_refused(&self) -> bool {
        self.original.is_refused()
    }
    pub fn check_original(&mut self) -> Result<(&str, &str), ArtifactError> {
        self.original.check_original()
    }
}

fn join(
    originals: Vec<RootOwnedInputRead>,
    selection: AssertedSignerBuildSelection,
    receipt: &[u8],
    companions: &SignerRuntimeCompanions<'_>,
) -> Result<AssertedGnuRuntimeRead, ArtifactError> {
    let mut read = AssertedGnuRuntimeRead {
        originals,
        receipt_sha256: String::new(),
        catalog_sha256: String::new(),
        refused: false,
    };
    let result = (|| {
        if read.originals.len() != 4 {
            return Err(ArtifactError::Contents);
        }
        let images = read
            .originals
            .iter_mut()
            .map(|r| r.read_original_bounded(MAX_IMAGE))
            .collect::<std::io::Result<Vec<_>>>()?;
        let catalog = inspect_gnu_declarations([&images[0], &images[1], &images[2], &images[3]])?;
        let mut manifest = crate::canonical_json(&catalog)?;
        manifest.push(b'\n');
        let model = selection.verify_supplied(
            receipt,
            &SignerBuildContents {
                binary: &images[0],
                root_lock: companions.root_lock,
                build_log: companions.build_log,
                runtime_manifest: &manifest,
                bootstrap_context: companions.bootstrap_context,
            },
        )?;
        read.receipt_sha256 = model.receipt_sha256().to_owned();
        read.catalog_sha256 = crate::sha256_hex(&manifest);
        read.check_original()?;
        Ok(())
    })();
    if let Err(error) = result {
        read.refuse();
        return Err(error);
    }
    Ok(read)
}

/// Original input consistency only. Not a receipt issuer, bootstrap or executable.
///
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_signer_runtime_input::AssertedGnuRuntimeRead;
/// fn duplicate(r: AssertedGnuRuntimeRead) { let _ = r.clone(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_signer_runtime_input::AssertedGnuRuntimeRead;
/// fn decode<T: serde::de::DeserializeOwned>() {}
/// decode::<AssertedGnuRuntimeRead>();
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_signer_runtime_input::AssertedGnuRuntimeRead;
/// use std::os::fd::AsRawFd;
/// fn export(r: AssertedGnuRuntimeRead) { let _ = r.as_raw_fd(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_signer_runtime_input::AssertedGnuRuntimeRead;
/// fn move_thread(r: AssertedGnuRuntimeRead) { std::thread::spawn(move || drop(r)); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::{diagnostic_signer_runtime_input::AssertedGnuRuntimeRead,
///     diagnostic_signer_bundle::AssertedSignerBundle};
/// fn admit(r: AssertedGnuRuntimeRead) -> AssertedSignerBundle { r.into() }
/// ```
pub struct AssertedGnuRuntimeRead {
    originals: Vec<RootOwnedInputRead>,
    receipt_sha256: String,
    catalog_sha256: String,
    refused: bool,
}
impl AssertedGnuRuntimeRead {
    fn refuse(&mut self) {
        self.refused = true;
        refuse_all(&mut self.originals);
    }
    pub fn is_refused(&self) -> bool {
        self.refused
    }
    /// Revalidate every original before returning diagnostic hashes only.
    /// Sequential observation, not continuous protection or retained-writer revocation.
    pub fn check_original(&mut self) -> Result<(&str, &str), ArtifactError> {
        if self.refused {
            return Err(ArtifactError::Contents);
        }
        for original in &mut self.originals {
            if let Err(error) = original.check_original() {
                self.refuse();
                return Err(error.into());
            }
        }
        Ok((&self.receipt_sha256, &self.catalog_sha256))
    }
}

#[cfg(test)]
#[path = "diagnostic_signer_runtime_input_tests.rs"]
mod tests;
