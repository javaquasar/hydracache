use super::*;
use crate::diagnostic_ipc::{DiagnosticController, PRESET};
use crate::diagnostic_lease::{
    DiagnosticIdentity, DiagnosticStage, DiagnosticState, SOURCE_COMMIT,
};
use ed25519_dalek::{Signer, SigningKey};
use serde_json::{json, Value};

fn fixture() -> LaunchBody {
    let consent = StartConsentBody {
        schema_version: crate::diagnostic_start_consent::BODY_SCHEMA.into(),
        action: "start-cell-one".into(),
        consent_request_id: "1".repeat(64),
        reservation_request_id: "2".repeat(64),
        controller_attempt_sha256: "3".repeat(64),
        collector_reserved_sha256: "4".repeat(64),
        reserved_state_sha256: "5".repeat(64),
        challenge_nonce_sha256: "6".repeat(64),
        authenticated_response_sha256: "7".repeat(64),
        worker_policy_sha256: "8".repeat(64),
        worker_policy_epoch: 1,
        collector_unit_sha256: "9".repeat(64),
        controller: DiagnosticController {
            repository_id: 1,
            run_id: 2,
            run_attempt: 1,
            actor_id: 3,
        },
        source_commit: SOURCE_COMMIT.into(),
        preset: PRESET.into(),
        starting_state: DiagnosticState {
            identity: DiagnosticIdentity {
                lease_id: "a".repeat(64),
                boot_id: "00000000-0000-4000-8000-000000000074".into(),
                binary_sha256: "b".repeat(64),
                build_provenance_sha256: "c".repeat(64),
            },
            revision: 2,
            stage: DiagnosticStage::Starting,
            completed_cells: 0,
            reserved_monotonic_ns: 1_000_000_000,
            last_observed_monotonic_ns: 2_000_000_001,
            controller_monotonic_ns: 2_000_000_000,
            cell_started_monotonic_ns: Some(2_000_000_001),
            cgroup_inode: None,
            reason: None,
            cleanup_confirmed: false,
            promotable: false,
            admission_allowed: false,
        },
        issued_at_unix_seconds: 100,
        expires_at_unix_seconds: 160,
    };
    let artifacts = LaunchArtifacts {
        launcher_sha256: "d".repeat(64),
        watchdog_sha256: "e".repeat(64),
        collector_sha256: "f".repeat(64),
    };
    let requirements_sha256 = sha256_hex(&launch_requirements(&consent, &artifacts).unwrap());
    LaunchBody {
        schema_version: BODY_SCHEMA.into(),
        action: "prepare-held-cell-one".into(),
        consent_claims: consent,
        artifacts,
        requirements_sha256,
    }
}
fn key() -> SigningKey {
    SigningKey::from_bytes(&[74; 32])
}
fn signed(body: LaunchBody) -> SignedLaunch {
    let signature = key().sign(&launch_signing_message(&body).unwrap());
    SignedLaunch {
        schema_version: ENVELOPE_SCHEMA.into(),
        launch: body,
        signature_hex: signature
            .to_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
    }
}
fn inspect(raw: &[u8]) -> Result<LaunchBody, LaunchError> {
    inspect_launch_signature(raw, &key().verifying_key())
}
#[test]
fn two_phase_launch_roundtrip_is_expected_bytes_not_start_authority() {
    let body = fixture();
    let projection = launch_requirements(&body.consent_claims, &body.artifacts).unwrap();
    let p: Value = serde_json::from_slice(&projection).unwrap();
    assert_eq!(p["schema_version"], REQUIREMENTS_SCHEMA);
    assert_eq!(
        p["protocol"]["order"][0],
        "independent-watchdog-armed-before-first-delivery"
    );
    assert_eq!(
        p["protocol"]["release"],
        "one-shot-after-original-live-binding-and-complete-frame"
    );
    assert_eq!(
        p["protocol"]["unknown_outcome"],
        "retain-incomplete-attempt-and-fence-no-resend"
    );
    assert_eq!(p["deadlines"]["seconds"], json!([300, 60, 10]));
    assert_eq!(p["deadlines"]["renewal"], false);
    let raw = line(&signed(body.clone())).unwrap();
    let inspected = inspect(&raw).unwrap();
    assert_eq!(line(&inspected).unwrap(), line(&body).unwrap());
    assert_eq!(
        projection,
        launch_requirements(&inspected.consent_claims, &inspected.artifacts).unwrap()
    );
    assert!(raw.len() <= MAX_ENVELOPE_BYTES);
    // Legacy readers cannot interpret a new outer signature as old consent.
    assert!(
        crate::diagnostic_start_consent::inspect_start_consent_signature(
            &raw,
            &key().verifying_key()
        )
        .is_err()
    );
    let legacy = crate::diagnostic_start_consent::SignedStartConsent {
        schema_version: crate::diagnostic_start_consent::ENVELOPE_SCHEMA.into(),
        consent: body.consent_claims,
        signature_hex: "0".repeat(128),
    };
    assert!(inspect(&line(&legacy).unwrap()).is_err());
}
#[test]
fn two_phase_launch_digest_binds_every_consent_claim_and_artifact_role() {
    let original = fixture();
    let old = launch_requirements(&original.consent_claims, &original.artifacts).unwrap();
    let consent = serde_json::to_value(&original.consent_claims).unwrap();
    // Every scalar leaf is either structurally invalid after mutation, or
    // changes the projection. No signed nested claim is projected away.
    fn leaves(value: &Value, path: Vec<String>, out: &mut Vec<Vec<String>>) {
        if let Some(map) = value.as_object() {
            for (k, v) in map {
                let mut p = path.clone();
                p.push(k.clone());
                leaves(v, p, out);
            }
        } else {
            out.push(path);
        }
    }
    let mut paths = vec![];
    leaves(&consent, vec![], &mut paths);
    assert!(paths.len() >= 35);
    for path in paths {
        let mut changed = consent.clone();
        let mut at = &mut changed;
        for part in &path {
            at = &mut at[part];
        }
        *at = match at.clone() {
            Value::String(s) => Value::String(if s.len() == 64 {
                "0".repeat(64)
            } else {
                format!("{s}x")
            }),
            Value::Number(n) => json!(n.as_u64().unwrap() + 1),
            Value::Bool(b) => json!(!b),
            Value::Null => json!(1),
            _ => panic!("unexpected leaf"),
        };
        if let Ok(c) = serde_json::from_value(changed) {
            if let Ok(new) = launch_requirements(&c, &original.artifacts) {
                assert_ne!(old, new, "{path:?}");
            }
        }
    }
    for role in ["launcher_sha256", "watchdog_sha256", "collector_sha256"] {
        let mut v = serde_json::to_value(&original).unwrap();
        v["artifacts"][role] = json!("0".repeat(64));
        let changed: LaunchBody = serde_json::from_value(v).unwrap();
        assert_ne!(
            old,
            launch_requirements(&changed.consent_claims, &changed.artifacts).unwrap()
        );
        assert_eq!(
            launch_signing_message(&changed).unwrap_err(),
            LaunchError::Binding
        );
    }
}
#[test]
fn two_phase_launch_domain_signature_and_digest_refuse_independent_drift() {
    let body = fixture();
    let message = launch_signing_message(&body).unwrap();
    assert!(message.starts_with(SIGNATURE_DOMAIN));
    assert_eq!(message[SIGNATURE_DOMAIN.len()], 0);
    let doc = signed(body.clone());
    let raw = line(&doc).unwrap();
    assert_eq!(
        inspect_launch_signature(&raw, &SigningKey::from_bytes(&[75; 32]).verifying_key())
            .unwrap_err(),
        LaunchError::Signature
    );
    let mut changed = doc.clone();
    changed.launch.consent_claims.worker_policy_epoch += 1;
    changed.launch.requirements_sha256 = sha256_hex(
        &launch_requirements(&changed.launch.consent_claims, &changed.launch.artifacts).unwrap(),
    );
    assert_eq!(
        inspect(&line(&changed).unwrap()).unwrap_err(),
        LaunchError::Signature
    );
    let mut wrong_domain = doc;
    wrong_domain.signature_hex = key()
        .sign(
            &crate::diagnostic_start_consent::start_consent_signing_message(&body.consent_claims)
                .unwrap(),
        )
        .to_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(
        inspect(&line(&wrong_domain).unwrap()).unwrap_err(),
        LaunchError::Signature
    );
    let mut changed = body;
    changed.requirements_sha256 = "0".repeat(64);
    assert_eq!(
        launch_signing_message(&changed).unwrap_err(),
        LaunchError::Binding
    );
}
#[test]
fn two_phase_launch_document_bounds_and_unknown_shapes_refuse() {
    let doc = signed(fixture());
    let raw = line(&doc).unwrap();
    for end in 0..raw.len() {
        assert!(inspect(&raw[..end]).is_err(), "prefix {end}");
    }
    for raw in [
        vec![],
        vec![b' '; MAX_ENVELOPE_BYTES + 1],
        [raw.as_slice(), b"\n"].concat(),
        serde_json::to_vec_pretty(&doc).unwrap(),
    ] {
        assert!(inspect(&raw).is_err());
    }
    for path in [
        vec!["schema_version"],
        vec!["launch", "schema_version"],
        vec!["launch", "action"],
        vec!["launch", "consent_claims", "schema_version"],
    ] {
        let mut v = serde_json::to_value(&doc).unwrap();
        let mut at = &mut v;
        for p in path {
            at = &mut at[p];
        }
        *at = json!("future");
        assert!(inspect(&line(&v).unwrap()).is_err());
    }
    for path in [
        vec![],
        vec!["launch"],
        vec!["launch", "artifacts"],
        vec!["launch", "consent_claims"],
        vec!["launch", "consent_claims", "starting_state"],
    ] {
        let mut v = serde_json::to_value(&doc).unwrap();
        let mut at = &mut v;
        for p in path {
            at = &mut at[p];
        }
        at["future"] = json!(true);
        assert!(inspect(&line(&v).unwrap()).is_err());
    }
    for sig in [
        "A".repeat(128),
        "g".repeat(128),
        "0".repeat(127),
        "0".repeat(129),
    ] {
        let mut d = doc.clone();
        d.signature_hex = sig;
        assert_eq!(
            inspect(&line(&d).unwrap()).unwrap_err(),
            LaunchError::Document
        );
    }
    for role in ["launcher_sha256", "watchdog_sha256", "collector_sha256"] {
        for value in ["A".repeat(64), "0".repeat(63), "x".repeat(65)] {
            let mut v = serde_json::to_value(&doc.launch).unwrap();
            v["artifacts"][role] = json!(value);
            let body: LaunchBody = serde_json::from_value(v).unwrap();
            assert_eq!(
                launch_signing_message(&body).unwrap_err(),
                LaunchError::Document
            );
        }
    }
    let duplicate = String::from_utf8(raw).unwrap().replacen(
        "\"signature_hex\":",
        "\"signature_hex\":\"duplicate\",\"signature_hex\":",
        1,
    );
    assert_eq!(
        inspect(duplicate.as_bytes()).unwrap_err(),
        LaunchError::Document
    );
}
#[test]
fn two_phase_launch_inherits_old_consent_bounds_without_new_clock_or_renewal() {
    for kind in [
        "controller-bound",
        "reservation-bound",
        "cell-origin",
        "epoch",
        "future-state",
        "time-window",
    ] {
        let mut b = fixture();
        let s = &mut b.consent_claims.starting_state;
        match kind {
            "controller-bound" => {
                s.last_observed_monotonic_ns = s.controller_monotonic_ns + 10_000_000_000;
                s.cell_started_monotonic_ns = Some(s.last_observed_monotonic_ns);
            }
            "reservation-bound" => {
                s.last_observed_monotonic_ns = s.reserved_monotonic_ns + 300_000_000_000;
                s.controller_monotonic_ns = s.last_observed_monotonic_ns - 1;
                s.cell_started_monotonic_ns = Some(s.last_observed_monotonic_ns);
            }
            "cell-origin" => s.cell_started_monotonic_ns = Some(s.last_observed_monotonic_ns + 1),
            "epoch" => b.consent_claims.worker_policy_epoch = 0,
            "future-state" => s.stage = DiagnosticStage::Running,
            "time-window" => b.consent_claims.expires_at_unix_seconds += 1,
            _ => unreachable!(),
        }
        assert_eq!(
            launch_requirements(&b.consent_claims, &b.artifacts).unwrap_err(),
            LaunchError::Document,
            "{kind}"
        );
    }
}
