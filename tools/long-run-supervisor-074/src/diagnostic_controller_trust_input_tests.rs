//! Synthetic signers/nonroot roots and real local kernel messages, never enrollment.
use super::*;
use crate::diagnostic_ipc::{
    self, DiagnosticController, DiagnosticOperation, SignedDiagnosticRequest,
};
use crate::diagnostic_lease::{DiagnosticIdentity, SOURCE_COMMIT};
use ed25519_dalek::{Signer, SigningKey};
use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::TempDir;

fn uid() -> u32 {
    unsafe { libc::geteuid() }
}
fn gid() -> u32 {
    unsafe { libc::getegid() }
}
fn line(value: &impl Serialize) -> Vec<u8> {
    let mut raw = canonical_json(value).unwrap();
    raw.push(b'\n');
    raw
}
pub(super) struct Model {
    root: TempDir,
    pub(super) key: SigningKey,
    document: ControllerDocument,
}
impl Model {
    pub(super) fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        fs::create_dir(root.path().join("etc")).unwrap();
        fs::create_dir(root.path().join("etc/hydracache-perf")).unwrap();
        let key = SigningKey::from_bytes(&[76; 32]);
        let document = ControllerDocument {
            schema_version: CONTROLLER_INPUT_SCHEMA.into(),
            repository_id: crate::diagnostic_builder::REPOSITORY_ID,
            controller_key_hex: crate::hex(&key.verifying_key().to_bytes()),
            actor_ids: vec![42],
            client_uids: vec![uid()],
            client_gid: gid(),
        };
        let model = Self {
            root,
            key,
            document,
        };
        model.write();
        model
    }
    pub(super) fn path(&self) -> PathBuf {
        self.root
            .path()
            .join("etc/hydracache-perf")
            .join(CONTROLLER_FILE)
    }
    fn write(&self) {
        fs::write(self.path(), line(&self.document)).unwrap();
        fs::set_permissions(self.path(), fs::Permissions::from_mode(0o600)).unwrap();
    }
    pub(super) fn input(&self) -> ControllerTrustInput {
        ControllerTrustInput {
            source: ControllerSource::open(self.root.path(), uid(), &self.key.verifying_key())
                .unwrap(),
            refused: false,
            _original_thread: PhantomData,
        }
    }
    pub(super) fn request(&self) -> DiagnosticRequest {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        DiagnosticRequest {
            schema_version: 1,
            request_id: "a".repeat(64),
            nonce_sha256: "b".repeat(64),
            operation: DiagnosticOperation::Reserve,
            expected_state_revision: 0,
            identity: DiagnosticIdentity {
                lease_id: "c".repeat(64),
                boot_id: fs::read_to_string("/proc/sys/kernel/random/boot_id")
                    .unwrap()
                    .trim_end_matches('\n')
                    .into(),
                binary_sha256: "d".repeat(64),
                build_provenance_sha256: "e".repeat(64),
            },
            source_commit: SOURCE_COMMIT.into(),
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
    pub(super) fn sign(&self, request: DiagnosticRequest, key: &SigningKey) -> Vec<u8> {
        let signature = key.sign(&diagnostic_ipc::signing_message(&request).unwrap());
        line(&SignedDiagnosticRequest {
            request,
            signature_hex: crate::hex(&signature.to_bytes()),
        })
    }
}
#[test]
fn controller_trust_strict_schema_principals_key_and_budget() {
    let model = Model::new();
    let raw = line(&model.document);
    decode_controller(&raw, &model.key.verifying_key()).unwrap();
    assert!(decode_controller(
        &raw,
        &crate::diagnostic_reviewed_trust::controller_key().unwrap()
    )
    .is_err());
    for field in [
        "schema_version",
        "repository_id",
        "controller_key_hex",
        "actor_ids",
        "client_uids",
        "client_gid",
    ] {
        let value: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        let mut missing = value.clone();
        missing.as_object_mut().unwrap().remove(field);
        let mut null = value.clone();
        null[field] = serde_json::Value::Null;
        let duplicate = format!(
            "{{\"{field}\":{},{}",
            value[field],
            String::from_utf8(raw.clone())
                .unwrap()
                .trim_start_matches('{')
        );
        for bad in [line(&missing), line(&null), duplicate.into_bytes()] {
            assert!(decode_controller(&bad, &model.key.verifying_key()).is_err());
        }
    }
    for (field, value) in [
        ("schema_version", serde_json::json!("future")),
        ("repository_id", serde_json::json!(17)),
        ("unknown", serde_json::json!(true)),
        ("actor_ids", serde_json::json!([])),
        ("actor_ids", serde_json::json!([0])),
        ("actor_ids", serde_json::json!([42, 42])),
        ("actor_ids", serde_json::json!([43, 42])),
        ("client_uids", serde_json::json!([])),
        ("client_uids", serde_json::json!([0])),
        ("client_uids", serde_json::json!([u32::MAX])),
        ("client_uids", serde_json::json!([2, 1])),
        ("client_gid", serde_json::json!(0)),
        ("client_gid", serde_json::json!(u32::MAX)),
        (
            "controller_key_hex",
            serde_json::json!(crate::diagnostic_reviewed_trust::WORKER_ISSUER_KEY_HEX),
        ),
        (
            "actor_ids",
            serde_json::json!((1..=33).collect::<Vec<u64>>()),
        ),
        (
            "client_uids",
            serde_json::json!((1..=33).collect::<Vec<u32>>()),
        ),
    ] {
        let mut bad: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        bad[field] = value;
        assert!(decode_controller(&line(&bad), &model.key.verifying_key()).is_err());
    }
    for bad in [
        vec![],
        vec![b'x'; MAX_TRUST_BYTES + 1],
        raw[..raw.len() - 1].to_vec(),
        [raw.clone(), b" ".to_vec()].concat(),
    ] {
        assert!(decode_controller(&bad, &model.key.verifying_key()).is_err());
    }
    let mut exact: ControllerDocument = serde_json::from_slice(&raw).unwrap();
    exact.actor_ids = (1..=32).collect();
    exact.client_uids = (1..=32).collect();
    decode_controller(&line(&exact), &model.key.verifying_key()).unwrap();
}
#[test]
fn controller_trust_signed_roundtrip_retains_originals_and_healthy_drop() {
    let model = Model::new();
    let request = model.request();
    let expected = crate::sha256_hex(&canonical_json(&request).unwrap());
    let bytes = model.sign(request, &model.key);
    let (_root, _authority_root, mut source, mut authority, worker_bytes) =
        super::super::tests::controller_worker_fixture();
    let mut packet = super::super::tests::kernel_packet_bytes(&bytes);
    let mut worker = source
        .verify_kernel_policy(&worker_bytes, &mut packet, &mut authority)
        .unwrap();
    let mut controller = model.input();
    {
        let mut guard = controller.authenticate_controller(&mut worker).unwrap();
        assert_eq!(guard.request_sha256(), expected);
        for _ in 0..3 {
            guard.revalidate().unwrap();
        }
        assert!(!guard.is_refused());
    }
    assert!(!controller.is_refused() && !worker.is_refused());
    controller
        .authenticate_controller(&mut worker)
        .unwrap()
        .revalidate()
        .unwrap();
    drop(worker);
    assert!(!source.is_refused() && !authority.is_failed() && !packet.is_refused());
}
#[test]
fn controller_trust_signature_actor_uid_gid_time_and_boot_refusals_cross_latch() {
    for case in 0..8 {
        let mut model = Model::new();
        let mut request = model.request();
        match case {
            0 => request.controller.actor_id = 43,
            1 => model.document.client_uids = vec![u32::MAX - 1],
            2 => model.document.client_gid = u32::MAX - 1,
            3 => {
                request.issued_at_unix_seconds += 3600;
                request.expires_at_unix_seconds += 3600;
            }
            4 => {
                request.issued_at_unix_seconds -= 120;
                request.expires_at_unix_seconds -= 120;
            }
            5 => request.identity.boot_id = "11111111-2222-3333-4444-555555555555".into(),
            6 => request.controller.repository_id = 17,
            _ => (),
        }
        model.write();
        let key = if case == 7 {
            SigningKey::from_bytes(&[77; 32])
        } else {
            model.key.clone()
        };
        let bytes = model.sign(request, &key);
        let (_root, _authority_root, mut source, mut authority, worker_bytes) =
            super::super::tests::controller_worker_fixture();
        let mut packet = super::super::tests::kernel_packet_bytes(&bytes);
        let mut worker = source
            .verify_kernel_policy(&worker_bytes, &mut packet, &mut authority)
            .unwrap();
        let mut controller = model.input();
        assert!(controller.authenticate_controller(&mut worker).is_err());
        assert!(controller.is_refused() && worker.is_refused());
        assert!(matches!(
            controller.authenticate_controller(&mut worker),
            Err(TrustInputError::Refused)
        ));
        drop(worker);
        assert!(source.is_refused() && authority.is_failed() && packet.is_refused());
    }
}
#[test]
fn controller_trust_source_worker_and_original_request_faults_stay_refused() {
    for case in 0..6 {
        let model = Model::new();
        let bytes = model.sign(model.request(), &model.key);
        let (_root, _authority_root, mut source, mut authority, worker_bytes) =
            super::super::tests::controller_worker_fixture();
        let mut packet = super::super::tests::kernel_packet_bytes(&bytes);
        let mut worker = source
            .verify_kernel_policy(&worker_bytes, &mut packet, &mut authority)
            .unwrap();
        let mut controller = model.input();
        {
            let mut guard = controller.authenticate_controller(&mut worker).unwrap();
            match case {
                0 => fs::write(model.path(), b"changed\n").unwrap(),
                1 => {
                    fs::rename(model.path(), model.path().with_extension("old")).unwrap();
                    model.write();
                }
                2 => fs::set_permissions(model.path(), fs::Permissions::from_mode(0o400)).unwrap(),
                3 => guard.worker.latch_refusal(),
                4 => guard.request.controller.run_id += 1,
                _ => guard.controller.refused = true,
            }
            assert!(guard.revalidate().is_err());
            fs::set_permissions(model.path(), fs::Permissions::from_mode(0o600)).unwrap();
            model.write();
            assert!(matches!(guard.revalidate(), Err(TrustInputError::Refused)));
        }
        assert!(controller.is_refused() && worker.is_refused());
        drop(worker);
        assert!(source.is_refused() && authority.is_failed() && packet.is_refused());
    }
}
#[test]
fn controller_trust_constructor_uses_shared_nofollow_metadata_without_repairs() {
    for case in 0..6 {
        let model = Model::new();
        match case {
            0 => fs::set_permissions(model.path(), fs::Permissions::from_mode(0o666)).unwrap(),
            1 => {
                fs::rename(model.path(), model.path().with_extension("old")).unwrap();
                symlink(model.path().with_extension("old"), model.path()).unwrap();
            }
            2 => fs::hard_link(model.path(), model.path().with_extension("link")).unwrap(),
            3 => fs::write(model.path(), []).unwrap(),
            4 => fs::write(model.path(), vec![b'x'; MAX_TRUST_BYTES + 1]).unwrap(),
            _ => fs::set_permissions(
                model.root.path().join("etc"),
                fs::Permissions::from_mode(0o777),
            )
            .unwrap(),
        }
        assert!(
            ControllerSource::open(model.root.path(), uid(), &model.key.verifying_key()).is_err()
        );
    }
    assert!(ControllerSource::open(
        Model::new().root.path(),
        uid() + 1,
        &SigningKey::from_bytes(&[76; 32]).verifying_key()
    )
    .is_err());
}
#[test]
fn controller_trust_fixed_constructor_refuses_nonroot_and_has_exact_path() {
    assert_ne!(
        uid(),
        0,
        "local proof must not open real configuration as root"
    );
    assert_eq!(
        Path::new("/")
            .join(ANCESTORS[0])
            .join(ANCESTORS[1])
            .join(CONTROLLER_FILE),
        Path::new(FIXED_CONTROLLER_PATH)
    );
    assert!(matches!(
        open_fixed_controller_trust(),
        Err(TrustInputError::Invalid)
    ));
}

#[test]
fn controller_trust_prior_refusal_or_source_failure_cross_latches_at_construction() {
    for case in 0..3 {
        let model = Model::new();
        let bytes = model.sign(model.request(), &model.key);
        let (_root, _authority_root, mut source, mut authority, worker_bytes) =
            super::super::tests::controller_worker_fixture();
        let mut packet = super::super::tests::kernel_packet_bytes(&bytes);
        let mut worker = source
            .verify_kernel_policy(&worker_bytes, &mut packet, &mut authority)
            .unwrap();
        let mut controller = model.input();
        match case {
            0 => controller.refused = true,
            1 => worker.latch_refusal(),
            _ => fs::write(model.path(), b"changed\n").unwrap(),
        }
        assert!(controller.authenticate_controller(&mut worker).is_err());
        assert!(controller.is_refused() && worker.is_refused());
        drop(worker);
        assert!(source.is_refused() && authority.is_failed() && packet.is_refused());
    }
}
#[test]
fn controller_trust_seeded_disallowed_actors_and_independent_threads() {
    let mut seed = 0x07532074_u64;
    for _ in 0..32 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let model = Model::new();
        let mut request = model.request();
        request.controller.actor_id = 100 + seed % 1000;
        let bytes = model.sign(request, &model.key);
        let (_root, _authority_root, mut source, mut authority, worker_bytes) =
            super::super::tests::controller_worker_fixture();
        let mut packet = super::super::tests::kernel_packet_bytes(&bytes);
        let mut worker = source
            .verify_kernel_policy(&worker_bytes, &mut packet, &mut authority)
            .unwrap();
        let mut controller = model.input();
        assert!(controller.authenticate_controller(&mut worker).is_err());
        assert!(controller.is_refused() && worker.is_refused());
    }
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|_| {
                scope.spawn(|| {
                    let model = Model::new();
                    let bytes = model.sign(model.request(), &model.key);
                    let (_root, _authority_root, mut source, mut authority, worker_bytes) =
                        super::super::tests::controller_worker_fixture();
                    let mut packet = super::super::tests::kernel_packet_bytes(&bytes);
                    let mut worker = source
                        .verify_kernel_policy(&worker_bytes, &mut packet, &mut authority)
                        .unwrap();
                    model
                        .input()
                        .authenticate_controller(&mut worker)
                        .unwrap()
                        .revalidate()
                        .unwrap();
                    assert!(!worker.is_refused());
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
    });
    println!("seed=0x07532074 signed_disallowed_actors=32 independent_threads=4 production_enrollment=false");
}
