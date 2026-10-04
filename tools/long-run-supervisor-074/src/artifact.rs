use crate::{canonical_json, is_hash, sha256_hex, verify_journal, ProcessIdentity, Role};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use thiserror::Error;

pub const PACKET_MANIFEST_NAME: &str = "packet-manifest.json";
pub const RAW_MANIFEST_NAME: &str = "raw-manifest.json";
const CAMPAIGN_MANIFEST_NAME: &str = "campaign-start.json";
const RELEASE: &str = "0.74";
const MAXIMUM_FILES: usize = 20_000;
const MAXIMUM_BYTES: u64 = 21_474_836_480;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PacketResult {
    Complete,
    Incomplete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardEvidenceInput {
    pub id: String,
    pub passed: bool,
    pub source_relative_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleEvidenceInput {
    pub role: Role,
    pub result: PacketResult,
    pub journal_relative_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacketPlan {
    pub campaign_id: String,
    pub campaign_manifest_sha256: String,
    pub result: PacketResult,
    pub promotable: bool,
    pub terminal_reason: Option<String>,
    pub required_final_guards: Vec<String>,
    pub guard_evidence: Vec<GuardEvidenceInput>,
    pub roles: Vec<RoleEvidenceInput>,
    pub raw_files: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketLimits {
    pub maximum_files: usize,
    pub maximum_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacketReceipt {
    pub packet_directory: PathBuf,
    pub packet_manifest_path: PathBuf,
    pub packet_manifest_sha256: String,
    pub raw_manifest_sha256: String,
    pub raw_manifest_set_sha256: String,
    pub raw_file_count: usize,
    pub raw_total_bytes: u64,
}

#[derive(Debug, Error)]
pub enum ArtifactError {
    #[error("packet identity, result, guard, role, or limit contract is invalid")]
    Contract,
    #[error("packet source or destination path is unsafe")]
    Path,
    #[error("packet source contains a symlink, hardlink, device, or changed file")]
    File,
    #[error("packet source exceeds the frozen file or byte limit")]
    Limit,
    #[error("packet checkpoint journal is invalid: {0}")]
    Journal(#[from] crate::ChainError),
    #[error("packet JSON serialization failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("packet I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct RawFile {
    path: String,
    size: u64,
    sha256: String,
}

#[derive(Debug, Serialize)]
struct RawManifest<'a> {
    schema_version: u32,
    release: &'static str,
    campaign_id: &'a str,
    file_count: usize,
    total_bytes: u64,
    set_sha256: &'a str,
    files: &'a [RawFile],
}

#[derive(Debug, Serialize)]
struct GuardResult<'a> {
    id: &'a str,
    passed: bool,
    evidence_sha256: &'a str,
}

#[derive(Debug, Serialize)]
struct RoleManifest {
    id: Role,
    result: PacketResult,
    journal: String,
    journal_sha256: String,
    expected_bytes: u64,
    expected_records: u64,
    first_record_sha256: String,
    expected_head_sha256: String,
    allow_incomplete_trailing_bytes: bool,
    harness: ProcessIdentity,
    daemon: ProcessIdentity,
}

#[derive(Debug, Serialize)]
struct PacketManifest<'a> {
    schema_version: u32,
    release: &'static str,
    campaign_id: &'a str,
    campaign_manifest: &'static str,
    campaign_manifest_sha256: &'a str,
    raw_manifest: &'static str,
    raw_manifest_sha256: &'a str,
    raw_manifest_set_sha256: &'a str,
    result: PacketResult,
    promotable: bool,
    terminal_reason: &'a Option<String>,
    required_final_guards: &'a [String],
    guard_results: Vec<GuardResult<'a>>,
    roles: &'a [RoleManifest],
}

pub fn build_packet(
    campaign_directory: &Path,
    output_directory: &Path,
    plan: &PacketPlan,
    limits: PacketLimits,
) -> Result<PacketReceipt, ArtifactError> {
    validate_plan(plan, limits)?;
    let campaign_directory = canonical_directory(campaign_directory)?;
    if campaign_directory
        .file_name()
        .and_then(|name| name.to_str())
        != Some(plan.campaign_id.as_str())
    {
        return Err(ArtifactError::Path);
    }
    let output_parent = canonical_directory(output_directory.parent().ok_or(ArtifactError::Path)?)?;
    let output_name = output_directory
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| safe_component(name))
        .ok_or(ArtifactError::Path)?;
    let output_directory = output_parent.join(output_name);
    if output_directory.starts_with(&campaign_directory)
        || fs::symlink_metadata(&output_directory).is_ok()
    {
        return Err(ArtifactError::Path);
    }
    let staging = output_parent.join(format!(".{output_name}.building"));
    if fs::symlink_metadata(&staging).is_ok() {
        return Err(ArtifactError::Path);
    }
    fs::create_dir(&staging)?;
    let raw_directory = staging.join("raw");
    fs::create_dir(&raw_directory)?;

    let raw_files = copy_raw_files(&campaign_directory, &raw_directory, plan, limits)?;
    let raw_total_bytes = checked_total_bytes(&raw_files).ok_or(ArtifactError::Limit)?;
    let raw_manifest_set_sha256 = sha256_hex(&canonical_json(&raw_files)?);
    let raw_manifest = RawManifest {
        schema_version: 1,
        release: RELEASE,
        campaign_id: &plan.campaign_id,
        file_count: raw_files.len(),
        total_bytes: raw_total_bytes,
        set_sha256: &raw_manifest_set_sha256,
        files: &raw_files,
    };
    let raw_manifest_bytes = canonical_json(&raw_manifest)?;
    let raw_manifest_sha256 = sha256_hex(&raw_manifest_bytes);
    write_new_synced(&staging.join(RAW_MANIFEST_NAME), &raw_manifest_bytes)?;

    verify_campaign_manifest(&staging, plan)?;
    let file_by_source = raw_files_by_source(&raw_files)?;
    let guard_results = build_guard_results(plan, &file_by_source)?;
    let roles = build_role_manifests(&raw_directory, plan, &file_by_source)?;
    if (plan.result == PacketResult::Complete
        && (roles
            .iter()
            .any(|role| role.result != PacketResult::Complete)
            || guard_results.iter().any(|guard| !guard.passed)))
        || (plan.promotable && roles.len() != 2)
    {
        return Err(ArtifactError::Contract);
    }
    let packet_manifest = PacketManifest {
        schema_version: 1,
        release: RELEASE,
        campaign_id: &plan.campaign_id,
        campaign_manifest: "raw/campaign-start.json",
        campaign_manifest_sha256: &plan.campaign_manifest_sha256,
        raw_manifest: RAW_MANIFEST_NAME,
        raw_manifest_sha256: &raw_manifest_sha256,
        raw_manifest_set_sha256: &raw_manifest_set_sha256,
        result: plan.result,
        promotable: plan.promotable,
        terminal_reason: &plan.terminal_reason,
        required_final_guards: &plan.required_final_guards,
        guard_results,
        roles: &roles,
    };
    let packet_manifest_bytes = canonical_json(&packet_manifest)?;
    let packet_manifest_sha256 = sha256_hex(&packet_manifest_bytes);
    write_new_synced(&staging.join(PACKET_MANIFEST_NAME), &packet_manifest_bytes)?;
    sync_tree_directories(&raw_directory)?;
    sync_directory(&staging)?;
    fs::rename(&staging, &output_directory)?;
    sync_directory(&output_parent)?;

    Ok(PacketReceipt {
        packet_manifest_path: output_directory.join(PACKET_MANIFEST_NAME),
        packet_directory: output_directory,
        packet_manifest_sha256,
        raw_manifest_sha256,
        raw_manifest_set_sha256,
        raw_file_count: raw_files.len(),
        raw_total_bytes,
    })
}

fn validate_plan(plan: &PacketPlan, limits: PacketLimits) -> Result<(), ArtifactError> {
    if !is_hash(&plan.campaign_id)
        || !is_hash(&plan.campaign_manifest_sha256)
        || limits.maximum_files == 0
        || limits.maximum_files > MAXIMUM_FILES
        || limits.maximum_bytes == 0
        || limits.maximum_bytes > MAXIMUM_BYTES
        || plan.raw_files.is_empty()
        || plan.raw_files.len() > limits.maximum_files
        || plan.roles.is_empty()
        || plan.roles.len() > 2
    {
        return Err(ArtifactError::Contract);
    }
    match (plan.result, plan.terminal_reason.as_deref()) {
        (PacketResult::Complete, None) => {}
        (PacketResult::Incomplete, Some(reason))
            if !reason.is_empty()
                && reason.len() <= 128
                && !reason.contains(['\n', '\r', '\0']) => {}
        _ => return Err(ArtifactError::Contract),
    }
    if plan.promotable && plan.result != PacketResult::Complete {
        return Err(ArtifactError::Contract);
    }
    let raw = unique_paths(&plan.raw_files)?;
    if !raw.contains(Path::new(CAMPAIGN_MANIFEST_NAME)) {
        return Err(ArtifactError::Contract);
    }
    let required = plan
        .required_final_guards
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let provided = plan
        .guard_evidence
        .iter()
        .map(|guard| guard.id.as_str())
        .collect::<BTreeSet<_>>();
    if required.is_empty()
        || required.len() != plan.required_final_guards.len()
        || provided != required
        || provided.len() != plan.guard_evidence.len()
        || required
            .iter()
            .any(|id| id.is_empty() || id.len() > 128 || id.contains(['\n', '\r', '\0']))
        || plan
            .guard_evidence
            .iter()
            .any(|guard| !raw.contains(&guard.source_relative_path))
    {
        return Err(ArtifactError::Contract);
    }
    let roles = plan
        .roles
        .iter()
        .map(|role| role_name(&role.role))
        .collect::<BTreeSet<_>>();
    if roles.len() != plan.roles.len()
        || !roles.contains("i74")
        || plan
            .roles
            .iter()
            .any(|role| !raw.contains(&role.journal_relative_path))
        || (plan.roles.len() == 1 && plan.roles[0].role != Role::I74)
    {
        return Err(ArtifactError::Contract);
    }
    Ok(())
}

fn unique_paths(paths: &[PathBuf]) -> Result<BTreeSet<PathBuf>, ArtifactError> {
    let mut unique = BTreeSet::new();
    for path in paths {
        validate_relative_path(path)?;
        if !unique.insert(path.clone()) {
            return Err(ArtifactError::Contract);
        }
    }
    Ok(unique)
}

fn copy_raw_files(
    campaign_directory: &Path,
    raw_directory: &Path,
    plan: &PacketPlan,
    limits: PacketLimits,
) -> Result<Vec<RawFile>, ArtifactError> {
    let sources = unique_paths(&plan.raw_files)?;
    let mut files = Vec::with_capacity(sources.len());
    let mut total = 0_u64;
    for relative in sources {
        let source = campaign_directory.join(&relative);
        reject_symlink_components(campaign_directory, &relative)?;
        let destination = raw_directory.join(&relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        let (size, sha256) = copy_one(&source, &destination, limits.maximum_bytes - total)?;
        total = total.checked_add(size).ok_or(ArtifactError::Limit)?;
        files.push(RawFile {
            path: format!("raw/{}", normalized(&relative)?),
            size,
            sha256,
        });
    }
    if total == 0 || total > limits.maximum_bytes {
        return Err(ArtifactError::Limit);
    }
    files.sort_by(|left, right| left.path.as_bytes().cmp(right.path.as_bytes()));
    Ok(files)
}

fn copy_one(
    source: &Path,
    destination: &Path,
    remaining: u64,
) -> Result<(u64, String), ArtifactError> {
    let metadata = fs::symlink_metadata(source)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || has_multiple_links(source, &metadata)?
        || metadata.len() > remaining
    {
        return Err(if metadata.len() > remaining {
            ArtifactError::Limit
        } else {
            ArtifactError::File
        });
    }
    let mut input = File::open(source)?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?;
    let mut digest = Sha256::new();
    let mut copied = 0_u64;
    let mut buffer = vec![0_u8; 1024 * 1024];
    loop {
        let read = input.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        copied = copied
            .checked_add(read as u64)
            .ok_or(ArtifactError::Limit)?;
        if copied > remaining {
            return Err(ArtifactError::Limit);
        }
        digest.update(&buffer[..read]);
        output.write_all(&buffer[..read])?;
    }
    output.sync_all()?;
    let after = input.metadata()?;
    if copied != metadata.len()
        || after.len() != metadata.len()
        || has_multiple_links(source, &after)?
    {
        return Err(ArtifactError::File);
    }
    Ok((copied, hex(&digest.finalize())))
}

fn raw_files_by_source(files: &[RawFile]) -> Result<BTreeMap<PathBuf, &RawFile>, ArtifactError> {
    files
        .iter()
        .map(|file| {
            let relative = file
                .path
                .strip_prefix("raw/")
                .ok_or(ArtifactError::Contract)?;
            Ok((PathBuf::from(relative), file))
        })
        .collect()
}

fn build_guard_results<'a>(
    plan: &'a PacketPlan,
    files: &BTreeMap<PathBuf, &'a RawFile>,
) -> Result<Vec<GuardResult<'a>>, ArtifactError> {
    plan.required_final_guards
        .iter()
        .map(|id| {
            let guard = plan
                .guard_evidence
                .iter()
                .find(|guard| &guard.id == id)
                .ok_or(ArtifactError::Contract)?;
            let file = files
                .get(&guard.source_relative_path)
                .ok_or(ArtifactError::Contract)?;
            Ok(GuardResult {
                id,
                passed: guard.passed,
                evidence_sha256: &file.sha256,
            })
        })
        .collect()
}

fn build_role_manifests(
    copied_raw_directory: &Path,
    plan: &PacketPlan,
    files: &BTreeMap<PathBuf, &RawFile>,
) -> Result<Vec<RoleManifest>, ArtifactError> {
    let mut roles = Vec::with_capacity(plan.roles.len());
    for input in &plan.roles {
        let report = verify_journal(&copied_raw_directory.join(&input.journal_relative_path))?;
        if report.campaign_id != plan.campaign_id
            || report.role != input.role
            || (input.result == PacketResult::Complete
                && report.recovered_incomplete_trailing_bytes != 0)
        {
            return Err(ArtifactError::Contract);
        }
        let file = files
            .get(&input.journal_relative_path)
            .ok_or(ArtifactError::Contract)?;
        roles.push(RoleManifest {
            id: input.role.clone(),
            result: input.result,
            journal: file.path.clone(),
            journal_sha256: file.sha256.clone(),
            expected_bytes: file.size,
            expected_records: report.records,
            first_record_sha256: report.first_record_sha256,
            expected_head_sha256: report.head_sha256,
            allow_incomplete_trailing_bytes: report.recovered_incomplete_trailing_bytes != 0,
            harness: report.harness,
            daemon: report.daemon,
        });
    }
    roles.sort_by(|left, right| role_name(&left.id).cmp(role_name(&right.id)));
    Ok(roles)
}

fn verify_campaign_manifest(staging: &Path, plan: &PacketPlan) -> Result<(), ArtifactError> {
    let bytes = fs::read(staging.join("raw").join(CAMPAIGN_MANIFEST_NAME))?;
    let encoded = bytes.strip_suffix(b"\n").unwrap_or(&bytes);
    if sha256_hex(encoded) != plan.campaign_manifest_sha256
        || encoded.contains(&b'\n')
        || encoded.contains(&b'\r')
    {
        return Err(ArtifactError::Contract);
    }
    let value: serde_json::Value = serde_json::from_slice(encoded)?;
    if canonical_json(&value)? != encoded
        || value.get("release").and_then(serde_json::Value::as_str) != Some(RELEASE)
        || value.get("campaign_id").and_then(serde_json::Value::as_str)
            != Some(plan.campaign_id.as_str())
    {
        return Err(ArtifactError::Contract);
    }
    Ok(())
}

fn validate_relative_path(path: &Path) -> Result<(), ArtifactError> {
    if path.is_absolute() {
        return Err(ArtifactError::Path);
    }
    let normalized = normalized(path)?;
    if normalized.is_empty() || normalized.len() > 1024 {
        return Err(ArtifactError::Path);
    }
    Ok(())
}

fn normalized(path: &Path) -> Result<String, ArtifactError> {
    let mut output = Vec::new();
    for component in path.components() {
        let Component::Normal(component) = component else {
            return Err(ArtifactError::Path);
        };
        let component = component
            .to_str()
            .filter(|value| safe_component(value))
            .ok_or(ArtifactError::Path)?;
        output.push(component);
    }
    Ok(output.join("/"))
}

fn safe_component(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && !value.contains(['/', '\\', ':', '\0', '\n', '\r'])
        && !value.bytes().any(|byte| byte < 0x20 || byte == 0x7f)
}

fn reject_symlink_components(root: &Path, relative: &Path) -> Result<(), ArtifactError> {
    let mut current = root.to_path_buf();
    for component in relative.components() {
        let Component::Normal(component) = component else {
            return Err(ArtifactError::Path);
        };
        current.push(component);
        if fs::symlink_metadata(&current)?.file_type().is_symlink() {
            return Err(ArtifactError::File);
        }
    }
    Ok(())
}

fn canonical_directory(path: &Path) -> Result<PathBuf, ArtifactError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(ArtifactError::Path);
    }
    Ok(fs::canonicalize(path)?)
}

fn write_new_synced(path: &Path, bytes: &[u8]) -> Result<(), ArtifactError> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn checked_total_bytes(files: &[RawFile]) -> Option<u64> {
    files
        .iter()
        .try_fold(0_u64, |total, file| total.checked_add(file.size))
}

fn role_name(role: &Role) -> &'static str {
    match role {
        Role::I74 => "i74",
        Role::C74 => "c74",
    }
}

fn sync_tree_directories(root: &Path) -> Result<(), ArtifactError> {
    let mut directories = vec![root.to_path_buf()];
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_dir() {
            sync_tree_directories(&path)?;
            directories.push(path);
        }
    }
    for directory in directories.into_iter().rev() {
        sync_directory(&directory)?;
    }
    Ok(())
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<(), ArtifactError> {
    File::open(path)?.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> Result<(), ArtifactError> {
    Ok(())
}

#[cfg(unix)]
fn has_multiple_links(_path: &Path, metadata: &fs::Metadata) -> Result<bool, ArtifactError> {
    use std::os::unix::fs::MetadataExt;
    Ok(metadata.nlink() > 1)
}

#[cfg(windows)]
fn has_multiple_links(path: &Path, _metadata: &fs::Metadata) -> Result<bool, ArtifactError> {
    use std::mem::zeroed;
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
    };

    let file = File::open(path)?;
    let mut information: BY_HANDLE_FILE_INFORMATION = unsafe { zeroed() };
    // SAFETY: the file handle and writable information structure remain valid for the call.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut information) } == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(information.nNumberOfLinks > 1)
}

#[cfg(not(any(unix, windows)))]
fn has_multiple_links(_path: &Path, _metadata: &fs::Metadata) -> Result<bool, ArtifactError> {
    Ok(false)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
