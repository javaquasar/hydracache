#![cfg(target_os = "linux")]

use ed25519_dalek::{Signer, SigningKey};
use hydracache_long_run_supervisor_074::auth::{
    canonical_document, canonical_message, AuthorizationBody, SignedAuthorization,
};
use hydracache_long_run_supervisor_074::protocol::{
    ControllerIdentity, Operation, Request, WireRequest, HOST_OBSERVATION_CAMPAIGN_SCOPE_SHA256,
    HOST_OBSERVATION_MANIFEST_SCOPE_SHA256,
};
use hydracache_long_run_supervisor_074::service::{authorize_packet, ServiceError, ServicePolicy};
use hydracache_long_run_supervisor_074::unix_transport::PeerCredentials;
use sha2::{Digest, Sha256};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn request(operation: Operation) -> Request {
    Request {
        schema_version: 1,
        request_id: "123e4567-e89b-42d3-a456-426614174000".to_owned(),
        operation,
        campaign_id: "a".repeat(64),
        expected_state_revision: 7,
        manifest_path: (operation == Operation::Start).then(|| {
            format!(
                "/var/lib/hydracache-performance/staging/{}/campaign-start.json",
                "a".repeat(64)
            )
        }),
        manifest_sha256: "b".repeat(64),
        controller: ControllerIdentity {
            repository_id: 10,
            run_id: 20,
            run_attempt: 1,
            actor_id: 30,
            authorization_sha256: "c".repeat(64),
        },
        abort_reason: None,
        approval_nonce_sha256: None,
    }
}

fn signed_wire(key: &SigningKey) -> Vec<u8> {
    let mut request = request(Operation::Attach);
    let body = AuthorizationBody {
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
    };
    let document = SignedAuthorization {
        signature_hex: hex(&key.sign(&canonical_message(&body).unwrap()).to_bytes()),
        body,
    };
    request.controller.authorization_sha256 =
        hex(&Sha256::digest(canonical_document(&document).unwrap()));
    serde_json::to_vec(&WireRequest {
        request,
        authorization: Some(document),
    })
    .unwrap()
}

fn policy(key: &SigningKey) -> ServicePolicy {
    ServicePolicy {
        expected_repository_id: 10,
        allowed_actor_ids: vec![30],
        allowed_client_uids: vec![1_001],
        required_client_gid: 2_001,
        verifying_key: key.verifying_key(),
    }
}

fn peer() -> PeerCredentials {
    PeerCredentials {
        pid: 100,
        uid: 1_001,
        gid: 1_001,
        supplemental_gids: vec![2_001],
    }
}

#[test]
fn exact_peer_principal_and_signature_are_required_together() {
    let key = SigningKey::from_bytes(&[7; 32]);
    let authorized = authorize_packet(&signed_wire(&key), &peer(), 1_100, &policy(&key)).unwrap();
    assert_eq!(authorized.request.operation, Operation::Attach);
    assert!(authorized.authorization.is_some());

    let mut wrong_peer = peer();
    wrong_peer.supplemental_gids.clear();
    assert!(matches!(
        authorize_packet(&signed_wire(&key), &wrong_peer, 1_100, &policy(&key)),
        Err(ServiceError::Peer)
    ));

    let wrong_key = SigningKey::from_bytes(&[8; 32]);
    assert!(matches!(
        authorize_packet(&signed_wire(&key), &peer(), 1_100, &policy(&wrong_key)),
        Err(ServiceError::Authorization(_))
    ));
}

#[test]
fn read_only_status_has_no_signed_document_but_still_checks_peer_and_principal() {
    let key = SigningKey::from_bytes(&[7; 32]);
    let packet = serde_json::to_vec(&WireRequest {
        request: request(Operation::Status),
        authorization: None,
    })
    .unwrap();
    let authorized = authorize_packet(&packet, &peer(), 1_100, &policy(&key)).unwrap();
    assert!(authorized.authorization.is_none());

    let mut wrong_policy = policy(&key);
    wrong_policy.allowed_actor_ids = vec![31];
    assert!(matches!(
        authorize_packet(&packet, &peer(), 1_100, &wrong_policy),
        Err(ServiceError::Principal)
    ));
}

#[test]
fn read_only_host_observation_has_no_signature_but_keeps_peer_admission() {
    let key = SigningKey::from_bytes(&[7; 32]);
    let mut observation = request(Operation::HostObservation);
    observation.campaign_id = HOST_OBSERVATION_CAMPAIGN_SCOPE_SHA256.to_owned();
    observation.manifest_sha256 = HOST_OBSERVATION_MANIFEST_SCOPE_SHA256.to_owned();
    observation.expected_state_revision = 0;
    observation.controller.authorization_sha256 = "0".repeat(64);
    let packet = serde_json::to_vec(&WireRequest {
        request: observation,
        authorization: None,
    })
    .unwrap();
    let authorized = authorize_packet(&packet, &peer(), 1_100, &policy(&key)).unwrap();
    assert_eq!(authorized.request.operation, Operation::HostObservation);
    assert!(authorized.authorization.is_none());

    let mut wrong_peer = peer();
    wrong_peer.supplemental_gids.clear();
    assert!(matches!(
        authorize_packet(&packet, &wrong_peer, 1_100, &policy(&key)),
        Err(ServiceError::Peer)
    ));
}
