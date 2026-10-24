//! Bounded pipe-to-file writer. Only a nonroot fixture entry is enrolled.
//! Production start, root ancestry, watchdog and cgroup revocation are separate.
use crate::diagnostic_receipts::STREAM_BYTES;
use sha2::{Digest, Sha256};
use std::ffi::CString;
use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};
use thiserror::Error;

const ATTEMPT: &str = "collector-attempt-v1";
const NAMES: [&str; 2] = ["collector-stdout.bin", "collector-stderr.bin"];
const QUANTUM: usize = 8192;

#[derive(Debug, Error)]
pub enum CollectorError {
    #[error("collector original object or fixture path refused")]
    Invalid,
    #[error("collector attempt already exists; retained without replay")]
    Conflict,
    #[error("collector stream exceeds the fixed byte ceiling; bounded prefix retained")]
    Overflow,
    #[error("collector streams have not both reached kernel read EOF")]
    NotDrained,
    #[error("collector is already refused")]
    Refused,
    #[error("collector injected local test failure at {0}")]
    Injected(&'static str),
    #[error("collector I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    DirectoryCreated,
    Directory,
    StdoutCreated,
    StdoutSynced,
    StderrCreated,
    StderrSynced,
    Write,
    StdoutSync,
    StderrSync,
    Readonly,
    ReadonlySynced,
    DirectorySync,
    Close,
    Acknowledge,
}
fn checkpoint(fault: Option<Stage>, at: Stage) -> Result<(), CollectorError> {
    if fault == Some(at) {
        return Err(CollectorError::Injected(match at {
            Stage::DirectoryCreated => "directory-created",
            Stage::Directory => "directory",
            Stage::StdoutCreated => "stdout-created",
            Stage::StdoutSynced => "stdout-synced",
            Stage::StderrCreated => "stderr-created",
            Stage::StderrSynced => "stderr-synced",
            Stage::Write => "write",
            Stage::StdoutSync => "stdout-sync",
            Stage::StderrSync => "stderr-sync",
            Stage::Readonly => "readonly",
            Stage::ReadonlySynced => "readonly-synced",
            Stage::DirectorySync => "directory-sync",
            Stage::Close => "close",
            Stage::Acknowledge => "acknowledge",
        }));
    }
    #[cfg(test)]
    crash_tests::pause_at(at);
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
struct Stamp {
    dev: u64,
    ino: u64,
    uid: u32,
    gid: u32,
    mode: u32,
    links: u64,
    len: u64,
    mtime: (i64, i64),
    ctime: (i64, i64),
}
impl Stamp {
    fn of(m: &Metadata) -> Self {
        Self {
            dev: m.dev(),
            ino: m.ino(),
            uid: m.uid(),
            gid: m.gid(),
            mode: m.mode(),
            links: m.nlink(),
            len: m.len(),
            mtime: (m.mtime(), m.mtime_nsec()),
            ctime: (m.ctime(), m.ctime_nsec()),
        }
    }
    fn same_directory(&self, m: &Metadata) -> bool {
        m.is_dir()
            && self.dev == m.dev()
            && self.ino == m.ino()
            && self.uid == m.uid()
            && self.gid == m.gid()
            && self.mode == m.mode()
    }
}
struct Stream {
    reader: File,
    sink: File,
    stamp: Stamp,
    digest: Sha256,
    bytes: u64,
    eof: bool,
}
struct Collector {
    path: PathBuf,
    parent: File,
    parent_stamp: Stamp,
    directory: File,
    directory_stamp: Stamp,
    streams: [Stream; 2],
    limit: u64,
    refused: bool,
    fault: Option<Stage>,
}
struct Writers([File; 2]);
struct DrainedStreams {
    bytes: [u64; 2],
    hashes: [String; 2],
}

/// Point-in-time stream data, not process completion or resource authority.
#[derive(Debug, Clone, Copy)]
pub struct DrainProgress {
    bytes: [u64; 2],
    eof: [bool; 2],
}
impl DrainProgress {
    pub fn both_eof(&self) -> bool {
        self.eof == [true, true]
    }
    pub fn bytes(&self) -> [u64; 2] {
        self.bytes
    }
}

fn open_at(parent: &File, name: &str, flags: i32, mode: u32) -> Result<File, CollectorError> {
    let name = CString::new(name).map_err(|_| CollectorError::Invalid)?;
    // SAFETY: retained parent and valid CString; resulting FD is newly owned.
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            flags | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            mode as libc::mode_t,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}
fn open_fixture_root(path: &Path) -> Result<File, CollectorError> {
    let uid = unsafe { libc::geteuid() };
    let gid = unsafe { libc::getegid() };
    if uid == 0
        || gid == 0
        || !path.is_absolute()
        || !(path.starts_with("/tmp") || path.starts_with("/var/tmp"))
        || path.starts_with("/var/lib/hydracache-performance")
        || path.starts_with("/opt/hydracache-performance")
        || path.as_os_str().as_bytes().windows(2).any(|v| v == b"//")
        || path.as_os_str().as_bytes().ends_with(b"/")
        || path
            .as_os_str()
            .as_bytes()
            .split(|b| *b == b'/')
            .any(|v| v == b"." || v == b"..")
        || fs::canonicalize(path)? != path
    {
        return Err(CollectorError::Invalid);
    }
    let mut directory = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open("/")?;
    for component in path.components() {
        match component {
            Component::RootDir => (),
            Component::Normal(name) => {
                directory = open_at(
                    &directory,
                    name.to_str().ok_or(CollectorError::Invalid)?,
                    libc::O_RDONLY | libc::O_DIRECTORY,
                    0,
                )?
            }
            _ => return Err(CollectorError::Invalid),
        }
    }
    let m = directory.metadata()?;
    if !m.is_dir() || m.uid() != uid || m.gid() != gid || m.mode() & 0o7777 != 0o700 {
        return Err(CollectorError::Invalid);
    }
    Ok(directory)
}
fn pipe() -> Result<(File, File), CollectorError> {
    let mut fds = [-1; 2];
    // SAFETY: initialized array with space for both newly owned pipe descriptors.
    if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let reader = unsafe { File::from_raw_fd(fds[0]) };
    let writer = unsafe { File::from_raw_fd(fds[1]) };
    // Only the collector reader becomes nonblocking. Worker writes backpressure.
    if unsafe { libc::fcntl(reader.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok((reader, writer))
}
fn regular(m: &Metadata, uid: u32, gid: u32, len: u64, mode: u32) -> Result<(), CollectorError> {
    if !m.is_file()
        || m.uid() != uid
        || m.gid() != gid
        || m.nlink() != 1
        || m.mode() & 0o7777 != mode
        || m.len() != len
    {
        return Err(CollectorError::Invalid);
    }
    Ok(())
}
fn prepare_fixture(
    path: &Path,
    limit: u64,
    fault: Option<Stage>,
) -> Result<(Collector, Writers), CollectorError> {
    if limit == 0 || limit > STREAM_BYTES as u64 {
        return Err(CollectorError::Invalid);
    }
    let parent = open_fixture_root(path)?;
    let parent_stamp = Stamp::of(&parent.metadata()?);
    let name = CString::new(ATTEMPT).unwrap();
    if unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) } != 0 {
        let error = std::io::Error::last_os_error();
        return Err(if error.kind() == std::io::ErrorKind::AlreadyExists {
            CollectorError::Conflict
        } else {
            error.into()
        });
    }
    checkpoint(fault, Stage::DirectoryCreated)?;
    let directory = open_at(&parent, ATTEMPT, libc::O_RDONLY | libc::O_DIRECTORY, 0)?;
    let m = directory.metadata()?;
    if !m.is_dir()
        || m.uid() != parent_stamp.uid
        || m.gid() != parent_stamp.gid
        || m.mode() & 0o7777 != 0o700
    {
        return Err(CollectorError::Invalid);
    }
    let directory_stamp = Stamp::of(&m);
    directory.sync_all()?;
    parent.sync_all()?;
    checkpoint(fault, Stage::Directory)?;
    let mut streams = Vec::with_capacity(2);
    let mut writers = Vec::with_capacity(2);
    for (index, name) in NAMES.iter().enumerate() {
        let (reader, writer) = pipe()?;
        let sink = open_at(
            &directory,
            name,
            libc::O_RDWR | libc::O_CREAT | libc::O_EXCL,
            0o600,
        )?;
        regular(
            &sink.metadata()?,
            parent_stamp.uid,
            parent_stamp.gid,
            0,
            0o600,
        )?;
        checkpoint(
            fault,
            if index == 0 {
                Stage::StdoutCreated
            } else {
                Stage::StderrCreated
            },
        )?;
        sink.sync_all()?;
        directory.sync_all()?;
        let stamp = Stamp::of(&sink.metadata()?);
        streams.push(Stream {
            reader,
            sink,
            stamp,
            digest: Sha256::new(),
            bytes: 0,
            eof: false,
        });
        writers.push(writer);
        checkpoint(
            fault,
            if index == 0 {
                Stage::StdoutSynced
            } else {
                Stage::StderrSynced
            },
        )?;
    }
    let collector = Collector {
        path: path.into(),
        parent,
        parent_stamp,
        directory,
        directory_stamp,
        streams: streams.try_into().map_err(|_| CollectorError::Invalid)?,
        limit,
        refused: false,
        fault,
    };
    collector.revalidate()?;
    Ok((
        collector,
        Writers(writers.try_into().map_err(|_| CollectorError::Invalid)?),
    ))
}
fn pipe_ready(reader: &File) -> Result<bool, CollectorError> {
    let mut p = libc::pollfd {
        fd: reader.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    let ready = unsafe { libc::poll(&mut p, 1, 0) };
    if ready < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    classify_poll(ready, p.revents)
}
fn classify_poll(ready: i32, events: i16) -> Result<bool, CollectorError> {
    if ready == 0 && events == 0 {
        return Ok(false);
    }
    if ready == 1
        && (events == libc::POLLIN
            || events == libc::POLLHUP
            || events == libc::POLLIN | libc::POLLHUP)
    {
        return Ok(true);
    }
    Err(CollectorError::Invalid)
}
impl Collector {
    fn revalidate(&self) -> Result<(), CollectorError> {
        self.revalidate_mode(0o600)
    }
    fn revalidate_mode(&self, mode: u32) -> Result<(), CollectorError> {
        let named = open_fixture_root(&self.path)?;
        let directory = open_at(&self.parent, ATTEMPT, libc::O_RDONLY | libc::O_DIRECTORY, 0)?;
        if !self.parent_stamp.same_directory(&named.metadata()?)
            || !self.parent_stamp.same_directory(&self.parent.metadata()?)
            || !self.directory_stamp.same_directory(&directory.metadata()?)
            || !self
                .directory_stamp
                .same_directory(&self.directory.metadata()?)
        {
            return Err(CollectorError::Invalid);
        }
        for (index, stream) in self.streams.iter().enumerate() {
            let named = open_at(&self.directory, NAMES[index], libc::O_RDONLY, 0)?;
            regular(
                &stream.sink.metadata()?,
                self.parent_stamp.uid,
                self.parent_stamp.gid,
                stream.bytes,
                mode,
            )?;
            if Stamp::of(&stream.sink.metadata()?) != stream.stamp
                || Stamp::of(&named.metadata()?) != stream.stamp
            {
                return Err(CollectorError::Invalid);
            }
        }
        let mut inventory = fs::read_dir(format!("/proc/self/fd/{}", self.directory.as_raw_fd()))?;
        let mut names = std::collections::BTreeSet::new();
        for entry in inventory.by_ref().take(3) {
            names.insert(entry?.file_name());
        }
        if names != NAMES.map(std::ffi::OsString::from).into_iter().collect()
            || inventory.next().is_some()
        {
            return Err(CollectorError::Invalid);
        }
        Ok(())
    }
    fn progress(&self) -> DrainProgress {
        DrainProgress {
            bytes: self.streams.each_ref().map(|s| s.bytes),
            eof: self.streams.each_ref().map(|s| s.eof),
        }
    }
    fn pump(&mut self) -> Result<DrainProgress, CollectorError> {
        if self.refused {
            return Err(CollectorError::Refused);
        }
        let result = self.pump_inner();
        if result.is_err() {
            self.refused = true;
        }
        result
    }
    fn pump_inner(&mut self) -> Result<DrainProgress, CollectorError> {
        self.revalidate()?;
        let mut buffer = [0; QUANTUM];
        for stream in &mut self.streams {
            if stream.eof || !pipe_ready(&stream.reader)? {
                continue;
            }
            let count = match stream.reader.read(&mut buffer) {
                Ok(count) => count,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                Err(e) => return Err(e.into()),
            };
            if count == 0 {
                stream.eof = true;
                continue;
            }
            let retained = count.min((self.limit - stream.bytes) as usize);
            stream.sink.write_all(&buffer[..retained])?;
            stream.digest.update(&buffer[..retained]);
            stream.bytes += retained as u64;
            stream.stamp = Stamp::of(&stream.sink.metadata()?);
            checkpoint(self.fault, Stage::Write)?;
            if retained != count {
                return Err(CollectorError::Overflow);
            }
        }
        self.revalidate()?;
        Ok(self.progress())
    }
    fn finish(mut self) -> Result<DrainedStreams, CollectorError> {
        if self.refused {
            return Err(CollectorError::Refused);
        }
        if !self.progress().both_eof() {
            return Err(CollectorError::NotDrained);
        }
        self.revalidate()?;
        let bytes = self.streams.each_ref().map(|s| s.bytes);
        for (index, stream) in self.streams.iter().enumerate() {
            stream.sink.sync_all()?;
            checkpoint(
                self.fault,
                if index == 0 {
                    Stage::StdoutSync
                } else {
                    Stage::StderrSync
                },
            )?;
        }
        self.revalidate()?;
        for stream in &mut self.streams {
            stream
                .sink
                .set_permissions(fs::Permissions::from_mode(0o400))?;
            checkpoint(self.fault, Stage::Readonly)?;
            stream.sink.sync_all()?;
            stream.stamp = Stamp::of(&stream.sink.metadata()?);
            checkpoint(self.fault, Stage::ReadonlySynced)?;
        }
        self.revalidate_mode(0o400)?;
        // Reconcile retained bytes even on filesystems with coarse timestamps.
        // No file-sized allocation and no reopening a pathname as byte authority.
        let mut buffer = [0; QUANTUM];
        for stream in &mut self.streams {
            stream.sink.seek(SeekFrom::Start(0))?;
            let mut hash = Sha256::new();
            let mut length = 0u64;
            loop {
                let count = stream.sink.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                length += count as u64;
                if length > stream.bytes {
                    return Err(CollectorError::Invalid);
                }
                hash.update(&buffer[..count]);
            }
            if length != stream.bytes || hash.finalize() != stream.digest.clone().finalize() {
                return Err(CollectorError::Invalid);
            }
        }
        self.revalidate_mode(0o400)?;
        self.directory.sync_all()?;
        checkpoint(self.fault, Stage::DirectorySync)?;
        self.revalidate_mode(0o400)?;
        let hashes = self
            .streams
            .map(|stream| -> Result<String, CollectorError> {
                close(stream.sink)?;
                close(stream.reader)?;
                checkpoint(self.fault, Stage::Close)?;
                Ok(crate::hex(&stream.digest.finalize()))
            });
        let [stdout, stderr] = hashes;
        let hashes = [stdout?, stderr?];
        close(self.directory)?;
        close(self.parent)?;
        checkpoint(self.fault, Stage::Acknowledge)?;
        Ok(DrainedStreams { bytes, hashes })
    }
}
fn close(file: File) -> Result<(), CollectorError> {
    // Linux closes the FD even on EINTR; never retry against a reused FD number.
    let fd = file.into_raw_fd();
    if unsafe { libc::close(fd) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(())
}

/// Nonroot temporary fixtures only. No root producer or authority conversion.
pub mod fixture {
    use super::*;
    pub struct FixtureCollector(Collector);
    pub struct FixtureWriters(Writers);
    /// Consumed local drain data, never authenticated completion or cleanup.
    ///
    /// ```compile_fail
    /// use hydracache_long_run_supervisor_074::diagnostic_collector::fixture::FixtureDrain;
    /// fn forge() { let _ = FixtureDrain { bytes: [0, 0] }; }
    /// ```
    /// ```compile_fail
    /// use hydracache_long_run_supervisor_074::diagnostic_collector::fixture::FixtureDrain;
    /// fn promote(d: FixtureDrain) { d.complete_cell(); }
    /// ```
    pub struct FixtureDrain(DrainedStreams);
    pub fn prepare(root: &Path) -> Result<(FixtureCollector, FixtureWriters), CollectorError> {
        let (collector, writers) = prepare_fixture(root, STREAM_BYTES as u64, None)?;
        Ok((FixtureCollector(collector), FixtureWriters(writers)))
    }
    impl FixtureWriters {
        /// Transfers only pipe write ends, never an evidence-file descriptor.
        pub fn into_stdio(self) -> (std::process::Stdio, std::process::Stdio) {
            let [stdout, stderr] = self.0 .0;
            (stdout.into(), stderr.into())
        }
    }
    impl FixtureCollector {
        pub fn pump(&mut self) -> Result<DrainProgress, CollectorError> {
            self.0.pump()
        }
        /// Consumes the sole collector even on failure. Retained files cannot
        /// recreate this owner, a drain acknowledgement or a retry capability.
        pub fn finish(self) -> Result<FixtureDrain, CollectorError> {
            self.0.finish().map(FixtureDrain)
        }
    }
    impl FixtureDrain {
        pub fn stdout_bytes(&self) -> u64 {
            self.0.bytes[0]
        }
        pub fn stderr_bytes(&self) -> u64 {
            self.0.bytes[1]
        }
        pub fn stdout_sha256(&self) -> &str {
            &self.0.hashes[0]
        }
        pub fn stderr_sha256(&self) -> &str {
            &self.0.hashes[1]
        }
    }
}

#[cfg(test)]
#[path = "diagnostic_collector_crash_tests.rs"]
mod crash_tests;
#[cfg(test)]
#[path = "diagnostic_collector_tests.rs"]
mod tests;
