use super::*;

#[test]
fn signer_namespace_original_environment_survives_nd0_without_reopen_fallback() {
    let mut own = owner(Case::Probe(Probe::Unshare)).unwrap();
    for name in ["ns/user", "root", "cwd", "exe"] {
        println!(
            "post_ND0_magic_{name}={}",
            magic(format!("/proc/{}/{name}", own.pid)).is_ok()
        );
    }
    for name in ["task", "fd"] {
        println!(
            "post_ND0_directory_{name}={}",
            fs::read_dir(format!("/proc/{}/{name}", own.pid))
                .map(|r| r.count())
                .is_ok()
        );
    }
    for name in [
        "uid_map",
        "gid_map",
        "setgroups",
        "status",
        "environ",
        "maps",
    ] {
        println!(
            "post_ND0_text_{name}={}",
            text(format!("/proc/{}/{name}", own.pid)).is_ok()
        );
    }
    assert_eq!(
        text(format!("/proc/{}/environ", own.pid))
            .unwrap_err()
            .kind(),
        io::ErrorKind::PermissionDenied
    );
    assert!(own.empty_environment().unwrap());
    assert_eq!(
        fs::read_dir(format!("/proc/{}/fd", own.pid))
            .unwrap_err()
            .kind(),
        io::ErrorKind::PermissionDenied
    );
    assert_eq!(
        descriptors(own.fd_directory.as_ref().unwrap()).unwrap(),
        ["0", "1", "2"]
            .map(std::ffi::OsString::from)
            .into_iter()
            .collect()
    );
    own.context(false).unwrap();
    println!("post_ND0_original_environ_and_fd_inventory=retained fresh_open=EACCES live_context_checked=true");
    own.refuse().unwrap();
}

#[test]
fn signer_namespace_original_fd_directory_faults_refuse() {
    for case in 0..4 {
        let mut own = owner(Case::Probe(Probe::Unshare)).unwrap();
        match case {
            0 => own.fd_directory = None,
            1 => own.fd_directory.as_mut().unwrap().1[1] ^= 1,
            2 => own.fd_directory.as_mut().unwrap().0 = File::open("/proc/self/fd").unwrap(),
            _ => {
                // SAFETY: only this owned fixture's descriptor flags are changed.
                assert_eq!(
                    unsafe {
                        libc::fcntl(
                            own.fd_directory.as_ref().unwrap().0.as_raw_fd(),
                            libc::F_SETFD,
                            0,
                        )
                    },
                    0
                );
            }
        }
        assert!(finish(&mut own).is_err());
        assert!(own.reaped && own.root.refused);
        assert_eq!(own.phase, Phase::Refused);
    }
}

#[test]
fn signer_namespace_directory_record_decoder_refuses_malformed_inventory() {
    fn record(name: &str, kind: u8) -> Vec<u8> {
        let size = (20 + name.len() + 7) & !7;
        let mut out = vec![0; size];
        out[16..18].copy_from_slice(&(size as u16).to_le_bytes());
        out[18] = kind;
        out[19..19 + name.len()].copy_from_slice(name.as_bytes());
        out
    }
    let good = record("0", libc::DT_LNK);
    let mut names = BTreeSet::new();
    directory_records(&good, &mut names).unwrap();
    assert!(directory_records(&good, &mut names).is_err());
    for raw in [
        vec![0; 513],
        vec![0; 18],
        vec![0; 24],
        record("x", libc::DT_LNK),
        record("0", libc::DT_DIR),
        record(".", libc::DT_LNK),
        record("", libc::DT_LNK),
        record("12345678901", libc::DT_LNK),
    ] {
        assert!(directory_records(&raw, &mut BTreeSet::new()).is_err());
    }
    for length in [0u16, 19, 23, 25, 512, 65535] {
        let mut bad = good.clone();
        bad[16..18].copy_from_slice(&length.to_le_bytes());
        assert!(directory_records(&bad, &mut BTreeSet::new()).is_err());
    }
    let mut missing_zero = good.clone();
    missing_zero[19..].fill(b'0');
    assert!(directory_records(&missing_zero, &mut BTreeSet::new()).is_err());
    let mut full = BTreeSet::new();
    for n in 0..8 {
        directory_records(&record(&n.to_string(), libc::DT_LNK), &mut full).unwrap();
    }
    assert!(directory_records(&record("8", libc::DT_LNK), &mut full).is_err());
}

#[test]
fn signer_namespace_missing_foreign_or_weakened_environment_fd_refuses() {
    for case in 0..5 {
        let mut own = owner(Case::Probe(Probe::Unshare)).unwrap();
        match case {
            0 => own.environment = None,
            1 => own.environment.as_mut().unwrap().1[1] ^= 1,
            2 => own.environment.as_mut().unwrap().0 = tempfile::tempfile().unwrap(),
            3 => own.environment.as_mut().unwrap().0 = File::open("/proc/self/status").unwrap(),
            _ => {
                // SAFETY: remove CLOEXEC only from this owned fixture's descriptor.
                assert_eq!(
                    unsafe {
                        libc::fcntl(
                            own.environment.as_ref().unwrap().0.as_raw_fd(),
                            libc::F_SETFD,
                            0,
                        )
                    },
                    0
                );
            }
        }
        assert!(finish(&mut own).is_err(), "environment fault={case}");
        assert_eq!(own.phase, Phase::Refused);
        assert!(own.reaped && own.root.refused);
    }
}

fn action(program: &[libc::sock_filter], arch: u32, nr: u32, args: [u64; 2]) -> Option<u32> {
    let mut pc = 0usize;
    let mut a = 0u32;
    for _ in 0..program.len() * 2 {
        let ins = program.get(pc)?;
        match ins.code {
            0x20 => {
                a = match ins.k {
                    0 => nr,
                    4 => arch,
                    16 => args[0] as u32,
                    20 => (args[0] >> 32) as u32,
                    24 => args[1] as u32,
                    28 => (args[1] >> 32) as u32,
                    _ => return None,
                };
                pc = pc.checked_add(1)?;
            }
            0x15 | 0x45 => {
                let yes = if ins.code == 0x15 {
                    a == ins.k
                } else {
                    a & ins.k != 0
                };
                pc = pc.checked_add(
                    1 + if yes {
                        ins.jt as usize
                    } else {
                        ins.jf as usize
                    },
                )?;
            }
            0x06 => return Some(ins.k),
            _ => return None,
        }
    }
    None
}
#[test]
fn signer_namespace_policy_exact_extension_and_abi_model() {
    let new = program();
    let old = policy();
    assert_eq!(encoded(&new).len(), 512);
    assert_eq!(
        encoded(&new).as_slice(),
        crate::diagnostic_signer_isolation::policy_bytes().as_slice()
    );
    assert_eq!(encoded(&new[..22]), encoded(&old[..22]));
    assert_eq!(encoded(&new[52..]), encoded(&old[22..]));
    assert_eq!(
        DENIED,
        [
            libc::SYS_unshare as u32,
            libc::SYS_setns as u32,
            libc::SYS_chroot as u32,
            libc::SYS_chdir as u32,
            libc::SYS_fchdir as u32,
            libc::SYS_mount as u32,
            libc::SYS_umount2 as u32,
            libc::SYS_pivot_root as u32,
            libc::SYS_open_tree as u32,
            libc::SYS_move_mount as u32,
            libc::SYS_fsopen as u32,
            libc::SYS_fsconfig as u32,
            libc::SYS_fsmount as u32,
            libc::SYS_fspick as u32,
            libc::SYS_mount_setattr as u32
        ]
    );
    for nr in 0..1024 {
        let baseline = action(&old, ARCH, nr, [0; 2]).unwrap();
        assert_eq!(
            action(&new, ARCH, nr, [0; 2]),
            Some(if DENIED.contains(&nr) {
                0x0005_0001
            } else {
                baseline
            }),
            "nr={nr}"
        );
        assert_eq!(
            action(&new, ARCH, nr | 0x4000_0000, [0; 2]),
            Some(libc::SECCOMP_RET_KILL_PROCESS)
        );
        assert_eq!(
            action(&new, 0x4000_0003, nr, [0; 2]),
            Some(libc::SECCOMP_RET_KILL_PROCESS)
        );
    }
    assert_eq!(
        action(&new, ARCH, 157, [4, 0]),
        Some(libc::SECCOMP_RET_ALLOW)
    );
    for args in [[4, 1], [4 | 1 << 32, 0], [4, 1 << 32], [22, 0]] {
        assert_eq!(action(&new, ARCH, 157, args), Some(0x0005_0001));
    }
    for control in [
        Case::ControlUnshare,
        Case::ControlChdir,
        Case::ControlFchdir,
        Case::ControlSetns,
    ] {
        assert_eq!(expected(control), encoded(&old));
    }
    println!("namespace_policy_instructions=64 bytes=512 added_native_denials=15 old_prefix_tail_exact=true");
}
#[test]
fn signer_namespace_policy_mutations_and_native_args_are_bounded() {
    let original = program();
    assert!(action(&[], ARCH, 39, [0; 2]).is_none());
    let mut bad = original;
    bad[0].code = 0xffff;
    assert!(action(&bad, ARCH, 39, [0; 2]).is_none());
    bad = original;
    bad[0].k = 4096;
    assert!(action(&bad, ARCH, 39, [0; 2]).is_none());
    bad = original;
    bad[1].jt = 255;
    assert!(action(&bad, ARCH, 39, [0; 2]).is_none());
    let mut seed = 0x0740_3b2b_u64;
    for _ in 0..256 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let nr = DENIED[seed as usize % DENIED.len()];
        assert_eq!(
            action(&original, ARCH, nr, [seed, seed.rotate_left(31)]),
            Some(0x0005_0001)
        );
        let mut altered = original;
        altered[seed as usize % 64].k ^= seed as u32;
        let _ = action(&altered, ARCH, seed as u32, [seed; 2]);
    }
    println!("namespace_policy_seed=0x07403b2b bounded_mutations=256");
}
#[test]
fn signer_namespace_fifteen_native_mutations_denied_after_exact_install() {
    for probe in PROBES {
        let mut own = owner(Case::Probe(probe)).unwrap();
        assert_eq!(own.program_size(), 512);
        finish(&mut own).unwrap();
        assert_eq!(own.phase, Phase::Terminal);
        assert!(own.reaped);
    }
    println!("post_install_native_namespace_root_mount_denials=15 original_context_unchanged=true handoff=false");
}
#[test]
fn signer_namespace_old_policy_controls_retain_negative_namespace_regain_result() {
    let parent_ns = identity(&magic("/proc/self/ns/user").unwrap()).unwrap();
    let parent_root = identity(&magic("/").unwrap()).unwrap();
    // SAFETY: local original-parent identity queries only.
    let ids = unsafe { (libc::getuid(), libc::getgid()) };
    for case in [
        Case::ControlUnshare,
        Case::ControlChdir,
        Case::ControlFchdir,
        Case::ControlSetns,
    ] {
        let mut own = owner(case).unwrap();
        assert_eq!(own.program_size(), 272);
        finish(&mut own).unwrap();
        assert!(own.reaped);
        assert_eq!(
            own.phase,
            if case == Case::ControlUnshare {
                Phase::Refused
            } else {
                Phase::Terminal
            }
        );
    }
    assert_eq!(
        identity(&magic("/proc/self/ns/user").unwrap()).unwrap(),
        parent_ns
    );
    assert_eq!(identity(&magic("/").unwrap()).unwrap(), parent_root);
    // SAFETY: same local parent, no identity mutation.
    assert_eq!(unsafe { (libc::getuid(), libc::getgid()) }, ids);
    println!("old_policy_controls=4 parent_namespace_root_uid_gid_unchanged=true");
}
#[test]
fn signer_namespace_exec_denials_and_actual_alternate_abi_sigsys() {
    for case in [Case::SelfExec, Case::OtherExec] {
        let mut own = owner(case).unwrap();
        own.denied().unwrap();
        assert_eq!(own.phase, Phase::Terminal);
    }
    for case in [Case::X32, Case::I386] {
        let mut own = owner(case).unwrap();
        fatal(&mut own).unwrap();
        assert!(own.reaped);
        assert_eq!(own.phase, Phase::Terminal);
    }
    println!("namespace_candidate_x32_i386=SIGSYS same_child_original_exec_install=true");
}
#[test]
fn signer_namespace_installer_startup_and_environment_faults_refuse() {
    for case in [
        Case::Flags,
        Case::Policy,
        Case::Length,
        Case::Header,
        Case::Pointer,
        Case::EarlyExec,
        Case::EarlyMount,
        Case::Env,
    ] {
        let mut own = RootedExec::namespace(case).unwrap();
        own.enter().unwrap();
        let result = own.first_exec().and_then(|()| own.install());
        assert!(result.is_err(), "fault={case:?}");
        assert_eq!(own.phase, Phase::Refused);
        assert!(own.reaped && own.root.refused);
        assert!(own.install().is_err());
    }
}
#[test]
fn signer_namespace_context_phase_replay_and_memory_bounds_refuse() {
    use std::os::unix::fs::PermissionsExt;
    for case in 0..5 {
        let mut own = RootedExec::namespace(Case::Probe(Probe::Unshare)).unwrap();
        let result = match case {
            0 => finish(&mut own),
            1 => {
                own.enter().unwrap();
                own.namespace = Some(magic("/proc/self/ns/user").unwrap());
                own.first_exec()
            }
            2 => {
                own.enter().unwrap();
                fs::set_permissions(&own.root.path, fs::Permissions::from_mode(0o755)).unwrap();
                own.first_exec()
            }
            3 => {
                own.enter().unwrap();
                own.first_exec().unwrap();
                own.install().unwrap();
                own.install()
            }
            _ => {
                own.enter().unwrap();
                own.first_exec().unwrap();
                own.install().unwrap();
                finish(&mut own).unwrap();
                finish(&mut own)
            }
        };
        assert!(result.is_err());
        assert_eq!(own.phase, Phase::Refused);
        assert!(own.reaped && own.root.refused);
    }
    let mut own = RootedExec::namespace(Case::Probe(Probe::Unshare)).unwrap();
    for (address, count) in [
        (0, 16),
        (0x1000, 0),
        (0x1000, 513),
        (0x2fff, 16),
        (usize::MAX - 2, 16),
    ] {
        assert!(own.memory((0x1000, 0x3000), address, count).is_err());
    }
    own.refuse().unwrap();
}
#[test]
fn signer_namespace_sigkill_and_drop_at_four_original_boundaries() {
    for killed in [true, false] {
        for phase in 0..4 {
            let mut own = RootedExec::namespace(Case::Probe(Probe::Unshare)).unwrap();
            if phase >= 1 {
                own.enter().unwrap();
            }
            if phase >= 2 {
                own.first_exec().unwrap();
            }
            if phase >= 3 {
                own.install().unwrap();
            }
            let pid = own.pid;
            let path = own.root.path.clone();
            if killed {
                // SAFETY: this exact original owned pidfd only.
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
                assert!(finish(&mut own).is_err());
                assert_eq!(own.phase, Phase::Refused);
                assert!(own.reaped && own.root.refused);
            }
            drop(own);
            let mut status = 0;
            // SAFETY: diagnostic wait of this already-reaped original child only.
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
    println!("namespace_original_SIGKILL_drop=initial,rooted,first_exec,installed tracer_loss_proved=false");
}
#[test]
fn signer_namespace_seeded_fresh_probes_and_independent_owners() {
    let mut seed = 0x0740_3b2b_u64;
    for _ in 0..8 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut own = owner(Case::Probe(PROBES[seed as usize % 15])).unwrap();
        finish(&mut own).unwrap();
    }
    let threads: Vec<_> = (0..4)
        .map(|i| {
            std::thread::spawn(move || {
                let mut own = owner(Case::Probe(PROBES[i])).unwrap();
                finish(&mut own).unwrap();
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    println!("namespace_probe_seed=0x07403b2b fresh_probes=8 independent_owners=4 multithreaded_signer=false");
}
