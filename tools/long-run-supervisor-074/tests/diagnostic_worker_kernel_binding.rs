#![cfg(target_os = "linux")]
#[test]
fn signer_bundle_api_has_asserted_inputs_and_no_replacement_recheck_arguments() {
    use hydracache_long_run_supervisor_074::{
        diagnostic_artifacts::ArtifactError,
        diagnostic_signer_build::AssertedSignerBuildSelection,
        diagnostic_signer_bundle::{retain_asserted_bundle, AssertedSignerBundle},
    };
    use std::path::Path;
    let _: fn(
        &Path,
        u32,
        u32,
        AssertedSignerBuildSelection,
    ) -> Result<AssertedSignerBundle, ArtifactError> = retain_asserted_bundle;
    let _: fn(&mut AssertedSignerBundle) -> Result<&str, ArtifactError> =
        AssertedSignerBundle::check_original;
}

#[test]
fn retained_producer_public_api_consumes_preparation_not_request_paths_or_origin() {
    use hydracache_long_run_supervisor_074::diagnostic_controller_producer::{
        linux::sign_prepared_files, PreparedDiagnosticRequest, ProducerError,
    };
    use std::path::Path;
    let _: fn(PreparedDiagnosticRequest, &str, &Path, &Path) -> Result<(), ProducerError> =
        sign_prepared_files;
}

#[test]
fn producer_cli_audience_and_wrong_key_are_job_data_not_admission() {
    use hydracache_long_run_supervisor_074::diagnostic_controller_producer::prepare_request;
    use sha2::{Digest, Sha256};
    use std::os::unix::fs::PermissionsExt;
    use std::time::{SystemTime, UNIX_EPOCH};
    const POLICY: &[u8] = include_bytes!(
        "../../../docs/testing/performance/0.74/diagnostic-builder-public-policy.json"
    );
    const RECEIPT: &[u8] = include_bytes!("../../../docs/testing/performance/0.74/local-runs/diagnostic-builder-hosted-37923423889/build-receipt-v1.json");
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let request = serde_json::json!({
        "schema_version": 1, "request_id": "a".repeat(64), "nonce_sha256": "b".repeat(64),
        "operation": "reserve", "expected_state_revision": 0,
        "identity": {"lease_id": "c".repeat(64), "boot_id": "00000000-0000-0000-0000-000000000000",
        "binary_sha256": "bda1c094f3de7c6d42ec45c757838c7a41a8ff1d63a025c661af84455169a101",
        "build_provenance_sha256": Sha256::digest(RECEIPT).iter().map(|b| format!("{b:02x}")).collect::<String>()},
        "source_commit": hydracache_long_run_supervisor_074::diagnostic_lease::SOURCE_COMMIT,
        "preset": hydracache_long_run_supervisor_074::diagnostic_ipc::PRESET,
        "controller": {"repository_id": 1217101761u64, "run_id": 77, "run_attempt": 1, "actor_id": 42},
        "issued_at_unix_seconds": now, "expires_at_unix_seconds": now + 60
    });
    let mut bytes = serde_json::to_vec(&request).unwrap();
    bytes.push(b'\n');
    let expected = prepare_request(&bytes, POLICY, RECEIPT).unwrap();
    let dir = tempfile::tempdir().unwrap();
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let paths = [
        dir.path().join("request"),
        dir.path().join("policy"),
        dir.path().join("receipt"),
    ];
    for (path, raw) in paths.iter().zip([bytes.as_slice(), POLICY, RECEIPT]) {
        std::fs::write(path, raw).unwrap();
    }
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_diagnostic_controller"))
        .arg("audience")
        .args(&paths)
        .output()
        .unwrap();
    assert!(result.status.success(), "{:?}", result.stderr);
    assert_eq!(
        String::from_utf8(result.stdout).unwrap(),
        format!("{}\n", expected.audience())
    );
    let key = dir.path().join("synthetic-not-real-key");
    std::fs::write(&key, "4c".repeat(32)).unwrap();
    std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600)).unwrap();
    let output = dir.path().join("packet");
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_diagnostic_controller"))
        .arg("sign")
        .args(&paths)
        .arg(&key)
        .arg(&output)
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(result.stdout.is_empty() && !output.exists());
    // No public verifier/time override, including after otherwise valid inputs.
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_diagnostic_controller"))
        .arg("audience")
        .args(&paths)
        .arg("--now=1")
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(result.stdout.is_empty());
}
#[test]
fn controller_trust_input_public_entry_is_fixed_and_readonly() {
    use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::{open_fixed_controller_trust, SignedControllerInput};
    let _ = open_fixed_controller_trust;
    let _ = SignedControllerInput::revalidate;
}
#[test]
fn controller_build_input_public_entry_has_no_identity_or_trust_override() {
    use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::SignedControllerInput;
    let _ = SignedControllerInput::bind_reviewed_build;
}
use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::{
    KernelWorkerPolicyInput, WorkerTrustInput,
};

#[test]
fn oidc_public_entry_is_fixed_original_binding_not_execution() {
    use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::{ControllerBuildInput, oidc_input::open_fixed_oidc_trust};
    let _ = open_fixed_oidc_trust;
    let _ = ControllerBuildInput::bind_oidc;
}

#[test]
fn worker_kernel_binding_public_entry_has_no_host_or_revalidation_override() {
    fn bind<'s, 'a, 'p>(
        source: &'s mut WorkerTrustInput,
        journal: &'a mut hydracache_long_run_supervisor_074::diagnostic_worker_policy::authority_registry::fixed::FixedAuthorityRegistry,
        packet: &'p mut hydracache_long_run_supervisor_074::diagnostic_kernel_admission::KernelPacket,
        bytes: &[u8],
    ) -> Result<KernelWorkerPolicyInput<'s, 'a, 'p>, hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::TrustInputError>{
        source.verify_kernel_policy(bytes, packet, journal)
    }
    let _ = bind;
    let _ = KernelWorkerPolicyInput::revalidate;
}
