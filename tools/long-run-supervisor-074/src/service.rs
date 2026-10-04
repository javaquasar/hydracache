use crate::auth::{
    canonical_document, verify_authorization, AuthorizationBody, AuthorizationError,
};
use crate::protocol::{parse_wire_request, Operation, ProtocolError, Request, WireRequest};
#[cfg(target_os = "linux")]
use crate::unix_transport::PeerCredentials;
use ed25519_dalek::VerifyingKey;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct ServicePolicy {
    pub expected_repository_id: u64,
    pub allowed_actor_ids: Vec<u64>,
    pub allowed_client_uids: Vec<u32>,
    pub required_client_gid: u32,
    pub verifying_key: VerifyingKey,
}

#[derive(Debug, Clone)]
pub struct AuthorizedRequest {
    pub request: Request,
    pub authorization: Option<AuthorizationBody>,
}

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("request protocol rejected: {0}")]
    Protocol(#[from] ProtocolError),
    #[error("peer uid or group is not admitted")]
    Peer,
    #[error("request repository or actor is not admitted")]
    Principal,
    #[error("signed operation authorization rejected: {0}")]
    Authorization(#[from] AuthorizationError),
}

#[cfg(target_os = "linux")]
pub fn authorize_packet(
    packet: &[u8],
    peer: &PeerCredentials,
    now_unix_seconds: u64,
    policy: &ServicePolicy,
) -> Result<AuthorizedRequest, ServiceError> {
    if !policy.allowed_client_uids.contains(&peer.uid)
        || !peer.belongs_to_group(policy.required_client_gid)
    {
        return Err(ServiceError::Peer);
    }
    let WireRequest {
        request,
        authorization,
    } = parse_wire_request(packet)?;
    if request.controller.repository_id != policy.expected_repository_id
        || !policy
            .allowed_actor_ids
            .contains(&request.controller.actor_id)
    {
        return Err(ServiceError::Principal);
    }
    let authorization = match request.operation {
        Operation::Start | Operation::Attach | Operation::Seal | Operation::Abort => {
            let document = authorization.ok_or(ProtocolError::OperationFields)?;
            let bytes = canonical_document(&document)?;
            Some(verify_authorization(
                &bytes,
                &request,
                now_unix_seconds,
                &policy.verifying_key,
                policy.expected_repository_id,
                &policy.allowed_actor_ids,
            )?)
        }
        Operation::Status | Operation::Verify => None,
    };
    Ok(AuthorizedRequest {
        request,
        authorization,
    })
}
