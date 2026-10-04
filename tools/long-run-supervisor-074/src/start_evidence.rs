use crate::host_receipt::{
    parse_and_validate as parse_host_receipt, verify_receipt_manifest_binding,
    HostObservationReceipt, HostReceiptError, HOST_RECEIPT_HEAD_NAME, HOST_RECEIPT_NAME,
    MAX_HOST_RECEIPT_BYTES,
};
use crate::manifest::{
    parse_and_validate as parse_manifest, CampaignManifest, ManifestError, MAX_MANIFEST_BYTES,
};
use crate::manifest_evidence::{CAMPAIGN_MANIFEST_HEAD_NAME, CAMPAIGN_MANIFEST_NAME};
use crate::protocol::{Operation, Request};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug)]
pub struct PreparedCampaignEvidence {
    pub campaign_directory: PathBuf,
    pub manifest: CampaignManifest,
    pub host_receipt: HostObservationReceipt,
}

pub fn load_start_transport_inputs(
    bundle_directory: &Path,
    request: &Request,
    now_unix_seconds: u64,
) -> Result<(Vec<u8>, Vec<u8>), StartEvidenceError> {
    if request.operation != Operation::Start || request.expected_state_revision != 0 {
        return Err(StartEvidenceError::Path);
    }
    let bundle_directory = canonical_directory(bundle_directory)?;
    let manifest_bytes = read_input(
        &bundle_directory.join(CAMPAIGN_MANIFEST_NAME),
        MAX_MANIFEST_BYTES as u64,
    )?;
    verify_head(
        &bundle_directory.join(CAMPAIGN_MANIFEST_HEAD_NAME),
        &request.manifest_sha256,
    )?;
    let manifest = parse_manifest(&manifest_bytes, request, now_unix_seconds)?;
    let host_receipt_bytes = read_input(
        &bundle_directory.join(HOST_RECEIPT_NAME),
        MAX_HOST_RECEIPT_BYTES as u64,
    )?;
    verify_head(
        &bundle_directory.join(HOST_RECEIPT_HEAD_NAME),
        &manifest.host_receipt_sha256,
    )?;
    let receipt = parse_host_receipt(&host_receipt_bytes, &manifest.host_receipt_sha256)?;
    verify_receipt_manifest_binding(&receipt, &manifest)?;
    Ok((manifest_bytes, host_receipt_bytes))
}

pub fn publish_received_start_evidence(
    staging_root: &Path,
    request: &Request,
    now_unix_seconds: u64,
    manifest_bytes: &[u8],
    host_receipt_bytes: &[u8],
) -> Result<PathBuf, StartEvidenceError> {
    if request.operation != Operation::Start
        || request.expected_state_revision != 0
        || !is_hash(&request.campaign_id)
        || manifest_bytes.is_empty()
        || manifest_bytes.len() > MAX_MANIFEST_BYTES
        || host_receipt_bytes.is_empty()
        || host_receipt_bytes.len() > MAX_HOST_RECEIPT_BYTES
    {
        return Err(StartEvidenceError::Input);
    }
    let requested_manifest = request
        .manifest_path
        .as_deref()
        .map(Path::new)
        .filter(|path| path.is_absolute())
        .ok_or(StartEvidenceError::Path)?;
    if requested_manifest
        != staging_root
            .join(&request.campaign_id)
            .join(CAMPAIGN_MANIFEST_NAME)
    {
        return Err(StartEvidenceError::Path);
    }
    let staging_root = canonical_directory(staging_root)?;
    let final_directory = staging_root.join(&request.campaign_id);

    validate_received_bytes(
        request,
        now_unix_seconds,
        manifest_bytes,
        host_receipt_bytes,
    )?;
    if final_directory.exists() {
        verify_received_directory(
            &staging_root,
            &final_directory,
            request,
            now_unix_seconds,
            manifest_bytes,
            host_receipt_bytes,
        )?;
        return Ok(final_directory);
    }

    let temporary_directory = staging_root.join(format!(".{}.upload", request.campaign_id));
    if temporary_directory.exists() {
        verify_received_directory(
            &staging_root,
            &temporary_directory,
            request,
            now_unix_seconds,
            manifest_bytes,
            host_receipt_bytes,
        )?;
    } else {
        create_private_directory(&temporary_directory)?;
        write_new(
            &temporary_directory.join(CAMPAIGN_MANIFEST_NAME),
            manifest_bytes,
        )?;
        write_new(
            &temporary_directory.join(CAMPAIGN_MANIFEST_HEAD_NAME),
            format!("{}\n", request.manifest_sha256).as_bytes(),
        )?;
        let manifest = parse_manifest(manifest_bytes, request, now_unix_seconds)?;
        write_new(
            &temporary_directory.join(HOST_RECEIPT_NAME),
            host_receipt_bytes,
        )?;
        write_new(
            &temporary_directory.join(HOST_RECEIPT_HEAD_NAME),
            format!("{}\n", manifest.host_receipt_sha256).as_bytes(),
        )?;
        sync_directory(&temporary_directory)?;
    }
    fs::rename(&temporary_directory, &final_directory)?;
    sync_directory(&staging_root)?;
    verify_received_directory(
        &staging_root,
        &final_directory,
        request,
        now_unix_seconds,
        manifest_bytes,
        host_receipt_bytes,
    )?;
    Ok(final_directory)
}

fn validate_received_bytes(
    request: &Request,
    now_unix_seconds: u64,
    manifest_bytes: &[u8],
    host_receipt_bytes: &[u8],
) -> Result<(), StartEvidenceError> {
    let manifest = parse_manifest(manifest_bytes, request, now_unix_seconds)?;
    let host_receipt = parse_host_receipt(host_receipt_bytes, &manifest.host_receipt_sha256)?;
    verify_receipt_manifest_binding(&host_receipt, &manifest)?;
    Ok(())
}

fn verify_received_directory(
    staging_root: &Path,
    directory: &Path,
    request: &Request,
    now_unix_seconds: u64,
    manifest_bytes: &[u8],
    host_receipt_bytes: &[u8],
) -> Result<(), StartEvidenceError> {
    let directory = canonical_direct_child(staging_root, directory)?;
    verify_received_permissions(&directory)?;
    let stored_manifest = read_input(
        &directory.join(CAMPAIGN_MANIFEST_NAME),
        MAX_MANIFEST_BYTES as u64,
    )?;
    let stored_receipt = read_input(
        &directory.join(HOST_RECEIPT_NAME),
        MAX_HOST_RECEIPT_BYTES as u64,
    )?;
    if stored_manifest != manifest_bytes || stored_receipt != host_receipt_bytes {
        return Err(StartEvidenceError::Input);
    }
    verify_head(
        &directory.join(CAMPAIGN_MANIFEST_HEAD_NAME),
        &request.manifest_sha256,
    )?;
    let manifest = parse_manifest(&stored_manifest, request, now_unix_seconds)?;
    verify_head(
        &directory.join(HOST_RECEIPT_HEAD_NAME),
        &manifest.host_receipt_sha256,
    )?;
    let receipt = parse_host_receipt(&stored_receipt, &manifest.host_receipt_sha256)?;
    verify_receipt_manifest_binding(&receipt, &manifest)?;
    Ok(())
}

#[cfg(unix)]
fn verify_received_permissions(directory: &Path) -> Result<(), StartEvidenceError> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    let directory_metadata = fs::symlink_metadata(directory)?;
    if directory_metadata.permissions().mode() & 0o7777 != 0o750
        || directory_metadata.uid() != unsafe { libc::geteuid() }
        || directory_metadata.gid() != unsafe { libc::getegid() }
    {
        return Err(StartEvidenceError::Input);
    }
    for name in [
        CAMPAIGN_MANIFEST_NAME,
        CAMPAIGN_MANIFEST_HEAD_NAME,
        HOST_RECEIPT_NAME,
        HOST_RECEIPT_HEAD_NAME,
    ] {
        let metadata = fs::symlink_metadata(directory.join(name))?;
        if metadata.permissions().mode() & 0o7777 != 0o400
            || metadata.uid() != directory_metadata.uid()
            || metadata.gid() != directory_metadata.gid()
        {
            return Err(StartEvidenceError::Input);
        }
    }
    Ok(())
}

#[cfg(not(unix))]
fn verify_received_permissions(_directory: &Path) -> Result<(), StartEvidenceError> {
    Ok(())
}

#[derive(Debug, Error)]
pub enum StartEvidenceError {
    #[error("start evidence request or path is outside the create-new staging contract")]
    Path,
    #[error("start evidence input is not one bounded regular single-link file")]
    Input,
    #[error("start manifest failed validation: {0}")]
    Manifest(#[from] ManifestError),
    #[error("start host receipt failed validation: {0}")]
    Host(#[from] HostReceiptError),
    #[error("start evidence digest sidecar is malformed")]
    Head,
    #[error("start evidence I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

pub fn prepare_campaign_evidence(
    campaign_root: &Path,
    staging_root: &Path,
    request: &Request,
    now_unix_seconds: u64,
) -> Result<PreparedCampaignEvidence, StartEvidenceError> {
    if request.operation != Operation::Start
        || request.expected_state_revision != 0
        || !is_hash(&request.campaign_id)
    {
        return Err(StartEvidenceError::Path);
    }
    let campaign_root = canonical_directory(campaign_root)?;
    let staging_root = canonical_directory(staging_root)?;
    let staging_directory = staging_root.join(&request.campaign_id);
    let staging_directory = canonical_direct_child(&staging_root, &staging_directory)?;
    let manifest_path = staging_directory.join(CAMPAIGN_MANIFEST_NAME);
    let requested_manifest = request
        .manifest_path
        .as_deref()
        .map(Path::new)
        .filter(|path| path.is_absolute())
        .ok_or(StartEvidenceError::Path)?;
    if fs::canonicalize(requested_manifest)? != fs::canonicalize(&manifest_path)? {
        return Err(StartEvidenceError::Path);
    }

    let manifest_bytes = read_input(&manifest_path, MAX_MANIFEST_BYTES as u64)?;
    verify_head(
        &staging_directory.join(CAMPAIGN_MANIFEST_HEAD_NAME),
        &request.manifest_sha256,
    )?;
    let manifest = parse_manifest(&manifest_bytes, request, now_unix_seconds)?;

    let host_receipt_bytes = read_input(
        &staging_directory.join(HOST_RECEIPT_NAME),
        MAX_HOST_RECEIPT_BYTES as u64,
    )?;
    verify_head(
        &staging_directory.join(HOST_RECEIPT_HEAD_NAME),
        &manifest.host_receipt_sha256,
    )?;
    let host_receipt = parse_host_receipt(&host_receipt_bytes, &manifest.host_receipt_sha256)?;
    verify_receipt_manifest_binding(&host_receipt, &manifest)?;

    let final_directory = campaign_root.join(&request.campaign_id);
    if final_directory.exists() {
        return Err(StartEvidenceError::Path);
    }
    let temporary_name = format!(".{}.admission", request.campaign_id);
    let temporary_directory = campaign_root.join(temporary_name);
    create_private_directory(&temporary_directory)?;
    write_new(
        &temporary_directory.join(CAMPAIGN_MANIFEST_NAME),
        &manifest_bytes,
    )?;
    write_new(
        &temporary_directory.join(CAMPAIGN_MANIFEST_HEAD_NAME),
        format!("{}\n", request.manifest_sha256).as_bytes(),
    )?;
    write_new(
        &temporary_directory.join(HOST_RECEIPT_NAME),
        &host_receipt_bytes,
    )?;
    write_new(
        &temporary_directory.join(HOST_RECEIPT_HEAD_NAME),
        format!("{}\n", manifest.host_receipt_sha256).as_bytes(),
    )?;
    sync_directory(&temporary_directory)?;
    fs::rename(&temporary_directory, &final_directory)?;
    sync_directory(&campaign_root)?;

    Ok(PreparedCampaignEvidence {
        campaign_directory: final_directory,
        manifest,
        host_receipt,
    })
}

pub fn load_campaign_evidence(
    campaign_root: &Path,
    request: &Request,
    now_unix_seconds: u64,
) -> Result<PreparedCampaignEvidence, StartEvidenceError> {
    if request.operation != Operation::Start || !is_hash(&request.campaign_id) {
        return Err(StartEvidenceError::Path);
    }
    let campaign_root = canonical_directory(campaign_root)?;
    let campaign_directory = campaign_root.join(&request.campaign_id);
    let campaign_directory = canonical_direct_child(&campaign_root, &campaign_directory)?;
    let manifest_bytes = read_input(
        &campaign_directory.join(CAMPAIGN_MANIFEST_NAME),
        MAX_MANIFEST_BYTES as u64,
    )?;
    verify_head(
        &campaign_directory.join(CAMPAIGN_MANIFEST_HEAD_NAME),
        &request.manifest_sha256,
    )?;
    let manifest = parse_manifest(&manifest_bytes, request, now_unix_seconds)?;
    let host_receipt_bytes = read_input(
        &campaign_directory.join(HOST_RECEIPT_NAME),
        MAX_HOST_RECEIPT_BYTES as u64,
    )?;
    verify_head(
        &campaign_directory.join(HOST_RECEIPT_HEAD_NAME),
        &manifest.host_receipt_sha256,
    )?;
    let host_receipt = parse_host_receipt(&host_receipt_bytes, &manifest.host_receipt_sha256)?;
    verify_receipt_manifest_binding(&host_receipt, &manifest)?;
    Ok(PreparedCampaignEvidence {
        campaign_directory,
        manifest,
        host_receipt,
    })
}

fn canonical_directory(path: &Path) -> Result<PathBuf, StartEvidenceError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(StartEvidenceError::Path);
    }
    Ok(fs::canonicalize(path)?)
}

fn canonical_direct_child(root: &Path, child: &Path) -> Result<PathBuf, StartEvidenceError> {
    let metadata = fs::symlink_metadata(child)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(StartEvidenceError::Path);
    }
    let child = fs::canonicalize(child)?;
    if child.parent() != Some(root) {
        return Err(StartEvidenceError::Path);
    }
    Ok(child)
}

fn read_input(path: &Path, maximum: u64) -> Result<Vec<u8>, StartEvidenceError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() == 0
        || metadata.len() > maximum
        || has_multiple_links(&metadata)
    {
        return Err(StartEvidenceError::Input);
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    File::open(path)?
        .take(maximum + 1)
        .read_to_end(&mut bytes)?;
    if bytes.is_empty() || bytes.len() as u64 > maximum {
        return Err(StartEvidenceError::Input);
    }
    Ok(bytes)
}

fn verify_head(path: &Path, expected: &str) -> Result<(), StartEvidenceError> {
    let bytes = read_input(path, 65).map_err(|_| StartEvidenceError::Head)?;
    if bytes != format!("{expected}\n").as_bytes() {
        return Err(StartEvidenceError::Head);
    }
    Ok(())
}

fn is_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
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
fn create_private_directory(path: &Path) -> Result<(), StartEvidenceError> {
    use std::os::unix::fs::DirBuilderExt;
    let mut builder = fs::DirBuilder::new();
    builder.mode(0o750).create(path)?;
    Ok(())
}

#[cfg(not(unix))]
fn create_private_directory(path: &Path) -> Result<(), StartEvidenceError> {
    fs::create_dir(path)?;
    Ok(())
}

#[cfg(unix)]
fn write_new(path: &Path, bytes: &[u8]) -> Result<(), StartEvidenceError> {
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o400)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn write_new(path: &Path, bytes: &[u8]) -> Result<(), StartEvidenceError> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<(), StartEvidenceError> {
    File::open(path)?.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> Result<(), StartEvidenceError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::ControllerIdentity;

    fn request(operation: Operation, campaign_id: String) -> Request {
        Request {
            schema_version: 1,
            request_id: "123e4567-e89b-42d3-a456-426614174000".to_owned(),
            operation,
            campaign_id,
            expected_state_revision: 0,
            manifest_path: None,
            manifest_sha256: "b".repeat(64),
            controller: ControllerIdentity {
                repository_id: 1,
                run_id: 1,
                run_attempt: 1,
                actor_id: 1,
                authorization_sha256: "c".repeat(64),
            },
            abort_reason: None,
            approval_nonce_sha256: None,
        }
    }

    #[test]
    fn rejects_non_start_and_invalid_campaign_before_touching_paths() {
        let status = request(Operation::Status, "a".repeat(64));
        assert!(matches!(
            prepare_campaign_evidence(Path::new("missing"), Path::new("missing"), &status, 1),
            Err(StartEvidenceError::Path)
        ));
        let start = request(Operation::Start, "../escape".to_owned());
        assert!(matches!(
            prepare_campaign_evidence(Path::new("missing"), Path::new("missing"), &start, 1),
            Err(StartEvidenceError::Path)
        ));
    }

    #[test]
    fn bounded_input_and_create_new_output_are_enforced() {
        let temporary = tempfile::tempdir().unwrap();
        let input = temporary.path().join("input");
        fs::write(&input, b"evidence").unwrap();
        assert_eq!(read_input(&input, 8).unwrap(), b"evidence");
        assert!(matches!(
            read_input(&input, 7),
            Err(StartEvidenceError::Input)
        ));
        let output = temporary.path().join("output");
        write_new(&output, b"first").unwrap();
        assert!(write_new(&output, b"second").is_err());
        assert_eq!(fs::read(output).unwrap(), b"first");
    }

    #[cfg(unix)]
    #[test]
    fn symlink_and_hardlink_inputs_are_rejected() {
        use std::os::unix::fs::symlink;

        let temporary = tempfile::tempdir().unwrap();
        let input = temporary.path().join("input");
        fs::write(&input, b"evidence").unwrap();
        let alias = temporary.path().join("alias");
        fs::hard_link(&input, &alias).unwrap();
        assert!(matches!(
            read_input(&input, 64),
            Err(StartEvidenceError::Input)
        ));
        fs::remove_file(alias).unwrap();
        let link = temporary.path().join("link");
        symlink(&input, &link).unwrap();
        assert!(matches!(
            read_input(&link, 64),
            Err(StartEvidenceError::Input)
        ));
    }
}
