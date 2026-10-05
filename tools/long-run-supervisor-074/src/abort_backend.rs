use crate::abort_lifecycle::AbortBackend;
use crate::lease_expiry::{LeaseExpiryBackend, LeaseExpiryCause};
use crate::manifest::CampaignManifest;
use crate::progress_loss::{ProgressLossBackend, ProgressLossCause};
use crate::protocol::Request;
use crate::state::DurableCampaignState;
use crate::systemd_unit::{
    inspect_unit_optional, stop_unit_and_wait, verify_unit_identity, verify_unit_terminal,
    UnitSnapshot,
};
use crate::{canonical_json, Role};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

const MAX_DIAGNOSTIC_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AbortDiagnostic {
    schema_version: u32,
    release: String,
    campaign_id: String,
    request_id: String,
    request_sha256: String,
    role: Role,
    state: DurableCampaignState,
    unit: UnitSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LeaseExpiryDiagnostic {
    schema_version: u32,
    release: String,
    campaign_id: String,
    lease_id: String,
    lease_deadline_unix_seconds: u64,
    cause_sha256: String,
    role: Role,
    state: DurableCampaignState,
    unit: UnitSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProgressLossDiagnostic {
    schema_version: u32,
    release: String,
    campaign_id: String,
    cause_sha256: String,
    role: Role,
    state: DurableCampaignState,
    unit: UnitSnapshot,
}

#[derive(Debug, Error)]
pub enum SystemdAbortError {
    #[error("abort backend was not bound to verified manifest evidence")]
    Manifest,
    #[error("abort diagnostic path, identity, or existing bytes are unsafe")]
    Binding,
    #[error("abort diagnostic exceeds the frozen byte limit")]
    Limit,
    #[error("abort unit observation or stop failed: {0}")]
    Unit(#[from] crate::systemd_unit::UnitError),
    #[error("abort diagnostic I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("abort diagnostic JSON failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("abort request digest failed: {0}")]
    Event(#[from] crate::event::EventError),
}

#[derive(Debug, Default)]
pub struct SystemdAbortBackend {
    manifest: Option<CampaignManifest>,
}

impl SystemdAbortBackend {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bind_manifest(&mut self, manifest: CampaignManifest) {
        self.manifest = Some(manifest);
    }
}

impl AbortBackend for SystemdAbortBackend {
    fn capture_and_stop(
        &mut self,
        campaign_directory: &Path,
        request: &Request,
        state: &DurableCampaignState,
    ) -> Result<(), String> {
        self.capture_and_stop_inner(campaign_directory, request, state)
            .map_err(|error| error.to_string())
    }
}

impl LeaseExpiryBackend for SystemdAbortBackend {
    fn capture_and_stop_expired(
        &mut self,
        campaign_directory: &Path,
        cause: &LeaseExpiryCause,
        state: &DurableCampaignState,
    ) -> Result<(), String> {
        self.capture_and_stop_expired_inner(campaign_directory, cause, state)
            .map_err(|error| error.to_string())
    }
}

impl ProgressLossBackend for SystemdAbortBackend {
    fn capture_and_stop_stalled(
        &mut self,
        campaign_directory: &Path,
        cause: &ProgressLossCause,
        state: &DurableCampaignState,
    ) -> Result<(), String> {
        self.capture_and_stop_stalled_inner(campaign_directory, cause, state)
            .map_err(|error| error.to_string())
    }
}

impl SystemdAbortBackend {
    fn capture_and_stop_inner(
        &mut self,
        campaign_directory: &Path,
        request: &Request,
        state: &DurableCampaignState,
    ) -> Result<(), SystemdAbortError> {
        let manifest = self.manifest.as_ref().ok_or(SystemdAbortError::Manifest)?;
        if manifest.campaign_id != request.campaign_id
            || manifest.campaign_id != state.identity.campaign_id
        {
            return Err(SystemdAbortError::Binding);
        }
        let harness = state.harness.as_ref().ok_or(SystemdAbortError::Binding)?;
        let daemon = state.daemon.as_ref().ok_or(SystemdAbortError::Binding)?;
        let role = role_from_unit(&harness.unit_name, &request.campaign_id)?;
        let diagnostic_path = diagnostic_path(campaign_directory, &role, &request.request_id)?;
        match inspect_unit_optional(&harness.unit_name)? {
            Some(snapshot) => {
                if snapshot.sub_state == "running" {
                    verify_unit_identity(harness, daemon, &snapshot)?;
                    publish_diagnostic(
                        &diagnostic_path,
                        request,
                        state,
                        role,
                        snapshot,
                        manifest.output_limits.diagnostic_bytes,
                    )?;
                } else if snapshot.active_state == "active" && snapshot.sub_state == "exited" {
                    verify_unit_terminal(harness, daemon, &snapshot)?;
                    publish_diagnostic(
                        &diagnostic_path,
                        request,
                        state,
                        role,
                        snapshot,
                        manifest.output_limits.diagnostic_bytes,
                    )?;
                } else {
                    if snapshot.unit_name != harness.unit_name
                        || snapshot.control_group != harness.cgroup_path
                        || (snapshot.main_pid != 0 && snapshot.main_pid != harness.pid)
                    {
                        return Err(SystemdAbortError::Binding);
                    }
                    verify_existing_diagnostic(
                        &diagnostic_path,
                        request,
                        state,
                        &role,
                        manifest.output_limits.diagnostic_bytes,
                    )?;
                }
                stop_unit_and_wait(&harness.unit_name, manifest.diagnostic_grace_seconds)?;
            }
            None => {
                verify_existing_diagnostic(
                    &diagnostic_path,
                    request,
                    state,
                    &role,
                    manifest.output_limits.diagnostic_bytes,
                )?;
            }
        }
        Ok(())
    }

    fn capture_and_stop_expired_inner(
        &mut self,
        campaign_directory: &Path,
        cause: &LeaseExpiryCause,
        state: &DurableCampaignState,
    ) -> Result<(), SystemdAbortError> {
        let manifest = self.manifest.as_ref().ok_or(SystemdAbortError::Manifest)?;
        if manifest.campaign_id != cause.campaign_id
            || manifest.campaign_id != state.identity.campaign_id
            || manifest.lease_id != cause.lease_id
            || manifest.product_lease_deadline_unix_seconds != cause.lease_deadline_unix_seconds
            || state.identity.lease_id != cause.lease_id
            || state.identity.lease_deadline_unix_seconds != cause.lease_deadline_unix_seconds
        {
            return Err(SystemdAbortError::Binding);
        }
        let harness = state.harness.as_ref().ok_or(SystemdAbortError::Binding)?;
        let daemon = state.daemon.as_ref().ok_or(SystemdAbortError::Binding)?;
        let role = role_from_unit(&harness.unit_name, &cause.campaign_id)?;
        let diagnostic_path =
            lease_expiry_diagnostic_path(campaign_directory, &role, &cause.lease_id)?;
        match inspect_unit_optional(&harness.unit_name)? {
            Some(snapshot) => {
                if snapshot.sub_state == "running" {
                    verify_unit_identity(harness, daemon, &snapshot)?;
                    publish_lease_expiry_diagnostic(
                        &diagnostic_path,
                        cause,
                        state,
                        role,
                        snapshot,
                        manifest.output_limits.diagnostic_bytes,
                    )?;
                } else if snapshot.active_state == "active" && snapshot.sub_state == "exited" {
                    verify_unit_terminal(harness, daemon, &snapshot)?;
                    publish_lease_expiry_diagnostic(
                        &diagnostic_path,
                        cause,
                        state,
                        role,
                        snapshot,
                        manifest.output_limits.diagnostic_bytes,
                    )?;
                } else {
                    if snapshot.unit_name != harness.unit_name
                        || snapshot.control_group != harness.cgroup_path
                        || (snapshot.main_pid != 0 && snapshot.main_pid != harness.pid)
                    {
                        return Err(SystemdAbortError::Binding);
                    }
                    verify_existing_lease_expiry_diagnostic(
                        &diagnostic_path,
                        cause,
                        state,
                        &role,
                        manifest.output_limits.diagnostic_bytes,
                    )?;
                }
                stop_unit_and_wait(&harness.unit_name, manifest.diagnostic_grace_seconds)?;
            }
            None => {
                verify_existing_lease_expiry_diagnostic(
                    &diagnostic_path,
                    cause,
                    state,
                    &role,
                    manifest.output_limits.diagnostic_bytes,
                )?;
            }
        }
        Ok(())
    }

    fn capture_and_stop_stalled_inner(
        &mut self,
        campaign_directory: &Path,
        cause: &ProgressLossCause,
        state: &DurableCampaignState,
    ) -> Result<(), SystemdAbortError> {
        let manifest = self.manifest.as_ref().ok_or(SystemdAbortError::Manifest)?;
        if manifest.campaign_id != cause.campaign_id
            || manifest.campaign_id != state.identity.campaign_id
            || manifest.progress_rejection_gap_seconds != cause.rejection_gap_seconds
            || state.checkpoint != cause.checkpoint
        {
            return Err(SystemdAbortError::Binding);
        }
        let harness = state.harness.as_ref().ok_or(SystemdAbortError::Binding)?;
        let daemon = state.daemon.as_ref().ok_or(SystemdAbortError::Binding)?;
        let role = role_from_unit(&harness.unit_name, &cause.campaign_id)?;
        let diagnostic_path =
            progress_loss_diagnostic_path(campaign_directory, &role, &cause.sha256()?)?;
        match inspect_unit_optional(&harness.unit_name)? {
            Some(snapshot) => {
                if snapshot.sub_state == "running" {
                    verify_unit_identity(harness, daemon, &snapshot)?;
                    publish_progress_loss_diagnostic(
                        &diagnostic_path,
                        cause,
                        state,
                        role,
                        snapshot,
                        manifest.output_limits.diagnostic_bytes,
                    )?;
                } else if snapshot.active_state == "active" && snapshot.sub_state == "exited" {
                    verify_unit_terminal(harness, daemon, &snapshot)?;
                    publish_progress_loss_diagnostic(
                        &diagnostic_path,
                        cause,
                        state,
                        role,
                        snapshot,
                        manifest.output_limits.diagnostic_bytes,
                    )?;
                } else {
                    if snapshot.unit_name != harness.unit_name
                        || snapshot.control_group != harness.cgroup_path
                        || (snapshot.main_pid != 0 && snapshot.main_pid != harness.pid)
                    {
                        return Err(SystemdAbortError::Binding);
                    }
                    verify_existing_progress_loss_diagnostic(
                        &diagnostic_path,
                        cause,
                        state,
                        &role,
                        manifest.output_limits.diagnostic_bytes,
                    )?;
                }
                stop_unit_and_wait(&harness.unit_name, manifest.diagnostic_grace_seconds)?;
            }
            None => {
                verify_existing_progress_loss_diagnostic(
                    &diagnostic_path,
                    cause,
                    state,
                    &role,
                    manifest.output_limits.diagnostic_bytes,
                )?;
            }
        }
        Ok(())
    }
}

fn role_from_unit(unit: &str, campaign_id: &str) -> Result<Role, SystemdAbortError> {
    let i74 = format!("hydracache-performance-074-i74-{campaign_id}.service");
    let c74 = format!("hydracache-performance-074-c74-{campaign_id}.service");
    match unit {
        value if value == i74 => Ok(Role::I74),
        value if value == c74 => Ok(Role::C74),
        _ => Err(SystemdAbortError::Binding),
    }
}

fn diagnostic_path(
    campaign_directory: &Path,
    role: &Role,
    request_id: &str,
) -> Result<PathBuf, SystemdAbortError> {
    diagnostic_path_with_prefix(campaign_directory, role, "abort", request_id)
}

fn lease_expiry_diagnostic_path(
    campaign_directory: &Path,
    role: &Role,
    lease_id: &str,
) -> Result<PathBuf, SystemdAbortError> {
    diagnostic_path_with_prefix(campaign_directory, role, "lease-expiry", lease_id)
}

fn progress_loss_diagnostic_path(
    campaign_directory: &Path,
    role: &Role,
    checkpoint_sha256: &str,
) -> Result<PathBuf, SystemdAbortError> {
    if !crate::is_hash(checkpoint_sha256) {
        return Err(SystemdAbortError::Binding);
    }
    diagnostic_path_with_validated_identity(
        campaign_directory,
        role,
        "progress-loss",
        checkpoint_sha256,
    )
}

fn diagnostic_path_with_prefix(
    campaign_directory: &Path,
    role: &Role,
    prefix: &str,
    identity: &str,
) -> Result<PathBuf, SystemdAbortError> {
    if identity.len() != 36
        || !identity
            .bytes()
            .enumerate()
            .all(|(index, byte)| match index {
                8 | 13 | 18 | 23 => byte == b'-',
                _ => byte.is_ascii_hexdigit(),
            })
    {
        return Err(SystemdAbortError::Binding);
    }
    diagnostic_path_with_validated_identity(campaign_directory, role, prefix, identity)
}

fn diagnostic_path_with_validated_identity(
    campaign_directory: &Path,
    role: &Role,
    prefix: &str,
    identity: &str,
) -> Result<PathBuf, SystemdAbortError> {
    let role_name = match role {
        Role::I74 => "i74",
        Role::C74 => "c74",
    };
    let campaign = fs::canonicalize(campaign_directory)?;
    if campaign
        .file_name()
        .and_then(|name| name.to_str())
        .is_none()
    {
        return Err(SystemdAbortError::Binding);
    }
    let role_directory = campaign.join("roles").join(role_name);
    let metadata = fs::symlink_metadata(&role_directory)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(SystemdAbortError::Binding);
    }
    let diagnostics = role_directory.join("diagnostics");
    match fs::symlink_metadata(&diagnostics) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
        Ok(_) => return Err(SystemdAbortError::Binding),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir(&diagnostics)?;
            sync_directory(&role_directory)?;
        }
        Err(error) => return Err(error.into()),
    }
    Ok(diagnostics.join(format!("{prefix}-{identity}.json")))
}

fn publish_diagnostic(
    path: &Path,
    request: &Request,
    state: &DurableCampaignState,
    role: Role,
    unit: UnitSnapshot,
    limit: u64,
) -> Result<(), SystemdAbortError> {
    let document = AbortDiagnostic {
        schema_version: 1,
        release: "0.74".to_owned(),
        campaign_id: request.campaign_id.clone(),
        request_id: request.request_id.clone(),
        request_sha256: crate::event::request_sha256(request)?,
        role,
        state: state.clone(),
        unit,
    };
    let bytes = canonical_json(&document)?;
    enforce_limit(bytes.len() as u64, limit)?;
    publish_exact(path, &bytes)
}

fn verify_existing_diagnostic(
    path: &Path,
    request: &Request,
    state: &DurableCampaignState,
    role: &Role,
    limit: u64,
) -> Result<(), SystemdAbortError> {
    let pending = path.with_extension("json.pending");
    if pending.exists() {
        let final_bytes = read_regular_allow_links(path, limit.min(MAX_DIAGNOSTIC_BYTES))?;
        if read_regular_allow_links(&pending, limit.min(MAX_DIAGNOSTIC_BYTES))? != final_bytes {
            return Err(SystemdAbortError::Binding);
        }
        fs::remove_file(&pending)?;
        sync_directory(path.parent().ok_or(SystemdAbortError::Binding)?)?;
    }
    let bytes = read_regular(path, limit.min(MAX_DIAGNOSTIC_BYTES))?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    if serde_json::to_vec(&value)? != bytes {
        return Err(SystemdAbortError::Binding);
    }
    let document: AbortDiagnostic = serde_json::from_value(value)?;
    if document.schema_version != 1
        || document.release != "0.74"
        || document.campaign_id != request.campaign_id
        || document.request_id != request.request_id
        || document.request_sha256 != crate::event::request_sha256(request)?
        || &document.role != role
        || document.state != *state
    {
        return Err(SystemdAbortError::Binding);
    }
    Ok(())
}

fn publish_lease_expiry_diagnostic(
    path: &Path,
    cause: &LeaseExpiryCause,
    state: &DurableCampaignState,
    role: Role,
    unit: UnitSnapshot,
    limit: u64,
) -> Result<(), SystemdAbortError> {
    let document = LeaseExpiryDiagnostic {
        schema_version: 1,
        release: "0.74".to_owned(),
        campaign_id: cause.campaign_id.clone(),
        lease_id: cause.lease_id.clone(),
        lease_deadline_unix_seconds: cause.lease_deadline_unix_seconds,
        cause_sha256: cause.sha256()?,
        role,
        state: state.clone(),
        unit,
    };
    let bytes = canonical_json(&document)?;
    enforce_limit(bytes.len() as u64, limit)?;
    publish_exact(path, &bytes)
}

fn verify_existing_lease_expiry_diagnostic(
    path: &Path,
    cause: &LeaseExpiryCause,
    state: &DurableCampaignState,
    role: &Role,
    limit: u64,
) -> Result<(), SystemdAbortError> {
    let pending = path.with_extension("json.pending");
    if pending.exists() {
        let final_bytes = read_regular_allow_links(path, limit.min(MAX_DIAGNOSTIC_BYTES))?;
        if read_regular_allow_links(&pending, limit.min(MAX_DIAGNOSTIC_BYTES))? != final_bytes {
            return Err(SystemdAbortError::Binding);
        }
        fs::remove_file(&pending)?;
        sync_directory(path.parent().ok_or(SystemdAbortError::Binding)?)?;
    }
    let bytes = read_regular(path, limit.min(MAX_DIAGNOSTIC_BYTES))?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    if serde_json::to_vec(&value)? != bytes {
        return Err(SystemdAbortError::Binding);
    }
    let document: LeaseExpiryDiagnostic = serde_json::from_value(value)?;
    if document.schema_version != 1
        || document.release != "0.74"
        || document.campaign_id != cause.campaign_id
        || document.lease_id != cause.lease_id
        || document.lease_deadline_unix_seconds != cause.lease_deadline_unix_seconds
        || document.cause_sha256 != cause.sha256()?
        || &document.role != role
        || document.state != *state
    {
        return Err(SystemdAbortError::Binding);
    }
    Ok(())
}

fn publish_progress_loss_diagnostic(
    path: &Path,
    cause: &ProgressLossCause,
    state: &DurableCampaignState,
    role: Role,
    unit: UnitSnapshot,
    limit: u64,
) -> Result<(), SystemdAbortError> {
    let document = ProgressLossDiagnostic {
        schema_version: 1,
        release: "0.74".to_owned(),
        campaign_id: cause.campaign_id.clone(),
        cause_sha256: cause.sha256()?,
        role,
        state: state.clone(),
        unit,
    };
    let bytes = canonical_json(&document)?;
    enforce_limit(bytes.len() as u64, limit)?;
    publish_exact(path, &bytes)
}

fn verify_existing_progress_loss_diagnostic(
    path: &Path,
    cause: &ProgressLossCause,
    state: &DurableCampaignState,
    role: &Role,
    limit: u64,
) -> Result<(), SystemdAbortError> {
    let pending = path.with_extension("json.pending");
    if pending.exists() {
        let final_bytes = read_regular_allow_links(path, limit.min(MAX_DIAGNOSTIC_BYTES))?;
        if read_regular_allow_links(&pending, limit.min(MAX_DIAGNOSTIC_BYTES))? != final_bytes {
            return Err(SystemdAbortError::Binding);
        }
        fs::remove_file(&pending)?;
        sync_directory(path.parent().ok_or(SystemdAbortError::Binding)?)?;
    }
    let bytes = read_regular(path, limit.min(MAX_DIAGNOSTIC_BYTES))?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    if serde_json::to_vec(&value)? != bytes {
        return Err(SystemdAbortError::Binding);
    }
    let document: ProgressLossDiagnostic = serde_json::from_value(value)?;
    if document.schema_version != 1
        || document.release != "0.74"
        || document.campaign_id != cause.campaign_id
        || document.cause_sha256 != cause.sha256()?
        || &document.role != role
        || document.state != *state
    {
        return Err(SystemdAbortError::Binding);
    }
    Ok(())
}

fn enforce_limit(size: u64, limit: u64) -> Result<(), SystemdAbortError> {
    if size == 0 || limit == 0 || limit > MAX_DIAGNOSTIC_BYTES || size > limit {
        return Err(SystemdAbortError::Limit);
    }
    Ok(())
}

fn publish_exact(path: &Path, bytes: &[u8]) -> Result<(), SystemdAbortError> {
    let pending = path.with_extension("json.pending");
    if path.exists() {
        let current = if pending.exists() {
            let current = read_regular_allow_links(path, bytes.len() as u64)?;
            if read_regular_allow_links(&pending, bytes.len() as u64)? != bytes {
                return Err(SystemdAbortError::Binding);
            }
            fs::remove_file(&pending)?;
            sync_directory(path.parent().ok_or(SystemdAbortError::Binding)?)?;
            current
        } else {
            read_regular(path, bytes.len() as u64)?
        };
        return if current == bytes && read_regular(path, bytes.len() as u64)? == bytes {
            Ok(())
        } else {
            Err(SystemdAbortError::Binding)
        };
    }
    let parent = path.parent().ok_or(SystemdAbortError::Binding)?;
    if pending.exists() {
        if read_regular_allow_links(&pending, bytes.len() as u64)? != bytes {
            return Err(SystemdAbortError::Binding);
        }
    } else {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&pending)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    match fs::hard_link(&pending, path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if read_regular_allow_links(path, bytes.len() as u64)? != bytes {
                return Err(SystemdAbortError::Binding);
            }
        }
        Err(error) => return Err(error.into()),
    }
    fs::remove_file(&pending)?;
    sync_directory(parent)?;
    if read_regular(path, bytes.len() as u64)? != bytes {
        return Err(SystemdAbortError::Binding);
    }
    Ok(())
}

fn read_regular(path: &Path, maximum: u64) -> Result<Vec<u8>, SystemdAbortError> {
    let metadata = fs::symlink_metadata(path)?;
    if has_multiple_links(&metadata) {
        return Err(SystemdAbortError::Binding);
    }
    read_regular_inner(path, maximum, &metadata)
}

fn read_regular_allow_links(path: &Path, maximum: u64) -> Result<Vec<u8>, SystemdAbortError> {
    let metadata = fs::symlink_metadata(path)?;
    read_regular_inner(path, maximum, &metadata)
}

fn read_regular_inner(
    path: &Path,
    maximum: u64,
    metadata: &fs::Metadata,
) -> Result<Vec<u8>, SystemdAbortError> {
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() == 0
        || metadata.len() > maximum
    {
        return Err(SystemdAbortError::Binding);
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    File::open(path)?
        .take(maximum.saturating_add(1))
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 != metadata.len() {
        return Err(SystemdAbortError::Binding);
    }
    Ok(bytes)
}

#[cfg(unix)]
fn has_multiple_links(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    metadata.nlink() != 1
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<(), SystemdAbortError> {
    File::open(path)?.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        diagnostic_path, lease_expiry_diagnostic_path, progress_loss_diagnostic_path,
        publish_diagnostic, publish_lease_expiry_diagnostic, publish_progress_loss_diagnostic,
        verify_existing_diagnostic, verify_existing_lease_expiry_diagnostic,
        verify_existing_progress_loss_diagnostic,
    };
    use crate::lease_expiry::LeaseExpiryCause;
    use crate::progress_loss::ProgressLossCause;
    use crate::protocol::{ControllerIdentity, Operation, Request};
    use crate::state::{
        CampaignState, CheckpointHead, ControllerLease, DurableCampaignState, FrozenIdentity,
    };
    use crate::systemd_unit::UnitSnapshot;
    use crate::{ProcessIdentity, Role};

    fn hash(value: char) -> String {
        value.to_string().repeat(64)
    }

    fn fixture() -> (
        tempfile::TempDir,
        std::path::PathBuf,
        Request,
        DurableCampaignState,
        UnitSnapshot,
    ) {
        let temporary = tempfile::tempdir().unwrap();
        let campaign_id = hash('a');
        let campaign = temporary.path().join(&campaign_id);
        std::fs::create_dir_all(campaign.join("roles/i74")).unwrap();
        let unit = format!("hydracache-performance-074-i74-{campaign_id}.service");
        let process = |pid| ProcessIdentity {
            boot_id: "boot-a".to_owned(),
            pid,
            start_ticks: u64::from(pid) * 10,
            process_group: 100,
            cgroup_path: format!("/system.slice/{unit}"),
            cgroup_inode: 500,
            unit_name: unit.clone(),
        };
        let state = DurableCampaignState {
            revision: 6,
            campaign_state: CampaignState::AbortedIncomplete,
            identity: FrozenIdentity {
                campaign_id: campaign_id.clone(),
                manifest_sha256: hash('b'),
                contract_sha256: hash('c'),
                scenario_sha256: hash('d'),
                tooling_sha256: hash('e'),
                source_bundle_sha256: hash('f'),
                binary_bundle_sha256: hash('1'),
                workload_bundle_sha256: hash('2'),
                machine_id: "machine-a".to_owned(),
                boot_id: "boot-a".to_owned(),
                host_receipt_sha256: hash('3'),
                mount_identity: "mount-a".to_owned(),
                isolated_cpuset: "2-7".to_owned(),
                housekeeping_cpuset: "0-1".to_owned(),
                command_environment_sha256: hash('4'),
                lease_id: "00000000-0000-4000-8000-000000000074".to_owned(),
                lease_deadline_unix_seconds: 2_000,
            },
            harness: Some(process(100)),
            daemon: Some(process(101)),
            checkpoint: None,
            controller_lease: Some(ControllerLease {
                holder_request_id: "123e4567-e89b-42d3-a456-426614174000".to_owned(),
                authorization_sha256: hash('5'),
                repository_id: 10,
                run_id: 20,
                actor_id: 30,
                expires_unix_seconds: 1_100,
            }),
            recorded_failure: false,
            duplicate_executor: false,
            durable_history_corrupt: false,
        };
        let request = Request {
            schema_version: 1,
            request_id: "223e4567-e89b-42d3-a456-426614174000".to_owned(),
            operation: Operation::Abort,
            campaign_id,
            expected_state_revision: 5,
            manifest_path: None,
            manifest_sha256: hash('b'),
            controller: ControllerIdentity {
                repository_id: 10,
                run_id: 20,
                run_attempt: 1,
                actor_id: 30,
                authorization_sha256: hash('6'),
            },
            abort_reason: Some("operator-request".to_owned()),
            approval_nonce_sha256: Some(hash('7')),
        };
        let snapshot = UnitSnapshot {
            unit_name: unit,
            active_state: "active".to_owned(),
            sub_state: "running".to_owned(),
            main_pid: 100,
            control_group: state.harness.as_ref().unwrap().cgroup_path.clone(),
            result: "success".to_owned(),
        };
        (temporary, campaign, request, state, snapshot)
    }

    #[test]
    fn diagnostic_is_canonical_bounded_and_exactly_replayed() {
        let (_temporary, campaign, request, state, snapshot) = fixture();
        let path = diagnostic_path(&campaign, &Role::I74, &request.request_id).unwrap();
        publish_diagnostic(
            &path,
            &request,
            &state,
            Role::I74,
            snapshot.clone(),
            1_048_576,
        )
        .unwrap();
        let first = std::fs::read(&path).unwrap();
        std::fs::rename(&path, path.with_extension("json.pending")).unwrap();
        publish_diagnostic(&path, &request, &state, Role::I74, snapshot, 1_048_576).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), first);
        std::fs::hard_link(&path, path.with_extension("json.pending")).unwrap();
        verify_existing_diagnostic(&path, &request, &state, &Role::I74, 1_048_576).unwrap();
        assert!(!path.with_extension("json.pending").exists());
    }

    #[test]
    fn diagnostic_limit_and_existing_byte_drift_fail_closed() {
        let (_temporary, campaign, request, state, snapshot) = fixture();
        let path = diagnostic_path(&campaign, &Role::I74, &request.request_id).unwrap();
        assert!(
            publish_diagnostic(&path, &request, &state, Role::I74, snapshot.clone(), 1,).is_err()
        );
        publish_diagnostic(&path, &request, &state, Role::I74, snapshot, 1_048_576).unwrap();
        std::fs::write(&path, b"{}").unwrap();
        assert!(
            verify_existing_diagnostic(&path, &request, &state, &Role::I74, 1_048_576).is_err()
        );
    }

    #[test]
    fn lease_expiry_diagnostic_is_cause_bound_and_exactly_replayed() {
        let (_temporary, campaign, _request, mut state, snapshot) = fixture();
        state.campaign_state = CampaignState::LeaseExpiredIncomplete;
        let cause = LeaseExpiryCause {
            schema_version: 1,
            campaign_id: state.identity.campaign_id.clone(),
            lease_id: state.identity.lease_id.clone(),
            lease_deadline_unix_seconds: state.identity.lease_deadline_unix_seconds,
        };
        let path = lease_expiry_diagnostic_path(&campaign, &Role::I74, &cause.lease_id).unwrap();
        publish_lease_expiry_diagnostic(&path, &cause, &state, Role::I74, snapshot, 1_048_576)
            .unwrap();
        let first = std::fs::read(&path).unwrap();
        std::fs::rename(&path, path.with_extension("json.pending")).unwrap();
        std::fs::hard_link(path.with_extension("json.pending"), &path).unwrap();
        verify_existing_lease_expiry_diagnostic(&path, &cause, &state, &Role::I74, 1_048_576)
            .unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), first);

        let mut drifted = cause;
        drifted.lease_deadline_unix_seconds += 1;
        assert!(verify_existing_lease_expiry_diagnostic(
            &path,
            &drifted,
            &state,
            &Role::I74,
            1_048_576,
        )
        .is_err());
    }

    #[test]
    fn progress_loss_diagnostic_is_cause_bound_and_exactly_replayed() {
        let (_temporary, campaign, _request, mut state, snapshot) = fixture();
        state.campaign_state = CampaignState::FailedIncomplete;
        let checkpoint = CheckpointHead {
            sequence: 8,
            record_sha256: hash('8'),
            useful_progress_unix_seconds: 990,
        };
        state.checkpoint = Some(checkpoint.clone());
        let cause = ProgressLossCause {
            schema_version: 1,
            campaign_id: state.identity.campaign_id.clone(),
            checkpoint: Some(checkpoint),
            last_useful_progress_unix_seconds: 990,
            rejection_gap_seconds: 180,
            rejection_deadline_unix_seconds: 1_170,
        };
        let path =
            progress_loss_diagnostic_path(&campaign, &Role::I74, &cause.sha256().unwrap()).unwrap();
        publish_progress_loss_diagnostic(
            &path,
            &cause,
            &state,
            Role::I74,
            snapshot.clone(),
            1_048_576,
        )
        .unwrap();
        let first = std::fs::read(&path).unwrap();
        std::fs::rename(&path, path.with_extension("json.pending")).unwrap();
        publish_progress_loss_diagnostic(
            &path,
            &cause,
            &state,
            Role::I74,
            snapshot.clone(),
            1_048_576,
        )
        .unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), first);
        std::fs::hard_link(&path, path.with_extension("json.pending")).unwrap();
        verify_existing_progress_loss_diagnostic(&path, &cause, &state, &Role::I74, 1_048_576)
            .unwrap();
        assert!(!path.with_extension("json.pending").exists());

        state.checkpoint = None;
        let startup_cause = ProgressLossCause {
            schema_version: 1,
            campaign_id: state.identity.campaign_id.clone(),
            checkpoint: None,
            last_useful_progress_unix_seconds: 1_000,
            rejection_gap_seconds: 180,
            rejection_deadline_unix_seconds: 1_180,
        };
        let startup_path =
            progress_loss_diagnostic_path(&campaign, &Role::I74, &startup_cause.sha256().unwrap())
                .unwrap();
        publish_progress_loss_diagnostic(
            &startup_path,
            &startup_cause,
            &state,
            Role::I74,
            snapshot,
            1_048_576,
        )
        .unwrap();
        verify_existing_progress_loss_diagnostic(
            &startup_path,
            &startup_cause,
            &state,
            &Role::I74,
            1_048_576,
        )
        .unwrap();
    }
}
