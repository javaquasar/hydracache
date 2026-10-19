//! Exact local dummy children and fixture-only cleanup, never completion.
use super::*;
use std::ffi::CString;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{FileExt, OpenOptionsExt};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::{Child, Command, Stdio};
pub(super) struct Dummy {
    child: Child,
    pin: File,
}
impl Dummy {
    pub(super) fn creator_pin(&self) -> std::os::fd::RawFd {
        self.pin.as_raw_fd()
    }
    pub(super) fn creator_input(&self) -> std::os::fd::RawFd {
        self.child.stdin.as_ref().unwrap().as_raw_fd()
    }
    pub(super) fn assert_live(&mut self) {
        assert!(
            self.child.try_wait().unwrap().is_none(),
            "writer exited before revocation"
        );
        let mut event = libc::pollfd {
            fd: self.pin.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        assert_eq!(unsafe { libc::poll(&mut event, 1, 0) }, 0);
        assert_eq!(event.revents, 0);
    }
    fn kill_wait(&mut self) {
        if self.child.try_wait().unwrap().is_none() {
            let n = unsafe {
                libc::syscall(
                    libc::SYS_pidfd_send_signal,
                    self.pin.as_raw_fd(),
                    libc::SIGKILL,
                    0usize,
                    0u32,
                )
            };
            assert!(n == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH));
        }
        self.child.wait().unwrap();
    }
    pub(super) fn killed(&mut self) {
        let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert_eq!(status.signal(), Some(libc::SIGKILL));
                return;
            }
            assert!(std::time::Instant::now() < until, "dummy did not die");
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
}
impl Drop for Dummy {
    fn drop(&mut self) {
        self.kill_wait();
    }
}
pub(super) struct Cgroup {
    _temporary: tempfile::TempDir,
    pub(super) name: CString,
    pub(super) parent: File,
    path: PathBuf,
    pub(super) writers: Vec<Dummy>,
    pub(super) guardians: Option<[File; 2]>,
}
impl Cgroup {
    pub(super) fn creator_path(&self) -> &Path {
        &self.path
    }
    pub(super) fn new() -> Self {
        let temporary =
            crate::diagnostic_two_phase_launch::root_enrollment::privileged_fixture::new();
        let name = CString::new(temporary.path().file_name().unwrap().as_encoded_bytes()).unwrap();
        let parent = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open("/sys/fs/cgroup")
            .unwrap();
        let path = Path::new("/sys/fs/cgroup").join(name.to_str().unwrap());
        assert_eq!(path.parent(), Some(Path::new("/sys/fs/cgroup")));
        fs::create_dir(&path).unwrap();
        fs::create_dir(path.join("nested")).unwrap();
        Self {
            _temporary: temporary,
            name,
            parent,
            path,
            writers: vec![],
            guardians: None,
        }
    }
    pub(super) fn writer(&mut self, nested: bool, out: File, err: File) {
        let mut command = Command::new("/bin/cat");
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::from(out))
            .stderr(Stdio::from(err));
        unsafe {
            command.pre_exec(|| {
                if libc::setgroups(0, std::ptr::null()) != 0
                    || libc::setresgid(65534, 65534, 65534) != 0
                    || libc::setresuid(65534, 65534, 65534) != 0
                    || libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0
                {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut child = command.spawn().unwrap();
        let raw = unsafe { libc::syscall(libc::SYS_pidfd_open, child.id(), 0u32) };
        if raw < 0 {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("dummy pidfd");
        }
        let pin = unsafe { File::from_raw_fd(raw as _) };
        let mut dummy = Dummy { child, pin };
        let target = if nested {
            self.path.join("nested")
        } else {
            self.path.clone()
        };
        let procs = OpenOptions::new()
            .write(true)
            .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
            .open(target.join("cgroup.procs"))
            .unwrap();
        let id = dummy.child.id().to_string();
        assert_eq!(procs.write_at(id.as_bytes(), 0).unwrap(), id.len());
        let membership = target
            .strip_prefix("/sys/fs/cgroup")
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(
            fs::read_to_string(format!("/proc/{id}/cgroup")).unwrap(),
            format!("0::/{membership}\n")
        );
        dummy
            .child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(b"hc074-live-writer\n")
            .unwrap();
        self.writers.push(dummy);
    }
    pub(super) fn signal_guardian(&self, index: usize, signal: i32) {
        assert_eq!(
            unsafe {
                libc::syscall(
                    libc::SYS_pidfd_send_signal,
                    self.guardians.as_ref().unwrap()[index].as_raw_fd(),
                    signal,
                    0usize,
                    0u32,
                )
            },
            0
        );
    }
    pub(super) fn empty(&self) {
        assert_eq!(
            fs::read(self.path.join("cgroup.events")).unwrap(),
            b"populated 0\nfrozen 0\n"
        );
    }
}
impl Drop for Cgroup {
    fn drop(&mut self) {
        // Cleanup is not evidence. Stop/reap every owned dummy before guardians.
        self.writers.clear();
        if let Some(pins) = self.guardians.take() {
            for pin in pins {
                cleanup_guardian(&pin);
            }
        }
        self.empty();
        fs::remove_dir(self.path.join("nested")).unwrap();
        fs::remove_dir(&self.path).unwrap();
    }
}
fn cleanup_guardian(pin: &File) {
    let n = unsafe {
        libc::syscall(
            libc::SYS_pidfd_send_signal,
            pin.as_raw_fd(),
            libc::SIGKILL,
            0usize,
            0u32,
        )
    };
    assert!(n == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH));
    loop {
        let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
        if unsafe {
            libc::waitid(
                libc::P_PIDFD,
                pin.as_raw_fd() as _,
                &mut info,
                libc::WEXITED,
            )
        } == 0
        {
            break;
        }
        let error = std::io::Error::last_os_error().raw_os_error();
        if error == Some(libc::ECHILD) {
            // Original controls may already have reaped this exact guardian.
            dead(pin);
            break;
        }
        assert_eq!(error, Some(libc::EINTR));
    }
}
#[test]
fn root_guardian_fixture_cleanup_accepts_only_terminal_already_reaped_pidfd() {
    let mut child = Command::new("/bin/sleep").arg("30").spawn().unwrap();
    let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, child.id(), 0u32) };
    if fd < 0 {
        child.kill().unwrap();
        child.wait().unwrap();
        panic!("cleanup test pidfd");
    }
    let pin = unsafe { File::from_raw_fd(fd as _) };
    cleanup_guardian(&pin);
    dead(&pin);
    assert_eq!(child.wait().unwrap_err().raw_os_error(), Some(libc::ECHILD));
    cleanup_guardian(&pin); // ECHILD after the exact child has been reaped.
    dead(&pin);
}
pub(super) fn drain(reader: &mut File) -> bool {
    let mut bytes = [0; 4096];
    loop {
        match reader.read(&mut bytes) {
            Ok(0) => return true,
            Ok(_) => (),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return false,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => (),
            Err(e) => panic!("pipe drain: {e}"),
        }
    }
}
pub(super) fn ready(reader: &mut File, count: usize) {
    let expected = b"hc074-live-writer\n".repeat(count);
    let mut received = Vec::new();
    let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while received.len() < expected.len() {
        let mut bytes = [0; 4096];
        match reader.read(&mut bytes) {
            Ok(0) => panic!("writers reached EOF before revocation"),
            Ok(n) => received.extend_from_slice(&bytes[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                assert!(
                    std::time::Instant::now() < until,
                    "writers did not produce readiness bytes"
                );
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => (),
            Err(e) => panic!("writer readiness: {e}"),
        }
    }
    assert_eq!(received, expected);
}
pub(super) fn dead(pin: &File) {
    let mut event = libc::pollfd {
        fd: pin.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    assert_eq!(unsafe { libc::poll(&mut event, 1, 0) }, 1);
    assert_eq!(event.revents, libc::POLLIN);
}
