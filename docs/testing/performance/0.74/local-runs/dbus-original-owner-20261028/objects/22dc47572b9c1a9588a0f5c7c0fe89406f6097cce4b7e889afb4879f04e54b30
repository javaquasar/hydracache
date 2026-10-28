#![cfg(target_os = "linux")]
//! Synthetic keys/assertions and owned durable fixtures; no production execution.
use crate::diagnostic_ipc::{DiagnosticController, DiagnosticPeer, DiagnosticPolicy};
use crate::diagnostic_lease::{DiagnosticIdentity, DiagnosticStage, DiagnosticState};
use crate::diagnostic_worker_policy::intent_projection::*;
use crate::diagnostic_worker_policy::*;
use ed25519_dalek::{Signer, SigningKey};
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};

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

use super::*;
use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};
fn stored(cell: usize) -> (tempfile::TempDir, std::path::PathBuf, Fixture) {
    let temp = tempfile::Builder::new()
        .prefix("hydracache-intent-")
        .tempdir_in("/tmp")
        .unwrap();
    fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let root = temp.path().join("campaigns");
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(root.join(".host-execution.lock"), []).unwrap();
    fs::set_permissions(
        root.join(".host-execution.lock"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    let f = Fixture::new(cell);
    persist(&root, &f.state);
    (temp, root, f)
}
fn persist(root: &Path, state: &DiagnosticState) {
    let mut canonical = line(state);
    canonical.pop();
    fs::write(
        root.join("active-diagnostic.json"),
        line(&json!({
            "schema_version": 1, "state": state, "state_sha256": digest(&canonical),
        })),
    )
    .unwrap();
    fs::set_permissions(
        root.join("active-diagnostic.json"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
}
fn bytes(f: &Fixture) -> Vec<u8> {
    let mut body = f.body();
    body.request_id = format!("{:064x}", f.state.completed_cells + 1);
    body.nonce_sha256 = format!("{:064x}", f.state.completed_cells + 100);
    f.signed(&body)
}
#[test]
fn intent_journal_four_cells_reopen_without_second_consumption() {
    let (_temp, root, mut f) = stored(0);
    for cell in 0..4 {
        f.state.completed_cells = cell;
        f.state.revision = 1 + cell as u64;
        f.state.last_observed_monotonic_ns = (2 + cell as u64 * 60) * 1_000_000_000;
        f.state.controller_monotonic_ns = f.state.last_observed_monotonic_ns;
        persist(&root, &f.state);
        let packet = bytes(&f);
        let mut worker = f.worker();
        let mut guard = verify_intent_projection(&mut worker, &packet, &f.check(100)).unwrap();
        let result = retain_fixture_intent(&root, &mut guard, &packet, &f.check(100)).unwrap();
        assert_eq!(result.consumed_cells, cell + 1);
        let result = inspect_fixture_intents(&root, &mut guard, &packet, &f.check(100)).unwrap();
        assert_eq!(result.consumed_cells, cell + 1);
        assert!(matches!(
            retain_fixture_intent(&root, &mut guard, &packet, &f.check(100)),
            Err(IntentJournalError::Replay)
        ));
        assert!(guard.is_refused());
        drop(guard);
        assert!(!worker.is_refused());
    }
}

#[test]
fn intent_journal_publication_faults_retain_without_retry_or_replay() {
    for stage in [
        PublicationStage::RecordCreated,
        PublicationStage::RecordSynced,
        PublicationStage::MarkerCreated,
        PublicationStage::DurableCommit,
    ] {
        let (_temp, root, f) = stored(0);
        let packet = bytes(&f);
        let mut worker = f.worker();
        let mut guard = verify_intent_projection(&mut worker, &packet, &f.check(100)).unwrap();
        assert!(matches!(
            operate(
                &root,
                &mut guard,
                &packet,
                &f.check(100),
                true,
                |observed, _| {
                    if observed == stage {
                        Err(IntentJournalError::Uncertain)
                    } else {
                        Ok(())
                    }
                }
            ),
            Err(IntentJournalError::Uncertain)
        ));
        let retained = fs::read(root.join(name(0, "json"))).unwrap();
        assert!(guard.is_refused());
        assert!(matches!(
            retain_fixture_intent(&root, &mut guard, &packet, &f.check(100)),
            Err(IntentJournalError::Intent(IntentProjectionError::Refused))
        ));
        assert_eq!(retained, fs::read(root.join(name(0, "json"))).unwrap());
        drop(guard);
        assert!(!worker.is_refused());
        let mut worker = f.worker();
        let mut guard = verify_intent_projection(&mut worker, &packet, &f.check(100)).unwrap();
        let inspected = inspect_fixture_intents(&root, &mut guard, &packet, &f.check(100));
        if stage == PublicationStage::DurableCommit {
            assert_eq!(inspected.unwrap().consumed_cells, 1);
            assert!(matches!(
                retain_fixture_intent(&root, &mut guard, &packet, &f.check(100)),
                Err(IntentJournalError::Replay)
            ));
        } else {
            assert!(inspected.is_err());
            assert!(guard.is_refused());
        }
    }
}

#[test]
fn intent_journal_bad_expired_or_production_inputs_never_write() {
    let (_temp, root, f) = stored(0);
    let packet = bytes(&f);
    assert_eq!(f.reject(&[], 100), IntentProjectionError::Document);
    for mutation in 0..3 {
        let mut worker = f.worker();
        let mut guard = verify_intent_projection(&mut worker, &packet, &f.check(100)).unwrap();
        let target = if mutation == 2 {
            Path::new("/var/lib/hydracache-performance/campaigns")
        } else {
            &root
        };
        assert!(retain_fixture_intent(
            target,
            &mut guard,
            if mutation == 0 { b"invalid" } else { &packet },
            &f.check(if mutation == 1 { 160 } else { 100 })
        )
        .is_err());
        assert!(guard.is_refused());
        assert!(!root.join(name(0, "json")).exists());
    }
}

#[test]
fn intent_journal_new_valid_signature_does_not_reopen_consumed_cell() {
    let (_temp, root, f) = stored(0);
    let packet = bytes(&f);
    let mut worker = f.worker();
    let mut guard = verify_intent_projection(&mut worker, &packet, &f.check(100)).unwrap();
    retain_fixture_intent(&root, &mut guard, &packet, &f.check(100)).unwrap();
    drop(guard);
    let original = fs::read(root.join(name(0, "json"))).unwrap();
    let mut body = f.body();
    body.request_id = "6".repeat(64);
    body.nonce_sha256 = "7".repeat(64);
    let replacement = f.signed(&body);
    let mut worker = f.worker();
    let mut guard = verify_intent_projection(&mut worker, &replacement, &f.check(100)).unwrap();
    assert!(matches!(
        retain_fixture_intent(&root, &mut guard, &replacement, &f.check(100)),
        Err(IntentJournalError::Replay)
    ));
    assert_eq!(original, fs::read(root.join(name(0, "json"))).unwrap());
}

#[test]
fn intent_journal_repeated_nonce_or_request_refuses_before_new_record() {
    for repeat_nonce in [false, true] {
        let (_temp, root, mut f) = stored(0);
        let first = bytes(&f);
        let mut worker = f.worker();
        let mut guard = verify_intent_projection(&mut worker, &first, &f.check(100)).unwrap();
        retain_fixture_intent(&root, &mut guard, &first, &f.check(100)).unwrap();
        drop(guard);
        f.state.completed_cells = 1;
        f.state.revision += 1;
        persist(&root, &f.state);
        let old: SignedIntentProjection = serde_json::from_slice(&first).unwrap();
        let mut next: SignedIntentProjection = serde_json::from_slice(&bytes(&f)).unwrap();
        if repeat_nonce {
            next.intent.nonce_sha256 = old.intent.nonce_sha256;
        } else {
            next.intent.request_id = old.intent.request_id;
        }
        let packet = f.signed(&next.intent);
        let mut worker = f.worker();
        let mut guard = verify_intent_projection(&mut worker, &packet, &f.check(100)).unwrap();
        assert!(matches!(
            retain_fixture_intent(&root, &mut guard, &packet, &f.check(100)),
            Err(IntentJournalError::Replay)
        ));
        assert!(!root.join(name(1, "json")).exists());
    }
}

#[test]
fn intent_journal_persisted_state_pending_and_ipc_fail_closed() {
    for mutation in 0..5 {
        let (_temp, root, mut f) = stored(0);
        let packet = bytes(&f);
        let mut worker = f.worker();
        let mut guard = verify_intent_projection(&mut worker, &packet, &f.check(100)).unwrap();
        match mutation {
            0 => {
                let mut state = f.state.clone();
                state.revision += 1;
                persist(&root, &state);
            }
            1 => symlink("missing", root.join(".active-diagnostic.pending")).unwrap(),
            2 => symlink("missing", root.join(".diagnostic-requests.pending")).unwrap(),
            3 => fs::write(root.join("active-campaign"), []).unwrap(),
            _ => {
                fs::write(root.join("diagnostic-requests-v1.json"), b"invalid").unwrap();
                fs::set_permissions(
                    root.join("diagnostic-requests-v1.json"),
                    fs::Permissions::from_mode(0o600),
                )
                .unwrap();
            }
        }
        assert!(retain_fixture_intent(&root, &mut guard, &packet, &f.check(100)).is_err());
        assert!(guard.is_refused());
        f.state.revision += 1;
        assert!(!root.join(name(0, "json")).exists());
    }
}

#[test]
fn intent_journal_corrupt_schema_chain_marker_and_metadata_stay_retained() {
    for mutation in 0..6 {
        let (_temp, root, f) = stored(0);
        let packet = bytes(&f);
        let mut worker = f.worker();
        let mut guard = verify_intent_projection(&mut worker, &packet, &f.check(100)).unwrap();
        retain_fixture_intent(&root, &mut guard, &packet, &f.check(100)).unwrap();
        drop(guard);
        let path = root.join(name(0, "json"));
        let original = fs::read(&path).unwrap();
        if mutation < 3 {
            let mut value: serde_json::Value = serde_json::from_slice(&original).unwrap();
            match mutation {
                0 => value["schema_version"] = json!("future"),
                1 => value["previous_record_sha256"] = json!("a".repeat(64)),
                _ => value["future"] = json!(1),
            }
            let changed = line(&value);
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
            fs::write(&path, &changed).unwrap();
            let marker = root.join(name(0, "commit"));
            fs::set_permissions(&marker, fs::Permissions::from_mode(0o600)).unwrap();
            fs::write(&marker, marker_bytes(&changed)).unwrap();
            fs::set_permissions(marker, fs::Permissions::from_mode(0o400)).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
        } else {
            match mutation {
                3 => fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap(),
                4 => fs::hard_link(&path, root.join("extra-link")).unwrap(),
                _ => {
                    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
                    fs::write(&path, vec![0; MAX_RECORD_BYTES + 1]).unwrap();
                    fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
                }
            }
        }
        let retained = fs::read(&path).unwrap();
        let mut worker = f.worker();
        let mut guard = verify_intent_projection(&mut worker, &packet, &f.check(100)).unwrap();
        assert!(inspect_fixture_intents(&root, &mut guard, &packet, &f.check(100)).is_err());
        assert!(guard.is_refused());
        assert_eq!(retained, fs::read(&path).unwrap());
    }
}

#[test]
fn intent_journal_unknown_slots_gaps_and_directory_budget_refuse() {
    for mutation in 0..4 {
        let (_temp, root, f) = stored(if mutation == 3 { 1 } else { 0 });
        let packet = bytes(&f);
        let mut worker = f.worker();
        let mut guard = verify_intent_projection(&mut worker, &packet, &f.check(100)).unwrap();
        match mutation {
            0 => {
                fs::write(root.join("diagnostic-intent-cell-5.json"), []).unwrap();
                fs::write(root.join("diagnostic-intent-cell-5.commit"), []).unwrap();
            }
            1 => symlink("missing", root.join(".diagnostic-intent-future")).unwrap(),
            2 => {
                for i in 0..MAX_DIRECTORY_ENTRIES {
                    fs::write(root.join(format!("extra-{i}")), []).unwrap();
                }
            }
            _ => (),
        }
        assert!(retain_fixture_intent(&root, &mut guard, &packet, &f.check(100)).is_err());
        assert!(!root.join(name(f.state.completed_cells, "json")).exists());
    }
}

#[test]
fn intent_journal_historical_signature_cannot_change_controller() {
    let (_temp, root, mut f) = stored(0);
    let packet = bytes(&f);
    let mut worker = f.worker();
    let mut guard = verify_intent_projection(&mut worker, &packet, &f.check(100)).unwrap();
    retain_fixture_intent(&root, &mut guard, &packet, &f.check(100)).unwrap();
    drop(guard);
    let original = fs::read(root.join(name(0, "json"))).unwrap();
    f.key = SigningKey::from_bytes(&[76; 32]);
    f.controller.verifying_key = f.key.verifying_key();
    let packet = bytes(&f);
    let mut worker = f.worker();
    let mut guard = verify_intent_projection(&mut worker, &packet, &f.check(100)).unwrap();
    assert!(matches!(
        inspect_fixture_intents(&root, &mut guard, &packet, &f.check(100)),
        Err(IntentJournalError::Intent(IntentProjectionError::Signature))
    ));
    assert_eq!(original, fs::read(root.join(name(0, "json"))).unwrap());
}

#[test]
fn intent_journal_asserted_reboot_never_adopts_old_lease_history() {
    let (_temp, root, mut f) = stored(0);
    let packet = bytes(&f);
    let mut worker = f.worker();
    let mut guard = verify_intent_projection(&mut worker, &packet, &f.check(100)).unwrap();
    retain_fixture_intent(&root, &mut guard, &packet, &f.check(100)).unwrap();
    drop(guard);
    let original = fs::read(root.join(name(0, "json"))).unwrap();
    let mut policy: SignedWorkerPolicy = serde_json::from_slice(&f.worker_bytes).unwrap();
    f.host.boot_id = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".into();
    policy.policy.boot_id = f.host.boot_id.clone();
    let mut message = WORKER_POLICY_DOMAIN.to_vec();
    message.push(0);
    message.extend(line(&policy.policy));
    policy.signature_hex = hex(&SigningKey::from_bytes(&[74; 32]).sign(&message).to_bytes());
    f.trust = WorkerPolicyTrust::new(
        SigningKey::from_bytes(&[74; 32]).verifying_key(),
        &digest(&line(&policy.policy)),
        7,
    )
    .unwrap();
    f.worker_bytes = line(&policy);
    f.state.identity.boot_id = f.host.boot_id.clone();
    persist(&root, &f.state);
    let packet = bytes(&f);
    let mut worker = f.worker();
    let mut guard = verify_intent_projection(&mut worker, &packet, &f.check(100)).unwrap();
    assert!(matches!(
        retain_fixture_intent(&root, &mut guard, &packet, &f.check(100)),
        Err(IntentJournalError::History)
    ));
    assert_eq!(original, fs::read(root.join(name(0, "json"))).unwrap());
}

#[test]
fn intent_journal_original_lock_state_and_root_replacement_latch() {
    for mutation in 0..3 {
        let (_temp, root, f) = stored(0);
        let packet = bytes(&f);
        let mut worker = f.worker();
        let mut guard = verify_intent_projection(&mut worker, &packet, &f.check(100)).unwrap();
        let result = operate(
            &root,
            &mut guard,
            &packet,
            &f.check(100),
            true,
            |stage, _| {
                if stage == PublicationStage::RecordCreated {
                    match mutation {
                        0 => {
                            let path = root.join(".host-execution.lock");
                            fs::rename(&path, root.join("retained-lock")).unwrap();
                            fs::write(&path, []).unwrap();
                            fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
                        }
                        1 => {
                            let path = root.join("active-diagnostic.json");
                            fs::rename(path, root.join("retained-state")).unwrap();
                            persist(&root, &f.state);
                        }
                        _ => {
                            fs::rename(&root, root.with_extension("retained")).unwrap();
                            fs::create_dir(&root).unwrap();
                            fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
                        }
                    }
                }
                Ok(())
            },
        );
        assert!(result.is_err());
        assert!(guard.is_refused());
    }
}

#[test]
fn intent_journal_concurrent_requests_consume_only_one_cell() {
    let (_temp, root, _f) = stored(0);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let root = root.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let f = Fixture::new(0);
                let packet = bytes(&f);
                let mut worker = f.worker();
                let mut guard =
                    verify_intent_projection(&mut worker, &packet, &f.check(100)).unwrap();
                barrier.wait();
                retain_fixture_intent(&root, &mut guard, &packet, &f.check(100)).is_ok()
            })
        })
        .collect();
    assert_eq!(
        handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .filter(|success| *success)
            .count(),
        1
    );
    assert!(root.join(name(0, "json")).is_file());
    assert!(root.join(name(0, "commit")).is_file());
    assert!(!root.join(name(1, "json")).exists());
}

#[test]
fn intent_journal_created_record_replacement_is_not_adopted() {
    let (_temp, root, f) = stored(0);
    let packet = bytes(&f);
    let mut worker = f.worker();
    let mut guard = verify_intent_projection(&mut worker, &packet, &f.check(100)).unwrap();
    let mut reached = false;
    let result = operate(
        &root,
        &mut guard,
        &packet,
        &f.check(100),
        true,
        |stage, _| {
            if stage == PublicationStage::RecordSynced {
                reached = true;
                let path = root.join(name(0, "json"));
                let raw = fs::read(&path).unwrap();
                fs::rename(&path, root.join("retained-original-record")).unwrap();
                fs::write(&path, raw).unwrap();
                fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
            }
            Ok(())
        },
    );
    assert!(
        reached,
        "must reach sealed record before testing replacement: {result:?}"
    );
    assert!(
        result.is_err(),
        "new named inode with identical bytes must not replace created object"
    );
    assert!(guard.is_refused());
}

#[test]
fn intent_journal_marker_replacement_after_sync_is_not_adopted() {
    let (_temp, root, f) = stored(0);
    let packet = bytes(&f);
    let mut worker = f.worker();
    let mut guard = verify_intent_projection(&mut worker, &packet, &f.check(100)).unwrap();
    let mut reached = false;
    let result = operate(
        &root,
        &mut guard,
        &packet,
        &f.check(100),
        true,
        |stage, _| {
            if stage == PublicationStage::DurableCommit {
                reached = true;
                let path = root.join(name(0, "commit"));
                let raw = fs::read(&path).unwrap();
                fs::rename(&path, root.join("retained-original-marker")).unwrap();
                fs::write(&path, raw).unwrap();
                fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
            }
            Ok(())
        },
    );
    assert!(reached);
    assert!(result.is_err());
    assert!(guard.is_refused());
}

#[test]
fn intent_journal_symlink_fifo_and_unsafe_parent_refuse_without_writes() {
    for mutation in 0..3 {
        let (_temp, root, f) = stored(0);
        let packet = bytes(&f);
        let mut worker = f.worker();
        let mut guard = verify_intent_projection(&mut worker, &packet, &f.check(100)).unwrap();
        if mutation == 2 {
            fs::set_permissions(root.parent().unwrap(), fs::Permissions::from_mode(0o755)).unwrap();
        } else {
            let path = root.join(name(0, "json"));
            if mutation == 0 {
                symlink("missing", &path).unwrap();
            } else {
                use std::os::unix::ffi::OsStrExt;
                let c = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
                assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o400) }, 0);
            }
            fs::write(root.join(name(0, "commit")), []).unwrap();
            fs::set_permissions(
                root.join(name(0, "commit")),
                fs::Permissions::from_mode(0o400),
            )
            .unwrap();
        }
        assert!(inspect_fixture_intents(&root, &mut guard, &packet, &f.check(100)).is_err());
        assert!(guard.is_refused());
        assert!(!root.join(name(1, "json")).exists());
    }
}

#[test]
fn intent_journal_historical_binding_is_checked_after_rehashing() {
    for mutation in 0..4 {
        let (_temp, root, f) = stored(0);
        let packet = bytes(&f);
        let mut worker = f.worker();
        let mut guard = verify_intent_projection(&mut worker, &packet, &f.check(100)).unwrap();
        retain_fixture_intent(&root, &mut guard, &packet, &f.check(100)).unwrap();
        drop(guard);
        let path = root.join(name(0, "json"));
        let mut record: Record = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        match mutation {
            0 => record.peer_uid = 901,
            1 => record.peer_gids.push(903),
            2 => record.state.revision += 1,
            _ => {
                let mut signed: SignedIntentProjection =
                    serde_json::from_str(&record.signed_intent).unwrap();
                signed.intent.controller.run_id += 1;
                record.signed_intent = String::from_utf8(f.signed(&signed.intent)).unwrap();
            }
        }
        let changed = record_bytes(&record).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        fs::write(&path, &changed).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
        let marker = root.join(name(0, "commit"));
        fs::set_permissions(&marker, fs::Permissions::from_mode(0o600)).unwrap();
        fs::write(&marker, marker_bytes(&changed)).unwrap();
        fs::set_permissions(marker, fs::Permissions::from_mode(0o400)).unwrap();
        let mut worker = f.worker();
        let mut guard = verify_intent_projection(&mut worker, &packet, &f.check(100)).unwrap();
        assert!(inspect_fixture_intents(&root, &mut guard, &packet, &f.check(100)).is_err());
        assert!(guard.is_refused());
        assert_eq!(changed, fs::read(&path).unwrap());
    }
}

#[test]
fn intent_journal_previous_record_replacement_is_not_adopted() {
    let (_temp, root, mut f) = stored(0);
    let first = bytes(&f);
    let mut worker = f.worker();
    let mut guard = verify_intent_projection(&mut worker, &first, &f.check(100)).unwrap();
    retain_fixture_intent(&root, &mut guard, &first, &f.check(100)).unwrap();
    drop(guard);
    f.state.completed_cells = 1;
    f.state.revision += 1;
    persist(&root, &f.state);
    let packet = bytes(&f);
    let mut worker = f.worker();
    let mut guard = verify_intent_projection(&mut worker, &packet, &f.check(100)).unwrap();
    let mut reached = false;
    let result = operate(
        &root,
        &mut guard,
        &packet,
        &f.check(100),
        true,
        |stage, _| {
            if stage == PublicationStage::RecordCreated {
                reached = true;
                let path = root.join(name(0, "json"));
                let raw = fs::read(&path).unwrap();
                fs::rename(&path, root.join("retained-previous-record")).unwrap();
                fs::write(&path, raw).unwrap();
                fs::set_permissions(path, fs::Permissions::from_mode(0o400)).unwrap();
            }
            Ok(())
        },
    );
    assert!(reached);
    assert!(result.is_err());
    assert!(guard.is_refused());
    assert!(!root.join(name(1, "commit")).exists());
}

#[test]
fn intent_journal_readonly_inspection_is_not_consumption_permission() {
    let (_temp, root, f) = stored(0);
    let packet = bytes(&f);
    let mut worker = f.worker();
    let mut guard = verify_intent_projection(&mut worker, &packet, &f.check(100)).unwrap();
    let original = fs::read(root.join("active-diagnostic.json")).unwrap();
    for _ in 0..3 {
        assert_eq!(
            inspect_fixture_intents(&root, &mut guard, &packet, &f.check(100))
                .unwrap()
                .consumed_cells,
            0
        );
    }
    assert_eq!(
        original,
        fs::read(root.join("active-diagnostic.json")).unwrap()
    );
    assert!(!root.join(name(0, "json")).exists());
    assert!(!guard.is_refused());
}
#[test]
fn intent_journal_historical_expiry_never_renews_execution_eligibility() {
    let (_temp, root, f) = stored(0);
    let packet = bytes(&f);
    let mut worker = f.worker();
    let mut guard = verify_intent_projection(&mut worker, &packet, &f.check(100)).unwrap();
    retain_fixture_intent(&root, &mut guard, &packet, &f.check(100)).unwrap();
    drop(guard);
    let mut body = f.body();
    body.issued_at_unix_seconds = 200;
    body.expires_at_unix_seconds = 260;
    let current = f.signed(&body);
    let mut worker = f.worker();
    let mut guard = verify_intent_projection(&mut worker, &current, &f.check(200)).unwrap();
    assert_eq!(
        inspect_fixture_intents(&root, &mut guard, &current, &f.check(200))
            .unwrap()
            .consumed_cells,
        1
    );
    assert!(matches!(
        retain_fixture_intent(&root, &mut guard, &current, &f.check(200)),
        Err(IntentJournalError::Replay)
    ));
    drop(guard);
    let mut worker = f.worker();
    let mut guard = verify_intent_projection(&mut worker, &current, &f.check(200)).unwrap();
    assert!(matches!(
        inspect_fixture_intents(&root, &mut guard, &current, &f.check(260)),
        Err(IntentJournalError::Intent(IntentProjectionError::Time))
    ));
    assert!(guard.is_refused());
}

#[test]
fn signed_history_empty_anchor_is_observation_only() {
    let f = Fixture::new(0);
    let packet = bytes(&f);
    let anchor = record_bytes(&Record {
        schema_version: RECORD_SCHEMA.into(),
        state: f.state.clone(),
        signed_intent: String::from_utf8(packet.clone()).unwrap(),
        peer_uid: f.peer.uid,
        peer_gids: f.peer.gids.clone(),
        previous_record_sha256: None,
    })
    .unwrap();
    let mut worker = f.worker();
    let mut guard = verify_intent_projection(&mut worker, &packet, &f.check(100)).unwrap();
    let observation =
        signed_history::inspect_signed_history(&mut guard, &packet, &f.check(100), &anchor, &[])
            .unwrap();
    assert_eq!(observation.retained_cells, 0);
    assert_eq!(observation.anchor_sha256, digest(&anchor));
    assert_eq!(observation.last_record_sha256, None);
    assert!(!guard.is_refused());
}

struct SignedChain {
    f: Fixture,
    anchor: Vec<u8>,
    pairs: Vec<(Vec<u8>, Vec<u8>)>,
}
impl SignedChain {
    fn new(cells: usize, heartbeats: u64) -> Self {
        let mut f = Fixture::new(0);
        let mut pairs = Vec::new();
        for cell in 0..cells {
            f.state.completed_cells = cell;
            f.state.revision = 1 + cell as u64 * (3 + heartbeats);
            f.state.last_observed_monotonic_ns = (2 + cell as u64 * 60) * 1_000_000_000;
            f.state.controller_monotonic_ns = f.state.last_observed_monotonic_ns;
            let raw = record_bytes(&Record {
                schema_version: RECORD_SCHEMA.into(),
                state: f.state.clone(),
                signed_intent: String::from_utf8(bytes(&f)).unwrap(),
                peer_uid: f.peer.uid,
                peer_gids: f.peer.gids.clone(),
                previous_record_sha256: pairs.last().map(|p: &(Vec<u8>, Vec<u8>)| digest(&p.0)),
            })
            .unwrap();
            pairs.push((raw.clone(), marker_bytes(&raw)));
        }
        Self {
            f,
            anchor: pairs[0].0.clone(),
            pairs,
        }
    }
    fn rewrite(
        &mut self,
        index: usize,
        change: impl FnOnce(&mut Record, &mut IntentProjectionBody),
    ) {
        let mut record: Record = serde_json::from_slice(&self.pairs[index].0).unwrap();
        let mut body: SignedIntentProjection = serde_json::from_str(&record.signed_intent).unwrap();
        change(&mut record, &mut body.intent);
        let projection = project_reserved_state(&record.state).unwrap();
        body.intent.state_sha256 = projection.state_sha256;
        body.intent.unit_spec_sha256 = projection.unit_spec_sha256;
        record.signed_intent = String::from_utf8(self.f.signed(&body.intent)).unwrap();
        self.pairs[index].0 = record_bytes(&record).unwrap();
        for i in index..self.pairs.len() {
            if i > index {
                let mut record: Record = serde_json::from_slice(&self.pairs[i].0).unwrap();
                record.previous_record_sha256 = Some(digest(&self.pairs[i - 1].0));
                self.pairs[i].0 = record_bytes(&record).unwrap();
            }
            self.pairs[i].1 = marker_bytes(&self.pairs[i].0);
        }
    }
    fn inspect(
        &self,
        now: u64,
    ) -> Result<signed_history::SignedHistoryObservation, signed_history::SignedHistoryError> {
        let packet = bytes(&self.f);
        let mut worker = self.f.worker();
        let mut guard = verify_intent_projection(&mut worker, &packet, &self.f.check(100)).unwrap();
        let pairs: Vec<_> = self
            .pairs
            .iter()
            .map(|(record, commit)| signed_history::RetainedIntentPair { record, commit })
            .collect();
        let result = signed_history::inspect_signed_history(
            &mut guard,
            &packet,
            &self.f.check(now),
            &self.anchor,
            &pairs,
        );
        assert_eq!(guard.is_refused(), result.is_err());
        if result.is_err() {
            assert!(matches!(
                guard.revalidate(&packet, &self.f.check(100)),
                Err(IntentProjectionError::Refused)
            ));
        }
        result
    }
}

#[test]
fn signed_history_all_prefixes_and_heartbeat_revisions_are_readonly() {
    for cells in 1..=4 {
        for heartbeats in [0, 1, 19] {
            let mut chain = SignedChain::new(cells, heartbeats);
            let original = chain.pairs.clone();
            for _ in 0..3 {
                let view = chain.inspect(100).unwrap();
                assert_eq!(view.retained_cells, cells);
                assert_eq!(
                    view.last_record_sha256,
                    Some(digest(&chain.pairs[cells - 1].0))
                );
            }
            assert_eq!(chain.pairs, original);
            if cells < 4 {
                chain.f.state.completed_cells += 1;
                chain.f.state.revision += 3;
                assert_eq!(chain.inspect(100).unwrap().retained_cells, cells);
            }
        }
    }
}

#[test]
fn signed_history_exact_anchor_pair_and_document_budgets_refuse() {
    for mutation in 0..11 {
        let mut chain = SignedChain::new(3, 0);
        match mutation {
            0 => chain.anchor.push(b' '),
            1 => chain.anchor = vec![0; MAX_RECORD_BYTES + 1],
            2 => chain.pairs[1].0 = vec![0; MAX_RECORD_BYTES + 1],
            3 => chain.pairs[1].1.pop().map(|_| ()).unwrap(),
            4 => chain.pairs[1].1 = vec![b'a'; MARKER_BYTES],
            5 => chain.pairs.remove(1).0.clear(),
            6 => chain.pairs.push(chain.pairs[2].clone()),
            7 => {
                chain.pairs.push(chain.pairs[2].clone());
                chain.pairs.push(chain.pairs[2].clone());
            }
            8 => {
                let mut record: Record = serde_json::from_slice(&chain.pairs[1].0).unwrap();
                record.schema_version = "future".into();
                chain.pairs[1].0 = record_bytes(&record).unwrap();
                chain.pairs[1].1 = marker_bytes(&chain.pairs[1].0);
            }
            9 => {
                chain.rewrite(0, |_, body| body.request_id = "9".repeat(64));
            }
            _ => {
                chain.pairs[1].0.push(b' ');
                chain.pairs[1].1 = marker_bytes(&chain.pairs[1].0);
            }
        }
        let before = chain.pairs.clone();
        assert!(chain.inspect(100).is_err(), "mutation {mutation}");
        assert_eq!(before, chain.pairs);
    }
}

#[test]
fn signed_history_resigned_origin_clock_and_revision_mutations_refuse() {
    for mutation in 0..10 {
        let mut chain = SignedChain::new(4, 0);
        chain.rewrite(1, |record, _| match mutation {
            0 => record.state.identity.lease_id = "a".repeat(64),
            1 => record.state.identity.binary_sha256 = "a".repeat(64),
            2 => record.state.identity.build_provenance_sha256 = "a".repeat(64),
            3 => record.state.reserved_monotonic_ns += 1,
            4 => record.state.revision = 3,
            5 => record.state.revision = 1,
            6 => {
                record.state.last_observed_monotonic_ns = 1_500_000_000;
                record.state.controller_monotonic_ns = 1_500_000_000;
            }
            7 => record.state.completed_cells = 2,
            8 => record.previous_record_sha256 = Some("a".repeat(64)),
            _ => record.previous_record_sha256 = None,
        });
        assert!(chain.inspect(100).is_err(), "signed mutation {mutation}");
    }
}

#[test]
fn signed_history_duplicate_requests_and_nonces_refuse() {
    for nonce in [false, true] {
        let mut chain = SignedChain::new(4, 0);
        let first: Record = serde_json::from_slice(&chain.pairs[0].0).unwrap();
        let first: SignedIntentProjection = serde_json::from_str(&first.signed_intent).unwrap();
        chain.rewrite(2, |_, body| {
            if nonce {
                body.nonce_sha256 = first.intent.nonce_sha256;
            } else {
                body.request_id = first.intent.request_id;
            }
        });
        assert_eq!(
            chain.inspect(100),
            Err(signed_history::SignedHistoryError::Replay)
        );
    }
}

#[test]
fn signed_history_current_projection_and_missing_prefix_refuse() {
    for mutation in 0..6 {
        let mut chain = SignedChain::new(3, 0);
        match mutation {
            0 => chain.f.state.identity.lease_id = "a".repeat(64),
            1 => chain.f.state.revision -= 1,
            2 => chain.f.state.completed_cells = 0,
            3 => chain.pairs.clear(),
            4 => {
                chain.f.state.completed_cells += 1;
                chain.f.state.revision += 2;
            }
            _ => chain.f.state.reserved_monotonic_ns += 1,
        }
        assert!(chain.inspect(100).is_err(), "current mutation {mutation}");
    }
}

#[test]
fn signed_history_signatures_principals_and_time_are_not_renewed() {
    for mutation in 0..7 {
        let mut chain = SignedChain::new(3, 0);
        if mutation == 0 {
            let mut record: Record = serde_json::from_slice(&chain.pairs[1].0).unwrap();
            let mut signed: SignedIntentProjection =
                serde_json::from_str(&record.signed_intent).unwrap();
            signed.signature_hex = "0".repeat(128);
            record.signed_intent = String::from_utf8(line(&signed)).unwrap();
            chain.pairs[1].0 = record_bytes(&record).unwrap();
            chain.pairs[1].1 = marker_bytes(&chain.pairs[1].0);
            let previous = digest(&chain.pairs[1].0);
            chain.rewrite(2, |record, _| {
                record.previous_record_sha256 = Some(previous)
            });
        } else if mutation < 6 {
            chain.rewrite(1, |record, body| match mutation {
                1 => body.controller.run_id += 1,
                2 => record.peer_uid = 901,
                3 => body.worker_policy_epoch += 1,
                4 => body.boot_id = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".into(),
                _ => {
                    body.issued_at_unix_seconds = 101;
                    body.expires_at_unix_seconds = 161;
                }
            });
        }
        assert!(
            chain
                .inspect(if mutation == 6 { 160 } else { 100 })
                .is_err(),
            "authentication mutation {mutation}"
        );
    }
}

#[test]
fn signed_history_original_guard_input_drift_latches() {
    let chain = SignedChain::new(1, 0);
    let packet = bytes(&chain.f);
    let mut worker = chain.f.worker();
    let mut guard = verify_intent_projection(&mut worker, &packet, &chain.f.check(100)).unwrap();
    let mut changed = chain.f.body();
    changed.request_id = "8".repeat(64);
    assert!(signed_history::inspect_signed_history(
        &mut guard,
        &chain.f.signed(&changed),
        &chain.f.check(100),
        &chain.anchor,
        &[]
    )
    .is_err());
    assert!(guard.is_refused());
}

#[test]
fn signed_history_seeded_plausibility_never_proves_completion() {
    let mut seed = 0x074fu64;
    for _ in 0..64 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let mut chain = SignedChain::new(4, seed % 23);
        assert_eq!(chain.inspect(100).unwrap().retained_cells, 4);
        let index = 1 + (seed as usize % 3);
        let previous: Record = serde_json::from_slice(&chain.pairs[index - 1].0).unwrap();
        chain.rewrite(index, |record, _| {
            record.state.revision = previous.state.revision + 2
        });
        assert!(chain.inspect(100).is_err());
    }
    println!(
        "signed history seed=0x074f legal=64 signed_mutations=64; inspection is not completion"
    );
}

#[test]
fn signed_history_expired_history_is_not_current_intent_renewal() {
    let chain = SignedChain::new(4, 0);
    let mut body = chain.f.body();
    body.issued_at_unix_seconds = 200;
    body.expires_at_unix_seconds = 260;
    let current = chain.f.signed(&body);
    let mut worker = chain.f.worker();
    let mut guard = verify_intent_projection(&mut worker, &current, &chain.f.check(200)).unwrap();
    let pairs: Vec<_> = chain
        .pairs
        .iter()
        .map(|(record, commit)| signed_history::RetainedIntentPair { record, commit })
        .collect();
    assert_eq!(
        signed_history::inspect_signed_history(
            &mut guard,
            &current,
            &chain.f.check(200),
            &chain.anchor,
            &pairs
        )
        .unwrap()
        .retained_cells,
        4
    );
    assert!(signed_history::inspect_signed_history(
        &mut guard,
        &current,
        &chain.f.check(260),
        &chain.anchor,
        &pairs
    )
    .is_err());
    assert!(guard.is_refused());
}

#[test]
fn signed_history_independent_concurrent_inspections_share_no_acknowledgement() {
    let chain = std::sync::Arc::new(SignedChain::new(4, 1));
    let before = chain.pairs.clone();
    let threads: Vec<_> = (0..4)
        .map(|_| {
            let chain = chain.clone();
            std::thread::spawn(move || chain.inspect(100).unwrap())
        })
        .collect();
    for thread in threads {
        assert_eq!(thread.join().unwrap().retained_cells, 4);
    }
    assert_eq!(before, chain.pairs);
}
