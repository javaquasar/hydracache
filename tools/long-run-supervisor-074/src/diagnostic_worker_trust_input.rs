//! Original root configuration/policy/journal consistency, not host or start authority.
use super::authority_registry::{fixed::FixedAuthorityRegistry, AuthorityError, AuthorityView};
use super::{AssertedWorkerHost, CheckedWorkerPolicy, WorkerPolicyError, WorkerPolicyTrust};
use crate::{canonical_json, is_hash};
use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Serialize};
use std::ffi::CString;
use std::fs::{File, Metadata, OpenOptions};
use std::marker::PhantomData;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{FileExt, MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use thiserror::Error;

#[path = "diagnostic_worker_kernel_binding.rs"]
mod kernel_binding;
pub use kernel_binding::KernelWorkerPolicyInput;

pub const TRUST_INPUT_SCHEMA: &str = "diagnostic-worker-trust-input-074-v1";
pub const FIXED_TRUST_PATH: &str = "/etc/hydracache-perf/worker-trust-074.json";
pub const MAX_TRUST_BYTES: usize = 2048;
const TRUST_FILE: &str = "worker-trust-074.json";
const ANCESTORS: [&str; 2] = ["etc", "hydracache-perf"];

#[derive(Debug, Error)]
pub enum TrustInputError {
    #[error("worker trust source/document/metadata differs or is unsafe")]
    Invalid,
    #[error("original worker trust input or authority previously refused")]
    Refused,
    #[error("worker trust source I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("worker policy verification failed: {0}")]
    Policy(#[from] WorkerPolicyError),
    #[error("original authority inspection failed: {0}")]
    Authority(#[from] AuthorityError),
    #[error("original kernel packet context failed: {0}")]
    Kernel(#[from] crate::diagnostic_kernel_admission::KernelError),
    #[error("original fixed worker context failed: {0}")]
    Context(#[from] super::local_context::WorkerContextError),
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema_version: String,
    repository_id: u64,
    issuer_key_hex: String,
    worker_policy_body_sha256: String,
    policy_epoch: u64,
}
fn decode(bytes: &[u8], selected_key: &VerifyingKey) -> Result<WorkerPolicyTrust, TrustInputError> {
    if bytes.is_empty() || bytes.len() > MAX_TRUST_BYTES {
        return Err(TrustInputError::Invalid);
    }
    let document: Document = serde_json::from_slice(bytes).map_err(|_| TrustInputError::Invalid)?;
    let mut canonical = canonical_json(&document).map_err(|_| TrustInputError::Invalid)?;
    canonical.push(b'\n');
    if canonical != bytes
        || document.schema_version != TRUST_INPUT_SCHEMA
        || document.repository_id != crate::diagnostic_builder::REPOSITORY_ID
        || !is_hash(&document.worker_policy_body_sha256)
        || document.policy_epoch == 0
        || selected_key.is_weak()
        || crate::diagnostic_builder::decode_key(&document.issuer_key_hex)
            .map_err(|_| TrustInputError::Invalid)?
            != *selected_key
    {
        return Err(TrustInputError::Invalid);
    }
    Ok(WorkerPolicyTrust::new(
        *selected_key,
        &document.worker_policy_body_sha256,
        document.policy_epoch,
    )?)
}

#[derive(PartialEq, Eq)]
struct DirectoryStamp {
    dev: u64,
    ino: u64,
    uid: u32,
    gid: u32,
    mode: u32,
}
impl DirectoryStamp {
    fn of(m: &Metadata) -> Self {
        Self {
            dev: m.dev(),
            ino: m.ino(),
            uid: m.uid(),
            gid: m.gid(),
            mode: m.mode(),
        }
    }
}
#[derive(PartialEq, Eq)]
struct FileStamp {
    directory: DirectoryStamp,
    len: u64,
    links: u64,
    mtime: i64,
    mtime_ns: i64,
    ctime: i64,
    ctime_ns: i64,
}
impl FileStamp {
    fn of(m: &Metadata) -> Self {
        Self {
            directory: DirectoryStamp::of(m),
            len: m.len(),
            links: m.nlink(),
            mtime: m.mtime(),
            mtime_ns: m.mtime_nsec(),
            ctime: m.ctime(),
            ctime_ns: m.ctime_nsec(),
        }
    }
}
struct Directory {
    file: File,
    stamp: DirectoryStamp,
}
impl Directory {
    fn new(file: File, uid: u32) -> Result<Self, TrustInputError> {
        let m = file.metadata()?;
        if !m.is_dir() || m.uid() != uid || m.mode() & 0o7022 != 0 || m.mode() & 0o100 == 0 {
            return Err(TrustInputError::Invalid);
        }
        Ok(Self {
            file,
            stamp: DirectoryStamp::of(&m),
        })
    }
    fn same(&self, named: &File) -> Result<(), TrustInputError> {
        if DirectoryStamp::of(&self.file.metadata()?) != self.stamp
            || DirectoryStamp::of(&named.metadata()?) != self.stamp
        {
            return Err(TrustInputError::Invalid);
        }
        Ok(())
    }
}
fn child(parent: &File, name: &str, directory: bool) -> Result<File, TrustInputError> {
    let name = CString::new(name).map_err(|_| TrustInputError::Invalid)?;
    let flags = libc::O_RDONLY
        | libc::O_NOFOLLOW
        | libc::O_CLOEXEC
        | libc::O_NONBLOCK
        | if directory { libc::O_DIRECTORY } else { 0 };
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}
fn bounded_read(
    mut read: impl FnMut(&mut [u8], u64) -> std::io::Result<usize>,
) -> Result<Vec<u8>, TrustInputError> {
    let mut bytes = vec![0; MAX_TRUST_BYTES + 1];
    let mut offset = 0;
    for _ in 0..MAX_TRUST_BYTES + 2 {
        let count = match read(&mut bytes[offset..], offset as u64) {
            Ok(count) => count,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error.into()),
        };
        if count > bytes.len() - offset {
            return Err(TrustInputError::Invalid);
        }
        if count == 0 {
            if offset == 0 {
                return Err(TrustInputError::Invalid);
            }
            bytes.truncate(offset);
            return Ok(bytes);
        }
        offset += count;
        if offset > MAX_TRUST_BYTES {
            return Err(TrustInputError::Invalid);
        }
    }
    Err(TrustInputError::Invalid)
}
struct Snapshot {
    file: File,
    stamp: FileStamp,
    raw: Vec<u8>,
}
impl Snapshot {
    fn new(file: File, uid: u32) -> Result<Self, TrustInputError> {
        let m = file.metadata()?;
        if !m.is_file()
            || m.uid() != uid
            || m.nlink() != 1
            || m.mode() & 0o7133 != 0
            || m.mode() & 0o400 == 0
            || m.len() == 0
            || m.len() > MAX_TRUST_BYTES as u64
        {
            return Err(TrustInputError::Invalid);
        }
        let stamp = FileStamp::of(&m);
        let raw = bounded_read(|buffer, offset| file.read_at(buffer, offset))?;
        if raw.len() as u64 != stamp.len || FileStamp::of(&file.metadata()?) != stamp {
            return Err(TrustInputError::Invalid);
        }
        Ok(Self { file, stamp, raw })
    }
    fn same(&self, named: &Snapshot) -> Result<(), TrustInputError> {
        if FileStamp::of(&self.file.metadata()?) != self.stamp
            || self.stamp != named.stamp
            || self.raw != named.raw
            || bounded_read(|b, o| self.file.read_at(b, o))? != self.raw
            || FileStamp::of(&self.file.metadata()?) != self.stamp
        {
            return Err(TrustInputError::Invalid);
        }
        Ok(())
    }
}
struct Source {
    root_name: PathBuf,
    root: Directory,
    ancestors: Vec<Directory>,
    snapshot: Snapshot,
    trust: WorkerPolicyTrust,
    uid: u32,
}
impl Source {
    // Private walker: modeled roots/issuers are available only to module tests.
    fn open(
        root_name: &Path,
        uid: u32,
        selected_key: &VerifyingKey,
    ) -> Result<Self, TrustInputError> {
        let root = Directory::new(
            OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
                .open(root_name)?,
            uid,
        )?;
        let mut ancestors: Vec<Directory> = Vec::with_capacity(2);
        for name in ANCESTORS {
            let parent = ancestors.last().map_or(&root.file, |d| &d.file);
            ancestors.push(Directory::new(child(parent, name, true)?, uid)?);
        }
        let parent = &ancestors.last().ok_or(TrustInputError::Invalid)?.file;
        let snapshot = Snapshot::new(child(parent, TRUST_FILE, false)?, uid)?;
        let trust = decode(&snapshot.raw, selected_key)?;
        let source = Self {
            root_name: root_name.into(),
            root,
            ancestors,
            snapshot,
            trust,
            uid,
        };
        source.revalidate()?;
        Ok(source)
    }
    fn revalidate(&self) -> Result<(), TrustInputError> {
        let root = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&self.root_name)?;
        self.root.same(&root)?;
        let mut parent = &self.root.file;
        for (name, directory) in ANCESTORS.iter().zip(&self.ancestors) {
            directory.same(&child(parent, name, true)?)?;
            parent = &directory.file;
        }
        self.snapshot
            .same(&Snapshot::new(child(parent, TRUST_FILE, false)?, self.uid)?)
    }
}

/// Non-cloneable original-thread config owner. No public model/path/FD constructor.
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::WorkerTrustInput;
/// fn share(value: WorkerTrustInput) { std::thread::spawn(move || drop(value)); }
/// ```
pub struct WorkerTrustInput {
    source: Source,
    refused: bool,
    _original_thread: PhantomData<Rc<()>>,
}
pub fn open_fixed_worker_trust() -> Result<WorkerTrustInput, TrustInputError> {
    if unsafe { libc::geteuid() } != 0 || unsafe { libc::getegid() } != 0 {
        return Err(TrustInputError::Invalid);
    }
    let key = crate::diagnostic_reviewed_trust::worker_issuer_key()
        .map_err(|_| TrustInputError::Invalid)?;
    Ok(WorkerTrustInput {
        source: Source::open(Path::new("/"), 0, &key)?,
        refused: false,
        _original_thread: PhantomData,
    })
}
fn active(
    authority: &mut FixedAuthorityRegistry,
    trust: &WorkerPolicyTrust,
    host: &AssertedWorkerHost,
) -> Result<AuthorityView, TrustInputError> {
    let view = authority.inspect(trust, host)?;
    if view.refused {
        return Err(TrustInputError::Refused);
    }
    Ok(view)
}
impl WorkerTrustInput {
    pub fn is_refused(&self) -> bool {
        self.refused
    }
    fn check(&self) -> Result<(), TrustInputError> {
        if self.refused {
            return Err(TrustInputError::Refused);
        }
        self.source.revalidate()
    }
    pub fn verify_policy<'s, 'a>(
        &'s mut self,
        bytes: &[u8],
        host: &AssertedWorkerHost,
        authority: &'a mut FixedAuthorityRegistry,
    ) -> Result<WorkerPolicyInput<'s, 'a>, TrustInputError> {
        let result = (|| {
            self.check()?;
            let before = active(authority, &self.source.trust, host)?;
            let policy = super::verify_worker_policy(bytes, &self.source.trust, host)?;
            if active(authority, &self.source.trust, host)? != before {
                return Err(TrustInputError::Invalid);
            }
            self.check()?;
            Ok(policy)
        })();
        match result {
            Ok(policy) => Ok(WorkerPolicyInput {
                input: self,
                authority,
                policy,
                envelope: bytes.to_vec(),
                host: host.clone(),
            }),
            Err(error) => {
                self.refused = true;
                authority.latch_refusal();
                Err(error)
            }
        }
    }
}
/// Owns original policy, envelope and asserted context; borrows both original owners.
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::WorkerPolicyInput;
/// fn extract(input: WorkerPolicyInput<'_, '_>) { input.into_worker_policy(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::WorkerPolicyInput;
/// fn start(input: WorkerPolicyInput<'_, '_>) { input.start_worker(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::{AssertedWorkerHost, trusted_input::WorkerTrustInput};
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::authority_registry::fixed::FixedAuthorityRegistry;
/// fn refresh(source: &mut WorkerTrustInput, journal: &mut FixedAuthorityRegistry, bytes: &[u8], host: &AssertedWorkerHost) {
///     let mut original = source.verify_policy(bytes, host, journal).unwrap();
///     let replacement = source.verify_policy(bytes, host, journal);
///     original.revalidate().unwrap();
/// }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::WorkerPolicyInput;
/// fn replace(mut original: WorkerPolicyInput<'_, '_>) { original.revalidate(b"replacement"); }
/// ```
pub struct WorkerPolicyInput<'s, 'a> {
    input: &'s mut WorkerTrustInput,
    authority: &'a mut FixedAuthorityRegistry,
    policy: CheckedWorkerPolicy,
    envelope: Vec<u8>,
    host: AssertedWorkerHost,
}
impl WorkerPolicyInput<'_, '_> {
    fn latch_refusal(&mut self) {
        self.policy.refused = true;
        self.input.refused = true;
        self.authority.latch_refusal();
    }
    pub fn policy_sha256(&self) -> &str {
        self.policy.policy_sha256()
    }
    pub fn is_refused(&self) -> bool {
        self.policy.is_refused() || self.input.refused || self.authority.is_failed()
    }
    pub fn revalidate(&mut self) -> Result<(), TrustInputError> {
        let result = (|| {
            if self.is_refused() {
                return Err(TrustInputError::Refused);
            }
            self.input.check()?;
            let before = active(self.authority, &self.input.source.trust, &self.host)?;
            self.policy
                .revalidate(&self.envelope, &self.input.source.trust, &self.host)?;
            if active(self.authority, &self.input.source.trust, &self.host)? != before {
                return Err(TrustInputError::Invalid);
            }
            self.input.check()
        })();
        if result.is_err() {
            self.latch_refusal();
        }
        result
    }
}

#[cfg(test)]
#[path = "diagnostic_worker_trust_input_tests.rs"]
mod tests;
