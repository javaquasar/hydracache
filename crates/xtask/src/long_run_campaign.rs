use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

const DOMAIN: &[u8] = b"hydracache-long-run-record-v1";
const GENESIS: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const MAX_CAMPAIGN_BYTES: u64 = 21_474_836_480;
const MAX_CAMPAIGN_FILES: usize = 20_000;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PacketManifest {
    schema_version: u32,
    release: String,
    campaign_id: String,
    campaign_manifest: PathBuf,
    campaign_manifest_sha256: String,
    raw_manifest: PathBuf,
    raw_manifest_sha256: String,
    raw_manifest_set_sha256: String,
    result: String,
    promotable: bool,
    terminal_reason: Option<String>,
    required_final_guards: Vec<String>,
    guard_results: Vec<GuardResult>,
    roles: Vec<RoleManifest>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GuardResult {
    id: String,
    passed: bool,
    evidence_sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RoleManifest {
    id: String,
    result: String,
    journal: PathBuf,
    journal_sha256: String,
    expected_bytes: u64,
    expected_records: u64,
    first_record_sha256: String,
    expected_head_sha256: String,
    allow_incomplete_trailing_bytes: bool,
    harness: ProcessIdentity,
    daemon: ProcessIdentity,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawManifest {
    schema_version: u32,
    release: String,
    campaign_id: String,
    file_count: usize,
    total_bytes: u64,
    set_sha256: String,
    files: Vec<RawFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawFile {
    pub path: PathBuf,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessIdentity {
    pub boot_id: String,
    pub pid: u32,
    pub start_ticks: u64,
    pub process_group: i64,
    pub cgroup_path: String,
    pub cgroup_inode: u64,
    pub unit_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    schema_version: u32,
    sequence: u64,
    previous_record_sha256: String,
    payload: Value,
    payload_sha256: String,
    record_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalReport {
    pub records: u64,
    pub head_sha256: String,
    pub campaign_id: String,
    pub role: String,
    pub first_record_sha256: String,
    pub harness: ProcessIdentity,
    pub daemon: ProcessIdentity,
    pub incomplete_trailing_bytes: usize,
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let mut release = None;
    let mut manifest = None;
    let mut args = args.into_iter();
    while let Some(name) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value after {name}"))?;
        match name.as_str() {
            "--release" => release = Some(value),
            "--manifest" => manifest = Some(PathBuf::from(value)),
            _ => return Err(format!("unsupported long-run-campaign-check argument {name}").into()),
        }
    }
    if release.as_deref() != Some("0.74") {
        return Err("long-run-campaign-check currently requires --release 0.74".into());
    }
    let manifest = manifest.ok_or("--manifest is required")?;
    verify_manifest(&manifest)?;
    println!("long-run-campaign-check 0.74: OK ({})", manifest.display());
    Ok(())
}

pub fn verify_manifest(path: &Path) -> Result<Vec<JournalReport>, Box<dyn Error>> {
    let packet_metadata = fs::symlink_metadata(path)?;
    if !packet_metadata.is_file()
        || packet_metadata.file_type().is_symlink()
        || has_multiple_links(path, &packet_metadata)?
    {
        return Err("packet manifest must be one ordinary unlinked file".into());
    }
    let packet_bytes = fs::read(path)?;
    let value: PacketManifest = parse_canonical_json(&packet_bytes, "packet manifest")?;
    if value.schema_version != 1 || value.release != "0.74" || !is_hash(&value.campaign_id) {
        return Err("packet manifest identity is invalid".into());
    }
    if !matches!(value.result.as_str(), "complete" | "incomplete") {
        return Err("packet result must be complete or incomplete".into());
    }
    if value.roles.is_empty() || value.roles.len() > 2 {
        return Err("packet must contain one or two role journals".into());
    }
    if value.promotable && (value.result != "complete" || value.roles.len() != 2) {
        return Err("promotable packet requires complete I74 and C74 roles".into());
    }
    let root = path.parent().unwrap_or_else(|| Path::new("."));
    let packet_relative = path
        .file_name()
        .map(PathBuf::from)
        .ok_or("packet manifest path has no file name")?;
    validate_packet_guards(&value)?;
    let raw = verify_raw_manifest(root, &packet_relative, &value)?;
    if value.guard_results.iter().any(|guard| {
        !raw.files
            .iter()
            .any(|file| file.sha256 == guard.evidence_sha256)
    }) {
        return Err("guard evidence digest is absent from the raw file set".into());
    }
    verify_campaign_manifest(root, &raw, &value)?;

    let mut reports = Vec::with_capacity(value.roles.len());
    for role in &value.roles {
        if !matches!(role.id.as_str(), "i74" | "c74") {
            return Err(format!("unsupported role {}", role.id).into());
        }
        if !matches!(role.result.as_str(), "complete" | "incomplete") {
            return Err(format!("{} role result is invalid", role.id).into());
        }
        if value.promotable && role.result != "complete" {
            return Err("promotable packet requires complete role results".into());
        }
        validate_process_identity(&role.harness)?;
        validate_process_identity(&role.daemon)?;
        if role.harness.boot_id != role.daemon.boot_id {
            return Err(format!("{} role process boot identities differ", role.id).into());
        }
        validate_relative_path(&role.journal)?;
        if !is_hash(&role.journal_sha256) {
            return Err(format!("{} journal digest is invalid", role.id).into());
        }
        let journal_path = root.join(&role.journal);
        reject_symlink_components(root, &role.journal)?;
        let metadata = fs::metadata(&journal_path)?;
        if !metadata.is_file()
            || metadata.len() != role.expected_bytes
            || metadata.len() > MAX_CAMPAIGN_BYTES
        {
            return Err(format!("{} journal size or file type is invalid", role.id).into());
        }
        let journal_bytes = fs::read(&journal_path)?;
        if hex(&Sha256::digest(&journal_bytes)) != role.journal_sha256 {
            return Err(format!(
                "{} journal digest does not match the packet manifest",
                role.id
            )
            .into());
        }
        let report = verify_journal_bytes(&journal_bytes)?;
        if report.campaign_id != value.campaign_id
            || report.role != role.id
            || report.records != role.expected_records
            || report.first_record_sha256 != role.first_record_sha256
            || report.head_sha256 != role.expected_head_sha256
            || report.harness != role.harness
            || report.daemon != role.daemon
        {
            return Err(format!("{} journal does not match the packet manifest", role.id).into());
        }
        if report.incomplete_trailing_bytes != 0 && !role.allow_incomplete_trailing_bytes {
            return Err(format!("{} journal has an unapproved incomplete tail", role.id).into());
        }
        if value.promotable && report.incomplete_trailing_bytes != 0 {
            return Err("promotable packet cannot recover incomplete trailing bytes".into());
        }
        reports.push(report);
    }
    reports.sort_by(|left, right| left.role.cmp(&right.role));
    reports.dedup_by(|left, right| left.role == right.role);
    if reports.len() != value.roles.len() {
        return Err("packet contains a duplicate role".into());
    }
    if value.promotable
        && reports
            .iter()
            .map(|report| report.role.as_str())
            .collect::<Vec<_>>()
            != ["c74", "i74"]
    {
        return Err("promotable packet must contain I74 and C74 exactly once".into());
    }
    Ok(reports)
}

fn validate_packet_guards(value: &PacketManifest) -> Result<(), Box<dyn Error>> {
    if !is_hash(&value.campaign_manifest_sha256)
        || !is_hash(&value.raw_manifest_sha256)
        || !is_hash(&value.raw_manifest_set_sha256)
        || value.required_final_guards.is_empty()
    {
        return Err("packet manifest digest or guard identity is invalid".into());
    }
    match (value.result.as_str(), value.terminal_reason.as_deref()) {
        ("complete", None) => {}
        ("incomplete", Some(reason)) if !reason.is_empty() && reason.len() <= 128 => {}
        _ => return Err("packet terminal reason does not match its result".into()),
    }
    let required = value
        .required_final_guards
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if required.len() != value.required_final_guards.len()
        || required
            .iter()
            .any(|guard| guard.is_empty() || guard.len() > 128)
    {
        return Err("required final guards must be unique bounded identifiers".into());
    }
    let results = value
        .guard_results
        .iter()
        .map(|guard| guard.id.as_str())
        .collect::<BTreeSet<_>>();
    if results != required
        || results.len() != value.guard_results.len()
        || value
            .guard_results
            .iter()
            .any(|guard| !is_hash(&guard.evidence_sha256) || (value.promotable && !guard.passed))
    {
        return Err("guard results do not exactly satisfy the required guards".into());
    }
    Ok(())
}

fn verify_raw_manifest(
    root: &Path,
    packet_relative: &Path,
    packet: &PacketManifest,
) -> Result<RawManifest, Box<dyn Error>> {
    validate_relative_path(&packet.raw_manifest)?;
    if packet.raw_manifest != Path::new("raw-manifest.json") {
        return Err("raw manifest must use the frozen raw-manifest.json path".into());
    }
    reject_symlink_components(root, &packet.raw_manifest)?;
    let raw_path = root.join(&packet.raw_manifest);
    let raw_metadata = fs::metadata(&raw_path)?;
    if !raw_metadata.is_file() || has_multiple_links(&raw_path, &raw_metadata)? {
        return Err("raw manifest must be one ordinary unlinked file".into());
    }
    let raw_bytes = fs::read(raw_path)?;
    if hex(&Sha256::digest(&raw_bytes)) != packet.raw_manifest_sha256 {
        return Err("raw manifest digest does not match the packet".into());
    }
    let raw: RawManifest = parse_canonical_json(&raw_bytes, "raw manifest")?;
    if raw.schema_version != 1
        || raw.release != "0.74"
        || raw.campaign_id != packet.campaign_id
        || raw.file_count != raw.files.len()
        || raw.file_count == 0
        || raw.file_count > MAX_CAMPAIGN_FILES
        || raw.total_bytes == 0
        || raw.total_bytes > MAX_CAMPAIGN_BYTES
        || raw.set_sha256 != packet.raw_manifest_set_sha256
        || raw.set_sha256 != raw_file_set_sha256(&raw.files)?
        || raw
            .files
            .iter()
            .any(|file| !is_hash(&file.sha256) || validate_relative_path(&file.path).is_err())
    {
        return Err("raw manifest identity, limits, or set digest is invalid".into());
    }
    let actual = enumerate_raw_files(root, packet_relative, &packet.raw_manifest)?;
    if actual != raw.files || checked_total_bytes(&actual) != Some(raw.total_bytes) {
        return Err("raw manifest does not enumerate the exact packet files".into());
    }
    Ok(raw)
}

fn verify_campaign_manifest(
    root: &Path,
    raw: &RawManifest,
    packet: &PacketManifest,
) -> Result<(), Box<dyn Error>> {
    validate_relative_path(&packet.campaign_manifest)?;
    let row = raw
        .files
        .iter()
        .find(|file| file.path == packet.campaign_manifest)
        .ok_or("campaign manifest is absent from the raw manifest")?;
    let campaign_bytes = fs::read(root.join(&packet.campaign_manifest))?;
    if row.sha256 != hex(&Sha256::digest(&campaign_bytes)) {
        return Err("campaign manifest raw digest does not match its file row".into());
    }
    let encoded = campaign_bytes
        .strip_suffix(b"\n")
        .unwrap_or(campaign_bytes.as_slice());
    if hex(&Sha256::digest(encoded)) != packet.campaign_manifest_sha256 {
        return Err("campaign manifest identity digest does not match the packet".into());
    }
    let campaign: Value = parse_canonical_json(encoded, "campaign manifest")?;
    if campaign.get("release").and_then(Value::as_str) != Some("0.74")
        || campaign.get("campaign_id").and_then(Value::as_str) != Some(packet.campaign_id.as_str())
    {
        return Err("campaign manifest identity does not match the packet".into());
    }
    Ok(())
}

fn validate_process_identity(identity: &ProcessIdentity) -> Result<(), Box<dyn Error>> {
    if identity.pid == 0
        || identity.start_ticks == 0
        || identity.process_group <= 0
        || identity.cgroup_inode == 0
        || identity.boot_id.is_empty()
        || identity.boot_id.len() > 256
        || identity.cgroup_path.is_empty()
        || identity.cgroup_path.len() > 1024
        || identity.unit_name.is_empty()
        || identity.unit_name.len() > 256
    {
        return Err("packet process identity is invalid".into());
    }
    Ok(())
}

fn reject_symlink_components(root: &Path, relative: &Path) -> Result<(), Box<dyn Error>> {
    let mut current = root.to_path_buf();
    for component in relative.components() {
        let std::path::Component::Normal(name) = component else {
            return Err("journal path contains a non-normal component".into());
        };
        current.push(name);
        if fs::symlink_metadata(&current)?.file_type().is_symlink() {
            return Err(format!("journal path contains symlink {}", current.display()).into());
        }
    }
    Ok(())
}

fn validate_relative_path(relative: &Path) -> Result<(), Box<dyn Error>> {
    let text = relative.to_str().ok_or("packet path is not valid UTF-8")?;
    if text.is_empty()
        || text.len() > 1024
        || text.contains('\0')
        || relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err("packet path must contain only relative normal components".into());
    }
    Ok(())
}

fn enumerate_raw_files(
    root: &Path,
    packet_relative: &Path,
    raw_manifest_relative: &Path,
) -> Result<Vec<RawFile>, Box<dyn Error>> {
    let mut files = Vec::new();
    visit_files(
        root,
        root,
        packet_relative,
        raw_manifest_relative,
        &mut files,
    )?;
    files.sort_by(|left, right| {
        left.path
            .to_str()
            .unwrap_or_default()
            .as_bytes()
            .cmp(right.path.to_str().unwrap_or_default().as_bytes())
    });
    if files.len() > MAX_CAMPAIGN_FILES
        || checked_total_bytes(&files).is_none_or(|bytes| bytes > MAX_CAMPAIGN_BYTES)
    {
        return Err("packet file count or byte limit exceeded".into());
    }
    Ok(files)
}

fn checked_total_bytes(files: &[RawFile]) -> Option<u64> {
    files
        .iter()
        .try_fold(0_u64, |total, file| total.checked_add(file.size))
}

fn visit_files(
    root: &Path,
    directory: &Path,
    packet_relative: &Path,
    raw_manifest_relative: &Path,
    files: &mut Vec<RawFile>,
) -> Result<(), Box<dyn Error>> {
    let mut entries = fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Err(format!("packet contains symlink {}", path.display()).into());
        }
        if metadata.is_dir() {
            visit_files(root, &path, packet_relative, raw_manifest_relative, files)?;
            continue;
        }
        if !metadata.is_file() || has_multiple_links(&path, &metadata)? {
            return Err(format!("packet contains unsupported file {}", path.display()).into());
        }
        let relative = path.strip_prefix(root)?.to_path_buf();
        validate_relative_path(&relative)?;
        if relative == packet_relative || relative == raw_manifest_relative {
            continue;
        }
        if relative.to_str().is_none() {
            return Err("packet paths must be valid UTF-8".into());
        }
        let bytes = fs::read(&path)?;
        if bytes.len() as u64 != metadata.len() {
            return Err("packet file changed while it was verified".into());
        }
        files.push(RawFile {
            path: relative,
            size: metadata.len(),
            sha256: hex(&Sha256::digest(bytes)),
        });
    }
    Ok(())
}

#[cfg(unix)]
fn has_multiple_links(_path: &Path, metadata: &fs::Metadata) -> Result<bool, Box<dyn Error>> {
    use std::os::unix::fs::MetadataExt;
    Ok(metadata.nlink() > 1)
}

#[cfg(windows)]
fn has_multiple_links(path: &Path, _metadata: &fs::Metadata) -> Result<bool, Box<dyn Error>> {
    use std::fs::File;
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
fn has_multiple_links(_path: &Path, _metadata: &fs::Metadata) -> Result<bool, Box<dyn Error>> {
    Ok(false)
}

pub fn raw_file_set_sha256(files: &[RawFile]) -> Result<String, Box<dyn Error>> {
    Ok(hex(&Sha256::digest(canonical_json(files)?)))
}

fn parse_canonical_json<T: for<'de> Deserialize<'de>>(
    bytes: &[u8],
    label: &str,
) -> Result<T, Box<dyn Error>> {
    let encoded = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    if encoded.is_empty() || encoded.contains(&b'\r') || encoded.contains(&b'\n') {
        return Err(format!("{label} is not one canonical JSON document").into());
    }
    let value: Value = serde_json::from_slice(encoded)?;
    if serde_json::to_vec(&value)? != encoded {
        return Err(format!("{label} is not canonical JSON").into());
    }
    Ok(serde_json::from_value(value)?)
}

fn canonical_json<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(serde_json::to_vec(&serde_json::to_value(value)?)?)
}

pub fn verify_journal_bytes(bytes: &[u8]) -> Result<JournalReport, Box<dyn Error>> {
    if bytes.is_empty() {
        return Err("checkpoint journal is empty".into());
    }
    let terminal_newline = bytes.last() == Some(&b'\n');
    let mut lines = bytes.split(|byte| *byte == b'\n').collect::<Vec<_>>();
    if terminal_newline {
        lines.pop();
    }
    let mut incomplete_trailing_bytes = 0;
    if !terminal_newline {
        let tail = lines.last().copied().unwrap_or_default();
        if serde_json::from_slice::<Envelope>(tail).is_err() {
            incomplete_trailing_bytes = tail.len();
            lines.pop();
        }
    }
    if lines.is_empty() {
        return Err("checkpoint journal has no complete records".into());
    }

    let mut previous = GENESIS.to_owned();
    let mut campaign_id = None;
    let mut role = None;
    let mut identities = None;
    let mut elapsed = 0;
    let mut wall_clock = String::new();
    let mut head = String::new();
    let mut first_record_sha256 = None;
    for (index, line) in lines.iter().enumerate() {
        let envelope: Envelope = serde_json::from_slice(line)
            .map_err(|error| format!("invalid checkpoint {}: {error}", index + 1))?;
        let sequence = index as u64 + 1;
        if envelope.schema_version != 1 || envelope.sequence != sequence {
            return Err(format!("checkpoint sequence/schema mismatch at {sequence}").into());
        }
        if !is_hash(&envelope.previous_record_sha256)
            || !is_hash(&envelope.payload_sha256)
            || !is_hash(&envelope.record_sha256)
            || envelope.previous_record_sha256 != previous
        {
            return Err(format!("checkpoint hash link is invalid at {sequence}").into());
        }
        let canonical_payload = serde_json::to_vec(&envelope.payload)?;
        let payload_hash = hex(&Sha256::digest(canonical_payload));
        if payload_hash != envelope.payload_sha256 {
            return Err(format!("checkpoint payload hash is invalid at {sequence}").into());
        }
        let record_hash = record_hash(
            sequence,
            &envelope.previous_record_sha256,
            &envelope.payload_sha256,
        )?;
        if record_hash != envelope.record_sha256 {
            return Err(format!("checkpoint record hash is invalid at {sequence}").into());
        }
        let current_campaign = required_string(&envelope.payload, "campaign_id")?;
        let current_role = required_string(&envelope.payload, "role")?;
        let current_identities = (
            serde_json::from_value::<ProcessIdentity>(
                envelope
                    .payload
                    .get("harness")
                    .cloned()
                    .ok_or("checkpoint payload is missing harness identity")?,
            )?,
            serde_json::from_value::<ProcessIdentity>(
                envelope
                    .payload
                    .get("daemon")
                    .cloned()
                    .ok_or("checkpoint payload is missing daemon identity")?,
            )?,
        );
        let current_elapsed = required_u64(&envelope.payload, "monotonic_elapsed_ns")?;
        let current_wall = required_string(&envelope.payload, "wall_clock_utc")?;
        if let Some(expected) = &campaign_id {
            if expected != current_campaign
                || role.as_deref() != Some(current_role)
                || identities.as_ref() != Some(&current_identities)
            {
                return Err(format!("checkpoint identity drift at {sequence}").into());
            }
            if current_elapsed < elapsed || current_wall < wall_clock.as_str() {
                return Err(format!("checkpoint timestamp reversal at {sequence}").into());
            }
        } else {
            if !is_hash(current_campaign) || !matches!(current_role, "i74" | "c74") {
                return Err("checkpoint campaign or role identity is invalid".into());
            }
            campaign_id = Some(current_campaign.to_owned());
            role = Some(current_role.to_owned());
            identities = Some(current_identities);
            first_record_sha256 = Some(envelope.record_sha256.clone());
        }
        elapsed = current_elapsed;
        wall_clock = current_wall.to_owned();
        previous.clone_from(&envelope.record_sha256);
        head = envelope.record_sha256;
    }
    let (harness, daemon) = identities.ok_or("missing process identities")?;
    Ok(JournalReport {
        records: lines.len() as u64,
        head_sha256: head,
        campaign_id: campaign_id.ok_or("missing campaign id")?,
        role: role.ok_or("missing role")?,
        first_record_sha256: first_record_sha256.ok_or("missing first record digest")?,
        harness,
        daemon,
        incomplete_trailing_bytes,
    })
}

fn record_hash(sequence: u64, previous: &str, payload: &str) -> Result<String, Box<dyn Error>> {
    let mut digest = Sha256::new();
    digest.update(DOMAIN);
    digest.update([0]);
    digest.update(sequence.to_be_bytes());
    digest.update(decode_hash(previous).ok_or("invalid previous hash")?);
    digest.update(decode_hash(payload).ok_or("invalid payload hash")?);
    Ok(hex(&digest.finalize()))
}

fn required_string<'a>(value: &'a Value, field: &str) -> Result<&'a str, Box<dyn Error>> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("checkpoint payload is missing string {field}").into())
}

fn required_u64(value: &Value, field: &str) -> Result<u64, Box<dyn Error>> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("checkpoint payload is missing integer {field}").into())
}

fn is_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn decode_hash(value: &str) -> Option<[u8; 32]> {
    if !is_hash(value) {
        return None;
    }
    let mut output = [0; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
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
