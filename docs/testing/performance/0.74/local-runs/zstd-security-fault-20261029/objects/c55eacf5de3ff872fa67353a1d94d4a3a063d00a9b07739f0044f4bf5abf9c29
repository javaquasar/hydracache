use super::*;
fn refused(own: &RootedExec, sent: u32) {
    assert_eq!(own.phase, Phase::Refused);
    assert!(own.reaped && own.root.refused);
    let channel = own.channel.as_ref().unwrap();
    assert_eq!(channel.step, Step::Refused);
    assert_eq!(channel.sends, sent);
}
#[test]
fn signer_handoff_original_child_receives_dummy_once_then_closes() {
    let mut own = owner(Case::Good).unwrap();
    run(&mut own).unwrap();
    assert_eq!(own.phase, Phase::Terminal);
    assert!(own.reaped);
    let ch = own.channel.as_ref().unwrap();
    assert_eq!(ch.step, Step::Done);
    assert!(ch.attempted && ch.child_closed);
    assert_eq!(ch.sends, 1);
    println!("handoff_original_ready_dummy_ack_close_exit=exact packets=16 sends=1 real_credentials=false");
}
#[test]
fn signer_handoff_dummy_withheld_at_every_readiness_boundary() {
    for phase in 0..5 {
        let mut own = RootedExec::handoff(Case::Good).unwrap();
        if phase >= 1 {
            own.enter().unwrap();
        }
        if phase >= 2 {
            own.first_exec().unwrap();
        }
        if phase >= 3 {
            own.install().unwrap();
        }
        if phase >= 4 {
            ready(&mut own).unwrap();
        }
        assert_eq!(own.channel.as_ref().unwrap().sends, 0);
        assert!(!own.channel.as_ref().unwrap().attempted);
        if phase < 4 {
            assert!(deliver(&mut own).is_err());
            refused(&own, 0);
        }
    }
    println!("handoff_withheld=initial,rooted,exec,installed,ready no_send_before_original_read_entry=true");
}
#[test]
fn signer_handoff_malformed_duplicate_readiness_and_wrong_fd_refuse_before_send() {
    for case in [
        Case::WrongReady,
        Case::DuplicateReady,
        Case::Short,
        Case::Oversized,
        Case::WrongFd,
        Case::Closed,
        Case::Exit,
    ] {
        let mut own = owner(case).unwrap();
        assert!(run(&mut own).is_err(), "case={case:?}");
        refused(&own, 0);
    }
    println!("handoff_bad_ready_cases=7 sends=0 replacement=false");
}
#[test]
fn signer_handoff_startup_channel_use_and_inherited_descriptor_leak_refuse() {
    for case in [Case::Early, Case::Leak] {
        let mut own = RootedExec::handoff(case).unwrap();
        own.enter().unwrap();
        own.first_exec().unwrap();
        assert!(own.install().is_err());
        assert!(own.reaped && own.root.refused);
        assert_eq!(own.channel.as_ref().unwrap().sends, 0);
        assert!(!own.channel.as_ref().unwrap().attempted);
    }
}
#[test]
fn signer_handoff_ancillary_capable_call_fd_replacement_and_nonstack_pointer_refuse() {
    for case in [Case::Ancillary, Case::Replace, Case::Rodata] {
        let mut own = owner(case).unwrap();
        assert!(run(&mut own).is_err());
        refused(&own, 0);
    }
    println!("handoff_sendmsg_dup2_nonstack=refused_before_dispatch ancillary_FDs_received=0");
}
#[test]
fn signer_handoff_send_failure_latches_attempt_before_dispatch_without_retry() {
    let mut own = owner(Case::Good).unwrap();
    ready(&mut own).unwrap();
    // SAFETY: disable only this test's private parent endpoint writer.
    assert_eq!(
        unsafe {
            libc::shutdown(
                own.channel.as_ref().unwrap().socket.as_raw_fd(),
                libc::SHUT_WR,
            )
        },
        0
    );
    assert!(deliver(&mut own).is_err());
    refused(&own, 0);
    assert!(own.channel.as_ref().unwrap().attempted);
    assert!(deliver(&mut own).is_err());
    refused(&own, 0);
    println!("handoff_send_failure=latched attempted=true sends=0 retry=false");
}
#[test]
fn signer_handoff_lost_wrong_and_duplicate_ack_never_resend() {
    for case in [Case::NoAck, Case::WrongAck, Case::DuplicateAck] {
        let mut own = owner(case).unwrap();
        ready(&mut own).unwrap();
        deliver(&mut own).unwrap();
        assert!(complete(&mut own).is_err());
        refused(&own, 1);
        assert!(deliver(&mut own).is_err());
        assert!(run(&mut own).is_err());
        refused(&own, 1);
    }
    println!("handoff_ACK_fault_cases=3 exactly_one_dummy_no_retry=true");
}
#[test]
fn signer_handoff_phase_replay_is_sticky_and_terminal_not_reusable() {
    for phase in 0..3 {
        let mut own = owner(Case::Good).unwrap();
        ready(&mut own).unwrap();
        let sent = if phase > 0 {
            deliver(&mut own).unwrap();
            1
        } else {
            0
        };
        if phase > 1 {
            complete(&mut own).unwrap();
        }
        let result = match phase {
            0 => ready(&mut own),
            1 => deliver(&mut own),
            _ => complete(&mut own),
        };
        assert!(result.is_err());
        refused(&own, sent);
    }
}
#[test]
fn signer_handoff_foreign_endpoint_and_changed_flags_refuse() {
    for fault in 0..4 {
        let mut own = owner(Case::Good).unwrap();
        let ch = own.channel.as_mut().unwrap();
        match fault {
            0 => ch.inode[1] ^= 1,
            1 => {
                let (foreign, peer) = pair(Case::Good).unwrap();
                ch.socket = foreign.socket;
                drop(peer);
            }
            2 => {
                // SAFETY: only this fixture's owned endpoint flags.
                assert_eq!(
                    unsafe { libc::fcntl(ch.socket.as_raw_fd(), libc::F_SETFD, 0) },
                    0
                );
            }
            _ => {
                // SAFETY: only this fixture's owned endpoint flags.
                assert_eq!(
                    unsafe { libc::fcntl(ch.socket.as_raw_fd(), libc::F_SETFL, 0) },
                    0
                );
            }
        }
        assert!(run(&mut own).is_err());
        refused(&own, 0);
    }
    println!("handoff_foreign_endpoint_refused original_inode_and_flags_required=true");
}
#[test]
fn signer_handoff_original_context_and_missing_channel_refuse() {
    for fault in 0..3 {
        let mut own = owner(Case::Good).unwrap();
        match fault {
            0 => own.namespace = Some(magic("/proc/self/ns/user").unwrap()),
            1 => own.environment = None,
            _ => own.fd_directory = None,
        }
        assert!(run(&mut own).is_err());
        refused(&own, 0);
    }
    let mut own = owner(Case::Good).unwrap();
    own.channel = None;
    assert!(run(&mut own).is_err());
    assert!(own.reaped && own.root.refused);
}
#[test]
fn signer_handoff_original_sigkill_and_drop_at_three_transfer_boundaries() {
    for killed in [false, true] {
        for phase in 0..3 {
            let mut own = owner(Case::Good).unwrap();
            if phase > 0 {
                ready(&mut own).unwrap();
            }
            if phase > 1 {
                deliver(&mut own).unwrap();
            }
            let expected = u32::from(phase > 1);
            assert_eq!(own.channel.as_ref().unwrap().sends, expected);
            let pid = own.pid;
            let path = own.root.path.clone();
            if killed {
                // SAFETY: only this owned original pidfd, no PID adoption.
                assert_eq!(
                    unsafe {
                        libc::syscall(
                            libc::SYS_pidfd_send_signal,
                            own.pidfd.as_ref().unwrap().as_raw_fd(),
                            libc::SIGKILL,
                            0usize,
                            0u32,
                        )
                    },
                    0
                );
                let status = own.wait().unwrap();
                assert!(libc::WIFSIGNALED(status));
                assert_eq!(libc::WTERMSIG(status), libc::SIGKILL);
                assert!(complete(&mut own).is_err());
                refused(&own, expected);
            }
            drop(own);
            let mut status = 0;
            // SAFETY: diagnostic exact original already-reaped child only.
            assert_eq!(
                unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) },
                -1
            );
            assert_eq!(
                io::Error::last_os_error().raw_os_error(),
                Some(libc::ECHILD)
            );
            assert!(!path.exists());
        }
    }
    println!("handoff_child_SIGKILL_drop=installed,ready,delivered tracer_loss_proved=false");
}
#[test]
fn signer_handoff_seqpacket_bounds_eof_and_empty_queue_are_explicit() {
    for size in [0usize, 15, 16, 17, 128] {
        let (channel, child) = pair(Case::Good).unwrap();
        channel.empty().unwrap();
        let mut bytes = [0u8; 128];
        bytes[..16].copy_from_slice(READY);
        // SAFETY: bounded public record between this test's owned endpoints only.
        assert_eq!(
            unsafe {
                libc::send(
                    child.as_raw_fd(),
                    bytes.as_ptr().cast(),
                    size,
                    libc::MSG_NOSIGNAL,
                )
            },
            size as isize
        );
        assert!(channel.empty().is_err());
        assert_eq!(channel.receive(READY).is_ok(), size == 16);
    }
    let (channel, child) = pair(Case::Good).unwrap();
    assert!(channel.receive(READY).is_err());
    drop(child);
    assert!(channel.empty().is_err());
    assert!(channel.receive(READY).is_err());
}
#[test]
fn signer_handoff_seeded_cases_and_four_independent_owners() {
    let mut seed = 0x0740_4a11_u64;
    for _ in 0..8 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let case = if seed & 1 == 0 {
            Case::Good
        } else {
            Case::WrongReady
        };
        let mut own = owner(case).unwrap();
        assert_eq!(run(&mut own).is_ok(), case == Case::Good);
    }
    let threads: Vec<_> = (0..4)
        .map(|_| {
            std::thread::spawn(|| {
                let mut own = owner(Case::Good).unwrap();
                run(&mut own).unwrap();
            })
        })
        .collect();
    for t in threads {
        t.join().unwrap();
    }
    println!(
        "handoff_seed=0x07404a11 fresh_cases=8 independent_owners=4 multithreaded_signer=false"
    );
}
