//! Selected public key only; no private key, actual build signature or runtime custody.
use ed25519_dalek::{Signature, Signer, SigningKey};
use hydracache_long_run_supervisor_074::diagnostic_signer_issuer as selected;
use hydracache_long_run_supervisor_074::{
    diagnostic_artifacts::ArtifactDigest, diagnostic_builder, diagnostic_reviewed_trust as old,
    diagnostic_signer_build::*,
};
use sha2::{Digest, Sha256};
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn decode(s: &str) -> Vec<u8> {
    s.as_bytes()
        .chunks_exact(2)
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}
fn statement() -> SignerBuildStatement {
    let digest = ArtifactDigest::of(b"synthetic bytes");
    SignerBuildStatement {
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
        binary: digest.clone(),
        root_lock: digest.clone(),
        build_log: digest.clone(),
        runtime_manifest: digest.clone(),
        bootstrap_context: digest,
    }
}

#[test]
fn selected_signer_issuer_matches_approved_raw_public_fingerprint() {
    let key = selected::issuer_key().unwrap();
    assert_eq!(hex(key.as_bytes()), selected::KEY_HEX);
    assert_eq!(
        hex(&Sha256::digest(key.as_bytes())),
        selected::RAW_PUBLIC_SHA256
    );
    for previous in [
        old::CONTROLLER_KEY_HEX,
        old::BUILDER_KEY_HEX,
        old::WORKER_ISSUER_KEY_HEX,
    ] {
        assert_ne!(selected::KEY_HEX, previous);
    }
    assert!(selected::asserted_build_selection(statement()).is_ok());
}

#[test]
fn original_public_selfcheck_is_valid_without_becoming_a_receipt() {
    let raw = include_bytes!(
        "../../../docs/testing/performance/0.74/diagnostic-signer-issuer-public.json"
    );
    let p: serde_json::Value = serde_json::from_slice(raw).unwrap();
    assert_eq!(p["public_key_hex"], selected::KEY_HEX);
    assert_eq!(p["public_key_sha256"], selected::RAW_PUBLIC_SHA256);
    assert_eq!(p["public_pin_approved"], false); // Immutable generation event, not today's selection.
    let signature =
        Signature::from_slice(&decode(p["selfcheck_signature_hex"].as_str().unwrap())).unwrap();
    let message = b"hydracache-diagnostic-signer-build-key-selfcheck-074-v1\0not-a-build-receipt\n";
    let key = selected::issuer_key().unwrap();
    key.verify_strict(message, &signature).unwrap();
    assert!(key.verify_strict(SIGNATURE_DOMAIN, &signature).is_err());
    for index in 0..64 {
        let mut changed = signature.to_bytes();
        changed[index] ^= 1;
        assert!(key
            .verify_strict(message, &Signature::from_bytes(&changed))
            .is_err());
    }
}
#[test]
fn caller_selected_synthetic_issuer_cannot_enter_selected_key_model() {
    let key = SigningKey::from_bytes(&[91; 32]);
    let s = statement();
    let canonical = serde_json::to_vec(&serde_json::to_value(&s).unwrap()).unwrap();
    let message = [SIGNATURE_DOMAIN, b"\0", &canonical].concat();
    let signed = SignedSignerBuild {
        statement: s.clone(),
        signature_hex: hex(&key.sign(&message).to_bytes()),
    };
    let mut raw = serde_json::to_vec(&serde_json::to_value(signed).unwrap()).unwrap();
    raw.push(b'\n');
    let contents = SignerBuildContents {
        binary: b"synthetic bytes",
        root_lock: b"synthetic bytes",
        build_log: b"synthetic bytes",
        runtime_manifest: b"synthetic bytes",
        bootstrap_context: b"synthetic bytes",
    };
    assert!(
        AssertedSignerBuildSelection::new(key.verifying_key(), s.clone())
            .unwrap()
            .verify_supplied(&raw, &contents)
            .is_ok()
    );
    assert!(selected::asserted_build_selection(s)
        .unwrap()
        .verify_supplied(&raw, &contents)
        .is_err());
}
#[test]
fn invalid_expected_profile_is_not_repaired_by_key_selection() {
    for fault in 0..5 {
        let mut s = statement();
        match fault {
            0 => s.role = "observer".into(),
            1 => s.allocator = "other".into(),
            2 => s.source_commit.clear(),
            3 => s.binary.bytes = MAX_BINARY_BYTES + 1,
            _ => s.counting_allocator = true,
        }
        assert!(selected::asserted_build_selection(s).is_err());
    }
}
#[test]
fn approval_record_selects_key_only_without_installing_runtime_or_job() {
    let p: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../docs/testing/performance/0.74/diagnostic-signer-issuer-selection.json"
    ))
    .unwrap();
    assert_eq!(p["public_key_hex"], selected::KEY_HEX);
    assert_eq!(p["raw_public_sha256"], selected::RAW_PUBLIC_SHA256);
    assert_eq!(p["operator_public_key_selected"], true);
    for field in [
        "artifact_selected",
        "runtime_custody_enrolled",
        "job_enrolled",
        "production_signing_enabled",
        "server_admission",
    ] {
        assert_eq!(p[field], false, "{field}");
    }
}
#[test]
fn selected_key_is_stable_under_independent_concurrent_model_creation() {
    let joins: Vec<_> = (0..4)
        .map(|_| {
            std::thread::spawn(|| {
                for _ in 0..32 {
                    assert_eq!(
                        hex(selected::issuer_key().unwrap().as_bytes()),
                        selected::KEY_HEX
                    );
                    assert!(selected::asserted_build_selection(statement()).is_ok());
                }
            })
        })
        .collect();
    for join in joins {
        join.join().unwrap();
    }
}
