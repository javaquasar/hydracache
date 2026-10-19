//! Explicit local-root cgroup fixtures. No service, original owner or product.
use super::*;
use std::io::Read;
use std::os::unix::net::UnixDatagram;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const TEST: &str = "diagnostic_two_phase_launch::root_enrollment::kernel::tests::fault::cgroup_root_kernel_fault_matrix";
const PREFIX: &str = "hc074-root-fixture-";
struct OwnedChild {
    child: Child,
    pin: File,
}
impl OwnedChild {
    fn new(mut command: Command) -> Self {
        let mut child = command.spawn().unwrap();
        let raw = unsafe { libc::syscall(libc::SYS_pidfd_open, child.id(), 0u32) };
        if raw < 0 {
            // Still-unreaped owned Command child cannot be a recycled PID.
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("pidfd acquisition failed");
        }
        Self {
            child,
            pin: unsafe { File::from_raw_fd(raw as _) },
        }
    }
    fn signal(&self, signal: i32) {
        assert_eq!(
            unsafe {
                libc::syscall(
                    libc::SYS_pidfd_send_signal,
                    self.pin.as_raw_fd(),
                    signal,
                    0usize,
                    0u32,
                )
            },
            0
        );
    }
    fn killed(&mut self) {
        let bound = Instant::now() + Duration::from_secs(5); // harness only
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert_eq!(status.signal(), Some(libc::SIGKILL));
                return;
            }
            assert!(Instant::now() < bound, "owned target did not terminate");
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    fn live(&mut self) -> bool {
        self.child.try_wait().unwrap().is_none()
    }
}
impl Drop for OwnedChild {
    fn drop(&mut self) {
        if self.child.try_wait().unwrap().is_none() {
            let result = unsafe {
                libc::syscall(
                    libc::SYS_pidfd_send_signal,
                    self.pin.as_raw_fd(),
                    libc::SIGKILL,
                    0usize,
                    0u32,
                )
            };
            assert!(result == 0 || unsafe { *libc::__errno_location() } == libc::ESRCH);
        }
        self.child.wait().unwrap();
    }
}
struct Fixture {
    temporary: tempfile::TempDir,
    name: CString,
    parent: File,
    path: PathBuf,
    nested: bool,
    children: Vec<OwnedChild>,
    removed: bool,
    fence: File,
}
impl Fixture {
    fn new() -> Self {
        let temporary = tempfile::Builder::new().prefix(PREFIX).tempdir().unwrap();
        let name = CString::new(temporary.path().file_name().unwrap().as_encoded_bytes()).unwrap();
        let parent = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW)
            .open("/sys/fs/cgroup")
            .unwrap();
        stamp(&parent).unwrap();
        let path = Path::new("/sys/fs/cgroup").join(name.to_str().unwrap());
        assert!(
            name.to_bytes().starts_with(PREFIX.as_bytes())
                && path.parent() == Some(Path::new("/sys/fs/cgroup"))
        );
        std::fs::create_dir(&path).unwrap();
        let fence = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(temporary.path().join("fence"))
            .unwrap();
        assert_eq!(
            unsafe { libc::flock(fence.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
            0
        );
        Self {
            temporary,
            name,
            parent,
            path,
            nested: false,
            children: vec![],
            removed: false,
            fence,
        }
    }
    fn pin(&self) -> PinnedRoot {
        PinnedRoot::pin_empty(self.parent.as_fd(), &self.name).unwrap()
    }
    fn target(&mut self, nested: bool, output: Option<File>) -> usize {
        let mut command = Command::new("/bin/cat");
        command
            .stdin(Stdio::piped())
            .stdout(output.map_or_else(Stdio::null, Stdio::from))
            .stderr(Stdio::null())
            .uid(65534)
            .gid(65534);
        let owned = OwnedChild::new(command);
        let destination = if nested {
            if !self.nested {
                std::fs::create_dir(self.path.join("nested")).unwrap();
                self.nested = true;
            }
            self.path.join("nested")
        } else {
            self.path.clone()
        };
        let procs = OpenOptions::new()
            .write(true)
            .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
            .open(destination.join("cgroup.procs"))
            .unwrap();
        let pid = owned.child.id().to_string();
        assert_eq!(procs.write_at(pid.as_bytes(), 0).unwrap(), pid.len());
        let expected = destination
            .strip_prefix("/sys/fs/cgroup")
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(format!("/proc/{pid}/cgroup")).unwrap(),
            format!("0::/{expected}\n")
        );
        self.children.push(owned);
        self.children.len() - 1
    }
    fn empty(&self, root: &mut PinnedRoot) {
        let bound = Instant::now() + Duration::from_secs(5);
        while root.observe().unwrap().populated {
            assert!(Instant::now() < bound, "root did not become empty");
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    fn cleanup(&mut self) {
        self.children.clear(); // exact pidfd kill/wait precedes directory cleanup
        let other = OpenOptions::new()
            .read(true)
            .write(true)
            .open(self.temporary.path().join("fence"))
            .unwrap();
        assert_eq!(
            unsafe { libc::flock(other.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
            -1
        );
        assert!(matches!(
            io::Error::last_os_error().raw_os_error(),
            Some(libc::EWOULDBLOCK)
        ));
        assert_eq!(
            self.fence.metadata().unwrap().ino(),
            other.metadata().unwrap().ino()
        );
        if self.nested {
            std::fs::remove_dir(self.path.join("nested")).unwrap();
            self.nested = false;
        }
        if !self.removed {
            std::fs::remove_dir(&self.path).unwrap();
            self.removed = true;
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.cleanup();
    }
}

fn victim() {
    let name = CString::new(std::env::var("HC074_ROOT_NAME").unwrap()).unwrap();
    assert!(name.to_bytes().starts_with(PREFIX.as_bytes()));
    let parent = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW)
        .open("/sys/fs/cgroup")
        .unwrap();
    let mut root = PinnedRoot::pin_empty(parent.as_fd(), &name).unwrap();
    let socket = UnixDatagram::unbound().unwrap();
    socket
        .connect(std::env::var("HC074_ROOT_NOTICE").unwrap())
        .unwrap();
    socket.send(b"P").unwrap(); // root pinned while empty, before parent adds target
    let mut go = [0];
    std::io::stdin().read_exact(&mut go).unwrap();
    assert_eq!(go, [b'G']);
    root.submit_kill().unwrap();
    checkpoint(3); // successful API return, confirmation deliberately absent
    let bound = Instant::now() + Duration::from_secs(5);
    while root.observe().unwrap().populated {
        assert!(Instant::now() < bound);
        std::thread::sleep(Duration::from_millis(1));
    }
    checkpoint(4);
    panic!("victim must be killed at its selected cut");
}
fn cut(stage: u8) {
    let mut fixture = Fixture::new();
    let socket_path = fixture.temporary.path().join("notice.sock");
    let socket = UnixDatagram::bind(&socket_path).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            TEST,
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("HC074_ROOT_CUT", stage.to_string())
        .env("HC074_ROOT_NAME", fixture.name.to_str().unwrap())
        .env("HC074_ROOT_NOTICE", &socket_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    let mut owner = OwnedChild::new(command);
    let mut notice = [0; 2];
    assert_eq!(socket.recv(&mut notice).unwrap(), 1);
    assert_eq!(notice[0], b'P');
    let target = fixture.target(true, None);
    std::io::Write::write_all(owner.child.stdin.as_mut().unwrap(), b"G").unwrap();
    assert_eq!(socket.recv(&mut notice).unwrap(), 1);
    assert_eq!(notice[0], stage);
    owner.signal(libc::SIGKILL);
    owner.killed();
    if stage == 1 {
        assert!(
            fixture.children[target].live(),
            "before-write cut must not invent revocation"
        );
    } else {
        fixture.children[target].killed();
    }
    // No recovery/adoption/replay or successful attempt receipt is constructed.
    println!("HC074-CGROUP-ROOT cut={stage} actual_sigkill=true lost_confirmation=true product=false original_authority=false");
    fixture.cleanup();
}

#[test]
#[ignore = "manual local root cgroup2 fixtures; no service, original start or product"]
fn cgroup_root_kernel_fault_matrix() {
    assert_eq!(
        std::env::var("HC074_ROOT_FAULT_FIXTURE").as_deref(),
        Ok("local-only-v1")
    );
    assert_eq!(unsafe { libc::geteuid() }, 0);
    if std::env::var_os("HC074_ROOT_CUT").is_some() {
        victim();
        return;
    }
    let seed = 0x074c_2019_u64;
    println!("HC074-CGROUP-ROOT seed={seed:#x} local_only=true product=false");
    {
        let mut f = Fixture::new();
        let mut root = f.pin();
        assert!(!root.observe().unwrap().populated);
        root.submit_kill().unwrap();
        f.empty(&mut root);
        // Empty-at-read cannot prevent a later admitted writer.
        let target = f.target(false, None);
        assert!(root.observe().unwrap().populated);
        root.submit_kill().unwrap();
        f.children[target].killed();
        f.empty(&mut root);
        println!("HC074-CGROUP-ROOT case=empty-and-late-population completion=false");
    }
    {
        let mut f = Fixture::new();
        let mut root = f.pin();
        let a = f.target(false, None);
        let b = f.target(true, None);
        assert!(PinnedRoot::pin_empty(f.parent.as_fd(), &f.name).is_err());
        root.submit_kill().unwrap();
        f.children[a].killed();
        f.children[b].killed();
        f.empty(&mut root);
        println!("HC074-CGROUP-ROOT case=recursive-nonroot-writers actual_sigkill=true");
    }
    {
        let mut f = Fixture::new();
        let mut root = f.pin();
        let mut pipes = [-1; 2];
        assert_eq!(
            unsafe { libc::pipe2(pipes.as_mut_ptr(), libc::O_CLOEXEC | libc::O_NONBLOCK) },
            0
        );
        let mut reader = unsafe { File::from_raw_fd(pipes[0]) };
        let hidden = unsafe { File::from_raw_fd(pipes[1]) };
        let target = f.target(true, Some(hidden.try_clone().unwrap()));
        root.submit_kill().unwrap();
        f.children[target].killed();
        f.empty(&mut root);
        let mut byte = [0];
        assert_eq!(
            reader.read(&mut byte).unwrap_err().kind(),
            io::ErrorKind::WouldBlock
        );
        drop(hidden);
        assert_eq!(reader.read(&mut byte).unwrap(), 0);
        println!("HC074-CGROUP-ROOT case=external-writer-blocks-eof all_writers=false");
    }
    {
        let mut f = Fixture::new();
        let mut root = f.pin();
        let original = root.identity;
        std::fs::remove_dir(&f.path).unwrap();
        std::fs::create_dir(&f.path).unwrap();
        assert_ne!(std::fs::metadata(&f.path).unwrap().ino(), original.inode);
        let target = f.target(false, None);
        assert!(root.submit_kill().is_err() && root.refused && !root.submitted);
        assert!(f.children[target].live());
        println!("HC074-CGROUP-ROOT case=replacement-refused replacement_not_signaled=true");
    }
    {
        let mut f = Fixture::new();
        let mut root = f.pin();
        std::fs::remove_dir(&f.path).unwrap();
        f.removed = true;
        assert!(root.observe().is_err() && root.submit_kill().is_err());
        println!("HC074-CGROUP-ROOT case=removed-refused");
    }
    {
        let f = Fixture::new();
        let mut root = f.pin();
        let mode = std::fs::metadata(&f.path).unwrap().permissions();
        std::fs::set_permissions(&f.path, std::fs::Permissions::from_mode(0o777)).unwrap();
        assert!(root.observe().is_err());
        std::fs::set_permissions(&f.path, mode).unwrap();
        assert!(root.observe().is_err() && root.submit_kill().is_err() && !root.submitted);
        println!("HC074-CGROUP-ROOT case=permission-drift-sticky");
    }
    {
        let f = Fixture::new();
        let kind = OpenOptions::new()
            .write(true)
            .open(f.path.join("cgroup.type"))
            .unwrap();
        assert_eq!(kind.write_at(b"threaded", 0).unwrap(), 8);
        assert!(PinnedRoot::pin_empty(f.parent.as_fd(), &f.name).is_err());
        println!("HC074-CGROUP-ROOT case=threaded-refused");
    }
    for stage in 1..=4 {
        cut(stage);
    }
    println!("HC074-CGROUP-ROOT completed=true cases=11 original_authority=false guardian_armed=false completion=false");
}
