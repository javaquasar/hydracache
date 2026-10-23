use super::*;
use crate::diagnostic_builder::bootstrap_build::tests::Fixture;
use std::fs;
fn line(value: &impl Serialize) -> Vec<u8> {
    let mut raw = canonical_json(value).unwrap();
    raw.push(b'\n');
    raw
}
fn key(byte: u8) -> VerifyingKey {
    ed25519_dalek::SigningKey::from_bytes(&[byte; 32]).verifying_key()
}
struct Input {
    root: tempfile::TempDir,
    build: Fixture,
}
impl Input {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("etc/hydracache-perf")).unwrap();
        let build = Fixture::new();
        let old = serde_json::json!({"schema_version":TRUST_SCHEMA,"repository_id":crate::diagnostic_builder::REPOSITORY_ID,
            "controller_key_hex":crate::hex(key(1).as_bytes()),"held_builder_key_hex":crate::hex(key(4).as_bytes()),
            "held_policy_sha256":"c".repeat(64),"worker_builder_policy_sha256":"a".repeat(64),"worker_policy_sha256":"b".repeat(64),"policy_epoch":7});
        fs::write(
            root.path().join("etc/hydracache-perf").join(TRUST_FILE),
            line(&old),
        )
        .unwrap();
        let doc = Document {
            schema_version: SCHEMA.into(),
            repository_id: crate::diagnostic_builder::REPOSITORY_ID,
            bootstrap_builder_key_hex: crate::hex(key(5).as_bytes()),
            bootstrap_policy_sha256: crate::sha256_hex(&build.policy_bytes()),
            bootstrap_sha256: build.policy.bootstrap_sha256.clone(),
            held_policy_sha256: "c".repeat(64),
            policy_epoch: 7,
        };
        fs::write(
            root.path().join("etc/hydracache-perf").join(FILE),
            line(&doc),
        )
        .unwrap();
        Self { root, build }
    }
    fn input(&self) -> HeldTrustInput {
        HeldTrustInput {
            source: HeldSource::open(
                self.root.path(),
                unsafe { libc::geteuid() },
                Selection {
                    controller: key(1),
                    worker_builder: key(2),
                    worker_issuer: key(3),
                    worker_builder_policy_sha256: "a".repeat(64),
                },
            )
            .unwrap(),
            refused: false,
            _original_thread: PhantomData,
        }
    }
    fn original(&self) -> [&str; 4] {
        let p = &self.build.policy;
        [
            &p.held_policy_sha256,
            &p.held_receipt_sha256,
            &p.original_host_sha256,
            &p.held_launcher_sha256,
        ]
    }
}
#[test]
fn bootstrap_selected_build_retains_source_and_original_binding_without_authority_export() {
    let f = Input::new();
    let mut input = f.input();
    let first = input
        .inspect_bootstrap_build(
            &f.build.policy_bytes(),
            &f.build.receipt_bytes(),
            f.original(),
            None,
        )
        .unwrap();
    input
        .inspect_bootstrap_build(
            &f.build.policy_bytes(),
            &f.build.receipt_bytes(),
            f.original(),
            Some(&first),
        )
        .unwrap();
    assert!(!input.is_refused());
}
#[test]
fn bootstrap_selected_build_each_original_digest_mismatch_poison_is_shared() {
    for at in 0..4 {
        let f = Input::new();
        let mut input = f.input();
        let mut original = f.original();
        original[at] = "0";
        assert!(input
            .inspect_bootstrap_build(
                &f.build.policy_bytes(),
                &f.build.receipt_bytes(),
                original,
                None
            )
            .is_err());
        assert!(input.is_refused());
        assert!(input
            .inspect_bootstrap_build(
                &f.build.policy_bytes(),
                &f.build.receipt_bytes(),
                f.original(),
                None
            )
            .is_err());
    }
}
#[test]
fn bootstrap_selected_build_same_bytes_replacement_cannot_reopen_retained_selection() {
    let f = Input::new();
    let mut input = f.input();
    let first = input
        .inspect_bootstrap_build(
            &f.build.policy_bytes(),
            &f.build.receipt_bytes(),
            f.original(),
            None,
        )
        .unwrap();
    let path = f.root.path().join("etc/hydracache-perf").join(FILE);
    let next = path.with_extension("next");
    fs::write(&next, fs::read(&path).unwrap()).unwrap();
    fs::rename(next, path).unwrap();
    assert!(input
        .inspect_bootstrap_build(
            &f.build.policy_bytes(),
            &f.build.receipt_bytes(),
            f.original(),
            Some(&first)
        )
        .is_err());
    assert!(input.is_refused());
}
#[test]
fn bootstrap_selected_build_signature_document_or_budget_failure_poison_is_shared() {
    for at in 0..4 {
        let f = Input::new();
        let mut input = f.input();
        let mut policy = f.build.policy_bytes();
        let mut receipt = f.build.receipt_bytes();
        match at {
            0 => policy.push(b' '),
            1 => receipt[0] = b' ',
            2 => policy = vec![b' '; crate::diagnostic_builder::MAX_POLICY_BYTES + 1],
            _ => receipt = vec![],
        };
        assert!(input
            .inspect_bootstrap_build(&policy, &receipt, f.original(), None)
            .is_err());
        assert!(input.is_refused());
    }
}
