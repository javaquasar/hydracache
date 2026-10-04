use crate::protocol::{Operation, Request};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

const DOMAIN: &[u8] = b"hydracache-long-run-authorization-v1";
pub const MAX_AUTHORIZATION_BYTES: usize = 16_384;
pub const MAX_AUTHORIZATION_LIFETIME_SECONDS: u64 = 600;
pub const MAX_CLOCK_SKEW_SECONDS: u64 = 60;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorizationBody {
    pub schema_version: u32,
    pub request_id: String,
    pub operation: Operation,
    pub campaign_id: String,
    pub manifest_sha256: String,
    pub repository_id: u64,
    pub run_id: u64,
    pub actor_id: u64,
    pub issued_at_unix_seconds: u64,
    pub expires_at_unix_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedAuthorization {
    pub body: AuthorizationBody,
    pub signature_hex: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AuthorizationError {
    #[error("authorization document is empty, oversized, or malformed")]
    Document,
    #[error("authorization digest does not match the request")]
    Digest,
    #[error("authorization signature is invalid")]
    Signature,
    #[error("authorization is outside its frozen time window")]
    Time,
    #[error("authorization identity does not match the request")]
    Binding,
    #[error("repository or actor is not allowlisted")]
    Principal,
}

pub fn verify_authorization(
    bytes: &[u8],
    request: &Request,
    now_unix_seconds: u64,
    verifying_key: &VerifyingKey,
    expected_repository_id: u64,
    allowed_actor_ids: &[u64],
) -> Result<AuthorizationBody, AuthorizationError> {
    if bytes.is_empty() || bytes.len() > MAX_AUTHORIZATION_BYTES {
        return Err(AuthorizationError::Document);
    }
    if hex(&Sha256::digest(bytes)) != request.controller.authorization_sha256 {
        return Err(AuthorizationError::Digest);
    }
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let document = SignedAuthorization::deserialize(&mut deserializer)
        .map_err(|_| AuthorizationError::Document)?;
    deserializer
        .end()
        .map_err(|_| AuthorizationError::Document)?;
    let signature_bytes =
        decode_exact::<64>(&document.signature_hex).ok_or(AuthorizationError::Document)?;
    let signature = Signature::from_bytes(&signature_bytes);
    verifying_key
        .verify(&canonical_message(&document.body)?, &signature)
        .map_err(|_| AuthorizationError::Signature)?;

    let body = &document.body;
    if body.schema_version != 1
        || body.request_id != request.request_id
        || body.operation != request.operation
        || body.campaign_id != request.campaign_id
        || body.manifest_sha256 != request.manifest_sha256
        || body.repository_id != request.controller.repository_id
        || body.run_id != request.controller.run_id
        || body.actor_id != request.controller.actor_id
    {
        return Err(AuthorizationError::Binding);
    }
    if body.repository_id != expected_repository_id || !allowed_actor_ids.contains(&body.actor_id) {
        return Err(AuthorizationError::Principal);
    }
    if body.expires_at_unix_seconds < body.issued_at_unix_seconds
        || body
            .expires_at_unix_seconds
            .saturating_sub(body.issued_at_unix_seconds)
            > MAX_AUTHORIZATION_LIFETIME_SECONDS
        || body.issued_at_unix_seconds > now_unix_seconds.saturating_add(MAX_CLOCK_SKEW_SECONDS)
        || body.expires_at_unix_seconds < now_unix_seconds
    {
        return Err(AuthorizationError::Time);
    }
    Ok(document.body)
}

pub fn canonical_message(body: &AuthorizationBody) -> Result<Vec<u8>, AuthorizationError> {
    let mut message = DOMAIN.to_vec();
    message.push(0);
    message.extend(serde_json::to_vec(body).map_err(|_| AuthorizationError::Document)?);
    Ok(message)
}

fn decode_exact<const N: usize>(value: &str) -> Option<[u8; N]> {
    if value.len() != N * 2 {
        return None;
    }
    let mut output = [0; N];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        output[index] = (nibble(pair[0])? << 4) | nibble(pair[1])?;
    }
    Some(output)
}

fn nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(DIGITS[(byte >> 4) as usize] as char);
        output.push(DIGITS[(byte & 15) as usize] as char);
    }
    output
}
