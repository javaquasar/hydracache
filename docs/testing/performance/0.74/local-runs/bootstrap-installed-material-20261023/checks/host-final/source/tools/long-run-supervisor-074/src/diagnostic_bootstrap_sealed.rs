//! Same retained sealed object, positional reads only. No execution or FD export.
use super::*;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::os::{
    fd::AsRawFd,
    unix::fs::{FileExt, MetadataExt},
};

#[derive(PartialEq, Eq)]
pub(crate) struct Stamp {
    identity: [u64; 3],
    metadata: [i64; 9],
}
fn stamp(file: &File) -> Result<Stamp, ArtifactError> {
    let m = file.metadata()?;
    if !m.is_file() {
        return Err(ArtifactError::Contents);
    }
    Ok(Stamp {
        identity: [m.dev(), m.ino(), m.len()],
        metadata: [
            m.uid().into(),
            m.gid().into(),
            m.mode().into(),
            m.nlink() as i64,
            m.mtime(),
            m.mtime_nsec(),
            m.ctime(),
            m.ctime_nsec(),
            m.rdev() as i64,
        ],
    })
}
pub(crate) fn inspect(file: &File, build: &VerifiedBuild) -> Result<Stamp, ArtifactError> {
    let expected = &build.statement.bootstrap;
    let fd = file.as_raw_fd();
    // SAFETY: query-only fcntls on the caller's still-owned file.
    let (flags, fdflags, seals) = unsafe {
        (
            libc::fcntl(fd, libc::F_GETFL),
            libc::fcntl(fd, libc::F_GETFD),
            libc::fcntl(fd, libc::F_GET_SEALS),
        )
    };
    if flags < 0
        || flags & (libc::O_ACCMODE | libc::O_PATH) != libc::O_RDONLY
        || fdflags < 0
        || fdflags & libc::FD_CLOEXEC == 0
        || seals != libc::F_SEAL_SEAL | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_WRITE
        || !bounded(expected, MAX_BINARY_BYTES)
    {
        return Err(ArtifactError::Contents);
    }
    let opening = stamp(file)?;
    if opening.identity[2] != expected.bytes {
        return Err(ArtifactError::Contents);
    }
    let mut header = [0; 64];
    file.read_exact_at(&mut header, 0)?;
    check_elf(&header, expected.bytes)?;
    let mut offset = 0;
    let mut hash = Sha256::new();
    let mut buffer = [0; 65536];
    while offset < expected.bytes {
        let limit = (expected.bytes - offset).min(buffer.len() as u64) as usize;
        let n = match file.read_at(&mut buffer[..limit], offset) {
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            result => result?,
        };
        if n == 0 {
            return Err(ArtifactError::Contents);
        }
        hash.update(&buffer[..n]);
        offset += n as u64;
    }
    if crate::hex(&hash.finalize()) != expected.sha256 || stamp(file)? != opening {
        return Err(ArtifactError::Contents);
    }
    Ok(opening)
}
