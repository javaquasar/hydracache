//! Fixed root-selected pathname and retained inode, NOT a child trust opener.
use super::*;
use crate::diagnostic_artifacts::linux::held_role_census::bootstrap_images::entry::{
    installed::InstalledBindings, EntryBindings,
};
use sha2::{Digest, Sha256};
use std::{
    ffi::CStr,
    fs::{File, Metadata, OpenOptions},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
};
const DIRECTORIES: [&CStr; 3] = [c"opt", c"hydracache-perf", c"bin"];
fn need(ok: bool) -> Result<(), ArtifactError> {
    if ok {
        Ok(())
    } else {
        Err(ArtifactError::Contents)
    }
}
fn root(path: &Path) -> Result<File, ArtifactError> {
    Ok(OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open(path)?)
}
fn child(parent: &File, name: &CStr, directory: bool) -> Result<File, ArtifactError> {
    let flags = libc::O_RDONLY
        | libc::O_CLOEXEC
        | libc::O_NOFOLLOW
        | libc::O_NONBLOCK
        | if directory { libc::O_DIRECTORY } else { 0 };
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}
fn readonly(file: &File) -> Result<[i32; 2], ArtifactError> {
    let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) };
    let fdflags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFD) };
    need(
        flags >= 0
            && flags & (libc::O_ACCMODE | libc::O_PATH) == libc::O_RDONLY
            && fdflags == libc::FD_CLOEXEC,
    )?;
    Ok([fdflags, flags])
}
#[derive(PartialEq, Eq)]
struct DirectoryStamp([u64; 7]);
fn directory(file: &File, owner: [u32; 2]) -> Result<DirectoryStamp, ArtifactError> {
    let flags = readonly(file)?;
    let m = file.metadata()?;
    need(
        m.is_dir()
            && [m.uid(), m.gid()] == owner
            && m.mode() & 0o7022 == 0
            && m.mode() & 0o101 == 0o101,
    )?;
    Ok(DirectoryStamp([
        m.dev(),
        m.ino(),
        u64::from(m.uid()),
        u64::from(m.gid()),
        u64::from(m.mode()),
        flags[0] as u64,
        flags[1] as u64,
    ]))
}
#[derive(PartialEq, Eq)]
struct ImageStamp([u64; 11], [i32; 2]);
fn metadata(m: &Metadata) -> [u64; 11] {
    [
        m.dev(),
        m.ino(),
        m.len(),
        u64::from(m.uid()),
        u64::from(m.gid()),
        u64::from(m.mode()),
        m.nlink(),
        m.mtime() as u64,
        m.mtime_nsec() as u64,
        m.ctime() as u64,
        m.ctime_nsec() as u64,
    ]
}
fn image(file: &File, owner: [u32; 2]) -> Result<ImageStamp, ArtifactError> {
    let flags = readonly(file)?;
    let m = file.metadata()?;
    need(
        m.is_file()
            && [m.uid(), m.gid()] == owner
            && m.nlink() == 1
            && matches!(m.mode() & 0o7777, 0o555 | 0o755)
            && m.len() > 0
            && m.len() <= MAX_BINARY_BYTES
            && unsafe { libc::lseek(file.as_raw_fd(), 0, libc::SEEK_CUR) } == 0,
    )?;
    Ok(ImageStamp(metadata(&m), flags))
}
fn contents(file: &File, build: &VerifiedBuild) -> Result<(), ArtifactError> {
    let expected = &build.statement.bootstrap;
    need(bounded(expected, MAX_BINARY_BYTES))?;
    let mut header = [0; 64];
    need(pread(file, &mut header, 0)? == header.len())?;
    check_elf(&header, expected.bytes)?;
    let mut hash = Sha256::new();
    let mut buffer = [0; 65536];
    let mut at = 0;
    while at < expected.bytes {
        let count = (expected.bytes - at).min(buffer.len() as u64) as usize;
        // Errors, including EINTR, refuse; there is no unbounded retry loop.
        let n = pread(file, &mut buffer[..count], at)?;
        need(n > 0 && n <= count)?;
        hash.update(&buffer[..n]);
        at += n as u64;
    }
    need(crate::hex(&hash.finalize()) == expected.sha256)
}
fn pread(file: &File, raw: &mut [u8], at: u64) -> Result<usize, ArtifactError> {
    need(raw.len() <= 65536 && at <= MAX_BINARY_BYTES)?;
    let n = unsafe {
        libc::pread(
            file.as_raw_fd(),
            raw.as_mut_ptr().cast(),
            raw.len(),
            at as libc::off_t,
        )
    };
    if n < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    need(n as usize <= raw.len())?;
    Ok(n as usize)
}
pub(crate) struct InstalledSource {
    root_name: PathBuf,
    owner: [u32; 2],
    directories: Vec<(File, DirectoryStamp)>,
    image: File,
    retained: ImageStamp,
    refused: bool,
}
impl InstalledSource {
    // No public/caller-selected root or image. This is only the privileged parent.
    pub(crate) fn open_fixed(build: &VerifiedBuild) -> Result<Self, ArtifactError> {
        need(unsafe { libc::geteuid() } == 0 && unsafe { libc::getegid() } == 0)?;
        Self::open_under(Path::new("/"), [0, 0], build)
    }
    fn open_under(
        path: &Path,
        owner: [u32; 2],
        build: &VerifiedBuild,
    ) -> Result<Self, ArtifactError> {
        let file = root(path)?;
        let mut directories = Vec::with_capacity(4);
        let first = directory(&file, owner)?;
        directories.push((file, first));
        for name in DIRECTORIES {
            let file = child(
                &directories.last().ok_or(ArtifactError::Contents)?.0,
                name,
                true,
            )?;
            let stamp = directory(&file, owner)?;
            directories.push((file, stamp));
        }
        let file = child(&directories[3].0, c"diagnostic-held-bootstrap-074", false)?;
        let retained = image(&file, owner)?;
        need(retained.0[2] == build.statement.bootstrap.bytes)?;
        contents(&file, build)?;
        need(image(&file, owner)? == retained)?;
        let mut source = Self {
            root_name: path.into(),
            owner,
            directories,
            image: file,
            retained,
            refused: false,
        };
        source.revalidate(build)?;
        Ok(source)
    }
    fn check(&self, build: &VerifiedBuild) -> Result<(), ArtifactError> {
        need(!self.refused && self.directories.len() == 4)?;
        let mut named = root(&self.root_name)?;
        for (at, (held, original)) in self.directories.iter().enumerate() {
            need(
                directory(held, self.owner)? == *original
                    && directory(&named, self.owner)? == *original,
            )?;
            if let Some(name) = DIRECTORIES.get(at) {
                named = child(&named, name, true)?;
            }
        }
        let named = child(&named, c"diagnostic-held-bootstrap-074", false)?;
        need(
            image(&named, self.owner)? == self.retained
                && image(&self.image, self.owner)? == self.retained
                && self.retained.0[2] == build.statement.bootstrap.bytes,
        )?;
        contents(&self.image, build)?;
        need(
            image(&named, self.owner)? == self.retained
                && image(&self.image, self.owner)? == self.retained,
        )?;
        Ok(())
    }
    fn revalidate(&mut self, build: &VerifiedBuild) -> Result<(), ArtifactError> {
        let result = self.check(build);
        if result.is_err() {
            self.refused = true;
        }
        result
    }
    pub(crate) fn inspect(
        &mut self,
        build: &VerifiedBuild,
        old: &EntryBindings,
    ) -> Result<InstalledBindings, ArtifactError> {
        let result = (|| {
            self.revalidate(build)?;
            let directories = std::array::from_fn(|at| &self.directories[at].0);
            let value = old.append_installed_origin(directories, &self.image)?;
            self.revalidate(build)?;
            Ok(value)
        })();
        if result.is_err() {
            self.refused = true;
        }
        result
    }
}
#[cfg(test)]
#[path = "diagnostic_bootstrap_installed_source_tests.rs"]
mod tests;
