use super::*;
#[test]
fn rooted_exec_original_selected_root_exec_and_install() {
    let mut owner = RootedExec::new(Case::SelfExec).unwrap();
    owner.enter().unwrap();
    owner.first_exec().unwrap();
    owner.install().unwrap();
    owner.denied().unwrap();
    assert_eq!(owner.phase, Phase::Terminal);
    println!("rooted_original_exec_and_exact_install=same_child selected_VMA_file_ids=4 memory_attested=false handoff=false");
}

#[test]
fn rooted_exec_other_image_is_denied_after_install() {
    let mut owner = RootedExec::new(Case::OtherExec).unwrap();
    owner.enter().unwrap();
    owner.first_exec().unwrap();
    owner.install().unwrap();
    owner.denied().unwrap();
    assert!(owner.reaped);
    assert_eq!(owner.phase, Phase::Terminal);
}

fn synthetic_maps(root: &Root, loaded: bool) -> String {
    let mut raw = String::new();
    for (i, file) in root
        .files
        .iter()
        .take(if loaded { 4 } else { 2 })
        .enumerate()
    {
        let start = 0x400000 + i * 0x10000;
        raw.push_str(&format!(
            "{start:x}-{:x} r-xp 00000000 {:x}:{:x} {} /{}\n",
            start + 4096,
            libc::major(file.stamp.0[0]),
            libc::minor(file.stamp.0[0]),
            file.stamp.0[1],
            super::super::FILES[i],
        ));
    }
    raw.push_str("600000-601000 rw-p 00000000 00:00 0\n610000-611000 rw-p 00000000 00:00 0 [heap]\n700000-702000 rw-p 00000000 00:00 0 [stack]\n800000-801000 r--p 00000000 00:00 0 [vvar]\n810000-811000 r--p 00000000 00:00 0 [vvar_vclock]\n820000-821000 r-xp 00000000 00:00 0 [vdso]\nffffffffff600000-ffffffffff601000 --xp 00000000 00:00 0 [vsyscall]\n");
    raw
}

#[test]
fn rooted_exec_maps_require_exact_selected_file_and_pseudo_identities() {
    let root = Root::new().unwrap();
    let raw = synthetic_maps(&root, true);
    assert_eq!(
        maps::validate(&raw, &root, true).unwrap(),
        (0x700000, 0x702000)
    );
    let first = synthetic_maps(&root, false);
    maps::validate(&first, &root, false).unwrap();
    assert!(maps::validate(&first, &root, true).is_err());
    assert!(maps::validate(&raw, &root, false).is_err());
    let absolute = raw.replace(" /", &format!(" {}/", root.path.display()));
    maps::validate(&absolute, &root, true).unwrap();
    let bad: Vec<String> = vec![
        String::new(),
        raw.trim_end().to_owned(),
        "x".repeat(65537),
        raw.replace("400000-401000", "0-401000"),
        raw.replace("400000-401000", "400001-401000"),
        raw.replace("400000-401000", "401000-400000"),
        raw.replace("400000-401000", "400000-411000"),
        raw.replace("400000-401000", "fffffffffffffffff-401000"),
        raw.replace("r-xp", "rwxp"),
        raw.replace("r-xp", "r-xs"),
        raw.replace("r-xp", "r--p"),
        raw.replace("00000000", "00000001"),
        raw.replacen("00000000", "fffffffffffff000", 1),
        raw.replace("/signer", "/signer(deleted)"),
        raw.replace("/signer", "/signer (deleted)"),
        raw.replace("/signer", "/foreign"),
        raw.replace(
            &format!(" {} /signer", root.files[0].stamp.0[1]),
            " 18446744073709551615 /signer",
        ),
        raw.replace("00:00 0 [stack]", "01:00 0 [stack]"),
        raw.replace("[stack]", "[unknown]"),
        raw.replace("[vdso]", "[unknown]"),
        raw.replace("700000-702000 rw-p", "700000-702000 r-xp"),
        raw.replace("600000-601000 rw-p", "600000-601000 r-xp"),
        raw.replace("ffffffffff600000", "ffffffffff602000"),
        raw.replace("820000-821000 r-xp", "820000-821000 rw-p"),
        raw.replace("810000-811000", "800000-801000"),
        raw.replace("[vvar_vclock]", "[vvar]"),
        raw.replace("[vvar_vclock]", "[stack]"),
        raw.replace("[vvar_vclock]", "[vdso]"),
        raw.replace("[vsyscall]", "[vdso]"),
        raw.lines()
            .skip(1)
            .map(|line| format!("{line}\n"))
            .collect(),
        "1000-2000 rw-p 00000000 00:00 0\n".repeat(129),
    ];
    for (case, value) in bad.iter().enumerate() {
        assert!(
            maps::validate(value, &root, true).is_err(),
            "map fault={case}"
        );
    }
    // Equal bytes do not substitute for original VMA device/inode identity.
    let foreign = Root::new().unwrap();
    assert!(maps::validate(&raw, &foreign, true).is_err());
}

#[test]
fn rooted_exec_maps_seeded_parser_inputs_do_not_panic() {
    let root = Root::new().unwrap();
    let raw = synthetic_maps(&root, true);
    let mut seed = 0x0740_3b2a_u64;
    for _ in 0..256 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut bytes = raw.as_bytes().to_vec();
        bytes[seed as usize % raw.len()] = (seed >> 32) as u8;
        let value = String::from_utf8_lossy(&bytes);
        let _ = maps::validate(&value, &root, true);
    }
    println!("maps_mutation_seed=0x07403b2a cases=256 memory_attested=false");
}

#[test]
fn rooted_exec_exact_installer_faults_refuse_stickily() {
    for case in [
        Case::Flags,
        Case::Policy,
        Case::Length,
        Case::Header,
        Case::Pointer,
        Case::EarlyExec,
    ] {
        let mut owner = RootedExec::new(case).unwrap();
        owner.enter().unwrap();
        owner.first_exec().unwrap();
        assert!(owner.install().is_err(), "installer fault={case:?}");
        assert_eq!(owner.phase, Phase::Refused);
        assert!(owner.reaped && owner.root.refused);
        assert!(owner.install().is_err());
    }
}

#[test]
fn rooted_exec_nonempty_environment_refuses_at_original_exec_stop() {
    let mut owner = RootedExec::new(Case::Env).unwrap();
    owner.enter().unwrap();
    assert!(owner.first_exec().is_err());
    assert_eq!(owner.phase, Phase::Refused);
    assert!(owner.reaped && owner.root.refused);
}

#[test]
fn rooted_exec_original_tree_hash_and_namespace_drift_refuse() {
    use std::os::unix::fs::PermissionsExt;
    for case in 0..4 {
        let mut owner = RootedExec::new(Case::SelfExec).unwrap();
        owner.enter().unwrap();
        match case {
            0 => owner.namespace = Some(magic("/proc/self/ns/user").unwrap()),
            1 => fs::set_permissions(&owner.root.path, fs::Permissions::from_mode(0o755)).unwrap(),
            2 => {
                let path = owner.root.path.join(super::super::FILES[2]);
                let mut bytes = super::super::read_bounded(&owner.root.files[2].file).unwrap();
                *bytes.last_mut().unwrap() ^= 1;
                fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
                fs::write(&path, bytes).unwrap();
                fs::set_permissions(&path, fs::Permissions::from_mode(0o555)).unwrap();
                // Synthetic metadata reset isolates the retained digest check.
                owner.root.files[2].stamp =
                    super::super::Stamp::of(owner.root.files[2].file.metadata().unwrap());
            }
            _ => {
                let dir = owner.root.path.join(super::super::DIRS[2]);
                fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
                fs::write(dir.join("foreign.so"), b"fixture").unwrap();
            }
        }
        assert!(owner.first_exec().is_err(), "context fault={case}");
        assert_eq!(owner.phase, Phase::Refused);
        assert!(owner.reaped && owner.root.refused);
    }
}

#[test]
fn rooted_exec_phase_replay_native_numbers_and_stack_bounds_refuse() {
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
        libc::SYS_unshare,
        libc::SYS_setns,
        libc::SYS_chroot,
        libc::SYS_chdir,
        libc::SYS_fchdir,
        libc::SYS_mount,
        libc::SYS_umount2,
        libc::SYS_pivot_root,
    ] {
        for number in [nr as u64, nr as u64 | 1 << 32, nr as u64 | 0x4000_0000] {
            assert!(!startup_allowed(number));
        }
    }
    for nr in [1024, u32::MAX as u64, u64::MAX] {
        assert!(!startup_allowed(nr));
    }
    assert!(startup_allowed(libc::SYS_getpid as u64));
    let owner = RootedExec::new(Case::SelfExec).unwrap();
    for (address, count) in [
        (0, 16),
        (0x1000, 0),
        (0x1000, 273),
        (0x2fff, 16),
        (usize::MAX - 2, 16),
    ] {
        assert!(owner.memory((0x1000, 0x3000), address, count).is_err());
    }
    drop(owner);
    for phase in 0..6 {
        let mut owner = RootedExec::new(Case::SelfExec).unwrap();
        let result = match phase {
            0 => owner.first_exec(),
            1 => owner.install(),
            2 => {
                owner.enter().unwrap();
                owner.enter()
            }
            3 => {
                owner.enter().unwrap();
                owner.first_exec().unwrap();
                owner.first_exec()
            }
            4 => {
                owner.enter().unwrap();
                owner.first_exec().unwrap();
                owner.install().unwrap();
                owner.install()
            }
            _ => {
                owner.enter().unwrap();
                owner.first_exec().unwrap();
                owner.install().unwrap();
                owner.denied().unwrap();
                owner.denied()
            }
        };
        assert!(result.is_err());
        assert_eq!(owner.phase, Phase::Refused);
        assert!(owner.reaped && owner.root.refused);
    }
}

#[test]
fn rooted_exec_sigkill_at_four_boundaries_reaps_only_original() {
    for phase in 0..4 {
        let mut owner = RootedExec::new(Case::SelfExec).unwrap();
        if phase >= 1 {
            owner.enter().unwrap();
        }
        if phase >= 2 {
            owner.first_exec().unwrap();
        }
        if phase >= 3 {
            owner.install().unwrap();
        }
        // SAFETY: exact owned original pidfd; no process enumeration or adoption.
        assert_eq!(
            unsafe {
                libc::syscall(
                    libc::SYS_pidfd_send_signal,
                    owner.pidfd.as_ref().unwrap().as_raw_fd(),
                    libc::SIGKILL,
                    0usize,
                    0u32,
                )
            },
            0
        );
        let status = owner.wait().unwrap();
        assert!(libc::WIFSIGNALED(status));
        assert_eq!(libc::WTERMSIG(status), libc::SIGKILL);
        assert!(owner.reaped);
        assert!(owner.install().is_err());
        assert_eq!(owner.phase, Phase::Refused);
        assert!(owner.root.refused);
    }
    println!("owned_rooted_exec_SIGKILL=initial,rooted,first_exec,installed original_reaped=true tracer_loss_proved=false");
}

#[test]
fn rooted_exec_healthy_drop_reaps_original_without_replacement() {
    for phase in 0..4 {
        let mut owner = RootedExec::new(Case::SelfExec).unwrap();
        if phase >= 1 {
            owner.enter().unwrap();
        }
        if phase >= 2 {
            owner.first_exec().unwrap();
        }
        if phase >= 3 {
            owner.install().unwrap();
        }
        let pid = owner.pid;
        let path = owner.root.path.clone();
        drop(owner);
        // SAFETY: only a diagnostic wait for this already-reaped own child.
        let mut status = 0;
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

#[test]
fn rooted_exec_seeded_fresh_execs_and_independent_nonroot_owners() {
    let parent_namespace = identity(&magic("/proc/self/ns/user").unwrap()).unwrap();
    let parent_root = identity(&magic("/").unwrap()).unwrap();
    let mut seed = 0x0740_3b2a_u64;
    for _ in 0..8 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut owner = RootedExec::new(if seed & 1 == 0 {
            Case::SelfExec
        } else {
            Case::OtherExec
        })
        .unwrap();
        owner.enter().unwrap();
        owner.first_exec().unwrap();
        owner.install().unwrap();
        owner.denied().unwrap();
    }
    let threads: Vec<_> = (0..4)
        .map(|_| {
            std::thread::spawn(|| {
                let mut owner = RootedExec::new(Case::SelfExec).unwrap();
                owner.enter().unwrap();
                owner.first_exec().unwrap();
                owner.install().unwrap();
                owner.denied().unwrap();
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    assert_eq!(
        identity(&magic("/proc/self/ns/user").unwrap()).unwrap(),
        parent_namespace
    );
    assert_eq!(identity(&magic("/").unwrap()).unwrap(), parent_root);
    println!("rooted_exec_seed=0x07403b2a fresh_execs=8 independent_owners=4 parent_root_changed=false actual_multithread_signer=false");
}
