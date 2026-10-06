use crate::manifest::CampaignManifest;
#[cfg(any(target_os = "linux", test))]
use crate::manifest::InstalledBinary;
use crate::state::DurableCampaignState;
use crate::{canonical_json, is_hash, sha256_hex};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "linux")]
use std::fs::File;
#[cfg(target_os = "linux")]
use std::io::Read;
#[cfg(target_os = "linux")]
use std::io::Write;
#[cfg(target_os = "linux")]
use std::path::Path;
#[cfg(target_os = "linux")]
use std::path::PathBuf;
use thiserror::Error;

pub const HOST_RECEIPT_NAME: &str = "host-observation.json";
pub const HOST_RECEIPT_HEAD_NAME: &str = "host-observation.sha256";
pub const MAX_HOST_RECEIPT_BYTES: usize = 65_536;
pub const SUPERVISOR_BINARY_PATH: &str =
    "/opt/hydracache-perf/bin/hydracache-long-run-supervisor-074";
pub const REFERENCE_HOST_FREEZE_PATH: &str =
    "/var/lib/hydracache-perf/host-tuning-v1/freeze/host-freeze.json";
pub const PROVISIONING_RECEIPT_PATH: &str =
    "/var/lib/hydracache-performance/provisioning-receipt-074.json";
pub const CAMPAIGN_FIXTURE_BINARY_PATH: &str =
    "/opt/hydracache-performance/0.74/campaign-lifecycle-fixture";
#[cfg(target_os = "linux")]
const INSTALLED_BINARY_ROOT: &str = "/opt/hydracache-performance/0.74/";

const TUNABLES: [(&str, &str); 7] = [
    ("kernel.numa_balancing", "/proc/sys/kernel/numa_balancing"),
    (
        "kernel.sched_autogroup_enabled",
        "/proc/sys/kernel/sched_autogroup_enabled",
    ),
    (
        "kernel.sched_migration_cost_ns",
        "/proc/sys/kernel/sched_migration_cost_ns",
    ),
    ("kernel.watchdog", "/proc/sys/kernel/watchdog"),
    (
        "vm.dirty_background_ratio",
        "/proc/sys/vm/dirty_background_ratio",
    ),
    ("vm.dirty_ratio", "/proc/sys/vm/dirty_ratio"),
    ("vm.swappiness", "/proc/sys/vm/swappiness"),
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MountIdentity {
    pub mount_id: u64,
    pub device_major_minor: String,
    pub root: String,
    pub mount_point: String,
    pub mount_options: Vec<String>,
    pub filesystem_type: String,
    pub source: String,
    pub super_options: Vec<String>,
}

#[derive(Serialize)]
struct StableMountIdentity<'a> {
    device_major_minor: &'a str,
    root: &'a str,
    mount_point: &'a str,
    mount_options: &'a [String],
    filesystem_type: &'a str,
    source: &'a str,
    super_options: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BinaryIdentity {
    pub path: String,
    pub sha256: String,
    pub size: u64,
    pub inode: u64,
    pub device: u64,
    pub uid: u64,
    pub gid: u64,
    pub mode: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostObservationReceipt {
    pub schema_version: u32,
    pub machine_id: String,
    pub boot_id: String,
    pub kernel_release: String,
    pub kernel_command_line_sha256: String,
    pub campaign_mount: MountIdentity,
    pub mount_identity: String,
    pub online_cpuset: String,
    pub isolated_cpuset: String,
    pub housekeeping_cpuset: String,
    pub cpu_governors: BTreeMap<String, String>,
    pub kernel_tunables: BTreeMap<String, String>,
    pub supervisor_binary: BinaryIdentity,
    pub reference_host_freeze_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledProvisioningIdentity {
    pub source_commit: String,
    pub fixture_binary_sha256: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum HostReceiptError {
    #[error("host observation receipt is empty, oversized, non-canonical, or malformed")]
    Document,
    #[error("host observation receipt path or digest sidecar is unsafe")]
    Path,
    #[error("host observation receipt digest does not match admitted identity")]
    Digest,
    #[error("host observation receipt violates its structural invariants")]
    Invariant,
    #[error("host observation receipt does not match campaign identity")]
    Binding,
    #[error("live host observation differs from admitted receipt")]
    Drift,
    #[error("live host observation is unavailable or malformed")]
    Observation,
    #[error("host observation I/O failed: {0}")]
    Io(String),
}

impl From<std::io::Error> for HostReceiptError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

pub fn parse_and_validate(
    bytes: &[u8],
    expected_sha256: &str,
) -> Result<HostObservationReceipt, HostReceiptError> {
    if bytes.is_empty() || bytes.len() > MAX_HOST_RECEIPT_BYTES || !is_hash(expected_sha256) {
        return Err(HostReceiptError::Document);
    }
    let encoded = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    if encoded.is_empty() || encoded.contains(&b'\n') || encoded.contains(&b'\r') {
        return Err(HostReceiptError::Document);
    }
    if sha256_hex(encoded) != expected_sha256 {
        return Err(HostReceiptError::Digest);
    }
    let mut deserializer = serde_json::Deserializer::from_slice(encoded);
    let receipt = HostObservationReceipt::deserialize(&mut deserializer)
        .map_err(|_| HostReceiptError::Document)?;
    deserializer.end().map_err(|_| HostReceiptError::Document)?;
    let value: serde_json::Value =
        serde_json::from_slice(encoded).map_err(|_| HostReceiptError::Document)?;
    if serde_json::to_vec(&value).map_err(|_| HostReceiptError::Document)? != encoded {
        return Err(HostReceiptError::Document);
    }
    validate_receipt(&receipt)?;
    Ok(receipt)
}

pub fn verify_receipt_binding(
    receipt: &HostObservationReceipt,
    manifest: &CampaignManifest,
    state: &DurableCampaignState,
) -> Result<(), HostReceiptError> {
    verify_receipt_manifest_binding(receipt, manifest)?;
    if state.identity.machine_id != manifest.machine_id
        || state.identity.boot_id != manifest.boot_id
        || state.identity.mount_identity != manifest.mount_identity
        || state.identity.isolated_cpuset != manifest.isolated_cpuset
        || state.identity.housekeeping_cpuset != manifest.housekeeping_cpuset
        || state.identity.host_receipt_sha256 != manifest.host_receipt_sha256
    {
        return Err(HostReceiptError::Binding);
    }
    Ok(())
}

pub fn verify_receipt_manifest_binding(
    receipt: &HostObservationReceipt,
    manifest: &CampaignManifest,
) -> Result<(), HostReceiptError> {
    if receipt.machine_id != manifest.machine_id
        || receipt.boot_id != manifest.boot_id
        || receipt.mount_identity != manifest.mount_identity
        || receipt.isolated_cpuset != manifest.isolated_cpuset
        || receipt.housekeeping_cpuset != manifest.housekeeping_cpuset
    {
        return Err(HostReceiptError::Binding);
    }
    Ok(())
}

pub fn verify_live_observation(
    admitted: &HostObservationReceipt,
    observed: &HostObservationReceipt,
) -> Result<(), HostReceiptError> {
    validate_receipt(observed)?;
    let mut normalized = observed.clone();
    normalized.campaign_mount.mount_id = admitted.campaign_mount.mount_id;
    if admitted == &normalized {
        Ok(())
    } else {
        Err(HostReceiptError::Drift)
    }
}

#[cfg(any(target_os = "linux", test))]
fn verify_quarantine_recovery_live_observation(
    admitted: &HostObservationReceipt,
    observed: &HostObservationReceipt,
) -> Result<(), HostReceiptError> {
    validate_receipt(observed)?;
    let mut normalized = observed.clone();
    normalized.supervisor_binary = admitted.supervisor_binary.clone();
    verify_live_observation(admitted, &normalized)
}

/// Hashes the stable backing-mount properties while retaining `mount_id` only
/// as diagnostic evidence. Linux mount IDs are local to a mount namespace and
/// may change when systemd reconstructs an otherwise identical private
/// namespace after a supervisor restart.
pub fn mount_identity_digest(mount: &MountIdentity) -> Result<String, HostReceiptError> {
    validate_mount(mount)?;
    let stable = StableMountIdentity {
        device_major_minor: &mount.device_major_minor,
        root: &mount.root,
        mount_point: &mount.mount_point,
        mount_options: &mount.mount_options,
        filesystem_type: &mount.filesystem_type,
        source: &mount.source,
        super_options: &mount.super_options,
    };
    Ok(sha256_hex(
        &canonical_json(&stable).map_err(|_| HostReceiptError::Invariant)?,
    ))
}

#[cfg(target_os = "linux")]
pub fn verify_host_receipt_evidence(
    campaign_directory: &Path,
    manifest: &CampaignManifest,
    state: &DurableCampaignState,
) -> Result<HostObservationReceipt, HostReceiptError> {
    let receipt_path = campaign_directory.join(HOST_RECEIPT_NAME);
    let head_path = campaign_directory.join(HOST_RECEIPT_HEAD_NAME);
    let bytes = read_regular_bounded(&receipt_path, MAX_HOST_RECEIPT_BYTES as u64)?;
    verify_head(&head_path, &manifest.host_receipt_sha256)?;
    let receipt = parse_and_validate(&bytes, &manifest.host_receipt_sha256)?;
    verify_receipt_binding(&receipt, manifest, state)?;
    let observed = collect_host_observation(campaign_directory)?;
    verify_live_observation(&receipt, &observed)?;
    Ok(receipt)
}

/// Revalidate a quarantined campaign after a signed in-place supervisor
/// upgrade. Every admitted host property remains frozen; only the supervisor
/// binary identity may differ, and the replacement must be bound to the
/// root-owned provisioning receipt produced by the signed installer.
#[cfg(target_os = "linux")]
pub fn verify_quarantine_recovery_host_receipt_evidence(
    campaign_directory: &Path,
    manifest: &CampaignManifest,
    state: &DurableCampaignState,
) -> Result<HostObservationReceipt, HostReceiptError> {
    let receipt_path = campaign_directory.join(HOST_RECEIPT_NAME);
    let head_path = campaign_directory.join(HOST_RECEIPT_HEAD_NAME);
    let bytes = read_regular_bounded(&receipt_path, MAX_HOST_RECEIPT_BYTES as u64)?;
    verify_head(&head_path, &manifest.host_receipt_sha256)?;
    let receipt = parse_and_validate(&bytes, &manifest.host_receipt_sha256)?;
    verify_receipt_binding(&receipt, manifest, state)?;
    let observed = collect_host_observation(campaign_directory)?;
    collect_installed_provisioning_identity(&observed.supervisor_binary.sha256)?;
    verify_quarantine_recovery_live_observation(&receipt, &observed)?;
    Ok(receipt)
}

/// Verify the deliberately non-promotable campaign-smoke receipt against its
/// campaign-local freeze. Production callers must use [`verify_host_receipt_evidence`],
/// which always recollects against [`REFERENCE_HOST_FREEZE_PATH`].
#[cfg(target_os = "linux")]
pub fn verify_fixture_host_receipt_evidence(
    campaign_directory: &Path,
    fixture_freeze: &Path,
    manifest: &CampaignManifest,
    state: &DurableCampaignState,
) -> Result<HostObservationReceipt, HostReceiptError> {
    let receipt_path = campaign_directory.join(HOST_RECEIPT_NAME);
    let head_path = campaign_directory.join(HOST_RECEIPT_HEAD_NAME);
    let bytes = read_regular_bounded(&receipt_path, MAX_HOST_RECEIPT_BYTES as u64)?;
    verify_head(&head_path, &manifest.host_receipt_sha256)?;
    let receipt = parse_and_validate(&bytes, &manifest.host_receipt_sha256)?;
    verify_receipt_binding(&receipt, manifest, state)?;
    let observed = collect_fixture_host_observation(campaign_directory, fixture_freeze)?;
    verify_live_observation(&receipt, &observed)?;
    Ok(receipt)
}

#[cfg(target_os = "linux")]
pub fn collect_host_observation(
    campaign_directory: &Path,
) -> Result<HostObservationReceipt, HostReceiptError> {
    collect_host_observation_with_freeze(campaign_directory, Path::new(REFERENCE_HOST_FREEZE_PATH))
}

#[cfg(target_os = "linux")]
pub fn collect_installed_provisioning_identity(
    supervisor_binary_sha256: &str,
) -> Result<InstalledProvisioningIdentity, HostReceiptError> {
    use std::os::unix::fs::MetadataExt;

    let path = Path::new(PROVISIONING_RECEIPT_PATH);
    let metadata = safe_regular_metadata(path, MAX_HOST_RECEIPT_BYTES as u64)?;
    if metadata.uid() != 0 || metadata.gid() != 0 || metadata.mode() & 0o7777 != 0o444 {
        return Err(HostReceiptError::Path);
    }
    let bytes = read_regular_bounded(path, MAX_HOST_RECEIPT_BYTES as u64)?;
    parse_installed_provisioning_identity(&bytes, supervisor_binary_sha256)
}

#[cfg(any(target_os = "linux", test))]
fn parse_installed_provisioning_identity(
    bytes: &[u8],
    supervisor_binary_sha256: &str,
) -> Result<InstalledProvisioningIdentity, HostReceiptError> {
    if bytes.is_empty()
        || bytes.len() > MAX_HOST_RECEIPT_BYTES
        || !is_hash(supervisor_binary_sha256)
    {
        return Err(HostReceiptError::Document);
    }
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| HostReceiptError::Document)?;
    let object = value.as_object().ok_or(HostReceiptError::Document)?;
    const EXPECTED_FIELDS: [&str; 24] = [
        "schema_version",
        "source_commit",
        "created_at_utc",
        "mutation_performed",
        "repository_id",
        "allowed_actor_ids",
        "runner_uid",
        "runner_gid",
        "supervisor_uid",
        "client_gid",
        "runner_group_database_membership",
        "runner_process_group_refresh_may_be_required",
        "service_active",
        "socket_mode",
        "binary_sha256",
        "fixture_binary_sha256",
        "config_sha256",
        "service_sha256",
        "sysusers_sha256",
        "tmpfiles_sha256",
        "verification_key_sha256",
        "unit_properties_sha256",
        "machine_id_sha256",
        "boot_id_sha256",
    ];
    if object.len() != EXPECTED_FIELDS.len()
        || EXPECTED_FIELDS
            .iter()
            .any(|field| !object.contains_key(*field))
        || object.get("schema_version").and_then(|item| item.as_str())
            != Some("hydracache-w11-host-provisioning-v1")
        || object
            .get("mutation_performed")
            .and_then(|item| item.as_bool())
            != Some(true)
        || object.get("service_active").and_then(|item| item.as_bool()) != Some(true)
        || object.get("binary_sha256").and_then(|item| item.as_str())
            != Some(supervisor_binary_sha256)
    {
        return Err(HostReceiptError::Invariant);
    }
    let source_commit = object
        .get("source_commit")
        .and_then(|item| item.as_str())
        .ok_or(HostReceiptError::Invariant)?;
    if source_commit.len() != 40
        || !source_commit
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(HostReceiptError::Invariant);
    }
    let fixture_binary_sha256 = object
        .get("fixture_binary_sha256")
        .and_then(|item| item.as_str())
        .ok_or(HostReceiptError::Invariant)?;
    if !is_hash(fixture_binary_sha256) {
        return Err(HostReceiptError::Invariant);
    }
    Ok(InstalledProvisioningIdentity {
        source_commit: source_commit.to_owned(),
        fixture_binary_sha256: fixture_binary_sha256.to_owned(),
    })
}

#[cfg(target_os = "linux")]
pub fn collect_campaign_fixture_binary(
    expected_sha256: &str,
) -> Result<BinaryIdentity, HostReceiptError> {
    let identity = collect_binary_identity(Path::new(CAMPAIGN_FIXTURE_BINARY_PATH))?;
    if identity.path != CAMPAIGN_FIXTURE_BINARY_PATH
        || identity.sha256 != expected_sha256
        || identity.uid != 0
        || identity.gid != 0
        || identity.mode != 0o755
    {
        return Err(HostReceiptError::Invariant);
    }
    Ok(identity)
}

#[cfg(target_os = "linux")]
pub fn verify_installed_binaries(
    installed_binaries: &[InstalledBinary],
) -> Result<(), HostReceiptError> {
    if installed_binaries.len() != 2 {
        return Err(HostReceiptError::Invariant);
    }
    for expected in installed_binaries {
        if !expected.path.starts_with(INSTALLED_BINARY_ROOT)
            || expected.path.len() <= INSTALLED_BINARY_ROOT.len()
            || expected.path.split('/').any(|component| component == "..")
        {
            return Err(HostReceiptError::Path);
        }
        let observed = collect_binary_identity(Path::new(&expected.path))?;
        if !binary_identity_matches(&observed, expected) {
            return Err(HostReceiptError::Drift);
        }
    }
    Ok(())
}

#[cfg(any(target_os = "linux", test))]
fn binary_identity_matches(observed: &BinaryIdentity, expected: &InstalledBinary) -> bool {
    observed.path == expected.path
        && observed.sha256 == expected.sha256
        && observed.size == expected.size
        && observed.inode == expected.inode
        && observed.device == expected.device
        && observed.uid == expected.uid
        && observed.gid == expected.gid
        && u64::from(observed.mode) == expected.mode
}

#[cfg(target_os = "linux")]
pub fn collect_fixture_host_observation(
    campaign_directory: &Path,
    fixture_freeze: &Path,
) -> Result<HostObservationReceipt, HostReceiptError> {
    use std::os::unix::fs::MetadataExt;

    let campaign = fs::canonicalize(campaign_directory)?;
    let freeze = fs::canonicalize(fixture_freeze)?;
    let metadata = fs::symlink_metadata(&freeze)?;
    if freeze.parent() != Some(campaign.as_path())
        || freeze.file_name().and_then(|value| value.to_str()) != Some("fixture-host-freeze.json")
        || !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.nlink() != 1
        || metadata.uid() != 0
        || metadata.gid() != 0
        || metadata.mode() & 0o7777 != 0o400
    {
        return Err(HostReceiptError::Path);
    }
    collect_host_observation_with_freeze(&campaign, &freeze)
}

#[cfg(target_os = "linux")]
fn collect_host_observation_with_freeze(
    campaign_directory: &Path,
    freeze_path: &Path,
) -> Result<HostObservationReceipt, HostReceiptError> {
    use std::os::unix::fs::MetadataExt;

    if unsafe { libc::geteuid() } != 0 {
        return Err(HostReceiptError::Observation);
    }
    let machine_id = read_trimmed(Path::new("/etc/machine-id"), 256)?;
    let boot_id = read_trimmed(Path::new("/proc/sys/kernel/random/boot_id"), 256)?;
    let kernel_release = read_trimmed(Path::new("/proc/sys/kernel/osrelease"), 256)?;
    let kernel_command_line = read_trimmed(Path::new("/proc/cmdline"), 65_536)?;
    let mountinfo = read_virtual(Path::new("/proc/self/mountinfo"), 4 * 1024 * 1024)?;
    let canonical_campaign = campaign_directory.canonicalize()?;
    let campaign_mount = parse_mountinfo(&mountinfo, &canonical_campaign)?;
    let mount_identity =
        mount_identity_digest(&campaign_mount).map_err(|_| HostReceiptError::Observation)?;

    let online_cpuset = canonical_cpuset(&read_trimmed(
        Path::new("/sys/devices/system/cpu/online"),
        65_536,
    )?)?;
    let isolated_cpuset = canonical_cpuset(&read_trimmed(
        Path::new("/sys/devices/system/cpu/isolated"),
        65_536,
    )?)?;
    let online = parse_cpuset(&online_cpuset)?;
    let isolated = parse_cpuset(&isolated_cpuset)?;
    if isolated.is_empty() || !isolated.is_subset(&online) {
        return Err(HostReceiptError::Observation);
    }
    let housekeeping = online
        .difference(&isolated)
        .copied()
        .collect::<BTreeSet<_>>();
    if housekeeping.is_empty() {
        return Err(HostReceiptError::Observation);
    }
    let housekeeping_cpuset = format_cpuset(&housekeeping);

    let mut cpu_governors = BTreeMap::new();
    for cpu in &online {
        let key = format!("cpu{cpu}");
        let path = PathBuf::from(format!(
            "/sys/devices/system/cpu/{key}/cpufreq/scaling_governor"
        ));
        cpu_governors.insert(key, read_trimmed(&path, 256)?);
    }

    let mut kernel_tunables = BTreeMap::new();
    for (key, proc_path) in TUNABLES {
        let primary = Path::new(proc_path);
        let value = if primary.exists() {
            read_trimmed(primary, 4_096)?
        } else if key == "kernel.sched_migration_cost_ns" {
            read_trimmed(
                Path::new("/sys/kernel/debug/sched/migration_cost_ns"),
                4_096,
            )?
        } else {
            return Err(HostReceiptError::Observation);
        };
        kernel_tunables.insert(key.to_owned(), value);
    }

    let binary_path = Path::new(SUPERVISOR_BINARY_PATH);
    let metadata = safe_regular_metadata(binary_path, 512 * 1024 * 1024)?;
    let current_metadata = fs::metadata(std::env::current_exe()?)?;
    if metadata.dev() != current_metadata.dev() || metadata.ino() != current_metadata.ino() {
        return Err(HostReceiptError::Observation);
    }
    let supervisor_binary = collect_binary_identity(binary_path)?;
    if supervisor_binary.device != current_metadata.dev()
        || supervisor_binary.inode != current_metadata.ino()
    {
        return Err(HostReceiptError::Observation);
    }
    let reference_host_freeze_sha256 = sha256_file(freeze_path, MAX_HOST_RECEIPT_BYTES as u64)?;

    let receipt = HostObservationReceipt {
        schema_version: 1,
        machine_id,
        boot_id,
        kernel_release,
        kernel_command_line_sha256: sha256_hex(kernel_command_line.as_bytes()),
        campaign_mount,
        mount_identity,
        online_cpuset,
        isolated_cpuset,
        housekeeping_cpuset,
        cpu_governors,
        kernel_tunables,
        supervisor_binary,
        reference_host_freeze_sha256,
    };
    validate_receipt(&receipt)?;
    Ok(receipt)
}

#[cfg(target_os = "linux")]
fn collect_binary_identity(path: &Path) -> Result<BinaryIdentity, HostReceiptError> {
    use std::os::unix::fs::MetadataExt;

    let metadata = safe_regular_metadata(path, 512 * 1024 * 1024)?;
    let sha256 = sha256_file(path, 512 * 1024 * 1024)?;
    let current = safe_regular_metadata(path, 512 * 1024 * 1024)?;
    if metadata.dev() != current.dev()
        || metadata.ino() != current.ino()
        || metadata.len() != current.len()
        || metadata.uid() != current.uid()
        || metadata.gid() != current.gid()
        || metadata.mode() != current.mode()
    {
        return Err(HostReceiptError::Observation);
    }
    let path = path.to_str().ok_or(HostReceiptError::Path)?.to_owned();
    Ok(BinaryIdentity {
        path,
        sha256,
        size: metadata.len(),
        inode: metadata.ino(),
        device: metadata.dev(),
        uid: u64::from(metadata.uid()),
        gid: u64::from(metadata.gid()),
        mode: metadata.mode() & 0o7777,
    })
}

fn validate_receipt(receipt: &HostObservationReceipt) -> Result<(), HostReceiptError> {
    if receipt.schema_version != 1
        || !bounded(&receipt.machine_id, 256)
        || !bounded(&receipt.boot_id, 256)
        || !bounded(&receipt.kernel_release, 256)
        || !is_hash(&receipt.kernel_command_line_sha256)
        || !is_hash(&receipt.mount_identity)
        || !is_hash(&receipt.reference_host_freeze_sha256)
        || receipt.supervisor_binary.path != SUPERVISOR_BINARY_PATH
        || !is_hash(&receipt.supervisor_binary.sha256)
        || receipt.supervisor_binary.size == 0
        || receipt.supervisor_binary.inode == 0
        || receipt.supervisor_binary.device == 0
        || receipt.supervisor_binary.uid != 0
        || receipt.supervisor_binary.gid != 0
        || receipt.supervisor_binary.mode != 0o755
        || validate_mount(&receipt.campaign_mount).is_err()
    {
        return Err(HostReceiptError::Invariant);
    }
    let expected_mount = mount_identity_digest(&receipt.campaign_mount)?;
    if receipt.mount_identity != expected_mount {
        return Err(HostReceiptError::Invariant);
    }
    let online = require_canonical_cpuset(&receipt.online_cpuset)?;
    let isolated = require_canonical_cpuset(&receipt.isolated_cpuset)?;
    let housekeeping = require_canonical_cpuset(&receipt.housekeeping_cpuset)?;
    if online.is_empty()
        || isolated.is_empty()
        || housekeeping.is_empty()
        || !isolated.is_disjoint(&housekeeping)
        || isolated
            .union(&housekeeping)
            .copied()
            .collect::<BTreeSet<_>>()
            != online
    {
        return Err(HostReceiptError::Invariant);
    }
    let expected_cpu_keys = online
        .iter()
        .map(|cpu| format!("cpu{cpu}"))
        .collect::<BTreeSet<_>>();
    if receipt
        .cpu_governors
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>()
        != expected_cpu_keys
        || receipt
            .cpu_governors
            .values()
            .any(|value| !bounded(value, 64))
    {
        return Err(HostReceiptError::Invariant);
    }
    let expected_tunables = TUNABLES
        .iter()
        .map(|(key, _)| (*key).to_owned())
        .collect::<BTreeSet<_>>();
    if receipt
        .kernel_tunables
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>()
        != expected_tunables
        || receipt
            .kernel_tunables
            .values()
            .any(|value| !bounded(value, 4_096))
    {
        return Err(HostReceiptError::Invariant);
    }
    Ok(())
}

fn validate_mount(mount: &MountIdentity) -> Result<(), HostReceiptError> {
    if mount.mount_id == 0
        || !bounded(&mount.device_major_minor, 64)
        || !bounded(&mount.root, 4_096)
        || !bounded(&mount.mount_point, 4_096)
        || !bounded(&mount.filesystem_type, 256)
        || !bounded(&mount.source, 4_096)
        || mount.mount_options.is_empty()
        || mount.super_options.is_empty()
        || mount.mount_options.len() > 128
        || mount.super_options.len() > 128
        || mount.mount_options.iter().any(|value| !bounded(value, 256))
        || mount.super_options.iter().any(|value| !bounded(value, 256))
        || !strictly_sorted(&mount.mount_options)
        || !strictly_sorted(&mount.super_options)
    {
        return Err(HostReceiptError::Invariant);
    }
    let mut device = mount.device_major_minor.split(':');
    if device
        .next()
        .and_then(|part| part.parse::<u32>().ok())
        .is_none()
        || device
            .next()
            .and_then(|part| part.parse::<u32>().ok())
            .is_none()
        || device.next().is_some()
    {
        return Err(HostReceiptError::Invariant);
    }
    Ok(())
}

fn require_canonical_cpuset(value: &str) -> Result<BTreeSet<u32>, HostReceiptError> {
    let parsed = parse_cpuset(value).map_err(|_| HostReceiptError::Invariant)?;
    if format_cpuset(&parsed) != value {
        return Err(HostReceiptError::Invariant);
    }
    Ok(parsed)
}

#[cfg(any(test, target_os = "linux"))]
fn canonical_cpuset(value: &str) -> Result<String, HostReceiptError> {
    Ok(format_cpuset(&parse_cpuset(value)?))
}

fn parse_cpuset(value: &str) -> Result<BTreeSet<u32>, HostReceiptError> {
    if value.is_empty() || value.len() > 65_536 {
        return Err(HostReceiptError::Observation);
    }
    let mut cpus = BTreeSet::new();
    for item in value.split(',') {
        if item.is_empty() {
            return Err(HostReceiptError::Observation);
        }
        let mut bounds = item.split('-');
        let start = bounds
            .next()
            .and_then(|part| part.parse::<u32>().ok())
            .filter(|cpu| *cpu <= 65_535)
            .ok_or(HostReceiptError::Observation)?;
        let end = bounds
            .next()
            .map(|part| part.parse::<u32>())
            .transpose()
            .map_err(|_| HostReceiptError::Observation)?
            .unwrap_or(start);
        if bounds.next().is_some() || end < start || end > 65_535 {
            return Err(HostReceiptError::Observation);
        }
        cpus.extend(start..=end);
    }
    Ok(cpus)
}

fn format_cpuset(cpus: &BTreeSet<u32>) -> String {
    let mut ranges = Vec::new();
    let mut iter = cpus.iter().copied();
    let Some(mut start) = iter.next() else {
        return String::new();
    };
    let mut end = start;
    for cpu in iter {
        if cpu == end + 1 {
            end = cpu;
        } else {
            ranges.push(format_range(start, end));
            start = cpu;
            end = cpu;
        }
    }
    ranges.push(format_range(start, end));
    ranges.join(",")
}

fn format_range(start: u32, end: u32) -> String {
    if start == end {
        start.to_string()
    } else {
        format!("{start}-{end}")
    }
}

fn bounded(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && !value.contains('\0')
        && !value.contains('\n')
        && !value.contains('\r')
}

fn strictly_sorted(values: &[String]) -> bool {
    values.windows(2).all(|pair| pair[0] < pair[1])
}

#[cfg(target_os = "linux")]
fn parse_mountinfo(value: &str, target: &Path) -> Result<MountIdentity, HostReceiptError> {
    let target = target.to_str().ok_or(HostReceiptError::Observation)?;
    let mut matches = Vec::new();
    for line in value.lines() {
        let (before, after) = line
            .split_once(" - ")
            .ok_or(HostReceiptError::Observation)?;
        let fields = before.split_ascii_whitespace().collect::<Vec<_>>();
        let tail = after.split_ascii_whitespace().collect::<Vec<_>>();
        if fields.len() < 6 || tail.len() != 3 {
            return Err(HostReceiptError::Observation);
        }
        let mount_point = unescape_mount_field(fields[4])?;
        if path_contains(&mount_point, target) {
            matches.push(MountIdentity {
                mount_id: fields[0]
                    .parse::<u64>()
                    .map_err(|_| HostReceiptError::Observation)?,
                device_major_minor: fields[2].to_owned(),
                root: unescape_mount_field(fields[3])?,
                mount_point,
                mount_options: sorted_options(fields[5])?,
                filesystem_type: unescape_mount_field(tail[0])?,
                source: unescape_mount_field(tail[1])?,
                super_options: sorted_options(tail[2])?,
            });
        }
    }
    matches
        .into_iter()
        .max_by_key(|mount| mount.mount_point.len())
        .ok_or(HostReceiptError::Observation)
}

#[cfg(target_os = "linux")]
fn path_contains(mount_point: &str, target: &str) -> bool {
    mount_point == "/"
        || target == mount_point
        || target
            .strip_prefix(mount_point)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

#[cfg(target_os = "linux")]
fn sorted_options(value: &str) -> Result<Vec<String>, HostReceiptError> {
    let mut options = value
        .split(',')
        .map(unescape_mount_field)
        .collect::<Result<Vec<_>, _>>()?;
    if options.iter().any(String::is_empty) {
        return Err(HostReceiptError::Observation);
    }
    options.sort();
    options.dedup();
    Ok(options)
}

#[cfg(target_os = "linux")]
fn unescape_mount_field(value: &str) -> Result<String, HostReceiptError> {
    let bytes = value.as_bytes();
    let mut result = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'\\' {
            result.push(bytes[index]);
            index += 1;
            continue;
        }
        if index + 3 >= bytes.len() {
            return Err(HostReceiptError::Observation);
        }
        let escaped = &bytes[index + 1..index + 4];
        let decoded = match escaped {
            b"040" => b' ',
            b"011" => b'\t',
            b"012" => b'\n',
            b"134" => b'\\',
            _ => return Err(HostReceiptError::Observation),
        };
        result.push(decoded);
        index += 4;
    }
    let result = String::from_utf8(result).map_err(|_| HostReceiptError::Observation)?;
    if result.contains('\0') || result.contains('\n') || result.contains('\r') {
        return Err(HostReceiptError::Observation);
    }
    Ok(result)
}

#[cfg(target_os = "linux")]
fn read_trimmed(path: &Path, maximum: usize) -> Result<String, HostReceiptError> {
    let value = read_virtual(path, maximum)?;
    let value = value.trim();
    if !bounded(value, maximum) {
        return Err(HostReceiptError::Observation);
    }
    Ok(value.to_owned())
}

#[cfg(target_os = "linux")]
fn read_virtual(path: &Path, maximum: usize) -> Result<String, HostReceiptError> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(maximum as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.is_empty() || bytes.len() > maximum {
        return Err(HostReceiptError::Observation);
    }
    String::from_utf8(bytes).map_err(|_| HostReceiptError::Observation)
}

#[cfg(target_os = "linux")]
fn safe_regular_metadata(path: &Path, maximum: u64) -> Result<fs::Metadata, HostReceiptError> {
    use std::os::unix::fs::MetadataExt;

    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() == 0
        || metadata.len() > maximum
        || metadata.nlink() != 1
    {
        return Err(HostReceiptError::Path);
    }
    Ok(metadata)
}

#[cfg(target_os = "linux")]
fn read_regular_bounded(path: &Path, maximum: u64) -> Result<Vec<u8>, HostReceiptError> {
    let metadata = safe_regular_metadata(path, maximum)?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    File::open(path)?
        .take(maximum + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > maximum {
        return Err(HostReceiptError::Path);
    }
    Ok(bytes)
}

#[cfg(target_os = "linux")]
fn sha256_file(path: &Path, maximum: u64) -> Result<String, HostReceiptError> {
    use sha2::{Digest, Sha256};

    let metadata = safe_regular_metadata(path, maximum)?;
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut consumed = 0_u64;
    let mut buffer = [0_u8; 65_536];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        consumed = consumed.saturating_add(count as u64);
        if consumed > maximum {
            return Err(HostReceiptError::Path);
        }
        digest.update(&buffer[..count]);
    }
    if consumed != metadata.len() {
        return Err(HostReceiptError::Path);
    }
    let digest = digest.finalize();
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

#[cfg(target_os = "linux")]
fn verify_head(path: &Path, expected: &str) -> Result<(), HostReceiptError> {
    let bytes = read_regular_bounded(path, 65).map_err(|_| HostReceiptError::Path)?;
    if bytes != format!("{expected}\n").as_bytes() {
        return Err(HostReceiptError::Digest);
    }
    Ok(())
}

pub fn encode_canonical(receipt: &HostObservationReceipt) -> Result<Vec<u8>, HostReceiptError> {
    validate_receipt(receipt)?;
    canonical_json(receipt).map_err(|_| HostReceiptError::Document)
}

#[cfg(target_os = "linux")]
pub fn write_receipt_for_admission(
    receipt: &HostObservationReceipt,
    campaign_directory: &Path,
) -> Result<String, HostReceiptError> {
    use std::os::unix::fs::OpenOptionsExt;

    if unsafe { libc::geteuid() } != 0 {
        return Err(HostReceiptError::Path);
    }
    let directory_metadata = fs::symlink_metadata(campaign_directory)?;
    if !directory_metadata.is_dir() || directory_metadata.file_type().is_symlink() {
        return Err(HostReceiptError::Path);
    }
    let bytes = encode_canonical(receipt)?;
    let digest = sha256_hex(&bytes);
    let output = campaign_directory.join(HOST_RECEIPT_NAME);
    let head = campaign_directory.join(HOST_RECEIPT_HEAD_NAME);
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o400)
        .open(output)?;
    file.write_all(&bytes)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    let mut head_file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o400)
        .open(head)?;
    head_file.write_all(format!("{digest}\n").as_bytes())?;
    head_file.sync_all()?;
    File::open(campaign_directory)?.sync_all()?;
    Ok(digest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn receipt() -> HostObservationReceipt {
        let campaign_mount = MountIdentity {
            mount_id: 31,
            device_major_minor: "8:2".to_owned(),
            root: "/".to_owned(),
            mount_point: "/var/lib/hydracache-performance".to_owned(),
            mount_options: vec!["relatime".to_owned(), "rw".to_owned()],
            filesystem_type: "ext4".to_owned(),
            source: "/dev/nvme0n1p2".to_owned(),
            super_options: vec!["errors=remount-ro".to_owned(), "rw".to_owned()],
        };
        HostObservationReceipt {
            schema_version: 1,
            machine_id: "a".repeat(32),
            boot_id: "123e4567-e89b-42d3-a456-426614174000".to_owned(),
            kernel_release: "6.8.0-90-generic".to_owned(),
            kernel_command_line_sha256: "b".repeat(64),
            mount_identity: mount_identity_digest(&campaign_mount).unwrap(),
            campaign_mount,
            online_cpuset: "0-7".to_owned(),
            isolated_cpuset: "1-4".to_owned(),
            housekeeping_cpuset: "0,5-7".to_owned(),
            cpu_governors: (0..8)
                .map(|cpu| (format!("cpu{cpu}"), "performance".to_owned()))
                .collect(),
            kernel_tunables: TUNABLES
                .iter()
                .map(|(key, _)| ((*key).to_owned(), "0".to_owned()))
                .collect(),
            supervisor_binary: BinaryIdentity {
                path: SUPERVISOR_BINARY_PATH.to_owned(),
                sha256: "c".repeat(64),
                size: 1_024,
                inode: 44,
                device: 8,
                uid: 0,
                gid: 0,
                mode: 0o755,
            },
            reference_host_freeze_sha256: "d".repeat(64),
        }
    }

    #[test]
    fn canonical_receipt_round_trips_and_detects_drift() {
        let receipt = receipt();
        let encoded = encode_canonical(&receipt).unwrap();
        let digest = sha256_hex(&encoded);
        assert_eq!(parse_and_validate(&encoded, &digest).unwrap(), receipt);
        assert!(matches!(
            parse_and_validate(&[encoded, b" ".to_vec()].concat(), &digest),
            Err(HostReceiptError::Digest)
        ));
        let mut drifted = receipt.clone();
        drifted.kernel_release.push_str("-drift");
        assert_eq!(
            verify_live_observation(&receipt, &drifted),
            Err(HostReceiptError::Drift)
        );
    }

    #[test]
    fn quarantine_recovery_allows_only_the_provisioned_supervisor_identity_to_change() {
        let admitted = receipt();
        let mut upgraded = admitted.clone();
        upgraded.supervisor_binary.sha256 = "e".repeat(64);
        upgraded.supervisor_binary.size += 1;
        upgraded.supervisor_binary.inode += 1;
        assert_eq!(
            verify_quarantine_recovery_live_observation(&admitted, &upgraded),
            Ok(())
        );

        upgraded.kernel_release.push_str("-drift");
        assert_eq!(
            verify_quarantine_recovery_live_observation(&admitted, &upgraded),
            Err(HostReceiptError::Drift)
        );
    }

    #[test]
    fn live_observation_ignores_only_namespace_local_mount_id() {
        let admitted = receipt();
        let mut new_namespace = admitted.clone();
        new_namespace.campaign_mount.mount_id += 1;

        assert_eq!(
            mount_identity_digest(&admitted.campaign_mount),
            mount_identity_digest(&new_namespace.campaign_mount)
        );
        assert_eq!(verify_live_observation(&admitted, &new_namespace), Ok(()));

        let mutations: [fn(&mut MountIdentity); 7] = [
            |mount| mount.device_major_minor = "8:3".to_owned(),
            |mount| mount.root = "/different-root".to_owned(),
            |mount| mount.mount_point = "/different-mount".to_owned(),
            |mount| mount.mount_options.push("sync".to_owned()),
            |mount| mount.filesystem_type = "xfs".to_owned(),
            |mount| mount.source = "/dev/nvme1n1p2".to_owned(),
            |mount| mount.super_options.insert(0, "discard".to_owned()),
        ];
        for mutate in mutations {
            let mut different_backing = new_namespace.clone();
            mutate(&mut different_backing.campaign_mount);
            different_backing.mount_identity =
                mount_identity_digest(&different_backing.campaign_mount).unwrap();
            assert_eq!(
                verify_live_observation(&admitted, &different_backing),
                Err(HostReceiptError::Drift)
            );
        }
    }

    #[test]
    fn provisioning_source_is_strict_and_binary_bound() {
        let binary = "e".repeat(64);
        let source = "f".repeat(40);
        let mut value = serde_json::json!({
            "schema_version": "hydracache-w11-host-provisioning-v1",
            "source_commit": source,
            "created_at_utc": "2026-10-06T00:00:00Z",
            "mutation_performed": true,
            "repository_id": 1,
            "allowed_actor_ids": [2],
            "runner_uid": 1001,
            "runner_gid": 1001,
            "supervisor_uid": 986,
            "client_gid": 987,
            "runner_group_database_membership": true,
            "runner_process_group_refresh_may_be_required": true,
            "service_active": true,
            "socket_mode": 432,
            "binary_sha256": binary,
            "fixture_binary_sha256": "e".repeat(64),
            "config_sha256": "1".repeat(64),
            "service_sha256": "2".repeat(64),
            "sysusers_sha256": "3".repeat(64),
            "tmpfiles_sha256": "4".repeat(64),
            "verification_key_sha256": "5".repeat(64),
            "unit_properties_sha256": "6".repeat(64),
            "machine_id_sha256": "7".repeat(64),
            "boot_id_sha256": "8".repeat(64),
        });
        let bytes = serde_json::to_vec(&value).unwrap();
        let identity = parse_installed_provisioning_identity(&bytes, &"e".repeat(64)).unwrap();
        assert_eq!(identity.source_commit, "f".repeat(40));
        assert_eq!(identity.fixture_binary_sha256, "e".repeat(64));

        assert_eq!(
            parse_installed_provisioning_identity(&bytes, &"d".repeat(64)),
            Err(HostReceiptError::Invariant)
        );
        value["fixture_binary_sha256"] = serde_json::json!("invalid");
        assert_eq!(
            parse_installed_provisioning_identity(
                &serde_json::to_vec(&value).unwrap(),
                &"e".repeat(64)
            ),
            Err(HostReceiptError::Invariant)
        );
        value["fixture_binary_sha256"] = serde_json::json!("e".repeat(64));
        value["unexpected"] = serde_json::json!(true);
        assert_eq!(
            parse_installed_provisioning_identity(
                &serde_json::to_vec(&value).unwrap(),
                &"e".repeat(64)
            ),
            Err(HostReceiptError::Invariant)
        );
    }

    #[test]
    fn installed_binary_identity_requires_every_manifest_field() {
        let observed = receipt().supervisor_binary;
        let mut expected = InstalledBinary {
            role: "i74".to_owned(),
            path: observed.path.clone(),
            sha256: observed.sha256.clone(),
            size: observed.size,
            inode: observed.inode,
            device: observed.device,
            uid: observed.uid,
            gid: observed.gid,
            mode: u64::from(observed.mode),
        };
        assert!(binary_identity_matches(&observed, &expected));
        let mut drifted = Vec::new();
        let mut value = expected.clone();
        value.path.push_str("-other");
        drifted.push(value);
        let mut value = expected.clone();
        value.sha256 = "0".repeat(64);
        drifted.push(value);
        let mut value = expected.clone();
        value.size += 1;
        drifted.push(value);
        let mut value = expected.clone();
        value.inode += 1;
        drifted.push(value);
        let mut value = expected.clone();
        value.device += 1;
        drifted.push(value);
        let mut value = expected.clone();
        value.uid += 1;
        drifted.push(value);
        let mut value = expected.clone();
        value.gid += 1;
        drifted.push(value);
        expected.mode += 1;
        drifted.push(expected);
        assert!(drifted
            .iter()
            .all(|value| !binary_identity_matches(&observed, value)));
    }

    #[test]
    fn partition_and_fixed_probe_set_are_fail_closed() {
        let mut value = receipt();
        value.housekeeping_cpuset = "0,4-7".to_owned();
        assert_eq!(encode_canonical(&value), Err(HostReceiptError::Invariant));

        let mut value = receipt();
        value.cpu_governors.remove("cpu7");
        assert_eq!(encode_canonical(&value), Err(HostReceiptError::Invariant));

        let mut value = receipt();
        value
            .kernel_tunables
            .insert("vm.extra".to_owned(), "1".to_owned());
        assert_eq!(encode_canonical(&value), Err(HostReceiptError::Invariant));
    }

    #[test]
    fn cpuset_parser_canonicalizes_ranges() {
        assert_eq!(canonical_cpuset("0,1,2-4,7").unwrap(), "0-4,7");
        assert!(parse_cpuset("4-2").is_err());
        assert!(parse_cpuset("0,,1").is_err());
        assert!(parse_cpuset("0-70000").is_err());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn mountinfo_parser_selects_longest_mount_and_unescapes() {
        let input = concat!(
            "30 1 8:1 / / rw,relatime shared:1 - ext4 /dev/root rw,errors=remount-ro\n",
            "31 30 8:2 / /var/lib/hydracache\\040performance rw,nosuid - ext4 /dev/nvme0n1p2 rw,nodev\n"
        );
        let mount = parse_mountinfo(
            input,
            Path::new("/var/lib/hydracache performance/campaigns/a"),
        )
        .unwrap();
        assert_eq!(mount.mount_id, 31);
        assert_eq!(mount.mount_point, "/var/lib/hydracache performance");
        assert_eq!(mount.mount_options, ["nosuid", "rw"]);
    }
}
