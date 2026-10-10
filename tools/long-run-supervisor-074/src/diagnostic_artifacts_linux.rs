//! Linux-only pinned descriptor inspection, not a live systemd exec capability.

use super::*;
use crate::diagnostic_builder::CheckedBuilderPolicy;
use crate::diagnostic_lease::DiagnosticState;
use crate::diagnostic_process::{ProcessError, ProcessIoRead, ProcessRead};
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

#[derive(Debug, Error)]
pub enum ProcessIoBindingError {
    #[error("checked start material refused: {0}")]
    Material(#[from] ArtifactError),
    #[error("original process IO observation refused: {0}")]
    Process(#[from] ProcessError),
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
    pub fn is_refused(&self) -> bool {
        self.refused
    }
    pub(crate) fn refuse(&mut self) {
        self.refused = true;
    }
    pub(crate) fn matches_production_state(&self, state: &DiagnosticState) -> bool {
        !self.is_fixture() && state == &self.state
    }
    /// Bind the original signed executable to a retained process generation.
    /// Output FDs are caller assertions, NOT certification of production names,
    /// safe ancestry or original start. No unit mutation/execution is enabled.
    pub fn bind_asserted_process_io<'a>(
        &mut self,
        state: &DiagnosticState,
        process: &'a ProcessRead,
        stdout: &File,
        stderr: &File,
    ) -> Result<ProcessIoRead<'a>, ProcessIoBindingError> {
        self.revalidate_for(state)?;
        let result = (|| {
            let binary = self
                .bundle
                .files
                .get("timing-controls-074")
                .ok_or(ArtifactError::Contents)?;
            let mut io = process.pin_io(&binary.file, stdout, stderr)?;
            self.revalidate_for(state)?;
            io.revalidate()?;
            Ok(io)
        })();
        if result.is_err() {
            self.refused = true;
        }
        result
    }
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

#[cfg(test)]
mod io_binding_tests {
    use super::*;
    use crate::diagnostic_builder::{
        load_policy, policy_bytes, BuilderPolicy, BUILDER_ID, POLICY_SCHEMA, REPOSITORY_ID,
    };
    use crate::diagnostic_lease::DiagnosticStage;
    use ed25519_dalek::{Signer, SigningKey};
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use std::process::{Child, Command, Stdio};
    use std::time::{Duration, Instant};

    struct Fixture {
        child: Child,
        root: PathBuf,
        temporary: tempfile::TempDir,
        state: DiagnosticState,
        policy: CheckedBuilderPolicy,
        process: ProcessRead,
        stdout: File,
        stderr: File,
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            self.child.stdin.take();
            self.child.wait().unwrap();
            fs::set_permissions(&self.root, fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
    impl Fixture {
        fn new() -> Self {
            // SYNTHETIC test signature/log. The real /bin/cat copy is a
            // non-product helper, not the signed hosted observer or provenance.
            let temporary = tempfile::tempdir().unwrap();
            let root = temporary.path().join("bundle");
            fs::create_dir(&root).unwrap();
            let binary = fs::read("/bin/cat").unwrap();
            let receipt: SignedBuild = serde_json::from_slice(include_bytes!(
                "../../../docs/testing/performance/0.74/local-runs/diagnostic-builder-hosted-37923423889/build-receipt-v1.json"
            )).unwrap();
            let mut statement = receipt.statement;
            let log = b"{\"reason\":\"compiler-artifact\",\"package_id\":\"path+file:///synthetic/tools/get-owner-scheduled-controls-074#get-owner-scheduled-controls-074@0.0.0\",\"target\":{\"name\":\"timing-controls-074\",\"kind\":[\"bin\"],\"crate_types\":[\"bin\"],\"src_path\":\"/synthetic/tools/get-owner-scheduled-controls-074/src/bin/timing_controls.rs\"},\"profile\":{\"opt_level\":\"3\",\"debug_assertions\":false,\"test\":false},\"features\":[],\"executable\":\"/synthetic/tools/get-owner-scheduled-controls-074/target/x86_64-unknown-linux-gnu/release/timing-controls-074\"}\n{\"reason\":\"build-finished\",\"success\":true}\n";
            statement.binary = ArtifactDigest::of(&binary);
            statement.build_log = ArtifactDigest::of(log);
            let key = SigningKey::from_bytes(&[74; 32]);
            let signed = receipt_bytes(&SignedBuild {
                signature_hex: crate::hex(
                    &key.sign(&signing_message(&statement).unwrap()).to_bytes(),
                ),
                statement: statement.clone(),
            })
            .unwrap();
            for (name, bytes) in [
                ("timing-controls-074", binary.as_slice()),
                ("build-log.jsonl", log.as_slice()),
                ("build-receipt-v1.json", signed.as_slice()),
                (
                    "Cargo.lock.root",
                    include_bytes!("../../../Cargo.lock").as_slice(),
                ),
                (
                    "Cargo.lock.observer",
                    include_bytes!("../../get-owner-scheduled-controls-074/Cargo.lock").as_slice(),
                ),
                (
                    "embedded.json",
                    include_bytes!(
                        "../../../docs/testing/performance/0.74/rental-pilot-draft/embedded.json"
                    )
                    .as_slice(),
                ),
                (
                    "direct.json",
                    include_bytes!(
                        "../../../docs/testing/performance/0.74/rental-pilot-draft/direct.json"
                    )
                    .as_slice(),
                ),
                (
                    "resp2.json",
                    include_bytes!(
                        "../../../docs/testing/performance/0.74/rental-pilot-draft/resp2.json"
                    )
                    .as_slice(),
                ),
                (
                    "resp3.json",
                    include_bytes!(
                        "../../../docs/testing/performance/0.74/rental-pilot-draft/resp3.json"
                    )
                    .as_slice(),
                ),
            ] {
                fs::write(root.join(name), bytes).unwrap();
                fs::set_permissions(
                    root.join(name),
                    fs::Permissions::from_mode(if name == "timing-controls-074" {
                        0o555
                    } else {
                        0o444
                    }),
                )
                .unwrap();
            }
            fs::set_permissions(&root, fs::Permissions::from_mode(0o555)).unwrap();
            let writer = |name: &str| {
                OpenOptions::new()
                    .create_new(true)
                    .append(true)
                    .mode(0o600)
                    .open(temporary.path().join(name))
                    .unwrap()
            };
            let mut child = Command::new(root.join("timing-controls-074"))
                .stdin(Stdio::piped())
                .stdout(writer("stdout"))
                .stderr(writer("stderr"))
                .spawn()
                .unwrap();
            child.stdin.as_mut().unwrap().write_all(b"ready\n").unwrap();
            let deadline = Instant::now() + Duration::from_secs(5);
            while fs::read(temporary.path().join("stdout")).unwrap() != b"ready\n" {
                assert!(Instant::now() < deadline, "helper handshake");
                assert!(child.try_wait().unwrap().is_none());
                std::thread::sleep(Duration::from_millis(2));
            }
            let process = crate::diagnostic_process::pin_owned_test_helper(child.id());
            let state = DiagnosticState {
                identity: DiagnosticIdentity {
                    lease_id: "a".repeat(64),
                    boot_id: process.observation().expected().boot_id.clone(),
                    binary_sha256: statement.binary.sha256,
                    build_provenance_sha256: sha256_hex(&signed),
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
            let controller = SigningKey::from_bytes(&[12; 32]).verifying_key();
            let bytes = policy_bytes(&BuilderPolicy {
                schema_version: POLICY_SCHEMA.into(),
                repository_id: REPOSITORY_ID,
                builder_id: BUILDER_ID.into(),
                builder_key_hex: crate::hex(key.verifying_key().as_bytes()),
                controller_key_hex: crate::hex(controller.as_bytes()),
            })
            .unwrap();
            let policy = load_policy(&bytes, &sha256_hex(&bytes), &controller).unwrap();
            let stdout = File::open(temporary.path().join("stdout")).unwrap();
            let stderr = File::open(temporary.path().join("stderr")).unwrap();
            Self {
                child,
                root,
                temporary,
                state,
                policy,
                process,
                stdout,
                stderr,
            }
        }
        fn material(&self) -> PinnedStartMaterial {
            // SAFETY: side-effect-free identity queries for explicit fixture ownership.
            let (uid, gid) = unsafe { (libc::getuid(), libc::getgid()) };
            prepare_fixture_start_material(&self.root, uid, gid, &self.state, &self.policy).unwrap()
        }
        fn named_outputs(&self) -> (PathBuf, crate::diagnostic_named_output::FixtureOutputRead) {
            let output_root = self.temporary.path().join("outputs");
            let lease = output_root.join(&self.state.identity.lease_id);
            let cell = lease.join("embedded");
            fs::create_dir_all(&cell).unwrap();
            for dir in [&output_root, &lease, &cell] {
                fs::set_permissions(dir, fs::Permissions::from_mode(0o700)).unwrap();
            }
            fs::rename(
                self.temporary.path().join("stdout"),
                cell.join("stdout.json"),
            )
            .unwrap();
            fs::rename(
                self.temporary.path().join("stderr"),
                cell.join("stderr.log"),
            )
            .unwrap();
            // SAFETY: read-only identity queries for explicit fixture ownership.
            let (uid, gid) = unsafe { (libc::geteuid(), libc::getegid()) };
            let outputs = crate::diagnostic_named_output::pin_fixture_outputs(
                &output_root,
                &self.state,
                uid,
                gid,
            )
            .unwrap();
            (cell, outputs)
        }
    }
    #[test]
    fn synthetic_material_origin_and_exact_execution_state_cannot_drift() {
        let f = Fixture::new();
        let mut material = f.material();
        assert!(!material.matches_production_state(&f.state));
        // Synthetic private policy flag ONLY. No fixed install or signature
        // origin is authenticated by flipping it; do not bind or execute here.
        material.bundle.fixture = false;
        assert!(material.matches_production_state(&f.state));
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
            assert!(!material.matches_production_state(&wrong), "field {field}");
        }
    }
    #[test]
    fn fixture_execution_entry_refuses_before_manager_and_latches_all_original_inputs() {
        use crate::diagnostic_live_execution::{pin_live_execution, ExecutionError};
        use crate::diagnostic_live_identity::unjoined_fixture_identity;
        use crate::diagnostic_manager::ManagerClient;
        use crate::diagnostic_named_output::fixture_output_for_refusal_test;
        use crate::diagnostic_tree::read_fixture_tree;
        for refused_input in 0..10 {
            let f = Fixture::new();
            let tree_root = f.temporary.path().join("tree");
            fs::create_dir(&tree_root).unwrap();
            fs::set_permissions(&tree_root, fs::Permissions::from_mode(0o700)).unwrap();
            for (name, bytes) in [
                ("cgroup.type", "domain\n"),
                ("cgroup.events", "populated 0\nfrozen 0\n"),
                ("cgroup.procs", ""),
            ] {
                fs::write(tree_root.join(name), bytes).unwrap();
                fs::set_permissions(tree_root.join(name), fs::Permissions::from_mode(0o600))
                    .unwrap();
            }
            // SAFETY: read-only credential queries for explicit fixture ownership.
            let (uid, gid) = unsafe { (libc::geteuid(), libc::getegid()) };
            let tree = read_fixture_tree(&tree_root, uid, gid).unwrap();
            let mut identity = unjoined_fixture_identity(&f.state, &f.process, &tree);
            let (cell, outputs) = f.named_outputs();
            let mut outputs = fixture_output_for_refusal_test(outputs);
            let mut material = f.material();
            if refused_input >= 4 {
                let io = material
                    .bind_asserted_process_io(&f.state, &f.process, &f.stdout, &f.stderr)
                    .unwrap();
                let failure = match refused_input {
                    4 => ExecutionError::Identity(
                        crate::diagnostic_live_identity::IdentityError::Invalid,
                    ),
                    5 => ExecutionError::Outputs(
                        crate::diagnostic_named_output::NamedOutputError::Invalid,
                    ),
                    6 => ExecutionError::Material(ArtifactError::Contents),
                    7 => ExecutionError::Io(ProcessError::NotLive),
                    8 => ExecutionError::Invalid,
                    _ => ExecutionError::Refused,
                };
                let expected = failure.to_string();
                let error = crate::diagnostic_live_execution::finish_fixture_execution_refusal(
                    &mut identity,
                    &mut outputs,
                    &mut material,
                    io,
                    failure,
                );
                assert_eq!(error.to_string(), expected);
                // Six error categories use the concrete finish path. Seven
                // sequence positions are tested separately; no live join here.
            }
            match refused_input {
                1 => identity.refuse(),
                2 => {
                    fs::set_permissions(
                        cell.join("stdout.json"),
                        fs::Permissions::from_mode(0o644),
                    )
                    .unwrap();
                    assert!(outputs.revalidate().is_err());
                    fs::set_permissions(
                        cell.join("stdout.json"),
                        fs::Permissions::from_mode(0o600),
                    )
                    .unwrap();
                }
                3 => {
                    let mut wrong = f.state.clone();
                    wrong.revision += 1;
                    assert!(material.revalidate_for(&wrong).is_err());
                }
                _ => {}
            }
            assert!(matches!(
                pin_live_execution(
                    &mut identity,
                    &mut outputs,
                    &mut material,
                    &mut ManagerClient::default()
                ),
                Err(ExecutionError::Invalid | ExecutionError::Refused)
            ));
            assert!(identity.is_refused());
            assert!(outputs.is_refused());
            assert!(material.is_refused());
            assert!(matches!(
                pin_live_execution(
                    &mut identity,
                    &mut outputs,
                    &mut material,
                    &mut ManagerClient::default()
                ),
                Err(ExecutionError::Refused)
            ));
            f.process.revalidate().unwrap(); // Only the owned non-product helper remains alive.
            assert!(material.revalidate_for(&f.state).is_err());
        }
    }
    #[test]
    fn fixture_named_process_binding_refuses_replaced_path_while_inode_only_accepts() {
        let f = Fixture::new();
        let mut material = f.material();
        let mut inode_only = material
            .bind_asserted_process_io(&f.state, &f.process, &f.stdout, &f.stderr)
            .unwrap();
        let (cell, mut outputs) = f.named_outputs();
        let mut composed = outputs
            .bind_fixture_process_io(&mut material, &f.state, &f.process)
            .unwrap();
        assert!(composed.is_fixture());
        composed.revalidate().unwrap();
        let path = cell.join("stdout.json");
        let displaced = f.temporary.path().join("displaced-stdout");
        fs::rename(&path, &displaced).unwrap();
        OpenOptions::new()
            .create_new(true)
            .append(true)
            .mode(0o600)
            .open(&path)
            .unwrap();
        inode_only.revalidate().unwrap();
        assert!(composed.revalidate().is_err());
        assert!(composed.is_refused());
        fs::remove_file(&path).unwrap();
        fs::rename(&displaced, &path).unwrap();
        assert!(composed.revalidate().is_err());
        drop(composed);
        assert!(outputs
            .bind_fixture_process_io(&mut material, &f.state, &f.process)
            .is_err());
    }
    #[test]
    fn fixture_named_binding_refuses_state_mismatch_and_process_exit() {
        let f = Fixture::new();
        let (_, mut outputs) = f.named_outputs();
        let mut material = f.material();
        let mut wrong = f.state.clone();
        wrong.revision += 1;
        assert!(outputs
            .bind_fixture_process_io(&mut material, &wrong, &f.process)
            .is_err());
        assert!(outputs.is_refused());
        assert!(outputs
            .bind_fixture_process_io(&mut material, &f.state, &f.process)
            .is_err());

        let mut f = Fixture::new();
        let (_, mut outputs) = f.named_outputs();
        let mut material = f.material();
        let mut composed = outputs
            .bind_fixture_process_io(&mut material, &f.state, &f.process)
            .unwrap();
        f.child.stdin.take();
        f.child.wait().unwrap();
        assert!(composed.revalidate().is_err());
        assert!(composed.is_refused());
        assert!(composed.revalidate().is_err());
    }
    #[test]
    fn fixture_named_binding_propagates_material_refusal_without_new_capability() {
        let f = Fixture::new();
        let (_, mut outputs) = f.named_outputs();
        let mut material = f.material();
        let mut wrong = f.state.clone();
        wrong.revision += 1;
        assert!(material.revalidate_for(&wrong).is_err());
        assert!(matches!(
            outputs.bind_fixture_process_io(&mut material, &f.state, &f.process),
            Err(crate::diagnostic_named_output::NamedOutputError::Binding(_))
        ));
        assert!(outputs.is_refused());
    }
    #[test]
    fn checked_material_io_binding_is_fixture_explicit_and_refuses_state_or_process_drift() {
        let f = Fixture::new();
        let mut material = f.material();
        assert!(material.is_fixture());
        material
            .bind_asserted_process_io(&f.state, &f.process, &f.stdout, &f.stderr)
            .unwrap()
            .revalidate()
            .unwrap();
        let mut wrong_state = f.state.clone();
        wrong_state.revision += 1;
        assert!(matches!(
            material.bind_asserted_process_io(&wrong_state, &f.process, &f.stdout, &f.stderr),
            Err(ProcessIoBindingError::Material(_))
        ));
        assert!(material
            .bind_asserted_process_io(&f.state, &f.process, &f.stdout, &f.stderr)
            .is_err());
        let mut material = f.material();
        assert!(matches!(
            material.bind_asserted_process_io(&f.state, &f.process, &f.stderr, &f.stdout),
            Err(ProcessIoBindingError::Process(_))
        ));
        assert!(material
            .bind_asserted_process_io(&f.state, &f.process, &f.stdout, &f.stderr)
            .is_err());
        // The output location is an explicit temporary caller assertion.
        assert!(f.temporary.path().exists());
    }
}
