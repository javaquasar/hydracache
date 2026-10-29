//! Numeric-root path/content inventory in the original reader context.
//! Not initial-host, filesystem, build/runtime/job custody or writer revocation.
use crate::diagnostic_artifacts::ArtifactDigest;
use crate::diagnostic_signer_build::MAX_BINARY_BYTES;
use sha2::{Digest, Sha256};
use std::ffi::CString;
use std::fs::{File, OpenOptions};
use std::io;
use std::marker::PhantomData;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{FileExt, MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::rc::Rc;

const MASK: u32 = libc::STATX_TYPE
    | libc::STATX_MODE
    | libc::STATX_NLINK
    | libc::STATX_UID
    | libc::STATX_GID
    | libc::STATX_MTIME
    | libc::STATX_CTIME
    | libc::STATX_INO
    | libc::STATX_SIZE
    | libc::STATX_MNT_ID;
fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "original root-owned input inventory refused",
    )
}
fn readonly(file: &File) -> io::Result<()> {
    // SAFETY: fixed flag queries on a still-owned descriptor.
    let (flags, fdflags) = unsafe {
        (
            libc::fcntl(file.as_raw_fd(), libc::F_GETFL),
            libc::fcntl(file.as_raw_fd(), libc::F_GETFD),
        )
    };
    if flags < 0
        || fdflags < 0
        || flags & (libc::O_ACCMODE | libc::O_PATH) != libc::O_RDONLY
        || fdflags & libc::FD_CLOEXEC == 0
    {
        return Err(invalid());
    }
    Ok(())
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Stamp {
    device: (u32, u32),
    inode: u64,
    mount: u64,
    uid: u32,
    gid: u32,
    mode: u16,
    links: u32,
    bytes: u64,
    modified: (i64, u32),
    changed: (i64, u32),
}
impl Stamp {
    fn decode(s: &libc::statx) -> io::Result<Self> {
        if s.stx_mask & MASK != MASK
            || s.stx_mnt_id == 0
            || s.stx_ino == 0
            || s.stx_mtime.tv_nsec >= 1_000_000_000
            || s.stx_ctime.tv_nsec >= 1_000_000_000
        {
            return Err(invalid());
        }
        Ok(Self {
            device: (s.stx_dev_major, s.stx_dev_minor),
            inode: s.stx_ino,
            mount: s.stx_mnt_id,
            uid: s.stx_uid,
            gid: s.stx_gid,
            mode: s.stx_mode,
            links: s.stx_nlink,
            bytes: s.stx_size,
            modified: (s.stx_mtime.tv_sec, s.stx_mtime.tv_nsec),
            changed: (s.stx_ctime.tv_sec, s.stx_ctime.tv_nsec),
        })
    }
    fn same_directory(&self, other: &Self) -> bool {
        self.device == other.device
            && self.inode == other.inode
            && self.mount == other.mount
            && self.uid == other.uid
            && self.gid == other.gid
            && self.mode == other.mode
    }
    fn allowed(&self, directory: bool, owner: (u32, u32), expected_bytes: u64) -> io::Result<()> {
        let kind = u32::from(self.mode) & libc::S_IFMT;
        let mode = self.mode & 0o7777;
        if (self.uid, self.gid) != owner
            || if directory {
                kind != libc::S_IFDIR || self.links == 0 || ![0o555, 0o755].contains(&mode)
            } else {
                kind != libc::S_IFREG
                    || ![0o444, 0o555, 0o644, 0o755].contains(&mode)
                    || self.links != 1
                    || self.bytes != expected_bytes
            }
        {
            return Err(invalid());
        }
        Ok(())
    }
}
fn stamp(file: &File) -> io::Result<Stamp> {
    readonly(file)?;
    // SAFETY: initialized kernel output, fixed empty path and a live descriptor.
    let mut s: libc::statx = unsafe { std::mem::zeroed() };
    if unsafe {
        libc::statx(
            file.as_raw_fd(),
            c"".as_ptr(),
            libc::AT_EMPTY_PATH | libc::AT_SYMLINK_NOFOLLOW,
            MASK,
            &mut s,
        )
    } != 0
    {
        return Err(io::Error::last_os_error());
    }
    Stamp::decode(&s)
}
fn directory(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open(path)
}
fn child(parent: &File, name: &CString, directory: bool) -> io::Result<File> {
    let flags = libc::O_RDONLY
        | libc::O_NOFOLLOW
        | libc::O_CLOEXEC
        | libc::O_NONBLOCK
        | if directory { libc::O_DIRECTORY } else { 0 };
    // SAFETY: original parent, validated one-component CString, read-only open.
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: exactly one new descriptor owned after successful openat.
    Ok(unsafe { File::from_raw_fd(fd) })
}
fn names(path: &Path, expected: &ArtifactDigest) -> io::Result<Vec<CString>> {
    let bytes = path.as_os_str().as_bytes();
    if bytes.len() > 4096
        || !crate::is_hash(&expected.sha256)
        || expected.bytes == 0
        || expected.bytes > MAX_BINARY_BYTES
    {
        return Err(invalid());
    }
    let rest = bytes.strip_prefix(b"/").ok_or_else(invalid)?;
    let parts: Vec<_> = rest.split(|b| *b == b'/').take(33).collect();
    if parts.is_empty() || parts.len() > 32 {
        return Err(invalid());
    }
    parts
        .into_iter()
        .map(|part| {
            if part.is_empty() || part.len() > 255 || part == b"." || part == b".." {
                return Err(invalid());
            }
            CString::new(part).map_err(|_| invalid())
        })
        .collect()
}
fn tid() -> libc::pid_t {
    // SAFETY: fixed no-argument thread identity query.
    unsafe { libc::syscall(libc::SYS_gettid) as libc::pid_t }
}
struct Namespace {
    file: File,
    id: (u64, u64),
    kind: i32,
}
fn namespace_id(file: &File, kind: i32) -> io::Result<(u64, u64)> {
    readonly(file)?;
    // SAFETY: initialized metadata output and fixed no-pointer nsfs ioctl.
    let mut fs: libc::statfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::fstatfs(file.as_raw_fd(), &mut fs) } != 0 {
        return Err(io::Error::last_os_error());
    }
    if fs.f_type as u64 != libc::NSFS_MAGIC as u64
        || unsafe { libc::ioctl(file.as_raw_fd(), libc::NS_GET_NSTYPE) } != kind
    {
        return Err(invalid());
    }
    let m = file.metadata()?;
    Ok((m.dev(), m.ino()))
}
fn namespace(kind: i32) -> io::Result<Namespace> {
    let path = match kind {
        libc::CLONE_NEWUSER => "/proc/thread-self/ns/user",
        libc::CLONE_NEWNS => "/proc/thread-self/ns/mnt",
        _ => return Err(invalid()),
    };
    // Fixed procfs magic links followed deliberately, never a caller path/FD.
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open(path)?;
    let id = namespace_id(&file, kind)?;
    Ok(Namespace { file, id, kind })
}
struct Context {
    tid: libc::pid_t,
    user: Namespace,
    mount: Namespace,
    root: File,
    root_stamp: Stamp,
}
impl Context {
    fn capture() -> io::Result<Self> {
        let root = directory(Path::new("/"))?;
        let root_stamp = stamp(&root)?;
        let context = Self {
            tid: tid(),
            user: namespace(libc::CLONE_NEWUSER)?,
            mount: namespace(libc::CLONE_NEWNS)?,
            root,
            root_stamp,
        };
        context.check()?;
        Ok(context)
    }
    fn check(&self) -> io::Result<()> {
        if tid() != self.tid {
            return Err(invalid());
        }
        for original in [&self.user, &self.mount] {
            if namespace_id(&original.file, original.kind)? != original.id
                || namespace(original.kind)?.id != original.id
            {
                return Err(invalid());
            }
        }
        if !stamp(&self.root)?.same_directory(&self.root_stamp)
            || !stamp(&directory(Path::new("/"))?)?.same_directory(&self.root_stamp)
            || tid() != self.tid
        {
            return Err(invalid());
        }
        Ok(())
    }
}
struct Node {
    file: File,
    stamp: Stamp,
}
fn walk(anchor: &Path, parts: &[CString], owner: (u32, u32), bytes: u64) -> io::Result<Vec<Node>> {
    let file = directory(anchor)?;
    let root = stamp(&file)?;
    root.allowed(true, owner, bytes)?;
    let mut nodes = vec![Node { file, stamp: root }];
    for (n, part) in parts.iter().enumerate() {
        let is_dir = n + 1 < parts.len();
        let file = child(&nodes.last().ok_or_else(invalid)?.file, part, is_dir)?;
        let stamp = stamp(&file)?;
        stamp.allowed(is_dir, owner, bytes)?;
        nodes.push(Node { file, stamp });
    }
    Ok(nodes)
}
fn hash(file: &File, expected: &ArtifactDigest) -> io::Result<()> {
    let mut buffer = [0u8; 65536];
    let mut count = 0;
    let mut hash = Sha256::new();
    loop {
        let budget = (expected.bytes + 1 - count).min(buffer.len() as u64) as usize;
        let n = file.read_at(&mut buffer[..budget], count)?;
        if n == 0 {
            break;
        }
        count += n as u64;
        if count > expected.bytes {
            return Err(invalid());
        }
        hash.update(&buffer[..n]);
    }
    if count != expected.bytes || crate::hex(&hash.finalize()) != expected.sha256 {
        return Err(invalid());
    }
    Ok(())
}

/// Observe numeric root-owned path and bytes in this thread's current context.
/// Path/digest are assertions, not independent runtime/job or initial-host policy.
/// No caller UID/GID, context descriptor or copy-guard conversion is accepted.
pub fn inspect_root_owned_input(
    path: &Path,
    expected: ArtifactDigest,
) -> io::Result<RootOwnedInputRead> {
    inspect_at(path, expected, Path::new("/"), (0, 0))
}
fn inspect_at(
    path: &Path,
    expected: ArtifactDigest,
    anchor: &Path,
    owner: (u32, u32),
) -> io::Result<RootOwnedInputRead> {
    let parts = names(path, &expected)?;
    let context = Context::capture()?;
    let nodes = walk(anchor, &parts, owner, expected.bytes)?;
    let mut read = RootOwnedInputRead {
        parts,
        anchor: anchor.to_owned(),
        owner,
        context,
        nodes,
        expected,
        refused: false,
        _thread: PhantomData,
    };
    read.check_original()?;
    Ok(read)
}
/// Original read-only inventory, never serialized, executable or signing authority.
/// Numeric root and same namespaces do not authenticate initial-host enrollment.
///
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_root_owned_input::RootOwnedInputRead;
/// fn copy(r: RootOwnedInputRead) { let _ = r.clone(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_root_owned_input::RootOwnedInputRead;
/// use std::os::fd::AsRawFd;
/// fn export(r: RootOwnedInputRead) { let _ = r.as_raw_fd(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_root_owned_input::RootOwnedInputRead;
/// fn move_thread(r: RootOwnedInputRead) { std::thread::spawn(move || drop(r)); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::{diagnostic_root_owned_input::RootOwnedInputRead, diagnostic_signer_bundle::AssertedSignerBundle};
/// fn admit(r: RootOwnedInputRead) -> AssertedSignerBundle { r.into() }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::{diagnostic_root_owned_input::inspect_root_owned_input, diagnostic_artifacts::ArtifactDigest};
/// fn override_owner(path: &std::path::Path, digest: ArtifactDigest) { inspect_root_owned_input(path, digest, (1000, 1000)); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_root_owned_input::RootOwnedInputRead;
/// fn decode<T: serde::de::DeserializeOwned>() {}
/// decode::<RootOwnedInputRead>();
/// ```
pub struct RootOwnedInputRead {
    parts: Vec<CString>,
    anchor: PathBuf,
    owner: (u32, u32),
    context: Context,
    nodes: Vec<Node>,
    expected: ArtifactDigest,
    refused: bool,
    _thread: PhantomData<Rc<()>>,
}
impl RootOwnedInputRead {
    pub(crate) fn refuse(&mut self) {
        self.refused = true;
    }

    // Internal snapshot seam for the runtime join, never a public FD or byte reader.
    // Both original checks and this read use the same retained leaf description.
    pub(crate) fn read_original_bounded(&mut self, maximum: u64) -> io::Result<Vec<u8>> {
        let result = (|| {
            if self.expected.bytes > maximum || maximum > MAX_BINARY_BYTES {
                return Err(invalid());
            }
            self.check_original()?;
            let file = &self.nodes.last().ok_or_else(invalid)?.file;
            let mut bytes = vec![0; self.expected.bytes as usize + 1];
            let mut count = 0;
            while count < bytes.len() {
                let n = file.read_at(&mut bytes[count..], count as u64)?;
                if n == 0 {
                    break;
                }
                count += n;
            }
            if count as u64 != self.expected.bytes
                || crate::sha256_hex(&bytes[..count]) != self.expected.sha256
            {
                return Err(invalid());
            }
            self.check_original()?;
            bytes.truncate(count);
            Ok(bytes)
        })();
        if result.is_err() {
            self.refuse();
        }
        result
    }

    pub fn is_refused(&self) -> bool {
        self.refused
    }
    pub fn check_original(&mut self) -> io::Result<&ArtifactDigest> {
        if self.refused {
            return Err(invalid());
        }
        let result = self.check();
        if result.is_err() {
            self.refused = true;
        }
        result?;
        Ok(&self.expected)
    }
    fn compare(&self, fresh: &[Node]) -> io::Result<()> {
        if fresh.len() != self.nodes.len() {
            return Err(invalid());
        }
        for (i, (a, b)) in self.nodes.iter().zip(fresh).enumerate() {
            let current = stamp(&a.file)?;
            let named = stamp(&b.file)?;
            let directory = i + 1 < self.nodes.len();
            current.allowed(directory, self.owner, self.expected.bytes)?;
            named.allowed(directory, self.owner, self.expected.bytes)?;
            if if directory {
                !current.same_directory(&a.stamp) || !named.same_directory(&a.stamp)
            } else {
                current != a.stamp || named != a.stamp
            } {
                return Err(invalid());
            }
        }
        Ok(())
    }
    fn check(&self) -> io::Result<()> {
        self.context.check()?;
        let fresh = walk(&self.anchor, &self.parts, self.owner, self.expected.bytes)?;
        self.compare(&fresh)?;
        self.context.check()?;
        hash(&self.nodes.last().ok_or_else(invalid)?.file, &self.expected)?;
        self.compare(&fresh)?;
        self.context.check()?;
        self.compare(&walk(
            &self.anchor,
            &self.parts,
            self.owner,
            self.expected.bytes,
        )?)?;
        self.context.check()
    }
}
#[cfg(test)]
pub(crate) fn inspect_runtime_fixture(
    path: &Path,
    expected: ArtifactDigest,
    anchor: &Path,
    owner: (u32, u32),
) -> io::Result<RootOwnedInputRead> {
    inspect_at(path, expected, anchor, owner)
}
#[cfg(test)]
#[path = "diagnostic_root_owned_input_tests.rs"]
mod tests;
