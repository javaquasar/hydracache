use super::*;
use std::os::unix::fs::symlink;
#[test]
fn signer_root_original_kernel_root_is_not_signer_launch() {
    let parent_ns = identity(&magic("/proc/self/ns/user").unwrap()).unwrap();
    let parent_root = identity(&magic("/").unwrap()).unwrap();
    let mut root = Root::new().unwrap();
    let mut owner = Owner::new(&mut root).unwrap();
    owner.enter().unwrap();
    owner.probe().unwrap();
    assert_eq!(owner.phase, Phase::Terminal);
    assert!(owner.reaped);
    drop(owner);
    root.check().unwrap();
    assert_eq!(
        identity(&magic("/proc/self/ns/user").unwrap()).unwrap(),
        parent_ns
    );
    assert_eq!(identity(&magic("/").unwrap()).unwrap(), parent_root);
    println!("owned_nonroot_child_root=selected_inode namespace=child_only caps=zero host_paths=absent signer_exec=false custody=false");
}

#[test]
fn signer_root_bounded_mapping_and_credentials_refuse_unknown_or_weakened_forms() {
    mapping("      1000       1000          1\n", 1000).unwrap();
    for raw in [
        "",
        "0 1000 1",
        "1000 0 1",
        "1000 1000 2",
        "1000 1000 1\n2 2 1",
        "1000 1000 x",
    ] {
        assert!(mapping(raw, 1000).is_err());
    }
    assert!(mapping(&" ".repeat(4097), 1000).is_err());
    let status = "Uid:\t1000 1000 1000 1000\nGid:\t1000 1000 1000 1000\nCapInh:\t0000000000000000\nCapPrm:\t0000000000000000\nCapEff:\t0000000000000000\nCapBnd:\t0000000000000000\nCapAmb:\t0000000000000000\nNoNewPrivs:\t1\n";
    credentials(status, 1000, 1000).unwrap();
    for line in status.lines() {
        assert!(credentials(&status.replace(&format!("{line}\n"), ""), 1000, 1000).is_err());
        assert!(credentials(&format!("{status}{line}\n"), 1000, 1000).is_err());
    }
    assert!(credentials(&status.replace("1000", "0"), 1000, 1000).is_err());
    assert!(credentials(
        &status.replace("0000000000000000", "0000000000000001"),
        1000,
        1000
    )
    .is_err());
    assert!(credentials(
        &status.replace("NoNewPrivs:\t1", "NoNewPrivs:\t0"),
        1000,
        1000
    )
    .is_err());
    assert!(credentials(status, 0, 1000).is_err());
    assert!(credentials(&"x".repeat(65537), 1000, 1000).is_err());
}

#[test]
fn signer_root_exact_tree_files_and_hash_drift_refuse_stickily() {
    for case in 0..8 {
        let mut root = Root::new().unwrap();
        let path = root.path.join(FILES[2]);
        match case {
            0 => fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap(),
            1 => fs::set_permissions(&root.path, fs::Permissions::from_mode(0o755)).unwrap(),
            2 | 3 => {
                fs::set_permissions(root.path.join(DIRS[2]), fs::Permissions::from_mode(0o755))
                    .unwrap();
                fs::remove_file(&path).unwrap();
                if case == 3 {
                    symlink(root.path.join(FILES[0]), &path).unwrap();
                }
            }
            4 => {
                fs::set_permissions(root.path.join(DIRS[2]), fs::Permissions::from_mode(0o755))
                    .unwrap();
                fs::write(root.path.join(DIRS[2]).join("foreign.so"), b"fixture").unwrap();
            }
            5 | 6 => {
                let mut bytes = read_bounded(&root.files[2].file).unwrap();
                if case == 5 {
                    *bytes.last_mut().unwrap() ^= 1;
                } else {
                    bytes[0] = 0;
                }
                fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
                fs::write(&path, bytes).unwrap();
                fs::set_permissions(&path, fs::Permissions::from_mode(0o555)).unwrap();
                // Private synthetic reset isolates digest/parser from metadata refusal.
                root.files[2].stamp = Stamp::of(root.files[2].file.metadata().unwrap());
            }
            _ => {
                fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
                root.files[2].file = OpenOptions::new()
                    .read(true)
                    .write(true)
                    .custom_flags(libc::O_CLOEXEC)
                    .open(&path)
                    .unwrap();
                fs::set_permissions(&path, fs::Permissions::from_mode(0o555)).unwrap();
                root.files[2].stamp = Stamp::of(root.files[2].file.metadata().unwrap());
            }
        }
        assert!(root.check().is_err(), "case={case}");
        assert!(root.refused);
        assert!(root.check().is_err());
        assert!(Owner::new(&mut root).is_err());
    }
}

#[test]
fn signer_root_owner_sequence_namespace_and_terminal_replay_refuse() {
    for case in 0..4 {
        let mut root = Root::new().unwrap();
        let mut owner = Owner::new(&mut root).unwrap();
        match case {
            0 => assert!(owner.probe().is_err()),
            1 => {
                owner.enter().unwrap();
                assert!(owner.enter().is_err());
            }
            2 => {
                owner.enter().unwrap();
                owner.namespace = Some(magic("/proc/self/ns/user").unwrap());
                assert!(owner.probe().is_err());
            }
            _ => {
                owner.enter().unwrap();
                owner.probe().unwrap();
                assert!(owner.probe().is_err());
            }
        }
        assert_eq!(owner.phase, Phase::Refused);
        assert!(owner.reaped && owner.root.refused);
    }
}

#[test]
fn signer_root_sigkill_and_healthy_drop_reap_only_original_and_prevent_reuse() {
    for rooted in [false, true] {
        let mut root = Root::new().unwrap();
        let mut owner = Owner::new(&mut root).unwrap();
        if rooted {
            owner.enter().unwrap();
        }
        // SAFETY: signal only this retained original pidfd, never process enumeration.
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
        assert!(if rooted { owner.probe() } else { owner.enter() }.is_err());
        assert!(owner.reaped && owner.root.refused);
    }
    let mut root = Root::new().unwrap();
    let duplicate = root.dirs[0].file.try_clone().unwrap();
    let owner = Owner::new(&mut root).unwrap();
    let pid = owner.pid;
    drop(owner);
    // SAFETY: diagnostic wait of exact already-reaped own child; no wait-any/adoption.
    let mut status = 0;
    assert_eq!(
        unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) },
        -1
    );
    assert_eq!(
        io::Error::last_os_error().raw_os_error(),
        Some(libc::ECHILD)
    );
    assert!(duplicate.metadata().unwrap().is_dir());
    root.check().unwrap();
    assert!(Owner::new(&mut root).is_err());
    assert!(root.refused);
    println!("owned_root_SIGKILL=initial,rooted original_child_reaped=true signer_exec=false tracer_loss_proved=false");
}

#[test]
fn signer_root_cleanup_uses_original_directory_fds_not_replaced_ancestors() {
    let mut root = Root::new().unwrap();
    let foreign = tempfile::tempdir().unwrap();
    let destination = foreign.path().join("x86_64-linux-gnu");
    fs::create_dir(&destination).unwrap();
    fs::set_permissions(&destination, fs::Permissions::from_mode(0o555)).unwrap();
    fs::set_permissions(&root.path, fs::Permissions::from_mode(0o755)).unwrap();
    fs::rename(root.path.join("lib"), root.path.join("lib-original")).unwrap();
    symlink(foreign.path(), root.path.join("lib")).unwrap();
    assert!(root.check().is_err());
    drop(root);
    assert_eq!(fs::metadata(&destination).unwrap().mode() & 0o7777, 0o555);
    fs::set_permissions(&destination, fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn signer_root_seeded_probes_and_independent_nonroot_owners() {
    let mut seed = 0x0740_3b29_u64;
    for _ in 0..8 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut root = Root::new().unwrap();
        let mut owner = Owner::new(&mut root).unwrap();
        owner.enter().unwrap();
        if seed & 1 != 0 {
            owner.rooted().unwrap();
        }
        owner.probe().unwrap();
    }
    let threads: Vec<_> = (0..4)
        .map(|_| {
            std::thread::spawn(|| {
                let mut root = Root::new().unwrap();
                let mut owner = Owner::new(&mut root).unwrap();
                owner.enter().unwrap();
                owner.probe().unwrap();
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    println!("seed=0x07403b29 rooted_path_probes=8 independent_owners=4 parent_root_changed=false signer_exec=false");
}
