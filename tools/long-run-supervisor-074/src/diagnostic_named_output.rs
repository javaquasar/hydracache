//! Fixed fixture output names; no production ownership or launch authority.

use crate::diagnostic_artifacts::linux::{PinnedStartMaterial, ProcessIoBindingError};
use crate::diagnostic_lease::DiagnosticState;
use crate::diagnostic_process::{ProcessIoRead, ProcessRead};
use crate::diagnostic_unit::diagnostic_start_intent;
use std::ffi::{CString, OsString};
use std::fs::{self, File, Metadata, OpenOptions};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;
use thiserror::Error;

const NAMES: [&str; 2] = ["stdout.json", "stderr.log"];
const STREAM_BYTES: u64 = crate::diagnostic_receipts::STREAM_BYTES as u64;
const MAX_PATH_BYTES: usize = 4096;
const MAX_DEPTH: usize = 64;

#[derive(Debug, Error)]
pub enum NamedOutputError {
    #[error("fixed fixture output path, state, metadata or byte budget refused")]
    Invalid,
    #[error("original fixture output observation already refused")]
    Refused,
    #[error("fixed fixture output I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("fixture original-process binding failed: {0}")]
    Binding(#[from] ProcessIoBindingError),
    #[error("fixture original-process observation failed: {0}")]
    Process(#[from] crate::diagnostic_process::ProcessError),
}

#[derive(Clone, PartialEq, Eq)]
struct Stable {
    dev: u64,
    ino: u64,
    uid: u32,
    gid: u32,
    mode: u32,
}
impl Stable {
    fn of(m: &Metadata) -> Self {
        Self {
            dev: m.dev(),
            ino: m.ino(),
            uid: m.uid(),
            gid: m.gid(),
            mode: m.mode(),
        }
    }
}
struct Directory {
    file: File,
    stamp: Stable,
    name: CString,
}

/// Explicit fixture ownership assertions, never production ancestry proof.
/// No stream content access, raw FD export, serialization or automatic refresh.
pub struct FixtureOutputRead {
    state: DiagnosticState,
    directories: Vec<Directory>,
    streams: [File; 2],
    stamps: [Stable; 2],
    high_water: [u64; 2],
    refused: bool,
}

/// Borrows both original capabilities; a failed composed observation also
/// refuses its named-output guard. Not durable failure or start authentication.
pub struct FixtureProcessIoRead<'a> {
    outputs: &'a mut FixtureOutputRead,
    io: ProcessIoRead<'a>,
}

pub fn pin_fixture_outputs(
    root: &Path,
    state: &DiagnosticState,
    uid: u32,
    gid: u32,
) -> Result<FixtureOutputRead, NamedOutputError> {
    let intent = diagnostic_start_intent(state).map_err(|_| NamedOutputError::Invalid)?;
    let raw = root.as_os_str().as_bytes();
    if !root.is_absolute()
        || root.starts_with("/var/lib/hydracache-performance")
        || root.starts_with("/opt/hydracache-performance")
        || raw.len() > MAX_PATH_BYTES
    {
        return Err(NamedOutputError::Invalid);
    }
    // Reject raw dot/empty components before Path normalization can hide them.
    let parts: Vec<_> = raw.split(|b| *b == b'/').skip(1).collect();
    if parts.is_empty()
        || parts.len() + 2 > MAX_DEPTH
        || parts
            .iter()
            .any(|p| p.is_empty() || *p == b"." || *p == b"..")
        || raw.len() + intent.lease_id.len() + intent.surface.len() + 2 > MAX_PATH_BYTES
    {
        return Err(NamedOutputError::Invalid);
    }
    let first = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open("/")?;
    let mut directories = vec![Directory {
        stamp: Stable::of(&first.metadata()?),
        file: first,
        name: CString::new("/").unwrap(),
    }];
    for (i, bytes) in parts
        .iter()
        .copied()
        .chain([intent.lease_id.as_bytes(), intent.surface.as_bytes()])
        .enumerate()
    {
        let name = CString::new(bytes).map_err(|_| NamedOutputError::Invalid)?;
        let file = open_at(&directories.last().unwrap().file, &name, true)?;
        let meta = file.metadata()?;
        if i + 1 >= parts.len()
            && (meta.uid() != uid || meta.gid() != gid || meta.mode() & 0o7777 != 0o700)
        {
            return Err(NamedOutputError::Invalid);
        }
        directories.push(Directory {
            stamp: Stable::of(&meta),
            file,
            name,
        });
    }
    let leaf = &directories.last().unwrap().file;
    check_entries(leaf)?;
    let streams = [open_stream(leaf, NAMES[0])?, open_stream(leaf, NAMES[1])?];
    let metadata = [streams[0].metadata()?, streams[1].metadata()?];
    for meta in &metadata {
        check_stream(meta)?;
        if meta.uid() != uid || meta.gid() != gid {
            return Err(NamedOutputError::Invalid);
        }
    }
    if metadata[0].dev() == metadata[1].dev() && metadata[0].ino() == metadata[1].ino() {
        return Err(NamedOutputError::Invalid);
    }
    let mut guard = FixtureOutputRead {
        state: state.clone(),
        directories,
        streams,
        stamps: [Stable::of(&metadata[0]), Stable::of(&metadata[1])],
        high_water: [metadata[0].len(), metadata[1].len()],
        refused: false,
    };
    guard.revalidate()?;
    Ok(guard)
}

impl FixtureOutputRead {
    pub fn is_fixture(&self) -> bool {
        true
    }
    pub fn is_refused(&self) -> bool {
        self.refused
    }
    pub fn revalidate(&mut self) -> Result<(), NamedOutputError> {
        if self.refused {
            return Err(NamedOutputError::Refused);
        }
        let result = self.inspect();
        if result.is_err() {
            self.refused = true;
        }
        result
    }
    pub fn bind_fixture_process_io<'a>(
        &'a mut self,
        material: &mut PinnedStartMaterial,
        state: &DiagnosticState,
        process: &'a ProcessRead,
    ) -> Result<FixtureProcessIoRead<'a>, NamedOutputError> {
        let result = (|| {
            self.revalidate()?;
            if state != &self.state || !material.is_fixture() {
                return Err(NamedOutputError::Invalid);
            }
            let io = material.bind_asserted_process_io(
                state,
                process,
                &self.streams[0],
                &self.streams[1],
            )?;
            self.revalidate()?;
            Ok(io)
        })();
        match result {
            Ok(io) => Ok(FixtureProcessIoRead { outputs: self, io }),
            Err(error) => {
                self.refused = true;
                Err(error)
            }
        }
    }
    fn inspect(&mut self) -> Result<(), NamedOutputError> {
        self.check_directories()?;
        let leaf = &self.directories.last().unwrap().file;
        check_entries(leaf)?;
        for (i, name) in NAMES.iter().enumerate() {
            let named = open_stream(leaf, name)?;
            for meta in [
                self.streams[i].metadata()?,
                named.metadata()?,
                self.streams[i].metadata()?,
            ] {
                check_stream(&meta)?;
                if Stable::of(&meta) != self.stamps[i] || meta.len() < self.high_water[i] {
                    return Err(NamedOutputError::Invalid);
                }
                self.high_water[i] = meta.len();
            }
        }
        check_entries(leaf)?;
        self.check_directories()
    }
    fn check_directories(&self) -> Result<(), NamedOutputError> {
        for (i, dir) in self.directories.iter().enumerate() {
            let named = if i == 0 {
                OpenOptions::new()
                    .read(true)
                    .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
                    .open("/")?
            } else {
                open_at(&self.directories[i - 1].file, &dir.name, true)?
            };
            if Stable::of(&dir.file.metadata()?) != dir.stamp
                || Stable::of(&named.metadata()?) != dir.stamp
            {
                return Err(NamedOutputError::Invalid);
            }
        }
        Ok(())
    }
}
impl FixtureProcessIoRead<'_> {
    pub fn is_fixture(&self) -> bool {
        true
    }
    pub fn is_refused(&self) -> bool {
        self.outputs.is_refused() || self.io.is_refused()
    }
    pub fn revalidate(&mut self) -> Result<(), NamedOutputError> {
        let result = (|| {
            self.outputs.revalidate()?;
            self.io.revalidate()?;
            self.outputs.revalidate()
        })();
        if result.is_err() {
            self.outputs.refused = true;
        }
        result
    }
}
fn check_stream(meta: &Metadata) -> Result<(), NamedOutputError> {
    if !meta.is_file()
        || meta.nlink() != 1
        || meta.mode() & 0o7777 != 0o600
        || meta.len() > STREAM_BYTES
    {
        return Err(NamedOutputError::Invalid);
    }
    Ok(())
}
fn open_stream(parent: &File, name: &str) -> Result<File, NamedOutputError> {
    open_at(
        parent,
        &CString::new(name).map_err(|_| NamedOutputError::Invalid)?,
        false,
    )
}
fn open_at(parent: &File, name: &CString, directory: bool) -> Result<File, NamedOutputError> {
    let flags = libc::O_RDONLY
        | libc::O_NOFOLLOW
        | libc::O_CLOEXEC
        | libc::O_NONBLOCK
        | if directory { libc::O_DIRECTORY } else { 0 };
    // SAFETY: retained owned parent and bounded NUL-terminated single component.
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    // SAFETY: successful openat returns a new owned descriptor.
    Ok(unsafe { File::from_raw_fd(fd) })
}
fn check_entries(directory: &File) -> Result<(), NamedOutputError> {
    let mut seen = std::collections::BTreeSet::new();
    // This fixed self-FD link refers to the retained owned directory, not a
    // caller pathname or another process. Limit collection to two entries.
    for entry in fs::read_dir(format!("/proc/self/fd/{}", directory.as_raw_fd()))? {
        seen.insert(entry?.file_name());
        if seen.len() > NAMES.len() {
            return Err(NamedOutputError::Invalid);
        }
    }
    if seen != NAMES.into_iter().map(OsString::from).collect() {
        return Err(NamedOutputError::Invalid);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic_lease::{DiagnosticIdentity, DiagnosticStage};
    use std::fs::{self, OpenOptions};
    use std::io::{Seek, SeekFrom, Write};
    use std::os::unix::fs::{symlink, OpenOptionsExt, PermissionsExt};

    struct Fixture {
        temp: tempfile::TempDir,
        root: std::path::PathBuf,
        cell: std::path::PathBuf,
        state: DiagnosticState,
        uid: u32,
        gid: u32,
    }
    impl Fixture {
        fn new() -> Self {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path().join("outputs");
            let state = DiagnosticState {
                identity: DiagnosticIdentity {
                    lease_id: "a".repeat(64),
                    boot_id: "00000000-0000-4000-8000-000000000074".into(),
                    binary_sha256: "b".repeat(64),
                    build_provenance_sha256: "c".repeat(64),
                },
                revision: 1,
                stage: DiagnosticStage::Reserved,
                completed_cells: 0,
                reserved_monotonic_ns: 1,
                last_observed_monotonic_ns: 1,
                controller_monotonic_ns: 1,
                cell_started_monotonic_ns: None,
                cgroup_inode: None,
                reason: None,
                cleanup_confirmed: true,
                promotable: false,
                admission_allowed: false,
            };
            let cell = root.join(&state.identity.lease_id).join("embedded");
            fs::create_dir_all(&cell).unwrap();
            for path in [&root, cell.parent().unwrap(), &cell] {
                fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
            }
            for name in NAMES {
                OpenOptions::new()
                    .create_new(true)
                    .append(true)
                    .mode(0o600)
                    .open(cell.join(name))
                    .unwrap();
            }
            // SAFETY: read-only identity queries for temporary fixture ownership.
            let (uid, gid) = unsafe { (libc::geteuid(), libc::getegid()) };
            Self {
                temp,
                root,
                cell,
                state,
                uid,
                gid,
            }
        }
        fn pin(&self) -> Result<FixtureOutputRead, NamedOutputError> {
            pin_fixture_outputs(&self.root, &self.state, self.uid, self.gid)
        }
    }

    #[test]
    fn fixed_fixture_names_accept_append_without_reading_or_seeking() {
        let f = Fixture::new();
        let mut guard = f.pin().unwrap();
        assert!(guard.is_fixture());
        for stream in &mut guard.streams {
            stream.seek(SeekFrom::Start(3)).unwrap();
        }
        for i in 0..128 {
            OpenOptions::new()
                .append(true)
                .open(f.cell.join(NAMES[i % 2]))
                .unwrap()
                .write_all(b"append\n")
                .unwrap();
            guard.revalidate().unwrap();
        }
        for stream in &mut guard.streams {
            assert_eq!(stream.stream_position().unwrap(), 3);
        }
        assert!(!guard.is_refused());
    }

    #[test]
    fn replaced_stream_and_repair_cannot_refresh_original_names() {
        let f = Fixture::new();
        let mut guard = f.pin().unwrap();
        let path = f.cell.join(NAMES[0]);
        let kept = f.temp.path().join("original");
        fs::rename(&path, &kept).unwrap();
        OpenOptions::new()
            .create_new(true)
            .append(true)
            .mode(0o600)
            .open(&path)
            .unwrap();
        assert!(guard.revalidate().is_err());
        fs::remove_file(&path).unwrap();
        fs::rename(&kept, &path).unwrap();
        assert!(guard.revalidate().is_err());
        assert!(guard.is_refused());
    }

    #[test]
    fn every_fixture_directory_generation_and_ancestor_binding_is_retained() {
        for level in 0..4 {
            let f = Fixture::new();
            let mut guard = f.pin().unwrap();
            let path = match level {
                0 => f.cell.clone(),
                1 => f.cell.parent().unwrap().to_owned(),
                2 => f.root.clone(),
                _ => f.temp.path().to_owned(),
            };
            let displaced = path.with_extension("displaced");
            fs::rename(&path, &displaced).unwrap();
            fs::create_dir(&path).unwrap();
            let failed = guard.revalidate().is_err();
            fs::remove_dir(&path).unwrap();
            fs::rename(&displaced, &path).unwrap();
            assert!(failed);
            assert!(guard.revalidate().is_err());
        }
    }

    #[test]
    fn names_links_special_files_and_owner_modes_refuse() {
        for case in 0..10 {
            let f = Fixture::new();
            let path = f.cell.join(NAMES[0]);
            match case {
                0 => {
                    fs::write(f.cell.join("extra"), b"").unwrap();
                }
                1 => {
                    fs::remove_file(&path).unwrap();
                }
                2 => {
                    fs::remove_file(&path).unwrap();
                    symlink(f.cell.join(NAMES[1]), &path).unwrap();
                }
                3 => {
                    fs::hard_link(&path, f.temp.path().join("alias")).unwrap();
                }
                4 => {
                    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
                }
                5 => {
                    fs::set_permissions(&f.cell, fs::Permissions::from_mode(0o750)).unwrap();
                }
                6 => {
                    fs::remove_file(&path).unwrap();
                    fs::create_dir(&path).unwrap();
                }
                7 => {
                    fs::remove_file(&path).unwrap();
                    let name = CString::new(path.as_os_str().as_bytes()).unwrap();
                    // SAFETY: temporary owned name; no production FIFO or process.
                    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
                }
                8 => {
                    assert!(pin_fixture_outputs(&f.root, &f.state, u32::MAX, f.gid).is_err());
                    continue;
                }
                _ => {
                    assert!(pin_fixture_outputs(&f.root, &f.state, f.uid, u32::MAX).is_err());
                    continue;
                }
            }
            assert!(f.pin().is_err(), "case {case}");
        }
    }

    #[test]
    fn fixture_scope_refuses_production_noncanonical_paths_and_nonstartable_state() {
        let f = Fixture::new();
        for root in [
            "/var/lib/hydracache-performance/diagnostics",
            "/opt/hydracache-performance/0.74/diagnostic-pilot",
            "relative",
            "/tmp/../tmp",
            "/tmp/./outputs",
            "/tmp//outputs",
            "/tmp/outputs/",
        ] {
            assert!(pin_fixture_outputs(Path::new(root), &f.state, f.uid, f.gid).is_err());
        }
        for root in [
            format!("/{}", "a".repeat(4096)),
            format!("/{}", vec!["a"; 65].join("/")),
        ] {
            assert!(pin_fixture_outputs(Path::new(&root), &f.state, f.uid, f.gid).is_err());
        }
        let linked = f.temp.path().join("linked");
        symlink(&f.root, &linked).unwrap();
        assert!(pin_fixture_outputs(&linked, &f.state, f.uid, f.gid).is_err());
        let mut wrong = f.state.clone();
        wrong.stage = DiagnosticStage::Running;
        assert!(pin_fixture_outputs(&f.root, &wrong, f.uid, f.gid).is_err());
        wrong = f.state.clone();
        wrong.identity.lease_id = "../escape".into();
        assert!(pin_fixture_outputs(&f.root, &wrong, f.uid, f.gid).is_err());
    }

    #[test]
    fn observed_output_shrink_overflow_and_metadata_drift_latch() {
        for case in 0..5 {
            let f = Fixture::new();
            let path = f.cell.join(NAMES[0]);
            fs::write(&path, b"before").unwrap();
            let mut guard = f.pin().unwrap();
            match case {
                0 => {
                    OpenOptions::new()
                        .write(true)
                        .open(&path)
                        .unwrap()
                        .set_len(0)
                        .unwrap();
                }
                1 => {
                    OpenOptions::new()
                        .write(true)
                        .open(&path)
                        .unwrap()
                        .set_len(STREAM_BYTES + 1)
                        .unwrap();
                }
                2 => {
                    fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
                }
                3 => {
                    fs::hard_link(&path, f.temp.path().join("alias")).unwrap();
                }
                _ => {
                    fs::set_permissions(&f.root, fs::Permissions::from_mode(0o750)).unwrap();
                }
            }
            assert!(guard.revalidate().is_err(), "case {case}");
            assert!(guard.is_refused());
        }
        let f = Fixture::new();
        OpenOptions::new()
            .write(true)
            .open(f.cell.join(NAMES[0]))
            .unwrap()
            .set_len(STREAM_BYTES + 1)
            .unwrap();
        assert!(f.pin().is_err());
    }

    #[test]
    fn parallel_named_readers_allow_append_and_ignore_unrelated_ancestor_entries() {
        let f = Fixture::new();
        let mut guards: Vec<_> = (0..4).map(|_| f.pin().unwrap()).collect();
        fs::create_dir(f.temp.path().join("unrelated")).unwrap();
        std::thread::scope(|scope| {
            for guard in &mut guards {
                scope.spawn(move || {
                    for _ in 0..64 {
                        guard.revalidate().unwrap();
                    }
                });
            }
            for _ in 0..256 {
                OpenOptions::new()
                    .append(true)
                    .open(f.cell.join(NAMES[0]))
                    .unwrap()
                    .write_all(b"x")
                    .unwrap();
            }
        });
    }
}
