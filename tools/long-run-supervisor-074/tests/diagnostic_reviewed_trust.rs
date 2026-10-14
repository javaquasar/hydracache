//! Retained public production builder signature; synthetic lease is not admission.
use hydracache_long_run_supervisor_074::{
    diagnostic_artifacts::{ArtifactDigest, SignedBuild},
    diagnostic_builder::{self, BuilderPolicy},
    diagnostic_ipc::DiagnosticPolicy,
    diagnostic_lease::DiagnosticIdentity,
    diagnostic_reviewed_trust::{
        self as reviewed, BUILDER_KEY_HEX, BUILDER_POLICY_SHA256, CONTROLLER_KEY_HEX,
    },
};

const POLICY: &[u8] =
    include_bytes!("../../../docs/testing/performance/0.74/diagnostic-builder-public-policy.json");
const RECEIPT: &[u8] = include_bytes!("../../../docs/testing/performance/0.74/local-runs/diagnostic-builder-hosted-37923423889/build-receipt-v1.json");

fn identity(bytes: &[u8]) -> DiagnosticIdentity {
    DiagnosticIdentity {
        lease_id: "1".repeat(64),
        boot_id: "00000000-0000-0000-0000-000000000000".into(),
        binary_sha256: "bda1c094f3de7c6d42ec45c757838c7a41a8ff1d63a025c661af84455169a101".into(),
        build_provenance_sha256: ArtifactDigest::of(bytes).sha256,
    }
}

#[test]
fn reviewed_trust_existing_public_policy_matches_independent_pins() {
    assert_eq!(
        BUILDER_POLICY_SHA256,
        "dfe81cf770088029e03f04faa4da429ef163583c0ba1b7002b8ec5606e766e22"
    );
    assert_eq!(
        CONTROLLER_KEY_HEX,
        "c56f89595c45d0656225f1b8915babed046d278dc9ce1908c6bcb6e9c93e2fda"
    );
    assert_eq!(
        BUILDER_KEY_HEX,
        "39215088b0102dffb987b937efa9f49a6c5c7704192fa1f16b4e19ccad12c3ba"
    );
    assert_ne!(BUILDER_KEY_HEX, CONTROLLER_KEY_HEX);
    assert_eq!(ArtifactDigest::of(POLICY).sha256, BUILDER_POLICY_SHA256);
    reviewed::load_builder_policy(POLICY).unwrap();
}

#[test]
fn reviewed_trust_retained_hosted_receipt_signature_verifies_without_execution() {
    let policy = reviewed::load_builder_policy(POLICY).unwrap();
    let build = policy.verify_receipt(RECEIPT, &identity(RECEIPT)).unwrap();
    assert_eq!(
        build.statement().repository_id,
        diagnostic_builder::REPOSITORY_ID
    );
    assert_eq!(
        build.statement().source_commit,
        "62114be0f5da3218706e30d7424acfb5d0579d07"
    );
    // No executable bytes, content verification, current key status or host enrollment claimed.
}

#[test]
fn reviewed_trust_policy_mutations_and_shared_keys_never_choose_trust() {
    for offset in 0..POLICY.len() {
        let mut changed = POLICY.to_vec();
        changed[offset] ^= 1;
        assert!(reviewed::load_builder_policy(&changed).is_err());
    }
    let original: BuilderPolicy = serde_json::from_slice(POLICY).unwrap();
    for case in 0..5 {
        let mut changed = original.clone();
        match case {
            0 => changed.builder_key_hex = CONTROLLER_KEY_HEX.into(),
            1 => changed.controller_key_hex = BUILDER_KEY_HEX.into(),
            2 => changed.repository_id += 1,
            3 => changed.schema_version.push_str("-future"),
            _ => {
                changed.builder_key_hex = ed25519_dalek::SigningKey::from_bytes(&[71; 32])
                    .verifying_key()
                    .to_bytes()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect()
            }
        }
        assert!(reviewed::load_builder_policy(
            &diagnostic_builder::policy_bytes(&changed).unwrap()
        )
        .is_err());
    }
}

#[test]
fn reviewed_trust_empty_oversized_noncanonical_and_future_fields_refuse() {
    let mut unknown: serde_json::Value = serde_json::from_slice(POLICY).unwrap();
    unknown["worker_issuer"] = serde_json::json!(CONTROLLER_KEY_HEX);
    for bytes in [
        vec![],
        vec![b' '; diagnostic_builder::MAX_POLICY_BYTES + 1],
        POLICY[..POLICY.len() - 1].to_vec(),
        [POLICY, b"\n"].concat(),
        serde_json::to_vec_pretty(&unknown).unwrap(),
    ] {
        assert!(reviewed::load_builder_policy(&bytes).is_err());
    }
}

#[test]
fn reviewed_trust_signature_mutation_refuses_even_with_recomputed_receipt_pin() {
    let policy = reviewed::load_builder_policy(POLICY).unwrap();
    let receipt: SignedBuild = serde_json::from_slice(RECEIPT).unwrap();
    for offset in 0..128 {
        let mut changed = receipt.clone();
        let mut signature = changed.signature_hex.into_bytes();
        signature[offset] = if signature[offset] == b'0' {
            b'1'
        } else {
            b'0'
        };
        changed.signature_hex = String::from_utf8(signature).unwrap();
        let bytes =
            hydracache_long_run_supervisor_074::diagnostic_artifacts::receipt_bytes(&changed)
                .unwrap();
        assert!(policy.verify_receipt(&bytes, &identity(&bytes)).is_err());
    }
    let mut wrong = identity(RECEIPT);
    wrong.binary_sha256 = "0".repeat(64);
    assert!(policy.verify_receipt(RECEIPT, &wrong).is_err());
    wrong = identity(RECEIPT);
    wrong.build_provenance_sha256 = "0".repeat(64);
    assert!(policy.verify_receipt(RECEIPT, &wrong).is_err());
}

#[test]
fn reviewed_trust_controller_check_requires_exact_repository_and_key() {
    let mut policy = DiagnosticPolicy {
        repository_id: diagnostic_builder::REPOSITORY_ID,
        actor_ids: vec![],
        client_uids: vec![],
        client_gid: 0,
        verifying_key: reviewed::controller_key().unwrap(),
    };
    // This checks key selection only; empty allowlists cannot admit any request.
    reviewed::require_controller_pin(&policy).unwrap();
    for key in [
        BUILDER_KEY_HEX,
        "0100000000000000000000000000000000000000000000000000000000000000",
    ] {
        policy.verifying_key = diagnostic_builder::decode_key(key).unwrap();
        assert!(reviewed::require_controller_pin(&policy).is_err());
    }
    policy.verifying_key = reviewed::controller_key().unwrap();
    for repository in [0, diagnostic_builder::REPOSITORY_ID - 1, u64::MAX] {
        policy.repository_id = repository;
        assert!(reviewed::require_controller_pin(&policy).is_err());
    }
}

#[test]
fn reviewed_trust_independent_concurrent_verifiers_keep_fixed_pins() {
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..8)
            .map(|_| {
                scope.spawn(|| {
                    let policy = reviewed::load_builder_policy(POLICY).unwrap();
                    policy.verify_receipt(RECEIPT, &identity(RECEIPT)).unwrap();
                    assert!(reviewed::load_builder_policy(b"{}").is_err());
                    policy.verify_receipt(RECEIPT, &identity(RECEIPT)).unwrap();
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
    });
}
