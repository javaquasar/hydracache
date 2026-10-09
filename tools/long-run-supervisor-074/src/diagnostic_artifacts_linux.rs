//! Linux-only pinned descriptor inspection, not a live systemd exec capability.

use super::*;
use crate::diagnostic_builder::CheckedBuilderPolicy;
use crate::diagnostic_lease::DiagnosticState;
use crate::diagnostic_unit::{build_diagnostic_unit_spec, diagnostic_start_intent};
use crate::systemd_unit::TransientUnitSpec;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::ffi::CString;
use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Component, Path, PathBuf};

const FILES: [&str; 9] = [
    "timing-controls-074",
    "build-receipt-v1.json",
    "build-log.jsonl",
    "Cargo.lock.root",
    "Cargo.lock.observer",
    "embedded.json",
    "direct.json",
    "resp2.json",
    "resp3.json",
];

#[derive(Clone, PartialEq, Eq)]
struct Stamp {
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
impl From<Metadata> for Stamp {
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

struct PinnedFile {
    file: File,
    stamp: Stamp,
    digest: ArtifactDigest,
}

/// Descriptors remain owned by this value. Revalidation is a gate, not a promise
/// that a future pathname exec cannot race a trusted install/root mutation.
pub struct BundleSnapshot {
    path: PathBuf,
    fixture: bool,
    uid: u32,
    gid: u32,
    directory: File,
    stamp: Stamp,
    files: BTreeMap<String, PinnedFile>,
    build: VerifiedBuild,
}

/// Only the fixed production root and root-owned safe ancestors are accepted.
/// This read-only function is not called by a production route in this slice.
pub fn inspect_fixed_install(
    identity: &DiagnosticIdentity,
    policy: &CheckedBuilderPolicy,
) -> Result<BundleSnapshot, ArtifactError> {
    inspect(Path::new(INSTALL_ROOT), false, 0, 0, |receipt| {
        policy.verify_receipt(receipt, identity)
    })
}

/// Explicit local fixture seam. Does NOT certify production ancestry/ownership.
pub fn inspect_fixture(
    path: &Path,
    uid: u32,
    gid: u32,
    identity: &DiagnosticIdentity,
    trust: &BuildTrust,
) -> Result<BundleSnapshot, ArtifactError> {
    if path == Path::new(INSTALL_ROOT) {
        return Err(ArtifactError::Contents);
    }
    inspect(path, true, uid, gid, |receipt| {
        verify_receipt(receipt, identity, trust)
    })
}

/// Policy/content/state binding with owned descriptors, NOT execution authority.
/// No serialization, unit mutation, clock observation or host lock is performed.
/// The backend must still fence a durable intent and authenticate actual start.
pub struct PinnedStartMaterial {
    bundle: BundleSnapshot,
    state: DiagnosticState,
    intent: CellIntent,
    spec: TransientUnitSpec,
    refused: bool,
}

/// Read-only preparation at the fixed install root using an independently
/// checked policy. Invalid/nonstartable state is refused before filesystem IO.
pub fn prepare_fixed_start_material(
    state: &DiagnosticState,
    policy: &CheckedBuilderPolicy,
) -> Result<PinnedStartMaterial, ArtifactError> {
    let spec = build_diagnostic_unit_spec(state).map_err(|_| ArtifactError::Invalid)?;
    let bundle = inspect_fixed_install(&state.identity, policy)?;
    prepare_material(bundle, state, spec)
}

/// Test-only origin, not certification of production install ancestry.
pub fn prepare_fixture_start_material(
    path: &Path,
    uid: u32,
    gid: u32,
    state: &DiagnosticState,
    policy: &CheckedBuilderPolicy,
) -> Result<PinnedStartMaterial, ArtifactError> {
    let spec = build_diagnostic_unit_spec(state).map_err(|_| ArtifactError::Invalid)?;
    if path == Path::new(INSTALL_ROOT) {
        return Err(ArtifactError::Contents);
    }
    let bundle = inspect(path, true, uid, gid, |receipt| {
        policy.verify_receipt(receipt, &state.identity)
    })?;
    prepare_material(bundle, state, spec)
}

fn prepare_material(
    mut bundle: BundleSnapshot,
    state: &DiagnosticState,
    spec: TransientUnitSpec,
) -> Result<PinnedStartMaterial, ArtifactError> {
    let intent = diagnostic_start_intent(state).map_err(|_| ArtifactError::Invalid)?;
    bundle.build().bind_intent(&intent)?;
    bundle.revalidate()?;
    Ok(PinnedStartMaterial {
        bundle,
        state: state.clone(),
        intent,
        spec,
        refused: false,
    })
}

impl PinnedStartMaterial {
    /// Read-only metadata, including after refusal; never a launch permission.
    pub fn intent(&self) -> &CellIntent {
        &self.intent
    }
    pub fn spec(&self) -> &TransientUnitSpec {
        &self.spec
    }
    pub fn is_fixture(&self) -> bool {
        self.bundle.is_fixture()
    }
    /// Exact state (including revision/clocks), plus the original bundle.
    /// A first refusal is sticky for this object, not a durable failure journal.
    pub fn revalidate_for(&mut self, state: &DiagnosticState) -> Result<(), ArtifactError> {
        if self.refused || state != &self.state {
            self.refused = true;
            return Err(ArtifactError::Invalid);
        }
        let result = self.bundle.revalidate();
        if result.is_err() {
            self.refused = true;
        }
        result
    }
}

fn inspect(
    path: &Path,
    fixture: bool,
    uid: u32,
    gid: u32,
    verify: impl FnOnce(&[u8]) -> Result<VerifiedBuild, ArtifactError>,
) -> Result<BundleSnapshot, ArtifactError> {
    let directory = open_directory(path, fixture, uid, gid)?;
    let stamp = Stamp::from(directory.metadata()?);
    check_entries(&directory)?;
    let mut files = BTreeMap::new();
    let mut retained = BTreeMap::new();
    let mut binary_header = Vec::new();
    for name in FILES {
        let (maximum, mode) = limits(name);
        let mut file = open_child(&directory, name, false)?;
        check_file(&file.metadata()?, uid, gid, mode, maximum)?;
        let original = Stamp::from(file.metadata()?);
        let (digest, bytes) = hash_file(&mut file, maximum, name != "timing-controls-074")?;
        if Stamp::from(file.metadata()?) != original || digest.bytes != original.len {
            return Err(ArtifactError::Contents);
        }
        if name == "timing-controls-074" {
            binary_header = bytes;
        } else {
            retained.insert(name, bytes);
        }
        files.insert(
            name.to_owned(),
            PinnedFile {
                file,
                stamp: original,
                digest,
            },
        );
    }
    let receipt = &retained["build-receipt-v1.json"];
    let build = verify(receipt)?;
    let s = build.statement();
    for (name, expected) in [
        ("timing-controls-074", &s.binary),
        ("build-log.jsonl", &s.build_log),
        ("Cargo.lock.root", &s.root_lock),
        ("Cargo.lock.observer", &s.observer_lock),
    ] {
        if &files[name].digest != expected {
            return Err(ArtifactError::Contents);
        }
    }
    for surface in SURFACES {
        if files[&format!("{surface}.json")].digest != s.configs[surface] {
            return Err(ArtifactError::Contents);
        }
    }
    check_elf(&binary_header, files["timing-controls-074"].digest.bytes)?;
    check_cargo_log(&retained["build-log.jsonl"])?;
    let mut snapshot = BundleSnapshot {
        path: path.to_owned(),
        fixture,
        uid,
        gid,
        directory,
        stamp,
        files,
        build,
    };
    snapshot.revalidate()?;
    Ok(snapshot)
}

impl BundleSnapshot {
    pub fn build(&self) -> &VerifiedBuild {
        &self.build
    }
    pub fn is_fixture(&self) -> bool {
        self.fixture
    }

    pub fn revalidate(&mut self) -> Result<(), ArtifactError> {
        let named = open_directory(&self.path, self.fixture, self.uid, self.gid)?;
        if Stamp::from(named.metadata()?) != self.stamp
            || Stamp::from(self.directory.metadata()?) != self.stamp
        {
            return Err(ArtifactError::Contents);
        }
        check_entries(&named)?;
        for (name, pinned) in &mut self.files {
            let current = open_child(&named, name, false)?;
            let (maximum, mode) = limits(name);
            check_file(&current.metadata()?, self.uid, self.gid, mode, maximum)?;
            if Stamp::from(current.metadata()?) != pinned.stamp
                || Stamp::from(pinned.file.metadata()?) != pinned.stamp
            {
                return Err(ArtifactError::Contents);
            }
            let (digest, _) = hash_file(&mut pinned.file, maximum, false)?;
            if digest != pinned.digest || Stamp::from(pinned.file.metadata()?) != pinned.stamp {
                return Err(ArtifactError::Contents);
            }
        }
        if Stamp::from(named.metadata()?) != self.stamp
            || Stamp::from(self.directory.metadata()?) != self.stamp
        {
            return Err(ArtifactError::Contents);
        }
        Ok(())
    }
}

fn limits(name: &str) -> (u64, u32) {
    match name {
        "timing-controls-074" => (MAX_BINARY_BYTES, 0o555),
        "build-receipt-v1.json" => (MAX_RECEIPT_BYTES, 0o444),
        "build-log.jsonl" => (MAX_LOG_BYTES, 0o444),
        "Cargo.lock.root" | "Cargo.lock.observer" => (MAX_LOCK_BYTES, 0o444),
        _ => (MAX_CONFIG_BYTES, 0o444),
    }
}

fn check_file(
    m: &Metadata,
    uid: u32,
    gid: u32,
    mode: u32,
    maximum: u64,
) -> Result<(), ArtifactError> {
    if !m.is_file()
        || m.uid() != uid
        || m.gid() != gid
        || m.mode() & 0o7777 != mode
        || m.nlink() != 1
        || m.len() == 0
        || m.len() > maximum
    {
        return Err(ArtifactError::Contents);
    }
    Ok(())
}

fn open_directory(path: &Path, fixture: bool, uid: u32, gid: u32) -> Result<File, ArtifactError> {
    if !path.is_absolute() {
        return Err(ArtifactError::Contents);
    }
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open("/")?;
    if !fixture {
        check_ancestor(&file.metadata()?)?;
    }
    for part in path.components() {
        match part {
            Component::RootDir => {}
            Component::Normal(name) => {
                let name = name.to_str().ok_or(ArtifactError::Contents)?;
                file = open_child(&file, name, true)?;
                if !fixture {
                    check_ancestor(&file.metadata()?)?;
                }
            }
            _ => return Err(ArtifactError::Contents),
        }
    }
    let m = file.metadata()?;
    if !m.is_dir() || m.uid() != uid || m.gid() != gid || m.mode() & 0o7777 != 0o555 {
        return Err(ArtifactError::Contents);
    }
    Ok(file)
}

fn check_ancestor(m: &Metadata) -> Result<(), ArtifactError> {
    if !m.is_dir() || m.uid() != 0 || m.gid() != 0 || m.mode() & 0o7022 != 0 {
        return Err(ArtifactError::Contents);
    }
    Ok(())
}

fn open_child(parent: &File, name: &str, directory: bool) -> Result<File, ArtifactError> {
    let name = CString::new(name).map_err(|_| ArtifactError::Contents)?;
    let flags = libc::O_RDONLY
        | libc::O_NOFOLLOW
        | libc::O_CLOEXEC
        | libc::O_NONBLOCK
        | if directory { libc::O_DIRECTORY } else { 0 };
    // SAFETY: owned parent fd and live NUL-terminated name; no creation flags.
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    // SAFETY: successful openat returned a new owned descriptor.
    Ok(unsafe { File::from_raw_fd(fd) })
}

fn check_entries(directory: &File) -> Result<(), ArtifactError> {
    // Linux procfs resolves our still-owned directory descriptor. Entries are
    // subsequently opened only by openat/O_NOFOLLOW, not by this pathname.
    let entries = fs::read_dir(format!("/proc/self/fd/{}", directory.as_raw_fd()))?
        .map(|entry| entry.map(|e| e.file_name()))
        .collect::<Result<BTreeSet<_>, _>>()?;
    let expected = FILES
        .into_iter()
        .map(std::ffi::OsString::from)
        .collect::<BTreeSet<_>>();
    if entries != expected {
        return Err(ArtifactError::Contents);
    }
    Ok(())
}

fn hash_file(
    file: &mut File,
    maximum: u64,
    retain: bool,
) -> Result<(ArtifactDigest, Vec<u8>), ArtifactError> {
    file.seek(SeekFrom::Start(0))?;
    let mut digest = Sha256::new();
    let mut bytes = Vec::new();
    let mut length = 0u64;
    let mut buffer = [0; 65_536];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        length = length
            .checked_add(count as u64)
            .ok_or(ArtifactError::Contents)?;
        if length > maximum {
            return Err(ArtifactError::Contents);
        }
        digest.update(&buffer[..count]);
        if retain {
            bytes.extend_from_slice(&buffer[..count]);
        } else if bytes.len() < 64 {
            bytes.extend_from_slice(&buffer[..count.min(64 - bytes.len())]);
        }
    }
    if length == 0 {
        return Err(ArtifactError::Contents);
    }
    Ok((
        ArtifactDigest {
            sha256: crate::hex(&digest.finalize()),
            bytes: length,
        },
        bytes,
    ))
}
