//! Owned immutable role content, not an original owner or live epoch capability.
//! No path is opened and no descriptor, execution or release API is exported.
use super::*;
use sha2::{Digest, Sha256};
use std::fs::{File, Metadata};
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::fs::{FileExt, MetadataExt};

#[path = "diagnostic_held_role_provenance.rs"]
mod provenance;

#[path = "diagnostic_held_role_projection.rs"]
mod projection;

const SEALS: i32 = libc::F_SEAL_SEAL | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_WRITE;

#[derive(PartialEq, Eq)]
struct Stamp {
    identity: [u64; 2],
    length: u64,
    owner: [u32; 3],
    links: u64,
    times: [i64; 4],
}
impl From<Metadata> for Stamp {
    fn from(m: Metadata) -> Self {
        Self {
            identity: [m.dev(), m.ino()],
            length: m.len(),
            owner: [m.uid(), m.gid(), m.mode()],
            links: m.nlink(),
            times: [m.mtime(), m.mtime_nsec(), m.ctime(), m.ctime_nsec()],
        }
    }
}
struct Role {
    file: File,
    stamp: Stamp,
}
fn inspect(file: &File, expected: &ArtifactDigest) -> Result<Stamp, ArtifactError> {
    let fd = file.as_raw_fd();
    // SAFETY: query-only fcntls on a still-owned descriptor, no variadic argument.
    let (flags, fd_flags, seals) = unsafe {
        (
            libc::fcntl(fd, libc::F_GETFL),
            libc::fcntl(fd, libc::F_GETFD),
            libc::fcntl(fd, libc::F_GET_SEALS),
        )
    };
    let metadata = file.metadata()?;
    if flags < 0
        || flags & (libc::O_ACCMODE | libc::O_PATH) != libc::O_RDONLY
        || fd_flags < 0
        || fd_flags & libc::FD_CLOEXEC == 0
        || seals != SEALS
        || !metadata.is_file()
        || !bounded(expected, MAX_BINARY_BYTES)
        || metadata.len() != expected.bytes
    {
        return Err(ArtifactError::Contents);
    }
    let stamp = Stamp::from(metadata);
    let mut header = [0; 64];
    file.read_exact_at(&mut header, 0)?;
    check_elf(&header, stamp.length)?;
    let mut hash = Sha256::new();
    let mut offset = 0;
    let mut buffer = [0; 65_536];
    while offset < stamp.length {
        let limit = (stamp.length - offset).min(buffer.len() as u64) as usize;
        let count = match file.read_at(&mut buffer[..limit], offset) {
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            result => result?,
        };
        if count == 0 {
            return Err(ArtifactError::Contents);
        }
        hash.update(&buffer[..count]);
        offset += count as u64;
    }
    if hex(&hash.finalize()) != expected.sha256 || Stamp::from(file.metadata()?) != stamp {
        return Err(ArtifactError::Contents);
    }
    Ok(stamp)
}

/// Retains two immutable files and the consumed checked receipt. This is not
/// installed-path, creator, running-process, original-policy or epoch proof.
/// ```compile_fail,E0599
/// use hydracache_long_run_supervisor_074::diagnostic_builder::held_build::role_files::PinnedRoles;
/// fn cannot_start(roles: PinnedRoles) {
///     roles.start();
/// }
/// ```
/// ```compile_fail,E0277
/// use hydracache_long_run_supervisor_074::diagnostic_builder::held_build::role_files::PinnedRoles;
/// use std::os::fd::IntoRawFd;
/// fn cannot_extract(roles: PinnedRoles) {
///     let _ = IntoRawFd::into_raw_fd(roles);
/// }
/// ```
pub struct PinnedRoles {
    build: VerifiedBuild,
    host: Role,
    launcher: Role,
    refused: bool,
}
impl VerifiedBuild {
    /// Consumes already-open objects. Four exact seals are mandatory even for
    /// root-owned files. Caller-provided role expectations grant no authority.
    pub fn pin_sealed_roles(
        self,
        original_host: File,
        held_launcher: File,
        root_lock: &[u8],
        build_log: &[u8],
        expected: &LaunchArtifacts,
    ) -> Result<PinnedRoles, ArtifactError> {
        self.matches_launch_artifacts(expected)?;
        if root_lock.len() as u64 > MAX_LOCK_BYTES
            || build_log.len() as u64 > MAX_LOG_BYTES
            || ArtifactDigest::of(root_lock) != self.statement.root_lock
            || ArtifactDigest::of(build_log) != self.statement.build_log
        {
            return Err(ArtifactError::Contents);
        }
        cargo_log(build_log)?;
        let host = Role {
            stamp: inspect(&original_host, &self.statement.images.original_host)?,
            file: original_host,
        };
        let launcher = Role {
            stamp: inspect(&held_launcher, &self.statement.images.held_launcher)?,
            file: held_launcher,
        };
        if host.stamp.identity == launcher.stamp.identity {
            return Err(ArtifactError::Contents);
        }
        let mut roles = PinnedRoles {
            build: self,
            host,
            launcher,
            refused: false,
        };
        roles.revalidate(expected)?;
        Ok(roles)
    }
}
impl PinnedRoles {
    /// Full content/stamp check on the same owned descriptors. Positional reads
    /// never change offsets shared with another descriptor. Refusal is sticky.
    pub fn revalidate(&mut self, expected: &LaunchArtifacts) -> Result<(), ArtifactError> {
        let result = (|| {
            if self.refused {
                return Err(ArtifactError::Contents);
            }
            self.build.matches_launch_artifacts(expected)?;
            if inspect(&self.host.file, &self.build.statement.images.original_host)?
                != self.host.stamp
                || inspect(
                    &self.launcher.file,
                    &self.build.statement.images.held_launcher,
                )? != self.launcher.stamp
            {
                return Err(ArtifactError::Contents);
            }
            Ok(())
        })();
        if result.is_err() {
            self.refused = true;
        }
        result
    }
}
