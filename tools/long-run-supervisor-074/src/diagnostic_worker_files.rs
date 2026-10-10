//! Fixed account files: sequential read-only consistency, never host/start authority.
use super::{
    AssertedWorkerHost, CheckedWorkerPolicy, WorkerPolicy, WorkerPolicyError, WorkerPolicyTrust,
};
use crate::sha256_hex;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::CString;
use std::fs::{File, Metadata, OpenOptions};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{FileExt, MetadataExt, OpenOptionsExt};
use std::path::{Component, Path, PathBuf};
use thiserror::Error;

pub const MAX_ACCOUNT_FILE_BYTES: usize = 1_048_576;
pub const MAX_ACCOUNT_LINE_BYTES: usize = 4096;
pub const MAX_ACCOUNT_RECORDS: usize = 16_384;
const FIXED_DIRECTORY: &str = "/etc";
const NAMES: [&str; 2] = ["passwd", "group"];

#[derive(Debug, Error)]
pub enum WorkerFilesError {
    #[error("original local worker files or policy previously refused")]
    Refused,
    #[error(transparent)]
    Policy(#[from] WorkerPolicyError),
    #[error("local worker file observation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("local worker file ancestry, ownership, mode, kind or size is unsafe")]
    Security,
    #[error("local account documents violate the bounded unambiguous grammar")]
    Document,
    #[error("local account mapping differs from the exact signed worker policy")]
    Mapping,
    #[error("original local account file object, metadata or digest changed")]
    Drift,
}

#[derive(PartialEq, Eq)]
struct DirectoryStamp {
    device: u64,
    inode: u64,
    uid: u32,
    gid: u32,
    mode: u32,
}
impl From<Metadata> for DirectoryStamp {
    fn from(m: Metadata) -> Self {
        Self {
            device: m.dev(),
            inode: m.ino(),
            uid: m.uid(),
            gid: m.gid(),
            mode: m.mode(),
        }
    }
}
#[derive(PartialEq, Eq)]
struct FileStamp {
    directory: DirectoryStamp,
    length: u64,
    links: u64,
    mtime: i64,
    mtime_ns: i64,
    ctime: i64,
    ctime_ns: i64,
}
impl From<Metadata> for FileStamp {
    fn from(m: Metadata) -> Self {
        Self {
            length: m.len(),
            links: m.nlink(),
            mtime: m.mtime(),
            mtime_ns: m.mtime_nsec(),
            ctime: m.ctime(),
            ctime_ns: m.ctime_nsec(),
            directory: m.into(),
        }
    }
}
struct PinnedFile {
    file: File,
    stamp: FileStamp,
}
struct FileSet {
    path: PathBuf,
    fixture: bool,
    uid: u32,
    gid: u32,
    directories: Vec<(File, DirectoryStamp)>,
    files: Vec<PinnedFile>,
}
struct Reader<'policy> {
    policy: &'policy mut CheckedWorkerPolicy,
    set: FileSet,
}

/// Root-owned fixed names only. Not admitted host context or production permission.
pub struct FixedWorkerFilesRead<'policy> {
    reader: Reader<'policy>,
}
/// Explicit caller-owned temporary origin; cannot convert into fixed production proof.
pub struct FixtureWorkerFilesRead<'policy> {
    reader: Reader<'policy>,
}

pub fn inspect_fixed_worker_files<'policy>(
    policy: &'policy mut CheckedWorkerPolicy,
    bytes: &[u8],
    trust: &WorkerPolicyTrust,
    host: &AssertedWorkerHost,
) -> Result<FixedWorkerFilesRead<'policy>, WorkerFilesError> {
    construct(
        Path::new(FIXED_DIRECTORY),
        false,
        (0, 0),
        policy,
        bytes,
        trust,
        host,
    )
    .map(|reader| FixedWorkerFilesRead { reader })
}
pub fn inspect_fixture_worker_files<'policy>(
    path: &Path,
    uid: u32,
    gid: u32,
    policy: &'policy mut CheckedWorkerPolicy,
    bytes: &[u8],
    trust: &WorkerPolicyTrust,
    host: &AssertedWorkerHost,
) -> Result<FixtureWorkerFilesRead<'policy>, WorkerFilesError> {
    construct(path, true, (uid, gid), policy, bytes, trust, host)
        .map(|reader| FixtureWorkerFilesRead { reader })
}
fn construct<'policy>(
    path: &Path,
    fixture: bool,
    owner: (u32, u32),
    policy: &'policy mut CheckedWorkerPolicy,
    bytes: &[u8],
    trust: &WorkerPolicyTrust,
    host: &AssertedWorkerHost,
) -> Result<Reader<'policy>, WorkerFilesError> {
    if policy.is_refused() {
        return Err(WorkerFilesError::Refused);
    }
    let result = (|| {
        policy.revalidate(bytes, trust, host)?;
        if fixture
            && (path.starts_with(FIXED_DIRECTORY)
                || matches!(owner.0, 0 | u32::MAX)
                || matches!(owner.1, 0 | u32::MAX))
        {
            return Err(WorkerFilesError::Security);
        }
        let set = FileSet::open(path, fixture, owner.0, owner.1)?;
        set.observe(&policy.original)?;
        Ok(set)
    })();
    match result {
        Ok(set) => Ok(Reader { policy, set }),
        Err(error) => {
            policy.refused = true;
            Err(error)
        }
    }
}
impl Reader<'_> {
    fn revalidate(
        &mut self,
        bytes: &[u8],
        trust: &WorkerPolicyTrust,
        host: &AssertedWorkerHost,
    ) -> Result<(), WorkerFilesError> {
        if self.policy.is_refused() {
            return Err(WorkerFilesError::Refused);
        }
        let result = (|| {
            self.policy.revalidate(bytes, trust, host)?;
            self.set.observe(&self.policy.original)
        })();
        if result.is_err() {
            self.policy.refused = true;
        }
        result
    }
}
impl FixedWorkerFilesRead<'_> {
    pub fn is_refused(&self) -> bool {
        self.reader.policy.is_refused()
    }
    pub fn revalidate(
        &mut self,
        bytes: &[u8],
        trust: &WorkerPolicyTrust,
        host: &AssertedWorkerHost,
    ) -> Result<(), WorkerFilesError> {
        self.reader.revalidate(bytes, trust, host)
    }
}
impl FixtureWorkerFilesRead<'_> {
    pub fn is_refused(&self) -> bool {
        self.reader.policy.is_refused()
    }
    pub fn revalidate(
        &mut self,
        bytes: &[u8],
        trust: &WorkerPolicyTrust,
        host: &AssertedWorkerHost,
    ) -> Result<(), WorkerFilesError> {
        self.reader.revalidate(bytes, trust, host)
    }
}

impl FileSet {
    fn open(path: &Path, fixture: bool, uid: u32, gid: u32) -> Result<Self, WorkerFilesError> {
        let directories = open_directories(path, fixture, uid, gid)?;
        let parent = &directories.last().ok_or(WorkerFilesError::Security)?.0;
        let mut files = Vec::new();
        for name in NAMES {
            let file = open_child(parent, name.as_bytes(), false)?;
            let metadata = file.metadata()?;
            check_leaf(&metadata, uid, gid)?;
            files.push(PinnedFile {
                file,
                stamp: metadata.into(),
            });
        }
        Ok(Self {
            path: path.into(),
            fixture,
            uid,
            gid,
            directories,
            files,
        })
    }
    fn check_names(&self) -> Result<(), WorkerFilesError> {
        let current = open_directories(&self.path, self.fixture, self.uid, self.gid)?;
        if current.len() != self.directories.len() {
            return Err(WorkerFilesError::Drift);
        }
        for ((_, now), (original, stamp)) in current.iter().zip(&self.directories) {
            if now != stamp || DirectoryStamp::from(original.metadata()?) != *stamp {
                return Err(WorkerFilesError::Drift);
            }
        }
        let parent = &current.last().ok_or(WorkerFilesError::Security)?.0;
        for (name, pinned) in NAMES.into_iter().zip(&self.files) {
            let named = open_child(parent, name.as_bytes(), false)?;
            let metadata = named.metadata()?;
            check_leaf(&metadata, self.uid, self.gid)?;
            if FileStamp::from(metadata) != pinned.stamp
                || FileStamp::from(pinned.file.metadata()?) != pinned.stamp
            {
                return Err(WorkerFilesError::Drift);
            }
        }
        Ok(())
    }
    fn observe(&self, policy: &WorkerPolicy) -> Result<(), WorkerFilesError> {
        // Two bounded sequential rounds, not an atomic cross-file or kernel context proof.
        for _ in 0..2 {
            self.check_names()?;
            let passwd = read_original(&self.files[0])?;
            let group = read_original(&self.files[1])?;
            if sha256_hex(&passwd) != policy.passwd_sha256
                || sha256_hex(&group) != policy.group_sha256
            {
                return Err(WorkerFilesError::Drift);
            }
            check_mapping(&passwd, &group, policy)?;
            self.check_names()?;
        }
        Ok(())
    }
}
fn open_directories(
    path: &Path,
    fixture: bool,
    uid: u32,
    gid: u32,
) -> Result<Vec<(File, DirectoryStamp)>, WorkerFilesError> {
    if !path.is_absolute() || path.as_os_str().len() > 4096 {
        return Err(WorkerFilesError::Security);
    }
    let mut result = Vec::new();
    let root = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open("/")?;
    if !fixture {
        check_directory(&root.metadata()?, 0, 0)?;
    }
    // Stamp the owned descriptor, never a pathname metadata lookup.
    let stamp = root.metadata()?.into();
    result.push((root, stamp));
    for component in path.components() {
        match component {
            Component::RootDir => {}
            Component::Normal(name) => {
                let file = open_child(
                    &result.last().ok_or(WorkerFilesError::Security)?.0,
                    name.as_bytes(),
                    true,
                )?;
                let metadata = file.metadata()?;
                if !fixture {
                    check_directory(&metadata, 0, 0)?;
                }
                result.push((file, metadata.into()));
            }
            _ => return Err(WorkerFilesError::Security),
        }
    }
    check_directory(
        &result
            .last()
            .ok_or(WorkerFilesError::Security)?
            .0
            .metadata()?,
        uid,
        gid,
    )?;
    Ok(result)
}
fn open_child(parent: &File, name: &[u8], directory: bool) -> Result<File, WorkerFilesError> {
    let name = CString::new(name).map_err(|_| WorkerFilesError::Security)?;
    let flags = libc::O_RDONLY
        | libc::O_NOFOLLOW
        | libc::O_CLOEXEC
        | libc::O_NONBLOCK
        | if directory { libc::O_DIRECTORY } else { 0 };
    // SAFETY: owned directory FD and live NUL-terminated component, no creation flags.
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    // SAFETY: successful openat returns a new owned descriptor.
    Ok(unsafe { File::from_raw_fd(fd) })
}
fn check_directory(m: &Metadata, uid: u32, gid: u32) -> Result<(), WorkerFilesError> {
    if !m.is_dir()
        || m.uid() != uid
        || m.gid() != gid
        || m.mode() & 0o7022 != 0
        || m.mode() & 0o100 == 0
    {
        return Err(WorkerFilesError::Security);
    }
    Ok(())
}
fn check_leaf(m: &Metadata, uid: u32, gid: u32) -> Result<(), WorkerFilesError> {
    if !m.is_file()
        || m.uid() != uid
        || m.gid() != gid
        || m.nlink() != 1
        || m.mode() & 0o7133 != 0
        || m.len() == 0
        || m.len() > MAX_ACCOUNT_FILE_BYTES as u64
    {
        return Err(WorkerFilesError::Security);
    }
    Ok(())
}
fn read_original(pinned: &PinnedFile) -> Result<Vec<u8>, WorkerFilesError> {
    read_original_with(pinned, |bytes| pinned.file.read_at(bytes, 0))
}
fn read_original_with(
    pinned: &PinnedFile,
    read: impl FnOnce(&mut [u8]) -> std::io::Result<usize>,
) -> Result<Vec<u8>, WorkerFilesError> {
    if FileStamp::from(pinned.file.metadata()?) != pinned.stamp {
        return Err(WorkerFilesError::Drift);
    }
    let mut bytes = vec![0; pinned.stamp.length as usize + 1];
    let read = read(&mut bytes)?;
    if read != pinned.stamp.length as usize
        || FileStamp::from(pinned.file.metadata()?) != pinned.stamp
    {
        return Err(WorkerFilesError::Drift);
    }
    bytes.truncate(read);
    Ok(bytes)
}

fn records(bytes: &[u8], fields: usize) -> Result<Vec<Vec<&str>>, WorkerFilesError> {
    if bytes.is_empty()
        || bytes.len() > MAX_ACCOUNT_FILE_BYTES
        || !bytes.ends_with(b"\n")
        || bytes
            .iter()
            .any(|b| !b.is_ascii() || (*b < 32 && *b != b'\n') || *b == 127)
    {
        return Err(WorkerFilesError::Document);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| WorkerFilesError::Document)?;
    let mut result = Vec::new();
    for line in text[..text.len() - 1].split('\n') {
        if line.is_empty()
            || line.len() > MAX_ACCOUNT_LINE_BYTES
            || result.len() == MAX_ACCOUNT_RECORDS
        {
            return Err(WorkerFilesError::Document);
        }
        let row: Vec<_> = line.split(':').collect();
        if row.len() != fields || !valid_name(row[0]) {
            return Err(WorkerFilesError::Document);
        }
        result.push(row);
    }
    Ok(result)
}
fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 256
        && name.bytes().enumerate().all(|(i, b)| {
            b.is_ascii_alphanumeric()
                || b == b'_'
                || (i > 0 && (b == b'-' || b == b'.' || (b == b'$' && i + 1 == name.len())))
        })
}
fn numeric(value: &str) -> Result<u32, WorkerFilesError> {
    if value.is_empty()
        || !value.bytes().all(|b| b.is_ascii_digit())
        || (value.len() > 1 && value.starts_with('0'))
    {
        return Err(WorkerFilesError::Document);
    }
    let id: u32 = value.parse().map_err(|_| WorkerFilesError::Document)?;
    if id == u32::MAX {
        return Err(WorkerFilesError::Document);
    }
    Ok(id)
}
fn check_mapping(
    passwd: &[u8],
    group: &[u8],
    policy: &WorkerPolicy,
) -> Result<(), WorkerFilesError> {
    let mut users = BTreeMap::new();
    let mut uids = BTreeSet::new();
    for row in records(passwd, 7)? {
        let uid = numeric(row[2])?;
        let gid = numeric(row[3])?;
        if users.insert(row[0], (uid, gid)).is_some() || !uids.insert(uid) {
            return Err(WorkerFilesError::Document);
        }
    }
    let mut groups = BTreeMap::new();
    let mut gids = BTreeSet::new();
    let mut supplementary = Vec::new();
    for row in records(group, 4)? {
        let gid = numeric(row[2])?;
        if groups.insert(row[0], gid).is_some() || !gids.insert(gid) {
            return Err(WorkerFilesError::Document);
        }
        let mut members = BTreeSet::new();
        if !row[3].is_empty() {
            for member in row[3].split(',') {
                if !valid_name(member) || !users.contains_key(member) || !members.insert(member) {
                    return Err(WorkerFilesError::Document);
                }
            }
        }
        if members.contains(policy.account.as_str()) {
            supplementary.push(gid);
        }
    }
    supplementary.sort_unstable();
    if users.get(policy.account.as_str()) != Some(&(policy.uid, policy.gid))
        || groups.get(policy.group.as_str()) != Some(&policy.gid)
        || supplementary != policy.supplementary_gids
    {
        return Err(WorkerFilesError::Mapping);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_positional_read_refuses_short_overrun_error_and_midread_drift() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("owned");
        std::fs::write(&path, b"original").unwrap();
        let file = File::open(&path).unwrap();
        let pinned = PinnedFile {
            stamp: file.metadata().unwrap().into(),
            file,
        };
        for count in [0, 7, 9] {
            assert!(matches!(
                read_original_with(&pinned, |bytes| {
                    assert_eq!(bytes.len(), 9);
                    Ok(count)
                }),
                Err(WorkerFilesError::Drift)
            ));
        }
        assert!(matches!(
            read_original_with(&pinned, |_| Err(std::io::ErrorKind::Interrupted.into())),
            Err(WorkerFilesError::Io(_))
        ));
        assert_eq!(read_original(&pinned).unwrap(), b"original");
        assert!(matches!(
            read_original_with(&pinned, |_| {
                // Length change is deterministic; do not assume timestamp resolution.
                std::fs::write(&path, b"changed!!")?;
                Ok(8)
            }),
            Err(WorkerFilesError::Drift)
        ));
        assert!(matches!(
            read_original_with(&pinned, |_| panic!(
                "metadata drift must refuse before read"
            )),
            Err(WorkerFilesError::Drift)
        ));
    }
}
