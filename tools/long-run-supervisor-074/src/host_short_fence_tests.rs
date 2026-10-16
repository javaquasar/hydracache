//! Bounded owned-fork controls; never a workload or production authority.
use super::*;
use crate::diagnostic_ipc::*;
use crate::diagnostic_lease::*;
use ed25519_dalek::{Signer, SigningKey};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::fs::MetadataExt;
use std::rc::Rc;

type Hook = Box<dyn FnOnce(&File)>;
thread_local! {
    static CHECKPOINT: RefCell<Option<Hook>> = const { RefCell::new(None) };
}
pub(super) fn after_acquire(file: &File) {
    let hook = CHECKPOINT.with(|slot| slot.borrow_mut().take());
    if let Some(hook) = hook {
        hook(file);
    }
}
struct Scope;
impl Drop for Scope {
    fn drop(&mut self) {
        CHECKPOINT.with(|slot| slot.borrow_mut().take());
    }
}
fn with_inherited<R>(run: impl FnOnce() -> R) -> (R, Inherited) {
    let child = Rc::new(RefCell::new(None));
    let out = child.clone();
    CHECKPOINT.with(|slot| {
        assert!(slot.borrow().is_none());
        *slot.borrow_mut() = Some(Box::new(move |file| {
            *out.borrow_mut() = Some(Inherited::fork(file));
        }));
    });
    let _scope = Scope;
    let result = run();
    let child = child.borrow_mut().take().expect("checkpoint did not run");
    (result, child)
}
fn pipe() -> (OwnedFd, OwnedFd) {
    let mut fds = [-1; 2];
    assert_eq!(unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) }, 0);
    unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) }
}
fn ready(fd: i32) -> bool {
    let mut p = libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };
    unsafe { libc::poll(&mut p, 1, 5000) == 1 && p.revents & libc::POLLIN != 0 }
}
// Only raw syscalls/pure stack operations run after fork. No allocation, Rust
// destructor, mutex, assertion, formatting, unwinding or test-harness callback.
unsafe fn child(lock: i32, command: i32, reply: i32, keep: [i32; 3], parent: i32) -> ! {
    if unsafe {
        libc::syscall(
            libc::SYS_prctl,
            libc::PR_SET_PDEATHSIG,
            libc::SIGKILL,
            0,
            0,
            0,
        )
    } != 0
        || unsafe { libc::getppid() } != parent
    {
        unsafe { libc::_exit(70) };
    }
    let mut first = 3u32;
    for fd in keep {
        if first < fd as u32
            && unsafe { libc::syscall(libc::SYS_close_range, first, fd as u32 - 1, 0u32) } != 0
        {
            unsafe { libc::_exit(71) };
        }
        first = fd as u32 + 1;
    }
    let flags = unsafe { libc::fcntl(lock, libc::F_GETFD) };
    if unsafe { libc::syscall(libc::SYS_close_range, first, u32::MAX, 0u32) } != 0
        || flags < 0
        || flags & libc::FD_CLOEXEC == 0
        || unsafe { libc::write(reply, b"R".as_ptr().cast(), 1) } != 1
    {
        unsafe { libc::_exit(72) };
    }
    let mut byte = 0u8;
    let mut p = libc::pollfd {
        fd: command,
        events: libc::POLLIN,
        revents: 0,
    };
    if unsafe { libc::poll(&mut p, 1, 5000) } != 1
        || unsafe { libc::read(command, (&mut byte as *mut u8).cast(), 1) } != 1
        || byte != b'U'
        || unsafe { libc::flock(lock, libc::LOCK_UN) } != 0
        || unsafe { libc::write(reply, b"U".as_ptr().cast(), 1) } != 1
    {
        unsafe { libc::_exit(73) };
    }
    if unsafe { libc::poll(&mut p, 1, 5000) } != 1
        || unsafe { libc::read(command, (&mut byte as *mut u8).cast(), 1) } != 1
        || byte != b'Q'
    {
        unsafe { libc::_exit(74) };
    }
    unsafe { libc::_exit(0) };
}
struct Inherited {
    pid: i32,
    pidfd: Option<OwnedFd>,
    command: OwnedFd,
    reply: OwnedFd,
    reaped: bool,
}
impl Inherited {
    fn fork(file: &File) -> Self {
        let lock = file.as_raw_fd();
        assert!(lock >= 3);
        let flags = unsafe { libc::fcntl(lock, libc::F_GETFD) };
        assert!(flags >= 0 && flags & libc::FD_CLOEXEC != 0);
        let (read, write) = pipe();
        let (reply, send) = pipe();
        let mut keep = [lock, read.as_raw_fd(), send.as_raw_fd()];
        keep.sort_unstable();
        assert!(keep.iter().all(|fd| *fd >= 3));
        let parent = unsafe { libc::getpid() };
        let pid = unsafe { libc::fork() };
        assert!(pid >= 0, "owned fork failed");
        if pid == 0 {
            unsafe { child(lock, read.as_raw_fd(), send.as_raw_fd(), keep, parent) };
        }
        let mut owned = Self {
            pid,
            pidfd: None,
            command: write,
            reply,
            reaped: false,
        };
        drop(read);
        drop(send);
        let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) };
        assert!(fd >= 0, "exact owned child pidfd unavailable");
        owned.pidfd = Some(unsafe { OwnedFd::from_raw_fd(fd as i32) });
        owned.ack(b'R');
        owned
    }
    fn ack(&self, expected: u8) {
        assert!(
            ready(self.reply.as_raw_fd()),
            "owned child acknowledgement absent"
        );
        let mut byte = 0u8;
        assert_eq!(
            unsafe { libc::read(self.reply.as_raw_fd(), (&mut byte as *mut u8).cast(), 1) },
            1
        );
        assert_eq!(byte, expected);
    }
    fn unlock_old(&self) {
        assert_eq!(
            unsafe { libc::write(self.command.as_raw_fd(), b"U".as_ptr().cast(), 1) },
            1
        );
        self.ack(b'U');
    }
    fn finish(mut self) {
        assert_eq!(
            unsafe { libc::write(self.command.as_raw_fd(), b"Q".as_ptr().cast(), 1) },
            1
        );
        assert!(
            ready(self.pidfd.as_ref().unwrap().as_raw_fd()),
            "owned child did not exit"
        );
        let mut status = 0;
        assert_eq!(unsafe { libc::waitpid(self.pid, &mut status, 0) }, self.pid);
        self.reaped = true;
        assert!(libc::WIFEXITED(status) && libc::WEXITSTATUS(status) == 0);
    }
}
impl Drop for Inherited {
    fn drop(&mut self) {
        if self.reaped {
            return;
        }
        if let Some(fd) = &self.pidfd {
            unsafe {
                libc::syscall(
                    libc::SYS_pidfd_send_signal,
                    fd.as_raw_fd(),
                    libc::SIGKILL,
                    std::ptr::null::<libc::siginfo_t>(),
                    0,
                )
            };
        } else {
            // Exact forked child has not been reaped, so this PID cannot be reused.
            unsafe { libc::kill(self.pid, libc::SIGKILL) };
        }
        loop {
            let result = unsafe { libc::waitpid(self.pid, std::ptr::null_mut(), 0) };
            if result == self.pid
                || std::io::Error::last_os_error().kind() != std::io::ErrorKind::Interrupted
            {
                break;
            }
        }
    }
}
fn busy(root: &Path) {
    assert!(matches!(
        lock_host_root(root),
        Err(HostExecutionError::Busy)
    ));
}
#[test]
fn short_fence_raw_close_negative_control_retains_inherited_lock() {
    let temp = tempfile::tempdir().unwrap();
    let file = open_lock(&temp.path().join(HOST_EXECUTION_LOCK_NAME)).unwrap();
    FileExt::try_lock_exclusive(&file).unwrap();
    let child = Inherited::fork(&file);
    busy(temp.path());
    drop(file);
    busy(temp.path());
    child.unlock_old();
    drop(lock_host_root(temp.path()).unwrap());
    child.finish();
    println!("short_fence raw_close_busy=true cloexec=true explicit_child_unlock_releases=true");
}
#[test]
fn short_fence_drop_releases_original_not_next_open_description() {
    let temp = tempfile::tempdir().unwrap();
    let (first, child) = with_inherited(|| lock_host_root(temp.path()).unwrap());
    busy(temp.path());
    drop(first);
    let next = lock_host_root(temp.path())
        .expect("short owner returned but inherited alias retained lock");
    busy(temp.path());
    child.unlock_old();
    busy(temp.path());
    child.finish();
    busy(temp.path());
    drop(next);
    drop(lock_host_root(temp.path()).unwrap());
    println!("short_fence drop_release=true failed_contender_preserves=true late_old_unlock_preserves_next=true");
}
#[test]
fn short_fence_admission_error_releases_original_description() {
    let temp = tempfile::tempdir().unwrap();
    let (result, child) =
        with_inherited(|| lock_host_root_with(temp.path(), |_, _| Err(HostExecutionError::Path)));
    assert!(matches!(result, Err(HostExecutionError::Path)));
    let next = lock_host_root(temp.path()).expect("admission error leaked inherited fence");
    child.unlock_old();
    busy(temp.path());
    child.finish();
    drop(next);
    println!("short_fence admission_error_release=true");
}
struct NoExecution;
impl DiagnosticBackend for NoExecution {
    fn start_once(&mut self, _: &CellIntent) -> Result<(), String> {
        panic!("unexpected start")
    }
    fn observe_tree(&mut self, _: &CellIntent) -> Result<TreeObservation, String> {
        panic!("unexpected observe")
    }
    fn stop_tree(&mut self, _: &CellIntent) -> Result<(), String> {
        panic!("unexpected stop")
    }
}
fn request(n: u64, operation: DiagnosticOperation) -> DiagnosticRequest {
    DiagnosticRequest {
        schema_version: 1,
        request_id: format!("{n:064x}"),
        nonce_sha256: format!("{:064x}", n + 1000),
        operation,
        expected_state_revision: if n == 1 { 0 } else { 1 },
        identity: DiagnosticIdentity {
            lease_id: "a".repeat(64),
            boot_id: "00000000-0000-4000-8000-000000000074".into(),
            binary_sha256: "b".repeat(64),
            build_provenance_sha256: "c".repeat(64),
        },
        source_commit: SOURCE_COMMIT.into(),
        preset: PRESET.into(),
        controller: DiagnosticController {
            repository_id: 1,
            run_id: 2,
            run_attempt: 3,
            actor_id: 4,
        },
        issued_at_unix_seconds: 100,
        expires_at_unix_seconds: 160,
    }
}
fn handle(
    root: &Path,
    n: u64,
    operation: DiagnosticOperation,
) -> Result<DiagnosticResponse, DiagnosticIpcError> {
    let request = request(n, operation);
    let key = SigningKey::from_bytes(&[74; 32]);
    let signed = SignedDiagnosticRequest {
        signature_hex: crate::hex(&key.sign(&signing_message(&request).unwrap()).to_bytes()),
        request,
    };
    handle_local(
        root,
        &serde_json::to_vec(&signed).unwrap(),
        &DiagnosticPeer {
            uid: 10,
            gids: vec![20],
        },
        &DiagnosticPolicy {
            repository_id: 1,
            actor_ids: vec![4],
            client_uids: vec![10],
            client_gid: 20,
            verifying_key: key.verifying_key(),
        },
        110,
        &DiagnosticClock {
            boot_id: request_boot(),
            monotonic_ns: 0,
        },
        &mut NoExecution,
    )
}
fn request_boot() -> String {
    request(1, DiagnosticOperation::Reserve).identity.boot_id
}
fn snapshot(root: &Path) -> BTreeMap<String, (u64, u32, Vec<u8>)> {
    fs::read_dir(root)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            let m = entry.metadata().unwrap();
            (
                entry.file_name().into_string().unwrap(),
                (m.ino(), m.mode(), fs::read(entry.path()).unwrap()),
            )
        })
        .collect()
}
fn corrupt(root: &Path, kind: usize) {
    let path = root.join(LEDGER_NAME);
    match kind {
        0 => fs::write(path, b"{}").unwrap(),
        3 => {
            let mut bytes = fs::read(&path).unwrap();
            bytes.push(b' ');
            fs::write(path, bytes).unwrap();
        }
        5 => fs::write(root.join(PENDING_NAME), b"crash").unwrap(),
        6 => fs::write(path, vec![b' '; MAX_LEDGER_BYTES as usize + 1]).unwrap(),
        _ => {
            let mut value: serde_json::Value =
                serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            match kind {
                1 => value["schema_version"] = serde_json::json!(2),
                2 => value["unknown"] = serde_json::json!(true),
                _ => {
                    value["ledger"]["entries"][0]["request_sha256"] =
                        serde_json::json!("0".repeat(64))
                }
            }
            value["ledger_sha256"] = serde_json::json!(crate::sha256_hex(
                &crate::canonical_json(&value["ledger"]).unwrap()
            ));
            let mut bytes = crate::canonical_json(&value).unwrap();
            bytes.push(b'\n');
            fs::write(path, bytes).unwrap();
        }
    }
}
#[test]
fn short_fence_corrupt_ledger_error_releases_before_original_recovery() {
    for kind in 0..7 {
        let temp = tempfile::tempdir().unwrap();
        assert!(
            handle(temp.path(), 1, DiagnosticOperation::Reserve)
                .unwrap()
                .ok
        );
        corrupt(temp.path(), kind);
        let before = snapshot(temp.path());
        let (result, child) =
            with_inherited(|| handle(temp.path(), 2, DiagnosticOperation::Heartbeat));
        assert!(matches!(
            result,
            Err(DiagnosticIpcError::Ledger | DiagnosticIpcError::Uncertain)
        ));
        println!("short_fence corruption_kind={kind} transaction_refused=true inherited_child_ready=true");
        let coordinator = DiagnosticCoordinator::recover(temp.path(), &"a".repeat(64))
            .expect("corrupt-ledger handler returned but inherited alias retained lock");
        assert_eq!(coordinator.state().unwrap().revision, 1);
        assert_eq!(snapshot(temp.path()), before);
        child.unlock_old();
        child.finish();
        assert_eq!(coordinator.state().unwrap().revision, 1);
        assert_eq!(snapshot(temp.path()), before);
    }
    println!("short_fence corruption_cases=7 backend_calls=0 state_and_ledger_preserved=true");
}
