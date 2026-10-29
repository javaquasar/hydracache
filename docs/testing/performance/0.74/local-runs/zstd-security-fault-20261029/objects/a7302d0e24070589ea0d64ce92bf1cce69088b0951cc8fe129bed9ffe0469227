//! Opt-in unsigned copy preparation, NOT authenticated original prebuilt custody.
//! Parent ownership and expected bytes are local assertions. No FD/exec authority.
use crate::diagnostic_artifacts::ArtifactDigest;
use crate::diagnostic_signer_build::MAX_BINARY_BYTES;
use sha2::{Digest, Sha256};
use std::ffi::CString;
use std::fs::{self, File, Metadata};
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd};
use std::os::unix::fs::{FileExt, MetadataExt, PermissionsExt};

const NAME: &str = "diagnostic_controller";
fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "asserted unsigned prebuilt copy refused",
    )
}
#[derive(Clone, PartialEq, Eq)]
struct Stamp {
    identity: (u64, u64),
    owner: (u32, u32),
    mode: u32,
    links: u64,
    bytes: u64,
    modified: (i64, i64),
    changed: (i64, i64),
}
impl Stamp {
    fn of(m: Metadata) -> Self {
        Self {
            identity: (m.dev(), m.ino()),
            owner: (m.uid(), m.gid()),
            mode: m.mode(),
            links: m.nlink(),
            bytes: m.len(),
            modified: (m.mtime(), m.mtime_nsec()),
            changed: (m.ctime(), m.ctime_nsec()),
        }
    }
}
fn readable(file: &File) -> io::Result<Metadata> {
    // SAFETY: flag queries on a borrowed live descriptor, no mutation/transfer.
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
    file.metadata()
}
fn parent_stamp(parent: &File) -> io::Result<Stamp> {
    let m = readable(parent)?;
    // SAFETY: fixed effective-ID queries, not host account/namespace enrollment.
    let owner = unsafe { (libc::geteuid(), libc::getegid()) };
    if owner.0 == 0 || !m.is_dir() || (m.uid(), m.gid()) != owner || m.mode() & 0o7777 != 0o700 {
        return Err(invalid());
    }
    Ok(Stamp::of(m))
}
fn child(parent: &File, name: &str, flags: i32, mode: u32) -> io::Result<File> {
    let name = CString::new(name).map_err(|_| invalid())?;
    // SAFETY: owned directory, validated single/static component, exclusive creation
    // when requested; every returned descriptor is owned exactly once.
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            flags | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
            mode,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: one new descriptor from successful openat.
    Ok(unsafe { File::from_raw_fd(fd) })
}
struct Positional<'a> {
    file: &'a File,
    offset: u64,
}
impl Read for Positional<'_> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let n = self.file.read_at(out, self.offset)?;
        self.offset += n as u64;
        Ok(n)
    }
}
fn copy_exact(
    input: &mut impl Read,
    output: &mut impl Write,
    expected: &ArtifactDigest,
) -> io::Result<()> {
    let mut hash = Sha256::new();
    let mut bytes = 0;
    let mut buffer = [0u8; 65536];
    loop {
        let budget = (expected.bytes + 1 - bytes).min(buffer.len() as u64) as usize;
        let count = input.read(&mut buffer[..budget])?;
        if count == 0 {
            break;
        }
        bytes += count as u64;
        if bytes > expected.bytes {
            return Err(invalid());
        }
        output.write_all(&buffer[..count])?;
        hash.update(&buffer[..count]);
    }
    if bytes != expected.bytes || crate::hex(&hash.finalize()) != expected.sha256 {
        return Err(invalid());
    }
    Ok(())
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Cut {
    Directory,
    Created,
    Copied,
    Synced,
    Mode,
    Closed,
    Sealed,
}

fn close_owned(file: File, close: impl FnOnce(i32) -> io::Result<()>) -> io::Result<()> {
    close(file.into_raw_fd())
}
fn linux_close(fd: i32) -> io::Result<()> {
    // SAFETY: caller consumes one uniquely owned descriptor; never retry on Linux.
    if unsafe { libc::close(fd) } == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Copy to a new inode only. Never grants execution, signing or all-writer custody.
/// The caller supplies original read-only CLOEXEC descriptors and an asserted digest.
/// Existing or failed attempt names are never repaired, overwritten or resumed.
pub fn prepare_asserted_copy(
    source: &File,
    parent: &File,
    attempt: &str,
    expected: ArtifactDigest,
) -> io::Result<AssertedPrebuiltCopy> {
    prepare(source, parent, attempt, expected, &mut |_| Ok(()))
}
fn prepare(
    source: &File,
    parent: &File,
    attempt: &str,
    expected: ArtifactDigest,
    cut: &mut impl FnMut(Cut) -> io::Result<()>,
) -> io::Result<AssertedPrebuiltCopy> {
    let nonce = attempt.strip_prefix("attempt-").ok_or_else(invalid)?;
    if nonce.len() != 32
        || !nonce
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        || !crate::is_hash(&expected.sha256)
        || expected.bytes == 0
        || expected.bytes > MAX_BINARY_BYTES
    {
        return Err(invalid());
    }
    let source_metadata = readable(source)?;
    if !source_metadata.is_file()
        || source_metadata.len() != expected.bytes
        || source_metadata.nlink() == 0
    {
        return Err(invalid());
    }
    let original = Stamp::of(source_metadata);
    let parent_original = parent_stamp(parent)?;
    let parent = parent.try_clone()?;
    let attempt_c = CString::new(attempt).map_err(|_| invalid())?;
    // SAFETY: validated absent single component under retained caller directory.
    if unsafe { libc::mkdirat(parent.as_raw_fd(), attempt_c.as_ptr(), 0o700) } != 0 {
        return Err(io::Error::last_os_error());
    }
    parent.sync_all()?;
    cut(Cut::Directory)?;
    let directory = child(&parent, attempt, libc::O_RDONLY | libc::O_DIRECTORY, 0)?;
    let created = parent_stamp(&directory)?;
    let mut writer = child(
        &directory,
        NAME,
        libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL,
        0o600,
    )?;
    let m = writer.metadata()?;
    if !m.is_file()
        || m.nlink() != 1
        || m.len() != 0
        || m.mode() & 0o7777 != 0o600
        || (m.uid(), m.gid()) != created.owner
        || (m.dev(), m.ino()) == original.identity
    {
        return Err(invalid());
    }
    cut(Cut::Created)?;
    copy_exact(
        &mut Positional {
            file: source,
            offset: 0,
        },
        &mut writer,
        &expected,
    )?;
    cut(Cut::Copied)?;
    if Stamp::of(source.metadata()?) != original {
        return Err(invalid());
    }
    // Inode timestamps can stay equal across a fast same-size write. Metadata
    // alone is not a content check; rehash the original source without a delay.
    copy_exact(
        &mut Positional {
            file: source,
            offset: 0,
        },
        &mut io::sink(),
        &expected,
    )?;
    if Stamp::of(source.metadata()?) != original {
        return Err(invalid());
    }
    writer.sync_all()?;
    cut(Cut::Synced)?;
    writer.set_permissions(fs::Permissions::from_mode(0o555))?;
    writer.sync_all()?;
    let written = Stamp::of(writer.metadata()?);
    cut(Cut::Mode)?;
    // No duplicate/export of the owned writer. Linux releases it even on close
    // error; never retry and risk closing a descriptor reused by another thread.
    close_owned(writer, linux_close)?;
    cut(Cut::Closed)?;
    let file = child(&directory, NAME, libc::O_RDONLY, 0)?;
    if Stamp::of(readable(&file)?) != written {
        return Err(invalid());
    }
    directory.set_permissions(fs::Permissions::from_mode(0o555))?;
    directory.sync_all()?;
    parent.sync_all()?;
    cut(Cut::Sealed)?;
    let directory_stamp = Stamp::of(directory.metadata()?);
    if directory_stamp.identity != created.identity || directory_stamp.owner != created.owner {
        return Err(invalid());
    }
    let mut copy = AssertedPrebuiltCopy {
        parent,
        parent_original,
        attempt: attempt.into(),
        directory,
        directory_stamp,
        file,
        file_stamp: written,
        expected,
        refused: false,
    };
    copy.check_original()?;
    Ok(copy)
}

/// Read-only inventory of the new copy, not the original Cargo inode or custody.
/// An untrusted same-UID writer can survive chmod; revalidation is not revocation.
///
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_prebuilt_copy::AssertedPrebuiltCopy;
/// fn duplicate(copy: AssertedPrebuiltCopy) { let _ = copy.clone(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_prebuilt_copy::AssertedPrebuiltCopy;
/// use std::os::fd::AsRawFd;
/// fn export(copy: AssertedPrebuiltCopy) { let _ = copy.as_raw_fd(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::{diagnostic_prebuilt_copy::AssertedPrebuiltCopy, diagnostic_signer_bundle::AssertedSignerBundle};
/// fn admit(copy: AssertedPrebuiltCopy) -> AssertedSignerBundle { copy.into() }
/// ```
pub struct AssertedPrebuiltCopy {
    parent: File,
    parent_original: Stamp,
    attempt: String,
    directory: File,
    directory_stamp: Stamp,
    file: File,
    file_stamp: Stamp,
    expected: ArtifactDigest,
    refused: bool,
}
impl AssertedPrebuiltCopy {
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
    fn names(&self) -> io::Result<()> {
        let parent = parent_stamp(&self.parent)?;
        // Different attempt creations may legitimately update parent timestamps.
        if parent.identity != self.parent_original.identity
            || parent.owner != self.parent_original.owner
        {
            return Err(invalid());
        }
        let named = child(
            &self.parent,
            &self.attempt,
            libc::O_RDONLY | libc::O_DIRECTORY,
            0,
        )?;
        if Stamp::of(readable(&named)?) != self.directory_stamp
            || Stamp::of(readable(&self.directory)?) != self.directory_stamp
            || self.directory_stamp.mode & 0o7777 != 0o555
        {
            return Err(invalid());
        }
        let names = fs::read_dir(format!("/proc/self/fd/{}", self.directory.as_raw_fd()))?
            .take(2)
            .map(|e| e.map(|e| e.file_name()))
            .collect::<io::Result<Vec<_>>>()?;
        if names != [std::ffi::OsString::from(NAME)] {
            return Err(invalid());
        }
        let named_file = child(&self.directory, NAME, libc::O_RDONLY, 0)?;
        if Stamp::of(readable(&named_file)?) != self.file_stamp
            || Stamp::of(readable(&self.file)?) != self.file_stamp
            || self.file_stamp.mode & 0o7777 != 0o555
            || self.file_stamp.links != 1
            || self.file_stamp.owner != self.parent_original.owner
            || self.file_stamp.bytes != self.expected.bytes
        {
            return Err(invalid());
        }
        Ok(())
    }
    fn check(&self) -> io::Result<()> {
        self.names()?;
        copy_exact(
            &mut Positional {
                file: &self.file,
                offset: 0,
            },
            &mut io::sink(),
            &self.expected,
        )?;
        self.names()
    }
}

#[cfg(test)]
#[path = "diagnostic_prebuilt_copy_tests.rs"]
mod tests;
