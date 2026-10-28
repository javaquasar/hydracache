use super::*;
fn binding() -> RootBindingClaims {
    RootBindingClaims {
        attempt_sha256: [3; 32],
        root_device: 1,
        root_inode: 2,
        deadline_boottime_ns: 17,
        original_context_sha256: [4; 32],
    }
}
fn packet() -> [u8; PACKET_BYTES] {
    PacketClaims {
        kind: Kind::Enroll,
        binding: binding(),
    }
    .encode()
    .unwrap()
}
fn pipe() -> [File; 2] {
    let mut fds = [-1; 2];
    assert_eq!(
        unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC | libc::O_NONBLOCK) },
        0
    );
    unsafe { [File::from_raw_fd(fds[0]), File::from_raw_fd(fds[1])] }
}
#[test]
fn root_guardian_stack_packet_matches_every_byte_flag_and_credential() {
    let origin = Origin::current();
    let expected = packet();
    let mut m = raw::Received {
        bytes: expected,
        len: PACKET_BYTES as _,
        flags: 0,
        rights: [-1; 64],
        count: 0,
        credentials: Some(libc::ucred {
            pid: origin.pid,
            uid: origin.uid,
            gid: origin.gid,
        }),
        shape: true,
    };
    assert!(m.matches(&expected, origin.pid, origin.uid, origin.gid, 0));
    for offset in 0..PACKET_BYTES {
        for bit in 0..8 {
            m.bytes[offset] ^= 1 << bit;
            assert!(!m.matches(&expected, origin.pid, origin.uid, origin.gid, 0));
            m.bytes[offset] ^= 1 << bit;
        }
    }
    for bit in 0..32 {
        m.flags = 1i32.wrapping_shl(bit);
        assert_eq!(
            m.matches(&expected, origin.pid, origin.uid, origin.gid, 0),
            m.flags == libc::MSG_CMSG_CLOEXEC
        );
    }
    m.flags = 0;
    for (pid, uid, gid) in [
        (origin.pid + 1, origin.uid, origin.gid),
        (origin.pid, origin.uid ^ 1, origin.gid),
        (origin.pid, origin.uid, origin.gid ^ 1),
    ] {
        assert!(!m.matches(&expected, pid, uid, gid, 0));
    }
    m.shape = false;
    assert!(!m.matches(&expected, origin.pid, origin.uid, origin.gid, 0));
}
#[test]
fn root_guardian_stack_receive_closes_all_extra_and_truncated_aliases() {
    let o = Origin::current();
    for count in [1, 2, 3, 4, 64, 253] {
        let [a, b] = pair().unwrap();
        let [reader, writer] = pipe();
        assert!(raw::send(
            a.as_raw_fd(),
            &packet(),
            &vec![writer.as_raw_fd(); count]
        ));
        drop(writer);
        let mut received = raw::receive(b.as_raw_fd());
        assert_eq!(
            received.matches(&packet(), o.pid, o.uid, o.gid, 3),
            count == 3
        );
        received.close();
        received.close();
        let mut byte = 0u8;
        assert_eq!(
            unsafe { libc::read(reader.as_raw_fd(), (&mut byte as *mut u8).cast(), 1) },
            0,
            "count={count}"
        );
    }
}
#[test]
fn root_guardian_stack_rights_free_reply_and_empty_receive_are_distinct() {
    let o = Origin::current();
    let [a, b] = pair().unwrap();
    let mut empty = raw::receive(b.as_raw_fd());
    assert!(!empty.shape);
    empty.close();
    assert!(raw::send(a.as_raw_fd(), &packet(), &[]));
    let mut reply = raw::receive(b.as_raw_fd());
    assert!(reply.matches(&packet(), o.pid, o.uid, o.gid, 0));
    assert!(!reply.matches(&packet(), o.pid, o.uid, o.gid, 3));
    reply.close();
}
#[test]
fn root_guardian_whitelist_is_exact_and_regular_files_cannot_be_roots() {
    assert_eq!(keep([9, 7, 5, 3, 1]).unwrap(), [1, 3, 5, 7, 9]);
    for fds in [[-1, 2, 3, 4, 5], [1, 1, 2, 3, 4], [1, 2, 3, 4, 4]] {
        assert!(keep(fds).is_err());
    }
    let file = tempfile::tempfile().unwrap();
    let stamp = Stamp {
        device: 1,
        inode: 2,
        uid: 0,
        gid: 0,
        mode: 0o100600,
    };
    let mut name = [0; 129];
    name[..4].copy_from_slice(b"root");
    let snapshot = Snapshot {
        parent: file.as_raw_fd(),
        parent_identity: stamp,
        name,
        roles: [stamp; 3],
        kind: stamp,
        procs: stamp,
        enroll: packet(),
    };
    assert!(!raw::inspect(&snapshot, [file.as_raw_fd(); 3], true));
}
#[test]
fn root_guardian_existing_timer_inspection_never_renews_or_reads_counter() {
    assert_eq!(
        remaining_from_info("it_value: (3, 17)\n"),
        Some(3_000_000_017)
    );
    for info in [
        "it_value: (03, 17)",
        "it_value: (+3, 17)",
        "it_value: (3, 1000000000)",
        "it_value: (18446744073709551615, 0)",
        "it_value: (3, 17)\nit_value: (3, 17)",
        "it_value:\t(3, 17)",
    ] {
        assert_eq!(remaining_from_info(info), None, "{info}");
    }
    let raw = unsafe {
        libc::timerfd_create(libc::CLOCK_BOOTTIME, libc::TFD_CLOEXEC | libc::TFD_NONBLOCK)
    };
    assert!(raw >= 0);
    let timer = unsafe { File::from_raw_fd(raw) };
    let deadline = boottime().unwrap() + 3_000_000_000; // fixture bound only
    let value = libc::itimerspec {
        it_interval: libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        it_value: libc::timespec {
            tv_sec: (deadline / 1_000_000_000) as _,
            tv_nsec: (deadline % 1_000_000_000) as _,
        },
    };
    assert_eq!(
        unsafe {
            libc::timerfd_settime(
                timer.as_raw_fd(),
                libc::TFD_TIMER_ABSTIME,
                &value,
                std::ptr::null_mut(),
            )
        },
        0
    );
    inspect_timer(&timer, deadline).unwrap();
    let alias = timer.try_clone().unwrap();
    inspect_timer(&alias, deadline).unwrap();
    assert!(inspect_timer(&alias, deadline + 1_000_000_000).is_err());
    let [reader, _writer] = pipe();
    assert!(inspect_timer(&reader, deadline).is_err());
    let mut ticks = 0u64;
    assert_eq!(
        unsafe { libc::read(alias.as_raw_fd(), (&mut ticks as *mut u64).cast(), 8) },
        -1
    );
    assert_eq!(io::Error::last_os_error().kind(), io::ErrorKind::WouldBlock);
    // A rejected expired periodic timer must not be forwarded by inspection.
    let fd = unsafe {
        libc::timerfd_create(libc::CLOCK_BOOTTIME, libc::TFD_CLOEXEC | libc::TFD_NONBLOCK)
    };
    assert!(fd >= 0);
    let periodic = unsafe { File::from_raw_fd(fd) };
    let first = boottime().unwrap() + 1_000_000;
    let value = libc::itimerspec {
        it_interval: libc::timespec {
            tv_sec: 30,
            tv_nsec: 0,
        },
        it_value: libc::timespec {
            tv_sec: (first / 1_000_000_000) as _,
            tv_nsec: (first % 1_000_000_000) as _,
        },
    };
    assert_eq!(
        unsafe { libc::timerfd_settime(fd, libc::TFD_TIMER_ABSTIME, &value, std::ptr::null_mut()) },
        0
    );
    let mut event = libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };
    assert_eq!(unsafe { libc::poll(&mut event, 1, 1000) }, 1); // harness bound only
    let before = std::fs::read_to_string(format!("/proc/self/fdinfo/{fd}")).unwrap();
    assert_eq!(remaining_from_info(&before), Some(0));
    assert!(inspect_timer(&periodic, first).is_err());
    let after = std::fs::read_to_string(format!("/proc/self/fdinfo/{fd}")).unwrap();
    assert_eq!(remaining_from_info(&after), Some(0));
    assert!(raw::terminal(fd));
}
