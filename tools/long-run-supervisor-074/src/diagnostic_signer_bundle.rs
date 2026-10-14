//! Original Linux file-input retention under explicitly asserted local selection.
//! This is not trusted ancestry, mapped executable/runtime or bootstrap authority.
use crate::diagnostic_artifacts::{ArtifactDigest, ArtifactError, INSTALL_ROOT};
use crate::diagnostic_signer_build::{
    self as model, AssertedSignerBuildSelection, CheckedSignerBuildModel,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::ffi::{CString, OsString};
use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

const FILES: [&str; 6] = [
    "diagnostic_controller",
    "signer-build-receipt-v1.json",
    "Cargo.lock.root",
    "build-log.jsonl",
    "runtime-manifest.json",
    "bootstrap-context.json",
];
const MAX_PATH_BYTES: usize = 4096;

#[derive(Clone, PartialEq, Eq)]
struct Stamp {
    dev: u64,
    ino: u64,
    uid: u32,
    gid: u32,
    mode: u32,
    links: u64,
    len: u64,
    mtime: i64,
    mtime_ns: i64,
    ctime: i64,
    ctime_ns: i64,
}
impl From<Metadata> for Stamp {
    fn from(m: Metadata) -> Self {
        Self {
            dev: m.dev(),
            ino: m.ino(),
            uid: m.uid(),
            gid: m.gid(),
            mode: m.mode(),
            links: m.nlink(),
            len: m.len(),
            mtime: m.mtime(),
            mtime_ns: m.mtime_nsec(),
            ctime: m.ctime(),
            ctime_ns: m.ctime_nsec(),
        }
    }
}

struct PinnedInput {
    name: &'static str,
    file: File,
    stamp: Stamp,
    digest: ArtifactDigest,
}
impl PinnedInput {
    fn check(&mut self, root: &File) -> Result<(), ArtifactError> {
        let current = open_child(root, self.name.as_bytes(), false)?;
        if Stamp::from(current.metadata()?) != self.stamp
            || Stamp::from(self.file.metadata()?) != self.stamp
        {
            return Err(ArtifactError::Contents);
        }
        let (digest, _) = hash_original(&mut self.file, limits(self.name).0, false)?;
        if digest != self.digest
            || Stamp::from(self.file.metadata()?) != self.stamp
            || Stamp::from(current.metadata()?) != self.stamp
        {
            return Err(ArtifactError::Contents);
        }
        Ok(())
    }
}

/// Holds original public fixture inputs, never production/signing authority.
/// No replacement inputs are accepted by recheck; first refusal is sticky.
///
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_signer_bundle::AssertedSignerBundle;
/// fn duplicate(bundle: AssertedSignerBundle) { let _ = bundle.clone(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_signer_bundle::AssertedSignerBundle;
/// fn decode<T: serde::de::DeserializeOwned>() {}
/// decode::<AssertedSignerBundle>();
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_signer_bundle::AssertedSignerBundle;
/// use std::os::fd::{AsRawFd, RawFd};
/// fn export(bundle: &AssertedSignerBundle) -> RawFd { bundle.as_raw_fd() }
/// ```
pub struct AssertedSignerBundle {
    path: PathBuf,
    uid: u32,
    gid: u32,
    directory: File,
    stamp: Stamp,
    files: Vec<PinnedInput>,
    checked: CheckedSignerBuildModel,
    refused: bool,
}

/// Read-only asserted fixture seam. Does not select issuer/pins or authenticate
/// ancestors, actual executable mapping, runtime, private key custody or a job.
pub fn retain_asserted_bundle(
    path: &Path,
    uid: u32,
    gid: u32,
    selection: AssertedSignerBuildSelection,
) -> Result<AssertedSignerBundle, ArtifactError> {
    if path == Path::new(INSTALL_ROOT) {
        return Err(ArtifactError::Contents);
    }
    let directory = open_root(path, uid, gid)?;
    let stamp = Stamp::from(directory.metadata()?);
    check_entries(&directory)?;
    let mut files = Vec::with_capacity(FILES.len());
    let mut receipt = Vec::new();
    for name in FILES {
        let mut file = open_child(&directory, name.as_bytes(), false)?;
        let metadata = file.metadata()?;
        let (maximum, mode) = limits(name);
        if !metadata.is_file()
            || metadata.uid() != uid
            || metadata.gid() != gid
            || metadata.mode() & 0o7777 != mode
            || metadata.nlink() != 1
            || metadata.len() == 0
            || metadata.len() > maximum
        {
            return Err(ArtifactError::Contents);
        }
        let original = Stamp::from(metadata);
        let (digest, retained) = hash_original(&mut file, maximum, name == FILES[1])?;
        if Stamp::from(file.metadata()?) != original {
            return Err(ArtifactError::Contents);
        }
        if name == FILES[1] {
            receipt = retained;
        }
        files.push(PinnedInput {
            name,
            file,
            stamp: original,
            digest,
        });
    }
    let checked = selection.verify_original_digests(
        &receipt,
        &[
            files[0].digest.clone(),
            files[2].digest.clone(),
            files[3].digest.clone(),
            files[4].digest.clone(),
            files[5].digest.clone(),
        ],
    )?;
    let mut bundle = AssertedSignerBundle {
        path: path.to_owned(),
        uid,
        gid,
        directory,
        stamp,
        files,
        checked,
        refused: false,
    };
    bundle.check_original()?;
    Ok(bundle)
}

impl AssertedSignerBundle {
    pub fn is_refused(&self) -> bool {
        self.refused
    }

    /// Inventory result after checking the original descriptors/names/bytes.
    /// Not an atomic promise about future path execution; never grants secrets.
    pub fn check_original(&mut self) -> Result<&str, ArtifactError> {
        if self.refused {
            return Err(ArtifactError::Contents);
        }
        let result = self.check();
        if result.is_err() {
            self.refused = true;
        }
        result?;
        Ok(self.checked.receipt_sha256())
    }

    fn check(&mut self) -> Result<(), ArtifactError> {
        let named = open_root(&self.path, self.uid, self.gid)?;
        self.check_root(&named)?;
        check_entries(&self.directory)?;
        for input in &mut self.files {
            input.check(&self.directory)?;
        }
        check_entries(&self.directory)?;
        self.check_root(&named)?;
        // Walk the name again to bracket root path replacement during hashing.
        self.check_root(&open_root(&self.path, self.uid, self.gid)?)
    }
    fn check_root(&self, named: &File) -> Result<(), ArtifactError> {
        if Stamp::from(named.metadata()?) != self.stamp
            || Stamp::from(self.directory.metadata()?) != self.stamp
        {
            return Err(ArtifactError::Contents);
        }
        Ok(())
    }
}

fn limits(name: &str) -> (u64, u32) {
    match name {
        "diagnostic_controller" => (model::MAX_BINARY_BYTES, 0o555),
        "signer-build-receipt-v1.json" => (model::MAX_RECEIPT_BYTES as u64, 0o444),
        "Cargo.lock.root" => (model::MAX_LOCK_BYTES, 0o444),
        "build-log.jsonl" => (model::MAX_LOG_BYTES, 0o444),
        _ => (model::MAX_CONTEXT_BYTES, 0o444),
    }
}

fn open_root(path: &Path, uid: u32, gid: u32) -> Result<File, ArtifactError> {
    let raw = path.as_os_str().as_bytes();
    if raw.len() > MAX_PATH_BYTES {
        return Err(ArtifactError::Contents);
    }
    let normal = raw.strip_prefix(b"/").ok_or(ArtifactError::Contents)?;
    if normal
        .split(|b| *b == b'/')
        .any(|name| name.is_empty() || name == b"." || name == b"..")
    {
        return Err(ArtifactError::Contents);
    }
    let mut root = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open("/")?;
    for component in normal.split(|b| *b == b'/') {
        root = open_child(&root, component, true)?;
    }
    let m = root.metadata()?;
    if !m.is_dir() || m.uid() != uid || m.gid() != gid || m.mode() & 0o7777 != 0o555 {
        return Err(ArtifactError::Contents);
    }
    Ok(root)
}

fn open_child(parent: &File, name: &[u8], directory: bool) -> Result<File, ArtifactError> {
    let name = CString::new(name).map_err(|_| ArtifactError::Contents)?;
    let flags = libc::O_RDONLY
        | libc::O_NOFOLLOW
        | libc::O_CLOEXEC
        | libc::O_NONBLOCK
        | if directory { libc::O_DIRECTORY } else { 0 };
    // SAFETY: still-owned directory fd, NUL-terminated component, no creation.
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    // SAFETY: successful openat transferred one newly owned descriptor.
    Ok(unsafe { File::from_raw_fd(fd) })
}

fn check_entries(root: &File) -> Result<(), ArtifactError> {
    let mut names = BTreeSet::new();
    // Own proc fd resolves only enumeration. Child reads use retained openat.
    for entry in fs::read_dir(format!("/proc/self/fd/{}", root.as_raw_fd()))? {
        let name = entry?.file_name();
        if names.len() == FILES.len() || !names.insert(name) {
            return Err(ArtifactError::Contents);
        }
    }
    if names != FILES.into_iter().map(OsString::from).collect() {
        return Err(ArtifactError::Contents);
    }
    Ok(())
}

fn hash_original(
    file: &mut File,
    maximum: u64,
    retain: bool,
) -> Result<(ArtifactDigest, Vec<u8>), ArtifactError> {
    file.seek(SeekFrom::Start(0))?;
    let mut hash = Sha256::new();
    let mut length = 0u64;
    let mut bytes = Vec::new();
    let mut buffer = [0; 65536];
    loop {
        let budget = ((maximum + 1 - length).min(buffer.len() as u64)) as usize;
        let count = file.read(&mut buffer[..budget])?;
        if count == 0 {
            break;
        }
        length += count as u64;
        if length > maximum {
            return Err(ArtifactError::Contents);
        }
        hash.update(&buffer[..count]);
        if retain {
            bytes.extend_from_slice(&buffer[..count]);
        }
    }
    if length == 0 {
        return Err(ArtifactError::Contents);
    }
    Ok((
        ArtifactDigest {
            sha256: crate::hex(&hash.finalize()),
            bytes: length,
        },
        bytes,
    ))
}

#[cfg(test)]
#[path = "diagnostic_signer_bundle_tests.rs"]
mod tests;
