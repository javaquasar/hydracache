use super::*;

#[path = "diagnostic_prearmed_watchdog_fixture.rs"]
mod process;

#[test]
fn prearmed_rejected_multiple_and_truncated_rights_close_installed_aliases() {
    for count in [2usize, 32] {
        let [sender, receiver] = pair().unwrap();
        let file = File::open("/dev/null").unwrap();
        let rights = vec![file.as_raw_fd(); count];
        let bytes = packet(ENROLL);
        let mut control = [0usize; 32];
        let mut iov = libc::iovec {
            iov_base: bytes.as_ptr().cast_mut().cast(),
            iov_len: 16,
        };
        let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
        msg.msg_iov = &mut iov;
        msg.msg_iovlen = 1;
        msg.msg_control = control.as_mut_ptr().cast();
        msg.msg_controllen =
            unsafe { libc::CMSG_SPACE(std::mem::size_of_val(rights.as_slice()) as _) } as _;
        unsafe {
            let c = libc::CMSG_FIRSTHDR(&msg);
            (*c).cmsg_level = libc::SOL_SOCKET;
            (*c).cmsg_type = libc::SCM_RIGHTS;
            (*c).cmsg_len = libc::CMSG_LEN(std::mem::size_of_val(rights.as_slice()) as _) as _;
            std::ptr::copy_nonoverlapping(
                rights.as_ptr(),
                libc::CMSG_DATA(c).cast::<RawFd>(),
                rights.len(),
            );
            assert_eq!(
                libc::sendmsg(sender.as_raw_fd(), &msg, libc::MSG_NOSIGNAL),
                16
            );
        }
        let origin = Origin::current();
        let mut message = receive(receiver.as_raw_fd());
        assert!(!message.valid(ENROLL, origin.pid, origin.uid, origin.gid, 1));
        if count == 32 {
            assert_ne!(message.flags & libc::MSG_CTRUNC, 0);
        }
        let installed = message.rights[..message.count].to_vec();
        assert!(!installed.is_empty());
        message.close();
        for fd in installed {
            assert_eq!(unsafe { libc::fcntl(fd, libc::F_GETFD) }, -1);
        }
        assert!(unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFD) } >= 0);
    }
}

#[test]
fn prearmed_rights_credentials_and_rejected_alias_closure_are_actual() {
    let [sender, receiver] = pair().unwrap();
    let origin = Origin::current();
    let file = File::open("/dev/null").unwrap();
    assert!(send(sender.as_raw_fd(), ENROLL, Some(file.as_raw_fd())));
    let mut message = receive(receiver.as_raw_fd());
    assert!(message.valid(ENROLL, origin.pid, origin.uid, origin.gid, 1));
    assert!(!message.valid(ENROLL, origin.pid + 1, origin.uid, origin.gid, 1));
    assert!(!message.valid(ENROLL, origin.pid, origin.uid + 1, origin.gid, 1));
    assert!(!message.valid(ENROLL, origin.pid, origin.uid, origin.gid + 1, 1));
    assert!(!live_child(message.rights[0], origin.pid));
    let fd = message.rights[0];
    message.close();
    assert_eq!(unsafe { libc::fcntl(fd, libc::F_GETFD) }, -1);
    message.close();
    assert!(send(sender.as_raw_fd(), PENDING, None));
    let mut message = receive(receiver.as_raw_fd());
    assert!(message.valid(PENDING, origin.pid, origin.uid, origin.gid, 0));
    message.credentials = None;
    assert!(!message.valid(PENDING, origin.pid, origin.uid, origin.gid, 0));
    message.close();
}

#[test]
fn prearmed_thread_drift_refuses_before_any_enrollment_send() {
    let origin = Origin::current();
    std::thread::spawn(move || {
        let file = || File::open("/dev/null").unwrap();
        let mut guard = Watchdog {
            pid: origin.pid,
            pinned: file(),
            peer: file(),
            alarm: file(),
            origin,
            pending: true,
            consumed: false,
            armed: false,
            refused: false,
        };
        assert!(guard.enroll_once(&file()).is_err());
        assert!(guard.refused && !guard.consumed && !guard.armed);
        assert!(guard.pending().is_err());
    })
    .join()
    .unwrap();
}

#[test]
fn prearmed_codec_refuses_every_changed_byte_and_shape() {
    for kind in [PENDING, ENROLL, ARMED, REFUSED, REVOKING] {
        let original = packet(kind);
        assert!(matches_packet(&original, 16, 0, kind));
        assert!(matches_packet(&original, 16, libc::MSG_CMSG_CLOEXEC, kind));
        for i in 0..16 {
            let mut wrong = original;
            wrong[i] ^= 1;
            assert!(!matches_packet(&wrong, 16, 0, kind));
        }
        for len in [-1, 0, 15, 17] {
            assert!(!matches_packet(&original, len, 0, kind));
        }
        for bit in 0..32 {
            let flags = 1i32.wrapping_shl(bit);
            if flags != libc::MSG_CMSG_CLOEXEC {
                assert!(!matches_packet(&original, 16, flags, kind));
            }
        }
    }
}

#[test]
fn prearmed_proc_number_parser_is_bounded_and_exact() {
    assert_eq!(
        number(b"pos:\t0\nPid:\t123\nNSpid:\t123\n", b"Pid:\t"),
        Some(123)
    );
    for text in [
        b"Pid:\t0\n".as_slice(),
        b"Pid:\t-1\n",
        b"Pid:\t2147483648\n",
        b"Pid:\t1x\n",
        b"Pid:\t1\nPid:\t2\n",
        b"Pid:\t\n",
    ] {
        assert_eq!(number(text, b"Pid:\t"), None);
    }
}
