use crate::protocol::Request;
use crate::state::FrozenIdentity;
use crate::{canonical_json, sha256_hex};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use thiserror::Error;

pub const MAX_MANIFEST_BYTES: usize = 65_536;

const INSTALLED_BINARY_ROOT: &str = "/opt/hydracache-performance/0.74/";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstalledBinary {
    pub role: String,
    pub path: String,
    pub sha256: String,
    pub size: u64,
    pub inode: u64,
    pub device: u64,
    pub uid: u64,
    pub gid: u64,
    pub mode: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoleArgvTemplates {
    pub i74: Vec<String>,
    pub c74: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhaseDurationsSeconds {
    pub warmup: u64,
    pub measured: u64,
    pub drain: u64,
    pub durable_companion: u64,
    pub post_work_idle: u64,
    pub reconciliation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputLimits {
    pub stdout_bytes: u64,
    pub stderr_bytes: u64,
    pub diagnostic_bytes: u64,
    pub final_artifact_bytes: u64,
    pub files: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedOutputSchemaSha256s {
    pub checkpoint: String,
    pub measurement: String,
    pub reconciliation: String,
    pub raw_manifest: String,
    pub packet_manifest: String,
}

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
    pub i74_tree_sha: String,
    pub c74_tree_sha: String,
    pub i74_cargo_lock_sha256: String,
    pub c74_cargo_lock_sha256: String,
    pub i74_dirty: bool,
    pub c74_dirty: bool,
    pub scenario_sha256: String,
    pub workload_sha256: String,
    pub offered_load_sha256: String,
    pub estimator_sha256: String,
    pub thresholds_sha256: String,
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
    pub installed_binaries: Vec<InstalledBinary>,
    pub argv_templates: RoleArgvTemplates,
    pub command_environment_sha256: String,
    pub role_order: Vec<String>,
    pub phase_durations_seconds: PhaseDurationsSeconds,
    pub output_limits: OutputLimits,
    pub expected_output_schema_sha256s: ExpectedOutputSchemaSha256s,
    pub required_final_guards: Vec<String>,
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
    let manifest = parse_document(bytes, &request.manifest_sha256)?;
    validate(
        &manifest,
        &request.campaign_id,
        request.controller.repository_id,
        now_unix_seconds,
    )?;
    Ok(manifest)
}

pub fn parse_stored_and_validate(
    bytes: &[u8],
    expected_sha256: &str,
    expected_campaign_id: &str,
) -> Result<CampaignManifest, ManifestError> {
    let manifest = parse_document(bytes, expected_sha256)?;
    validate(
        &manifest,
        expected_campaign_id,
        manifest.repository_id,
        manifest
            .product_lease_deadline_unix_seconds
            .saturating_sub(1),
    )?;
    Ok(manifest)
}

fn parse_document(bytes: &[u8], expected_sha256: &str) -> Result<CampaignManifest, ManifestError> {
    if bytes.is_empty() || bytes.len() > MAX_MANIFEST_BYTES {
        return Err(ManifestError::Document);
    }
    let encoded = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    if encoded.is_empty() || encoded.contains(&b'\n') || encoded.contains(&b'\r') {
        return Err(ManifestError::Document);
    }
    if hex(&Sha256::digest(encoded)) != expected_sha256 {
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
    Ok(manifest)
}

pub fn frozen_identity_from_manifest(
    manifest: &CampaignManifest,
    manifest_sha256: &str,
) -> Result<FrozenIdentity, serde_json::Error> {
    let source_bundle_sha256 = hash_json(&(
        &manifest.i74_source_sha,
        &manifest.c74_source_sha,
        &manifest.i74_tree_sha,
        &manifest.c74_tree_sha,
        &manifest.i74_cargo_lock_sha256,
        &manifest.c74_cargo_lock_sha256,
    ))?;
    let binary_bundle_sha256 = hash_json(&manifest.installed_binaries)?;
    let workload_bundle_sha256 = hash_json(&(
        &manifest.workload_sha256,
        &manifest.offered_load_sha256,
        &manifest.estimator_sha256,
        &manifest.thresholds_sha256,
        manifest.seed,
        manifest.checkpoint_cadence_seconds,
        manifest.progress_warning_gap_seconds,
        manifest.progress_rejection_gap_seconds,
        manifest.diagnostic_grace_seconds,
        &manifest.argv_templates,
        &manifest.phase_durations_seconds,
        &manifest.output_limits,
        &manifest.expected_output_schema_sha256s,
        &manifest.required_final_guards,
        &manifest.secret_identifiers,
    ))?;
    Ok(FrozenIdentity {
        campaign_id: manifest.campaign_id.clone(),
        manifest_sha256: manifest_sha256.to_owned(),
        contract_sha256: manifest.contract_sha256.clone(),
        scenario_sha256: manifest.scenario_sha256.clone(),
        tooling_sha256: sha256_hex(manifest.tooling_sha.as_bytes()),
        source_bundle_sha256,
        binary_bundle_sha256,
        workload_bundle_sha256,
        machine_id: manifest.machine_id.clone(),
        boot_id: manifest.boot_id.clone(),
        host_receipt_sha256: manifest.host_receipt_sha256.clone(),
        mount_identity: manifest.mount_identity.clone(),
        isolated_cpuset: manifest.isolated_cpuset.clone(),
        housekeeping_cpuset: manifest.housekeeping_cpuset.clone(),
        command_environment_sha256: manifest.command_environment_sha256.clone(),
        lease_id: manifest.lease_id.clone(),
        lease_deadline_unix_seconds: manifest.product_lease_deadline_unix_seconds,
    })
}

fn hash_json<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    Ok(sha256_hex(&canonical_json(value)?))
}

fn validate(
    manifest: &CampaignManifest,
    expected_campaign_id: &str,
    expected_repository_id: u64,
    now_unix_seconds: u64,
) -> Result<(), ManifestError> {
    if manifest.campaign_id != expected_campaign_id
        || manifest.repository_id != expected_repository_id
    {
        return Err(ManifestError::Binding);
    }
    if manifest.schema_version != 1
        || manifest.release != "0.74"
        || manifest.dirty
        || manifest.i74_dirty
        || manifest.c74_dirty
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
        || !is_hash(&manifest.i74_cargo_lock_sha256)
        || !is_hash(&manifest.c74_cargo_lock_sha256)
        || !is_hash(&manifest.workload_sha256)
        || !is_hash(&manifest.offered_load_sha256)
        || !is_hash(&manifest.estimator_sha256)
        || !is_hash(&manifest.thresholds_sha256)
        || !is_hash(&manifest.host_receipt_sha256)
        || !is_hash(&manifest.nonce_sha256)
        || !is_hash(&manifest.command_environment_sha256)
        || !is_git_sha(&manifest.tooling_sha)
        || !is_git_sha(&manifest.i74_source_sha)
        || !is_git_sha(&manifest.c74_source_sha)
        || !is_git_sha(&manifest.i74_tree_sha)
        || !is_git_sha(&manifest.c74_tree_sha)
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
    if manifest.secret_identifiers.len() > 32
        || manifest.secret_identifiers.iter().any(|value| {
            value.is_empty()
                || value.len() > 128
                || value.contains('=')
                || value.contains('\n')
                || value.contains('\r')
        })
    {
        return Err(ManifestError::Invariant);
    }
    validate_execution_contract(manifest)?;
    Ok(())
}

fn validate_execution_contract(manifest: &CampaignManifest) -> Result<(), ManifestError> {
    if manifest.role_order != ["i74", "c74"]
        || manifest.installed_binaries.len() != 2
        || manifest.required_final_guards.is_empty()
        || manifest.required_final_guards.len() > 128
        || manifest.phase_durations_seconds.warmup == 0
        || manifest.phase_durations_seconds.measured == 0
        || manifest.phase_durations_seconds.drain == 0
        || manifest.phase_durations_seconds.durable_companion == 0
        || manifest.phase_durations_seconds.post_work_idle == 0
        || manifest.phase_durations_seconds.reconciliation == 0
        || manifest.output_limits.stdout_bytes == 0
        || manifest.output_limits.stderr_bytes == 0
        || manifest.output_limits.diagnostic_bytes == 0
        || manifest.output_limits.final_artifact_bytes == 0
        || manifest.output_limits.files == 0
        || manifest.output_limits.final_artifact_bytes > manifest.maximum_campaign_bytes
        || manifest.output_limits.files > manifest.maximum_campaign_files
    {
        return Err(ManifestError::Invariant);
    }

    for (binary, role) in manifest.installed_binaries.iter().zip(["i74", "c74"]) {
        if binary.role != role
            || !valid_installed_path(&binary.path)
            || !is_hash(&binary.sha256)
            || binary.size == 0
            || binary.inode == 0
            || binary.device == 0
            || binary.mode > 0o7777
        {
            return Err(ManifestError::Invariant);
        }
    }
    validate_argv(
        &manifest.argv_templates.i74,
        &manifest.installed_binaries[0].path,
    )?;
    validate_argv(
        &manifest.argv_templates.c74,
        &manifest.installed_binaries[1].path,
    )?;

    for digest in [
        &manifest.expected_output_schema_sha256s.checkpoint,
        &manifest.expected_output_schema_sha256s.measurement,
        &manifest.expected_output_schema_sha256s.reconciliation,
        &manifest.expected_output_schema_sha256s.raw_manifest,
        &manifest.expected_output_schema_sha256s.packet_manifest,
    ] {
        if !is_hash(digest) {
            return Err(ManifestError::Invariant);
        }
    }

    let mut guards = HashSet::new();
    if manifest.required_final_guards.iter().any(|guard| {
        guard.is_empty()
            || guard.len() > 128
            || guard.contains('\0')
            || !guards.insert(guard.as_str())
    }) {
        return Err(ManifestError::Invariant);
    }
    Ok(())
}

fn valid_installed_path(path: &str) -> bool {
    path.starts_with(INSTALLED_BINARY_ROOT)
        && path.len() > INSTALLED_BINARY_ROOT.len()
        && path.len() <= 512
        && !path.split('/').any(|component| component == "..")
        && !path.contains('\0')
        && !path.contains('\n')
        && !path.contains('\r')
}

fn validate_argv(argv: &[String], expected_binary: &str) -> Result<(), ManifestError> {
    if argv.is_empty()
        || argv.len() > 128
        || argv.first().map(String::as_str) != Some(expected_binary)
        || argv
            .iter()
            .any(|item| item.is_empty() || item.len() > 1024 || item.contains('\0'))
    {
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
