//! Linux temporary-fixture spool only. No live route, process or host release.
use crate::canonical_json;
use crate::diagnostic_artifacts::{ArtifactDigest, ArtifactError, VerifiedBuild};
use crate::diagnostic_lease::CellIntent;
use crate::diagnostic_receipts::{self, Packet, ReceiptError, TerminalSummary, STREAM_BYTES};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::ffi::{CString, OsString};
use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};
use thiserror::Error;

const MANIFEST_BYTES: usize = 65_536;
const SOURCE_FILES: [&str; 2] = ["stdout.json", "stderr.log"];
const SEALED_FILES: [&str; 3] = ["stdout.prefix", "stderr.prefix", "manifest.json"];

#[derive(Debug, Error)]
pub enum SpoolError {
    #[error("fixture spool metadata, identity, bytes or schema refused")]
    Invalid,
    #[error("pending or conflicting published generation retained; no overwrite/recovery")]
    Conflict,
    #[error("logical publication fault after {0:?}; files retained")]
    Injected(FaultPoint),
    #[error("spool I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("spool manifest failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("spool build binding failed: {0}")]
    Artifact(#[from] ArtifactError),
    #[error("spool byte reconciliation failed: {0}")]
    Receipt(#[from] ReceiptError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultPoint {
    AfterPending,
    AfterStdout,
    AfterStderr,
    AfterManifest,
    AfterReadonly,
    AfterRename,
}
fn fault(requested: Option<FaultPoint>, at: FaultPoint) -> Result<(), SpoolError> {
    if requested == Some(at) {
        Err(SpoolError::Injected(at))
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileStamp {
    dev: u64,
    ino: u64,
    uid: u32,
    gid: u32,
    mode: u32,
    len: u64,
    links: u64,
    mtime: i64,
    mtime_ns: i64,
    ctime: i64,
    ctime_ns: i64,
}
impl From<Metadata> for FileStamp {
    fn from(m: Metadata) -> Self {
        Self {
            dev: m.dev(),
            ino: m.ino(),
            uid: m.uid(),
            gid: m.gid(),
            mode: m.mode(),
            len: m.len(),
            links: m.nlink(),
            mtime: m.mtime(),
            mtime_ns: m.mtime_nsec(),
            ctime: m.ctime(),
            ctime_ns: m.ctime_nsec(),
        }
    }
}
impl FileStamp {
    fn same_directory(&self, m: &Metadata) -> bool {
        m.is_dir()
            && self.dev == m.dev()
            && self.ino == m.ino()
            && self.uid == m.uid()
            && self.gid == m.gid()
            && self.mode & 0o7777 == m.mode() & 0o7777
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StreamReceipt {
    pub source: FileStamp,
    pub observed_bytes: u64,
    pub prefix: ArtifactDigest,
    pub complete: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SpoolDecision {
    Complete,
    OverflowRetained,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpoolManifest {
    pub schema_version: u32,
    pub contract_id: String,
    pub fixture_only: bool,
    pub intent: CellIntent,
    pub build_receipt_sha256: String,
    pub terminal: TerminalSummary,
    pub stdout: StreamReceipt,
    pub stderr: StreamReceipt,
    pub decision: SpoolDecision,
    pub packet: Option<Packet>,
    pub promotable: bool,
    pub admission_allowed: bool,
    pub product_numeric_claims_allowed: bool,
    pub live_cgroup_proven: bool,
}
fn manifest_bytes(manifest: &SpoolManifest) -> Result<Vec<u8>, SpoolError> {
    let mut raw = canonical_json(manifest)?;
    raw.push(b'\n');
    if raw.len() > MANIFEST_BYTES {
        return Err(SpoolError::Invalid);
    }
    Ok(raw)
}

struct PinnedStream {
    file: File,
    stamp: FileStamp,
    prefix: Vec<u8>,
}
pub struct SpoolSnapshot {
    path: PathBuf,
    uid: u32,
    gid: u32,
    directory: File,
    stamp: FileStamp,
    streams: [PinnedStream; 2],
}
/// Returned only after software sync barriers, not physical power-loss proof.
pub struct PublishedFixture {
    path: PathBuf,
    manifest: SpoolManifest,
    digest: ArtifactDigest,
    replay: bool,
}
impl PublishedFixture {
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn manifest(&self) -> &SpoolManifest {
        &self.manifest
    }
    pub fn manifest_digest(&self) -> &ArtifactDigest {
        &self.digest
    }
    pub fn was_replay(&self) -> bool {
        self.replay
    }
}

/// Explicit fixture entry point; production state/install trees are forbidden.
pub fn inspect_fixture_spool(path: &Path, uid: u32, gid: u32) -> Result<SpoolSnapshot, SpoolError> {
    let directory = open_directory(path, uid, gid, 0o700)?;
    let stamp = FileStamp::from(directory.metadata()?);
    entries(&directory, &SOURCE_FILES)?;
    let mut streams = Vec::new();
    for name in SOURCE_FILES {
        let mut file = open_child(&directory, name, false)?;
        regular(&file.metadata()?, uid, gid, 0o600)?;
        let stamp = FileStamp::from(file.metadata()?);
        let prefix = read_prefix(&mut file, STREAM_BYTES)?;
        if FileStamp::from(file.metadata()?) != stamp
            || prefix.len() as u64 != stamp.len.min(STREAM_BYTES as u64)
        {
            return Err(SpoolError::Invalid);
        }
        streams.push(PinnedStream {
            file,
            stamp,
            prefix,
        });
    }
    let mut snapshot = SpoolSnapshot {
        path: path.into(),
        uid,
        gid,
        directory,
        stamp,
        streams: streams.try_into().map_err(|_| SpoolError::Invalid)?,
    };
    snapshot.revalidate()?;
    Ok(snapshot)
}

impl SpoolSnapshot {
    pub fn revalidate(&mut self) -> Result<(), SpoolError> {
        let named = open_directory(&self.path, self.uid, self.gid, 0o700)?;
        if FileStamp::from(named.metadata()?) != self.stamp
            || FileStamp::from(self.directory.metadata()?) != self.stamp
        {
            return Err(SpoolError::Invalid);
        }
        entries(&named, &SOURCE_FILES)?;
        for (name, stream) in SOURCE_FILES.iter().zip(&mut self.streams) {
            let current = open_child(&named, name, false)?;
            regular(&current.metadata()?, self.uid, self.gid, 0o600)?;
            if FileStamp::from(current.metadata()?) != stream.stamp
                || FileStamp::from(stream.file.metadata()?) != stream.stamp
                || read_prefix(&mut stream.file, STREAM_BYTES)? != stream.prefix
                || FileStamp::from(stream.file.metadata()?) != stream.stamp
            {
                return Err(SpoolError::Invalid);
            }
        }
        if FileStamp::from(named.metadata()?) != self.stamp
            || FileStamp::from(self.directory.metadata()?) != self.stamp
        {
            return Err(SpoolError::Invalid);
        }
        Ok(())
    }

    pub fn publish_fixture(
        &mut self,
        output: &Path,
        build: &VerifiedBuild,
        intent: &CellIntent,
        terminal: &TerminalSummary,
        requested_fault: Option<FaultPoint>,
    ) -> Result<PublishedFixture, SpoolError> {
        build.bind_intent(intent)?;
        self.revalidate()?;
        let stdout = stream_receipt(&self.streams[0]);
        let stderr = stream_receipt(&self.streams[1]);
        let manifest = make_manifest(
            build,
            intent,
            terminal,
            stdout,
            stderr,
            &self.streams[0].prefix,
            &self.streams[1].prefix,
        )?;
        let raw = manifest_bytes(&manifest)?;
        let digest = ArtifactDigest::of(&raw);
        let parent = open_directory(output, self.uid, self.gid, 0o700)?;
        let parent_stamp = FileStamp::from(parent.metadata()?);
        let (pending, sealed) = names(intent);
        if exists(&parent, &pending)? {
            return Err(SpoolError::Conflict);
        }
        if exists(&parent, &sealed)? {
            let checked = verify_fixture_packet(
                &output.join(&sealed),
                self.uid,
                self.gid,
                build,
                intent,
                terminal,
                &digest,
            )?;
            let directory = open_child(&parent, &sealed, true)?;
            sync_published(&directory)?;
            check_named_parent(output, &parent, &parent_stamp, self.uid, self.gid)?;
            self.revalidate()?;
            parent.sync_all()?;
            return Ok(PublishedFixture {
                path: output.join(sealed),
                manifest: checked,
                digest,
                replay: true,
            });
        }
        mkdir_child(&parent, &pending)?;
        parent.sync_all()?;
        let stage = open_child(&parent, &pending, true)?;
        check_directory(&stage.metadata()?, self.uid, self.gid, 0o700)?;
        let stage_identity = FileStamp::from(stage.metadata()?);
        stage.sync_all()?;
        fault(requested_fault, FaultPoint::AfterPending)?;
        write_child(
            &stage,
            "stdout.prefix",
            &self.streams[0].prefix,
            self.uid,
            self.gid,
        )?;
        fault(requested_fault, FaultPoint::AfterStdout)?;
        write_child(
            &stage,
            "stderr.prefix",
            &self.streams[1].prefix,
            self.uid,
            self.gid,
        )?;
        fault(requested_fault, FaultPoint::AfterStderr)?;
        write_child(&stage, "manifest.json", &raw, self.uid, self.gid)?;
        fault(requested_fault, FaultPoint::AfterManifest)?;
        entries(&stage, &SEALED_FILES)?;
        stage.set_permissions(fs::Permissions::from_mode(0o500))?;
        stage.sync_all()?;
        fault(requested_fault, FaultPoint::AfterReadonly)?;
        self.revalidate()?;
        check_named_parent(output, &parent, &parent_stamp, self.uid, self.gid)?;
        let named_stage = open_child(&parent, &pending, true)?;
        let meta = named_stage.metadata()?;
        if meta.dev() != stage_identity.dev || meta.ino() != stage_identity.ino {
            return Err(SpoolError::Invalid);
        }
        check_directory(&meta, self.uid, self.gid, 0o500)?;
        rename_no_replace(&parent, &pending, &sealed)?;
        fault(requested_fault, FaultPoint::AfterRename)?;
        parent.sync_all()?;
        let checked = verify_fixture_packet(
            &output.join(&sealed),
            self.uid,
            self.gid,
            build,
            intent,
            terminal,
            &digest,
        )?;
        check_named_parent(output, &parent, &parent_stamp, self.uid, self.gid)?;
        Ok(PublishedFixture {
            path: output.join(sealed),
            manifest: checked,
            digest,
            replay: false,
        })
    }
}

fn stream_receipt(stream: &PinnedStream) -> StreamReceipt {
    StreamReceipt {
        source: stream.stamp.clone(),
        observed_bytes: stream.stamp.len,
        prefix: ArtifactDigest::of(&stream.prefix),
        complete: stream.stamp.len <= STREAM_BYTES as u64,
    }
}
#[allow(clippy::too_many_arguments)]
fn make_manifest(
    build: &VerifiedBuild,
    intent: &CellIntent,
    terminal: &TerminalSummary,
    stdout: StreamReceipt,
    stderr: StreamReceipt,
    out: &[u8],
    err: &[u8],
) -> Result<SpoolManifest, SpoolError> {
    // Bounds/binding of caller summary also apply to overflow-only packets.
    diagnostic_receipts::packet(build, intent, terminal, b"", b"")?;
    let complete = stdout.complete && stderr.complete;
    let packet = if complete {
        Some(diagnostic_receipts::packet(
            build, intent, terminal, out, err,
        )?)
    } else {
        None
    };
    Ok(SpoolManifest {
        schema_version: 1,
        contract_id: "diagnostic-spool-local-074-v1".into(),
        fixture_only: true,
        intent: intent.clone(),
        build_receipt_sha256: build.identity().build_provenance_sha256.clone(),
        terminal: terminal.clone(),
        stdout,
        stderr,
        decision: if complete {
            SpoolDecision::Complete
        } else {
            SpoolDecision::OverflowRetained
        },
        packet,
        promotable: false,
        admission_allowed: false,
        product_numeric_claims_allowed: false,
        live_cgroup_proven: false,
    })
}

/// Read-only offline verification. expected digest MUST come from outside the
/// packet; it cannot prove actual exit, observed tail bytes or prior fsync.
#[allow(clippy::too_many_arguments)]
pub fn verify_fixture_packet(
    path: &Path,
    uid: u32,
    gid: u32,
    build: &VerifiedBuild,
    intent: &CellIntent,
    terminal: &TerminalSummary,
    expected: &ArtifactDigest,
) -> Result<SpoolManifest, SpoolError> {
    build.bind_intent(intent)?;
    if path.file_name() != Some(std::ffi::OsStr::new(&names(intent).1))
        || expected.bytes == 0
        || expected.bytes > MANIFEST_BYTES as u64
    {
        return Err(SpoolError::Invalid);
    }
    let directory = open_directory(path, uid, gid, 0o500)?;
    let stamp = FileStamp::from(directory.metadata()?);
    entries(&directory, &SEALED_FILES)?;
    let mut manifest_file = open_child(&directory, "manifest.json", false)?;
    regular(&manifest_file.metadata()?, uid, gid, 0o400)?;
    let manifest_stamp = FileStamp::from(manifest_file.metadata()?);
    let raw = read_bounded(&mut manifest_file, MANIFEST_BYTES)?;
    if ArtifactDigest::of(&raw) != *expected {
        return Err(SpoolError::Invalid);
    }
    let manifest: SpoolManifest = serde_json::from_slice(&raw)?;
    if manifest_bytes(&manifest)? != raw {
        return Err(SpoolError::Invalid);
    }
    let mut stdout = open_child(&directory, "stdout.prefix", false)?;
    let mut stderr = open_child(&directory, "stderr.prefix", false)?;
    regular(&stdout.metadata()?, uid, gid, 0o400)?;
    regular(&stderr.metadata()?, uid, gid, 0o400)?;
    let stdout_stamp = FileStamp::from(stdout.metadata()?);
    let stderr_stamp = FileStamp::from(stderr.metadata()?);
    let out = read_bounded(&mut stdout, STREAM_BYTES)?;
    let err = read_bounded(&mut stderr, STREAM_BYTES)?;
    for (receipt, prefix) in [(&manifest.stdout, &out), (&manifest.stderr, &err)] {
        if receipt.source.uid != uid
            || receipt.source.gid != gid
            || receipt.source.mode & 0o7777 != 0o600
            || receipt.source.mode & libc::S_IFMT != libc::S_IFREG
            || receipt.source.links != 1
            || receipt.observed_bytes != receipt.source.len
            || receipt.source.ino == 0
            || receipt.prefix != ArtifactDigest::of(prefix)
            || receipt.prefix.bytes != receipt.observed_bytes.min(STREAM_BYTES as u64)
            || receipt.complete != (receipt.observed_bytes <= STREAM_BYTES as u64)
        {
            return Err(SpoolError::Invalid);
        }
    }
    let recomputed = make_manifest(
        build,
        intent,
        terminal,
        manifest.stdout.clone(),
        manifest.stderr.clone(),
        &out,
        &err,
    )?;
    if manifest != recomputed {
        return Err(SpoolError::Invalid);
    }
    let named = open_directory(path, uid, gid, 0o500)?;
    if FileStamp::from(named.metadata()?) != stamp
        || FileStamp::from(directory.metadata()?) != stamp
    {
        return Err(SpoolError::Invalid);
    }
    entries(&named, &SEALED_FILES)?;
    for (name, file, original) in [
        ("manifest.json", &manifest_file, &manifest_stamp),
        ("stdout.prefix", &stdout, &stdout_stamp),
        ("stderr.prefix", &stderr, &stderr_stamp),
    ] {
        let current = open_child(&named, name, false)?;
        if FileStamp::from(current.metadata()?) != *original
            || FileStamp::from(file.metadata()?) != *original
        {
            return Err(SpoolError::Invalid);
        }
    }
    Ok(manifest)
}

fn names(intent: &CellIntent) -> (String, String) {
    let base = format!("diagnostic-{}-{}", intent.lease_id, intent.surface);
    (format!(".{base}.pending"), format!("{base}.sealed"))
}
fn read_prefix(file: &mut File, limit: usize) -> Result<Vec<u8>, SpoolError> {
    file.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::with_capacity(file.metadata()?.len().min(limit as u64) as usize);
    file.take(limit as u64).read_to_end(&mut bytes)?;
    Ok(bytes)
}
fn read_bounded(file: &mut File, limit: usize) -> Result<Vec<u8>, SpoolError> {
    let before = FileStamp::from(file.metadata()?);
    if before.len > limit as u64 {
        return Err(SpoolError::Invalid);
    }
    let raw = read_prefix(file, limit)?;
    if FileStamp::from(file.metadata()?) != before || raw.len() as u64 != before.len {
        return Err(SpoolError::Invalid);
    }
    Ok(raw)
}
fn regular(m: &Metadata, uid: u32, gid: u32, mode: u32) -> Result<(), SpoolError> {
    if !m.is_file()
        || m.uid() != uid
        || m.gid() != gid
        || m.mode() & 0o7777 != mode
        || m.nlink() != 1
    {
        return Err(SpoolError::Invalid);
    }
    Ok(())
}
fn check_directory(m: &Metadata, uid: u32, gid: u32, mode: u32) -> Result<(), SpoolError> {
    if !m.is_dir() || m.uid() != uid || m.gid() != gid || m.mode() & 0o7777 != mode {
        return Err(SpoolError::Invalid);
    }
    Ok(())
}
fn open_directory(path: &Path, uid: u32, gid: u32, mode: u32) -> Result<File, SpoolError> {
    if !path.is_absolute()
        || path.starts_with("/var/lib/hydracache-performance")
        || path.starts_with("/opt/hydracache-performance")
    {
        return Err(SpoolError::Invalid);
    }
    let mut directory = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open("/")?;
    for part in path.components() {
        match part {
            Component::RootDir => {}
            Component::Normal(name) => {
                directory = open_at(
                    &directory,
                    &CString::new(name.as_bytes()).map_err(|_| SpoolError::Invalid)?,
                    true,
                )?
            }
            _ => return Err(SpoolError::Invalid),
        }
    }
    check_directory(&directory.metadata()?, uid, gid, mode)?;
    Ok(directory)
}
fn open_child(parent: &File, name: &str, directory: bool) -> Result<File, SpoolError> {
    open_at(
        parent,
        &CString::new(name).map_err(|_| SpoolError::Invalid)?,
        directory,
    )
}
fn open_at(parent: &File, name: &CString, directory: bool) -> Result<File, SpoolError> {
    let flags = libc::O_RDONLY
        | libc::O_NOFOLLOW
        | libc::O_CLOEXEC
        | libc::O_NONBLOCK
        | if directory { libc::O_DIRECTORY } else { 0 };
    // SAFETY: live owned parent FD and NUL-terminated name; creates nothing.
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    // SAFETY: successful openat returns a new owned descriptor.
    Ok(unsafe { File::from_raw_fd(fd) })
}
fn entries(directory: &File, expected: &[&str]) -> Result<(), SpoolError> {
    let mut seen = BTreeSet::new();
    for entry in fs::read_dir(format!("/proc/self/fd/{}", directory.as_raw_fd()))? {
        seen.insert(entry?.file_name());
        if seen.len() > expected.len() {
            return Err(SpoolError::Invalid);
        }
    }
    if seen != expected.iter().map(|name| OsString::from(*name)).collect() {
        return Err(SpoolError::Invalid);
    }
    Ok(())
}
fn exists(parent: &File, name: &str) -> Result<bool, SpoolError> {
    let name = CString::new(name).map_err(|_| SpoolError::Invalid)?;
    // SAFETY: zeroed stat is writable and parent/name live for fstatat.
    let mut stat: libc::stat = unsafe { std::mem::zeroed() };
    if unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            name.as_ptr(),
            &mut stat,
            libc::AT_SYMLINK_NOFOLLOW,
        )
    } == 0
    {
        return Ok(true);
    }
    let e = std::io::Error::last_os_error();
    if e.kind() == std::io::ErrorKind::NotFound {
        Ok(false)
    } else {
        Err(e.into())
    }
}
fn mkdir_child(parent: &File, name: &str) -> Result<(), SpoolError> {
    let name = CString::new(name).map_err(|_| SpoolError::Invalid)?;
    // SAFETY: owned parent FD, fixed validated child name; exclusive mkdir.
    if unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) } < 0 {
        let e = std::io::Error::last_os_error();
        return if e.kind() == std::io::ErrorKind::AlreadyExists {
            Err(SpoolError::Conflict)
        } else {
            Err(e.into())
        };
    }
    Ok(())
}
fn write_child(
    parent: &File,
    name: &str,
    bytes: &[u8],
    uid: u32,
    gid: u32,
) -> Result<(), SpoolError> {
    let name = CString::new(name).map_err(|_| SpoolError::Invalid)?;
    // SAFETY: owned FD and fixed name; O_EXCL forbids any existing file/link.
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            0o600,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    // SAFETY: new owned fd is transferred to File exactly once.
    let mut file = unsafe { File::from_raw_fd(fd) };
    regular(&file.metadata()?, uid, gid, 0o600)?;
    for chunk in bytes.chunks(65_536) {
        file.write_all(chunk)?;
    }
    file.set_permissions(fs::Permissions::from_mode(0o400))?;
    file.sync_all()?;
    Ok(())
}
fn rename_no_replace(parent: &File, source: &str, dest: &str) -> Result<(), SpoolError> {
    let source = CString::new(source).map_err(|_| SpoolError::Invalid)?;
    let dest = CString::new(dest).map_err(|_| SpoolError::Invalid)?;
    // SAFETY: both names and the owned directory FD remain live; no fallback.
    if unsafe {
        libc::renameat2(
            parent.as_raw_fd(),
            source.as_ptr(),
            parent.as_raw_fd(),
            dest.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    } < 0
    {
        let e = std::io::Error::last_os_error();
        return if e.kind() == std::io::ErrorKind::AlreadyExists {
            Err(SpoolError::Conflict)
        } else {
            Err(e.into())
        };
    }
    Ok(())
}
fn check_named_parent(
    path: &Path,
    pinned: &File,
    stamp: &FileStamp,
    uid: u32,
    gid: u32,
) -> Result<(), SpoolError> {
    let named = open_directory(path, uid, gid, 0o700)?;
    if !stamp.same_directory(&named.metadata()?) || !stamp.same_directory(&pinned.metadata()?) {
        return Err(SpoolError::Invalid);
    }
    Ok(())
}
fn sync_published(directory: &File) -> Result<(), SpoolError> {
    for name in SEALED_FILES {
        open_child(directory, name, false)?.sync_all()?;
    }
    directory.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exclusive_write_and_rename_primitives_never_replace_existing_names() {
        let temp = tempfile::tempdir().unwrap();
        fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o700)).unwrap();
        // SAFETY: get effective fixture identity, no mutation or external process.
        let (uid, gid) = unsafe { (libc::geteuid(), libc::getegid()) };
        let parent = open_directory(temp.path(), uid, gid, 0o700).unwrap();
        write_child(&parent, "kept", b"original", uid, gid).unwrap();
        assert!(write_child(&parent, "kept", b"replacement", uid, gid).is_err());
        assert_eq!(fs::read(temp.path().join("kept")).unwrap(), b"original");
        std::os::unix::fs::symlink("kept", temp.path().join("linked")).unwrap();
        assert!(write_child(&parent, "linked", b"replacement", uid, gid).is_err());
        mkdir_child(&parent, "staging").unwrap();
        mkdir_child(&parent, "sealed").unwrap();
        let sealed = open_child(&parent, "sealed", true).unwrap();
        write_child(&sealed, "kept", b"generation-one", uid, gid).unwrap();
        assert!(matches!(
            rename_no_replace(&parent, "staging", "sealed"),
            Err(SpoolError::Conflict)
        ));
        assert!(temp.path().join("staging").is_dir());
        assert_eq!(
            fs::read(temp.path().join("sealed/kept")).unwrap(),
            b"generation-one"
        );
    }
}
