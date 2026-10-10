//! Synthetic operator keys and asserted context; no real host/account enrollment.
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use hydracache_long_run_supervisor_074::diagnostic_worker_policy::*;
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn canonical<T: Serialize>(value: &T) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(&serde_json::to_value(value).unwrap()).unwrap();
    bytes.push(b'\n');
    bytes
}
fn digest(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}
fn policy() -> WorkerPolicy {
    WorkerPolicy {
        schema_version: WORKER_POLICY_SCHEMA.into(),
        repository_id: 1_217_101_761,
        purpose: "diagnostic-worker-only".into(),
        account_source: "local-files".into(),
        account: "hydracache-perf".into(),
        group: "hydracache-perf".into(),
        policy_epoch: 7,
        uid: 986,
        gid: 986,
        supplementary_gids: vec![987, 988],
        machine_id: "a".repeat(32),
        boot_id: "11111111-2222-3333-4444-555555555555".into(),
        user_namespace: NamespaceIdentity {
            device: 4,
            inode: 111,
        },
        mount_namespace: NamespaceIdentity {
            device: 4,
            inode: 222,
        },
        passwd_sha256: "b".repeat(64),
        group_sha256: "c".repeat(64),
    }
}
fn context(p: &WorkerPolicy) -> AssertedWorkerHost {
    AssertedWorkerHost {
        machine_id: p.machine_id.clone(),
        boot_id: p.boot_id.clone(),
        user_namespace: p.user_namespace,
        mount_namespace: p.mount_namespace,
    }
}
fn key() -> SigningKey {
    SigningKey::from_bytes(&[74; 32])
}
fn envelope(p: &WorkerPolicy, signing_key: &SigningKey, domain: &[u8]) -> Vec<u8> {
    let mut message = domain.to_vec();
    message.push(0);
    message.extend(canonical(p));
    canonical(&SignedWorkerPolicy {
        schema_version: SIGNED_WORKER_POLICY_SCHEMA.into(),
        policy: p.clone(),
        signature_hex: hex(&signing_key.sign(&message).to_bytes()),
    })
}
fn trust(p: &WorkerPolicy) -> WorkerPolicyTrust {
    WorkerPolicyTrust::new(
        key().verifying_key(),
        &digest(&canonical(p)),
        p.policy_epoch,
    )
    .unwrap()
}
fn error(bytes: &[u8], t: &WorkerPolicyTrust, host: &AssertedWorkerHost) -> WorkerPolicyError {
    verify_worker_policy(bytes, t, host)
        .map(|_| ())
        .unwrap_err()
}

#[test]
fn worker_policy_roundtrip_is_idempotent_not_host_enrollment() {
    let p = policy();
    let t = trust(&p);
    let host = context(&p);
    let bytes = envelope(&p, &key(), WORKER_POLICY_DOMAIN);
    let mut checked = verify_worker_policy(&bytes, &t, &host).unwrap();
    assert_eq!(checked.policy_sha256(), digest(&canonical(&p)));
    for _ in 0..3 {
        checked.revalidate(&bytes, &t, &host).unwrap();
    }
    assert!(!checked.is_refused());
}

#[test]
fn worker_policy_unsigned_field_changes_never_adopt_new_values() {
    let p = policy();
    let t = trust(&p);
    let host = context(&p);
    let bytes = envelope(&p, &key(), WORKER_POLICY_DOMAIN);
    let original: Value = serde_json::from_slice(&bytes).unwrap();
    for field in original["policy"].as_object().unwrap().keys() {
        let mut changed = original.clone();
        changed["policy"][field] = Value::Null;
        assert_eq!(
            error(&canonical(&changed), &t, &host),
            WorkerPolicyError::Document,
            "{field}"
        );
    }
    let mut changed = original;
    changed["policy"]["uid"] = json!(987);
    assert_eq!(
        error(&canonical(&changed), &t, &host),
        WorkerPolicyError::Pin
    );
}

#[test]
fn worker_policy_resigned_invalid_identity_and_context_refuse() {
    let mutations: [fn(&mut WorkerPolicy); 17] = [
        |p| p.schema_version.push('2'),
        |p| p.repository_id += 1,
        |p| p.purpose = "product-start".into(),
        |p| p.account_source = "nss".into(),
        |p| p.account = "root".into(),
        |p| p.group = "root".into(),
        |p| p.uid = 0,
        |p| p.uid = u32::MAX,
        |p| p.gid = 0,
        |p| p.gid = u32::MAX,
        |p| p.policy_epoch = 0,
        |p| p.machine_id = "0".repeat(32),
        |p| p.boot_id = "00000000-0000-0000-0000-000000000000".into(),
        |p| p.user_namespace.inode = 0,
        |p| p.mount_namespace.device = 0,
        |p| p.passwd_sha256 = "B".repeat(64),
        |p| p.group_sha256.push('0'),
    ];
    let original = policy();
    for mutate in mutations {
        let mut p = original.clone();
        mutate(&mut p);
        let t = trust(&original);
        assert_eq!(
            error(
                &envelope(&p, &key(), WORKER_POLICY_DOMAIN),
                &t,
                &context(&original)
            ),
            WorkerPolicyError::Invalid
        );
    }
}

#[test]
fn worker_policy_canonical_strict_json_and_budgets_refuse() {
    let p = policy();
    let t = trust(&p);
    let host = context(&p);
    let bytes = envelope(&p, &key(), WORKER_POLICY_DOMAIN);
    let mut variants = vec![
        vec![],
        vec![b' '; MAX_SIGNED_WORKER_POLICY_BYTES + 1],
        b"{}\n".to_vec(),
        bytes[..bytes.len() - 1].to_vec(),
    ];
    variants.push([bytes.as_slice(), b"\n"].concat());
    variants.push(
        String::from_utf8(bytes.clone())
            .unwrap()
            .replace('\n', "\r\n")
            .into_bytes(),
    );
    let mut value: Value = serde_json::from_slice(&bytes).unwrap();
    value["key_hex"] = json!("a".repeat(64));
    variants.push(canonical(&value));
    let mut value: Value = serde_json::from_slice(&bytes).unwrap();
    value["policy"]["unknown"] = json!(true);
    variants.push(canonical(&value));
    let duplicate = String::from_utf8(bytes.clone()).unwrap().replacen(
        "\"uid\":986",
        "\"uid\":986,\"uid\":986",
        1,
    );
    assert_ne!(duplicate.as_bytes(), bytes);
    variants.push(duplicate.into_bytes());
    let duplicate = String::from_utf8(bytes.clone()).unwrap().replacen(
        "\"inode\":111",
        "\"inode\":111,\"inode\":111",
        1,
    );
    assert_ne!(duplicate.as_bytes(), bytes);
    variants.push(duplicate.into_bytes());
    let mut value: Value = serde_json::from_slice(&bytes).unwrap();
    value["policy"]["account"] = json!("x".repeat(MAX_WORKER_POLICY_BYTES));
    variants.push(canonical(&value));
    variants.push(
        serde_json::to_vec_pretty(&serde_json::from_slice::<Value>(&bytes).unwrap()).unwrap(),
    );
    for variant in variants {
        assert_eq!(error(&variant, &t, &host), WorkerPolicyError::Document);
    }
    let mut value: Value = serde_json::from_slice(&bytes).unwrap();
    value["schema_version"] = json!("future");
    assert_eq!(
        error(&canonical(&value), &t, &host),
        WorkerPolicyError::Invalid
    );
}

#[test]
fn worker_policy_external_trust_rejects_weak_keys_and_invalid_pins() {
    let p = policy();
    let pin = digest(&canonical(&p));
    let weak = VerifyingKey::from_bytes(&[0; 32]).unwrap();
    assert!(weak.is_weak());
    assert!(WorkerPolicyTrust::new(weak, &pin, 7).is_err());
    assert!(WorkerPolicyTrust::new(key().verifying_key(), &pin, 0).is_err());
    for invalid in [
        "".to_owned(),
        pin.to_uppercase(),
        "a".repeat(63),
        "z".repeat(64),
    ] {
        assert!(WorkerPolicyTrust::new(key().verifying_key(), &invalid, 7).is_err());
    }
    let wrong_pin = WorkerPolicyTrust::new(key().verifying_key(), &"d".repeat(64), 7).unwrap();
    assert_eq!(
        error(
            &envelope(&p, &key(), WORKER_POLICY_DOMAIN),
            &wrong_pin,
            &context(&p)
        ),
        WorkerPolicyError::Pin
    );
    let wrong_epoch = WorkerPolicyTrust::new(key().verifying_key(), &pin, 8).unwrap();
    assert_eq!(
        error(
            &envelope(&p, &key(), WORKER_POLICY_DOMAIN),
            &wrong_epoch,
            &context(&p)
        ),
        WorkerPolicyError::Revoked
    );
}

#[test]
fn worker_policy_signatures_and_foreign_domains_refuse() {
    let p = policy();
    let t = trust(&p);
    let host = context(&p);
    let wrong = SigningKey::from_bytes(&[73; 32]);
    assert_eq!(
        error(&envelope(&p, &wrong, WORKER_POLICY_DOMAIN), &t, &host),
        WorkerPolicyError::Signature
    );
    assert_eq!(
        error(
            &envelope(&p, &key(), b"hydracache-long-run-authorization-v1"),
            &t,
            &host
        ),
        WorkerPolicyError::Signature
    );
    let bytes = envelope(&p, &key(), WORKER_POLICY_DOMAIN);
    let mut value: Value = serde_json::from_slice(&bytes).unwrap();
    value["signature_hex"] = json!("0".repeat(128));
    assert_eq!(
        error(&canonical(&value), &t, &host),
        WorkerPolicyError::Signature
    );
    for malformed in ["F".repeat(128), "0".repeat(127), "z".repeat(128)] {
        value["signature_hex"] = json!(malformed);
        assert_eq!(
            error(&canonical(&value), &t, &host),
            WorkerPolicyError::Document
        );
    }
}

#[test]
fn worker_policy_asserted_host_boot_and_both_namespaces_match_exactly() {
    let p = policy();
    let t = trust(&p);
    let bytes = envelope(&p, &key(), WORKER_POLICY_DOMAIN);
    let mutations: [fn(&mut AssertedWorkerHost); 5] = [
        |h| h.machine_id = "d".repeat(32),
        |h| h.boot_id = "11111111-2222-3333-4444-555555555556".into(),
        |h| h.user_namespace.inode += 1,
        |h| h.mount_namespace.inode += 1,
        |h| h.mount_namespace.device += 1,
    ];
    for mutate in mutations {
        let mut host = context(&p);
        mutate(&mut host);
        assert_eq!(error(&bytes, &t, &host), WorkerPolicyError::Context);
    }
    let mut host = context(&p);
    host.boot_id = "not-a-uuid".into();
    assert_eq!(error(&bytes, &t, &host), WorkerPolicyError::Invalid);
}

#[test]
fn worker_policy_group_rules_preserve_exact_lists_and_union_bound() {
    let mut p = policy();
    p.supplementary_gids.clear();
    verify_worker_policy(
        &envelope(&p, &key(), WORKER_POLICY_DOMAIN),
        &trust(&p),
        &context(&p),
    )
    .unwrap();
    p.supplementary_gids = (986..1018).collect();
    verify_worker_policy(
        &envelope(&p, &key(), WORKER_POLICY_DOMAIN),
        &trust(&p),
        &context(&p),
    )
    .unwrap();
    for groups in [
        vec![0],
        vec![u32::MAX],
        vec![987, 987],
        vec![988, 987],
        (987..1019).collect(),
        (986..1019).collect(),
    ] {
        p.supplementary_gids = groups;
        assert_eq!(
            error(
                &envelope(&p, &key(), WORKER_POLICY_DOMAIN),
                &trust(&p),
                &context(&p)
            ),
            WorkerPolicyError::Invalid
        );
    }
}

#[test]
fn worker_policy_revocation_is_first_error_and_cannot_restore() {
    let p = policy();
    let t = trust(&p);
    let host = context(&p);
    let bytes = envelope(&p, &key(), WORKER_POLICY_DOMAIN);
    let mut checked = verify_worker_policy(&bytes, &t, &host).unwrap();
    let revoked =
        WorkerPolicyTrust::new(key().verifying_key(), &digest(&canonical(&p)), 8).unwrap();
    let mut drifted = host.clone();
    drifted.user_namespace.inode += 1;
    assert_eq!(
        checked.revalidate(b"", &revoked, &drifted),
        Err(WorkerPolicyError::Revoked)
    );
    assert!(checked.is_refused());
    assert_eq!(
        checked.revalidate(&bytes, &t, &host),
        Err(WorkerPolicyError::Refused)
    );
    assert_eq!(checked.policy_sha256(), digest(&canonical(&p)));
}

#[test]
fn worker_policy_context_and_envelope_drift_latch_without_refresh() {
    let p = policy();
    let t = trust(&p);
    let host = context(&p);
    let bytes = envelope(&p, &key(), WORKER_POLICY_DOMAIN);
    let mut checked = verify_worker_policy(&bytes, &t, &host).unwrap();
    let mut drifted = host.clone();
    drifted.mount_namespace.inode += 1;
    assert_eq!(
        checked.revalidate(b"", &t, &drifted),
        Err(WorkerPolicyError::Context)
    );
    assert_eq!(
        checked.revalidate(&bytes, &t, &host),
        Err(WorkerPolicyError::Refused)
    );
    let mut checked = verify_worker_policy(&bytes, &t, &host).unwrap();
    assert_eq!(
        checked.revalidate(b"", &t, &host),
        Err(WorkerPolicyError::Drift)
    );
    assert_eq!(
        checked.revalidate(&bytes, &t, &host),
        Err(WorkerPolicyError::Refused)
    );
}

#[test]
fn worker_policy_oversized_revalidation_and_key_pin_changes_latch() {
    let p = policy();
    let t = trust(&p);
    let host = context(&p);
    let bytes = envelope(&p, &key(), WORKER_POLICY_DOMAIN);
    let changed_key = WorkerPolicyTrust::new(
        SigningKey::from_bytes(&[75; 32]).verifying_key(),
        &digest(&canonical(&p)),
        7,
    )
    .unwrap();
    let changed_pin = WorkerPolicyTrust::new(key().verifying_key(), &"d".repeat(64), 7).unwrap();
    for changed in [changed_key, changed_pin] {
        let mut checked = verify_worker_policy(&bytes, &t, &host).unwrap();
        assert_eq!(
            checked.revalidate(&bytes, &changed, &host),
            Err(WorkerPolicyError::Revoked)
        );
        assert_eq!(
            checked.revalidate(&bytes, &t, &host),
            Err(WorkerPolicyError::Refused)
        );
    }
    let mut checked = verify_worker_policy(&bytes, &t, &host).unwrap();
    let oversized = vec![b' '; MAX_SIGNED_WORKER_POLICY_BYTES + 1];
    assert_eq!(
        checked.revalidate(&oversized, &t, &host),
        Err(WorkerPolicyError::Document)
    );
    assert_eq!(
        checked.revalidate(&bytes, &t, &host),
        Err(WorkerPolicyError::Refused)
    );
}

#[test]
fn worker_policy_seeded_valid_resigned_mutations_keep_original_pin() {
    let p = policy();
    let t = trust(&p);
    let host = context(&p);
    let mut seed = 0x7512026_u32;
    eprintln!("worker-policy seed=0x{seed:x}; mutations=256");
    for i in 0..256 {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let mut changed = p.clone();
        match i % 4 {
            0 => changed.uid = 2000 + seed % 1000,
            1 => changed.gid = 4000 + seed % 1000,
            2 => changed.passwd_sha256 = digest(&seed.to_le_bytes()),
            _ => changed.group_sha256 = digest(&seed.to_le_bytes()),
        }
        let bytes = envelope(&changed, &key(), WORKER_POLICY_DOMAIN);
        verify_worker_policy(&bytes, &trust(&changed), &host).unwrap();
        assert_eq!(error(&bytes, &t, &host), WorkerPolicyError::Pin);
    }
}

#[test]
fn worker_policy_concurrent_guards_share_no_refusal_or_authority() {
    let threads: Vec<_> = (0..4)
        .map(|index| {
            std::thread::spawn(move || {
                let p = policy();
                let t = trust(&p);
                let host = context(&p);
                let bytes = envelope(&p, &key(), WORKER_POLICY_DOMAIN);
                let mut checked = verify_worker_policy(&bytes, &t, &host).unwrap();
                if index == 0 {
                    assert_eq!(
                        checked.revalidate(b"", &t, &host),
                        Err(WorkerPolicyError::Drift)
                    );
                } else {
                    checked.revalidate(&bytes, &t, &host).unwrap();
                    assert!(!checked.is_refused());
                }
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
}
