use ed25519_dalek::{Signer, SigningKey};
use hydracache_long_run_supervisor_074::diagnostic_ipc::*;
use hydracache_long_run_supervisor_074::diagnostic_lease::{
    CellIntent, DiagnosticBackend, DiagnosticClock, DiagnosticCoordinator, DiagnosticIdentity,
    DiagnosticStage, TreeObservation, ACTIVE_DIAGNOSTIC_NAME, SOURCE_COMMIT,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn key() -> SigningKey {
    SigningKey::from_bytes(&[74; 32])
}

fn policy() -> DiagnosticPolicy {
    DiagnosticPolicy {
        repository_id: 1,
        actor_ids: vec![4],
        client_uids: vec![10],
        client_gid: 20,
        verifying_key: key().verifying_key(),
    }
}

fn peer() -> DiagnosticPeer {
    DiagnosticPeer {
        uid: 10,
        gids: vec![20],
    }
}

fn request(n: u64, operation: DiagnosticOperation, revision: u64) -> DiagnosticRequest {
    DiagnosticRequest {
        schema_version: 1,
        request_id: format!("{n:064x}"),
        nonce_sha256: format!("{:064x}", n + 1000),
        operation,
        expected_state_revision: revision,
        identity: DiagnosticIdentity {
            lease_id: "a".repeat(64),
            boot_id: "00000000-0000-4000-8000-000000000074".into(),
            binary_sha256: "b".repeat(64),
            build_provenance_sha256: "c".repeat(64),
        },
        source_commit: SOURCE_COMMIT.into(),
        preset: PRESET.into(),
        controller: DiagnosticController {
            repository_id: 1,
            run_id: 2,
            run_attempt: 3,
            actor_id: 4,
        },
        issued_at_unix_seconds: 100,
        expires_at_unix_seconds: 160,
    }
}

fn clock(seconds: u64) -> DiagnosticClock {
    DiagnosticClock {
        boot_id: request(1, DiagnosticOperation::Reserve, 0).identity.boot_id,
        monotonic_ns: seconds * 1_000_000_000,
    }
}

fn packet(r: &DiagnosticRequest) -> Vec<u8> {
    serde_json::to_vec(&SignedDiagnosticRequest {
        request: r.clone(),
        signature_hex: hex(&key().sign(&signing_message(r).unwrap()).to_bytes()),
    })
    .unwrap()
}

#[derive(Default)]
struct Backend {
    starts: usize,
    stops: usize,
    empty: bool,
    crash: bool,
    refuse: bool,
}

impl DiagnosticBackend for Backend {
    fn start_once(&mut self, _: &CellIntent) -> Result<(), String> {
        self.starts += 1;
        Ok(())
    }
    fn observe_tree(&mut self, i: &CellIntent) -> Result<TreeObservation, String> {
        Ok(TreeObservation {
            unit_name: i.unit_name.clone(),
            boot_id: i.boot_id.clone(),
            cgroup_path: i.cgroup_path.clone(),
            cgroup_inode: 74,
            populated: !self.empty,
            stdout_bytes: 0,
            stderr_bytes: 0,
            receipts_retained: true,
            outcome: None,
        })
    }
    fn stop_tree(&mut self, _: &CellIntent) -> Result<(), String> {
        self.stops += 1;
        assert!(!self.crash, "injected loss after durable IPC intent");
        if self.refuse {
            return Err("private backend path must not leak".into());
        }
        self.empty = true;
        Ok(())
    }
}

fn handle(
    root: &Path,
    r: &DiagnosticRequest,
    seconds: u64,
    b: &mut Backend,
) -> Result<DiagnosticResponse, DiagnosticIpcError> {
    handle_local(
        root,
        &packet(r),
        &peer(),
        &policy(),
        110,
        &clock(seconds),
        b,
    )
}

fn fixture() -> (tempfile::TempDir, Backend) {
    let dir = tempfile::tempdir().unwrap();
    let mut b = Backend::default();
    assert!(
        handle(
            dir.path(),
            &request(1, DiagnosticOperation::Reserve, 0),
            0,
            &mut b
        )
        .unwrap()
        .ok
    );
    (dir, b)
}

fn rewrite_ledger(path: &Path, edit: impl FnOnce(&mut Value)) {
    let mut value: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    edit(&mut value);
    value["ledger_sha256"] = json!(hex(&Sha256::digest(
        serde_json::to_vec(&value["ledger"]).unwrap()
    )));
    let mut bytes = serde_json::to_vec(&value).unwrap();
    bytes.push(b'\n');
    fs::write(path, bytes).unwrap();
}

#[test]
fn diagnostic_signatures_have_a_separate_domain() {
    assert_eq!(SIGNATURE_DOMAIN, b"hydracache-diagnostic-request-074-v1");
}

#[test]
fn signature_binds_every_request_field_and_rejects_campaign_domain() {
    let r = request(1, DiagnosticOperation::Reserve, 0);
    let original: Value = serde_json::from_slice(&packet(&r)).unwrap();
    let edits = [
        ("/request/request_id", json!("d".repeat(64))),
        ("/request/nonce_sha256", json!("e".repeat(64))),
        ("/request/operation", json!("status")),
        ("/request/expected_state_revision", json!(1)),
        ("/request/identity/lease_id", json!("d".repeat(64))),
        (
            "/request/identity/boot_id",
            json!("00000000-0000-4000-8000-000000000075"),
        ),
        ("/request/identity/binary_sha256", json!("d".repeat(64))),
        (
            "/request/identity/build_provenance_sha256",
            json!("d".repeat(64)),
        ),
        ("/request/source_commit", json!("d".repeat(40))),
        ("/request/preset", json!("another-workload")),
        ("/request/controller/repository_id", json!(2)),
        ("/request/controller/run_id", json!(3)),
        ("/request/controller/run_attempt", json!(4)),
        ("/request/controller/actor_id", json!(5)),
        ("/request/issued_at_unix_seconds", json!(101)),
        ("/request/expires_at_unix_seconds", json!(159)),
        ("/request/schema_version", json!(2)),
    ];
    for (field, value) in edits {
        let mut tampered = original.clone();
        *tampered.pointer_mut(field).unwrap() = value;
        assert!(
            authenticate(
                &serde_json::to_vec(&tampered).unwrap(),
                &peer(),
                &policy(),
                110
            )
            .is_err(),
            "{field}"
        );
    }
    let mut domain_message = b"hydracache-long-run-authorization-v1\0".to_vec();
    // Same canonical body, wrong domain: isolation is cryptographic, not merely JSON shape.
    let body = serde_json::to_value(&r).unwrap();
    domain_message.extend(serde_json::to_vec(&body).unwrap());
    let wrong = SignedDiagnosticRequest {
        request: r,
        signature_hex: hex(&key().sign(&domain_message).to_bytes()),
    };
    assert!(matches!(
        authenticate(
            &serde_json::to_vec(&wrong).unwrap(),
            &peer(),
            &policy(),
            110
        ),
        Err(DiagnosticIpcError::Signature)
    ));
}

#[test]
fn authorization_window_has_exclusive_expiry_and_no_future_skew() {
    let r = request(1, DiagnosticOperation::Reserve, 0);
    for time in [100, 159] {
        assert!(authenticate(&packet(&r), &peer(), &policy(), time).is_ok());
    }
    for time in [0, 99, 160, u64::MAX] {
        assert!(matches!(
            authenticate(&packet(&r), &peer(), &policy(), time),
            Err(DiagnosticIpcError::Time)
        ));
    }
    for (issued, expires) in [(0, 1), (100, 100), (101, 100), (100, 161), (1, u64::MAX)] {
        let mut invalid = r.clone();
        invalid.issued_at_unix_seconds = issued;
        invalid.expires_at_unix_seconds = expires;
        assert!(signing_message(&invalid).is_err());
    }
}

#[test]
fn malformed_packets_unknown_fields_and_unsigned_overrides_are_rejected() {
    let good = packet(&request(1, DiagnosticOperation::Reserve, 0));
    for bad in [
        vec![],
        vec![b' '; MAX_PACKET_BYTES + 1],
        b"{}".to_vec(),
        [good.clone(), b"{}".to_vec()].concat(),
    ] {
        assert!(authenticate(&bad, &peer(), &policy(), 110).is_err());
    }
    let original: Value = serde_json::from_slice(&good).unwrap();
    for at in ["", "/request", "/request/identity", "/request/controller"] {
        let mut bad = original.clone();
        bad.pointer_mut(at).unwrap()["argv"] = json!(["--unsafe"]);
        assert!(authenticate(&serde_json::to_vec(&bad).unwrap(), &peer(), &policy(), 110).is_err());
    }
    for sig in [
        "00".repeat(64),
        "AB".repeat(64),
        "g".repeat(128),
        "0".repeat(126),
    ] {
        let mut bad = original.clone();
        bad["signature_hex"] = json!(sig);
        assert!(authenticate(&serde_json::to_vec(&bad).unwrap(), &peer(), &policy(), 110).is_err());
    }
    let mut duplicate = String::from_utf8(good).unwrap();
    duplicate.insert_str(1, "\"signature_hex\":\"00\",");
    assert!(authenticate(duplicate.as_bytes(), &peer(), &policy(), 110).is_err());
}

#[test]
fn peer_and_signed_principal_denials_do_not_create_ledger_or_lease() {
    let dir = tempfile::tempdir().unwrap();
    let r = request(1, DiagnosticOperation::Reserve, 0);
    for denied in [
        DiagnosticPeer {
            uid: 11,
            gids: vec![20],
        },
        DiagnosticPeer {
            uid: 10,
            gids: vec![21],
        },
    ] {
        assert!(matches!(
            handle_local(
                dir.path(),
                &packet(&r),
                &denied,
                &policy(),
                110,
                &clock(0),
                &mut Backend::default()
            ),
            Err(DiagnosticIpcError::Principal)
        ));
    }
    for actor in [0, 5] {
        let mut denied = r.clone();
        denied.controller.actor_id = actor;
        if actor == 0 {
            assert!(signing_message(&denied).is_err());
        } else {
            assert!(matches!(
                authenticate(&packet(&denied), &peer(), &policy(), 110),
                Err(DiagnosticIpcError::Principal)
            ));
        }
    }
    let mut denied = r.clone();
    denied.controller.repository_id = 2;
    assert!(matches!(
        authenticate(&packet(&denied), &peer(), &policy(), 110),
        Err(DiagnosticIpcError::Principal)
    ));
    assert!(!dir.path().join(LEDGER_NAME).exists());
    assert!(!dir.path().join(ACTIVE_DIAGNOSTIC_NAME).exists());
}

#[test]
fn completed_replay_is_cached_without_extending_heartbeat() {
    let (dir, mut b) = fixture();
    let h = request(2, DiagnosticOperation::Heartbeat, 1);
    let receipt = handle(dir.path(), &h, 1, &mut b).unwrap();
    let ledger = fs::read(dir.path().join(LEDGER_NAME)).unwrap();
    assert_eq!(handle(dir.path(), &h, 5, &mut b).unwrap(), receipt);
    assert_eq!(fs::read(dir.path().join(LEDGER_NAME)).unwrap(), ledger);
    let c = DiagnosticCoordinator::recover(dir.path(), &h.identity.lease_id).unwrap();
    assert_eq!(c.state().unwrap().controller_monotonic_ns, 1_000_000_000);
    assert_eq!(c.state().unwrap().revision, 2);
    assert!(matches!(
        handle_local(
            dir.path(),
            &packet(&h),
            &peer(),
            &policy(),
            160,
            &clock(5),
            &mut b
        ),
        Err(DiagnosticIpcError::Time)
    ));
    assert_eq!((b.starts, b.stops), (0, 0));
}

#[test]
fn changed_request_and_cross_lease_nonce_reuse_are_refused() {
    let (dir, mut b) = fixture();
    let original = request(1, DiagnosticOperation::Reserve, 0);
    let mut changed = original.clone();
    changed.controller.run_attempt += 1;
    assert!(matches!(
        handle(dir.path(), &changed, 0, &mut b),
        Err(DiagnosticIpcError::Replay)
    ));
    changed = request(2, DiagnosticOperation::Reserve, 0);
    changed.identity.lease_id = "d".repeat(64);
    changed.nonce_sha256 = original.nonce_sha256;
    assert!(matches!(
        handle(dir.path(), &changed, 0, &mut b),
        Err(DiagnosticIpcError::Replay)
    ));
    assert_eq!((b.starts, b.stops), (0, 0));
}

#[test]
fn stale_revision_and_controller_takeover_are_refused_before_mutation() {
    let (dir, mut b) = fixture();
    let h = request(2, DiagnosticOperation::Heartbeat, 2);
    let ledger = fs::read(dir.path().join(LEDGER_NAME)).unwrap();
    assert!(matches!(
        handle(dir.path(), &h, 1, &mut b),
        Err(DiagnosticIpcError::Revision)
    ));
    for drift in 0..5 {
        let mut h = request(2, DiagnosticOperation::Heartbeat, 1);
        match drift {
            0 => h.controller.run_attempt += 1,
            1 => h.controller.run_id += 1,
            2 => h.identity.binary_sha256 = "d".repeat(64),
            3 => h.identity.build_provenance_sha256 = "d".repeat(64),
            _ => h.identity.boot_id = "00000000-0000-4000-8000-000000000075".into(),
        }
        assert!(matches!(
            handle(dir.path(), &h, 1, &mut b),
            Err(DiagnosticIpcError::Binding)
        ));
    }
    assert_eq!(fs::read(dir.path().join(LEDGER_NAME)).unwrap(), ledger);
}

#[test]
fn pending_intent_blocks_reexecution_and_new_requests() {
    let (dir, mut b) = fixture();
    let c = DiagnosticCoordinator::recover(
        dir.path(),
        &request(1, DiagnosticOperation::Reserve, 0)
            .identity
            .lease_id,
    )
    .unwrap();
    c.advance(&clock(1), &mut b).unwrap();
    let cancel = request(2, DiagnosticOperation::Cancel, c.state().unwrap().revision);
    b.crash = true;
    let interrupted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        handle(dir.path(), &cancel, 2, &mut b)
    }));
    assert!(interrupted.is_err());
    assert!(dir.path().join(ACTIVE_DIAGNOSTIC_NAME).exists());
    b.crash = false;
    assert!(matches!(
        handle(dir.path(), &cancel, 3, &mut b),
        Err(DiagnosticIpcError::Uncertain)
    ));
    assert!(matches!(
        handle(
            dir.path(),
            &request(
                3,
                DiagnosticOperation::Heartbeat,
                c.state().unwrap().revision
            ),
            3,
            &mut b
        ),
        Err(DiagnosticIpcError::Uncertain)
    ));
    assert_eq!((b.starts, b.stops), (1, 1));
}

#[test]
fn persisted_heartbeat_crash_gap_does_not_repeat_mutation() {
    let (dir, mut b) = fixture();
    let h = request(2, DiagnosticOperation::Heartbeat, 1);
    handle(dir.path(), &h, 1, &mut b).unwrap();
    // Model a crash after state mutation but before response publication by
    // retaining the canonical prior intent. Only the temporary fixture is edited.
    rewrite_ledger(&dir.path().join(LEDGER_NAME), |v| {
        v["ledger"]["entries"][1]["response"] = Value::Null
    });
    assert!(matches!(
        handle(dir.path(), &h, 5, &mut b),
        Err(DiagnosticIpcError::Uncertain)
    ));
    let c = DiagnosticCoordinator::recover(dir.path(), &h.identity.lease_id).unwrap();
    assert_eq!(c.state().unwrap().controller_monotonic_ns, 1_000_000_000);
}

#[test]
fn status_is_signed_and_never_refreshes_controller_liveness() {
    let (dir, mut b) = fixture();
    let s = request(2, DiagnosticOperation::Status, 1);
    let receipt = handle(dir.path(), &s, 9, &mut b).unwrap();
    assert_eq!(receipt.state.unwrap().controller_monotonic_ns, 0);
    let h = request(3, DiagnosticOperation::Heartbeat, 1);
    let rejected = handle(dir.path(), &h, 10, &mut b).unwrap();
    assert!(!rejected.ok);
    assert_eq!(rejected.error_code.as_deref(), Some("model_refused"));
    assert_eq!(handle(dir.path(), &h, 11, &mut b).unwrap(), rejected);
}

#[test]
fn cancel_replay_after_terminal_release_never_stops_tree_again() {
    let (dir, mut b) = fixture();
    let c = DiagnosticCoordinator::recover(
        dir.path(),
        &request(1, DiagnosticOperation::Reserve, 0)
            .identity
            .lease_id,
    )
    .unwrap();
    c.advance(&clock(1), &mut b).unwrap();
    let cancel = request(2, DiagnosticOperation::Cancel, c.state().unwrap().revision);
    let result = handle(dir.path(), &cancel, 2, &mut b).unwrap();
    assert_eq!(
        result.state.as_ref().unwrap().stage,
        DiagnosticStage::Terminal
    );
    assert!(!dir.path().join(ACTIVE_DIAGNOSTIC_NAME).exists());
    assert_eq!(handle(dir.path(), &cancel, 5, &mut b).unwrap(), result);
    assert_eq!((b.starts, b.stops), (1, 1));
}

#[test]
fn backend_failure_is_cached_without_leaking_paths_or_releasing_marker() {
    let (dir, mut b) = fixture();
    let c = DiagnosticCoordinator::recover(
        dir.path(),
        &request(1, DiagnosticOperation::Reserve, 0)
            .identity
            .lease_id,
    )
    .unwrap();
    c.advance(&clock(1), &mut b).unwrap();
    let cancel = request(2, DiagnosticOperation::Cancel, c.state().unwrap().revision);
    b.refuse = true;
    let receipt = handle(dir.path(), &cancel, 2, &mut b).unwrap();
    assert_eq!(receipt.error_code.as_deref(), Some("model_refused"));
    assert!(!String::from_utf8(serde_json::to_vec(&receipt).unwrap())
        .unwrap()
        .contains("private"));
    assert_eq!(handle(dir.path(), &cancel, 3, &mut b).unwrap(), receipt);
    assert!(dir.path().join(ACTIVE_DIAGNOSTIC_NAME).exists());
    assert_eq!(b.stops, 1);
}

#[test]
fn corrupt_future_noncanonical_or_pending_ledgers_fail_closed() {
    for kind in 0..7 {
        let (dir, mut b) = fixture();
        let path = dir.path().join(LEDGER_NAME);
        match kind {
            0 => fs::write(&path, b"{}").unwrap(),
            1 => rewrite_ledger(&path, |v| v["schema_version"] = json!(2)),
            2 => rewrite_ledger(&path, |v| v["unknown"] = json!(true)),
            3 => {
                let mut bytes = fs::read(&path).unwrap();
                bytes.push(b' ');
                fs::write(&path, bytes).unwrap();
            }
            4 => rewrite_ledger(&path, |v| {
                v["ledger"]["entries"][0]["request_sha256"] = json!("0".repeat(64))
            }),
            5 => fs::write(dir.path().join(PENDING_NAME), b"crash").unwrap(),
            _ => fs::write(&path, vec![b' '; MAX_LEDGER_BYTES as usize + 1]).unwrap(),
        }
        assert!(handle(
            dir.path(),
            &request(2, DiagnosticOperation::Heartbeat, 1),
            1,
            &mut b
        )
        .is_err());
        assert_eq!(
            DiagnosticCoordinator::recover(
                dir.path(),
                &request(1, DiagnosticOperation::Reserve, 0)
                    .identity
                    .lease_id
            )
            .unwrap()
            .state()
            .unwrap()
            .revision,
            1
        );
    }
}

#[test]
fn ledger_capacity_never_evicts_replay_protection() {
    let (dir, mut b) = fixture();
    for n in 2..=MAX_LEDGER_ENTRIES as u64 {
        assert!(
            handle(
                dir.path(),
                &request(n, DiagnosticOperation::Status, 1),
                0,
                &mut b
            )
            .unwrap()
            .ok
        );
    }
    let bytes = fs::read(dir.path().join(LEDGER_NAME)).unwrap();
    assert!(matches!(
        handle(
            dir.path(),
            &request(200, DiagnosticOperation::Heartbeat, 1),
            1,
            &mut b
        ),
        Err(DiagnosticIpcError::Capacity)
    ));
    assert_eq!(fs::read(dir.path().join(LEDGER_NAME)).unwrap(), bytes);
    assert!(
        handle(
            dir.path(),
            &request(1, DiagnosticOperation::Reserve, 0),
            1,
            &mut b
        )
        .unwrap()
        .ok
    );
}

#[test]
fn concurrent_same_revision_has_at_most_one_mutation() {
    let (dir, _) = fixture();
    let barrier = std::sync::Barrier::new(8);
    let outcomes = std::thread::scope(|scope| {
        let jobs: Vec<_> = (2..10)
            .map(|n| {
                let root = dir.path();
                let barrier = &barrier;
                scope.spawn(move || {
                    barrier.wait();
                    handle(
                        root,
                        &request(n, DiagnosticOperation::Heartbeat, 1),
                        1,
                        &mut Backend::default(),
                    )
                })
            })
            .collect();
        jobs.into_iter()
            .map(|j| j.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(
        outcomes
            .iter()
            .filter(|r| r.as_ref().is_ok_and(|v| v.ok))
            .count(),
        1
    );
    let c = DiagnosticCoordinator::recover(
        dir.path(),
        &request(1, DiagnosticOperation::Reserve, 0)
            .identity
            .lease_id,
    )
    .unwrap();
    assert_eq!(c.state().unwrap().revision, 2);
}

#[test]
fn ledger_digest_duplicate_identity_and_invalid_response_are_rejected() {
    for kind in 0..8 {
        let (dir, mut b) = fixture();
        let path = dir.path().join(LEDGER_NAME);
        if kind == 0 {
            let mut v: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            v["ledger_sha256"] = json!("0".repeat(64));
            let mut bytes = serde_json::to_vec(&v).unwrap();
            bytes.push(b'\n');
            fs::write(&path, bytes).unwrap();
        } else {
            rewrite_ledger(&path, |v| match kind {
                1 => {
                    let e = v["ledger"]["entries"][0].clone();
                    v["ledger"]["entries"].as_array_mut().unwrap().push(e);
                }
                2 => v["ledger"]["entries"][0]["response"]["schema_version"] = json!(2),
                3 => v["ledger"]["entries"][0]["response"]["ok"] = json!(false),
                4 => v["ledger"]["entries"][0]["response"]["state"]["promotable"] = json!(true),
                5 => {
                    v["ledger"]["entries"][0]["response"]["state"]["identity"]["binary_sha256"] =
                        json!("d".repeat(64))
                }
                6 => v["ledger"]["entries"][0]["response"]["error_code"] = json!("unknown"),
                _ => v["ledger"]["entries"][0]["request"]["schema_version"] = json!(2),
            });
        }
        assert!(handle(
            dir.path(),
            &request(2, DiagnosticOperation::Status, 1),
            1,
            &mut b
        )
        .is_err());
        assert_eq!((b.starts, b.stops), (0, 0));
    }
}

#[test]
fn foreign_or_missing_lease_and_reversed_clock_never_mutate() {
    let dir = tempfile::tempdir().unwrap();
    let mut b = Backend::default();
    assert!(matches!(
        handle(
            dir.path(),
            &request(2, DiagnosticOperation::Heartbeat, 1),
            0,
            &mut b
        ),
        Err(DiagnosticIpcError::Binding)
    ));
    assert!(!dir.path().join(LEDGER_NAME).exists());
    let (dir, mut b) = fixture();
    let h = request(2, DiagnosticOperation::Heartbeat, 1);
    handle(dir.path(), &h, 2, &mut b).unwrap();
    let next = request(3, DiagnosticOperation::Heartbeat, 2);
    assert!(matches!(
        handle(dir.path(), &next, 1, &mut b),
        Err(DiagnosticIpcError::Revision)
    ));
    let mut boot = clock(3);
    boot.boot_id = "00000000-0000-4000-8000-000000000075".into();
    assert!(matches!(
        handle_local(
            dir.path(),
            &packet(&next),
            &peer(),
            &policy(),
            110,
            &boot,
            &mut b
        ),
        Err(DiagnosticIpcError::Binding)
    ));
    let mut takeover = next;
    takeover.controller.actor_id = 5;
    let mut p = policy();
    p.actor_ids.push(5);
    assert!(matches!(
        handle_local(
            dir.path(),
            &packet(&takeover),
            &peer(),
            &p,
            110,
            &clock(3),
            &mut b
        ),
        Err(DiagnosticIpcError::Binding)
    ));
}

#[test]
fn campaign_claim_refuses_diagnostic_reserve_without_backend_calls() {
    use hydracache_long_run_supervisor_074::host_execution::HostExecutionClaim;
    let dir = tempfile::tempdir().unwrap();
    let mut b = Backend::default();
    let claim = HostExecutionClaim::acquire(dir.path(), &"d".repeat(64)).unwrap();
    assert!(matches!(
        handle(
            dir.path(),
            &request(1, DiagnosticOperation::Reserve, 0),
            0,
            &mut b
        ),
        Err(DiagnosticIpcError::Host(_))
    ));
    drop(claim);
    let r = handle(
        dir.path(),
        &request(1, DiagnosticOperation::Reserve, 0),
        0,
        &mut b,
    )
    .unwrap();
    assert!(!r.ok);
    assert!(!dir.path().join(ACTIVE_DIAGNOSTIC_NAME).exists());
    assert_eq!((b.starts, b.stops), (0, 0));
}

#[cfg(target_os = "linux")]
#[test]
fn linked_ledger_is_refused_and_mode_is_private() {
    use std::os::unix::fs::PermissionsExt;
    let (dir, mut b) = fixture();
    let ledger = dir.path().join(LEDGER_NAME);
    assert_eq!(
        fs::metadata(&ledger).unwrap().permissions().mode() & 0o777,
        0o600
    );
    fs::hard_link(&ledger, dir.path().join("linked.json")).unwrap();
    assert!(matches!(
        handle(
            dir.path(),
            &request(2, DiagnosticOperation::Heartbeat, 1),
            1,
            &mut b
        ),
        Err(DiagnosticIpcError::Ledger)
    ));
    fs::remove_file(&ledger).unwrap();
    std::os::unix::fs::symlink(dir.path().join("linked.json"), &ledger).unwrap();
    assert!(matches!(
        handle(
            dir.path(),
            &request(2, DiagnosticOperation::Heartbeat, 1),
            1,
            &mut b
        ),
        Err(DiagnosticIpcError::Ledger)
    ));
}

#[cfg(target_os = "linux")]
#[test]
fn local_seqpacket_denies_non_allowlisted_kernel_peer_without_marker() {
    use hydracache_long_run_supervisor_074::unix_transport::{
        SeqpacketConnection, SeqpacketListener,
    };
    // SAFETY: getgid has no arguments or pointer lifetime requirements.
    let gid = unsafe { libc::getgid() };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("deny.sock");
    let listener = SeqpacketListener::bind(&path, 0o600, gid).unwrap();
    let mut p = policy();
    p.client_uids.clear();
    std::thread::scope(|scope| {
        let client = scope.spawn(|| {
            let connection = SeqpacketConnection::connect(&path).unwrap();
            connection
                .send_packet(&packet(&request(1, DiagnosticOperation::Reserve, 0)))
                .unwrap();
        });
        let connection = listener
            .accept_timeout(std::time::Duration::from_secs(1))
            .unwrap()
            .unwrap();
        assert!(matches!(
            handle_local_connection(
                dir.path(),
                &connection,
                &p,
                110,
                &clock(0),
                &mut Backend::default()
            ),
            Err(DiagnosticIpcError::Principal)
        ));
        client.join().unwrap();
    });
    assert!(!dir.path().join(LEDGER_NAME).exists());
    assert!(!dir.path().join(ACTIVE_DIAGNOSTIC_NAME).exists());
}

#[cfg(target_os = "linux")]
#[test]
fn local_seqpacket_uses_kernel_peer_credentials_without_production_route() {
    use hydracache_long_run_supervisor_074::unix_transport::{
        SeqpacketConnection, SeqpacketListener,
    };
    // SAFETY: getuid/getgid have no arguments or pointer lifetime requirements.
    let (uid, gid) = unsafe { (libc::getuid(), libc::getgid()) };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("diag.sock");
    let listener = SeqpacketListener::bind(&path, 0o600, gid).unwrap();
    let mut p = policy();
    p.client_uids = vec![uid];
    p.client_gid = gid;
    std::thread::scope(|scope| {
        let client = scope.spawn(|| {
            let connection = SeqpacketConnection::connect(&path).unwrap();
            connection
                .send_packet(&packet(&request(1, DiagnosticOperation::Reserve, 0)))
                .unwrap();
            let bytes = connection
                .receive_packet_timeout(std::time::Duration::from_secs(1))
                .unwrap()
                .unwrap();
            serde_json::from_slice::<DiagnosticResponse>(&bytes).unwrap()
        });
        let connection = listener
            .accept_timeout(std::time::Duration::from_secs(1))
            .unwrap()
            .unwrap();
        let mut b = Backend::default();
        let response =
            handle_local_connection(dir.path(), &connection, &p, 110, &clock(0), &mut b).unwrap();
        assert_eq!(client.join().unwrap(), response);
        assert_eq!((b.starts, b.stops), (0, 0));
    });
}
