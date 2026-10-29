use super::*;
use crate::diagnostic_start_consent::{
    start_consent_signing_message, SignedStartConsent, BODY_SCHEMA, ENVELOPE_SCHEMA,
};
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
fn body(point: &KernelTimePoint) -> StartConsentBody {
    serde_json::from_value(serde_json::json!({
        "schema_version":BODY_SCHEMA,"action":"start-cell-one",
        "consent_request_id":"1".repeat(64),"reservation_request_id":"2".repeat(64),
        "controller_attempt_sha256":"3".repeat(64),"collector_reserved_sha256":"4".repeat(64),
        "reserved_state_sha256":"5".repeat(64),"challenge_nonce_sha256":"6".repeat(64),
        "authenticated_response_sha256":"7".repeat(64),"worker_policy_sha256":"8".repeat(64),
        "worker_policy_epoch":1,"collector_unit_sha256":"9".repeat(64),
        "controller":{"repository_id":1,"run_id":2,"run_attempt":1,"actor_id":3},
        "source_commit":crate::diagnostic_lease::SOURCE_COMMIT,"preset":crate::diagnostic_ipc::PRESET,
        "starting_state":{
            "identity":{"lease_id":"a".repeat(64),"boot_id":point.boot_id(),"binary_sha256":"b".repeat(64),"build_provenance_sha256":"c".repeat(64)},
            "revision":2,"stage":"starting","completed_cells":0,
            "reserved_monotonic_ns":1,"controller_monotonic_ns":2,"last_observed_monotonic_ns":2,
            "cell_started_monotonic_ns":2,"cgroup_inode":null,"reason":null,
            "cleanup_confirmed":false,"promotable":false,"admission_allowed":false
        },
        "issued_at_unix_seconds":point.unix_seconds(),"expires_at_unix_seconds":point.unix_seconds()+60
    })).unwrap()
}
fn signed(body: StartConsentBody, key: &SigningKey) -> Vec<u8> {
    let signature_hex = crate::hex(
        &key.sign(&start_consent_signing_message(&body).unwrap())
            .to_bytes(),
    );
    let mut bytes = crate::canonical_json(&SignedStartConsent {
        schema_version: ENVELOPE_SCHEMA.into(),
        consent: body,
        signature_hex,
    })
    .unwrap();
    bytes.push(b'\n');
    bytes
}
#[test]
fn original_start_consent_real_kernel_origin_peer_signature_order_and_refusal() {
    for case in 0..12 {
        let (mut receiver, sender) = pair();
        let mut original = receive(&mut receiver, &sender, b"original");
        let point = original.received_time().clone();
        let key = SigningKey::from_bytes(&[123; 32]);
        let mut policy = DiagnosticPolicy {
            repository_id: 1,
            actor_ids: vec![3],
            client_uids: vec![unsafe { libc::geteuid() }],
            client_gid: unsafe { libc::getegid() },
            verifying_key: key.verifying_key(),
        };
        let mut b = body(&point);
        if case == 5 {
            b.issued_at_unix_seconds = 1;
            b.expires_at_unix_seconds = 2;
        }
        if case == 7 {
            policy.client_uids.clear();
        }
        if case == 8 {
            policy.repository_id = 4;
        }
        if case == 9 {
            policy.actor_ids.clear();
        }
        if case == 10 {
            policy.client_gid = u32::MAX;
        }
        if case == 11 {
            b.starting_state.identity.boot_id = "00000000-0000-4000-8000-000000000074".into();
        }
        let bytes = if case == 2 {
            b"malformed".to_vec()
        } else if case == 3 {
            signed(b, &SigningKey::from_bytes(&[124; 32]))
        } else {
            signed(b, &key)
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
            point.clone()
        };
        if case == 6 {
            next.latch_refusal();
        }
        let result = original.authenticate_original_start_consent(&mut next, &policy, &after);
        if case == 0 {
            assert_eq!(result.unwrap().consent_request_id, "1".repeat(64));
            original
                .authenticate_original_start_consent(&mut next, &policy, &after)
                .unwrap();
            assert!(!original.is_refused() && !next.is_refused());
        } else {
            assert!(result.is_err(), "case={case}");
            assert!(original.is_refused() && next.is_refused(), "case={case}");
            assert!(matches!(
                original.authenticate_original_start_consent(&mut next, &policy, &after),
                Err(KernelError::Refused)
            ));
        }
        assert_eq!(original.received_time().boottime_ns(), point.boottime_ns());
    }
    println!("original_start_consent kernel_cases=12 real_peer=true original_receive_time_unchanged=true replay_admission=false");
}
#[test]
fn original_start_consent_exclusive_age_and_principal_time_boundaries() {
    let limit = crate::diagnostic_lease::CONTROLLER_LOSS_SECONDS * 1_000_000_000;
    for (received, after, now, ok) in [
        (2, 1, 2, true),
        (2, 1, 2 + limit - 1, true),
        (2, 1, 2 + limit, false),
        (2, 2, 2, false),
        (1, 2, 3, false),
        (2, 1, 1, false),
        (u64::MAX, u64::MAX - 1, u64::MAX, true),
    ] {
        assert_eq!(age(received, after, now).is_ok(), ok);
    }
    let (mut receiver, sender) = pair();
    let packet = receive(&mut receiver, &sender, b"context");
    let mut b = body(packet.received_time());
    b.issued_at_unix_seconds = 100;
    b.expires_at_unix_seconds = 160;
    let policy = DiagnosticPolicy {
        repository_id: 1,
        actor_ids: vec![3],
        client_uids: vec![4],
        client_gid: 5,
        verifying_key: SigningKey::from_bytes(&[123; 32]).verifying_key(),
    };
    let peer = DiagnosticPeer {
        uid: 4,
        gids: vec![5],
    };
    for (now, ok) in [
        (99, false),
        (100, true),
        (159, true),
        (160, false),
        (161, false),
    ] {
        assert_eq!(
            principal_and_time(&b, &policy, &peer, now, packet.received_time().boot_id()).is_ok(),
            ok
        );
    }
}
