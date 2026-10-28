use super::*;
use crate::diagnostic_ipc::{DiagnosticRequest, PRESET};
use crate::diagnostic_lease::{DiagnosticStage, SOURCE_COMMIT};
use crate::diagnostic_start_consent::{StartConsentBody, BODY_SCHEMA};
use crate::diagnostic_worker_policy::intent_projection::collector::{
    project_collector_reserved_state, COLLECTOR_INTENT_SCHEMA, SIGNED_COLLECTOR_INTENT_SCHEMA,
};
use crate::diagnostic_worker_policy::intent_projection::IntentProjectionBody;
use serde_json::{json, Value};

fn fixture() -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    // Synthetic signed claims, no clock, enrolled key, runtime owner or file.
    let request: DiagnosticRequest = serde_json::from_value(json!({
        "schema_version":1,"request_id":"a".repeat(64),"nonce_sha256":"b".repeat(64),
        "operation":"reserve","expected_state_revision":0,
        "identity":{"lease_id":"c".repeat(64),"boot_id":"00000000-0000-4000-8000-000000000074", "binary_sha256":"d".repeat(64),"build_provenance_sha256":"e".repeat(64)},
        "source_commit":SOURCE_COMMIT,"preset":PRESET,
        "controller":{"repository_id":1,"run_id":2,"run_attempt":1,"actor_id":3},
        "issued_at_unix_seconds":100,"expires_at_unix_seconds":160
    })).unwrap();
    let request_record = RequestRecord {
        schema_version: super::super::super::super::super::CONTROLLER_ATTEMPT_SCHEMA.into(),
        request_sha256: sha256_hex(&canonical_json(&request).unwrap()),
        request: request.clone(),
        token_sha256: "0".repeat(64),
        builder_policy_sha256: "1".repeat(64),
        build_receipt_sha256: request.identity.build_provenance_sha256.clone(),
        controller_trust_sha256: "2".repeat(64),
        oidc_trust_sha256: "3".repeat(64),
        worker_trust_sha256: "4".repeat(64),
        worker_policy_sha256: "f".repeat(64),
        policy_epoch: 1,
        authority_revision: 1,
    };
    let request_raw = line(&request_record).unwrap();
    let reserved = DiagnosticState {
        identity: request.identity.clone(),
        revision: 1,
        stage: DiagnosticStage::Reserved,
        completed_cells: 0,
        reserved_monotonic_ns: 1_000_000_000,
        last_observed_monotonic_ns: 1_000_000_000,
        controller_monotonic_ns: 1_000_000_000,
        cell_started_monotonic_ns: None,
        cgroup_inode: None,
        reason: None,
        cleanup_confirmed: true,
        promotable: false,
        admission_allowed: false,
    };
    let projection = project_collector_reserved_state(&reserved).unwrap();
    let signed = super::super::SignedCollectorIntentProjection {
        schema_version: SIGNED_COLLECTOR_INTENT_SCHEMA.into(),
        intent: IntentProjectionBody {
            schema_version: COLLECTOR_INTENT_SCHEMA.into(),
            request_id: request.request_id.clone(),
            nonce_sha256: request.nonce_sha256.clone(),
            controller: request.controller.clone(),
            issued_at_unix_seconds: 100,
            expires_at_unix_seconds: 160,
            state_sha256: projection.state_sha256.clone(),
            worker_policy_sha256: request_record.worker_policy_sha256.clone(),
            worker_policy_epoch: 1,
            boot_id: reserved.identity.boot_id.clone(),
            unit_spec_sha256: projection.unit_spec_sha256.clone(),
        },
        signature_hex: "0".repeat(128),
    };
    let reserved_raw =
        super::super::encode(&request_raw, &reserved, &line(&signed).unwrap()).unwrap();
    let reserved_state_sha256 = sha256_hex(&canonical_json(&reserved).unwrap());
    let mut state = reserved;
    state.stage = DiagnosticStage::Starting;
    state.revision = 2;
    state.last_observed_monotonic_ns += 1;
    state.controller_monotonic_ns += 1;
    state.cell_started_monotonic_ns = Some(state.last_observed_monotonic_ns);
    state.cleanup_confirmed = false;
    let consent = SignedStartConsent {
        schema_version: ENVELOPE_SCHEMA.into(),
        signature_hex: "0".repeat(128),
        consent: StartConsentBody {
            schema_version: BODY_SCHEMA.into(),
            action: "start-cell-one".into(),
            consent_request_id: "5".repeat(64),
            reservation_request_id: request.request_id,
            controller_attempt_sha256: sha256_hex(&request_raw),
            collector_reserved_sha256: sha256_hex(&reserved_raw),
            reserved_state_sha256,
            challenge_nonce_sha256: "6".repeat(64),
            authenticated_response_sha256: "7".repeat(64),
            worker_policy_sha256: request_record.worker_policy_sha256,
            worker_policy_epoch: 1,
            collector_unit_sha256: projection.unit_spec_sha256,
            controller: request.controller,
            source_commit: request.source_commit,
            preset: request.preset,
            starting_state: state,
            issued_at_unix_seconds: 101,
            expires_at_unix_seconds: 161,
        },
    };
    (request_raw, reserved_raw, line(&consent).unwrap())
}
#[test]
fn collector_starting_record_roundtrip_is_not_signature_or_runtime_authority() {
    let (request, reserved, consent) = fixture();
    let raw = encode_collector_starting_record(&request, &reserved, &consent).unwrap();
    assert!(raw.ends_with(b"\n") && raw.len() <= MAX_STARTING_BYTES);
    for _ in 0..2 {
        validate_retained_collector_starting(&raw, &request, &reserved).unwrap();
    }
    let record: Record = serde_json::from_slice(&raw).unwrap();
    assert_eq!(consent_line(&record.signed_consent).unwrap(), consent);
    assert_eq!(record.signed_consent_sha256, sha256_hex(&consent));
    assert!(
        crate::diagnostic_start_consent::inspect_start_consent_signature(
            &consent,
            &ed25519_dalek::SigningKey::from_bytes(&[74; 32]).verifying_key(),
        )
        .is_err()
    );
    // No challenge/Heartbeat originals are available to a byte inspector.
    let mut other: SignedStartConsent = serde_json::from_slice(&consent).unwrap();
    other.consent.challenge_nonce_sha256 = "8".repeat(64);
    other.signature_hex = "1".repeat(128);
    encode_collector_starting_record(&request, &reserved, &line(&other).unwrap()).unwrap();
    println!(
        "collector_starting_record signature_shape_only=true inspected_bytes_cannot_start=true"
    );
}
#[test]
fn collector_starting_record_all_prefixes_and_noncanonical_forms_refuse() {
    let (request, reserved, consent) = fixture();
    let raw = encode_collector_starting_record(&request, &reserved, &consent).unwrap();
    for end in 0..raw.len() {
        assert!(
            validate_retained_collector_starting(&raw[..end], &request, &reserved).is_err(),
            "prefix={end}"
        );
    }
    for bad in [
        [raw.as_slice(), b"\n"].concat(),
        [b" ", raw.as_slice()].concat(),
        [&raw[..raw.len() - 1], b"\r\n".as_slice()].concat(),
        vec![b'x'; MAX_STARTING_BYTES + 1],
        serde_json::to_vec_pretty(&serde_json::from_slice::<Value>(&raw).unwrap()).unwrap(),
    ] {
        assert!(validate_retained_collector_starting(&bad, &request, &reserved).is_err());
    }
    let mut value: Value = serde_json::from_slice(&raw).unwrap();
    value["unknown"] = json!(true);
    assert!(
        validate_retained_collector_starting(&line(&value).unwrap(), &request, &reserved).is_err()
    );
    let duplicate = [
        &raw[..raw.len() - 2],
        b",\"schema_version\":\"future\"}\n".as_slice(),
    ]
    .concat();
    assert!(validate_retained_collector_starting(&duplicate, &request, &reserved).is_err());
    assert!(
        encode_collector_starting_record(&request, &reserved, &consent[..consent.len() - 1])
            .is_err()
    );
    assert!(encode_collector_starting_record(
        &vec![0; MAX_STARTING_BYTES + 1],
        &reserved,
        &consent
    )
    .is_err());
    println!(
        "collector_starting_record strict_prefixes={} noncanonical_and_bounds=true",
        raw.len()
    );
}
fn leaves(value: &Value, prefix: String, paths: &mut Vec<String>) {
    if let Value::Object(map) = value {
        for (key, value) in map {
            leaves(value, format!("{prefix}/{key}"), paths);
        }
    } else {
        paths.push(prefix);
    }
}
#[test]
fn collector_starting_record_seeded_all_leaf_mutations_refuse() {
    let (request, reserved, consent) = fixture();
    let raw = encode_collector_starting_record(&request, &reserved, &consent).unwrap();
    let value: Value = serde_json::from_slice(&raw).unwrap();
    let mut paths = Vec::new();
    leaves(&value, String::new(), &mut paths);
    let count = paths.len();
    assert!(count > 50);
    let mut seed = 0x0740_57a1_u64;
    println!("collector_starting_record mutation_seed=0x074057a1 leaves={count}");
    for i in (1..paths.len()).rev() {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        paths.swap(i, seed as usize % (i + 1));
    }
    for path in paths {
        let mut changed = value.clone();
        let v = changed.pointer_mut(&path).unwrap();
        *v = match v {
            Value::String(s) => json!(format!("{s}a")),
            Value::Number(n) => json!(n.as_u64().unwrap() + 1),
            Value::Bool(b) => json!(!*b),
            Value::Null => json!(1),
            _ => unreachable!(),
        };
        assert!(
            validate_retained_collector_starting(&line(&changed).unwrap(), &request, &reserved)
                .is_err(),
            "{path}"
        );
    }
}
#[test]
fn collector_starting_record_consistent_rehash_cannot_substitute_prior_bindings() {
    let (request, reserved, consent) = fixture();
    let original: SignedStartConsent = serde_json::from_slice(&consent).unwrap();
    for kind in 0..9 {
        let mut signed = original.clone();
        let b = &mut signed.consent;
        match kind {
            0 => b.worker_policy_epoch += 1,
            1 => b.worker_policy_sha256 = "9".repeat(64),
            2 => b.controller.actor_id += 1,
            3 => b.reserved_state_sha256 = "9".repeat(64),
            4 => b.collector_unit_sha256 = "9".repeat(64),
            5 => b.reservation_request_id = "9".repeat(64),
            6 => b.starting_state.identity.lease_id = "9".repeat(64),
            7 => b.starting_state.reserved_monotonic_ns -= 1,
            8 => b.collector_reserved_sha256 = "9".repeat(64),
            _ => unreachable!(),
        }
        assert!(
            start_consent_signing_message(b).is_ok(),
            "fixture mutation grammar={kind}"
        );
        assert!(
            encode_collector_starting_record(&request, &reserved, &line(&signed).unwrap()).is_err(),
            "binding={kind}"
        );
    }
    let mut changed_request: RequestRecord = serde_json::from_slice(&request).unwrap();
    changed_request.token_sha256 = "9".repeat(64);
    let changed_request = line(&changed_request).unwrap();
    let mut changed_reserved: super::super::Record = serde_json::from_slice(&reserved).unwrap();
    changed_reserved.controller_attempt_sha256 = sha256_hex(&changed_request);
    let changed_reserved = line(&changed_reserved).unwrap();
    validate_retained_collector_reserved(&changed_reserved, &changed_request).unwrap();
    assert!(
        encode_collector_starting_record(&changed_request, &changed_reserved, &consent).is_err()
    );
    println!("collector_starting_record rehashed_binding_mutations=9 mixed_history_refused=true");
}

#[test]
fn collector_starting_record_future_grammar_and_rehashed_state_refuse() {
    let (request, reserved, consent) = fixture();
    let raw = encode_collector_starting_record(&request, &reserved, &consent).unwrap();
    for kind in 0..7 {
        let mut record: Record = serde_json::from_slice(&raw).unwrap();
        match kind {
            0 => record.schema_version = "future".into(),
            1 => record.signed_consent.schema_version = "future".into(),
            2 => record.signed_consent.signature_hex = "A".repeat(128),
            3 => record.signed_consent.signature_hex = "0".repeat(127),
            4 => record.signed_consent.consent.action = "start-cell-two".into(),
            5 => {
                record.state.identity.lease_id = "9".repeat(64);
                record.signed_consent.consent.starting_state = record.state.clone();
            }
            6 => {
                let reserved: super::super::Record = serde_json::from_slice(&reserved).unwrap();
                // Old Reserved projection hashes LF state bytes; original consent
                // instead binds canonical state without LF. Never conflate them.
                record.signed_consent.consent.reserved_state_sha256 =
                    sha256_hex(&line(&reserved.state).unwrap());
            }
            _ => unreachable!(),
        }
        record.signed_consent_sha256 = sha256_hex(&line(&record.signed_consent).unwrap());
        assert!(
            validate_retained_collector_starting(&line(&record).unwrap(), &request, &reserved)
                .is_err(),
            "rehashed grammar/state={kind}"
        );
    }
    let mut wrong: Record = serde_json::from_slice(&raw).unwrap();
    wrong.state.last_observed_monotonic_ns += 1;
    assert!(
        validate_retained_collector_starting(&line(&wrong).unwrap(), &request, &reserved).is_err()
    );
    assert!(encode_collector_starting_record(
        &request,
        &reserved,
        &vec![0; MAX_ENVELOPE_BYTES + 1]
    )
    .is_err());
    println!("collector_starting_record rehashed_grammar_state_mutations=7 exact_state_copy=true");
}
