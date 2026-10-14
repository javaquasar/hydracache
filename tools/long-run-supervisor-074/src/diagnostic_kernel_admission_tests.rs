use super::*;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

fn pair() -> (KernelReceiver, OwnedFd) {
    let mut fds = [-1; 2];
    assert_eq!(
        unsafe {
            libc::socketpair(
                libc::AF_UNIX,
                libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC,
                0,
                fds.as_mut_ptr(),
            )
        },
        0
    );
    let receiver = unsafe { OwnedFd::from_raw_fd(fds[0]) };
    let sender = unsafe { OwnedFd::from_raw_fd(fds[1]) };
    (KernelReceiver::new(receiver).unwrap(), sender)
}
fn send(fd: &OwnedFd, bytes: &[u8]) {
    assert_eq!(
        unsafe {
            libc::send(
                fd.as_raw_fd(),
                bytes.as_ptr().cast(),
                bytes.len(),
                libc::MSG_NOSIGNAL,
            )
        },
        bytes.len() as isize
    );
}
fn packet(c: &mut KernelReceiver) -> KernelPacket {
    c.try_receive().unwrap().unwrap()
}

#[test]
fn kernel_admission_actual_sender_and_nonblocking_empty_receive() {
    let (mut receiver, fd) = pair();
    assert!(receiver.try_receive().unwrap().is_none());
    send(&fd, b"one\0\xff");
    send(&fd, b"two");
    let mut first = packet(&mut receiver);
    assert_eq!(first.sender_pid(), std::process::id());
    assert_eq!(first.bytes(), b"one\0\xff");
    first.revalidate_sender().unwrap();
    assert_eq!(packet(&mut receiver).bytes(), b"two");
    assert!(receiver.try_receive().unwrap().is_none());
}
#[test]
fn kernel_admission_exact_budget_and_oversized_packet_refuse() {
    for length in [
        MAX_PACKET_BYTES,
        MAX_PACKET_BYTES + 1,
        MAX_PACKET_BYTES + 1024,
    ] {
        let (mut receiver, fd) = pair();
        send(&fd, &vec![42; length]);
        if length == MAX_PACKET_BYTES {
            assert_eq!(packet(&mut receiver).bytes().len(), MAX_PACKET_BYTES);
        } else {
            assert!(receiver.try_receive().is_err());
            assert!(matches!(receiver.try_receive(), Err(KernelError::Refused)));
        }
    }
}
#[test]
fn kernel_admission_empty_disconnect_and_socket_option_drift_refuse() {
    for case in 0..3 {
        let (mut receiver, fd) = pair();
        match case {
            0 => send(&fd, b""),
            1 => drop(fd),
            _ => {
                let zero = 0i32;
                assert_eq!(
                    unsafe {
                        libc::setsockopt(
                            receiver.fd.as_raw_fd(),
                            libc::SOL_SOCKET,
                            libc::SO_PASSCRED,
                            (&zero as *const i32).cast(),
                            std::mem::size_of::<i32>() as libc::socklen_t,
                        )
                    },
                    0
                );
            }
        }
        assert!(receiver.try_receive().is_err());
        assert!(matches!(receiver.try_receive(), Err(KernelError::Refused)));
    }
}
#[test]
fn kernel_admission_packet_projection_drift_latches_refusal() {
    let (mut receiver, fd) = pair();
    send(&fd, b"original");
    let mut p = packet(&mut receiver);
    p.sender.projection.groups.push(u32::MAX);
    assert!(p.revalidate_sender().is_err());
    assert!(matches!(p.revalidate_sender(), Err(KernelError::Refused)));
}

#[test]
fn kernel_worker_context_packet_sender_clock_and_prior_refusal_are_sticky() {
    for case in 0..3 {
        let (mut receiver, fd) = pair();
        send(&fd, b"context-only");
        let mut p = packet(&mut receiver);
        let ns = |name: &str| {
            let m = File::open(format!("/proc/thread-self/ns/{name}"))
                .unwrap()
                .metadata()
                .unwrap();
            crate::diagnostic_worker_policy::NamespaceIdentity {
                device: m.dev(),
                inode: m.ino(),
            }
        };
        let host = crate::diagnostic_worker_policy::AssertedWorkerHost {
            machine_id: "a".repeat(32),
            boot_id: p.received.boot_id().into(),
            user_namespace: ns("user"),
            mount_namespace: ns("mnt"),
        };
        p.require_worker_context(&host).unwrap();
        match case {
            0 => p.sender.projection.groups.push(u32::MAX),
            1 => p.received.realtime_ns = u64::MAX,
            _ => p.latch_refusal(),
        }
        assert!(p.require_worker_context(&host).is_err());
        assert!(p.is_refused());
        assert!(matches!(
            p.require_worker_context(&host),
            Err(KernelError::Refused)
        ));
    }
}
#[test]
fn kernel_admission_wrong_socket_type_refuses_without_fallback() {
    let mut fds = [-1; 2];
    assert_eq!(
        unsafe {
            libc::socketpair(
                libc::AF_UNIX,
                libc::SOCK_STREAM | libc::SOCK_CLOEXEC,
                0,
                fds.as_mut_ptr(),
            )
        },
        0
    );
    let receiver = unsafe { OwnedFd::from_raw_fd(fds[0]) };
    let _sender = unsafe { OwnedFd::from_raw_fd(fds[1]) };
    assert!(KernelReceiver::new(receiver).is_err());
}
#[test]
fn kernel_admission_received_rights_close_all_copies_on_refusal() {
    for copies in [1, 32, 253] {
        let (mut receiver, fd) = pair();
        let mut pipe = [-1; 2];
        assert_eq!(
            unsafe { libc::pipe2(pipe.as_mut_ptr(), libc::O_CLOEXEC | libc::O_NONBLOCK) },
            0
        );
        let mut reader = unsafe { File::from_raw_fd(pipe[0]) };
        let writer = unsafe { OwnedFd::from_raw_fd(pipe[1]) };
        #[repr(C, align(16))]
        struct Aligned([u8; 1088]);
        let mut control = Aligned([0; 1088]);
        let mut payload = [1u8];
        let mut iov = libc::iovec {
            iov_base: payload.as_mut_ptr().cast(),
            iov_len: 1,
        };
        let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
        msg.msg_iov = &mut iov;
        msg.msg_iovlen = 1;
        msg.msg_control = control.0.as_mut_ptr().cast();
        msg.msg_controllen = unsafe { libc::CMSG_SPACE(copies * 4) } as usize;
        let c = unsafe { libc::CMSG_FIRSTHDR(&msg) };
        unsafe {
            (*c).cmsg_level = libc::SOL_SOCKET;
            (*c).cmsg_type = libc::SCM_RIGHTS;
            (*c).cmsg_len = libc::CMSG_LEN(copies * 4) as usize;
            for i in 0..copies {
                std::ptr::write_unaligned(
                    libc::CMSG_DATA(c).add(i as usize * 4).cast::<i32>(),
                    writer.as_raw_fd(),
                );
            }
        }
        assert_eq!(
            unsafe { libc::sendmsg(fd.as_raw_fd(), &msg, libc::MSG_NOSIGNAL) },
            1
        );
        drop(writer);
        assert!(receiver.try_receive().is_err());
        assert_eq!(
            reader.read(&mut [0]).unwrap(),
            0,
            "every received pipe-writer copy must close"
        );
    }
}
fn signed() -> (Vec<u8>, DiagnosticPolicy) {
    use crate::diagnostic_ipc::{
        DiagnosticController, DiagnosticOperation, SignedDiagnosticRequest,
    };
    use crate::diagnostic_lease::DiagnosticIdentity;
    use ed25519_dalek::{Signer, SigningKey};
    let key = SigningKey::from_bytes(&[71; 32]); // Local fixture only; never enrollment.
    let now = Origin::open().unwrap().sample().unwrap();
    let request = DiagnosticRequest {
        schema_version: 1,
        request_id: "a".repeat(64),
        nonce_sha256: "b".repeat(64),
        operation: DiagnosticOperation::Reserve,
        expected_state_revision: 0,
        identity: DiagnosticIdentity {
            lease_id: "c".repeat(64),
            boot_id: now.boot_id().into(),
            binary_sha256: "d".repeat(64),
            build_provenance_sha256: "e".repeat(64),
        },
        source_commit: crate::diagnostic_lease::SOURCE_COMMIT.into(),
        preset: diagnostic_ipc::PRESET.into(),
        controller: DiagnosticController {
            repository_id: 17,
            actor_id: 18,
            run_id: 19,
            run_attempt: 1,
        },
        issued_at_unix_seconds: now.unix_seconds(),
        expires_at_unix_seconds: now.unix_seconds() + 60,
    };
    let signature = key.sign(&diagnostic_ipc::signing_message(&request).unwrap());
    let bytes = crate::canonical_json(&SignedDiagnosticRequest {
        request,
        signature_hex: crate::hex(&signature.to_bytes()),
    })
    .unwrap();
    let policy = DiagnosticPolicy {
        repository_id: 17,
        actor_ids: vec![18],
        client_uids: vec![unsafe { libc::getuid() }],
        client_gid: unsafe { libc::getgid() },
        verifying_key: key.verifying_key(),
    };
    (bytes, policy)
}
#[test]
fn kernel_admission_authentication_has_no_caller_peer_or_time() {
    let (bytes, policy) = signed();
    let (mut receiver, fd) = pair();
    send(&fd, &bytes);
    let mut p = packet(&mut receiver);
    assert_eq!(p.authenticate(&policy).unwrap().controller.actor_id, 18);
}
#[test]
fn kernel_admission_reviewed_controller_refuses_fixture_trust_and_fallback() {
    use crate::diagnostic_reviewed_trust as reviewed;
    for case in 0..3 {
        let (bytes, mut policy) = signed();
        let (mut receiver, fd) = pair();
        send(&fd, &bytes);
        let mut p = packet(&mut receiver);
        match case {
            0 => policy.repository_id = crate::diagnostic_builder::REPOSITORY_ID,
            1 => policy.verifying_key = reviewed::controller_key().unwrap(),
            _ => {
                policy.repository_id = crate::diagnostic_builder::REPOSITORY_ID;
                policy.verifying_key = reviewed::controller_key().unwrap();
            }
        }
        let result = p.authenticate_reviewed_controller(&policy);
        if case < 2 {
            assert!(matches!(result, Err(KernelError::Invalid)));
        } else {
            assert!(matches!(
                result,
                Err(KernelError::Authentication(
                    diagnostic_ipc::DiagnosticIpcError::Signature
                ))
            ));
        }
        assert!(matches!(
            p.authenticate_reviewed_controller(&policy),
            Err(KernelError::Refused)
        ));
        assert!(matches!(
            p.authenticate(&signed().1),
            Err(KernelError::Refused)
        ));
        assert!(matches!(p.revalidate_sender(), Err(KernelError::Refused)));
    }
}
#[test]
fn kernel_admission_signature_principal_and_clock_refusals_are_sticky() {
    for case in 0..7 {
        let (mut bytes, mut policy) = signed();
        match case {
            0 => policy.actor_ids = vec![0],
            1 => policy.client_uids = vec![u32::MAX],
            2 => bytes[0] = b'x',
            4 => {
                policy.verifying_key =
                    ed25519_dalek::SigningKey::from_bytes(&[72; 32]).verifying_key()
            }
            5 | 6 => {
                use ed25519_dalek::Signer;
                let mut signed: diagnostic_ipc::SignedDiagnosticRequest =
                    serde_json::from_slice(&bytes).unwrap();
                if case == 5 {
                    signed.request.issued_at_unix_seconds -= 120;
                    signed.request.expires_at_unix_seconds -= 120;
                } else {
                    signed.request.identity.boot_id = "12345678-1234-1234-1234-123456789abc".into();
                }
                let signature = ed25519_dalek::SigningKey::from_bytes(&[71; 32])
                    .sign(&diagnostic_ipc::signing_message(&signed.request).unwrap());
                signed.signature_hex = crate::hex(&signature.to_bytes());
                bytes = crate::canonical_json(&signed).unwrap();
            }
            _ => (),
        }
        let (mut receiver, fd) = pair();
        send(&fd, &bytes);
        let mut p = packet(&mut receiver);
        if case == 3 {
            p.received.realtime_ns = u64::MAX;
        }
        assert!(p.authenticate(&policy).is_err());
        assert!(matches!(p.authenticate(&policy), Err(KernelError::Refused)));
    }
}
#[test]
fn kernel_admission_seeded_status_mutations_refuse_wrong_credentials() {
    let credentials = libc::ucred {
        pid: 123,
        uid: 1000,
        gid: 1001,
    };
    let good = b"Pid:\t123\nTgid:\t123\nUid:\t1000 1000 1000 1000\nGid:\t1001 1001 1001 1001\nGroups:\t1 2 3\n";
    assert!(projection(good, &credentials).is_ok());
    for wrong in [
        b"Pid:\t124\n".as_slice(),
        b"Pid:\t123\nPid:\t123\n",
        b"Uid:\t1000\n",
        b"Groups:\t3 2 1\n",
    ] {
        assert!(projection(wrong, &credentials).is_err());
    }
    let mut seed = 0x0751u64;
    for _ in 0..256 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let wrong = libc::ucred {
            pid: credentials.pid,
            uid: credentials.uid + (seed as u32 % 1024 + 1),
            gid: credentials.gid,
        };
        assert!(projection(good, &wrong).is_err());
    }
}
struct Owned(Child);
impl Drop for Owned {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}
#[test]
#[ignore = "manual nonroot Linux/python3 inherited sender and original message pidfd; no live service"]
fn kernel_admission_inherited_socket_identifies_child_and_refuses_after_sigkill() {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    let (mut receiver, fd) = pair();
    let mut connection_peer: libc::ucred = unsafe { std::mem::zeroed() };
    let mut length = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    assert_eq!(
        unsafe {
            libc::getsockopt(
                receiver.fd.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                (&mut connection_peer as *mut libc::ucred).cast(),
                &mut length,
            )
        },
        0
    );
    assert_eq!(connection_peer.pid as u32, std::process::id());
    let mut command = Command::new("python3");
    command.args(["-c", "import socket,sys; s=socket.socket(fileno=int(sys.argv[1])); s.send(b'child'); sys.stdin.buffer.read(1)", &fd.as_raw_fd().to_string()])
        .stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::inherit());
    let raw = fd.as_raw_fd();
    // SAFETY: pre_exec uses only async-signal-safe fcntl on this owned socket.
    unsafe {
        command.pre_exec(move || {
            if libc::fcntl(raw, libc::F_SETFD, 0) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = Owned(command.spawn().unwrap());
    let control = child.0.stdin.take().unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut from_child = loop {
        if let Some(p) = receiver.try_receive().unwrap() {
            break p;
        }
        assert!(Instant::now() < deadline, "owned sender stalled");
        std::thread::yield_now();
    };
    assert_eq!(from_child.sender_pid(), child.0.id());
    assert_ne!(from_child.sender_pid(), std::process::id());
    from_child.revalidate_sender().unwrap();
    send(&fd, b"parent");
    let mut from_parent = packet(&mut receiver);
    assert_eq!(from_parent.sender_pid(), std::process::id());
    child.0.kill().unwrap();
    assert_eq!(child.0.wait().unwrap().signal(), Some(libc::SIGKILL));
    drop(control);
    assert!(from_child.revalidate_sender().is_err());
    from_parent.revalidate_sender().unwrap();
    eprintln!("seed=0x0751 connection_peer=parent inherited_sender=child per_message_pidfd=original actual_sigkill=1 exited_sender=refused parent_sender=still_live production_enrollment=false");
}
