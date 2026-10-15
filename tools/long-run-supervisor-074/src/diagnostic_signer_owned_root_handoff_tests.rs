use super::*;
fn pair() -> (File, File, State) {
    let (parent, child, state) = transport(Some(Case::Good)).unwrap();
    (parent, child, state.unwrap())
}
fn send(socket: &File, raw: &[u8]) {
    assert_eq!(
        unsafe {
            libc::send(
                socket.as_raw_fd(),
                raw.as_ptr().cast(),
                raw.len(),
                libc::MSG_NOSIGNAL,
            )
        },
        raw.len() as isize
    );
}
#[test]
fn selectors_are_closed_and_child_modes_cannot_select_parent_controls() {
    for name in [
        "good",
        "wrong-ready",
        "duplicate-ready",
        "short",
        "oversized",
        "wrong-fd",
        "early",
        "leak",
        "no-ack",
        "wrong-ack",
        "duplicate-ack",
        "closed",
        "exit",
        "ancillary",
        "replace",
        "rodata",
    ] {
        let case = Case::parse(name).unwrap();
        assert_eq!(
            case.arg().to_str().unwrap(),
            format!("--root-handoff-{name}")
        );
        assert_eq!(
            spec(&format!("root-handoff-{name}")).unwrap(),
            (case, Action::Run)
        );
    }
    for name in [
        "withhold",
        "send-failure",
        "replay",
        "kill-installed",
        "drop-installed",
        "kill-ready",
        "drop-ready",
        "kill-delivered",
        "drop-delivered",
    ] {
        assert!(Case::parse(name).is_err());
        assert_ne!(
            spec(&format!("root-handoff-{name}")).unwrap().1,
            Action::Run
        );
    }
    for name in [
        "",
        "good",
        "root-handoff-",
        "root-handoff-unknown",
        "root-handoff-good-extra",
        "--root-handoff-good",
    ] {
        assert!(spec(name).is_err());
    }
}
#[test]
fn attempt_and_event_budgets_are_sticky_without_a_transport_success() {
    let (parent, _child, mut state) = pair();
    assert!(state.attempt().is_err());
    state.step = Step::Ready;
    state.attempt().unwrap();
    assert_eq!(
        unsafe { libc::shutdown(parent.as_raw_fd(), libc::SHUT_WR) },
        0
    );
    assert_eq!(
        unsafe {
            libc::send(
                parent.as_raw_fd(),
                DUMMY.as_ptr().cast(),
                16,
                libc::MSG_NOSIGNAL,
            )
        },
        -1
    );
    assert!(state.attempted && state.sends == 0);
    assert!(state.attempt().is_err());
    state.step = Step::Refused;
    assert!(state.attempt().is_err());
    for _ in 0..16 {
        state.next_event().unwrap();
    }
    assert!(state.next_event().is_err());
    assert_eq!(state.events, 16);
}
#[test]
fn original_endpoint_and_read_write_nonblock_cloexec_are_mandatory() {
    let (parent, _child, state) = pair();
    let (foreign, _, _) = pair();
    state.check(&parent).unwrap();
    assert!(state.check(&foreign).is_err());
    assert_eq!(
        unsafe { libc::fcntl(parent.as_raw_fd(), libc::F_SETFD, 0) },
        0
    );
    assert!(state.check(&parent).is_err());
    assert_eq!(
        unsafe { libc::fcntl(parent.as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC) },
        0
    );
    assert_eq!(
        unsafe { libc::fcntl(parent.as_raw_fd(), libc::F_SETFL, 0) },
        0
    );
    assert!(state.check(&parent).is_err());
    let (reader, _writer, state) = transport(None).unwrap();
    assert!(state.is_none());
    assert!(reader.metadata().unwrap().file_type().is_fifo());
    assert!(socket_flags(&reader).is_err());
}
#[test]
fn datagram_lengths_and_duplicate_packets_do_not_pass_as_empty_or_eof() {
    for len in [0, 15, 16, 17, 64] {
        let (parent, child, _) = pair();
        empty(&parent).unwrap();
        let mut raw = vec![0; len];
        if len >= 16 {
            raw[..16].copy_from_slice(READY);
        }
        send(&child, &raw);
        assert_eq!(receive(&parent, READY).is_ok(), len == 16);
    }
    let (parent, child, _) = pair();
    send(&child, READY);
    send(&child, READY);
    receive(&parent, READY).unwrap();
    assert!(empty(&parent).is_err());
    receive(&parent, READY).unwrap();
    assert!(drain_socket(&parent).is_err());
    drop(child);
    drain_socket(&parent).unwrap();
}
#[test]
fn failure_drain_uses_actual_record_length_and_keeps_the_48_byte_limit() {
    for lengths in [vec![], vec![16; 3], vec![16; 4], vec![64], vec![1; 5]] {
        let (parent, child, _) = pair();
        for len in &lengths {
            send(&child, &vec![0; *len]);
        }
        drop(child);
        let accepted = lengths.iter().sum::<usize>() <= 48 && lengths.len() < 5;
        assert_eq!(drain_socket(&parent).is_ok(), accepted);
    }
    let (parent, child, _) = pair();
    send(&child, &[]);
    assert!(drain_socket(&parent).is_err());
    send(&child, &[]);
    send(&child, READY);
    drop(child);
    assert!(drain_socket(&parent).is_err());
}
#[test]
fn startup_finite_allowlist_rejects_channel_io_replacement_and_mutating_prctl() {
    for nr in [
        libc::SYS_read,
        libc::SYS_write,
        libc::SYS_close,
        libc::SYS_sendto,
        libc::SYS_recvfrom,
        libc::SYS_sendmsg,
        libc::SYS_splice,
        libc::SYS_tee,
        libc::SYS_copy_file_range,
        libc::SYS_dup2,
        libc::SYS_clone,
        libc::SYS_execveat,
        libc::SYS_seccomp,
        libc::SYS_mount,
        libc::SYS_unshare,
    ] {
        let mut args = [0; 6];
        args[0] = 3;
        assert!(!startup_allowed(nr as u64, args), "{nr}");
    }
    assert!(startup_allowed(libc::SYS_read as u64, [4, 0, 16, 0, 0, 0]));
    assert!(startup_allowed(
        libc::SYS_prctl as u64,
        [libc::PR_GET_NO_NEW_PRIVS as u64, 0, 0, 0, 0, 0]
    ));
    for opt in [
        libc::PR_SET_DUMPABLE,
        libc::PR_SET_NO_NEW_PRIVS,
        libc::PR_SET_PDEATHSIG,
        libc::PR_CAPBSET_DROP,
    ] {
        assert!(!startup_allowed(
            libc::SYS_prctl as u64,
            [opt as u64, 0, 0, 0, 0, 0]
        ));
    }
    assert!(!startup_allowed(1024, [0; 6]));
    assert!(!startup_allowed(u64::MAX, [0; 6]));
}
#[test]
fn fixed_native_protocol_entry_rejects_every_argument_class_change() {
    let args = [3, 4096, 16, libc::MSG_NOSIGNAL as u64, 0, 0];
    assert_eq!(
        entry_pointer(
            Event::Entry {
                nr: libc::SYS_sendto as u64,
                args
            },
            libc::SYS_sendto,
            libc::MSG_NOSIGNAL
        )
        .unwrap(),
        4096
    );
    for at in [0, 2, 3, 4, 5] {
        for bit in 0..64 {
            let mut bad = args;
            bad[at] ^= 1 << bit;
            assert!(entry_pointer(
                Event::Entry {
                    nr: libc::SYS_sendto as u64,
                    args: bad
                },
                libc::SYS_sendto,
                libc::MSG_NOSIGNAL
            )
            .is_err());
        }
    }
    assert!(entry_pointer(
        Event::Entry {
            nr: libc::SYS_sendmsg as u64,
            args
        },
        libc::SYS_sendto,
        libc::MSG_NOSIGNAL
    )
    .is_err());
    assert!(entry_pointer(
        Event::Exit {
            value: 16,
            error: false
        },
        libc::SYS_sendto,
        libc::MSG_NOSIGNAL
    )
    .is_err());
}
#[test]
fn only_the_expected_fault_classes_may_have_received_the_public_dummy() {
    for case in [
        Case::WrongReady,
        Case::DuplicateReady,
        Case::Short,
        Case::Oversized,
        Case::WrongFd,
        Case::Early,
        Case::Leak,
        Case::Closed,
        Case::Exit,
        Case::Ancillary,
        Case::Replace,
        Case::Rodata,
    ] {
        assert_eq!(case.sent(), 0);
    }
    for case in [Case::Good, Case::NoAck, Case::WrongAck, Case::DuplicateAck] {
        assert_eq!(case.sent(), 1);
    }
    for step in [
        Step::Fresh,
        Step::Delivered,
        Step::Acked,
        Step::Done,
        Step::Refused,
    ] {
        let (_, _, mut state) = pair();
        state.step = step;
        assert!(state.attempt().is_err());
    }
}
