//! Synthetic controller signatures and public historical builder receipt; no enrollment.
use super::*;
use crate::diagnostic_ipc::{DiagnosticController, DiagnosticOperation};
use crate::diagnostic_lease::DiagnosticIdentity;
use serde_json::{json, Value};
// DiagnosticIdentity is owned by lease; import through its defining module.
const POLICY: &[u8] =
    include_bytes!("../../../docs/testing/performance/0.74/diagnostic-builder-public-policy.json");
const RECEIPT:&[u8]=include_bytes!("../../../docs/testing/performance/0.74/local-runs/diagnostic-builder-hosted-37923423889/build-receipt-v1.json");
pub(super) fn model_request() -> DiagnosticRequest {
    let now = job_unix_seconds().unwrap();
    DiagnosticRequest {
        schema_version: 1,
        request_id: "a".repeat(64),
        nonce_sha256: "b".repeat(64),
        operation: DiagnosticOperation::Reserve,
        expected_state_revision: 0,
        identity: DiagnosticIdentity {
            lease_id: "c".repeat(64),
            boot_id: "00000000-0000-0000-0000-000000000000".into(),
            binary_sha256: "bda1c094f3de7c6d42ec45c757838c7a41a8ff1d63a025c661af84455169a101"
                .into(),
            build_provenance_sha256: crate::sha256_hex(RECEIPT),
        },
        source_commit: crate::diagnostic_lease::SOURCE_COMMIT.into(),
        preset: diagnostic_ipc::PRESET.into(),
        controller: DiagnosticController {
            repository_id: crate::diagnostic_builder::REPOSITORY_ID,
            run_id: 77,
            run_attempt: 1,
            actor_id: 42,
        },
        issued_at_unix_seconds: now,
        expires_at_unix_seconds: now + 60,
    }
}
pub(super) fn line(v: &impl serde::Serialize) -> Vec<u8> {
    let mut raw = crate::canonical_json(v).unwrap();
    raw.push(b'\n');
    raw
}
fn prepared(r: &DiagnosticRequest) -> PreparedDiagnosticRequest {
    prepare_request(&line(r), POLICY, RECEIPT).unwrap()
}
#[test]
fn producer_roundtrip_exact_request_domain_audience_and_synthetic_signature() {
    let r = model_request();
    let p = prepared(&r);
    let hash = crate::sha256_hex(&crate::canonical_json(&r).unwrap());
    assert_eq!(p.request_sha256(), hash);
    assert_eq!(
        p.audience(),
        format!("urn:hydracache:diagnostic:074:request:{hash}")
    );
    let key = SigningKey::from_bytes(&[76; 32]);
    let bytes = p
        .sign_checked(&key, &key.verifying_key(), r.issued_at_unix_seconds)
        .unwrap();
    let signed: SignedDiagnosticRequest = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(signed.request, r);
    let policy = diagnostic_ipc::DiagnosticPolicy {
        repository_id: r.controller.repository_id,
        actor_ids: vec![42],
        client_uids: vec![1000],
        client_gid: 1000,
        verifying_key: key.verifying_key(),
    };
    let authenticated = diagnostic_ipc::authenticate(
        &bytes,
        &diagnostic_ipc::DiagnosticPeer {
            uid: 1000,
            gids: vec![1000],
        },
        &policy,
        r.issued_at_unix_seconds,
    )
    .unwrap();
    assert_eq!(authenticated, r);
    assert_eq!(line(&signed), bytes);
}
#[test]
fn producer_wrong_selected_key_time_and_retained_identity_refuse() {
    let r = model_request();
    let key = SigningKey::from_bytes(&[76; 32]);
    assert!(matches!(
        prepared(&r).sign_with_selected_controller(&key),
        Err(ProducerError::Key)
    ));
    let wrong = SigningKey::from_bytes(&[77; 32]);
    assert!(matches!(
        prepared(&r).sign_checked(&key, &wrong.verifying_key(), r.issued_at_unix_seconds),
        Err(ProducerError::Key)
    ));
    for now in [r.issued_at_unix_seconds - 1, r.expires_at_unix_seconds] {
        assert!(matches!(
            prepared(&r).sign_checked(&key, &key.verifying_key(), now),
            Err(ProducerError::Time)
        ));
    }
    prepared(&r)
        .sign_checked(&key, &key.verifying_key(), r.expires_at_unix_seconds - 1)
        .unwrap();
    for case in 0..2 {
        let mut p = prepared(&r);
        if case == 0 {
            p.request.controller.run_id += 1;
        } else {
            p.request.identity.binary_sha256 = "0".repeat(64);
        }
        assert!(p
            .sign_checked(&key, &key.verifying_key(), r.issued_at_unix_seconds)
            .is_err());
    }
}
#[test]
fn producer_strict_request_schema_builder_identity_and_budget() {
    let r = model_request();
    let raw = line(&r);
    let original: Value = serde_json::from_slice(&raw).unwrap();
    for field in original.as_object().unwrap().keys() {
        let mut v = original.clone();
        v.as_object_mut().unwrap().remove(field);
        assert!(prepare_request(&line(&v), POLICY, RECEIPT).is_err());
        v = original.clone();
        v[field] = Value::Null;
        assert!(prepare_request(&line(&v), POLICY, RECEIPT).is_err());
    }
    for (field, value) in [
        ("schema_version", json!(2)),
        ("source_commit", json!("0".repeat(40))),
        ("preset", json!("other")),
        ("nonce_sha256", json!("bad")),
        ("expected_state_revision", json!(1)),
        (
            "expires_at_unix_seconds",
            json!(r.issued_at_unix_seconds + 61),
        ),
        ("unknown", json!(true)),
    ] {
        let mut v = original.clone();
        v[field] = value;
        assert!(prepare_request(&line(&v), POLICY, RECEIPT).is_err());
    }
    for bad in [
        vec![],
        vec![0; MAX_PACKET_BYTES + 1],
        raw[..raw.len() - 1].to_vec(),
        [raw.clone(), b" ".to_vec()].concat(),
        format!(
            "{{\"schema_version\":1,{}",
            String::from_utf8(raw).unwrap().trim_start_matches('{')
        )
        .into_bytes(),
    ] {
        assert!(prepare_request(&bad, POLICY, RECEIPT).is_err());
    }
    for case in 0..3 {
        let mut wrong = r.clone();
        match case {
            0 => wrong.controller.repository_id = 17,
            1 => wrong.identity.binary_sha256 = "0".repeat(64),
            _ => wrong.identity.build_provenance_sha256 = "0".repeat(64),
        }
        assert!(prepare_request(&line(&wrong), POLICY, RECEIPT).is_err());
    }
    assert!(prepare_request(&line(&r), b"{}", RECEIPT).is_err());
    assert!(prepare_request(&line(&r), POLICY, b"{}").is_err());
}
#[test]
fn producer_seeded_valid_messages_and_independent_preparations() {
    let mut seed = 0x07562074_u64;
    for _ in 0..32 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut r = model_request();
        r.controller.run_id = 1 + seed % 10000;
        r.operation = match seed % 4 {
            0 => DiagnosticOperation::Reserve,
            1 => DiagnosticOperation::Heartbeat,
            2 => DiagnosticOperation::Status,
            _ => DiagnosticOperation::Cancel,
        };
        if r.operation != DiagnosticOperation::Reserve {
            r.expected_state_revision = 1;
        }
        let key = SigningKey::from_bytes(&[76; 32]);
        let p = prepared(&r);
        let expected = p.request_sha256().to_owned();
        let bytes = p
            .sign_checked(&key, &key.verifying_key(), r.issued_at_unix_seconds)
            .unwrap();
        let signed: SignedDiagnosticRequest = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            crate::sha256_hex(&crate::canonical_json(&signed.request).unwrap()),
            expected
        );
    }
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|_| {
                scope.spawn(|| {
                    let r = model_request();
                    let key = SigningKey::from_bytes(&[76; 32]);
                    prepared(&r)
                        .sign_checked(&key, &key.verifying_key(), r.issued_at_unix_seconds)
                        .unwrap();
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
    });
    println!("seed=0x07562074 signed_model_requests=32 independent_preparations=4 real_controller_signing=false");
}
