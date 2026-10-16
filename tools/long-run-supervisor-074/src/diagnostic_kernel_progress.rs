//! A fresh original-receiver observation, never a renewed admission packet.
use super::*;

fn progress_age(received: u64, after: u64, now: u64) -> Result<(), KernelError> {
    if received <= after
        || now < received
        || now - received >= crate::diagnostic_lease::CONTROLLER_LOSS_SECONDS * 1_000_000_000
    {
        return Err(kernel_invalid!(
            "progress-age",
            [
                received,
                after,
                now,
                0,
                0,
                crate::diagnostic_lease::CONTROLLER_LOSS_SECONDS * 1_000_000_000
            ]
        ));
    }
    Ok(())
}

impl KernelPacket {
    pub(crate) fn authenticate_original_progress(
        &mut self,
        next: &mut KernelPacket,
        policy: &DiagnosticPolicy,
        after: &KernelTimePoint,
    ) -> Result<DiagnosticRequest, KernelError> {
        let result = (|| {
            if self.is_refused() || next.is_refused() {
                return Err(KernelError::Refused);
            }
            if !Arc::ptr_eq(&self.origin, &next.origin) {
                return Err(KernelError::Invalid);
            }
            // Only this receiver's original objects and samples can enter this path.
            ordered(&self.received, &next.received)?;
            ordered(after, &next.received)?;
            let before = self.original_reservation_time(Some(after))?;
            ordered(&next.received, &before)?;
            progress_age(
                next.received.boottime_ns(),
                after.boottime_ns(),
                before.boottime_ns(),
            )?;
            let request = next.authenticate(policy)?;
            let now = self.original_reservation_time(Some(&before))?;
            ordered(&next.received, &now)?;
            progress_age(
                next.received.boottime_ns(),
                after.boottime_ns(),
                now.boottime_ns(),
            )?;
            Ok(request)
        })();
        if result.is_err() {
            self.latch_refusal();
            next.latch_refusal();
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic_ipc::{
        DiagnosticController, DiagnosticOperation, SignedDiagnosticRequest,
    };
    use crate::diagnostic_lease::{DiagnosticIdentity, SOURCE_COMMIT};
    use ed25519_dalek::{Signer, SigningKey};
    use std::os::fd::FromRawFd;

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
        (
            KernelReceiver::new(unsafe { OwnedFd::from_raw_fd(fds[0]) }).unwrap(),
            unsafe { OwnedFd::from_raw_fd(fds[1]) },
        )
    }
    fn receive(receiver: &mut KernelReceiver, sender: &OwnedFd, bytes: &[u8]) -> KernelPacket {
        assert_eq!(
            unsafe {
                libc::send(
                    sender.as_raw_fd(),
                    bytes.as_ptr().cast(),
                    bytes.len(),
                    libc::MSG_NOSIGNAL,
                )
            },
            bytes.len() as isize
        );
        receiver.try_receive().unwrap().unwrap()
    }
    fn request(point: &KernelTimePoint) -> DiagnosticRequest {
        DiagnosticRequest {
            schema_version: 1,
            request_id: "a".repeat(64),
            nonce_sha256: "b".repeat(64),
            operation: DiagnosticOperation::Heartbeat,
            expected_state_revision: 1,
            identity: DiagnosticIdentity {
                lease_id: "c".repeat(64),
                boot_id: point.boot_id().into(),
                binary_sha256: "d".repeat(64),
                build_provenance_sha256: "e".repeat(64),
            },
            source_commit: SOURCE_COMMIT.into(),
            preset: crate::diagnostic_ipc::PRESET.into(),
            controller: DiagnosticController {
                repository_id: 1,
                run_id: 2,
                run_attempt: 1,
                actor_id: 3,
            },
            issued_at_unix_seconds: point.unix_seconds(),
            expires_at_unix_seconds: point.unix_seconds() + 60,
        }
    }
    fn signed(request: DiagnosticRequest, key: &SigningKey) -> Vec<u8> {
        let signature_hex = crate::hex(
            &key.sign(&crate::diagnostic_ipc::signing_message(&request).unwrap())
                .to_bytes(),
        );
        crate::canonical_json(&SignedDiagnosticRequest {
            request,
            signature_hex,
        })
        .unwrap()
    }
    #[test]
    fn collector_progress_real_kernel_origin_signature_order_and_sticky_refusal() {
        // These are private kernel observations with a synthetic key/policy, not enrollment.
        for case in 0..7 {
            let (mut receiver, sender) = pair();
            let mut original = receive(&mut receiver, &sender, b"original-received-message");
            let received = original.received_time().clone();
            let key = SigningKey::from_bytes(&[121; 32]);
            let policy = DiagnosticPolicy {
                repository_id: 1,
                actor_ids: vec![3],
                client_uids: vec![unsafe { libc::geteuid() }],
                client_gid: unsafe { libc::getegid() },
                verifying_key: key.verifying_key(),
            };
            let mut r = request(&received);
            if case == 5 {
                r.issued_at_unix_seconds = 1;
                r.expires_at_unix_seconds = 2;
            }
            let bytes = if case == 2 {
                b"malformed".to_vec()
            } else if case == 3 {
                signed(r.clone(), &SigningKey::from_bytes(&[122; 32]))
            } else {
                signed(r.clone(), &key)
            };
            let mut next = if case == 1 {
                let (mut other, fd) = pair();
                receive(&mut other, &fd, &bytes)
            } else {
                receive(&mut receiver, &sender, &bytes)
            };
            let after = if case == 4 {
                next.received_time().clone()
            } else {
                received.clone()
            };
            if case == 6 {
                next.latch_refusal();
            }
            let result = original.authenticate_original_progress(&mut next, &policy, &after);
            if case == 0 {
                assert_eq!(result.unwrap(), r);
                original
                    .authenticate_original_progress(&mut next, &policy, &after)
                    .unwrap();
                assert!(!original.is_refused() && !next.is_refused());
            } else {
                assert!(result.is_err(), "case={case}");
                assert!(original.is_refused() && next.is_refused());
                assert!(matches!(
                    original.authenticate_original_progress(&mut next, &policy, &after),
                    Err(KernelError::Refused)
                ));
            }
            assert_eq!(
                original.received_time().boottime_ns(),
                received.boottime_ns()
            );
        }
        println!("collector_progress kernel_cases=7 renewed_receive_time=false");
    }
    #[test]
    fn collector_progress_real_packet_expires_without_receive_time_renewal() {
        let (mut receiver, sender) = pair();
        let mut original = receive(&mut receiver, &sender, b"original");
        let after = original.received_time().clone();
        let key = SigningKey::from_bytes(&[121; 32]);
        let policy = DiagnosticPolicy {
            repository_id: 1,
            actor_ids: vec![3],
            client_uids: vec![unsafe { libc::geteuid() }],
            client_gid: unsafe { libc::getegid() },
            verifying_key: key.verifying_key(),
        };
        let mut next = receive(&mut receiver, &sender, &signed(request(&after), &key));
        let received = next.received_time().boottime_ns();
        original
            .authenticate_original_progress(&mut next, &policy, &after)
            .unwrap();
        // Real bounded expiration, not an injected timestamp or extended deadline.
        std::thread::sleep(std::time::Duration::from_secs(
            crate::diagnostic_lease::CONTROLLER_LOSS_SECONDS,
        ));
        let (result, event) = crate::diagnostic_kernel_admission::refusal_trace::capture(|| {
            original.authenticate_original_progress(&mut next, &policy, &after)
        });
        assert!(matches!(result, Err(KernelError::Invalid)));
        assert_eq!(event.unwrap().site, "progress-age");
        assert_eq!(next.received_time().boottime_ns(), received);
        assert!(original.is_refused() && next.is_refused());
        println!("collector_progress real_expiry=true receive_time_unchanged=true deadline_extended=false");
    }
    #[test]
    fn collector_progress_exclusive_age_and_strict_receive_order() {
        let loss = crate::diagnostic_lease::CONTROLLER_LOSS_SECONDS * 1_000_000_000;
        for (received, after, now, ok) in [
            (2, 1, 2, true),
            (2, 1, 2 + loss - 1, true),
            (2, 1, 2 + loss, false),
            (2, 1, 3 + loss, false),
            (2, 2, 2, false),
            (1, 2, 3, false),
            (2, 1, 1, false),
            (u64::MAX, u64::MAX - 1, u64::MAX, true),
        ] {
            assert_eq!(progress_age(received, after, now).is_ok(), ok);
        }
    }
}
