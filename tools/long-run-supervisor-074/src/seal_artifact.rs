use crate::archive::{
    create_deterministic_archive, verify_archive, ArchiveError, ArchiveLimits, ArchiveReceipt,
    ArchiveVerification, ARCHIVE_NAME, OUTER_DIGEST_NAME,
};
use crate::artifact::{
    build_packet, verify_packet, ArtifactError, PacketLimits, PacketPlan, PacketReceipt,
};
use crate::state::DurableCampaignState;
use crate::{canonical_json, is_hash, sha256_hex, Role};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SealArtifactIntent {
    pub schema_version: u32,
    pub campaign_id: String,
    pub request_id: String,
    pub request_sha256: String,
    pub role: Role,
    pub plan_binding_sha256: String,
    pub packet_directory_name: String,
    pub archive_directory_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SealArtifactResult {
    pub schema_version: u32,
    pub intent_sha256: String,
    pub campaign_id: String,
    pub role: Role,
    pub packet_directory_name: String,
    pub packet_manifest_sha256: String,
    pub raw_manifest_sha256: String,
    pub raw_manifest_set_sha256: String,
    pub raw_file_count: usize,
    pub raw_total_bytes: u64,
    pub archive_directory_name: String,
    pub archive_sha256: String,
    pub archive_bytes: u64,
    pub archive_input_files: usize,
    pub archive_input_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SealResponseResult {
    pub state: DurableCampaignState,
    pub artifact: SealArtifactResult,
}

#[derive(Debug, Error)]
pub enum SealArtifactError {
    #[error("seal artifact identity, plan, or durable evidence is invalid")]
    Binding,
    #[error("seal artifact packet failed: {0}")]
    Packet(#[from] ArtifactError),
    #[error("seal artifact archive failed: {0}")]
    Archive(#[from] ArchiveError),
    #[error("seal artifact JSON failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("seal artifact I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Serialize)]
struct PlanBinding<'a> {
    plan: &'a PacketPlan,
    packet_limits: PacketLimits,
    archive_limits: ArchiveLimits,
}

#[allow(clippy::too_many_arguments)]
pub fn build_or_recover_seal_artifact(
    campaign_directory: &Path,
    seal_root: &Path,
    request_id: &str,
    request_sha256: &str,
    role: Role,
    plan: &PacketPlan,
    packet_limits: PacketLimits,
    archive_limits: ArchiveLimits,
) -> Result<SealArtifactResult, SealArtifactError> {
    validate_inputs(
        campaign_directory,
        seal_root,
        request_id,
        request_sha256,
        &role,
        plan,
    )?;
    let campaign_directory = canonical_directory(campaign_directory)?;
    let seal_root = canonical_directory(seal_root)?;
    if seal_root.starts_with(&campaign_directory) {
        return Err(SealArtifactError::Binding);
    }
    let role_name = role_name(&role);
    let base = format!("{}-{role_name}", plan.campaign_id);
    let intent = SealArtifactIntent {
        schema_version: 1,
        campaign_id: plan.campaign_id.clone(),
        request_id: request_id.to_owned(),
        request_sha256: request_sha256.to_owned(),
        role: role.clone(),
        plan_binding_sha256: sha256_hex(&canonical_json(&PlanBinding {
            plan,
            packet_limits,
            archive_limits,
        })?),
        packet_directory_name: format!("{base}-packet"),
        archive_directory_name: format!("{base}-archive"),
    };
    let intent_bytes = canonical_json(&intent)?;
    let intent_sha256 = sha256_hex(&intent_bytes);
    let intent_path = campaign_directory.join(format!("{role_name}-seal-intent.json"));
    persist_or_verify_document(&intent_path, &intent_bytes)?;

    let result_path = campaign_directory.join(format!("{role_name}-seal-result.json"));
    if fs::symlink_metadata(&result_path).is_ok() {
        let result: SealArtifactResult = read_canonical_document(&result_path)?;
        verify_result(
            &seal_root,
            &intent,
            &intent_sha256,
            &result,
            plan,
            packet_limits,
            archive_limits,
        )?;
        return Ok(result);
    }

    let packet_directory = seal_root.join(&intent.packet_directory_name);
    let packet =
        recover_or_build_packet(&campaign_directory, &packet_directory, plan, packet_limits)?;
    let archive_directory = seal_root.join(&intent.archive_directory_name);
    let archive =
        recover_or_build_archive(&packet.packet_directory, &archive_directory, archive_limits)?;
    let result = SealArtifactResult {
        schema_version: 1,
        intent_sha256,
        campaign_id: plan.campaign_id.clone(),
        role,
        packet_directory_name: intent.packet_directory_name,
        packet_manifest_sha256: packet.packet_manifest_sha256,
        raw_manifest_sha256: packet.raw_manifest_sha256,
        raw_manifest_set_sha256: packet.raw_manifest_set_sha256,
        raw_file_count: packet.raw_file_count,
        raw_total_bytes: packet.raw_total_bytes,
        archive_directory_name: intent.archive_directory_name,
        archive_sha256: archive.archive_sha256,
        archive_bytes: archive.archive_bytes,
        archive_input_files: archive.input_files,
        archive_input_bytes: archive.input_bytes,
    };
    persist_or_verify_document(&result_path, &canonical_json(&result)?)?;
    Ok(result)
}

fn recover_or_build_packet(
    campaign_directory: &Path,
    packet_directory: &Path,
    plan: &PacketPlan,
    limits: PacketLimits,
) -> Result<PacketReceipt, SealArtifactError> {
    if fs::symlink_metadata(packet_directory).is_ok() {
        return Ok(verify_packet(packet_directory, plan, limits)?);
    }
    let staging = building_path(packet_directory)?;
    if fs::symlink_metadata(&staging).is_ok() {
        verify_packet(&staging, plan, limits)?;
        fs::rename(&staging, packet_directory)?;
        sync_directory(
            packet_directory
                .parent()
                .ok_or(SealArtifactError::Binding)?,
        )?;
        return Ok(verify_packet(packet_directory, plan, limits)?);
    }
    Ok(build_packet(
        campaign_directory,
        packet_directory,
        plan,
        limits,
    )?)
}

fn recover_or_build_archive(
    packet_directory: &Path,
    archive_directory: &Path,
    limits: ArchiveLimits,
) -> Result<ArchiveReceipt, SealArtifactError> {
    if fs::symlink_metadata(archive_directory).is_ok() {
        let verified = verify_archive_from_outer(archive_directory, limits.maximum_archive_bytes)?;
        return Ok(ArchiveReceipt {
            archive_path: verified.archive_path,
            outer_digest_path: verified.outer_digest_path,
            archive_sha256: verified.archive_sha256,
            archive_bytes: verified.archive_bytes,
            input_files: count_packet_files(packet_directory)?,
            input_bytes: count_packet_bytes(packet_directory)?,
        });
    }
    let staging = building_path(archive_directory)?;
    if fs::symlink_metadata(&staging).is_ok() {
        let verified = verify_archive_from_outer(&staging, limits.maximum_archive_bytes)?;
        fs::rename(&staging, archive_directory)?;
        sync_directory(
            archive_directory
                .parent()
                .ok_or(SealArtifactError::Binding)?,
        )?;
        return Ok(ArchiveReceipt {
            archive_path: archive_directory.join(ARCHIVE_NAME),
            outer_digest_path: archive_directory.join(OUTER_DIGEST_NAME),
            archive_sha256: verified.archive_sha256,
            archive_bytes: verified.archive_bytes,
            input_files: count_packet_files(packet_directory)?,
            input_bytes: count_packet_bytes(packet_directory)?,
        });
    }
    Ok(create_deterministic_archive(
        packet_directory,
        archive_directory,
        limits,
    )?)
}

#[allow(clippy::too_many_arguments)]
fn verify_result(
    seal_root: &Path,
    intent: &SealArtifactIntent,
    intent_sha256: &str,
    result: &SealArtifactResult,
    plan: &PacketPlan,
    packet_limits: PacketLimits,
    archive_limits: ArchiveLimits,
) -> Result<(), SealArtifactError> {
    if result.schema_version != 1
        || result.intent_sha256 != intent_sha256
        || result.campaign_id != intent.campaign_id
        || result.role != intent.role
        || result.packet_directory_name != intent.packet_directory_name
        || result.archive_directory_name != intent.archive_directory_name
    {
        return Err(SealArtifactError::Binding);
    }
    let packet = verify_packet(
        &seal_root.join(&result.packet_directory_name),
        plan,
        packet_limits,
    )?;
    if packet.packet_manifest_sha256 != result.packet_manifest_sha256
        || packet.raw_manifest_sha256 != result.raw_manifest_sha256
        || packet.raw_manifest_set_sha256 != result.raw_manifest_set_sha256
        || packet.raw_file_count != result.raw_file_count
        || packet.raw_total_bytes != result.raw_total_bytes
    {
        return Err(SealArtifactError::Binding);
    }
    verify_archive(
        &seal_root.join(&result.archive_directory_name),
        &result.archive_sha256,
        result.archive_bytes,
        archive_limits.maximum_archive_bytes,
    )?;
    if count_packet_files(&packet.packet_directory)? != result.archive_input_files
        || count_packet_bytes(&packet.packet_directory)? != result.archive_input_bytes
    {
        return Err(SealArtifactError::Binding);
    }
    Ok(())
}

fn verify_archive_from_outer(
    directory: &Path,
    maximum_archive_bytes: u64,
) -> Result<ArchiveVerification, SealArtifactError> {
    let outer = read_bounded_file(&directory.join(OUTER_DIGEST_NAME), 65)?;
    let digest = outer
        .strip_suffix(b"\n")
        .and_then(|value| std::str::from_utf8(value).ok())
        .filter(|value| is_hash(value))
        .ok_or(SealArtifactError::Binding)?;
    let bytes = fs::symlink_metadata(directory.join(ARCHIVE_NAME))?.len();
    Ok(verify_archive(
        directory,
        digest,
        bytes,
        maximum_archive_bytes,
    )?)
}

fn validate_inputs(
    campaign_directory: &Path,
    seal_root: &Path,
    request_id: &str,
    request_sha256: &str,
    role: &Role,
    plan: &PacketPlan,
) -> Result<(), SealArtifactError> {
    if request_id.is_empty()
        || request_id.len() > 128
        || request_id.contains(['\0', '\n', '\r'])
        || !is_hash(request_sha256)
        || campaign_directory
            .file_name()
            .and_then(|value| value.to_str())
            != Some(plan.campaign_id.as_str())
        || !campaign_directory.is_absolute()
        || !seal_root.is_absolute()
    {
        return Err(SealArtifactError::Binding);
    }
    let has_i74 = plan.roles.iter().any(|item| item.role == Role::I74);
    let has_c74 = plan.roles.iter().any(|item| item.role == Role::C74);
    if !has_i74
        || (*role == Role::I74 && (has_c74 || plan.roles.len() != 1 || plan.promotable))
        || (*role == Role::C74 && (!has_c74 || plan.roles.len() != 2))
    {
        return Err(SealArtifactError::Binding);
    }
    Ok(())
}

fn persist_or_verify_document(path: &Path, bytes: &[u8]) -> Result<(), SealArtifactError> {
    let digest_path = sidecar(path)?;
    match fs::symlink_metadata(path) {
        Ok(_) => {
            if read_bounded_file(path, 1024 * 1024)? != bytes {
                return Err(SealArtifactError::Binding);
            }
            verify_or_recover_sidecar(path, bytes)?;
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            write_new_synced(path, bytes)?;
            write_new_synced(&digest_path, format!("{}\n", sha256_hex(bytes)).as_bytes())?;
            sync_directory(path.parent().ok_or(SealArtifactError::Binding)?)?;
            Ok(())
        }
        Err(error) => Err(error.into()),
    }
}

fn read_canonical_document<T: for<'de> Deserialize<'de>>(
    path: &Path,
) -> Result<T, SealArtifactError> {
    let bytes = read_bounded_file(path, 1024 * 1024)?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    if canonical_json(&value)? != bytes {
        return Err(SealArtifactError::Binding);
    }
    verify_or_recover_sidecar(path, &bytes)?;
    Ok(serde_json::from_value(value)?)
}

fn verify_or_recover_sidecar(path: &Path, bytes: &[u8]) -> Result<(), SealArtifactError> {
    let digest_path = sidecar(path)?;
    let expected = format!("{}\n", sha256_hex(bytes));
    match fs::symlink_metadata(&digest_path) {
        Ok(_) => {
            if read_bounded_file(&digest_path, 65)? != expected.as_bytes() {
                return Err(SealArtifactError::Binding);
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            write_new_synced(&digest_path, expected.as_bytes())?;
            sync_directory(path.parent().ok_or(SealArtifactError::Binding)?)?;
        }
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

fn read_bounded_file(path: &Path, maximum: u64) -> Result<Vec<u8>, SealArtifactError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > maximum
        || has_multiple_links(&metadata)
    {
        return Err(SealArtifactError::Binding);
    }
    let mut bytes = Vec::with_capacity(usize::try_from(metadata.len()).unwrap_or(0));
    File::open(path)?
        .take(maximum + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 != metadata.len() {
        return Err(SealArtifactError::Binding);
    }
    Ok(bytes)
}

fn write_new_synced(path: &Path, bytes: &[u8]) -> Result<(), SealArtifactError> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn sidecar(path: &Path) -> Result<PathBuf, SealArtifactError> {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or(SealArtifactError::Binding)?;
    Ok(path.with_file_name(format!("{name}.sha256")))
}

fn building_path(path: &Path) -> Result<PathBuf, SealArtifactError> {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or(SealArtifactError::Binding)?;
    Ok(path.with_file_name(format!(".{name}.building")))
}

fn count_packet_files(root: &Path) -> Result<usize, SealArtifactError> {
    let mut count = 0_usize;
    visit_packet(root, &mut count, &mut 0)?;
    Ok(count)
}

fn count_packet_bytes(root: &Path) -> Result<u64, SealArtifactError> {
    let mut bytes = 0_u64;
    visit_packet(root, &mut 0, &mut bytes)?;
    Ok(bytes)
}

fn visit_packet(
    directory: &Path,
    count: &mut usize,
    bytes: &mut u64,
) -> Result<(), SealArtifactError> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Err(SealArtifactError::Binding);
        }
        if metadata.is_dir() {
            visit_packet(&path, count, bytes)?;
        } else if metadata.is_file() {
            if has_multiple_links(&metadata) {
                return Err(SealArtifactError::Binding);
            }
            *count = count.checked_add(1).ok_or(SealArtifactError::Binding)?;
            *bytes = bytes
                .checked_add(metadata.len())
                .ok_or(SealArtifactError::Binding)?;
        } else {
            return Err(SealArtifactError::Binding);
        }
    }
    Ok(())
}

fn canonical_directory(path: &Path) -> Result<PathBuf, SealArtifactError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(SealArtifactError::Binding);
    }
    Ok(fs::canonicalize(path)?)
}

fn role_name(role: &Role) -> &'static str {
    match role {
        Role::I74 => "i74",
        Role::C74 => "c74",
    }
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
fn sync_directory(path: &Path) -> Result<(), SealArtifactError> {
    File::open(path)?.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> Result<(), SealArtifactError> {
    Ok(())
}
