use crate::protocol::Request;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const MAX_MANIFEST_BYTES: usize = 65_536;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignManifest {
    pub schema_version: u32,
    pub repository_id: u64,
    pub authorization_identity: String,
    pub contract_sha256: String,
    pub tooling_sha: String,
    pub i74_source_sha: String,
    pub c74_source_sha: String,
    pub scenario_sha256: String,
    pub host_receipt_sha256: String,
    pub lease_id: String,
    pub machine_id: String,
    pub boot_id: String,
    pub mount_identity: String,
    pub isolated_cpuset: String,
    pub housekeeping_cpuset: String,
    pub seed: u64,
    pub checkpoint_cadence_seconds: u64,
    pub progress_warning_gap_seconds: u64,
    pub progress_rejection_gap_seconds: u64,
    pub diagnostic_grace_seconds: u64,
    pub product_lease_deadline_unix_seconds: u64,
    pub maximum_campaign_bytes: u64,
    pub maximum_campaign_files: u64,
    pub secret_identifiers: Vec<String>,
    pub release: String,
    pub campaign_id: String,
    pub nonce_sha256: String,
    pub dirty: bool,
    pub controller_history: Vec<serde_json::Value>,
    pub state: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ManifestError {
    #[error("manifest is empty, oversized, non-canonical, or malformed")]
    Document,
    #[error("manifest digest does not match the request")]
    Digest,
    #[error("manifest does not match the request identity")]
    Binding,
    #[error("manifest violates a frozen campaign invariant")]
    Invariant,
}

pub fn parse_and_validate(
    bytes: &[u8],
    request: &Request,
    now_unix_seconds: u64,
) -> Result<CampaignManifest, ManifestError> {
    if bytes.is_empty() || bytes.len() > MAX_MANIFEST_BYTES {
        return Err(ManifestError::Document);
    }
    let encoded = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    if encoded.is_empty() || encoded.contains(&b'\n') || encoded.contains(&b'\r') {
        return Err(ManifestError::Document);
    }
    if hex(&Sha256::digest(encoded)) != request.manifest_sha256 {
        return Err(ManifestError::Digest);
    }
    let mut deserializer = serde_json::Deserializer::from_slice(encoded);
    let manifest =
        CampaignManifest::deserialize(&mut deserializer).map_err(|_| ManifestError::Document)?;
    deserializer.end().map_err(|_| ManifestError::Document)?;
    let canonical = serde_json::to_vec(
        &serde_json::from_slice::<serde_json::Value>(encoded)
            .map_err(|_| ManifestError::Document)?,
    )
    .map_err(|_| ManifestError::Document)?;
    if canonical != encoded {
        return Err(ManifestError::Document);
    }
    validate(&manifest, request, now_unix_seconds)?;
    Ok(manifest)
}

fn validate(
    manifest: &CampaignManifest,
    request: &Request,
    now_unix_seconds: u64,
) -> Result<(), ManifestError> {
    if manifest.campaign_id != request.campaign_id
        || manifest.repository_id != request.controller.repository_id
    {
        return Err(ManifestError::Binding);
    }
    if manifest.schema_version != 1
        || manifest.release != "0.74"
        || manifest.dirty
        || manifest.state != "PREPARED"
        || !manifest.controller_history.is_empty()
        || manifest.seed == 0
        || manifest.product_lease_deadline_unix_seconds <= now_unix_seconds
        || manifest.checkpoint_cadence_seconds != 30
        || manifest.progress_warning_gap_seconds != 90
        || manifest.progress_rejection_gap_seconds != 180
        || manifest.diagnostic_grace_seconds != 30
        || manifest.maximum_campaign_bytes != 21_474_836_480
        || manifest.maximum_campaign_files != 20_000
        || !is_hash(&manifest.campaign_id)
        || !is_hash(&manifest.contract_sha256)
        || !is_hash(&manifest.scenario_sha256)
        || !is_hash(&manifest.host_receipt_sha256)
        || !is_hash(&manifest.nonce_sha256)
        || !is_git_sha(&manifest.tooling_sha)
        || !is_git_sha(&manifest.i74_source_sha)
        || !is_git_sha(&manifest.c74_source_sha)
        || !is_uuid(&manifest.lease_id)
    {
        return Err(ManifestError::Invariant);
    }
    for value in [
        &manifest.authorization_identity,
        &manifest.machine_id,
        &manifest.boot_id,
        &manifest.mount_identity,
        &manifest.isolated_cpuset,
        &manifest.housekeeping_cpuset,
    ] {
        if value.is_empty() || value.len() > 256 {
            return Err(ManifestError::Invariant);
        }
    }
    if manifest.secret_identifiers.iter().any(|value| {
        value.is_empty()
            || value.len() > 128
            || value.contains('=')
            || value.contains('\n')
            || value.contains('\r')
    }) {
        return Err(ManifestError::Invariant);
    }
    Ok(())
}

fn is_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_git_sha(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_uuid(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 36
        && [8, 13, 18, 23]
            .into_iter()
            .all(|index| bytes[index] == b'-')
        && bytes.iter().enumerate().all(|(index, byte)| {
            matches!(index, 8 | 13 | 18 | 23)
                || byte.is_ascii_digit()
                || (b'a'..=b'f').contains(byte)
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
