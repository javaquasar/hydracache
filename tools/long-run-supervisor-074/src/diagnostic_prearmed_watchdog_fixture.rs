//! Private libtest subprocess, never a production owner or launch interface.
use super::*;
use std::fs::{self, OpenOptions};
use std::io::Read;
use std::os::unix::{fs::OpenOptionsExt, net::UnixDatagram};
use std::path::PathBuf;

struct Target {
    pin: File,
    reader: File,
    release: super::super::super::ReleaseAttempt,
    hidden: Option<File>,
}
fn target(guardian: &Watchdog, hidden: bool) -> Target {
    let [release, receive] = pair().unwrap();
    // Barrier v0 has no ancillary data. Disable auto-credentials on its receiver.
    let off = 0i32;
    assert_eq!(
        unsafe {
            libc::setsockopt(
                receive.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_PASSCRED,
                (&off as *const i32).cast(),
                4,
            )
        },
        0
    );
    let mut pipe = [-1; 2];
    assert_eq!(
        unsafe { libc::pipe2(pipe.as_mut_ptr(), libc::O_NONBLOCK | libc::O_CLOEXEC) },
        0
    );
    let [reader, writer] = unsafe { [File::from_raw_fd(pipe[0]), File::from_raw_fd(pipe[1])] };
    let hidden = hidden.then(|| writer.try_clone().unwrap());
    let owner = unsafe { libc::syscall(libc::SYS_pidfd_open, libc::getpid(), 0u32) };
    assert!(owner >= 0);
    let owner = unsafe { File::from_raw_fd(owner as RawFd) };
    let raw = [
        receive.as_raw_fd(),
        owner.as_raw_fd(),
        guardian.pinned.as_raw_fd(),
        guardian.alarm.as_raw_fd(),
        writer.as_raw_fd(),
    ];
    let mut keep = raw;
    keep.sort_unstable();
    let mut pinned = -1i32;
    let mut args = [0u64; 11];
    args[0] = libc::CLONE_PIDFD as u64;
    args[1] = (&mut pinned as *mut RawFd) as u64;
    args[4] = libc::SIGCHLD as u64;
    let blocked = u64::MAX;
    let mut previous = 0u64;
    assert_eq!(
        unsafe {
            libc::syscall(
                libc::SYS_rt_sigprocmask,
                libc::SIG_SETMASK,
                &blocked,
                &mut previous,
                8usize,
            )
        },
        0
    );
    let pid = unsafe {
        libc::syscall(
            libc::SYS_clone3,
            args.as_ptr(),
            std::mem::size_of_val(&args),
        )
    };
    if pid == 0 {
        if !isolate(&keep) {
            unsafe { libc::_exit(80) };
        }
        let code = super::super::super::wait_packet(raw[0], raw[1], raw[2], raw[3], &[74; 32]);
        if code != 0 {
            unsafe { libc::_exit(code) };
        }
        if unsafe { libc::write(raw[4], b"E".as_ptr().cast(), 1) } != 1 {
            unsafe { libc::_exit(81) };
        }
        // Deliberately no owner/clock observation after release. The REAL
        // independent guardian, not this target, must stop it on owner loss.
        park();
    }
    assert_eq!(
        unsafe {
            libc::syscall(
                libc::SYS_rt_sigprocmask,
                libc::SIG_SETMASK,
                &previous,
                0usize,
                8usize,
            )
        },
        0
    );
    assert!(pid > 0);
    drop(writer);
    drop(receive);
    drop(owner);
    Target {
        pin: unsafe { File::from_raw_fd(pinned) },
        reader,
        release: super::super::super::ReleaseAttempt {
            socket: release,
            token: [74; 32],
            consumed: false,
        },
        hidden,
    }
}
fn publish(
    socket: &UnixDatagram,
    guardian: &Watchdog,
    target: Option<&Target>,
    case: &str,
    deadline: u64,
) {
    let mut rights = vec![guardian.pinned.as_raw_fd()];
    if let Some(t) = target {
        rights.extend([t.pin.as_raw_fd(), t.reader.as_raw_fd()]);
        if let Some(h) = &t.hidden {
            rights.push(h.as_raw_fd());
        }
    }
    let raw = serde_json::to_vec(&serde_json::json!({"schema":"hc074-prearmed-owner-test-v1", "case":case,
        "deadline":deadline, "owner":std::process::id(), "pending":guardian.pending, "armed":guardian.armed})).unwrap();
    let mut control = [0usize; 8];
    let mut iov = libc::iovec {
        iov_base: raw.as_ptr().cast_mut().cast(),
        iov_len: raw.len(),
    };
    let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
    msg.msg_iov = &mut iov;
    msg.msg_iovlen = 1;
    msg.msg_control = control.as_mut_ptr().cast();
    msg.msg_controllen =
        unsafe { libc::CMSG_SPACE(std::mem::size_of_val(rights.as_slice()) as _) } as _;
    unsafe {
        let c = libc::CMSG_FIRSTHDR(&msg);
        (*c).cmsg_level = libc::SOL_SOCKET;
        (*c).cmsg_type = libc::SCM_RIGHTS;
        (*c).cmsg_len = libc::CMSG_LEN(std::mem::size_of_val(rights.as_slice()) as _) as _;
        std::ptr::copy_nonoverlapping(
            rights.as_ptr(),
            libc::CMSG_DATA(c).cast::<RawFd>(),
            rights.len(),
        );
        assert_eq!(
            libc::sendmsg(socket.as_raw_fd(), &msg, libc::MSG_NOSIGNAL),
            raw.len() as isize
        );
    }
}
fn command() {
    let mut b = [0u8];
    std::io::stdin().read_exact(&mut b).unwrap();
    assert_eq!(b, *b"G");
}

#[test]
#[ignore = "private nonroot subprocess for prearmed driver; no original authority"]
fn prearmed_owner_victim() {
    let root =
        PathBuf::from(std::env::var_os("HC074_PREARMED_ROOT").expect("private driver required"));
    assert_eq!(root.parent(), Some(std::path::Path::new("/tmp")));
    assert!(root
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("hc074-prearmed-"));
    assert_eq!(fs::canonicalize(&root).unwrap(), root);
    let meta = fs::symlink_metadata(&root).unwrap();
    assert!(meta.is_dir() && !meta.file_type().is_symlink());
    assert_eq!(meta.mode() & 0o7777, 0o700);
    assert_eq!(meta.uid(), unsafe { libc::getuid() });
    assert_ne!(meta.uid(), 0);
    let case = std::env::var("HC074_PREARMED_CASE").unwrap();
    assert!([
        "pending-owner-loss",
        "armed-owner-loss",
        "peer-loss",
        "expiry",
        "hidden-writer",
        "lost-ack",
        "foreign-sender",
        "wrong-right",
        "duplicate-wire",
        "guardian-death",
        "guardian-death-after-release",
        "guardian-stopped-after-release"
    ]
    .contains(&case.as_str()));
    let fence = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(root.join("fence"))
        .unwrap();
    let deadline = super::super::super::now().unwrap() + 10_000_000_000;
    let mut guardian = Watchdog::spawn(fence, deadline, case == "lost-ack").unwrap();
    guardian.pending().unwrap();
    let socket = UnixDatagram::unbound().unwrap();
    socket.connect(root.join("notice.sock")).unwrap();
    if case == "pending-owner-loss" {
        publish(&socket, &guardian, None, &case, deadline);
        command();
        panic!("pending owner must be killed");
    }
    let mut target = target(&guardian, case == "hidden-writer");
    let early = ["lost-ack", "foreign-sender", "wrong-right"].contains(&case.as_str());
    if !early {
        guardian.enroll_once(&target.pin).unwrap();
    }
    if !early && case != "guardian-death" {
        target.release.send_once().unwrap();
    }
    publish(&socket, &guardian, Some(&target), &case, deadline);
    // Shed the creator's hidden alias after its exact copy reaches the driver.
    target.hidden = None;
    command();
    match case.as_str() {
        "lost-ack" => {
            let _ = guardian.enroll_once(&target.pin);
            panic!("lost ACK owner must be killed");
        }
        "foreign-sender" => {
            let blocked = u64::MAX;
            let mut previous = 0u64;
            assert_eq!(
                unsafe {
                    libc::syscall(
                        libc::SYS_rt_sigprocmask,
                        libc::SIG_SETMASK,
                        &blocked,
                        &mut previous,
                        8usize,
                    )
                },
                0
            );
            let pid = unsafe { libc::syscall(libc::SYS_fork) };
            if pid == 0 {
                let sent = send(
                    guardian.peer.as_raw_fd(),
                    ENROLL,
                    Some(target.pin.as_raw_fd()),
                );
                unsafe { libc::_exit(if sent { 0 } else { 1 }) };
            }
            assert_eq!(
                unsafe {
                    libc::syscall(
                        libc::SYS_rt_sigprocmask,
                        libc::SIG_SETMASK,
                        &previous,
                        0usize,
                        8usize,
                    )
                },
                0
            );
            assert!(pid > 0);
            let mut status = 0;
            assert_eq!(
                unsafe { libc::waitpid(pid as i32, &mut status, 0) },
                pid as i32
            );
            assert_eq!(status, 0);
            guardian.notice(REFUSED).unwrap();
        }
        "wrong-right" => {
            let file = File::open("/dev/null").unwrap();
            assert!(send(
                guardian.peer.as_raw_fd(),
                ENROLL,
                Some(file.as_raw_fd())
            ));
            guardian.notice(REFUSED).unwrap();
        }
        "peer-loss" => {
            assert_eq!(
                unsafe { libc::shutdown(guardian.peer.as_raw_fd(), libc::SHUT_RDWR) },
                0
            );
        }
        "duplicate-wire" => {
            assert!(send(
                guardian.peer.as_raw_fd(),
                ENROLL,
                Some(target.pin.as_raw_fd())
            ));
        }
        "guardian-death" => {
            let _ = target.release.send_once();
            assert!(target.release.consumed);
        }
        _ => {
            assert!(guardian.enroll_once(&target.pin).is_err());
            assert!(guardian.consumed);
        }
    }
    socket.send(b"D").unwrap();
    command();
    panic!("fixture must end by owned SIGKILL, not cleanup/replay");
}
