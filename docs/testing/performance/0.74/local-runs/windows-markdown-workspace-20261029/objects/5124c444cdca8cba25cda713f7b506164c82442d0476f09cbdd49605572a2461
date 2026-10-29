use hydracache_long_run_supervisor_074::diagnostic_start_consent::*;

use ed25519_dalek::{Signer, SigningKey};
use serde_json::{json, Value};

fn fixture() -> StartConsentBody {
    serde_json::from_value(json!({
        "schema_version":BODY_SCHEMA,"action":"start-cell-one",
        "consent_request_id":"1".repeat(64),"reservation_request_id":"2".repeat(64),
        "controller_attempt_sha256":"3".repeat(64),"collector_reserved_sha256":"4".repeat(64),
        "reserved_state_sha256":"5".repeat(64),"challenge_nonce_sha256":"6".repeat(64),
        "authenticated_response_sha256":"7".repeat(64),"worker_policy_sha256":"8".repeat(64),
        "worker_policy_epoch":1,"collector_unit_sha256":"9".repeat(64),
        "controller":{"repository_id":1,"run_id":2,"run_attempt":3,"actor_id":4},
        "source_commit":hydracache_long_run_supervisor_074::diagnostic_lease::SOURCE_COMMIT,
        "preset":hydracache_long_run_supervisor_074::diagnostic_ipc::PRESET,
        "starting_state":{
            "identity":{"lease_id":"a".repeat(64),"boot_id":"00000000-0000-4000-8000-000000000074",
                "binary_sha256":"b".repeat(64),"build_provenance_sha256":"c".repeat(64)},
            "revision":2,"stage":"starting","completed_cells":0,
            "reserved_monotonic_ns":1,"controller_monotonic_ns":2,"last_observed_monotonic_ns":3,
            "cell_started_monotonic_ns":3,"cgroup_inode":null,"reason":null,
            "cleanup_confirmed":false,"promotable":false,"admission_allowed":false
        },
        "issued_at_unix_seconds":100,"expires_at_unix_seconds":160
    }))
    .unwrap()
}
fn line(value: &impl serde::Serialize) -> Vec<u8> {
    let mut b = serde_json::to_vec(&serde_json::to_value(value).unwrap()).unwrap();
    b.push(b'\n');
    b
}
fn document(body: StartConsentBody, key: &SigningKey) -> SignedStartConsent {
    let signature_hex = hex(&key
        .sign(&start_consent_signing_message(&body).unwrap())
        .to_bytes());
    SignedStartConsent {
        schema_version: ENVELOPE_SCHEMA.into(),
        consent: body,
        signature_hex,
    }
}
fn hex(raw: &[u8]) -> String {
    raw.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn consent_roundtrip_signature_and_repeat_inspection_are_data_only() {
    let key = SigningKey::from_bytes(&[74; 32]);
    let doc = document(fixture(), &key);
    let raw = line(&doc);
    assert!(raw.len() < MAX_ENVELOPE_BYTES);
    let message = start_consent_signing_message(&doc.consent).unwrap();
    assert!(message.starts_with(&[SIGNATURE_DOMAIN, b"\0"].concat()));
    assert_eq!(&message[SIGNATURE_DOMAIN.len() + 1..], line(&doc.consent));
    for _ in 0..2 {
        assert_eq!(
            line(&inspect_start_consent_signature(&raw, &key.verifying_key()).unwrap()),
            line(&doc.consent)
        );
    }
    assert!(matches!(
        inspect_start_consent_signature(&raw, &SigningKey::from_bytes(&[75; 32]).verifying_key()),
        Err(StartConsentError::Signature)
    ));
    // Past/future relative to this machine is NOT checked by this byte inspector.
    let mut old = fixture();
    old.issued_at_unix_seconds = 1;
    old.expires_at_unix_seconds = 2;
    inspect_start_consent_signature(&line(&document(old, &key)), &key.verifying_key()).unwrap();
    println!("start_consent supplied_key_only=true repeated_inspection=true freshness_or_replay_authority=false");
}
#[test]
fn consent_domains_and_old_messages_cannot_substitute() {
    let key = SigningKey::from_bytes(&[74; 32]);
    let body = fixture();
    for domain in [
        hydracache_long_run_supervisor_074::diagnostic_ipc::SIGNATURE_DOMAIN,
        b"hydracache-diagnostic-collector-intent-074-v1".as_slice(),
        b"hydracache-diagnostic-intent-074-v1".as_slice(),
        b"hydracache-diagnostic-collector-progress-challenge-074-v1".as_slice(),
    ] {
        assert_ne!(domain, SIGNATURE_DOMAIN);
        let mut doc = document(body.clone(), &key);
        let message = [domain, b"\0", line(&body).as_slice()].concat();
        doc.signature_hex = hex(&key.sign(&message).to_bytes());
        assert!(matches!(
            inspect_start_consent_signature(&line(&doc), &key.verifying_key()),
            Err(StartConsentError::Signature)
        ));
    }
    for schema in ["diagnostic-signed-collector-intent-074-v1", "future"] {
        let mut doc = document(body.clone(), &key);
        doc.schema_version = schema.into();
        assert!(matches!(
            inspect_start_consent_signature(&line(&doc), &key.verifying_key()),
            Err(StartConsentError::Document)
        ));
    }
    assert!(inspect_start_consent_signature(
        b"{\"request\":{},\"signature_hex\":\"\"}\n",
        &key.verifying_key()
    )
    .is_err());
    println!("start_consent domains=4 legacy_interpretation=false");
}
fn paths(value: &Value, prefix: Vec<String>, out: &mut Vec<Vec<String>>) {
    if let Some(map) = value.as_object() {
        for (k, v) in map {
            let mut p = prefix.clone();
            p.push(k.clone());
            paths(v, p, out);
        }
    } else {
        out.push(prefix);
    }
}
fn at<'a>(value: &'a mut Value, path: &[String]) -> &'a mut Value {
    let mut v = value;
    for p in path {
        v = &mut v[p];
    }
    v
}
#[test]
fn consent_seeded_all_signed_leaves_refuse_mutation() {
    let key = SigningKey::from_bytes(&[74; 32]);
    let original = serde_json::to_value(document(fixture(), &key)).unwrap();
    let mut leaves = Vec::new();
    paths(&original["consent"], vec![], &mut leaves);
    let mut order: Vec<_> = (0..leaves.len()).collect();
    let mut seed = 0x0740c0a2_u64;
    for i in (1..order.len()).rev() {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        order.swap(i, seed as usize % (i + 1));
    }
    let expected = [
        "schema_version",
        "action",
        "consent_request_id",
        "reservation_request_id",
        "controller_attempt_sha256",
        "collector_reserved_sha256",
        "reserved_state_sha256",
        "challenge_nonce_sha256",
        "authenticated_response_sha256",
        "worker_policy_sha256",
        "worker_policy_epoch",
        "collector_unit_sha256",
        "controller",
        "source_commit",
        "preset",
        "starting_state",
        "issued_at_unix_seconds",
        "expires_at_unix_seconds",
    ];
    assert_eq!(
        original["consent"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>(),
        expected.into_iter().collect()
    );
    assert_eq!(leaves.len(), 36);
    for i in order {
        let mut doc = original.clone();
        let v = at(&mut doc["consent"], &leaves[i]);
        *v = match v {
            Value::String(s) => {
                let mut b = s.as_bytes().to_vec();
                b[0] = if b[0] == b'a' { b'b' } else { b'a' };
                Value::String(String::from_utf8(b).unwrap())
            }
            Value::Number(n) => json!(n.as_u64().unwrap() + 1),
            Value::Bool(b) => json!(!*b),
            Value::Null => json!(1),
            _ => panic!("no array leaves"),
        };
        assert!(
            inspect_start_consent_signature(&line(&doc), &key.verifying_key()).is_err(),
            "{:?}",
            leaves[i]
        );
    }
    println!(
        "start_consent seed=0x0740c0a2 signed_leaf_mutations={} no_unsigned_binding=true",
        leaves.len()
    );
}
#[test]
fn consent_first_cell_time_epoch_and_integer_bounds_are_strict() {
    let key = SigningKey::from_bytes(&[74; 32]);
    let base = serde_json::to_value(document(fixture(), &key)).unwrap();
    let cases: Vec<(&[&str], Value)> = vec![
        (&["worker_policy_epoch"], json!(0)),
        (&["worker_policy_epoch"], json!(-1)),
        (&["issued_at_unix_seconds"], json!(0)),
        (&["expires_at_unix_seconds"], json!(100)),
        (&["expires_at_unix_seconds"], json!(99)),
        (&["expires_at_unix_seconds"], json!(161)),
        (&["action"], json!("start-cell-two")),
        (&["schema_version"], json!("future")),
        (&["consent_request_id"], json!("2".repeat(64))),
        (&["worker_policy_sha256"], json!("G".repeat(64))),
        (&["source_commit"], json!("0".repeat(64))),
        (&["preset"], json!("other")),
        (&["controller", "repository_id"], json!(0)),
        (&["controller", "run_id"], json!(0)),
        (&["controller", "run_attempt"], json!(0)),
        (&["controller", "actor_id"], json!(0)),
        (&["starting_state", "revision"], json!(3)),
        (&["starting_state", "stage"], json!("running")),
        (&["starting_state", "completed_cells"], json!(1)),
        (&["starting_state", "reserved_monotonic_ns"], json!(0)),
        (&["starting_state", "controller_monotonic_ns"], json!(1)),
        (&["starting_state", "last_observed_monotonic_ns"], json!(1)),
        (
            &["starting_state", "cell_started_monotonic_ns"],
            Value::Null,
        ),
        (&["starting_state", "cell_started_monotonic_ns"], json!(2)),
        (&["starting_state", "cgroup_inode"], json!(1)),
        (&["starting_state", "reason"], json!("operator_cancelled")),
        (&["starting_state", "cleanup_confirmed"], json!(true)),
        (&["starting_state", "promotable"], json!(true)),
        (&["starting_state", "admission_allowed"], json!(true)),
        (&["starting_state", "identity", "boot_id"], json!("bad")),
    ];
    for (path, value) in &cases {
        let mut doc = base.clone();
        *at(
            &mut doc["consent"],
            &path.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        ) = value.clone();
        assert!(
            matches!(
                inspect_start_consent_signature(&line(&doc), &key.verifying_key()),
                Err(StartConsentError::Document)
            ),
            "{path:?}"
        );
    }
    for (issued, expires, ok) in [
        (1, 2, true),
        (100, 160, true),
        (100, 161, false),
        (u64::MAX - 60, u64::MAX, true),
        (u64::MAX, u64::MAX, false),
    ] {
        let mut b = fixture();
        b.issued_at_unix_seconds = issued;
        b.expires_at_unix_seconds = expires;
        assert_eq!(start_consent_signing_message(&b).is_ok(), ok);
    }
    for (now, ok) in [
        (2 + 10_000_000_000 - 1, true),
        (2 + 10_000_000_000, false),
        (u64::MAX, false),
    ] {
        let mut b = fixture();
        b.starting_state.last_observed_monotonic_ns = now;
        b.starting_state.cell_started_monotonic_ns = Some(now);
        assert_eq!(start_consent_signing_message(&b).is_ok(), ok);
    }
    let mut b = fixture();
    b.starting_state.controller_monotonic_ns = 300_000_000_000;
    b.starting_state.last_observed_monotonic_ns = 300_000_000_001;
    b.starting_state.cell_started_monotonic_ns = Some(300_000_000_001);
    assert!(start_consent_signing_message(&b).is_err());
    println!(
        "start_consent grammar_cases={} exclusive_windows=true no_observed_clock=true",
        cases.len()
    );
}
#[test]
fn consent_canonical_unknown_duplicate_partial_and_budget_refusals() {
    let key = SigningKey::from_bytes(&[74; 32]);
    let doc = document(fixture(), &key);
    let raw = line(&doc);
    for bad in [
        vec![],
        b"{}".to_vec(),
        raw[..raw.len() - 1].to_vec(),
        [raw.as_slice(), b"\n"].concat(),
        serde_json::to_vec_pretty(&doc).unwrap(),
        vec![b' '; MAX_ENVELOPE_BYTES],
        vec![b' '; MAX_ENVELOPE_BYTES + 1],
    ] {
        assert!(matches!(
            inspect_start_consent_signature(&bad, &key.verifying_key()),
            Err(StartConsentError::Document)
        ));
    }
    for target in [
        vec![],
        vec!["consent"],
        vec!["consent", "starting_state"],
        vec!["consent", "controller"],
        vec!["consent", "starting_state", "identity"],
    ] {
        let mut v = serde_json::to_value(&doc).unwrap();
        at(
            &mut v,
            &target.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        )
        .as_object_mut()
        .unwrap()
        .insert("unknown".into(), json!(true));
        assert!(matches!(
            inspect_start_consent_signature(&line(&v), &key.verifying_key()),
            Err(StartConsentError::Document)
        ));
    }
    let text = String::from_utf8(raw.clone()).unwrap();
    for duplicate in [
        text.replacen(
            '{',
            &format!("{{\"schema_version\":\"{ENVELOPE_SCHEMA}\","),
            1,
        ),
        text.replacen(
            "\"consent\":{",
            "\"consent\":{\"worker_policy_epoch\":1,",
            1,
        ),
        text.replacen("\"controller\":{", "\"controller\":{\"actor_id\":4,", 1),
        text.replacen(
            "\"starting_state\":{",
            "\"starting_state\":{\"revision\":2,",
            1,
        ),
    ] {
        assert!(
            inspect_start_consent_signature(duplicate.as_bytes(), &key.verifying_key()).is_err()
        );
    }
    for signature in [
        "G".repeat(128),
        "0".repeat(127),
        "0".repeat(129),
        "0".repeat(128),
    ] {
        let mut doc = doc.clone();
        doc.signature_hex = signature;
        assert!(inspect_start_consent_signature(&line(&doc), &key.verifying_key()).is_err());
    }
    let mut huge = serde_json::to_value(&doc).unwrap();
    huge["consent"]["action"] = json!("x".repeat(MAX_BODY_BYTES + 1));
    assert!(inspect_start_consent_signature(&line(&huge), &key.verifying_key()).is_err());
    let bad_utf8 = [raw.as_slice(), &[255]].concat();
    assert!(inspect_start_consent_signature(&bad_utf8, &key.verifying_key()).is_err());
    println!("start_consent bounded_canonical=true unknown_duplicate_partial_refused=true");
}
#[test]
fn consent_independent_concurrent_signature_checks_share_no_authority() {
    let mut threads = Vec::new();
    for id in 1..=4u8 {
        threads.push(std::thread::spawn(move || {
            let key = SigningKey::from_bytes(&[id; 32]);
            let mut b = fixture();
            b.controller.run_id = u64::from(id);
            let raw = line(&document(b, &key));
            let inspected = inspect_start_consent_signature(&raw, &key.verifying_key()).unwrap();
            assert_eq!(inspected.controller.run_id, u64::from(id));
            assert!(inspect_start_consent_signature(
                &raw,
                &SigningKey::from_bytes(&[id + 10; 32]).verifying_key()
            )
            .is_err());
        }));
    }
    for t in threads {
        t.join().unwrap();
    }
    println!("start_consent independent_threads=4 runtime_authority=false");
}

#[test]
fn consent_contract_is_not_heartbeat_or_legacy_intent() {
    assert_eq!(BODY_SCHEMA, "diagnostic-collector-start-consent-074-v1");
    assert_eq!(
        ENVELOPE_SCHEMA,
        "diagnostic-signed-collector-start-consent-074-v1"
    );
    assert_ne!(
        SIGNATURE_DOMAIN,
        hydracache_long_run_supervisor_074::diagnostic_ipc::SIGNATURE_DOMAIN
    );
    assert_eq!(MAX_BODY_BYTES, 4096);
    assert_eq!(MAX_ENVELOPE_BYTES, 8192);
}
