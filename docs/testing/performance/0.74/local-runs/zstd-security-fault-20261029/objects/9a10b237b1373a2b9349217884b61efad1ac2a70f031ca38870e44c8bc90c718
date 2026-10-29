use super::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;

#[path = "diagnostic_signer_owned_probe_loss_tests.rs"]
mod loss;

fn fixture() -> (tempfile::TempDir, File) {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("owned-true");
    fs::copy("/bin/true", &path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o555)).unwrap();
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC)
        .open(path)
        .unwrap();
    (temp, file)
}
fn dead(fd: &OwnedFd) -> bool {
    let mut poll = libc::pollfd {
        fd: fd.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: read-only liveness of one retained original pidfd, no numeric PID adoption.
    unsafe { libc::poll(&mut poll, 1, 0) == 1 && poll.revents & libc::POLLIN != 0 }
}
fn witness(owner: &Owner) -> OwnedFd {
    let raw = unsafe {
        libc::fcntl(
            owner.pidfd.as_ref().unwrap().as_raw_fd(),
            libc::F_DUPFD_CLOEXEC,
            3,
        )
    };
    assert!(raw >= 3);
    unsafe { OwnedFd::from_raw_fd(raw) }
}

#[test]
fn owned_probe_parent_status_is_bounded_single_thread_observation() {
    assert!(supports_reaping(libc::SIG_DFL, 0));
    assert!(!supports_reaping(libc::SIG_IGN, 0));
    assert!(!supports_reaping(libc::SIG_DFL, libc::SA_NOCLDWAIT));
    assert!(single_thread_status(b"Name:\towned\nThreads:\t1\n"));
    for raw in [
        b"".to_vec(),
        b"Name:\towned\n".to_vec(),
        b"Threads:\t0\n".to_vec(),
        b"Threads:\t2\n".to_vec(),
        b"Threads:\t+1\n".to_vec(),
        b"Threads:\t1\nThreads:\t1\n".to_vec(),
        vec![0xff; 32],
        vec![b'x'; 65537],
    ] {
        assert!(!single_thread_status(&raw));
    }
}

#[test]
fn owned_probe_wait_status_exhaustive_exact_events_and_seeded_high_bits() {
    for high in [0u32, 1, 4, 0x8000, 0xffff] {
        for low in 0..=65535 {
            let raw = (high << 16) | low;
            let want = match raw {
                0x137f => Event::Initial,
                0x4057f => Event::Exec,
                0 => Event::ExitZero,
                31 => Event::SysDeath,
                0x1f7f => Event::SysDelivery,
                _ => Event::Other,
            };
            assert_eq!(event(raw as i32), want);
        }
    }
    let mut seed = 0x0740c07au32;
    for _ in 0..256 {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        let raw = seed | 0x80000000;
        assert_eq!(event(raw as i32), Event::Other);
    }
    println!("wait_status_cases=327680 seed=0x0740c07a mutations=256 authority=false");
}

#[test]
fn owned_probe_selected_descriptor_and_bounded_header_refusals() {
    let (temp, file) = fixture();
    let path = temp.path().join("owned-true");
    assert!(selected(&file).is_ok());
    assert_eq!(
        unsafe { libc::fcntl(file.as_raw_fd(), libc::F_SETFD, 0) },
        0
    );
    assert!(selected(&file).is_err());
    assert_eq!(
        unsafe { libc::fcntl(file.as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC) },
        0
    );
    let opath = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_PATH | libc::O_CLOEXEC)
        .open(&path)
        .unwrap();
    assert!(selected(&opath).is_err());
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(selected(&file).is_err());
    fs::set_permissions(&path, fs::Permissions::from_mode(0o555)).unwrap();
    fs::hard_link(&path, temp.path().join("alias")).unwrap();
    assert!(selected(&file).is_err());
    let template = fs::read("/bin/true").unwrap();
    for (case, index) in [0, 4, 5, 6, 16, 18, 20, 52].into_iter().enumerate() {
        let mut header = template[..64].to_vec();
        header[index] = 255;
        let bad = temp.path().join(format!("bad-{case}"));
        fs::write(&bad, header).unwrap();
        fs::set_permissions(&bad, fs::Permissions::from_mode(0o555)).unwrap();
        assert!(selected(&File::open(bad).unwrap()).is_err());
    }
    let short = temp.path().join("short");
    fs::write(&short, [0u8; 63]).unwrap();
    fs::set_permissions(&short, fs::Permissions::from_mode(0o555)).unwrap();
    assert!(selected(&File::open(short).unwrap()).is_err());
    let huge_path = temp.path().join("huge");
    let huge = File::create(&huge_path).unwrap();
    huge.set_len(134217729).unwrap();
    huge.set_permissions(fs::Permissions::from_mode(0o555))
        .unwrap();
    drop(huge);
    assert!(selected(&File::open(huge_path).unwrap()).is_err());
}

#[test]
fn owned_probe_output_is_exact_bounded_data_not_an_installation_receipt() {
    assert!(correct_output(CHILD_LINE, Case::Native));
    assert!(correct_output(b"", Case::X32));
    assert!(correct_output(b"", Case::I386));
    for raw in [
        b"".to_vec(),
        CHILD_LINE[..CHILD_LINE.len() - 1].to_vec(),
        [CHILD_LINE, b"extra"].concat(),
        vec![b'x'; 128],
        vec![b'x'; 129],
    ] {
        assert!(!correct_output(&raw, Case::Native));
    }
    for case in [Case::X32, Case::I386] {
        assert!(!correct_output(CHILD_LINE, case));
        assert!(!correct_output(b"x", case));
    }
}

#[test]
fn owned_probe_kernel_exec_rejects_header_only_not_a_runtime_admission() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("header-only");
    let raw = fs::read("/bin/true").unwrap();
    fs::write(&path, &raw[..64]).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o555)).unwrap();
    let mut own = Owner::create(File::open(path).unwrap(), Case::Native).unwrap();
    assert!(own.first().is_err());
    assert!(own.reaped);
    assert_eq!(own.phase, Phase::Refused);
}

#[test]
fn owned_probe_first_object_and_equal_byte_foreign_file_are_not_interchangeable() {
    let (_root, file) = fixture();
    let mut own = Owner::create(file, Case::Native).unwrap();
    own.first().unwrap();
    own.object().unwrap();
    let (_foreign, file) = fixture();
    own.expected = file;
    assert!(own.resume().is_err());
    assert_eq!(own.phase, Phase::Refused);
    assert!(own.reaped);
    assert!(own.first().is_err());
    assert!(own.resume().is_err());
    assert!(own.finish().is_err());
}

#[test]
fn owned_probe_phase_replay_and_wrong_terminal_output_refuse_stickily() {
    let (_root, file) = fixture();
    let mut own = Owner::create(file, Case::Native).unwrap();
    assert!(own.resume().is_err());
    assert_eq!(own.phase, Phase::Refused);
    assert!(own.reaped);
    assert!(own.first().is_err());
    let (_root, file) = fixture();
    let mut own = Owner::create(file, Case::Native).unwrap();
    own.first().unwrap();
    assert!(own.first().is_err());
    assert_eq!(own.phase, Phase::Refused);
    assert!(own.reaped);
    let (_root, file) = fixture();
    let mut own = Owner::create(file, Case::Native).unwrap();
    own.first().unwrap();
    own.resume().unwrap();
    // /bin/true exits zero but cannot supply the fixed child restriction line.
    assert!(own.finish().is_err());
    assert_eq!(own.phase, Phase::Refused);
    assert!(own.reaped);
    assert!(own.resume().is_err());
}

#[test]
fn owned_probe_sigkill_and_drop_at_both_original_stops_reap_only_owned_child() {
    for first in [false, true] {
        let (_root, file) = fixture();
        let mut own = Owner::create(file, Case::Native).unwrap();
        if first {
            own.first().unwrap();
        }
        let held = witness(&own);
        assert_eq!(
            unsafe {
                libc::syscall(
                    libc::SYS_pidfd_send_signal,
                    held.as_raw_fd(),
                    libc::SIGKILL,
                    std::ptr::null::<libc::siginfo_t>(),
                    0u32,
                )
            },
            0
        );
        assert_eq!(own.wait().unwrap(), libc::SIGKILL);
        assert!(own.reaped);
        assert!(own.first().is_err());
        assert_eq!(own.phase, Phase::Refused);
        assert!(own.object().is_err());
        assert!(dead(&held));
        let (_root, file) = fixture();
        let mut own = Owner::create(file, Case::Native).unwrap();
        if first {
            own.first().unwrap();
        }
        let held = witness(&own);
        drop(own);
        assert!(dead(&held));
    }
    println!("original_child_SIGKILL_and_drop=initial,first_exec exact_owned_pidfd=true parent_loss_proved=false");
}

#[test]
fn owned_probe_pre_acquisition_cleanup_model_and_four_independent_tracers() {
    let (_root, file) = fixture();
    let mut own = Owner::create(file, Case::Native).unwrap();
    let held = witness(&own);
    // Model the field state before pidfd_open, never supply/change a numeric PID.
    own.pidfd = None;
    drop(own);
    assert!(dead(&held));
    let readers: Vec<_> = (0..4)
        .map(|_| {
            std::thread::spawn(|| {
                let (_root, file) = fixture();
                let mut own = Owner::create(file, Case::Native).unwrap();
                own.first().unwrap();
                let held = witness(&own);
                drop(own);
                assert!(dead(&held));
            })
        })
        .collect();
    for reader in readers {
        reader.join().unwrap();
    }
}
