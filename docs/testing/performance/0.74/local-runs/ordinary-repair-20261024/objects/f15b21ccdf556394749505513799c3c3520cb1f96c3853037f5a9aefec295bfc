use super::*;
use std::os::fd::AsFd;

#[test]
fn signer_first_exec_inherited_foreign_writer_closed_before_initial_stop() {
    let first = Fixture::new();
    let second = Fixture::new();
    fs::set_permissions(&second.binary, fs::Permissions::from_mode(0o755)).unwrap();
    let foreign_writer = OpenOptions::new().write(true).open(&second.binary).unwrap();
    fs::set_permissions(&second.binary, fs::Permissions::from_mode(0o555)).unwrap();
    let mut first_owner = OwnedExec::stopped(&first).unwrap();
    // The parent releases the writable description, but without close_range
    // the stopped first child still holds its inherited CLOEXEC duplicate.
    drop(foreign_writer);
    let mut second_owner = OwnedExec::launch(&second).unwrap();
    second_owner.resume().unwrap();
    second_owner.wait_marker(b"ready\n").unwrap();
    second_owner.finish().unwrap();
    assert_eq!(first_owner.phase, Phase::Initial);
    first_owner.first_exec().unwrap();
    first_owner.resume().unwrap();
    first_owner.wait_marker(b"ready\n").unwrap();
    first_owner.finish().unwrap();
    println!("inherited_foreign_writer=closed_before_initial_stop CLOEXEC_alone=insufficient");
}

#[test]
fn signer_first_exec_original_stop_precedes_helper_output() {
    let fixture = Fixture::new();
    let mut owner = OwnedExec::launch(&fixture).unwrap();
    assert_eq!(owner.phase, Phase::FirstExecStopped);
    assert_eq!(owner.output.metadata().unwrap().len(), 0);
    owner.resume().unwrap();
    owner.wait_marker(b"ready\n").unwrap();
    owner.command(Control::Ack).unwrap();
    owner.wait_marker(b"ack\n").unwrap();
    owner.finish().unwrap();
    assert_eq!(owner.phase, Phase::Terminal);
    assert!(owner.reaped);
}

#[test]
fn signer_first_exec_wait_status_exhaustive_exact_event_only() {
    for event in [
        0,
        libc::PTRACE_EVENT_EXEC,
        libc::PTRACE_EVENT_FORK,
        libc::PTRACE_EVENT_CLONE,
        128,
    ] {
        for low in 0..=u16::MAX {
            let status = (event << 16) | i32::from(low);
            assert_eq!(
                is_exec_stop(status),
                event == libc::PTRACE_EVENT_EXEC && low == ((libc::SIGTRAP << 8) | 0x7f) as u16
            );
        }
    }
    for status in [-1, i32::MAX, EXEC_STOP | (1 << 24), libc::SIGKILL] {
        assert!(!is_exec_stop(status));
    }
    println!("exec-stop exact_event=true lower_status_cases=327680 unknown_high_bits=refused");
}

#[test]
fn signer_first_exec_header_descriptor_and_metadata_refusals() {
    let f = Fixture::new();
    let valid = File::open(&f.binary).unwrap();
    assert!(selected(&valid).is_ok());
    fs::set_permissions(&f.binary, fs::Permissions::from_mode(0o755)).unwrap();
    let writable = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&f.binary)
        .unwrap();
    fs::set_permissions(&f.binary, fs::Permissions::from_mode(0o555)).unwrap();
    assert!(selected(&writable).is_err());
    let opath = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_PATH)
        .open(&f.binary)
        .unwrap();
    assert!(selected(&opath).is_err());
    assert!(selected(&File::open(f._temp.path()).unwrap()).is_err());
    let empty = f._temp.path().join("empty");
    File::create(&empty).unwrap();
    assert!(selected(&File::open(&empty).unwrap()).is_err());
    let link = f._temp.path().join("link");
    fs::hard_link(&f.binary, &link).unwrap();
    assert!(selected(&valid).is_err());
    fs::remove_file(&link).unwrap();
    fs::set_permissions(&f.binary, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(selected(&valid).is_err());
    fs::set_permissions(&f.binary, fs::Permissions::from_mode(0o555)).unwrap();
    assert!(selected(&valid).is_ok());
    let mut bytes = fs::read(&f.binary).unwrap();
    for offset in [0, 4, 5, 6, 16, 18, 20, 52] {
        let old = bytes[offset];
        bytes[offset] = 0xff;
        fs::set_permissions(&f.binary, fs::Permissions::from_mode(0o755)).unwrap();
        fs::write(&f.binary, &bytes).unwrap();
        fs::set_permissions(&f.binary, fs::Permissions::from_mode(0o555)).unwrap();
        assert!(selected(&valid).is_err(), "header byte {offset}");
        bytes[offset] = old;
    }
}

#[test]
fn signer_first_exec_foreign_equal_byte_inode_cannot_match_original() {
    let f = Fixture::new();
    let foreign = f._temp.path().join("foreign");
    fs::copy(&f.binary, &foreign).unwrap();
    fs::set_permissions(&foreign, fs::Permissions::from_mode(0o555)).unwrap();
    let mut owner = OwnedExec::launch(&f).unwrap();
    assert_ne!(owner.stamp.ino, fs::metadata(&foreign).unwrap().ino());
    assert!(owner.check_object(&File::open(&foreign).unwrap()).is_err());
    owner.refuse().unwrap();
    assert_eq!(owner.phase, Phase::Refused);
    assert!(owner.resume().is_err());
    assert_eq!(owner.output.metadata().unwrap().len(), 0);
}

fn second_exec(control: Control) {
    let f = Fixture::new();
    let mut owner = OwnedExec::launch(&f).unwrap();
    owner.resume().unwrap();
    owner.wait_marker(b"ready\n").unwrap();
    owner.command(control).unwrap();
    owner.reject_next_exec().unwrap();
    assert_eq!(owner.phase, Phase::Refused);
    assert!(owner.reaped);
    let status = owner.last_status.unwrap();
    assert!(libc::WIFSIGNALED(status));
    assert_eq!(libc::WTERMSIG(status), libc::SIGKILL);
    assert!(owner.resume().is_err());
    assert!(owner.command(Control::Ack).is_err());
    let mut output = [0; 128];
    let n = owner.output.read_at(&mut output, 0).unwrap();
    assert_eq!(&output[..n], b"ready\n");
}
#[test]
fn signer_first_exec_same_inode_second_exec_is_stopped_before_new_user_code() {
    second_exec(Control::Same);
    println!("same_inode_second_exec=kernel_event_refused new_user_code_ran=false seccomp_installed=false");
}
#[test]
fn signer_first_exec_different_inode_second_exec_is_stopped_before_new_user_code() {
    second_exec(Control::Other);
}

#[test]
fn signer_first_exec_unexpected_signal_and_output_overflow_cross_refuse() {
    for overflow in [false, true] {
        let f = Fixture::new();
        let mut owner = OwnedExec::launch(&f).unwrap();
        owner.resume().unwrap();
        owner.wait_marker(b"ready\n").unwrap();
        if overflow {
            OpenOptions::new()
                .append(true)
                .open(&f.output)
                .unwrap()
                .write_all(&[b'x'; 129])
                .unwrap();
            assert!(owner.wait_marker(b"ready\n").is_err());
        } else {
            // SAFETY: original owned helper pidfd only; injected unexpected stop.
            let result = unsafe {
                libc::syscall(
                    libc::SYS_pidfd_send_signal,
                    owner.pidfd.as_ref().unwrap().as_raw_fd(),
                    libc::SIGSTOP,
                    std::ptr::null::<libc::siginfo_t>(),
                    0u32,
                )
            };
            assert_eq!(result, 0);
            assert!(owner.reject_next_exec().is_err());
        }
        assert_eq!(owner.phase, Phase::Refused);
        assert!(owner.reaped);
        assert!(owner.resume().is_err());
    }
}

#[test]
fn signer_first_exec_kernel_rejects_header_only_shape_without_admission() {
    let f = Fixture::new();
    let bytes = fs::read(&f.binary).unwrap();
    fs::set_permissions(&f.binary, fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(&f.binary, &bytes[..64]).unwrap();
    fs::set_permissions(&f.binary, fs::Permissions::from_mode(0o555)).unwrap();
    assert!(selected(&File::open(&f.binary).unwrap()).is_ok());
    assert!(OwnedExec::launch(&f).is_err());
    assert_eq!(fs::metadata(&f.output).unwrap().len(), 0);
}

fn terminal(fd: &OwnedFd) -> bool {
    let mut item = libc::pollfd {
        fd: fd.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: valid single pollfd, zero timeout, no ownership transfer.
    unsafe { libc::poll(&mut item, 1, 0) == 1 && item.revents & libc::POLLIN != 0 }
}
#[test]
fn signer_first_exec_cloexec_and_drop_clean_exact_owned_child() {
    for running in [false, true] {
        let f = Fixture::new();
        let mut owner = OwnedExec::launch(&f).unwrap();
        let expected_fd = owner.expected.as_raw_fd();
        let expected_id = (owner.stamp.dev, owner.stamp.ino);
        let pidfd = owner
            .pidfd
            .as_ref()
            .unwrap()
            .as_fd()
            .try_clone_to_owned()
            .unwrap();
        for fd in [
            owner.pidfd.as_ref().unwrap().as_raw_fd(),
            owner.expected.as_raw_fd(),
            owner.output.as_raw_fd(),
            owner.input.as_ref().unwrap().as_raw_fd(),
        ] {
            // SAFETY: valid borrowed original fixture descriptor, query only.
            let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
            assert!(flags >= 0 && flags & libc::FD_CLOEXEC != 0);
        }
        assert!(!terminal(&pidfd));
        if running {
            owner.resume().unwrap();
            owner.wait_marker(b"ready\n").unwrap();
        }
        drop(owner);
        assert!(terminal(&pidfd));
        // Other parallel tests may reuse the integer, but not this unique inode.
        let mut metadata = std::mem::MaybeUninit::<libc::stat>::uninit();
        // SAFETY: valid output storage; inspect only on successful fstat.
        if unsafe { libc::fstat(expected_fd, metadata.as_mut_ptr()) } == 0 {
            // SAFETY: fstat succeeded and initialized the complete stat value.
            let metadata = unsafe { metadata.assume_init() };
            assert_ne!((metadata.st_dev, metadata.st_ino), expected_id);
        }
    }
}

#[test]
fn signer_first_exec_seeded_originals_and_independent_tracer_threads() {
    let mut seed = 0x0740_3b25_u64;
    for _ in 0..16 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        if seed & 1 == 0 {
            second_exec(Control::Same);
        } else {
            let f = Fixture::new();
            let mut owner = OwnedExec::launch(&f).unwrap();
            owner.resume().unwrap();
            owner.wait_marker(b"ready\n").unwrap();
            owner.finish().unwrap();
        }
    }
    let threads: Vec<_> = (0..4)
        .map(|_| {
            std::thread::spawn(|| {
                let f = Fixture::new();
                let mut owner = OwnedExec::launch(&f).unwrap();
                owner.resume().unwrap();
                owner.wait_marker(b"ready\n").unwrap();
                owner.finish().unwrap();
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    println!("seed=0x07403b25 fresh_original_launches=16 independent_tracer_threads=4 signer_or_runtime_authority=false");
}
