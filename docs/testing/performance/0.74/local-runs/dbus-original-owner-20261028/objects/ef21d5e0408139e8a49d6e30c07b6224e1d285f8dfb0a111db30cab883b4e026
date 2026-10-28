use ed25519_dalek::{Signer, SigningKey};
use hydracache_long_run_supervisor_074::auth::{
    canonical_message, verify_authorization, AuthorizationBody, AuthorizationError,
    SignedAuthorization,
};
use hydracache_long_run_supervisor_074::protocol::{ControllerIdentity, Operation, Request};
use sha2::{Digest, Sha256};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn fixture_request() -> Request {
    Request {
        schema_version: 1,
        request_id: "123e4567-e89b-42d3-a456-426614174000".to_owned(),
        operation: Operation::Attach,
        campaign_id: "a".repeat(64),
        expected_state_revision: 7,
        manifest_path: None,
        manifest_sha256: "b".repeat(64),
        controller: ControllerIdentity {
            repository_id: 10,
            run_id: 20,
            run_attempt: 1,
            actor_id: 30,
            authorization_sha256: String::new(),
        },
        abort_reason: None,
        approval_nonce_sha256: None,
    }
}

fn body(request: &Request) -> AuthorizationBody {
    AuthorizationBody {
        schema_version: 1,
        request_id: request.request_id.clone(),
        operation: request.operation,
        campaign_id: request.campaign_id.clone(),
        manifest_sha256: request.manifest_sha256.clone(),
        repository_id: request.controller.repository_id,
        run_id: request.controller.run_id,
        actor_id: request.controller.actor_id,
        issued_at_unix_seconds: 1_000,
        expires_at_unix_seconds: 1_300,
    }
}

fn signed(request: &mut Request, body: AuthorizationBody, key: &SigningKey) -> Vec<u8> {
    let signature = key.sign(&canonical_message(&body).unwrap());
    let document = SignedAuthorization {
        body,
        signature_hex: hex(&signature.to_bytes()),
    };
    let bytes = serde_json::to_vec(&document).unwrap();
    request.controller.authorization_sha256 = hex(&Sha256::digest(&bytes));
    bytes
}

#[test]
fn exact_signed_authorization_is_accepted() {
    let key = SigningKey::from_bytes(&[7; 32]);
    let mut request = fixture_request();
    let authorization = body(&request);
    let document = signed(&mut request, authorization, &key);
    let verified =
        verify_authorization(&document, &request, 1_100, &key.verifying_key(), 10, &[30]).unwrap();
    assert_eq!(verified.request_id, request.request_id);
}

#[test]
fn digest_signature_and_request_binding_tamper_fail_closed() {
    let key = SigningKey::from_bytes(&[7; 32]);
    let mut request = fixture_request();
    let authorization = body(&request);
    let mut document = signed(&mut request, authorization, &key);
    document[20] ^= 1;
    assert!(matches!(
        verify_authorization(&document, &request, 1_100, &key.verifying_key(), 10, &[30]),
        Err(AuthorizationError::Digest)
    ));

    let mut request = fixture_request();
    let authorization = body(&request);
    let document = signed(&mut request, authorization, &key);
    request.controller.authorization_sha256 = hex(&Sha256::digest(&document));
    request.campaign_id = "c".repeat(64);
    assert!(matches!(
        verify_authorization(&document, &request, 1_100, &key.verifying_key(), 10, &[30]),
        Err(AuthorizationError::Binding)
    ));

    let wrong_key = SigningKey::from_bytes(&[8; 32]);
    let mut request = fixture_request();
    let authorization = body(&request);
    let document = signed(&mut request, authorization, &key);
    assert!(matches!(
        verify_authorization(
            &document,
            &request,
            1_100,
            &wrong_key.verifying_key(),
            10,
            &[30]
        ),
        Err(AuthorizationError::Signature)
    ));
}

#[test]
fn repository_actor_and_time_window_are_enforced() {
    let key = SigningKey::from_bytes(&[7; 32]);
    let mut request = fixture_request();
    let authorization = body(&request);
    let document = signed(&mut request, authorization, &key);
    assert_eq!(
        verify_authorization(&document, &request, 1_100, &key.verifying_key(), 11, &[30]),
        Err(AuthorizationError::Principal)
    );
    assert_eq!(
        verify_authorization(&document, &request, 1_301, &key.verifying_key(), 10, &[30]),
        Err(AuthorizationError::Time)
    );
    assert_eq!(
        verify_authorization(&document, &request, 1_100, &key.verifying_key(), 10, &[31]),
        Err(AuthorizationError::Principal)
    );

    let mut request = fixture_request();
    let mut too_long = body(&request);
    too_long.expires_at_unix_seconds = 1_601;
    let document = signed(&mut request, too_long, &key);
    assert_eq!(
        verify_authorization(&document, &request, 1_100, &key.verifying_key(), 10, &[30]),
        Err(AuthorizationError::Time)
    );
}
