//! Synthetic signer/build/context bytes only; no actual compilation or custody.
use ed25519_dalek::{Signer, SigningKey};
use hydracache_long_run_supervisor_074::{
    diagnostic_artifacts::{self, ArtifactDigest},
    diagnostic_builder,
    diagnostic_lease::DiagnosticIdentity,
    diagnostic_reviewed_trust as reviewed,
    diagnostic_signer_build::*,
};

fn canonical<T: serde::Serialize>(value: &T) -> Vec<u8> {
    serde_json::to_vec(&serde_json::to_value(value).unwrap()).unwrap()
}

fn lf<T: serde::Serialize>(value: &T) -> Vec<u8> {
    let mut bytes = canonical(value);
    bytes.push(b'\n');
    bytes
}

// Deliberately test-local, not a production statement signing API.
fn signed(statement: &SignerBuildStatement, key: &SigningKey, domain: &[u8]) -> Vec<u8> {
    let mut message = domain.to_vec();
    message.push(0);
    message.extend(canonical(statement));
    lf(&SignedSignerBuild {
        statement: statement.clone(),
        signature_hex: key
            .sign(&message)
            .to_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
    })
}

struct Fixture {
    key: SigningKey,
    statement: SignerBuildStatement,
    bytes: [Vec<u8>; 5],
}

impl Fixture {
    fn new() -> Self {
        let bytes = [
            b"not ELF".to_vec(),
            b"synthetic lock".to_vec(),
            b"invented log".to_vec(),
            b"opaque runtime assertion".to_vec(),
            b"opaque bootstrap assertion".to_vec(),
        ];
        let statement = SignerBuildStatement {
            schema_version: SCHEMA.into(),
            repository_id: diagnostic_builder::REPOSITORY_ID,
            role: ROLE.into(),
            source_commit: "a".repeat(40),
            source_tree: "b".repeat(40),
            source_clean_before: true,
            source_clean_after: true,
            rustc_version: RUSTC_VERSION.into(),
            cargo_version: CARGO_VERSION.into(),
            target: "x86_64-unknown-linux-gnu".into(),
            profile: "release".into(),
            features: vec![],
            allocator: "System".into(),
            counting_allocator: false,
            build_command: BUILD_COMMAND.iter().map(|s| s.to_string()).collect(),
            binary: ArtifactDigest::of(&bytes[0]),
            root_lock: ArtifactDigest::of(&bytes[1]),
            build_log: ArtifactDigest::of(&bytes[2]),
            runtime_manifest: ArtifactDigest::of(&bytes[3]),
            bootstrap_context: ArtifactDigest::of(&bytes[4]),
        };
        Self {
            key: SigningKey::from_bytes(&[91; 32]),
            statement,
            bytes,
        }
    }
    fn selection(&self) -> AssertedSignerBuildSelection {
        AssertedSignerBuildSelection::new(self.key.verifying_key(), self.statement.clone()).unwrap()
    }
    fn contents(&self) -> SignerBuildContents<'_> {
        SignerBuildContents {
            binary: &self.bytes[0],
            root_lock: &self.bytes[1],
            build_log: &self.bytes[2],
            runtime_manifest: &self.bytes[3],
            bootstrap_context: &self.bytes[4],
        }
    }
    fn receipt(&self) -> Vec<u8> {
        signed(&self.statement, &self.key, SIGNATURE_DOMAIN)
    }
}

#[test]
fn synthetic_signature_and_supplied_bytes_match_without_attesting_build_or_runtime() {
    let f = Fixture::new();
    let raw = f.receipt();
    let checked = f.selection().verify_supplied(&raw, &f.contents()).unwrap();
    assert_eq!(checked.receipt_sha256(), ArtifactDigest::of(&raw).sha256);
    // Non-ELF executable and opaque context are intentional: this proves matching
    // supplied bytes, not mapped executable provenance or authenticated bootstrap.
    assert_eq!(f.bytes[0], b"not ELF");
}

#[test]
fn old_reviewed_key_roles_and_weak_issuer_cannot_select_signer_build_purpose() {
    let f = Fixture::new();
    for hex in [
        reviewed::CONTROLLER_KEY_HEX,
        reviewed::BUILDER_KEY_HEX,
        reviewed::WORKER_ISSUER_KEY_HEX,
        "0100000000000000000000000000000000000000000000000000000000000000",
    ] {
        let key = diagnostic_builder::decode_key(hex).unwrap();
        assert!(AssertedSignerBuildSelection::new(key, f.statement.clone()).is_err());
    }
}

#[test]
fn every_statement_field_is_selected_and_even_valid_resigning_cannot_change_it() {
    let f = Fixture::new();
    let selection = f.selection();
    let original = serde_json::to_value(&f.statement).unwrap();
    for name in original.as_object().unwrap().keys() {
        let mut changed = original.clone();
        changed[name] = match &original[name] {
            serde_json::Value::String(s) => serde_json::json!(format!("{s}x")),
            serde_json::Value::Bool(b) => serde_json::json!(!b),
            serde_json::Value::Number(_) => serde_json::json!(0),
            serde_json::Value::Array(_) => serde_json::json!(["unexpected"]),
            serde_json::Value::Object(_) => serde_json::json!({"sha256":"c".repeat(64), "bytes":1}),
            _ => panic!("unexpected fixture type"),
        };
        let statement: SignerBuildStatement = serde_json::from_value(changed).unwrap();
        assert!(
            selection
                .verify_supplied(&signed(&statement, &f.key, SIGNATURE_DOMAIN), &f.contents())
                .is_err(),
            "{name}"
        );
    }
    for name in ["source_commit", "source_tree"] {
        let mut changed = original.clone();
        changed[name] = serde_json::json!("c".repeat(40));
        let statement = serde_json::from_value(changed).unwrap();
        assert!(selection
            .verify_supplied(&signed(&statement, &f.key, SIGNATURE_DOMAIN), &f.contents())
            .is_err());
    }
}

#[test]
fn malformed_selection_profile_source_and_artifact_limits_refuse_before_verification() {
    let f = Fixture::new();
    let original = serde_json::to_value(&f.statement).unwrap();
    for (field, bad) in [
        (
            "schema_version",
            serde_json::json!("diagnostic-signer-build-074-v2"),
        ),
        ("repository_id", serde_json::json!(0)),
        ("role", serde_json::json!("observer")),
        ("source_commit", serde_json::json!("A".repeat(40))),
        ("source_tree", serde_json::json!("0".repeat(39))),
        ("source_clean_before", serde_json::json!(false)),
        ("source_clean_after", serde_json::json!(false)),
        ("rustc_version", serde_json::json!("rustc unknown")),
        ("cargo_version", serde_json::json!("cargo unknown")),
        ("target", serde_json::json!("other")),
        ("profile", serde_json::json!("debug")),
        ("features", serde_json::json!(["profiling"])),
        ("allocator", serde_json::json!("other")),
        ("counting_allocator", serde_json::json!(true)),
        ("build_command", serde_json::json!([])),
    ] {
        let mut changed = original.clone();
        changed[field] = bad;
        let statement = serde_json::from_value(changed).unwrap();
        assert!(
            AssertedSignerBuildSelection::new(f.key.verifying_key(), statement).is_err(),
            "{field}"
        );
    }
    for (field, limit) in [
        ("binary", MAX_BINARY_BYTES),
        ("root_lock", MAX_LOCK_BYTES),
        ("build_log", MAX_LOG_BYTES),
        ("runtime_manifest", MAX_CONTEXT_BYTES),
        ("bootstrap_context", MAX_CONTEXT_BYTES),
    ] {
        for bytes in [0, limit + 1, u64::MAX] {
            let mut changed = original.clone();
            changed[field]["bytes"] = serde_json::json!(bytes);
            let statement = serde_json::from_value(changed).unwrap();
            assert!(AssertedSignerBuildSelection::new(f.key.verifying_key(), statement).is_err());
        }
        for hash in ["".into(), "A".repeat(64), "a".repeat(63), "g".repeat(64)] {
            let mut changed = original.clone();
            changed[field]["sha256"] = serde_json::json!(hash);
            let statement = serde_json::from_value(changed).unwrap();
            assert!(AssertedSignerBuildSelection::new(f.key.verifying_key(), statement).is_err());
        }
    }
}

#[test]
fn wrong_issuer_domain_and_all_signature_positions_refuse() {
    let f = Fixture::new();
    let selection = f.selection();
    let other = SigningKey::from_bytes(&[92; 32]);
    for domain in [
        artifact_domain(),
        b"hydracache-diagnostic-controller-074-v1",
        b"",
        SIGNATURE_DOMAIN,
    ] {
        let key = if domain == SIGNATURE_DOMAIN {
            &other
        } else {
            &f.key
        };
        assert!(selection
            .verify_supplied(&signed(&f.statement, key, domain), &f.contents())
            .is_err());
    }
    let original: SignedSignerBuild = serde_json::from_slice(&f.receipt()).unwrap();
    for position in 0..128 {
        let mut changed = original.clone();
        let mut signature = changed.signature_hex.into_bytes();
        signature[position] = if signature[position] == b'0' {
            b'1'
        } else {
            b'0'
        };
        changed.signature_hex = String::from_utf8(signature).unwrap();
        assert!(selection
            .verify_supplied(&lf(&changed), &f.contents())
            .is_err());
    }
    for bad in ["".into(), "a".repeat(127), "A".repeat(128), "g".repeat(128)] {
        let mut changed = original.clone();
        changed.signature_hex = bad;
        assert!(selection
            .verify_supplied(&lf(&changed), &f.contents())
            .is_err());
    }
}
fn artifact_domain() -> &'static [u8] {
    diagnostic_artifacts::SIGNATURE_DOMAIN
}

#[test]
fn strict_json_canonical_lf_schema_duplicates_and_budget_refuse() {
    let f = Fixture::new();
    let raw = f.receipt();
    let selection = f.selection();
    let original: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    let mut unknown = original.clone();
    unknown["authority"] = serde_json::json!(true);
    let mut nested = original.clone();
    nested["statement"]["binary"]["path"] = serde_json::json!("/tmp/untrusted");
    let mut missing = original.clone();
    missing["statement"].as_object_mut().unwrap().remove("role");
    let mut wrong_type = original.clone();
    wrong_type["statement"]["source_clean_before"] = serde_json::json!(1);
    let text = std::str::from_utf8(&raw).unwrap();
    for bytes in [
        vec![],
        vec![b' '; MAX_RECEIPT_BYTES + 1],
        raw[..raw.len() - 1].to_vec(),
        [raw.as_slice(), b"\n"].concat(),
        text.replace('\n', "\r\n").into_bytes(),
        serde_json::to_vec_pretty(&original).unwrap(),
        lf(&unknown),
        lf(&nested),
        lf(&missing),
        lf(&wrong_type),
        text.replacen(
            "\"role\":",
            "\"role\":\"diagnostic-controller-signer\",\"role\":",
            1,
        )
        .into_bytes(),
        text.replacen(
            "\"signature_hex\":",
            "\"signature_hex\":\"x\",\"signature_hex\":",
            1,
        )
        .into_bytes(),
        text.replacen("\"sha256\":", "\"sha256\":\"x\",\"sha256\":", 1)
            .into_bytes(),
    ] {
        assert!(selection.verify_supplied(&bytes, &f.contents()).is_err());
    }
}

#[test]
fn each_supplied_artifact_requires_exact_nonempty_bytes_and_length() {
    let mut f = Fixture::new();
    let raw = f.receipt();
    let selection = f.selection();
    for index in 0..5 {
        let original = f.bytes[index].clone();
        for bytes in [
            vec![],
            [original.as_slice(), b"x"].concat(),
            vec![b'x'; original.len()],
        ] {
            f.bytes[index] = bytes;
            assert!(
                selection.verify_supplied(&raw, &f.contents()).is_err(),
                "{index}"
            );
        }
        f.bytes[index] = original;
        selection.verify_supplied(&raw, &f.contents()).unwrap();
    }
}

#[test]
fn old_observer_and_new_signer_receipts_have_no_shared_reader_or_role() {
    const POLICY: &[u8] = include_bytes!(
        "../../../docs/testing/performance/0.74/diagnostic-builder-public-policy.json"
    );
    const OLD: &[u8] = include_bytes!("../../../docs/testing/performance/0.74/local-runs/diagnostic-builder-hosted-37923423889/build-receipt-v1.json");
    let f = Fixture::new();
    let new = f.receipt();
    assert!(f.selection().verify_supplied(OLD, &f.contents()).is_err());
    let identity = |raw: &[u8]| DiagnosticIdentity {
        lease_id: "1".repeat(64),
        boot_id: "00000000-0000-0000-0000-000000000000".into(),
        binary_sha256: "bda1c094f3de7c6d42ec45c757838c7a41a8ff1d63a025c661af84455169a101".into(),
        build_provenance_sha256: ArtifactDigest::of(raw).sha256,
    };
    let old = reviewed::load_builder_policy(POLICY).unwrap();
    old.verify_receipt(OLD, &identity(OLD)).unwrap();
    assert!(old.verify_receipt(&new, &identity(&new)).is_err());
}

#[test]
fn seeded_content_corruption_and_four_independent_concurrent_models() {
    let mut f = Fixture::new();
    let selection = f.selection();
    let raw = f.receipt();
    let mut seed = 0x0740_3b22u64;
    eprintln!(
        "seed=0x07403b22 content_mutations=64 independent_models=4 actual_build_or_bootstrap=false"
    );
    for _ in 0..64 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let index = seed as usize % 5;
        let position = (seed >> 8) as usize % f.bytes[index].len();
        f.bytes[index][position] ^= 1;
        assert!(selection.verify_supplied(&raw, &f.contents()).is_err());
        f.bytes[index][position] ^= 1;
        selection.verify_supplied(&raw, &f.contents()).unwrap();
    }
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|_| {
                scope.spawn(|| {
                    let f = Fixture::new();
                    let selection = f.selection();
                    selection
                        .verify_supplied(&f.receipt(), &f.contents())
                        .unwrap();
                    assert!(selection.verify_supplied(b"{}", &f.contents()).is_err());
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
    });
}
