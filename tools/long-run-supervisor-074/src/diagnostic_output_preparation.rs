//! Cooperative fixture preparation only, never a production write or launch route.

use crate::diagnostic_lease::{
    DiagnosticClock, DiagnosticStage, DiagnosticState, CONTROLLER_LOSS_SECONDS, TOTAL_SECONDS,
};
use crate::diagnostic_named_output::{pin_fixture_outputs, FixtureOutputRead, NamedOutputError};
use crate::diagnostic_unit::diagnostic_start_intent;
use fs2::FileExt;
use std::ffi::CString;
use std::fs::{self, File, Metadata, OpenOptions};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use thiserror::Error;
const NS: u64 = 1_000_000_000;
const OUTPUT_ROOT: &str = "diagnostic-fixture-outputs";

#[derive(Debug, Error)]
pub enum PreparationError {
    #[error("fixture preparation path, identity, state or asserted clock refused")]
    Invalid,
    #[error("fixture host execution fence is busy")]
    Busy,
    #[error("fixture cell or conflicting campaign/context/pending document exists")]
    Conflict,
    #[error("fixture preparation I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("fixture diagnostic state failed: {0}")]
    State(#[from] crate::diagnostic_lease::DiagnosticError),
    #[error("fixture named-output validation failed: {0}")]
    Output(#[from] NamedOutputError),
}

#[derive(Clone, PartialEq, Eq)]
struct Stamp {
    dev: u64,
    ino: u64,
    uid: u32,
    gid: u32,
    mode: u32,
}
impl Stamp {
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
    stamp: Stamp,
}
impl Directory {
    fn checked(file: File, uid: u32, gid: u32) -> Result<Self, PreparationError> {
        let m = file.metadata()?;
        if !m.is_dir() || m.mode() & 0o7777 != 0o700 || m.uid() != uid || m.gid() != gid {
            return Err(PreparationError::Invalid);
        }
        Ok(Self {
            file,
            stamp: Stamp::of(&m),
        })
    }
    fn revalidate_at(&self, parent: &File, name: &str) -> Result<(), PreparationError> {
        let current = open_at(parent, name, libc::O_RDONLY | libc::O_DIRECTORY, 0)?;
        if Stamp::of(&self.file.metadata()?) != self.stamp
            || Stamp::of(&current.metadata()?) != self.stamp
        {
            return Err(PreparationError::Invalid);
        }
        Ok(())
    }
}

/// Retained root and exact legacy lock object. Cooperative fixture authority
/// only: this is not root-owned production ancestry or signed IPC admission.
struct FixtureFence {
    path: PathBuf,
    root: Directory,
    lock: File,
    lock_stamp: Stamp,
    uid: u32,
    gid: u32,
    unlocked: bool,
}
impl FixtureFence {
    fn acquire(path: &Path) -> Result<Self, PreparationError> {
        // Kernel observations, not caller UID/GID assertions or NSS enrollment.
        let uid = unsafe { libc::geteuid() };
        let gid = unsafe { libc::getegid() };
        if uid == 0 || gid == 0 {
            return Err(PreparationError::Invalid);
        }
        let root = Directory::checked(open_root(path)?, uid, gid)?;
        // Reservation must already have created the lock. Never replace it.
        let lock = open_at(
            &root.file,
            crate::host_execution::HOST_EXECUTION_LOCK_NAME,
            libc::O_RDWR,
            0,
        )?;
        let m = lock.metadata()?;
        if !m.is_file()
            || m.nlink() != 1
            || m.uid() != uid
            || m.gid() != gid
            || m.mode() & 0o7022 != 0
        {
            return Err(PreparationError::Invalid);
        }
        FileExt::try_lock_exclusive(&lock).map_err(|e| {
            if e.kind() == std::io::ErrorKind::WouldBlock {
                PreparationError::Busy
            } else {
                PreparationError::Io(e)
            }
        })?;
        let fence = Self {
            path: path.to_owned(),
            root,
            lock,
            lock_stamp: Stamp::of(&m),
            uid,
            gid,
            unlocked: false,
        };
        fence.revalidate()?;
        Ok(fence)
    }
    fn revalidate(&self) -> Result<(), PreparationError> {
        let root = open_root(&self.path)?;
        if Stamp::of(&root.metadata()?) != self.root.stamp
            || Stamp::of(&self.root.file.metadata()?) != self.root.stamp
        {
            return Err(PreparationError::Invalid);
        }
        let named = open_at(
            &self.root.file,
            crate::host_execution::HOST_EXECUTION_LOCK_NAME,
            libc::O_RDWR,
            0,
        )?;
        for file in [&self.lock, &named] {
            let m = file.metadata()?;
            if Stamp::of(&m) != self.lock_stamp || m.nlink() != 1 {
                return Err(PreparationError::Invalid);
            }
        }
        Ok(())
    }
    fn release(mut self) -> Result<(), PreparationError> {
        FileExt::unlock(&self.lock)?;
        self.unlocked = true;
        Ok(())
    }
}
impl Drop for FixtureFence {
    fn drop(&mut self) {
        if !self.unlocked {
            // Error exits also release the shared OFD lock, not just this FD.
            let _ = FileExt::unlock(&self.lock);
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PreparationStage {
    CellCreated,
    StdoutSynced,
    StderrSynced,
}

/// Prepare fixed empty streams for the exact reserved temporary fixture cell.
/// No production guard, stream FD, launch, automatic replay or cleanup is exposed.
pub fn prepare_fixture_outputs(
    root: &Path,
    state: &DiagnosticState,
    now: &DiagnosticClock,
) -> Result<FixtureOutputRead, PreparationError> {
    prepare_inner(root, state, now, |_| Ok(()))
}

fn prepare_inner(
    root: &Path,
    state: &DiagnosticState,
    now: &DiagnosticClock,
    mut checkpoint: impl FnMut(PreparationStage) -> Result<(), PreparationError>,
) -> Result<FixtureOutputRead, PreparationError> {
    let intent = diagnostic_start_intent(state).map_err(|_| PreparationError::Invalid)?;
    if root.as_os_str().as_bytes().len()
        + OUTPUT_ROOT.len()
        + intent.lease_id.len()
        + intent.surface.len()
        + "stdout.json".len()
        + 4
        > 4096
        || state.stage != DiagnosticStage::Reserved
        || now.boot_id != state.identity.boot_id
        || now.monotonic_ns < state.last_observed_monotonic_ns
        || now.monotonic_ns - state.controller_monotonic_ns >= CONTROLLER_LOSS_SECONDS * NS
        || now.monotonic_ns - state.reserved_monotonic_ns >= TOTAL_SECONDS * NS
    {
        return Err(PreparationError::Invalid);
    }
    let fence = FixtureFence::acquire(root)?;
    check_state(&fence, state)?;
    let outputs = mkdir_private(&fence.root.file, OUTPUT_ROOT, false, fence.uid, fence.gid)?;
    let lease = mkdir_private(&outputs.file, &intent.lease_id, false, fence.uid, fence.gid)?;
    check_state(&fence, state)?;
    outputs.revalidate_at(&fence.root.file, OUTPUT_ROOT)?;
    lease.revalidate_at(&outputs.file, &intent.lease_id)?;
    let cell = mkdir_private(&lease.file, &intent.surface, true, fence.uid, fence.gid)?;
    checkpoint(PreparationStage::CellCreated)?;
    let revalidate = || -> Result<(), PreparationError> {
        check_state(&fence, state)?;
        outputs.revalidate_at(&fence.root.file, OUTPUT_ROOT)?;
        lease.revalidate_at(&outputs.file, &intent.lease_id)?;
        cell.revalidate_at(&lease.file, &intent.surface)
    };
    revalidate()?;
    new_stream(&cell.file, "stdout.json", fence.uid, fence.gid)?;
    checkpoint(PreparationStage::StdoutSynced)?;
    revalidate()?;
    new_stream(&cell.file, "stderr.log", fence.uid, fence.gid)?;
    checkpoint(PreparationStage::StderrSynced)?;
    revalidate()?;
    let mut guard = pin_fixture_outputs(&root.join(OUTPUT_ROOT), state, fence.uid, fence.gid)?;
    guard.revalidate()?;
    revalidate()?;
    // The fence remains held through the final named-output and state checks.
    fence.release()?;
    Ok(guard)
}

fn check_state(fence: &FixtureFence, expected: &DiagnosticState) -> Result<(), PreparationError> {
    fence.revalidate()?;
    for name in [
        crate::host_execution::ACTIVE_CAMPAIGN_NAME,
        ".active-diagnostic.pending",
        ".diagnostic-requests.pending",
    ] {
        require_absent(&fence.path.join(name))?;
    }
    let parent = fence.path.parent().ok_or(PreparationError::Invalid)?;
    for name in [
        "campaign-lifecycle-smoke-v1.json",
        "controller-loss-smoke-v1.json",
    ] {
        require_absent(&parent.join(name))?;
    }
    let document = open_at(
        &fence.root.file,
        crate::diagnostic_lease::ACTIVE_DIAGNOSTIC_NAME,
        libc::O_RDONLY,
        0,
    )?;
    let m = document.metadata()?;
    if !m.is_file()
        || m.nlink() != 1
        || m.mode() & 0o7777 != 0o600
        || m.uid() != fence.uid
        || m.gid() != fence.gid
    {
        return Err(PreparationError::Invalid);
    }
    if crate::diagnostic_lease::read_active(&fence.path)?.as_ref() != Some(expected) {
        return Err(PreparationError::Invalid);
    }
    fence.revalidate()
}

fn require_absent(path: &Path) -> Result<(), PreparationError> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
        Ok(_) => Err(PreparationError::Conflict),
    }
}

fn open_root(path: &Path) -> Result<File, PreparationError> {
    let raw = path.as_os_str().as_bytes();
    let parts: Vec<_> = raw.split(|b| *b == b'/').skip(1).collect();
    if !path.is_absolute()
        || raw.len() > 4096
        || parts.is_empty()
        || parts.len() > 60
        || parts
            .iter()
            .any(|p| p.is_empty() || *p == b"." || *p == b"..")
        || path.starts_with("/var/lib/hydracache-performance")
        || path.starts_with("/opt/hydracache-performance")
    {
        return Err(PreparationError::Invalid);
    }
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open("/")?;
    for part in parts {
        let name = std::str::from_utf8(part).map_err(|_| PreparationError::Invalid)?;
        file = open_at(&file, name, libc::O_RDONLY | libc::O_DIRECTORY, 0)?;
    }
    Ok(file)
}

fn open_at(parent: &File, name: &str, flags: i32, mode: u32) -> Result<File, PreparationError> {
    let name = CString::new(name).map_err(|_| PreparationError::Invalid)?;
    // Every caller uses one validated component; no path traversal or following.
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

fn mkdir_private(
    parent: &File,
    name: &str,
    exclusive: bool,
    uid: u32,
    gid: u32,
) -> Result<Directory, PreparationError> {
    let c = CString::new(name).map_err(|_| PreparationError::Invalid)?;
    if unsafe { libc::mkdirat(parent.as_raw_fd(), c.as_ptr(), 0o700) } != 0 {
        let e = std::io::Error::last_os_error();
        if e.kind() != std::io::ErrorKind::AlreadyExists {
            return Err(e.into());
        }
        if exclusive {
            return Err(PreparationError::Conflict);
        }
    }
    let directory = Directory::checked(
        open_at(parent, name, libc::O_RDONLY | libc::O_DIRECTORY, 0)?,
        uid,
        gid,
    )?;
    directory.file.sync_all()?;
    parent.sync_all()?;
    Ok(directory)
}

fn new_stream(parent: &File, name: &str, uid: u32, gid: u32) -> Result<(), PreparationError> {
    let file = open_at(
        parent,
        name,
        libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL,
        0o600,
    )?;
    let m = file.metadata()?;
    if !m.is_file()
        || m.nlink() != 1
        || m.len() != 0
        || m.mode() & 0o7777 != 0o600
        || m.uid() != uid
        || m.gid() != gid
    {
        return Err(PreparationError::Invalid);
    }
    file.sync_all()?;
    parent.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic_lease::{DiagnosticCoordinator, DiagnosticIdentity};
    use crate::{canonical_json, sha256_hex};
    use std::fs;
    use std::os::unix::fs::{symlink, MetadataExt, PermissionsExt};
    use std::sync::{Arc, Barrier};

    struct Fixture {
        _temp: tempfile::TempDir,
        root: std::path::PathBuf,
        state: DiagnosticState,
        clock: DiagnosticClock,
    }
    impl Fixture {
        fn new(cell: usize) -> Self {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path().join("campaigns");
            fs::create_dir(&root).unwrap();
            fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
            let clock = DiagnosticClock {
                boot_id: "12345678-1234-1234-1234-123456789abc".into(),
                monotonic_ns: 1_000_000_000,
            };
            let identity = DiagnosticIdentity {
                lease_id: "a".repeat(64),
                boot_id: clock.boot_id.clone(),
                binary_sha256: "b".repeat(64),
                build_provenance_sha256: "c".repeat(64),
            };
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(root.join(".host-execution.lock"))
                .unwrap();
            let fence = FixtureFence::acquire(&root).unwrap();
            DiagnosticCoordinator::reserve_fenced(&root, identity, &clock).unwrap();
            let mut state = crate::diagnostic_lease::read_active(&root)
                .unwrap()
                .unwrap();
            fence.release().unwrap();
            state.completed_cells = cell;
            let result = Self {
                _temp: temp,
                root,
                state,
                clock,
            };
            result.persist();
            result
        }
        // Synthetic canonical state, no backend or measured process is started.
        fn persist(&self) {
            let envelope = serde_json::json!({ "schema_version": 1, "state": self.state,
                "state_sha256": sha256_hex(&canonical_json(&self.state).unwrap()) });
            let mut bytes = canonical_json(&envelope).unwrap();
            bytes.push(b'\n');
            fs::write(self.root.join("active-diagnostic.json"), bytes).unwrap();
        }
        fn cell(&self) -> std::path::PathBuf {
            self.root
                .join(OUTPUT_ROOT)
                .join(&self.state.identity.lease_id)
                .join(crate::diagnostic_lease::SURFACES[self.state.completed_cells])
        }
    }

    #[test]
    fn fixture_preparation_four_cells_keep_state_and_return_readonly_guards() {
        for cell in 0..4 {
            let f = Fixture::new(cell);
            let before = fs::read(f.root.join("active-diagnostic.json")).unwrap();
            let mut read = prepare_fixture_outputs(&f.root, &f.state, &f.clock).unwrap();
            read.revalidate().unwrap();
            assert_eq!(
                fs::read(f.root.join("active-diagnostic.json")).unwrap(),
                before
            );
            assert_eq!(fs::metadata(f.cell()).unwrap().mode() & 0o7777, 0o700);
            for name in ["stdout.json", "stderr.log"] {
                let m = fs::metadata(f.cell().join(name)).unwrap();
                assert_eq!(m.mode() & 0o7777, 0o600);
                assert_eq!(m.len(), 0);
                assert_eq!(m.nlink(), 1);
                assert_eq!(m.uid(), unsafe { libc::geteuid() });
                assert_eq!(m.gid(), unsafe { libc::getegid() });
            }
            fs::write(f.cell().join("stdout.json"), b"retained").unwrap();
            assert!(matches!(
                prepare_fixture_outputs(&f.root, &f.state, &f.clock),
                Err(PreparationError::Conflict)
            ));
            assert_eq!(fs::read(f.cell().join("stdout.json")).unwrap(), b"retained");
        }
    }

    #[test]
    fn fixture_preparation_exact_state_and_asserted_clock_refuse_before_outputs() {
        let f = Fixture::new(0);
        for field in 0..6 {
            let mut wrong = f.state.clone();
            match field {
                0 => wrong.revision += 1,
                1 => wrong.completed_cells = 1,
                2 => wrong.identity.binary_sha256 = "d".repeat(64),
                3 => wrong.controller_monotonic_ns += 1,
                4 => wrong.stage = DiagnosticStage::Starting,
                _ => wrong.admission_allowed = true,
            }
            assert!(prepare_fixture_outputs(&f.root, &wrong, &f.clock).is_err());
        }
        for field in 0..4 {
            let mut clock = f.clock.clone();
            match field {
                0 => clock.boot_id = "22345678-1234-1234-1234-123456789abc".into(),
                1 => clock.monotonic_ns -= 1,
                2 => clock.monotonic_ns += CONTROLLER_LOSS_SECONDS * NS,
                _ => clock.monotonic_ns += TOTAL_SECONDS * NS,
            }
            assert!(prepare_fixture_outputs(&f.root, &f.state, &clock).is_err());
        }
        assert!(!f.root.join(OUTPUT_ROOT).exists());
        let mut near = f.clock.clone();
        near.monotonic_ns += CONTROLLER_LOSS_SECONDS * NS - 1;
        prepare_fixture_outputs(&f.root, &f.state, &near).unwrap();
    }

    #[test]
    fn fixture_preparation_total_deadline_refuses_despite_fresh_controller() {
        let mut f = Fixture::new(0);
        f.state.controller_monotonic_ns += (TOTAL_SECONDS - 1) * NS;
        f.state.last_observed_monotonic_ns = f.state.controller_monotonic_ns;
        f.persist();
        f.clock.monotonic_ns += TOTAL_SECONDS * NS;
        assert!(prepare_fixture_outputs(&f.root, &f.state, &f.clock).is_err());
        assert!(!f.root.join(OUTPUT_ROOT).exists());
    }

    #[test]
    fn fixture_preparation_serial_cells_reuse_private_parents_without_overwrite() {
        let mut f = Fixture::new(0);
        let mut previous = Vec::new();
        for cell in 0..4 {
            f.state.completed_cells = cell;
            f.state.revision += 1;
            f.persist();
            prepare_fixture_outputs(&f.root, &f.state, &f.clock).unwrap();
            let path = f.cell().join("stdout.json");
            fs::write(&path, format!("retained-cell-{cell}")).unwrap();
            previous.push(path);
            for (index, old) in previous.iter().enumerate() {
                assert_eq!(
                    fs::read(old).unwrap(),
                    format!("retained-cell-{index}").as_bytes()
                );
            }
        }
    }

    #[test]
    fn fixture_preparation_campaign_pending_and_lifecycle_contexts_refuse() {
        for name in [
            "active-campaign",
            ".active-diagnostic.pending",
            ".diagnostic-requests.pending",
            "campaign-lifecycle-smoke-v1.json",
            "controller-loss-smoke-v1.json",
        ] {
            let f = Fixture::new(0);
            let path = if name.ends_with("smoke-v1.json") {
                f.root.parent().unwrap().join(name)
            } else {
                f.root.join(name)
            };
            fs::write(path, b"retained-conflict").unwrap();
            assert!(
                prepare_fixture_outputs(&f.root, &f.state, &f.clock).is_err(),
                "{name}"
            );
            assert!(!f.root.join(OUTPUT_ROOT).exists());
        }
    }

    #[test]
    fn fixture_preparation_shares_coordinator_host_fence() {
        let f = Fixture::new(0);
        let (_root, fence) = crate::host_execution::lock_host_root(&f.root).unwrap();
        assert!(matches!(
            prepare_fixture_outputs(&f.root, &f.state, &f.clock),
            Err(PreparationError::Busy)
        ));
        assert!(!f.root.join(OUTPUT_ROOT).exists());
        FileExt::unlock(&fence).unwrap();
        drop(fence);
        prepare_inner(&f.root, &f.state, &f.clock, |stage| {
            if stage == PreparationStage::CellCreated {
                let c = DiagnosticCoordinator::recover(&f.root, &f.state.identity.lease_id);
                assert!(c.is_err(), "coordinator must not reacquire this lock");
            }
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn fixture_fence_drop_releases_lock_with_retained_duplicate_descriptor() {
        let f = Fixture::new(0);
        let fence = FixtureFence::acquire(&f.root).unwrap();
        // Same open file description, as with an inherited descriptor; no fork.
        let duplicate = fence.lock.try_clone().unwrap();
        drop(fence);
        let next = FixtureFence::acquire(&f.root);
        let released = next.is_ok();
        drop(next);
        drop(duplicate);
        assert!(released, "close alone must not retain the cooperative lock");
    }

    #[test]
    fn fixture_preparation_concurrent_duplicates_create_exactly_one_cell() {
        let f = Arc::new(Fixture::new(0));
        let barrier = Arc::new(Barrier::new(8));
        let results: Vec<_> = (0..8)
            .map(|_| {
                let f = Arc::clone(&f);
                let b = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    b.wait();
                    match prepare_fixture_outputs(&f.root, &f.state, &f.clock) {
                        Ok(_) => true,
                        Err(PreparationError::Busy | PreparationError::Conflict) => false,
                        Err(e) => panic!("unexpected refusal: {e}"),
                    }
                })
            })
            .collect();
        assert_eq!(
            results
                .into_iter()
                .map(|t| t.join().unwrap())
                .filter(|ok| *ok)
                .count(),
            1
        );
        assert_eq!(fs::read_dir(f.cell()).unwrap().count(), 2);
    }

    #[test]
    fn fixture_preparation_partial_faults_retain_without_replay_or_cleanup() {
        for (stage, count) in [
            (PreparationStage::CellCreated, 0),
            (PreparationStage::StdoutSynced, 1),
            (PreparationStage::StderrSynced, 2),
        ] {
            let f = Fixture::new(0);
            assert!(prepare_inner(&f.root, &f.state, &f.clock, |at| {
                if at == stage {
                    Err(PreparationError::Invalid)
                } else {
                    Ok(())
                }
            })
            .is_err());
            assert_eq!(fs::read_dir(f.cell()).unwrap().count(), count);
            if count > 0 {
                fs::write(f.cell().join("stdout.json"), b"partial-retained").unwrap();
            }
            assert!(matches!(
                prepare_fixture_outputs(&f.root, &f.state, &f.clock),
                Err(PreparationError::Conflict)
            ));
            assert_eq!(fs::read_dir(f.cell()).unwrap().count(), count);
            if count > 0 {
                assert_eq!(
                    fs::read(f.cell().join("stdout.json")).unwrap(),
                    b"partial-retained"
                );
            }
        }
    }

    #[test]
    fn fixture_preparation_links_and_unsafe_modes_never_redirect_writes() {
        for case in 0..6 {
            let f = Fixture::new(0);
            let outside = f._temp.path().join("outside");
            fs::create_dir(&outside).unwrap();
            match case {
                0 => symlink(&outside, f.root.join(OUTPUT_ROOT)).unwrap(),
                1 => fs::set_permissions(&f.root, fs::Permissions::from_mode(0o770)).unwrap(),
                2 => fs::hard_link(f.root.join(".host-execution.lock"), outside.join("lock"))
                    .unwrap(),
                3 => {
                    fs::remove_file(f.root.join(".host-execution.lock")).unwrap();
                    symlink(outside.join("lock"), f.root.join(".host-execution.lock")).unwrap();
                }
                4 => {
                    fs::create_dir(f.root.join(OUTPUT_ROOT)).unwrap();
                    fs::set_permissions(
                        f.root.join(OUTPUT_ROOT),
                        fs::Permissions::from_mode(0o700),
                    )
                    .unwrap();
                    symlink(
                        &outside,
                        f.root.join(OUTPUT_ROOT).join(&f.state.identity.lease_id),
                    )
                    .unwrap();
                }
                _ => {
                    fs::remove_file(f.root.join(".host-execution.lock")).unwrap();
                    let name =
                        CString::new(f.root.join(".host-execution.lock").as_os_str().as_bytes())
                            .unwrap();
                    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
                }
            }
            assert!(prepare_fixture_outputs(&f.root, &f.state, &f.clock).is_err());
            assert!(!outside.join("stdout.json").exists());
        }
    }

    #[test]
    fn fixture_preparation_directory_and_lock_replacement_refuse_after_cell() {
        for replacement in ["lock", "root", "outputs", "lease", "cell"] {
            let f = Fixture::new(0);
            let displaced = f._temp.path().join("displaced");
            assert!(prepare_inner(&f.root, &f.state, &f.clock, |stage| {
                if stage == PreparationStage::CellCreated {
                    if replacement == "lock" {
                        fs::rename(f.root.join(".host-execution.lock"), &displaced).unwrap();
                        fs::write(f.root.join(".host-execution.lock"), b"").unwrap();
                    } else {
                        let path = match replacement {
                            "root" => f.root.clone(),
                            "outputs" => f.root.join(OUTPUT_ROOT),
                            "lease" => f.root.join(OUTPUT_ROOT).join(&f.state.identity.lease_id),
                            _ => f.cell(),
                        };
                        fs::rename(&path, &displaced).unwrap();
                        fs::create_dir(&path).unwrap();
                        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
                    }
                }
                Ok(())
            })
            .is_err());
            let cell = match replacement {
                "lock" => f.cell(),
                "root" => displaced
                    .join(OUTPUT_ROOT)
                    .join(&f.state.identity.lease_id)
                    .join("embedded"),
                "outputs" => displaced.join(&f.state.identity.lease_id).join("embedded"),
                "lease" => displaced.join("embedded"),
                _ => displaced,
            };
            assert_eq!(fs::read_dir(cell).unwrap().count(), 0);
        }
    }

    #[test]
    fn fixture_preparation_changed_state_after_stdout_retains_incomplete_cell() {
        let mut f = Fixture::new(0);
        let old = f.state.clone();
        f.state.revision += 1;
        assert!(prepare_inner(&f.root, &old, &f.clock, |stage| {
            if stage == PreparationStage::StdoutSynced {
                f.persist();
            }
            Ok(())
        })
        .is_err());
        assert_eq!(fs::read_dir(f.cell()).unwrap().count(), 1);
        assert!(prepare_fixture_outputs(&f.root, &old, &f.clock).is_err());
        assert!(matches!(
            prepare_fixture_outputs(&f.root, &f.state, &f.clock),
            Err(PreparationError::Conflict)
        ));
        assert_eq!(fs::read_dir(f.cell()).unwrap().count(), 1);
    }

    #[test]
    fn fixture_prepared_guard_refuses_substitution_and_stays_refused() {
        let f = Fixture::new(0);
        let mut guard = prepare_fixture_outputs(&f.root, &f.state, &f.clock).unwrap();
        let path = f.cell().join("stdout.json");
        fs::rename(&path, f._temp.path().join("old-stdout")).unwrap();
        fs::write(&path, b"").unwrap();
        assert!(guard.revalidate().is_err());
        assert!(guard.is_refused());
        assert!(guard.revalidate().is_err());
    }

    #[test]
    fn fixture_preparation_production_and_noncanonical_paths_refuse() {
        let f = Fixture::new(0);
        for path in [
            "/var/lib/hydracache-performance/campaigns",
            "/opt/hydracache-performance/anything",
            "relative",
            "/tmp/../tmp",
            "/tmp//fixture",
            "/tmp/./fixture",
            "/tmp/fixture/",
        ] {
            assert!(prepare_fixture_outputs(Path::new(path), &f.state, &f.clock).is_err());
        }
        let alias = f._temp.path().join("alias");
        symlink(&f.root, &alias).unwrap();
        assert!(prepare_fixture_outputs(&alias, &f.state, &f.clock).is_err());
    }
}
