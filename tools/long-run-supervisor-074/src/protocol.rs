use crate::auth::SignedAuthorization;
use crate::host_receipt::{BinaryIdentity, HostObservationReceipt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const MAX_PACKET_BYTES: usize = 65_536;
pub const HOST_OBSERVATION_CAMPAIGN_SCOPE_SHA256: &str =
    "2dd3f4aa960289c385aad3988d988533197269368f851e5834f535a753cd92ba";
pub const HOST_OBSERVATION_MANIFEST_SCOPE_SHA256: &str =
    "baa6e451a7248643e7a96fc5908ac07e0c97d38255e31b1bb1494ea5ac26c6ec";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Start,
    Attach,
    Status,
    HostObservation,
    Seal,
    Abort,
    Verify,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControllerIdentity {
    pub repository_id: u64,
    pub run_id: u64,
    pub run_attempt: u32,
    pub actor_id: u64,
    pub authorization_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema_version: u32,
    pub request_id: String,
    pub operation: Operation,
    pub campaign_id: String,
    pub expected_state_revision: u64,
    pub manifest_path: Option<String>,
    pub manifest_sha256: String,
    pub controller: ControllerIdentity,
    pub abort_reason: Option<String>,
    pub approval_nonce_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireRequest {
    pub request: Request,
    pub authorization: Option<SignedAuthorization>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostObservationResult {
    pub schema_version: u32,
    pub installed_source_commit: String,
    pub fixture_binary: BinaryIdentity,
    pub receipt_sha256: String,
    pub receipt: HostObservationReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResponseBody {
    pub schema_version: u32,
    pub request_id: String,
    pub campaign_id: String,
    pub ok: bool,
    pub state_revision: u64,
    pub server_time_unix_seconds: u64,
    pub result: Option<Value>,
    pub error_code: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    #[serde(flatten)]
    pub body: ResponseBody,
    pub response_sha256: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProtocolError {
    #[error("request packet is empty or exceeds 65536 bytes")]
    Size,
    #[error("request is not one strict JSON object: {0}")]
    Json(String),
    #[error("request identity or hash field is invalid")]
    Identity,
    #[error("operation-specific fields are invalid")]
    OperationFields,
    #[error("manifest path is outside the fixed staging root")]
    ManifestPath,
}

pub fn parse_request(packet: &[u8]) -> Result<Request, ProtocolError> {
    if packet.is_empty() || packet.len() > MAX_PACKET_BYTES {
        return Err(ProtocolError::Size);
    }
    let mut deserializer = serde_json::Deserializer::from_slice(packet);
    let request = Request::deserialize(&mut deserializer)
        .map_err(|error| ProtocolError::Json(error.to_string()))?;
    deserializer
        .end()
        .map_err(|error| ProtocolError::Json(error.to_string()))?;
    validate_request(&request)?;
    Ok(request)
}

pub fn parse_wire_request(packet: &[u8]) -> Result<WireRequest, ProtocolError> {
    if packet.is_empty() || packet.len() > MAX_PACKET_BYTES {
        return Err(ProtocolError::Size);
    }
    let mut deserializer = serde_json::Deserializer::from_slice(packet);
    let wire = WireRequest::deserialize(&mut deserializer)
        .map_err(|error| ProtocolError::Json(error.to_string()))?;
    deserializer
        .end()
        .map_err(|error| ProtocolError::Json(error.to_string()))?;
    validate_request(&wire.request)?;
    let authorization_required = matches!(
        wire.request.operation,
        Operation::Start | Operation::Attach | Operation::Seal | Operation::Abort
    );
    if authorization_required != wire.authorization.is_some() {
        return Err(ProtocolError::OperationFields);
    }
    Ok(wire)
}

pub fn validate_request(request: &Request) -> Result<(), ProtocolError> {
    if request.schema_version != 1
        || !is_uuid_v4(&request.request_id)
        || !is_hash(&request.campaign_id)
        || !is_hash(&request.manifest_sha256)
        || request.controller.repository_id == 0
        || request.controller.run_id == 0
        || request.controller.run_attempt == 0
        || request.controller.actor_id == 0
        || !is_hash(&request.controller.authorization_sha256)
    {
        return Err(ProtocolError::Identity);
    }
    match request.operation {
        Operation::Start => {
            if request.abort_reason.is_some() || request.approval_nonce_sha256.is_some() {
                return Err(ProtocolError::OperationFields);
            }
            match (
                request.expected_state_revision,
                request.manifest_path.as_deref(),
            ) {
                (0, Some(path)) => validate_manifest_path(path, &request.campaign_id)?,
                (1.., None) => {}
                _ => return Err(ProtocolError::OperationFields),
            }
        }
        Operation::Abort => {
            if request.manifest_path.is_some()
                || request
                    .abort_reason
                    .as_deref()
                    .is_none_or(|reason| !matches!(reason, "operator-request" | "guard-failure"))
                || request
                    .approval_nonce_sha256
                    .as_deref()
                    .is_none_or(|value| !is_hash(value))
            {
                return Err(ProtocolError::OperationFields);
            }
        }
        Operation::HostObservation => {
            if request.expected_state_revision != 0
                || request.campaign_id != HOST_OBSERVATION_CAMPAIGN_SCOPE_SHA256
                || request.manifest_sha256 != HOST_OBSERVATION_MANIFEST_SCOPE_SHA256
                || request.manifest_path.is_some()
                || request.abort_reason.is_some()
                || request.approval_nonce_sha256.is_some()
                || request.controller.authorization_sha256 != "0".repeat(64)
            {
                return Err(ProtocolError::OperationFields);
            }
        }
        Operation::Attach | Operation::Status | Operation::Seal | Operation::Verify => {
            if request.manifest_path.is_some()
                || request.abort_reason.is_some()
                || request.approval_nonce_sha256.is_some()
            {
                return Err(ProtocolError::OperationFields);
            }
        }
    }
    Ok(())
}

pub fn sign_response(body: ResponseBody) -> Result<Response, serde_json::Error> {
    let canonical = serde_json::to_vec(&serde_json::to_value(&body)?)?;
    Ok(Response {
        body,
        response_sha256: hex(&Sha256::digest(canonical)),
    })
}

pub fn verify_response(response: &Response) -> Result<(), ProtocolError> {
    if !is_hash(&response.response_sha256) {
        return Err(ProtocolError::Identity);
    }
    let canonical = serde_json::to_vec(
        &serde_json::to_value(&response.body)
            .map_err(|error| ProtocolError::Json(error.to_string()))?,
    )
    .map_err(|error| ProtocolError::Json(error.to_string()))?;
    if hex(&Sha256::digest(canonical)) != response.response_sha256 {
        return Err(ProtocolError::Identity);
    }
    Ok(())
}

fn validate_manifest_path(path: &str, campaign_id: &str) -> Result<(), ProtocolError> {
    if !path.starts_with('/') || path.contains('\\') || path.contains("//") {
        return Err(ProtocolError::ManifestPath);
    }
    let components = path[1..].split('/').collect::<Vec<_>>();
    if components.len() < 3
        || components
            .iter()
            .any(|component| component.is_empty() || matches!(*component, "." | ".."))
        || components[components.len() - 2] != campaign_id
        || components.last() != Some(&"campaign-start.json")
    {
        return Err(ProtocolError::ManifestPath);
    }
    Ok(())
}

fn is_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_uuid_v4(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 36
        || bytes[8] != b'-'
        || bytes[13] != b'-'
        || bytes[18] != b'-'
        || bytes[23] != b'-'
        || bytes[14] != b'4'
        || !matches!(bytes[19], b'8' | b'9' | b'a' | b'b')
    {
        return false;
    }
    bytes.iter().enumerate().all(|(index, byte)| {
        matches!(index, 8 | 13 | 18 | 23) || byte.is_ascii_digit() || (b'a'..=b'f').contains(byte)
    })
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
