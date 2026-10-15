//! Private normal-build materializer, not a trusted launcher or runtime custodian.
use std::collections::BTreeSet;
use std::ffi::{CStr, CString};
use std::fs::{File, Metadata, OpenOptions};
use std::io::{self, Write};
use std::marker::PhantomData;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{FileExt, MetadataExt, OpenOptionsExt};
use std::rc::Rc;

const MAX_IMAGE: u64 = 8 * 1024 * 1024;
const DIRS: [(usize, &CStr); 3] = [(0, c"lib"), (1, c"x86_64-linux-gnu"), (0, c"lib64")];
const FILES: [(usize, &CStr); 4] = [
    (0, c"signer"),
    (3, c"ld-linux-x86-64.so.2"),
    (2, c"libc.so.6"),
    (2, c"libgcc_s.so.1"),
];
fn refused() -> io::Error {
    io::Error::other("fixed private GNU root refused")
}
fn require(value: bool) -> io::Result<()> {
    if value {
        Ok(())
    } else {
        Err(refused())
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Stamp {
    dev: u64,
    ino: u64,
    mode: u32,
    uid: u32,
    gid: u32,
    links: u64,
    len: u64,
    modified: (i64, i64),
    changed: (i64, i64),
}
impl Stamp {
    fn of(m: &Metadata) -> Self {
        Self {
            dev: m.dev(),
            ino: m.ino(),
            mode: m.mode(),
            uid: m.uid(),
            gid: m.gid(),
            links: m.nlink(),
            len: m.len(),
            modified: (m.mtime(), m.mtime_nsec()),
            changed: (m.ctime(), m.ctime_nsec()),
        }
    }
    fn same_object(&self, other: &Self) -> bool {
        (self.dev, self.ino, self.mode & libc::S_IFMT)
            == (other.dev, other.ino, other.mode & libc::S_IFMT)
    }
}
struct Node {
    file: File,
    stamp: Stamp,
}
impl Node {
    fn new(file: File) -> io::Result<Self> {
        let stamp = Stamp::of(&file.metadata()?);
        Ok(Self { file, stamp })
    }
    fn check(&self) -> io::Result<()> {
        require(Stamp::of(&self.file.metadata()?) == self.stamp)?;
        // SAFETY: query flags of a retained live descriptor only.
        let flags = unsafe { libc::fcntl(self.file.as_raw_fd(), libc::F_GETFL) };
        let descriptor = unsafe { libc::fcntl(self.file.as_raw_fd(), libc::F_GETFD) };
        require(
            flags >= 0
                && flags & libc::O_ACCMODE == libc::O_RDONLY
                && descriptor >= 0
                && descriptor & libc::FD_CLOEXEC != 0,
        )
    }
}
struct Source {
    node: Node,
    bytes: Vec<u8>,
    digest: [u8; 32],
}
fn digest(bytes: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes).into()
}
fn contents(node: &Node) -> io::Result<Vec<u8>> {
    node.check()?;
    require((64..=MAX_IMAGE).contains(&node.stamp.len))?;
    let mut bytes = vec![0; node.stamp.len as usize];
    node.file.read_exact_at(&mut bytes, 0)?;
    let mut extra = [0];
    require(node.file.read_at(&mut extra, node.stamp.len)? == 0)?;
    node.check()?;
    Ok(bytes)
}
impl Source {
    fn capture(file: File, uid: u32, self_image: bool) -> io::Result<Self> {
        let node = Node::new(file)?;
        require(
            node.stamp.mode & libc::S_IFMT == libc::S_IFREG
                && node.stamp.uid == uid
                && node.stamp.links == 1
                && node.stamp.mode & 0o7022 == 0
                && (!self_image || node.stamp.mode & 0o7777 == 0o555),
        )?;
        let bytes = contents(&node)?;
        let digest = digest(&bytes);
        Ok(Self {
            node,
            bytes,
            digest,
        })
    }
    fn check(&self) -> io::Result<()> {
        require(digest(&contents(&self.node)?) == self.digest)
    }
}
fn open_at(parent: &File, name: &CStr, directory: bool) -> io::Result<File> {
    // SAFETY: valid parent and NUL-terminated fixed name; ownership of new fd only.
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY
                | libc::O_CLOEXEC
                | libc::O_NOFOLLOW
                | libc::O_NONBLOCK
                | if directory { libc::O_DIRECTORY } else { 0 },
        )
    };
    if fd < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(unsafe { File::from_raw_fd(fd) })
    }
}
fn chmod(file: &File, mode: libc::mode_t) -> io::Result<()> {
    // SAFETY: retained original descriptor, not a reopened path.
    if unsafe { libc::fchmod(file.as_raw_fd(), mode) } == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}
fn mkdir(parent: &File, name: &CStr) -> io::Result<File> {
    // A collision refuses: no adoption, deletion or retry of the existing name.
    if unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) } != 0 {
        return Err(io::Error::last_os_error());
    }
    // If acquisition fails, leave the unowned name; never delete by guesswork.
    open_at(parent, name, true)
}
fn listing(file: &File) -> io::Result<BTreeSet<std::ffi::OsString>> {
    let entries = std::fs::read_dir(format!("/proc/self/fd/{}", file.as_raw_fd()))?
        .take(5)
        .map(|e| e.map(|e| e.file_name()))
        .collect::<io::Result<BTreeSet<_>>>()?;
    require(entries.len() <= 4)?;
    Ok(entries)
}
fn random_name() -> io::Result<CString> {
    let mut bytes = [0u8; 16];
    let mut filled = 0;
    for _ in 0..8 {
        let count = unsafe {
            libc::syscall(
                libc::SYS_getrandom,
                bytes[filled..].as_mut_ptr(),
                bytes.len() - filled,
                0,
            )
        };
        if count < 0 {
            if io::Error::last_os_error().raw_os_error() == Some(libc::EINTR) {
                continue;
            }
            return Err(refused());
        }
        require(count > 0 && count as usize <= bytes.len() - filled)?;
        filled += count as usize;
        if filled == bytes.len() {
            use std::fmt::Write;
            let mut name = String::from("hc074-owned-root-v1-");
            for byte in bytes {
                write!(name, "{byte:02x}").map_err(|_| refused())?;
            }
            return CString::new(name).map_err(|_| refused());
        }
    }
    Err(refused())
}
struct Root {
    parent: File,
    name: CString,
    dirs: Vec<Node>,
    files: Vec<Node>,
    sources: [Source; 4],
    refused: bool,
    cleanup_attempted: bool,
    _not_send_sync: PhantomData<Rc<()>>,
}
impl Root {
    fn create(parent: File, name: CString, sources: [Source; 4]) -> io::Result<Self> {
        for source in &sources {
            source.check()?;
        }
        hydracache_long_run_supervisor_074::diagnostic_signer_gnu::inspect_gnu_declarations(
            sources.each_ref().map(|source| source.bytes.as_slice()),
        )?;
        let first = Node::new(mkdir(&parent, &name)?)?;
        let mut root = Self {
            parent,
            name,
            dirs: vec![first],
            files: vec![],
            sources,
            refused: false,
            cleanup_attempted: false,
            _not_send_sync: PhantomData,
        };
        for (parent, name) in DIRS {
            root.dirs
                .push(Node::new(mkdir(&root.dirs[parent].file, name)?)?);
        }
        for (index, (parent, name)) in FILES.iter().enumerate() {
            let dir = &root.dirs[*parent].file;
            let fd = unsafe {
                libc::openat(
                    dir.as_raw_fd(),
                    name.as_ptr(),
                    libc::O_WRONLY
                        | libc::O_CREAT
                        | libc::O_EXCL
                        | libc::O_NOFOLLOW
                        | libc::O_CLOEXEC,
                    0o600,
                )
            };
            if fd < 0 {
                return Err(io::Error::last_os_error());
            }
            let mut writer = unsafe { File::from_raw_fd(fd) };
            // Track the original before any fallible data operation. The temporary
            // writer is dropped before Root's error cleanup or any future launch.
            root.files.push(Node::new(open_at(dir, name, false)?)?);
            writer.write_all(&root.sources[index].bytes)?;
            chmod(&writer, 0o555)?;
            drop(writer);
        }
        for dir in &root.dirs {
            chmod(&dir.file, 0o555)?;
        }
        for node in root.dirs.iter_mut().chain(root.files.iter_mut()) {
            node.stamp = Stamp::of(&node.file.metadata()?);
        }
        root.check()?;
        Ok(root)
    }
    fn binding(parent: &File, name: &CStr, node: &Node, directory: bool) -> io::Result<()> {
        require(
            node.stamp
                .same_object(&Stamp::of(&open_at(parent, name, directory)?.metadata()?)),
        )
    }
    fn names(&self) -> io::Result<()> {
        Self::binding(&self.parent, &self.name, &self.dirs[0], true)?;
        let mut expected: Vec<BTreeSet<std::ffi::OsString>> =
            (0..self.dirs.len()).map(|_| BTreeSet::new()).collect();
        for (i, (parent, name)) in DIRS.iter().take(self.dirs.len() - 1).enumerate() {
            Self::binding(&self.dirs[*parent].file, name, &self.dirs[i + 1], true)?;
            expected[*parent].insert(name.to_string_lossy().as_ref().into());
        }
        for (i, (parent, name)) in FILES.iter().take(self.files.len()).enumerate() {
            Self::binding(&self.dirs[*parent].file, name, &self.files[i], false)?;
            expected[*parent].insert(name.to_string_lossy().as_ref().into());
        }
        for (node, expected) in self.dirs.iter().zip(expected) {
            require(listing(&node.file)? == expected)?;
        }
        Ok(())
    }
    fn checked(&self) -> io::Result<()> {
        require(
            !self.refused
                && !self.cleanup_attempted
                && self.dirs.len() == 4
                && self.files.len() == 4,
        )?;
        self.names()?;
        for node in self.dirs.iter().chain(self.files.iter()) {
            node.check()?;
            require(
                node.stamp.uid == unsafe { libc::getuid() }
                    && node.stamp.gid == unsafe { libc::getgid() }
                    && node.stamp.mode & 0o7777 == 0o555,
            )?;
        }
        for (file, source) in self.files.iter().zip(&self.sources) {
            source.check()?;
            require(file.stamp.links == 1 && digest(&contents(file)?) == source.digest)?;
        }
        Ok(())
    }
    fn check(&mut self) -> io::Result<()> {
        let result = self.checked();
        self.refused |= result.is_err();
        result
    }
    fn cleanup(&mut self) -> io::Result<()> {
        require(!self.cleanup_attempted)?;
        self.cleanup_attempted = true;
        // All names/inventories checked BEFORE mutation. Cooperative local scope;
        // no atomic compare-and-unlink claim against a hostile same-UID mutator.
        self.names()?;
        for node in &self.dirs {
            chmod(&node.file, 0o700)?;
        }
        for (parent, name) in FILES.iter().take(self.files.len()) {
            unlink(&self.dirs[*parent].file, name, false)?;
        }
        for (parent, name) in DIRS.iter().take(self.dirs.len() - 1).rev() {
            unlink(&self.dirs[*parent].file, name, true)?;
        }
        unlink(&self.parent, &self.name, true)
    }
}
impl Drop for Root {
    fn drop(&mut self) {
        if !self.cleanup_attempted && self.cleanup().is_err() {
            eprintln!("Fixed root cleanup refused; original objects retained.");
        }
    }
}
fn unlink(parent: &File, name: &CStr, directory: bool) -> io::Result<()> {
    if unsafe {
        libc::unlinkat(
            parent.as_raw_fd(),
            name.as_ptr(),
            if directory { libc::AT_REMOVEDIR } else { 0 },
        )
    } == 0
    {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}
pub(super) fn prepare() -> io::Result<()> {
    require(
        std::env::vars_os().next().is_none()
            && unsafe { libc::getuid() } != 0
            && unsafe { libc::getuid() == libc::geteuid() && libc::getgid() == libc::getegid() }
            && unsafe { libc::syscall(libc::SYS_gettid) == libc::getpid() as libc::c_long }
            && std::fs::read_dir("/proc/self/task")?
                .take(2)
                .collect::<io::Result<Vec<_>>>()?
                .len()
                == 1,
    )?;
    let paths = [
        "/proc/self/exe",
        "/lib64/ld-linux-x86-64.so.2",
        "/lib/x86_64-linux-gnu/libc.so.6",
        "/lib/x86_64-linux-gnu/libgcc_s.so.1",
    ];
    let mut sources = Vec::new();
    for (index, path) in paths.iter().enumerate() {
        // Fixed distribution symlinks deliberately followed, never caller paths.
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_CLOEXEC)
            .open(path)?;
        sources.push(Source::capture(
            file,
            if index == 0 {
                unsafe { libc::getuid() }
            } else {
                0
            },
            index == 0,
        )?);
    }
    let parent = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open("/tmp")?;
    let meta = parent.metadata()?;
    require(meta.uid() == 0 && meta.mode() & 0o1777 == 0o1777)?;
    let mut root = Root::create(
        parent,
        random_name()?,
        sources.try_into().map_err(|_| refused())?,
    )?;
    root.check()?;
    root.cleanup()
}

#[cfg(test)]
#[path = "diagnostic_signer_owned_root_tests.rs"]
mod tests;
