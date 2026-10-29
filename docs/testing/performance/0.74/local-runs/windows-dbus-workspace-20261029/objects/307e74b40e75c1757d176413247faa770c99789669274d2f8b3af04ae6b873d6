use super::*;
use crate::diagnostic_worker_policy::{
    verify_worker_policy, NamespaceIdentity, SignedWorkerPolicy, WorkerPolicy,
    SIGNED_WORKER_POLICY_SCHEMA, WORKER_POLICY_DOMAIN, WORKER_POLICY_SCHEMA,
};
use ed25519_dalek::{Signer, SigningKey};

pub(super) fn checked(epoch: u64) -> CheckedWorkerPolicy {
    let host = AssertedWorkerHost {
        machine_id: "a".repeat(32),
        boot_id: "00000000-0000-0000-0000-000000000001".into(),
        user_namespace: NamespaceIdentity {
            device: 1,
            inode: 2,
        },
        mount_namespace: NamespaceIdentity {
            device: 1,
            inode: 3,
        },
    };
    let policy = WorkerPolicy {
        schema_version: WORKER_POLICY_SCHEMA.into(),
        repository_id: 1_217_101_761,
        purpose: "diagnostic-worker-only".into(),
        account_source: "local-files".into(),
        account: "hydracache-perf".into(),
        group: "hydracache-perf".into(),
        policy_epoch: epoch,
        uid: 1000,
        gid: 1000,
        supplementary_gids: vec![1000],
        machine_id: host.machine_id.clone(),
        boot_id: host.boot_id.clone(),
        user_namespace: host.user_namespace,
        mount_namespace: host.mount_namespace,
        passwd_sha256: "b".repeat(64),
        group_sha256: "c".repeat(64),
    };
    let key = SigningKey::from_bytes(&[74; 32]);
    let mut body = canonical_json(&policy).unwrap();
    body.push(b'\n');
    let trust = WorkerPolicyTrust::new(key.verifying_key(), &sha256_hex(&body), epoch).unwrap();
    let mut message = WORKER_POLICY_DOMAIN.to_vec();
    message.push(0);
    message.extend(body);
    let envelope = SignedWorkerPolicy {
        schema_version: SIGNED_WORKER_POLICY_SCHEMA.into(),
        policy,
        signature_hex: key
            .sign(&message)
            .to_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
    };
    let mut bytes = canonical_json(&envelope).unwrap();
    bytes.push(b'\n');
    verify_worker_policy(&bytes, &trust, &host).unwrap()
}

#[test]
fn authority_registry_canonical_record_and_independent_bindings_are_exact() {
    let p = checked(7);
    let record = Record::from_checked(&p).unwrap();
    let bytes = record.bytes().unwrap();
    assert_eq!(Record::parse(&bytes).unwrap(), record);
    let view =
        inspect_registry_history(std::slice::from_ref(&bytes), &p.trust, &p.context).unwrap();
    assert_eq!(
        view,
        AuthorityView {
            revision: 1,
            epoch: 7,
            refused: false
        }
    );
    let mut different = p.trust.clone();
    different.epoch += 1;
    assert!(matches!(
        inspect_registry_history(std::slice::from_ref(&bytes), &different, &p.context),
        Err(AuthorityError::Binding)
    ));
    for case in 0..6 {
        let mut record = record.clone();
        match case {
            0 => record.issuer_key_hex = "d".repeat(64),
            1 => record.policy_sha256 = "e".repeat(64),
            2 => record.machine_id = "f".repeat(32),
            3 => record.boot_id = "00000000-0000-0000-0000-000000000002".into(),
            4 => record.user_namespace.inode += 1,
            5 => record.mount_namespace.inode += 1,
            _ => unreachable!(),
        }
        assert!(matches!(
            record.bind(&p.trust, &p.context),
            Err(AuthorityError::Binding)
        ));
    }
}

#[test]
fn authority_registry_rejects_malformed_future_duplicate_noncanonical_and_oversized_records() {
    let bytes = Record::from_checked(&checked(7)).unwrap().bytes().unwrap();
    for bad in [
        vec![],
        b"{}".to_vec(),
        vec![b' '; MAX_RECORD_BYTES + 1],
        bytes[..bytes.len() - 1].to_vec(),
        [bytes.clone(), b"\n".to_vec()].concat(),
        [
            b"{\"schema_version\":\"duplicate\",".to_vec(),
            bytes[1..].to_vec(),
        ]
        .concat(),
    ] {
        assert!(Record::parse(&bad).is_err());
    }
    let record = Record::parse(&bytes).unwrap();
    for case in 0..8 {
        let mut r = record.clone();
        match case {
            0 => r.schema_version = "future".into(),
            1 => r.revision = 0,
            2 => r.revision = 129,
            3 => r.policy_epoch = 0,
            4 => r.refusal_reason = Some(RefusalReason::Policy),
            5 => r.machine_id = "0".repeat(32),
            6 => r.policy_sha256 = "G".repeat(64),
            7 => r.previous_sha256 = Some("bad".into()),
            _ => unreachable!(),
        }
        assert!(Record::parse(&r.bytes().unwrap()).is_err());
    }
    let mut extra = serde_json::to_value(&record).unwrap();
    extra["admission_allowed"] = serde_json::json!(true);
    let mut bytes = canonical_json(&extra).unwrap();
    bytes.push(b'\n');
    assert!(Record::parse(&bytes).is_err());
}

#[test]
fn authority_registry_refusal_and_higher_epoch_are_distinct_monotonic_transitions() {
    let old = checked(7);
    let new = checked(8);
    let active = Record::from_checked(&old).unwrap();
    assert!(matches!(
        active.advance(&new),
        Err(AuthorityError::Transition)
    ));
    let refused = active.refuse(RefusalReason::Operator).unwrap();
    assert!(matches!(
        refused.refuse(RefusalReason::Storage),
        Err(AuthorityError::Transition)
    ));
    assert!(matches!(
        refused.advance(&old),
        Err(AuthorityError::Transition)
    ));
    let next = refused.advance(&new).unwrap();
    let raw = vec![
        active.bytes().unwrap(),
        refused.bytes().unwrap(),
        next.bytes().unwrap(),
    ];
    assert_eq!(history(&raw).unwrap().len(), 3);
    assert_eq!(
        inspect_registry_history(&raw, &new.trust, &new.context)
            .unwrap()
            .epoch,
        8
    );
    assert!(matches!(
        inspect_registry_history(&raw, &old.trust, &old.context),
        Err(AuthorityError::Binding)
    ));
    let mut refused_policy = checked(8);
    refused_policy.refused = true;
    assert!(matches!(
        refused.advance(&refused_policy),
        Err(AuthorityError::Refused)
    ));
    let mut changed = checked(8);
    changed.context.machine_id = "d".repeat(32);
    assert!(matches!(
        refused.advance(&changed),
        Err(AuthorityError::Transition)
    ));
    let mut changed = checked(8);
    changed.trust.key = SigningKey::from_bytes(&[75; 32]).verifying_key();
    assert!(matches!(
        refused.advance(&changed),
        Err(AuthorityError::Transition)
    ));
    let mut changed = checked(8);
    changed.trust.policy_sha256 = old.trust.policy_sha256.clone();
    assert!(matches!(
        refused.advance(&changed),
        Err(AuthorityError::Transition)
    ));
}

#[test]
fn authority_registry_seeded_history_mutations_never_adopt_gaps_or_changed_refusals() {
    let mut rng = 0x7642026_u64;
    eprintln!("authority journal seed={rng:#x}; mutations=256");
    let first = Record::from_checked(&checked(7)).unwrap();
    let second = first.refuse(RefusalReason::Policy).unwrap();
    let third = second.advance(&checked(8)).unwrap();
    for _ in 0..256 {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        let mut records = vec![first.clone(), second.clone(), third.clone()];
        match rng % 7 {
            0 => records[1].previous_sha256 = Some("f".repeat(64)),
            1 => records[1].policy_epoch += 1,
            2 => records[1].boot_id = "00000000-0000-0000-0000-000000000002".into(),
            3 => records[2].policy_epoch = 7,
            4 => records[2].revision = 4,
            5 => records[2].issuer_key_hex = "e".repeat(64),
            6 => {
                records.remove(1);
            }
            _ => unreachable!(),
        }
        let raw = records
            .iter()
            .map(|r| r.bytes().unwrap())
            .collect::<Vec<_>>();
        assert!(history(&raw).is_err());
    }
}

#[test]
fn authority_registry_generation_capacity_is_finite_and_no_epoch_wrap_is_possible() {
    assert!(history(&[]).is_err());
    let mut records = vec![Record::from_checked(&checked(1)).unwrap()];
    for index in 2..=MAX_GENERATIONS {
        let old = records.last().unwrap();
        let next = if index % 2 == 0 {
            old.refuse(RefusalReason::Operator).unwrap()
        } else {
            old.advance(&checked(index as u64)).unwrap()
        };
        records.push(next);
    }
    let raw = records
        .iter()
        .map(|r| r.bytes().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(history(&raw).unwrap().len(), MAX_GENERATIONS);
    assert!(matches!(
        records.last().unwrap().advance(&checked(u64::MAX)),
        Err(AuthorityError::Budget)
    ));
    let mut overflow = raw;
    overflow.push(overflow[0].clone());
    assert!(matches!(history(&overflow), Err(AuthorityError::Budget)));
}
