//! Pure formatting and own-process pidfd units, never an admitted owner fixture.
use super::super::alive_read;
use super::InfoPath;
use crate::diagnostic_ipc::validation_trace::kernel_walk::{self, Kind};
use crate::diagnostic_kernel_admission::{bounded, KernelError};
use std::cell::Cell;
use std::fs::{File, OpenOptions};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::fs::{FileExt, MetadataExt, OpenOptionsExt};

fn pin() -> OwnedFd {
    let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, std::process::id(), 0) };
    assert!(fd >= 0);
    // SAFETY: the successful call returns a fresh owned descriptor.
    unsafe { OwnedFd::from_raw_fd(fd as i32) }
}
fn reason<T>(value: Result<T, KernelError>) -> String {
    match value {
        Ok(_) => "ok".into(),
        Err(KernelError::Io(e)) => format!("io:{:?}:{:?}", e.kind(), e.raw_os_error()),
        Err(e) => e.to_string(),
    }
}

#[test]
fn pidfd_info_path_signed_boundaries_seed_and_owned_lifetime() {
    let check = |fd| {
        let path = InfoPath::new(fd);
        let expected = format!("/proc/thread-self/fdinfo/{fd}");
        assert_eq!(
            path.as_path().as_os_str().as_encoded_bytes(),
            expected.as_bytes()
        );
        assert!(path.length <= path.bytes.len());
        assert!(!path.as_path().as_os_str().as_encoded_bytes().contains(&0));
    };
    for fd in [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
        check(fd);
    }
    let mut power = 10_i32;
    loop {
        for fd in [power - 1, power, power + 1] {
            check(fd);
            check(-fd);
        }
        if power == 1_000_000_000 {
            break;
        }
        power *= 10;
    }
    let mut seed = 0x074f_d174_u64;
    for _ in 0..16384 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        check(seed as i32);
    }
    let retained = InfoPath::new(i32::MIN);
    let other = InfoPath::new(0);
    assert_ne!(retained.as_path(), other.as_path());
    assert_eq!(
        retained.as_path().to_str(),
        Some("/proc/thread-self/fdinfo/-2147483648")
    );
    println!("HC074-PIDFD-PATH-PROPERTY seed=0x074fd174 cases=16384 signed_edges=true owned=true profile=false server=false product=false");
}

#[test]
fn pidfd_info_path_fresh_open_identity_flags_and_missing_name_match_reference() {
    let pin = pin();
    for fd in [pin.as_raw_fd(), -1, i32::MIN, i32::MAX] {
        let open = |path: &std::path::Path| {
            OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
                .open(path)
        };
        let name = format!("/proc/thread-self/fdinfo/{fd}");
        let old = open(std::path::Path::new(&name));
        let new = open(InfoPath::new(fd).as_path());
        match (old, new) {
            (Ok(a), Ok(b)) => {
                let id = |file: &File| {
                    let m = file.metadata().unwrap();
                    (m.dev(), m.ino(), m.mode())
                };
                assert_eq!(id(&a), id(&b));
                for op in [libc::F_GETFD, libc::F_GETFL] {
                    assert_eq!(unsafe { libc::fcntl(a.as_raw_fd(), op) }, unsafe {
                        libc::fcntl(b.as_raw_fd(), op)
                    });
                }
                let mut left = [0; 4096];
                let mut right = [0; 4096];
                let n = a.read_at(&mut left, 0).unwrap();
                let m = b.read_at(&mut right, 0).unwrap();
                assert_eq!(&left[..n], &right[..m]);
            }
            (Err(a), Err(b)) => {
                assert_eq!((a.kind(), a.raw_os_error()), (b.kind(), b.raw_os_error()))
            }
            _ => panic!("pathname changed fresh open result"),
        }
    }
}

#[test]
fn pidfd_info_path_each_actual_read_keeps_alive_document_trace_and_original_pid() {
    let pin = pin();
    let calls = Cell::new(0);
    let pid = std::process::id() as i32;
    for _ in 0..2 {
        let (result, report) = kernel_walk::capture(|| {
            alive_read(&pin, pid, |file| {
                calls.set(calls.get() + 1);
                bounded(file)
            })
        });
        result.unwrap();
        assert_eq!(report.nodes[Kind::Alive as usize].calls, 1);
        assert_eq!(report.nodes[Kind::Document as usize].calls, 1);
        assert_eq!(report.nodes.iter().map(|n| n.calls).sum::<u64>(), 2);
        assert_eq!(report.roots[Kind::Alive as usize], 1);
        assert_eq!(
            report.edges[Kind::Alive as usize][Kind::Document as usize].calls,
            1
        );
    }
    assert_eq!(calls.get(), 2);
    assert!(matches!(
        alive_read(&pin, pid + 1, bounded),
        Err(KernelError::NotLive)
    ));
}

#[test]
fn pidfd_info_path_changed_document_and_pid_fields_refuse_without_reusing_success() {
    let pin = pin();
    let pid = std::process::id() as i32;
    let calls = Cell::new(0);
    alive_read(&pin, pid, bounded).unwrap();
    let good = format!("Pid:\t{pid}\n");
    for raw in [
        b"".as_slice(),
        b"Pid: -1\n",
        b"Pid: 0\n",
        b"Pid: 2147483648\n",
        b"Pid: x\n",
        b"\xff",
        good.repeat(2).as_bytes(),
    ] {
        assert!(alive_read(&pin, pid, |_| {
            calls.set(calls.get() + 1);
            Ok(raw)
        })
        .is_err());
    }
    assert_eq!(calls.get(), 7);
    assert!(matches!(
        alive_read(&pin, 0, |_| Ok(good.as_bytes())),
        Err(KernelError::NotLive)
    ));
    alive_read(&pin, pid, bounded).unwrap(); // No cached document; leaf alone is not a refusal owner.
}

#[test]
fn pidfd_info_path_read_errors_propagate_once_without_retry() {
    let pin = pin();
    let pid = std::process::id() as i32;
    for code in [libc::EINTR, libc::EIO, libc::EBADF] {
        let calls = Cell::new(0);
        let result = alive_read(&pin, pid, |_| {
            calls.set(calls.get() + 1);
            Err::<Vec<u8>, _>(KernelError::Io(std::io::Error::from_raw_os_error(code)))
        });
        assert_eq!(calls.get(), 1);
        assert_eq!(
            reason(result),
            format!(
                "io:{:?}:{:?}",
                std::io::Error::from_raw_os_error(code).kind(),
                Some(code)
            )
        );
    }
}

#[test]
fn pidfd_info_path_non_pidfd_poll_refuses_before_read() {
    let pin: OwnedFd = tempfile::tempfile().unwrap().into();
    let calls = Cell::new(0);
    let result = alive_read(&pin, std::process::id() as i32, |_| {
        calls.set(calls.get() + 1);
        Ok(b"Pid: 1\n")
    });
    assert!(matches!(result, Err(KernelError::NotLive)));
    assert_eq!(calls.get(), 0);
}
