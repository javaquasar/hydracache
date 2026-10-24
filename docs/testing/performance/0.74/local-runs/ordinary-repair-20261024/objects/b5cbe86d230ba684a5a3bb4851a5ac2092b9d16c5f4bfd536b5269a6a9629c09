use super::*;

#[test]
fn signer_joined_image_preparation_has_no_parent_writer_to_inherit() {
    // Deliberate external-writer counterexample remains kernel-refused after
    // the preparation fix: this is not whole-writer revocation.
    let foreign = Fixture::new().unwrap();
    fs::set_permissions(&foreign.binary, fs::Permissions::from_mode(0o755)).unwrap();
    let writer = OpenOptions::new()
        .write(true)
        .open(&foreign.binary)
        .unwrap();
    fs::set_permissions(&foreign.binary, fs::Permissions::from_mode(0o555)).unwrap();
    let snapshot = Snapshot::new(Some(writer.as_raw_fd())).unwrap();
    drop(writer);
    let error = match Joined::launch(&foreign, Case::SelfExec) {
        Err(error) => error,
        Ok(_) => panic!("inherited writable executable was unexpectedly admitted"),
    };
    assert!(error.to_string().contains("0x9a00"), "{error}");
    drop(snapshot);
    println!("inherited_writer_counterexample=ETXTBSY whole_writer_revocation=false");
    let fixture = Fixture::new_with_snapshot(true).unwrap();
    assert!(fixture._snapshot.is_some());
    let mut owner = Joined::launch(&fixture, Case::SelfExec).unwrap();
    owner.install().unwrap();
    owner.denied_probe(Case::SelfExec).unwrap();
    println!("image_preparation_writer=isolated_copy_child stopped_parent_snapshot_does_not_block_exec=true");
}

#[test]
fn signer_joined_original_exec_and_installation_are_one_child() {
    let fixture = Fixture::new().unwrap();
    let mut owner = Joined::launch(&fixture, Case::SelfExec).unwrap();
    owner.install().unwrap();
    owner.denied_probe(Case::SelfExec).unwrap();
    println!("fresh_exec_and_exact_install=same_owned_child credential_handoff=false runtime_custody=false");
}

#[test]
fn signer_joined_other_exec_and_descriptor_exec_are_denied() {
    for case in [Case::OtherExec, Case::FdExec] {
        let fixture = Fixture::new().unwrap();
        let mut owner = Joined::launch(&fixture, case).unwrap();
        assert_eq!(owner.phase, Phase::FirstExecStopped);
        assert_eq!(owner.output.metadata().unwrap().len(), 0);
        owner.install().unwrap();
        owner.denied_probe(case).unwrap();
        assert_eq!(owner.phase, Phase::Terminal);
        assert!(owner.reaped);
    }
}

#[test]
fn signer_joined_flags_policy_length_and_nonstack_inputs_refuse() {
    for case in [
        Case::Flags,
        Case::Policy,
        Case::Length,
        Case::Header,
        Case::Pointer,
    ] {
        let fixture = Fixture::new().unwrap();
        let mut owner = Joined::launch(&fixture, case).unwrap();
        assert!(owner.install().is_err());
        assert_eq!(owner.phase, Phase::Refused);
        assert!(owner.reaped);
        assert_eq!(owner.output.metadata().unwrap().len(), 0);
        assert!(owner.install().is_err());
    }
}

#[test]
fn signer_joined_early_exec_is_refused_before_syscall_dispatch() {
    let fixture = Fixture::new().unwrap();
    let mut owner = Joined::launch(&fixture, Case::EarlyExec).unwrap();
    assert!(owner.install().is_err());
    assert_eq!(owner.phase, Phase::Refused);
    assert!(owner.reaped);
    assert_eq!(owner.output.metadata().unwrap().len(), 0);
    println!(
        "pre_install_exec=entry_refused original_child_reaped=true new_image_not_dispatched=true"
    );
}

#[test]
fn signer_joined_phase_replay_and_stack_range_inputs_refuse() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("bounded-image");
    fs::write(&path, [0u8; 64]).unwrap();
    assert_eq!(read_image(&path).unwrap().len(), 64);
    for len in [0, 63, MAX_IMAGE as u64 + 1] {
        File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_len(len)
            .unwrap();
        assert!(read_image(&path).is_err());
    }
    for nr in [
        libc::SYS_execve,
        libc::SYS_execveat,
        libc::SYS_clone,
        libc::SYS_clone3,
        libc::SYS_fork,
        libc::SYS_vfork,
        libc::SYS_ptrace,
        libc::SYS_prctl,
        libc::SYS_seccomp,
        libc::SYS_exit,
        libc::SYS_exit_group,
    ] {
        assert!(!startup_allowed(nr as u64));
        assert!(!startup_allowed(nr as u64 | 1 << 32));
        assert!(!startup_allowed(nr as u64 | 0x4000_0000));
    }
    for nr in [1024, u32::MAX as u64, u64::MAX] {
        assert!(!startup_allowed(nr));
    }
    assert!(startup_allowed(libc::SYS_getpid as u64));
    let line = "1000-3000 rw-p 00000000 00:00 0 [stack]\n";
    assert_eq!(stack_bounds(line).unwrap(), (0x1000, 0x3000));
    for bad in [
        String::new(),
        line.repeat(2),
        line.replace("rw-p", "rwxp"),
        line.replace("1000", "1001"),
        line.replace("3000", "0000"),
        line.replace("00:00", "01:00"),
        line.replace(" 0 [", " 1 ["),
        line.replace("1000", "ffffffffffffffffffffffff"),
        "x".repeat(65537),
    ] {
        assert!(stack_bounds(&bad).is_err());
    }
    for (address, count) in [
        (0usize, 16usize),
        (0x1000, 0),
        (0x1000, 273),
        (0x2fff, 16),
        (usize::MAX - 2, 16),
    ] {
        assert!(!within((0x1000, 0x3000), address, count));
    }
    assert!(within((0x1000, 0x3000), 0x2000, 272));
    for phase in 0..3 {
        let fixture = Fixture::new().unwrap();
        let mut owner = Joined::launch(&fixture, Case::SelfExec).unwrap();
        if phase == 0 {
            assert!(owner.denied_probe(Case::SelfExec).is_err());
        } else {
            owner.install().unwrap();
            if phase == 2 {
                owner.denied_probe(Case::SelfExec).unwrap();
            }
            assert!(owner.install().is_err());
        }
        assert_eq!(owner.phase, Phase::Refused);
        assert!(owner.reaped);
    }
}

#[test]
fn signer_joined_owned_child_sigkill_at_exec_and_installed_boundaries() {
    for installed in [false, true] {
        let fixture = Fixture::new().unwrap();
        let mut owner = Joined::launch(&fixture, Case::SelfExec).unwrap();
        if installed {
            owner.install().unwrap();
        }
        owner.signal_original().unwrap();
        let status = owner.wait().unwrap();
        assert!(libc::WIFSIGNALED(status));
        assert_eq!(libc::WTERMSIG(status), libc::SIGKILL);
        assert!(owner.reaped);
        assert!(owner.install().is_err());
        assert_eq!(owner.phase, Phase::Refused);
        assert_eq!(owner.output.metadata().unwrap().len(), 0);
    }
    println!("owned_child_SIGKILL_boundaries=first_exec,installed no_handoff=true tracer_loss_proved=false");
}

#[test]
fn signer_joined_foreign_inode_metadata_and_nonempty_output_refuse() {
    for fault in 0..3 {
        let fixture = Fixture::new().unwrap();
        let foreign = Fixture::new().unwrap();
        let mut owner = Joined::launch(&fixture, Case::SelfExec).unwrap();
        match fault {
            0 => owner.expected = File::open(&foreign.binary).unwrap(),
            1 => fs::set_permissions(&fixture.binary, fs::Permissions::from_mode(0o755)).unwrap(),
            _ => fs::write(&fixture.output, b"not-ready\n").unwrap(),
        }
        assert!(owner.install().is_err());
        assert_eq!(owner.phase, Phase::Refused);
        assert!(owner.reaped);
    }
}

#[test]
fn signer_joined_seeded_originals_and_independent_tracer_threads() {
    let mut seed = 0x0740_3b27_u64;
    for _ in 0..8 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let case = [Case::SelfExec, Case::OtherExec, Case::FdExec][seed as usize % 3];
        let fixture = Fixture::new().unwrap();
        let mut owner = Joined::launch(&fixture, case).unwrap();
        owner.install().unwrap();
        owner.denied_probe(case).unwrap();
    }
    let threads: Vec<_> = (0..4)
        .map(|_| {
            std::thread::spawn(|| {
                let fixture = Fixture::new().unwrap();
                let mut owner = Joined::launch(&fixture, Case::FdExec).unwrap();
                owner.install().unwrap();
                owner.denied_probe(Case::FdExec).unwrap();
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    println!("seed=0x07403b27 fresh_exec_install_probes=8 independent_tracer_threads=4 production_multithreading=false");
}
