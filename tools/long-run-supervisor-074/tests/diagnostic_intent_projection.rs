#![cfg(target_os = "linux")]
//! Synthetic keys/assertions only; no enrollment, durable intent or execution.
use ed25519_dalek::{Signer, SigningKey};
use hydracache_long_run_supervisor_074::diagnostic_ipc::{
    DiagnosticController, DiagnosticPeer, DiagnosticPolicy,
};
use hydracache_long_run_supervisor_074::diagnostic_lease::{
    DiagnosticIdentity, DiagnosticStage, DiagnosticState,
};
use hydracache_long_run_supervisor_074::diagnostic_worker_policy::intent_projection::*;
use hydracache_long_run_supervisor_074::diagnostic_worker_policy::*;
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

#[path = "support/diagnostic_collector_intent.rs"]
mod collector_intent_tests;

#[test]
fn collector_intent_requires_a_distinct_output_projection() {
    use hydracache_long_run_supervisor_074::diagnostic_worker_policy::intent_projection::collector::project_collector_reserved_state;
    let f = Fixture::new(0);
    let old = project_reserved_state(&f.state).unwrap();
    let new = project_collector_reserved_state(&f.state).unwrap();
    assert_eq!(old.state_sha256, new.state_sha256);
    assert_ne!(old.unit_spec_sha256, new.unit_spec_sha256);
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn line(value: &impl Serialize) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(&serde_json::to_value(value).unwrap()).unwrap();
    bytes.push(b'\n');
    bytes
}
fn digest(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}
struct Fixture {
    worker_bytes: Vec<u8>,
    trust: WorkerPolicyTrust,
    host: AssertedWorkerHost,
    state: DiagnosticState,
    controller: DiagnosticPolicy,
    peer: DiagnosticPeer,
    key: SigningKey,
}
impl Fixture {
    fn new(cell: usize) -> Self {
        let host = AssertedWorkerHost {
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
        };
        let policy = WorkerPolicy {
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
            machine_id: host.machine_id.clone(),
            boot_id: host.boot_id.clone(),
            user_namespace: host.user_namespace,
            mount_namespace: host.mount_namespace,
            passwd_sha256: "b".repeat(64),
            group_sha256: "c".repeat(64),
        };
        let worker_key = SigningKey::from_bytes(&[74; 32]);
        let mut message = WORKER_POLICY_DOMAIN.to_vec();
        message.push(0);
        message.extend(line(&policy));
        let worker_bytes = line(&SignedWorkerPolicy {
            schema_version: SIGNED_WORKER_POLICY_SCHEMA.into(),
            policy: policy.clone(),
            signature_hex: hex(&worker_key.sign(&message).to_bytes()),
        });
        let key = SigningKey::from_bytes(&[75; 32]);
        Self {
            worker_bytes,
            trust: WorkerPolicyTrust::new(worker_key.verifying_key(), &digest(&line(&policy)), 7)
                .unwrap(),
            state: DiagnosticState {
                identity: DiagnosticIdentity {
                    lease_id: "d".repeat(64),
                    boot_id: host.boot_id.clone(),
                    binary_sha256: "e".repeat(64),
                    build_provenance_sha256: "f".repeat(64),
                },
                revision: 1 + cell as u64,
                stage: DiagnosticStage::Reserved,
                completed_cells: cell,
                reserved_monotonic_ns: 1_000_000_000,
                last_observed_monotonic_ns: 2_000_000_000,
                controller_monotonic_ns: 2_000_000_000,
                cell_started_monotonic_ns: None,
                cgroup_inode: None,
                reason: None,
                cleanup_confirmed: true,
                promotable: false,
                admission_allowed: false,
            },
            controller: DiagnosticPolicy {
                repository_id: 1_217_101_761,
                actor_ids: vec![42, 43],
                client_uids: vec![900, 901],
                client_gid: 902,
                verifying_key: key.verifying_key(),
            },
            peer: DiagnosticPeer {
                uid: 900,
                gids: vec![902],
            },
            host,
            key,
        }
    }
    fn body(&self) -> IntentProjectionBody {
        let projection = project_reserved_state(&self.state).unwrap();
        IntentProjectionBody {
            schema_version: INTENT_SCHEMA.into(),
            request_id: "1".repeat(64),
            nonce_sha256: "2".repeat(64),
            controller: DiagnosticController {
                repository_id: 1_217_101_761,
                run_id: 9,
                run_attempt: 2,
                actor_id: 42,
            },
            issued_at_unix_seconds: 100,
            expires_at_unix_seconds: 160,
            state_sha256: projection.state_sha256,
            unit_spec_sha256: projection.unit_spec_sha256,
            boot_id: self.host.boot_id.clone(),
            worker_policy_sha256: digest(&line(
                &serde_json::from_slice::<SignedWorkerPolicy>(&self.worker_bytes)
                    .unwrap()
                    .policy,
            )),
            worker_policy_epoch: 7,
        }
    }
    fn signed(&self, body: &IntentProjectionBody) -> Vec<u8> {
        line(&SignedIntentProjection {
            schema_version: SIGNED_INTENT_SCHEMA.into(),
            intent: body.clone(),
            signature_hex: hex(&self
                .key
                .sign(&intent_signing_message(body).unwrap())
                .to_bytes()),
        })
    }
    fn check(&self, now: u64) -> IntentCheck<'_> {
        IntentCheck {
            state: &self.state,
            peer: &self.peer,
            controller_policy: &self.controller,
            now_unix_seconds: now,
            worker_bytes: &self.worker_bytes,
            worker_trust: &self.trust,
            worker_host: &self.host,
        }
    }
    fn worker(&self) -> CheckedWorkerPolicy {
        verify_worker_policy(&self.worker_bytes, &self.trust, &self.host).unwrap()
    }
    fn reject(&self, bytes: &[u8], now: u64) -> IntentProjectionError {
        let mut worker = self.worker();
        let error = verify_intent_projection(&mut worker, bytes, &self.check(now))
            .err()
            .unwrap();
        assert!(
            !worker.is_refused(),
            "bad intent must not revoke a healthy worker"
        );
        error
    }
}

#[test]
fn four_cells_bind_exact_projection_without_execution() {
    let mut digests = std::collections::BTreeSet::new();
    for cell in 0..4 {
        let f = Fixture::new(cell);
        let bytes = f.signed(&f.body());
        let mut worker = f.worker();
        let mut guard = verify_intent_projection(&mut worker, &bytes, &f.check(100)).unwrap();
        assert_eq!(guard.intent_sha256(), digest(&bytes));
        for now in [100, 101, 159] {
            guard.revalidate(&bytes, &f.check(now)).unwrap();
        }
        assert!(!guard.is_refused());
        assert!(digests.insert(project_reserved_state(&f.state).unwrap().unit_spec_sha256));
    }
}

#[test]
fn signed_binding_mutations_are_not_authority() {
    let f = Fixture::new(0);
    let original = serde_json::to_value(f.body()).unwrap();
    for (field, value) in [
        ("state_sha256", json!("3".repeat(64))),
        ("unit_spec_sha256", json!("4".repeat(64))),
        ("worker_policy_sha256", json!("5".repeat(64))),
        ("worker_policy_epoch", json!(8)),
        ("boot_id", json!("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee")),
    ] {
        let mut body = original.clone();
        body[field] = value;
        let body: IntentProjectionBody = serde_json::from_value(body).unwrap();
        assert_eq!(
            f.reject(&f.signed(&body), 100),
            IntentProjectionError::Binding
        );
    }
}

#[test]
fn malformed_noncanonical_unknown_duplicate_and_oversize_refuse() {
    let f = Fixture::new(0);
    let good = f.signed(&f.body());
    let mut unknown: Value = serde_json::from_slice(&good).unwrap();
    unknown["future"] = json!(1);
    let mut future = unknown.clone();
    future.as_object_mut().unwrap().remove("future");
    future["schema_version"] = json!("diagnostic-signed-intent-projection-074-v2");
    let duplicate = format!(
        "{{\"schema_version\":\"x\",{}",
        std::str::from_utf8(&good[1..]).unwrap()
    )
    .into_bytes();
    for bad in [
        Vec::new(),
        vec![b' '; MAX_SIGNED_INTENT_BYTES + 1],
        line(&unknown),
        line(&future),
        duplicate,
        serde_json::to_vec_pretty(&serde_json::from_slice::<Value>(&good).unwrap()).unwrap(),
        good[..good.len() - 1].to_vec(),
    ] {
        assert_eq!(f.reject(&bad, 100), IntentProjectionError::Document);
    }
}

#[test]
fn signature_domain_key_and_lowercase_encoding_are_strict() {
    let f = Fixture::new(0);
    let body = f.body();
    for domain in [
        WORKER_POLICY_DOMAIN,
        b"hydracache-diagnostic-request-074-v1",
    ] {
        let mut msg = domain.to_vec();
        msg.push(0);
        msg.extend(line(&body));
        let bad = line(&SignedIntentProjection {
            schema_version: SIGNED_INTENT_SCHEMA.into(),
            intent: body.clone(),
            signature_hex: hex(&f.key.sign(&msg).to_bytes()),
        });
        assert_eq!(f.reject(&bad, 100), IntentProjectionError::Signature);
    }
    let mut bad: SignedIntentProjection = serde_json::from_slice(&f.signed(&body)).unwrap();
    bad.signature_hex = "A".repeat(128);
    assert_eq!(f.reject(&line(&bad), 100), IntentProjectionError::Document);
    bad.signature_hex = hex(&SigningKey::from_bytes(&[76; 32])
        .sign(&intent_signing_message(&body).unwrap())
        .to_bytes());
    assert_eq!(f.reject(&line(&bad), 100), IntentProjectionError::Signature);
}

#[test]
fn window_is_positive_bounded_and_expiry_exclusive() {
    let f = Fixture::new(0);
    let body = f.body();
    let bytes = f.signed(&body);
    for now in [99, 160, u64::MAX] {
        assert_eq!(f.reject(&bytes, now), IntentProjectionError::Time);
    }
    for (issued, expires) in [(0, 60), (100, 100), (101, 100), (100, 161)] {
        let mut bad = body.clone();
        bad.issued_at_unix_seconds = issued;
        bad.expires_at_unix_seconds = expires;
        assert_eq!(f.reject(&f.signed(&bad), 100), IntentProjectionError::Time);
    }
}

#[test]
fn controller_and_peer_allowlists_cannot_be_supplied_by_the_envelope() {
    for mutation in 0..8 {
        let mut f = Fixture::new(0);
        let bytes = f.signed(&f.body());
        match mutation {
            0 => f.peer.uid = 0,
            1 => f.peer.gids.clear(),
            2 => f.peer.gids = vec![902, 902],
            3 => f.controller.actor_ids = vec![43],
            4 => f.controller.client_uids = vec![901],
            5 => f.controller.actor_ids = vec![42, 42],
            6 => f.controller.client_gid = 0,
            _ => {
                f.controller.verifying_key =
                    ed25519_dalek::VerifyingKey::from_bytes(&[0; 32]).unwrap()
            }
        }
        assert_eq!(f.reject(&bytes, 100), IntentProjectionError::Principal);
    }
}

#[test]
fn state_projection_rejects_interrupted_expired_and_admission_states() {
    for mutation in 0..8 {
        let mut f = Fixture::new(0);
        match mutation {
            0 => f.state.stage = DiagnosticStage::Starting,
            1 => f.state.stage = DiagnosticStage::Running,
            2 => f.state.stage = DiagnosticStage::Stopping,
            3 => f.state.stage = DiagnosticStage::Terminal,
            4 => f.state.admission_allowed = true,
            5 => f.state.completed_cells = 4,
            6 => f.state.last_observed_monotonic_ns = 12_000_000_000,
            _ => {
                f.state.last_observed_monotonic_ns = 301_000_000_000;
                f.state.controller_monotonic_ns = 301_000_000_000;
            }
        }
        assert_eq!(
            project_reserved_state(&f.state).unwrap_err(),
            IntentProjectionError::State
        );
    }
}

#[test]
fn state_hash_includes_every_field_and_unit_hash_changes_with_budget() {
    let f = Fixture::new(0);
    let initial = project_reserved_state(&f.state).unwrap();
    let mut changed = f.state.clone();
    changed.revision += 1;
    assert_ne!(
        initial.state_sha256,
        project_reserved_state(&changed).unwrap().state_sha256
    );
    changed.last_observed_monotonic_ns = 300_000_000_000;
    changed.controller_monotonic_ns = 300_000_000_000;
    assert_ne!(
        initial.unit_spec_sha256,
        project_reserved_state(&changed).unwrap().unit_spec_sha256
    );
}

#[test]
fn checked_intent_refuses_original_state_peer_trust_and_packet_drift_stickily() {
    for mutation in 0..5 {
        let mut f = Fixture::new(0);
        let bytes = f.signed(&f.body());
        let mut worker = f.worker();
        let mut guard = verify_intent_projection(&mut worker, &bytes, &f.check(100)).unwrap();
        let saved = f.state.clone();
        let saved_peer = f.peer.uid;
        match mutation {
            0 => f.state.revision += 1,
            1 => f.peer.uid = 901,
            2 => f.controller.actor_ids.push(44),
            _ => (),
        }
        let altered = if mutation == 3 {
            f.signed(&IntentProjectionBody {
                request_id: "3".repeat(64),
                ..f.body()
            })
        } else {
            bytes.clone()
        };
        assert!(guard
            .revalidate(&altered, &f.check(if mutation == 4 { 160 } else { 100 }))
            .is_err());
        assert!(guard.is_refused());
        f.state = saved;
        f.peer.uid = saved_peer;
        assert_eq!(
            guard.revalidate(&bytes, &f.check(100)),
            Err(IntentProjectionError::Refused)
        );
        drop(guard);
        assert!(!worker.is_refused());
    }
}

#[test]
fn original_worker_refusal_survives_intent_drop_and_restoration() {
    let mut f = Fixture::new(0);
    let bytes = f.signed(&f.body());
    let mut worker = f.worker();
    let mut guard = verify_intent_projection(&mut worker, &bytes, &f.check(100)).unwrap();
    let original = f.host.clone();
    f.host.mount_namespace.inode += 1;
    assert!(matches!(
        guard.revalidate(&bytes, &f.check(100)),
        Err(IntentProjectionError::Worker(_))
    ));
    drop(guard);
    f.host = original;
    assert!(worker.is_refused());
    assert!(matches!(
        verify_intent_projection(&mut worker, &bytes, &f.check(100)),
        Err(IntentProjectionError::Worker(WorkerPolicyError::Refused))
    ));
}

#[test]
fn boot_change_never_rebinds_an_original_worker() {
    let mut f = Fixture::new(0);
    f.state.identity.boot_id = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".into();
    let mut body = f.body();
    body.boot_id = f.state.identity.boot_id.clone();
    assert_eq!(
        f.reject(&f.signed(&body), 100),
        IntentProjectionError::Binding
    );
}

#[test]
fn independent_concurrent_checks_are_not_a_replay_ledger() {
    let handles: Vec<_> = (0..4)
        .map(|cell| {
            std::thread::spawn(move || {
                let f = Fixture::new(cell);
                let bytes = f.signed(&f.body());
                for _ in 0..3 {
                    let mut worker = f.worker();
                    verify_intent_projection(&mut worker, &bytes, &f.check(100)).unwrap();
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn authenticated_worker_trust_drift_revokes_original_not_just_wrapper() {
    let mut f = Fixture::new(0);
    let bytes = f.signed(&f.body());
    let mut worker = f.worker();
    let mut guard = verify_intent_projection(&mut worker, &bytes, &f.check(100)).unwrap();
    f.trust = WorkerPolicyTrust::new(
        SigningKey::from_bytes(&[74; 32]).verifying_key(),
        &"a".repeat(64),
        8,
    )
    .unwrap();
    assert_eq!(
        guard.revalidate(&bytes, &f.check(100)),
        Err(IntentProjectionError::Worker(WorkerPolicyError::Revoked))
    );
    drop(guard);
    assert!(worker.is_refused());
}

#[test]
fn controller_fields_and_bounded_configuration_fail_closed() {
    let f = Fixture::new(0);
    let original = serde_json::to_value(f.body()).unwrap();
    for (field, value) in [
        ("run_id", json!(0)),
        ("run_attempt", json!(0)),
        ("actor_id", json!(44)),
        ("repository_id", json!(1)),
    ] {
        let mut body = original.clone();
        body["controller"][field] = value;
        assert_eq!(
            f.reject(&f.signed(&serde_json::from_value(body).unwrap()), 100),
            IntentProjectionError::Principal
        );
    }
    for mutation in 0..5 {
        let mut f = Fixture::new(0);
        let bytes = f.signed(&f.body());
        match mutation {
            0 => f.controller.actor_ids = (1..=33).collect(),
            1 => f.controller.client_uids = (900..933).collect(),
            2 => f.peer.gids = (900..933).collect(),
            3 => f.controller.actor_ids.reverse(),
            _ => f.controller.client_uids.clear(),
        }
        assert_eq!(f.reject(&bytes, 100), IntentProjectionError::Principal);
    }
}

#[test]
fn body_budget_nested_schema_and_hash_encoding_are_strict() {
    let f = Fixture::new(0);
    let bytes = f.signed(&f.body());
    let original: Value = serde_json::from_slice(&bytes).unwrap();
    for (field, value) in [
        (
            "schema_version",
            json!("diagnostic-intent-projection-074-v2"),
        ),
        ("nonce_sha256", json!("A".repeat(64))),
        ("worker_policy_epoch", json!(0)),
        ("request_id", json!("x".repeat(MAX_INTENT_BODY_BYTES))),
    ] {
        let mut changed = original.clone();
        changed["intent"][field] = value;
        let body: IntentProjectionBody = serde_json::from_value(changed["intent"].clone()).unwrap();
        let mut message = INTENT_DOMAIN.to_vec();
        message.push(0);
        message.extend(line(&body));
        changed["signature_hex"] = json!(hex(&f.key.sign(&message).to_bytes()));
        assert_eq!(
            f.reject(&line(&changed), 100),
            IntentProjectionError::Document
        );
    }
    let mut changed = original;
    changed["intent"]["future"] = json!(1);
    assert_eq!(
        f.reject(&line(&changed), 100),
        IntentProjectionError::Document
    );
}

#[test]
fn seeded_state_mutations_bind_full_state_not_only_cell() {
    let f = Fixture::new(0);
    let initial = project_reserved_state(&f.state).unwrap();
    let mut seed = 74_u64;
    for _ in 0..64 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let mut changed = f.state.clone();
        changed.revision += 1 + seed % 4096;
        changed.identity.build_provenance_sha256 = format!("{seed:064x}");
        let projection = project_reserved_state(&changed).unwrap();
        assert_ne!(initial.state_sha256, projection.state_sha256);
        assert_eq!(initial.unit_spec_sha256, projection.unit_spec_sha256);
        let bytes = f.signed(&IntentProjectionBody {
            state_sha256: projection.state_sha256,
            ..f.body()
        });
        assert_eq!(f.reject(&bytes, 100), IntentProjectionError::Binding);
    }
}
