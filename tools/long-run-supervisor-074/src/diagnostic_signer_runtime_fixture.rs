//! Private readonly fixture inventory. No loaded-root, custody or launch authority.
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::ffi::CString;
use std::fs::{self, File, Metadata, OpenOptions};
use std::io;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{FileExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

#[path = "diagnostic_signer_runtime_elf.rs"]
mod elf;
const NAMES: [&str; 4] = [
    "signer",
    "ld-linux-x86-64.so.2",
    "libc.so.6",
    "libgcc_s.so.1",
];
fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "owned runtime inventory refused",
    )
}
pub(super) fn read_bounded(file: &File) -> io::Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut chunk = [0; 16384];
    loop {
        let remaining = elf::MAX_IMAGE + 1 - out.len();
        let count = file.read_at(&mut chunk[..remaining.min(16384)], out.len() as u64)?;
        if count == 0 {
            break;
        }
        out.extend_from_slice(&chunk[..count]);
        if out.len() > elf::MAX_IMAGE {
            return Err(invalid());
        }
    }
    if out.len() < 64 {
        return Err(invalid());
    }
    Ok(out)
}
struct Fixture {
    temp: tempfile::TempDir,
    root: PathBuf,
}
impl Fixture {
    fn new() -> io::Result<Self> {
        // SAFETY: fixed identity query; these local fixture operations require nonroot.
        if unsafe { libc::geteuid() } == 0 {
            return Err(invalid());
        }
        static IMAGES: OnceLock<Result<Vec<Vec<u8>>, String>> = OnceLock::new();
        let images = IMAGES.get_or_init(|| {
            let prepare = || -> io::Result<Vec<Vec<u8>>> {
                let mut images = vec![super::diagnostic_signer_joined_fixture::compiled_helper()?];
                // Explicit local source copies only; symlink resolution here is
                // fixture setup, never production source/ancestry enrollment.
                for source in [
                    "/lib64/ld-linux-x86-64.so.2",
                    "/lib/x86_64-linux-gnu/libc.so.6",
                    "/lib/x86_64-linux-gnu/libgcc_s.so.1",
                ] {
                    images.push(read_bounded(&File::open(source)?)?);
                }
                Ok(images)
            };
            prepare().map_err(|error| error.to_string())
        });
        let images = images.as_ref().map_err(|e| io::Error::other(e.clone()))?;
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("root");
        fs::create_dir(&root)?;
        let fixture = Self { temp, root };
        for (name, bytes) in NAMES.iter().zip(images) {
            let path = fixture.root.join(name);
            fs::write(&path, bytes)?;
            fs::set_permissions(path, fs::Permissions::from_mode(0o555))?;
        }
        fs::set_permissions(&fixture.root, fs::Permissions::from_mode(0o555))?;
        Ok(fixture)
    }
    fn writable(&self) {
        fs::set_permissions(&self.root, fs::Permissions::from_mode(0o755)).unwrap();
    }
    fn freeze(&self) {
        fs::set_permissions(&self.root, fs::Permissions::from_mode(0o555)).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        // Only owned temporary directories. Never follow a substituted symlink.
        for name in ["root", "root-old"] {
            let path = self.temp.path().join(name);
            if fs::symlink_metadata(&path).is_ok_and(|m| m.is_dir() && !m.file_type().is_symlink())
            {
                if let Err(error) = fs::set_permissions(path, fs::Permissions::from_mode(0o755)) {
                    eprintln!("owned runtime fixture cleanup preparation failed: {error}");
                }
            }
        }
    }
}
#[derive(Clone, PartialEq, Eq)]
struct Stamp {
    dev: u64,
    ino: u64,
    uid: u32,
    gid: u32,
    mode: u32,
    links: u64,
    length: u64,
    mtime: (i64, i64),
    ctime: (i64, i64),
}
impl Stamp {
    fn of(m: Metadata) -> Self {
        Self {
            dev: m.dev(),
            ino: m.ino(),
            uid: m.uid(),
            gid: m.gid(),
            mode: m.mode(),
            links: m.nlink(),
            length: m.len(),
            mtime: (m.mtime(), m.mtime_nsec()),
            ctime: (m.ctime(), m.ctime_nsec()),
        }
    }
}
fn root_file(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
}
fn child_file(root: &File, name: &'static str) -> io::Result<File> {
    if !NAMES.contains(&name) {
        return Err(invalid());
    }
    let name = CString::new(name).unwrap();
    // SAFETY: retained directory descriptor, bounded static child name and fixed flags.
    let fd = unsafe {
        libc::openat(
            root.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: one successful new descriptor transferred exactly once.
    Ok(unsafe { File::from_raw_fd(fd) })
}
fn readonly(file: &File, directory: bool, uid: u32, gid: u32) -> io::Result<Stamp> {
    let m = file.metadata()?;
    // SAFETY: flag query on a borrowed live FD, no transfer or mutation.
    let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) };
    if flags < 0
        || flags & libc::O_ACCMODE != libc::O_RDONLY
        || flags & libc::O_PATH != 0
        || m.uid() != uid
        || m.gid() != gid
        || m.mode() & 0o7777 != 0o555
        || if directory {
            !m.is_dir()
        } else {
            !m.is_file() || m.nlink() != 1 || m.len() < 64 || m.len() > elf::MAX_IMAGE as u64
        }
    {
        return Err(invalid());
    }
    Ok(Stamp::of(m))
}
fn entries(root: &File) -> io::Result<()> {
    let names = fs::read_dir(format!("/proc/self/fd/{}", root.as_raw_fd()))?
        .take(5)
        .map(|e| e.map(|e| e.file_name()))
        .collect::<io::Result<BTreeSet<_>>>()?;
    if names != NAMES.map(std::ffi::OsString::from).into_iter().collect() {
        return Err(invalid());
    }
    Ok(())
}
fn graph(info: &[elf::Info]) -> io::Result<()> {
    if info.len() != 4
        || info[0].interpreter.as_deref() != Some(elf::LOADER)
        || info[0].soname.is_some()
        || info[1].interpreter.is_some()
        || !info[1].needed.is_empty()
    {
        return Err(invalid());
    }
    for index in 1..4 {
        if info[index].soname.as_deref() != Some(NAMES[index]) {
            return Err(invalid());
        }
    }
    let mut reached = BTreeSet::from([0usize, 1]);
    for _ in 0..4 {
        for (index, item) in info.iter().enumerate() {
            for name in &item.needed {
                let next = NAMES
                    .iter()
                    .position(|candidate| candidate == name)
                    .ok_or_else(invalid)?;
                if next == 0 || next == index {
                    return Err(invalid());
                }
                if reached.contains(&index) {
                    reached.insert(next);
                }
            }
        }
    }
    if reached.len() != 4 {
        return Err(invalid());
    }
    Ok(())
}
// Pure declaration validation shared by private test fixtures, never a retained
// guard/FD conversion or runtime-custody acceptance token.
pub(super) fn validate_images(images: &[Vec<u8>; 4]) -> io::Result<()> {
    graph(
        &images
            .iter()
            .enumerate()
            .map(|(index, bytes)| elf::parse(bytes, index == 0))
            .collect::<io::Result<Vec<_>>>()?,
    )
}
struct Input {
    name: &'static str,
    file: File,
    stamp: Stamp,
    hash: Vec<u8>,
    info: elf::Info,
}
struct Retained {
    path: PathBuf,
    root: File,
    stamp: Stamp,
    uid: u32,
    gid: u32,
    inputs: Vec<Input>,
    refused: bool,
}
impl Retained {
    fn new(fixture: &Fixture) -> io::Result<Self> {
        // SAFETY: no-argument fixture identity queries, not account/provider enrollment.
        let (uid, gid) = unsafe { (libc::geteuid(), libc::getegid()) };
        if uid == 0 {
            return Err(invalid());
        }
        let root = root_file(&fixture.root)?;
        let stamp = readonly(&root, true, uid, gid)?;
        entries(&root)?;
        let mut inputs = Vec::new();
        for name in NAMES {
            let file = child_file(&root, name)?;
            let original = readonly(&file, false, uid, gid)?;
            let bytes = read_bounded(&file)?;
            let info = elf::parse(&bytes, name == NAMES[0])?;
            let hash = Sha256::digest(&bytes).to_vec();
            if readonly(&file, false, uid, gid)? != original {
                return Err(invalid());
            }
            inputs.push(Input {
                name,
                file,
                stamp: original,
                hash,
                info,
            });
        }
        graph(
            &inputs
                .iter()
                .map(|input| input.info.clone())
                .collect::<Vec<_>>(),
        )?;
        let mut retained = Self {
            path: fixture.root.clone(),
            root,
            stamp,
            uid,
            gid,
            inputs,
            refused: false,
        };
        retained.check()?;
        Ok(retained)
    }
    fn check_root(&self) -> io::Result<()> {
        let current = root_file(&self.path)?;
        if readonly(&current, true, self.uid, self.gid)? != self.stamp
            || readonly(&self.root, true, self.uid, self.gid)? != self.stamp
        {
            return Err(invalid());
        }
        entries(&self.root)?;
        Ok(())
    }
    fn check(&mut self) -> io::Result<()> {
        if self.refused {
            return Err(invalid());
        }
        let result = self.check_inner();
        if result.is_err() {
            self.refused = true;
        }
        result
    }
    fn check_inner(&self) -> io::Result<()> {
        self.check_root()?;
        for input in &self.inputs {
            let current = child_file(&self.root, input.name)?;
            if readonly(&current, false, self.uid, self.gid)? != input.stamp
                || readonly(&input.file, false, self.uid, self.gid)? != input.stamp
            {
                return Err(invalid());
            }
            let bytes = read_bounded(&input.file)?;
            if Sha256::digest(&bytes).as_slice() != input.hash
                || elf::parse(&bytes, input.name == NAMES[0])? != input.info
                || readonly(&input.file, false, self.uid, self.gid)? != input.stamp
                || readonly(&current, false, self.uid, self.gid)? != input.stamp
            {
                return Err(invalid());
            }
        }
        self.check_root()
    }
}

#[path = "diagnostic_signer_runtime_fixture_tests.rs"]
mod tests;
