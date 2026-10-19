//! Real nonroot sender for a root receiver. Carries signed fixture bytes only.
use super::*;
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::process::CommandExt;
use std::process::{Child, ChildStdin, Command, Stdio};
const MODE: &str = "HC074_PRIVATE_ROOT_CONTROLLER_RELAY";
pub(super) struct Relay {
    child: Child,
    input: Option<ChildStdin>,
    pin: OwnedFd,
    pub(super) receiver: KernelReceiver,
}
impl Relay {
    pub(super) fn new() -> Self {
        let mut fds = [-1; 2];
        assert_eq!(
            unsafe {
                libc::socketpair(
                    libc::AF_UNIX,
                    libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC,
                    0,
                    fds.as_mut_ptr(),
                )
            },
            0
        );
        let receiver = KernelReceiver::new(unsafe { OwnedFd::from_raw_fd(fds[0]) }).unwrap();
        let sender = unsafe { OwnedFd::from_raw_fd(fds[1]) };
        let fd = sender.as_raw_fd();
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "privileged_root_controller_relay_child",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(MODE, "local-only-v1")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit());
        unsafe {
            command.pre_exec(move || {
                if libc::dup2(fd, 3) != 3
                    || libc::fcntl(3, libc::F_SETFD, 0) != 0
                    || libc::setgroups(0, std::ptr::null()) != 0
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
        let pin = unsafe { libc::syscall(libc::SYS_pidfd_open, child.id(), 0u32) };
        if pin < 0 {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("relay pidfd");
        }
        let input = child.stdin.take();
        Self {
            child,
            input,
            pin: unsafe { OwnedFd::from_raw_fd(pin as _) },
            receiver,
        }
    }
    pub(super) fn receive(
        &mut self,
        bytes: &[u8],
    ) -> crate::diagnostic_kernel_admission::KernelPacket {
        assert!(!bytes.is_empty() && bytes.len() <= crate::protocol::MAX_PACKET_BYTES);
        let input = self.input.as_mut().unwrap();
        input
            .write_all(&(bytes.len() as u32).to_le_bytes())
            .unwrap();
        input.write_all(bytes).unwrap();
        input.flush().unwrap();
        let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            if let Some(packet) = self.receiver.try_receive().unwrap() {
                return packet;
            }
            assert!(self.child.try_wait().unwrap().is_none(), "relay exited");
            assert!(
                std::time::Instant::now() < until,
                "relay receive harness bound"
            );
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
}
impl Drop for Relay {
    fn drop(&mut self) {
        self.input.take();
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
}
#[test]
fn privileged_root_controller_relay_child() {
    if std::env::var(MODE).as_deref() != Ok("local-only-v1") {
        return;
    }
    assert_eq!(
        (unsafe { libc::geteuid() }, unsafe { libc::getegid() }),
        (65534, 65534)
    );
    assert_eq!(unsafe { libc::getgroups(0, std::ptr::null_mut()) }, 0);
    let mut input = std::io::stdin().lock();
    loop {
        let mut len = [0; 4];
        match input.read_exact(&mut len) {
            Ok(()) => (),
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return,
            Err(e) => panic!("relay framing: {e}"),
        }
        let n = u32::from_le_bytes(len) as usize;
        assert!(n > 0 && n <= crate::protocol::MAX_PACKET_BYTES);
        let mut bytes = vec![0; n];
        input.read_exact(&mut bytes).unwrap();
        assert_eq!(
            unsafe { libc::send(3, bytes.as_ptr().cast(), n, libc::MSG_NOSIGNAL) },
            n as isize
        );
    }
}
