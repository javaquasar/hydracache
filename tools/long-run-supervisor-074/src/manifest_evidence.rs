use crate::manifest::{
    frozen_identity_from_manifest, parse_and_validate, CampaignManifest, ManifestError,
    MAX_MANIFEST_BYTES,
};
use crate::protocol::Request;
use crate::state::DurableCampaignState;
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;
use thiserror::Error;

pub const CAMPAIGN_MANIFEST_NAME: &str = "campaign-start.json";
pub const CAMPAIGN_MANIFEST_HEAD_NAME: &str = "campaign-start.sha256";

#[derive(Debug, Error)]
pub enum ManifestEvidenceError {
    #[error("persistent campaign manifest path is unsafe")]
    Path,
    #[error("persistent campaign manifest digest file is malformed")]
    Head,
    #[error("persistent campaign manifest failed validation: {0}")]
    Manifest(#[from] ManifestError),
    #[error("persistent campaign manifest does not match durable state")]
    Binding,
    #[error("persistent campaign manifest serialization failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("persistent campaign manifest I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

pub fn verify_manifest_evidence(
    campaign_directory: &Path,
    request: &Request,
    state: &DurableCampaignState,
    now_unix_seconds: u64,
) -> Result<CampaignManifest, ManifestEvidenceError> {
    let manifest_path = campaign_directory.join(CAMPAIGN_MANIFEST_NAME);
    let head_path = campaign_directory.join(CAMPAIGN_MANIFEST_HEAD_NAME);
    let bytes = read_manifest(&manifest_path)?;
    verify_head(&head_path, &request.manifest_sha256)?;
    let manifest = parse_and_validate(&bytes, request, now_unix_seconds)?;
    let identity = frozen_identity_from_manifest(&manifest, &request.manifest_sha256)?;
    if identity != state.identity {
        return Err(ManifestEvidenceError::Binding);
    }
    Ok(manifest)
}

fn read_manifest(path: &Path) -> Result<Vec<u8>, ManifestEvidenceError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() == 0
        || metadata.len() > MAX_MANIFEST_BYTES as u64
        || has_multiple_links(&metadata)
    {
        return Err(ManifestEvidenceError::Path);
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    File::open(path)?
        .take(MAX_MANIFEST_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_MANIFEST_BYTES as u64 {
        return Err(ManifestEvidenceError::Path);
    }
    Ok(bytes)
}

fn verify_head(path: &Path, expected: &str) -> Result<(), ManifestEvidenceError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| ManifestEvidenceError::Head)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() != 65
        || has_multiple_links(&metadata)
    {
        return Err(ManifestEvidenceError::Head);
    }
    let value = fs::read(path).map_err(|_| ManifestEvidenceError::Head)?;
    if value != format!("{expected}\n").as_bytes() {
        return Err(ManifestEvidenceError::Head);
    }
    Ok(())
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
