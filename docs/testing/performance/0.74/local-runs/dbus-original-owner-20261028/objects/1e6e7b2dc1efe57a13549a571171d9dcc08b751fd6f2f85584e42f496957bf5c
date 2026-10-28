use super::*;
#[test]
fn signer_tracer_loss_entry() {
    match std::env::var(ROLE).ok().as_deref() {
        None => {}
        Some("monitor") => monitor(Spec::environment().unwrap()).unwrap(),
        Some("tracer") => tracer(Spec::environment().unwrap()).unwrap(),
        Some("fd-lifetime") => received_rights_checks(),
        _ => panic!("unknown closed fixture role"),
    }
}
fn proof(stage: Stage, mode: Mode) {
    let output = outer(Spec { stage, mode }).unwrap();
    assert!(output.contains("proof=complete"));
    for line in output
        .lines()
        .filter(|line| line.starts_with("loss_child_terminal "))
    {
        println!("{line}");
    }
}
#[test]
fn signer_tracer_loss_installed_original_sigkill_reaps_original_child() {
    proof(Stage::Installed, Mode::Kill);
    println!("tracer_loss=installed tracer=SIGKILL child=SIGKILL original_pidfd_reaped=true emergency_cleanup=false dummy_sends=0");
}
#[test]
fn signer_tracer_loss_ready_original_sigkill_reaps_original_child() {
    proof(Stage::Ready, Mode::Kill);
    println!("tracer_loss=ready tracer=SIGKILL child=SIGKILL original_pidfd_reaped=true emergency_cleanup=false dummy_sends=0");
}
#[test]
fn signer_tracer_loss_delivered_original_sigkill_reaps_original_child() {
    proof(Stage::Delivered, Mode::Kill);
    println!("tracer_loss=delivered tracer=SIGKILL child=SIGKILL original_pidfd_reaped=true emergency_cleanup=false dummy_sends=1");
}
#[test]
fn signer_tracer_loss_initial_rooted_and_first_exec_boundaries() {
    for stage in [Stage::Initial, Stage::Rooted, Stage::Exec] {
        proof(stage, Mode::Kill);
    }
    println!(
        "tracer_loss=initial,rooted,first_exec original_child=SIGKILL no_authority_or_restart=true"
    );
}
#[test]
fn signer_tracer_loss_no_exitkill_controls_keep_natural_channel_failure_distinct() {
    for stage in [Stage::Installed, Stage::Ready, Stage::Delivered] {
        proof(stage, Mode::NoExitkill);
    }
    println!("tracer_loss_without_EXITKILL=bounded_natural_exit old_exact_prediction=false live_orphan_reproduced=false direct_child_kill=false");
}
#[test]
fn signer_tracer_loss_healthy_control_finishes_original_protocol_and_cleans() {
    proof(Stage::Delivered, Mode::Healthy);
    println!(
        "tracer_loss_healthy_control=ACK_close_exit0 original_owner_reaped=true replaced=false"
    );
}
#[test]
fn signer_tracer_loss_bad_reports_refuse_before_fault_admission() {
    for mode in [
        Mode::WrongStage,
        Mode::Short,
        Mode::MissingFd,
        Mode::SelfPidfd,
        Mode::WrongDir,
        Mode::WrongProgress,
    ] {
        proof(Stage::Installed, mode);
    }
    println!("tracer_loss_report_faults=6 original_owner_refuses_and_reaps=true no_unvalidated_pidfd_signal=true");
}
#[test]
fn signer_tracer_loss_frame_and_bounded_scalar_mutations_refuse() {
    for stage in STAGES {
        for code in 0..=255 {
            assert_eq!(
                death_admitted(stage, Mode::Kill, (libc::CLD_KILLED, code)),
                code == libc::SIGKILL
            );
            assert!(!death_admitted(stage, Mode::Kill, (libc::CLD_EXITED, code)));
            assert_eq!(
                death_admitted(stage, Mode::NoExitkill, (libc::CLD_EXITED, code)),
                match stage {
                    Stage::Installed => matches!(code, 73 | 74),
                    Stage::Ready => code == 74,
                    Stage::Delivered => matches!(code, 0 | 76),
                    _ => false,
                }
            );
            assert!(!death_admitted(
                stage,
                Mode::NoExitkill,
                (libc::CLD_KILLED, code)
            ));
            assert!(!death_admitted(
                stage,
                Mode::Healthy,
                (libc::CLD_EXITED, code)
            ));
        }
    }
    for stage in STAGES {
        let bytes = frame(stage, 123);
        assert_eq!(decode(&bytes, stage).unwrap(), 123);
        for index in (0..20).chain(24..32) {
            let mut changed = bytes;
            changed[index] ^= 0x80;
            assert!(decode(&changed, stage).is_err());
        }
        for tid in [0u32, u32::MAX, i32::MAX as u32 + 1] {
            assert!(decode(&frame(stage, tid), stage).is_err());
        }
    }
    for text in [
        "",
        "Pid: 1\nPid: 1\n",
        "Pid: -1\n",
        "Pid: x\n",
        "Pid: 4294967296\n",
    ] {
        assert!(scalar(text, "Pid").is_err());
    }
    assert_eq!(scalar("other: x\nPid:\t123\n", "Pid").unwrap(), 123);
    for spec in [
        Spec {
            stage: Stage::Initial,
            mode: Mode::NoExitkill,
        },
        Spec {
            stage: Stage::Ready,
            mode: Mode::Healthy,
        },
        Spec {
            stage: Stage::Exec,
            mode: Mode::Short,
        },
    ] {
        assert!(spec.validate().is_err());
    }
    let mut seed = 0x0740_4a12_u64;
    for _ in 0..256 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let stage = STAGES[seed as usize % 6];
        let mut bytes = frame(stage, 123);
        let offset = seed as usize % 16;
        bytes[offset] ^= (seed as u8).max(1);
        assert!(decode(&bytes, stage).is_err());
    }
    println!("tracer_loss_frame_seed=0x07404a12 bounded_mutations=256 future_fields_refused=true");
}
#[test]
fn signer_tracer_loss_seeded_cases_and_independent_monitors() {
    let mut seed = 0x0740_4a12_u64;
    for _ in 0..8 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        proof(STAGES[seed as usize % 6], Mode::Kill);
    }
    let threads: Vec<_> = (0..4)
        .map(|i| {
            std::thread::spawn(move || {
                proof(STAGES[2 + i], Mode::Kill);
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    println!("tracer_loss_seed=0x07404a12 fresh_cases=8 independent_monitors=4 parent_subreaper_unchanged=true");
}

#[test]
fn signer_tracer_loss_received_rights_close_on_refusal_and_healthy_drop() {
    observe(
        "fd-lifetime",
        Spec {
            stage: Stage::Initial,
            mode: Mode::Kill,
        },
    )
    .unwrap();
    println!(
        "tracer_loss_received_rights=owned_before_refusal pipe_EOF_proves_no_leaked_writer=true"
    );
}
fn received_rights_checks() {
    for fault in 0..5 {
        let (parent, peer) = channel::sockets().unwrap();
        let mut raw = [-1; 2];
        // SAFETY: new private nonblocking pipe, initialized two-FD output.
        assert_eq!(
            unsafe { libc::pipe2(raw.as_mut_ptr(), libc::O_NONBLOCK | libc::O_CLOEXEC) },
            0
        );
        // SAFETY: both new descriptors transferred once.
        let (reader, writer) = unsafe { (File::from_raw_fd(raw[0]), File::from_raw_fd(raw[1])) };
        let rights = [writer.as_raw_fd(); 5];
        if matches!(fault, 0 | 3) {
            channel::send(&peer, &[0], &rights[..1]).unwrap();
        } else {
            channel::response(&peer, 0).unwrap();
            channel::response(&peer, 1).unwrap();
            let bytes = frame(Stage::Initial, 123);
            channel::send(
                &peer,
                &bytes[..if fault == 1 { 31 } else { 32 }],
                &rights[..if fault == 2 { 4 } else { 5 }],
            )
            .unwrap();
        }
        // SAFETY: current test process only; fault3 deliberately expects a nonpeer.
        let expected = if fault == 3 {
            0
        } else {
            (unsafe { libc::getpid() }) as u32
        };
        let report = channel::receive(&parent, expected, Stage::Initial);
        assert_eq!(report.is_ok(), fault == 4);
        drop(writer);
        let mut byte = 0u8;
        // SAFETY: nonblocking one-byte read on this original owned pipe.
        let before = unsafe { libc::read(reader.as_raw_fd(), (&mut byte as *mut u8).cast(), 1) };
        if fault == 4 {
            assert_eq!(before, -1);
            assert_eq!(
                io::Error::last_os_error().raw_os_error(),
                Some(libc::EAGAIN)
            );
        } else {
            assert_eq!(before, 0);
        }
        drop(report);
        // SAFETY: no writer may survive receiver refusal or owned report drop.
        assert_eq!(
            unsafe { libc::read(reader.as_raw_fd(), (&mut byte as *mut u8).cast(), 1) },
            0
        );
    }
    println!("received_rights proof=complete");
}
