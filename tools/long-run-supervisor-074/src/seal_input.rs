use crate::artifact::{
    verify_packet, GuardEvidenceInput, PacketLimits, PacketPlan, PacketResult, RoleEvidenceInput,
};
use crate::manifest::CampaignManifest;
use crate::seal_artifact::{load_seal_artifact_result, SealArtifactError};
use crate::{canonical_json, is_hash, sha256_hex, Role};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use thiserror::Error;

pub const SEAL_INPUT_INVENTORY_NAME: &str = "seal-input-inventory.json";
const RELEASE: &str = "0.74";
const MAXIMUM_INVENTORY_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryGuardEvidence {
    pub id: String,
    pub passed: bool,
    pub source_relative_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SealInputInventory {
    pub schema_version: u32,
    pub release: String,
    pub campaign_id: String,
    pub campaign_manifest_sha256: String,
    pub role: Role,
    pub result: PacketResult,
    pub terminal_reason: Option<String>,
    pub journal_relative_path: PathBuf,
    pub guard_evidence: Vec<InventoryGuardEvidence>,
    pub raw_files: Vec<PathBuf>,
}

#[derive(Debug, Error)]
pub enum SealInputError {
    #[error("seal input inventory is malformed, non-canonical, or unsafe")]
    Document,
    #[error("seal input inventory does not match the frozen campaign contract")]
    Binding,
    #[error("seal input inventory exceeds the frozen file or byte limit")]
    Limit,
    #[error("I74 continuation artifact is invalid: {0}")]
    Continuation(#[from] SealArtifactError),
    #[error("seal input I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("seal input JSON failed: {0}")]
    Json(#[from] serde_json::Error),
}

pub fn resolve_packet_plan(
    campaign_directory: &Path,
    seal_root: &Path,
    manifest: &CampaignManifest,
    campaign_manifest_sha256: &str,
    role: Role,
) -> Result<PacketPlan, SealInputError> {
    let campaign_directory = canonical_directory(campaign_directory)?;
    if Some(manifest.campaign_id.as_str())
        != campaign_directory
            .file_name()
            .and_then(|value| value.to_str())
        || !is_hash(campaign_manifest_sha256)
    {
        return Err(SealInputError::Binding);
    }
    let manifest_bytes = read_safe_file(
        &campaign_directory,
        Path::new("campaign-start.json"),
        crate::manifest::MAX_MANIFEST_BYTES as u64,
    )?;
    let encoded_manifest = manifest_bytes
        .strip_suffix(b"\n")
        .unwrap_or(&manifest_bytes);
    if encoded_manifest.is_empty()
        || encoded_manifest.contains(&b'\n')
        || encoded_manifest.contains(&b'\r')
        || sha256_hex(encoded_manifest) != campaign_manifest_sha256
    {
        return Err(SealInputError::Binding);
    }

    let active = read_inventory(
        &campaign_directory,
        manifest,
        campaign_manifest_sha256,
        role.clone(),
    )?;
    let mut inventories = vec![active.clone()];
    let continuation_packet_sha256 = if role == Role::C74 {
        let continuation = read_inventory(
            &campaign_directory,
            manifest,
            campaign_manifest_sha256,
            Role::I74,
        )?;
        if continuation.result != PacketResult::Complete {
            return Err(SealInputError::Binding);
        }
        let result = load_seal_artifact_result(&campaign_directory, Role::I74)?;
        if result.campaign_id != manifest.campaign_id || result.role != Role::I74 {
            return Err(SealInputError::Binding);
        }
        let continuation_plan = build_plan(
            manifest,
            campaign_manifest_sha256,
            continuation.clone(),
            vec![continuation.clone()],
            None,
        )?;
        let receipt = verify_packet(
            &canonical_directory(seal_root)?.join(&result.packet_directory_name),
            &continuation_plan,
            packet_limits(manifest)?,
        )
        .map_err(|_| SealInputError::Binding)?;
        if receipt.packet_manifest_sha256 != result.packet_manifest_sha256 {
            return Err(SealInputError::Binding);
        }
        verify_source_matches_continuation(
            &campaign_directory,
            &receipt.packet_directory,
            &continuation.raw_files,
            manifest.output_limits.final_artifact_bytes,
        )?;
        inventories.push(continuation);
        Some(result.packet_manifest_sha256)
    } else {
        None
    };

    build_plan(
        manifest,
        campaign_manifest_sha256,
        active,
        inventories,
        continuation_packet_sha256,
    )
}

fn build_plan(
    manifest: &CampaignManifest,
    campaign_manifest_sha256: &str,
    active: SealInputInventory,
    inventories: Vec<SealInputInventory>,
    continuation_packet_sha256: Option<String>,
) -> Result<PacketPlan, SealInputError> {
    let mut raw_files = inventories
        .iter()
        .flat_map(|inventory| inventory.raw_files.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    raw_files.sort_by_key(|path| normalized(path));
    if raw_files.len() > usize::try_from(manifest.output_limits.files).unwrap_or(usize::MAX) {
        return Err(SealInputError::Limit);
    }

    let mut roles = inventories
        .iter()
        .map(|inventory| RoleEvidenceInput {
            role: inventory.role.clone(),
            result: inventory.result,
            journal_relative_path: inventory.journal_relative_path.clone(),
        })
        .collect::<Vec<_>>();
    roles.sort_by_key(|input| match input.role {
        Role::I74 => 0,
        Role::C74 => 1,
    });

    Ok(PacketPlan {
        campaign_id: manifest.campaign_id.clone(),
        campaign_manifest_sha256: campaign_manifest_sha256.to_owned(),
        continuation_packet_sha256,
        result: active.result,
        promotable: active.role == Role::C74 && active.result == PacketResult::Complete,
        terminal_reason: active.terminal_reason,
        required_final_guards: manifest.required_final_guards.clone(),
        guard_evidence: active
            .guard_evidence
            .into_iter()
            .map(|guard| GuardEvidenceInput {
                id: guard.id,
                passed: guard.passed,
                source_relative_path: guard.source_relative_path,
            })
            .collect(),
        roles,
        raw_files,
    })
}

fn packet_limits(manifest: &CampaignManifest) -> Result<PacketLimits, SealInputError> {
    let maximum_files =
        usize::try_from(manifest.output_limits.files).map_err(|_| SealInputError::Limit)?;
    if maximum_files == 0 || manifest.output_limits.final_artifact_bytes == 0 {
        return Err(SealInputError::Limit);
    }
    Ok(PacketLimits {
        maximum_files,
        maximum_bytes: manifest.output_limits.final_artifact_bytes,
    })
}

fn read_inventory(
    campaign_directory: &Path,
    manifest: &CampaignManifest,
    campaign_manifest_sha256: &str,
    role: Role,
) -> Result<SealInputInventory, SealInputError> {
    let role_name = role_name(&role);
    let relative = PathBuf::from(format!("roles/{role_name}/{SEAL_INPUT_INVENTORY_NAME}"));
    let bytes = read_safe_file(campaign_directory, &relative, MAXIMUM_INVENTORY_BYTES)?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    if canonical_json(&value)? != bytes {
        return Err(SealInputError::Document);
    }
    let inventory: SealInputInventory = serde_json::from_value(value)?;
    validate_inventory(
        campaign_directory,
        manifest,
        campaign_manifest_sha256,
        &role,
        &relative,
        &inventory,
    )?;
    Ok(inventory)
}

fn validate_inventory(
    campaign_directory: &Path,
    manifest: &CampaignManifest,
    campaign_manifest_sha256: &str,
    role: &Role,
    inventory_relative_path: &Path,
    inventory: &SealInputInventory,
) -> Result<(), SealInputError> {
    if inventory.schema_version != 1
        || inventory.release != RELEASE
        || inventory.campaign_id != manifest.campaign_id
        || inventory.campaign_manifest_sha256 != campaign_manifest_sha256
        || &inventory.role != role
        || inventory.raw_files.is_empty()
        || inventory.raw_files.len()
            > usize::try_from(manifest.output_limits.files).unwrap_or(usize::MAX)
    {
        return Err(SealInputError::Binding);
    }
    match (inventory.result, inventory.terminal_reason.as_deref()) {
        (PacketResult::Complete, None) => {}
        (PacketResult::Incomplete, Some(reason))
            if !reason.is_empty()
                && reason.len() <= 128
                && !reason.contains(['\0', '\n', '\r']) => {}
        _ => return Err(SealInputError::Binding),
    }

    let role_root = PathBuf::from(format!("roles/{}/", role_name(role)));
    let expected_journal = role_root.join("checkpoints.jsonl");
    if inventory.journal_relative_path != expected_journal {
        return Err(SealInputError::Binding);
    }
    let raw_files = inventory
        .raw_files
        .iter()
        .map(|path| {
            validate_relative_path(path)?;
            Ok(path.clone())
        })
        .collect::<Result<BTreeSet<_>, SealInputError>>()?;
    if raw_files.len() != inventory.raw_files.len()
        || !raw_files.contains(Path::new("campaign-start.json"))
        || !raw_files.contains(inventory_relative_path)
        || !raw_files.contains(&expected_journal)
        || raw_files
            .iter()
            .any(|path| path != Path::new("campaign-start.json") && !path.starts_with(&role_root))
    {
        return Err(SealInputError::Binding);
    }

    let required = manifest
        .required_final_guards
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let provided = inventory
        .guard_evidence
        .iter()
        .map(|guard| guard.id.as_str())
        .collect::<BTreeSet<_>>();
    let guard_root = role_root.join("guards");
    if provided != required
        || provided.len() != inventory.guard_evidence.len()
        || inventory.guard_evidence.iter().any(|guard| {
            validate_relative_path(&guard.source_relative_path).is_err()
                || !guard.source_relative_path.starts_with(&guard_root)
                || !raw_files.contains(&guard.source_relative_path)
        })
    {
        return Err(SealInputError::Binding);
    }

    let mut total = 0_u64;
    for relative in &raw_files {
        let bytes = file_size(campaign_directory, relative)?;
        total = total.checked_add(bytes).ok_or(SealInputError::Limit)?;
        if total > manifest.output_limits.final_artifact_bytes
            || total > manifest.maximum_campaign_bytes
        {
            return Err(SealInputError::Limit);
        }
    }
    Ok(())
}

fn canonical_directory(path: &Path) -> Result<PathBuf, SealInputError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(SealInputError::Document);
    }
    Ok(fs::canonicalize(path)?)
}

fn read_safe_file(
    root: &Path,
    relative: &Path,
    maximum_bytes: u64,
) -> Result<Vec<u8>, SealInputError> {
    let size = file_size(root, relative)?;
    if size == 0 || size > maximum_bytes {
        return Err(SealInputError::Document);
    }
    let mut bytes = Vec::with_capacity(usize::try_from(size).unwrap_or(0));
    File::open(root.join(relative))?
        .take(maximum_bytes + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 != size {
        return Err(SealInputError::Document);
    }
    Ok(bytes)
}

fn verify_source_matches_continuation(
    campaign_directory: &Path,
    packet_directory: &Path,
    raw_files: &[PathBuf],
    maximum_bytes: u64,
) -> Result<(), SealInputError> {
    for relative in raw_files {
        let source = file_digest(campaign_directory, relative, maximum_bytes)?;
        let sealed = file_digest(&packet_directory.join("raw"), relative, maximum_bytes)?;
        if source != sealed {
            return Err(SealInputError::Binding);
        }
    }
    Ok(())
}

fn file_digest(
    root: &Path,
    relative: &Path,
    maximum_bytes: u64,
) -> Result<(u64, [u8; 32]), SealInputError> {
    let expected_size = file_size(root, relative)?;
    if expected_size > maximum_bytes {
        return Err(SealInputError::Limit);
    }
    let mut file = File::open(root.join(relative))?;
    let mut digest = Sha256::new();
    let mut total = 0_u64;
    let mut buffer = vec![0_u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        total = total
            .checked_add(read as u64)
            .ok_or(SealInputError::Limit)?;
        if total > maximum_bytes {
            return Err(SealInputError::Limit);
        }
        digest.update(&buffer[..read]);
    }
    if total != expected_size || file.metadata()?.len() != expected_size {
        return Err(SealInputError::Document);
    }
    Ok((total, digest.finalize().into()))
}

fn file_size(root: &Path, relative: &Path) -> Result<u64, SealInputError> {
    validate_relative_path(relative)?;
    reject_symlink_components(root, relative)?;
    let path = root.join(relative);
    let metadata = fs::symlink_metadata(&path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || has_multiple_links(&metadata) {
        return Err(SealInputError::Document);
    }
    Ok(metadata.len())
}

fn reject_symlink_components(root: &Path, relative: &Path) -> Result<(), SealInputError> {
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component.as_os_str());
        let metadata = fs::symlink_metadata(&current)?;
        if metadata.file_type().is_symlink() {
            return Err(SealInputError::Document);
        }
    }
    Ok(())
}

fn validate_relative_path(path: &Path) -> Result<(), SealInputError> {
    let normalized = normalized(path);
    if path.is_absolute()
        || normalized.is_empty()
        || normalized.len() > 1024
        || normalized.contains('\\')
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(SealInputError::Document);
    }
    Ok(())
}

fn normalized(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
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
