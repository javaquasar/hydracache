use crate::auth::{
    canonical_document, canonical_message, verify_authorization, AuthorizationBody,
    AuthorizationError, SignedAuthorization, MAX_AUTHORIZATION_LIFETIME_SECONDS,
};
use crate::protocol::{parse_request, parse_wire_request, Operation, ProtocolError, WireRequest};
use crate::sha256_hex;
use ed25519_dalek::{Signer, SigningKey};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use thiserror::Error;

const MAX_SIGNING_KEY_BYTES: u64 = 65;

#[derive(Debug, Error)]
pub enum RequestBuilderError {
    #[error("request builder supports only signed mutating operations")]
    Operation,
    #[error("authorization time window is invalid")]
    Time,
    #[error("signing key or output path is unsafe")]
    Path,
    #[error("request packet is invalid: {0}")]
    Protocol(#[from] ProtocolError),
    #[error("authorization construction failed: {0}")]
    Authorization(#[from] AuthorizationError),
    #[error("request builder I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("request builder JSON failed: {0}")]
    Json(#[from] serde_json::Error),
}

pub fn build_signed_request(
    request_path: &Path,
    signing_key_path: &Path,
    issued_at_unix_seconds: u64,
    expires_at_unix_seconds: u64,
    output_path: &Path,
) -> Result<String, RequestBuilderError> {
    if issued_at_unix_seconds == 0
        || expires_at_unix_seconds < issued_at_unix_seconds
        || expires_at_unix_seconds.saturating_sub(issued_at_unix_seconds)
            > MAX_AUTHORIZATION_LIFETIME_SECONDS
    {
        return Err(RequestBuilderError::Time);
    }
    let request_bytes = read_bounded_regular(
        request_path,
        crate::protocol::MAX_PACKET_BYTES as u64,
        false,
    )?;
    let mut request = parse_request(&request_bytes)?;
    if !matches!(
        request.operation,
        Operation::Start | Operation::Attach | Operation::Seal | Operation::Abort
    ) || request.controller.authorization_sha256 != "0".repeat(64)
    {
        return Err(RequestBuilderError::Operation);
    }
    let key_bytes = read_bounded_regular(signing_key_path, MAX_SIGNING_KEY_BYTES, true)?;
    let key_hex = key_bytes.strip_suffix(b"\n").unwrap_or(&key_bytes);
    let key = decode_signing_key(key_hex).ok_or(RequestBuilderError::Path)?;
    let signing_key = SigningKey::from_bytes(&key);
    let body = AuthorizationBody {
        schema_version: 1,
        request_id: request.request_id.clone(),
        operation: request.operation,
        campaign_id: request.campaign_id.clone(),
        manifest_sha256: request.manifest_sha256.clone(),
        repository_id: request.controller.repository_id,
        run_id: request.controller.run_id,
        actor_id: request.controller.actor_id,
        issued_at_unix_seconds,
        expires_at_unix_seconds,
    };
    let authorization = SignedAuthorization {
        signature_hex: hex(&signing_key.sign(&canonical_message(&body)?).to_bytes()),
        body,
    };
    let authorization_bytes = canonical_document(&authorization)?;
    request.controller.authorization_sha256 = sha256_hex(&authorization_bytes);
    let wire = WireRequest {
        request,
        authorization: Some(authorization),
    };
    let packet = serde_json::to_vec(&wire)?;
    let verified = parse_wire_request(&packet)?;
    verify_authorization(
        &authorization_bytes,
        &verified.request,
        issued_at_unix_seconds,
        &signing_key.verifying_key(),
        verified.request.controller.repository_id,
        &[verified.request.controller.actor_id],
    )?;
    write_new_private(output_path, &packet)?;
    Ok(sha256_hex(&packet))
}

fn read_bounded_regular(
    path: &Path,
    maximum: u64,
    require_private: bool,
) -> Result<Vec<u8>, RequestBuilderError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() == 0
        || metadata.len() > maximum
        || has_multiple_links(&metadata)
        || (require_private && !is_private(&metadata))
    {
        return Err(RequestBuilderError::Path);
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    File::open(path)?
        .take(maximum.saturating_add(1))
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 != metadata.len() {
        return Err(RequestBuilderError::Path);
    }
    Ok(bytes)
}

fn decode_signing_key(value: &[u8]) -> Option<[u8; 32]> {
    if value.len() != 64 {
        return None;
    }
    let mut output = [0_u8; 32];
    for (index, pair) in value.chunks_exact(2).enumerate() {
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

#[cfg(unix)]
fn has_multiple_links(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    metadata.nlink() != 1
}

#[cfg(not(unix))]
fn has_multiple_links(_metadata: &fs::Metadata) -> bool {
    false
}

#[cfg(unix)]
fn is_private(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    metadata.uid() == unsafe { libc::geteuid() } && metadata.permissions().mode() & 0o077 == 0
}

#[cfg(not(unix))]
fn is_private(_metadata: &fs::Metadata) -> bool {
    true
}

#[cfg(unix)]
fn write_new_private(path: &Path, bytes: &[u8]) -> Result<(), RequestBuilderError> {
    use std::os::unix::fs::OpenOptionsExt;
    let parent = safe_parent(path)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    File::open(parent)?.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn write_new_private(path: &Path, bytes: &[u8]) -> Result<(), RequestBuilderError> {
    let _parent = safe_parent(path)?;
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn safe_parent(path: &Path) -> Result<&Path, RequestBuilderError> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let metadata = fs::symlink_metadata(parent)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() || path.file_name().is_none() {
        return Err(RequestBuilderError::Path);
    }
    Ok(parent)
}
