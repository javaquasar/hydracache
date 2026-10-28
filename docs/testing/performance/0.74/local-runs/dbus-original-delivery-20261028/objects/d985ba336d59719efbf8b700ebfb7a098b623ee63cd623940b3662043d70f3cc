//! Fixed output names and retained ancestry; read-only, never launch authority.

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
const PRODUCTION_ROOT: &str = "/var/lib/hydracache-performance/diagnostics";

#[derive(Debug, Error)]
pub enum NamedOutputError {
    #[error("fixed output path, state, metadata or byte budget refused")]
    Invalid,
    #[error("original output observation already refused")]
    Refused,
    #[error("fixed output I/O failed: {0}")]
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
    production: bool,
}

/// Fixed-root read-only capability. Worker UID/GID are explicit assertions,
/// not account enrollment. Cannot convert to a fixture or expose stream FDs.
pub struct ProductionOutputRead {
    inner: FixtureOutputRead,
}

#[derive(Clone, Copy)]
enum ProductionLevel {
    Ancestor,
    Lease,
    Cell,
}

fn production_directory_valid(s: &Stable, level: ProductionLevel, uid: u32, gid: u32) -> bool {
    if s.mode & libc::S_IFMT != libc::S_IFDIR {
        return false;
    }
    match level {
        ProductionLevel::Ancestor => {
            s.uid == 0 && s.gid == 0 && s.mode & 0o7022 == 0 && s.mode & 0o001 != 0
        }
        // Root alone may list or change lease entries. The worker still needs
        // known-path search after systemd drops UID before applying its cwd.
        ProductionLevel::Lease => s.uid == 0 && s.gid == 0 && s.mode & 0o7777 == 0o711,
        ProductionLevel::Cell => s.uid == uid && s.gid == gid && s.mode & 0o7777 == 0o700,
    }
}

pub fn pin_production_outputs(
    state: &DiagnosticState,
    worker_uid: u32,
    worker_gid: u32,
) -> Result<ProductionOutputRead, NamedOutputError> {
    if [worker_uid, worker_gid]
        .iter()
        .any(|id| *id == 0 || *id == u32::MAX)
    {
        return Err(NamedOutputError::Invalid);
    }
    Ok(ProductionOutputRead {
        inner: pin_outputs(
            Path::new(PRODUCTION_ROOT),
            state,
            worker_uid,
            worker_gid,
            true,
        )?,
    })
}

impl ProductionOutputRead {
    pub(crate) fn refuse(&mut self) {
        self.inner.refused = true;
    }
    pub(crate) fn matches_production_state(&self, state: &DiagnosticState) -> bool {
        self.inner.production && state == &self.inner.state
    }
    pub(crate) fn bind_original_process_io<'a>(
        &mut self,
        state: &DiagnosticState,
        material: &mut PinnedStartMaterial,
        process: &'a ProcessRead,
    ) -> Result<ProcessIoRead<'a>, NamedOutputError> {
        let result = (|| {
            if !self.matches_production_state(state) || !material.matches_production_state(state) {
                return Err(NamedOutputError::Invalid);
            }
            self.revalidate()?;
            let io = material.bind_asserted_process_io(
                state,
                process,
                &self.inner.streams[0],
                &self.inner.streams[1],
            )?;
            self.revalidate()?;
            Ok(io)
        })();
        if result.is_err() {
            self.refuse();
            material.refuse();
        }
        result
    }
    pub fn is_refused(&self) -> bool {
        self.inner.is_refused()
    }
    pub fn revalidate(&mut self) -> Result<(), NamedOutputError> {
        self.inner.revalidate()
    }
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
    pin_outputs(root, state, uid, gid, false)
}

fn pin_outputs(
    root: &Path,
    state: &DiagnosticState,
    uid: u32,
    gid: u32,
    production: bool,
) -> Result<FixtureOutputRead, NamedOutputError> {
    let intent = diagnostic_start_intent(state).map_err(|_| NamedOutputError::Invalid)?;
    let raw = root.as_os_str().as_bytes();
    if !root.is_absolute()
        || (production && root != Path::new(PRODUCTION_ROOT))
        || (!production
            && (root.starts_with("/var/lib/hydracache-performance")
                || root.starts_with("/opt/hydracache-performance")))
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
    let first_stamp = Stable::of(&first.metadata()?);
    if production && !production_directory_valid(&first_stamp, ProductionLevel::Ancestor, uid, gid)
    {
        return Err(NamedOutputError::Invalid);
    }
    let mut directories = vec![Directory {
        stamp: first_stamp,
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
        if production {
            let level = if i < parts.len() {
                ProductionLevel::Ancestor
            } else if i == parts.len() {
                ProductionLevel::Lease
            } else {
                ProductionLevel::Cell
            };
            if !production_directory_valid(&Stable::of(&meta), level, uid, gid) {
                return Err(NamedOutputError::Invalid);
            }
        } else if i + 1 >= parts.len()
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
        production,
    };
    guard.revalidate()?;
    Ok(guard)
}

impl FixtureOutputRead {
    pub fn is_fixture(&self) -> bool {
        !self.production
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
            if self.production || state != &self.state || !material.is_fixture() {
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
pub(crate) fn fixture_output_for_refusal_test(inner: FixtureOutputRead) -> ProductionOutputRead {
    assert!(inner.is_fixture());
    // Keep fixture origin: no production conversion even in this negative seam.
    ProductionOutputRead { inner }
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
    fn synthetic_output_origin_and_exact_execution_state_cannot_drift() {
        let f = Fixture::new();
        let inner = f.pin().unwrap();
        let mut output = ProductionOutputRead { inner };
        assert!(!output.matches_production_state(&f.state));
        // Synthetic private policy bit only; never use this object for binding
        // or claim successful production ancestry from a temporary root.
        output.inner.production = true;
        assert!(output.matches_production_state(&f.state));
        for field in 0..9 {
            let mut wrong = f.state.clone();
            match field {
                0 => wrong.revision += 1,
                1 => wrong.last_observed_monotonic_ns += 1,
                2 => wrong.controller_monotonic_ns += 1,
                3 => wrong.reserved_monotonic_ns += 1,
                4 => wrong.completed_cells += 1,
                5 => wrong.identity.lease_id = "d".repeat(64),
                6 => wrong.identity.boot_id = "ffffffff-ffff-ffff-ffff-ffffffffffff".into(),
                7 => wrong.identity.binary_sha256 = "d".repeat(64),
                _ => wrong.identity.build_provenance_sha256 = "d".repeat(64),
            }
            assert!(!output.matches_production_state(&wrong), "field {field}");
        }
    }
    #[test]
    fn production_search_policy_preserves_nonroot_working_directory_traversal() {
        let mut s = Stable {
            dev: 1,
            ino: 2,
            uid: 0,
            gid: 0,
            mode: libc::S_IFDIR | 0o700,
        };
        assert!(!production_directory_valid(
            &s,
            ProductionLevel::Ancestor,
            1000,
            1000
        ));
        assert!(!production_directory_valid(
            &s,
            ProductionLevel::Lease,
            1000,
            1000
        ));
        s.mode = libc::S_IFDIR | 0o711;
        assert!(production_directory_valid(
            &s,
            ProductionLevel::Ancestor,
            1000,
            1000
        ));
        assert!(production_directory_valid(
            &s,
            ProductionLevel::Lease,
            1000,
            1000
        ));
        assert_eq!(s.mode & 0o077, 0o011); // Search only: no listing or namespace writes.
        s.mode = libc::S_IFDIR | 0o710;
        assert!(!production_directory_valid(
            &s,
            ProductionLevel::Ancestor,
            1000,
            1000
        ));
        assert!(!production_directory_valid(
            &s,
            ProductionLevel::Lease,
            1000,
            1000
        ));
    }

    #[test]
    fn production_ancestry_policy_requires_root_and_private_worker_cell() {
        let root = Stable {
            dev: 1,
            ino: 2,
            uid: 0,
            gid: 0,
            mode: libc::S_IFDIR | 0o755,
        };
        for level in [ProductionLevel::Ancestor, ProductionLevel::Lease] {
            let mut stamp = root.clone();
            if matches!(level, ProductionLevel::Lease) {
                stamp.mode = libc::S_IFDIR | 0o711;
            }
            assert!(production_directory_valid(&stamp, level, 1000, 1000));
            for case in 0..5 {
                let mut bad = stamp.clone();
                match case {
                    0 => bad.uid = 1000,
                    1 => bad.gid = 1000,
                    2 => bad.mode |= 0o020,
                    3 => bad.mode |= 0o002,
                    _ => bad.mode = libc::S_IFREG | 0o700,
                }
                assert!(!production_directory_valid(&bad, level, 1000, 1000));
            }
        }
        let cell = Stable {
            uid: 1000,
            gid: 1000,
            mode: libc::S_IFDIR | 0o700,
            ..root
        };
        assert!(production_directory_valid(
            &cell,
            ProductionLevel::Cell,
            1000,
            1000
        ));
        assert!(!production_directory_valid(
            &cell,
            ProductionLevel::Cell,
            1001,
            1000
        ));
        assert!(!production_directory_valid(
            &cell,
            ProductionLevel::Cell,
            1000,
            1001
        ));
    }

    #[test]
    fn production_directory_policy_exhaustively_refuses_special_or_writable_modes() {
        for mode in 0..=0o7777 {
            let stamp = Stable {
                dev: 1,
                ino: 2,
                uid: 0,
                gid: 0,
                mode: libc::S_IFDIR | mode,
            };
            assert_eq!(
                production_directory_valid(&stamp, ProductionLevel::Ancestor, 1000, 1000),
                mode & 0o7022 == 0 && mode & 0o001 != 0
            );
            assert_eq!(
                production_directory_valid(&stamp, ProductionLevel::Lease, 1000, 1000),
                mode == 0o711
            );
            let cell = Stable {
                uid: 1000,
                gid: 1000,
                ..stamp
            };
            assert_eq!(
                production_directory_valid(&cell, ProductionLevel::Cell, 1000, 1000),
                mode == 0o700
            );
        }
    }

    #[test]
    fn production_reader_refuses_invalid_owner_and_state_before_filesystem() {
        let f = Fixture::new();
        for (uid, gid) in [(0, 1000), (1000, 0), (u32::MAX, 1000), (1000, u32::MAX)] {
            assert!(matches!(
                pin_production_outputs(&f.state, uid, gid),
                Err(NamedOutputError::Invalid)
            ));
        }
        let mut state = f.state.clone();
        state.stage = DiagnosticStage::Running;
        assert!(matches!(
            pin_production_outputs(&state, 1000, 1000),
            Err(NamedOutputError::Invalid)
        ));
        state = f.state.clone();
        state.last_observed_monotonic_ns = 300_000_000_001;
        state.controller_monotonic_ns = state.last_observed_monotonic_ns;
        assert!(matches!(
            pin_production_outputs(&state, 1000, 1000),
            Err(NamedOutputError::Invalid)
        ));
    }

    #[test]
    fn synthetic_production_wrapper_retains_shared_reader_refusal() {
        // Private wrapper seam only; temporary ancestry is not production proof.
        let f = Fixture::new();
        let mut reader = ProductionOutputRead {
            inner: f.pin().unwrap(),
        };
        reader.revalidate().unwrap();
        assert!(!reader.is_refused());
        fs::set_permissions(&f.cell, fs::Permissions::from_mode(0o750)).unwrap();
        assert!(reader.revalidate().is_err());
        fs::set_permissions(&f.cell, fs::Permissions::from_mode(0o700)).unwrap();
        assert!(matches!(
            reader.revalidate(),
            Err(NamedOutputError::Refused)
        ));
        assert!(reader.is_refused());
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
