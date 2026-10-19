use super::*;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};

fn claims() -> RootBindingClaims {
    RootBindingClaims {
        attempt_sha256: [7; 32],
        root_device: 1,
        root_inode: 2,
        deadline_boottime_ns: 17,
        original_context_sha256: [9; 32],
    }
}
fn wire() -> [u8; PACKET_BYTES] {
    PacketClaims {
        kind: Kind::Enroll,
        binding: claims(),
    }
    .encode()
    .unwrap()
}
fn inbox(socket: File) -> Inbox {
    Inbox {
        socket,
        creator: Creator::current(),
        expected: claims(),
        consumed: false,
    }
}
fn pipe() -> [File; 2] {
    let mut fds = [-1; 2];
    assert_eq!(
        unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_NONBLOCK | libc::O_CLOEXEC) },
        0
    );
    unsafe { [File::from_raw_fd(fds[0]), File::from_raw_fd(fds[1])] }
}
fn eof(file: &File) {
    let mut byte = 0u8;
    assert_eq!(
        unsafe { libc::read(file.as_raw_fd(), (&mut byte as *mut u8).cast(), 1) },
        0,
        "rejected installed writer descriptors leaked"
    );
}
#[test]
fn cgroup_root_transport_kernel_credentials_and_atomic_cloexec() {
    let creator = Creator::current();
    let [send_socket, receive_socket] = socket_pair(creator).unwrap();
    let file = File::open("/dev/null").unwrap();
    send(&send_socket, &wire(), &[file.as_raw_fd(); 3]).unwrap();
    let frame = receive(&receive_socket).unwrap();
    assert!(creator.matches(&frame.credentials));
    assert_eq!(frame.rights.len(), 3);
    for received in &frame.rights {
        assert_eq!(
            unsafe { libc::fcntl(received.as_raw_fd(), libc::F_GETFD) },
            libc::FD_CLOEXEC
        );
    }
    for mut changed in [creator; 3].into_iter().enumerate() {
        match changed.0 {
            0 => changed.1.pid += 1,
            1 => changed.1.uid ^= 1,
            _ => changed.1.gid ^= 1,
        }
        assert!(!changed.1.matches(&frame.credentials));
    }
}
#[test]
fn cgroup_root_transport_rejected_right_counts_and_truncation_close_all_aliases() {
    for count in [0, 1, 2, 4, 32, 64, 253] {
        let [reader, writer] = pipe();
        let [a, b] = socket_pair(Creator::current()).unwrap();
        send(&a, &wire(), &vec![writer.as_raw_fd(); count]).unwrap();
        drop(writer);
        assert!(receive(&b).is_err(), "count={count}");
        eof(&reader);
    }
}
#[test]
fn cgroup_root_transport_size_flags_missing_credentials_and_fake_roots_refuse() {
    use std::os::unix::fs::OpenOptionsExt;
    let regular = tempfile::NamedTempFile::new().unwrap();
    let valid = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK | libc::O_CLOEXEC)
        .open(regular.path())
        .unwrap();
    access(&valid, libc::O_RDONLY, false).unwrap();
    assert!(access(&valid, libc::O_WRONLY, false).is_err());
    assert!(access(&valid, libc::O_RDONLY, true).is_err());
    for flags in [
        libc::O_CLOEXEC,
        libc::O_CLOEXEC | libc::O_NONBLOCK | libc::O_PATH,
    ] {
        let bad = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(flags)
            .open(regular.path())
            .unwrap();
        assert!(access(&bad, libc::O_RDONLY, false).is_err());
    }
    assert_eq!(
        unsafe { libc::fcntl(valid.as_raw_fd(), libc::F_SETFD, 0) },
        0
    );
    assert!(access(&valid, libc::O_RDONLY, false).is_err());
    for size in [0, 1, 16, 103, 105, 4096] {
        let [reader, writer] = pipe();
        let [a, b] = socket_pair(Creator::current()).unwrap();
        send(&a, &vec![0; size], &[writer.as_raw_fd(); 3]).unwrap();
        drop(writer);
        assert!(receive(&b).is_err(), "size={size}");
        eof(&reader);
    }
    let [a, b] = socket_pair(Creator::current()).unwrap();
    let off = 0i32;
    assert_eq!(
        unsafe {
            libc::setsockopt(
                b.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_PASSCRED,
                (&off as *const i32).cast(),
                std::mem::size_of_val(&off) as _,
            )
        },
        0
    );
    let [reader, writer] = pipe();
    send(&a, &wire(), &[writer.as_raw_fd(); 3]).unwrap();
    drop(writer);
    assert!(receive(&b).is_err());
    eof(&reader);
    let [a, b] = socket_pair(Creator::current()).unwrap();
    let file = File::open("/dev/null").unwrap();
    send(&a, &wire(), &[file.as_raw_fd(); 3]).unwrap();
    let mut receiver = inbox(b);
    assert!(receiver.receive_once().is_err());
    assert!(receiver.receive_once().is_err());
}
#[test]
fn cgroup_root_transport_every_bound_byte_refuses_and_closes_rights() {
    for offset in 0..PACKET_BYTES {
        let [reader, writer] = pipe();
        let [a, b] = socket_pair(Creator::current()).unwrap();
        let mut bytes = wire();
        bytes[offset] ^= 1;
        send(&a, &bytes, &[writer.as_raw_fd(); 3]).unwrap();
        drop(writer);
        let mut receiver = inbox(b);
        assert!(receiver.receive_once().is_err(), "offset={offset}");
        assert!(receiver.consumed);
        eof(&reader);
    }
}
#[test]
fn cgroup_root_transport_empty_read_consumes_attempt_and_late_packet_cannot_retry() {
    let [a, b] = socket_pair(Creator::current()).unwrap();
    let mut receiver = inbox(b);
    assert_eq!(
        receiver.receive_once().err().unwrap().kind(),
        io::ErrorKind::WouldBlock
    );
    assert!(receiver.consumed);
    let file = File::open("/dev/null").unwrap();
    send(&a, &wire(), &[file.as_raw_fd(); 3]).unwrap();
    assert!(receiver.receive_once().is_err());
    // A failed retry does not consume another message or reassign an attempt.
    assert_eq!(receive(&receiver.socket).unwrap().bytes, wire());
}
#[test]
fn cgroup_root_transport_foreign_sender_process_is_not_the_pair_creator() {
    const ENV: &str = "HC074_ROOT_FOREIGN_SENDER_FD";
    if let Ok(raw) = std::env::var(ENV) {
        let fd: i32 = raw.parse().unwrap();
        assert!(fd > 2);
        let socket = unsafe { File::from_raw_fd(fd) };
        let file = File::open("/dev/null").unwrap();
        send(&socket, &wire(), &[file.as_raw_fd(); 3]).unwrap();
        return;
    }
    for inspect_only in [true, false] {
        let creator = Creator::current();
        let [a, b] = socket_pair(creator).unwrap();
        let raw = a.as_raw_fd();
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .arg("cgroup_root_transport_foreign_sender_process_is_not_the_pair_creator")
            .args(["--nocapture", "--test-threads=1"])
            .env(ENV, raw.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        // Only the owned test child inherits this carrier across exec. Never
        // clear CLOEXEC in the multithreaded test parent.
        unsafe {
            command.pre_exec(move || {
                if libc::fcntl(raw, libc::F_SETFD, 0) < 0 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut child = command.spawn().unwrap();
        assert_ne!(child.id(), creator.pid as u32);
        assert!(child.wait().unwrap().success());
        if inspect_only {
            let frame = receive(&b).unwrap();
            assert_eq!(frame.credentials.pid as u32, child.id());
            assert!(!creator.matches(&frame.credentials));
        } else {
            let mut receiver = inbox(b);
            assert!(receiver.receive_once().is_err());
            assert!(receiver.consumed);
        }
    }
}

#[path = "diagnostic_cgroup_root_transport_fault_tests.rs"]
mod fault;
