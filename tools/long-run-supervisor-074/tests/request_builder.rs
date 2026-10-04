use ed25519_dalek::SigningKey;
use hydracache_long_run_supervisor_074::auth::{canonical_document, verify_authorization};
use hydracache_long_run_supervisor_074::protocol::{
    parse_wire_request, ControllerIdentity, Operation, Request,
};
use hydracache_long_run_supervisor_074::request_builder::{
    build_signed_request, RequestBuilderError,
};
use sha2::{Digest, Sha256};
use std::fs;

fn request() -> Request {
    Request {
        schema_version: 1,
        request_id: "123e4567-e89b-42d3-a456-426614174000".to_owned(),
        operation: Operation::Attach,
        campaign_id: "a".repeat(64),
        expected_state_revision: 5,
        manifest_path: None,
        manifest_sha256: "b".repeat(64),
        controller: ControllerIdentity {
            repository_id: 10,
            run_id: 20,
            run_attempt: 1,
            actor_id: 30,
            authorization_sha256: "0".repeat(64),
        },
        abort_reason: None,
        approval_nonce_sha256: None,
    }
}

fn write_key(path: &std::path::Path, byte: u8) {
    fs::write(path, format!("{}\n", format!("{byte:02x}").repeat(32))).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
fn builds_a_create_new_packet_bound_to_the_request_and_time_window() {
    let temporary = tempfile::tempdir().unwrap();
    let input = temporary.path().join("request.json");
    let key = temporary.path().join("signing-key");
    let output = temporary.path().join("packet.json");
    fs::write(&input, serde_json::to_vec(&request()).unwrap()).unwrap();
    write_key(&key, 7);

    let digest = build_signed_request(&input, &key, 1_000, 1_300, &output).unwrap();
    let packet = fs::read(&output).unwrap();
    assert_eq!(digest, sha256_hex(&packet));
    let wire = parse_wire_request(&packet).unwrap();
    let authorization = wire.authorization.as_ref().unwrap();
    let authorization_bytes = canonical_document(authorization).unwrap();
    assert_eq!(
        wire.request.controller.authorization_sha256,
        sha256_hex(&authorization_bytes)
    );
    let signing_key = SigningKey::from_bytes(&[7; 32]);
    verify_authorization(
        &authorization_bytes,
        &wire.request,
        1_100,
        &signing_key.verifying_key(),
        10,
        &[30],
    )
    .unwrap();
    assert!(matches!(
        build_signed_request(&input, &key, 1_000, 1_300, &output),
        Err(RequestBuilderError::Io(error)) if error.kind() == std::io::ErrorKind::AlreadyExists
    ));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&output).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}

#[test]
fn rejects_non_mutating_requests_time_drift_and_unsafe_key_files() {
    let temporary = tempfile::tempdir().unwrap();
    let input = temporary.path().join("request.json");
    let key = temporary.path().join("signing-key");
    write_key(&key, 7);

    let mut unsigned = request();
    unsigned.operation = Operation::Status;
    fs::write(&input, serde_json::to_vec(&unsigned).unwrap()).unwrap();
    assert!(matches!(
        build_signed_request(
            &input,
            &key,
            1_000,
            1_300,
            &temporary.path().join("status.json")
        ),
        Err(RequestBuilderError::Operation)
    ));

    fs::write(&input, serde_json::to_vec(&request()).unwrap()).unwrap();
    assert!(matches!(
        build_signed_request(
            &input,
            &key,
            1_000,
            1_601,
            &temporary.path().join("expired.json")
        ),
        Err(RequestBuilderError::Time)
    ));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&key, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(matches!(
            build_signed_request(
                &input,
                &key,
                1_000,
                1_300,
                &temporary.path().join("unsafe.json")
            ),
            Err(RequestBuilderError::Path)
        ));
        fs::set_permissions(&key, fs::Permissions::from_mode(0o600)).unwrap();
        fs::hard_link(&key, temporary.path().join("key-alias")).unwrap();
        assert!(matches!(
            build_signed_request(
                &input,
                &key,
                1_000,
                1_300,
                &temporary.path().join("hardlinked.json")
            ),
            Err(RequestBuilderError::Path)
        ));
    }
}
