use super::*;
use std::{
    os::unix::process::CommandExt,
    process::{Command, Stdio},
};
fn c(child: u32, deadline: u64) -> Challenge {
    Challenge {
        deadline,
        creator: [std::process::id(), unsafe { libc::getuid() }, unsafe {
            libc::getgid()
        }],
        child,
        nonce: [1; 32],
        entry: [2; 32],
        context: [3; 32],
        manager: [4; 32],
        client: [5; 32],
        invocation: [6; 16],
    }
}
#[test]
fn held_delivery_child_codec_binding_and_reserved_bytes() {
    let a = c(std::process::id() + 1, 10);
    let raw = a.encode().unwrap();
    assert_eq!(Challenge::decode(&raw).unwrap(), a);
    for at in (0..8).chain(208..256) {
        let mut bad = raw;
        bad[at] ^= 1;
        assert!(Challenge::decode(&bad).is_err());
    }
    for n in [0, 255, 257] {
        assert!(Challenge::decode(&vec![0; n]).is_err());
    }
    for at in 8..208 {
        let mut bad = raw;
        bad[at] ^= 1;
        if let Ok(b) = Challenge::decode(&bad) {
            assert_ne!(a.ack().unwrap(), b.ack().unwrap());
        }
    }
    let mut bad = a.clone();
    bad.child = bad.creator[0];
    assert!(bad.encode().is_err());
}
struct Guard(std::process::Child);
impl Drop for Guard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn spawn(pair: &Pair, mode: &str, deadline: u64) -> Guard {
    let raw = pair.child.fd.as_raw_fd();
    let mut cmd = Command::new(std::env::current_exe().unwrap());
    let module = module_path!().split_once("::").unwrap().1;
    cmd.args([
        "--exact",
        &format!("{module}::held_delivery_child_helper"),
        "--nocapture",
        "--test-threads=1",
    ])
    .env("HC074_EXCHANGE_HELPER", mode)
    .env("HC074_PARENT", std::process::id().to_string())
    .env("HC074_DEADLINE", deadline.to_string())
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(Stdio::inherit());
    unsafe {
        cmd.pre_exec(move || {
            if libc::dup2(raw, 3) < 0 || libc::fcntl(3, libc::F_SETFD, 0) < 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    Guard(cmd.spawn().unwrap())
}
#[test]
fn held_delivery_child_helper() {
    let Some(mode) = std::env::var_os("HC074_EXCHANGE_HELPER") else {
        return;
    };
    let parent = std::env::var("HC074_PARENT")
        .unwrap()
        .parse::<u32>()
        .unwrap();
    let deadline = std::env::var("HC074_DEADLINE")
        .unwrap()
        .parse::<u64>()
        .unwrap();
    let mut expected = c(std::process::id(), deadline);
    expected.creator[0] = parent;
    assert_eq!(
        unsafe { libc::fcntl(3, libc::F_SETFD, libc::FD_CLOEXEC) },
        0
    );
    let fd = unsafe { OwnedFd::from_raw_fd(3) };
    let mut child = Child::from_owned(fd).unwrap();
    // Bounded readiness wait is fixture scheduling only, not an admission retry.
    let mut p = libc::pollfd {
        fd: child.endpoint.fd.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    assert_eq!(unsafe { libc::poll(&mut p, 1, 2000) }, 1);
    if mode == "bad" {
        expected.context[0] ^= 1;
        assert!(child.answer_once(&expected).is_err());
        assert!(child.answer_once(&expected).is_err());
        return;
    }
    if mode == "drop" {
        let _ = child.endpoint.receive(expected.creator).unwrap();
        return;
    }
    child.answer_once(&expected).unwrap();
    assert!(child.answer_once(&expected).is_err());
    // Remain live until the parent has checked its atomically delivered pidfd.
    std::thread::sleep(std::time::Duration::from_millis(300));
}
#[test]
fn held_delivery_child_real_credentials_pidfd_one_shot_and_lost_ack() {
    for mode in ["ok", "bad", "drop"] {
        let mut pair = Pair::create().unwrap();
        let deadline = clock().unwrap() + 5_000_000_000;
        let mut child = spawn(&pair, mode, deadline);
        let challenge = c(child.0.id(), deadline);
        pair.begin(challenge).unwrap();
        let peer = [child.0.id(), unsafe { libc::getuid() }, unsafe {
            libc::getgid()
        }];
        let mut ready = false;
        for _ in 0..300 {
            match pair.try_ack(peer) {
                Ok(true) => {
                    ready = true;
                    break;
                }
                Ok(false) => {}
                Err(_) => break,
            }
            if child.0.try_wait().unwrap().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(ready, mode == "ok");
        pair.refuse();
        assert!(pair.begin(c(child.0.id(), deadline)).is_err());
        // Dropped/bad child outcomes never become a successful readiness proof.
    }
}
#[test]
fn held_delivery_child_wrong_kernel_peer_and_expired_bound_refuse() {
    let mut pair = Pair::create().unwrap();
    let deadline = clock().unwrap() + 5_000_000_000;
    let child = spawn(&pair, "ok", deadline);
    pair.begin(c(child.0.id(), deadline)).unwrap();
    // Same endpoint and packet, but wrong signed UID expectation.
    let mut p = libc::pollfd {
        fd: pair.parent.fd.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    assert_eq!(unsafe { libc::poll(&mut p, 1, 2000) }, 1);
    assert!(pair
        .try_ack([child.0.id(), unsafe { libc::getuid() } ^ 1, unsafe {
            libc::getgid()
        }])
        .is_err());
    assert_eq!(pair.phase, Phase::Refused);
    let mut expired = Pair::create().unwrap();
    let challenge = c(std::process::id() + 1, 1);
    assert!(expired.begin(challenge.clone()).is_err());
    assert!(expired.begin(challenge).is_err());
}
#[test]
fn held_delivery_child_send_error_never_reopens_window() {
    let mut pair = Pair::create().unwrap();
    let deadline = clock().unwrap() + 1_000_000_000;
    assert_eq!(
        unsafe { libc::shutdown(pair.child.fd.as_raw_fd(), libc::SHUT_RDWR) },
        0
    );
    let challenge = c(std::process::id() + 1, deadline);
    assert!(pair.begin(challenge.clone()).is_err());
    assert!(pair.begin(challenge).is_err());
    assert_eq!(pair.phase, Phase::Refused);
}

#[test]
fn held_entry_failed_full_frame_latches_before_receive_and_never_reenters() {
    let pair = Pair::create().unwrap();
    let mut child = Child::from_owned(pair.child.fd.try_clone().unwrap()).unwrap();
    let mut expected = c(std::process::id(), clock().unwrap() + 5_000_000_000);
    expected.nonce = [0; 32];
    let mut calls = 0;
    assert!(child
        .answer_framed_once(&expected, || {
            calls += 1;
            Err(denied())
        })
        .is_err());
    assert_eq!(calls, 1);
    assert_eq!(child.phase, Phase::Refused);
    assert!(child
        .answer_framed_once(&expected, || {
            calls += 1;
            Ok(())
        })
        .is_err());
    assert_eq!(calls, 1);
    assert!(pair.parent.receive(expected.creator).unwrap().is_none());
}
