use super::*;
use std::os::unix::{
    fs::PermissionsExt,
    net::{UnixListener, UnixStream},
};
#[test]
fn manager_channel_fixture_walk_rejects_uid_symlink_write_and_replacement() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path();
    std::fs::create_dir_all(path.join("run/dbus")).unwrap();
    let socket = path.join("run/dbus/system_bus_socket");
    let _listener = UnixListener::bind(&socket).unwrap();
    let uid = unsafe { libc::geteuid() };
    let pins = PathPins::walk(path, uid).unwrap();
    assert_eq!(pins.files.len(), 4);
    assert!(PathPins::walk(path, uid.wrapping_add(1)).is_err());
    std::fs::set_permissions(path.join("run"), std::fs::Permissions::from_mode(0o777)).unwrap();
    assert!(PathPins::walk(path, uid).is_err());
    std::fs::set_permissions(path.join("run"), std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::rename(&socket, path.join("old-socket")).unwrap();
    std::os::unix::fs::symlink(path.join("old-socket"), &socket).unwrap();
    assert!(PathPins::walk(path, uid).is_err());
    std::fs::remove_file(&socket).unwrap();
    let _new_listener = UnixListener::bind(&socket).unwrap();
    assert_ne!(PathPins::walk(path, uid).unwrap().stamps, pins.stamps);
}
#[test]
fn manager_channel_atomic_peer_pin_matches_actual_socket_credentials() {
    let (a, _b) = UnixStream::pair().unwrap();
    let pin = peer_pidfd(a.as_raw_fd()).unwrap();
    live(&pin).unwrap();
    assert_eq!(
        credentials(a.as_raw_fd()).unwrap(),
        [
            i64::from(unsafe { libc::getpid() }),
            i64::from(unsafe { libc::geteuid() }),
            i64::from(unsafe { libc::getegid() })
        ]
    );
    assert_eq!(
        unsafe { libc::fcntl(pin.as_raw_fd(), libc::F_GETFD) },
        libc::FD_CLOEXEC
    );
    assert!(peer_pidfd(-1).is_err());
    assert!(credentials(-1).is_err());
}
#[test]
fn manager_channel_original_owned_pidfd_death_is_observed_without_pid_signals() {
    use std::os::unix::process::ExitStatusExt;
    let mut child = std::process::Command::new("/bin/sleep")
        .arg("5")
        .spawn()
        .unwrap();
    // This child remains owned and unreaped, so its PID cannot be reused here.
    let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, child.id(), 0) };
    if fd < 0 {
        let _ = child.wait();
        panic!("owned pidfd unavailable");
    }
    struct Owned {
        child: std::process::Child,
        pin: File,
    }
    impl Drop for Owned {
        fn drop(&mut self) {
            unsafe {
                libc::syscall(
                    libc::SYS_pidfd_send_signal,
                    self.pin.as_raw_fd(),
                    libc::SIGKILL,
                    std::ptr::null::<libc::siginfo_t>(),
                    0,
                )
            };
            let _ = self.child.wait();
        }
    }
    let mut held = Owned {
        child,
        pin: unsafe { File::from_raw_fd(fd as RawFd) },
    };
    live(&held.pin).unwrap();
    assert_eq!(
        unsafe {
            libc::syscall(
                libc::SYS_pidfd_send_signal,
                held.pin.as_raw_fd(),
                libc::SIGKILL,
                std::ptr::null::<libc::siginfo_t>(),
                0,
            )
        },
        0
    );
    assert_eq!(held.child.wait().unwrap().signal(), Some(libc::SIGKILL));
    assert_eq!(live(&held.pin), Err(ChannelError::Changed));
}
#[test]
fn manager_channel_kernel_boot_is_canonical_and_only_procfs_is_accepted() {
    let value = boot().unwrap();
    assert_eq!(value.len(), 36);
    let file = tempfile::tempfile().unwrap();
    assert!(filesystem(&file, libc::PROC_SUPER_MAGIC as _).is_err());
    assert!(clock().unwrap() > 0);
}
