use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

const DOMAIN: &[u8] = b"hydracache-long-run-record-v1";
const GENESIS: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const MAX_CAMPAIGN_BYTES: u64 = 21_474_836_480;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PacketManifest {
    schema_version: u32,
    release: String,
    campaign_id: String,
    result: String,
    promotable: bool,
    roles: Vec<RoleManifest>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RoleManifest {
    id: String,
    journal: PathBuf,
    journal_sha256: String,
    expected_bytes: u64,
    expected_records: u64,
    expected_head_sha256: String,
    allow_incomplete_trailing_bytes: bool,
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
    let value: PacketManifest = serde_json::from_slice(&fs::read(path)?)?;
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
    let mut reports = Vec::with_capacity(value.roles.len());
    for role in &value.roles {
        if !matches!(role.id.as_str(), "i74" | "c74") {
            return Err(format!("unsupported role {}", role.id).into());
        }
        if role.journal.is_absolute()
            || role
                .journal
                .components()
                .any(|component| !matches!(component, std::path::Component::Normal(_)))
        {
            return Err("role journal must contain only relative normal path components".into());
        }
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
            || report.head_sha256 != role.expected_head_sha256
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
            envelope.payload.get("harness").cloned(),
            envelope.payload.get("daemon").cloned(),
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
        }
        elapsed = current_elapsed;
        wall_clock = current_wall.to_owned();
        previous.clone_from(&envelope.record_sha256);
        head = envelope.record_sha256;
    }
    Ok(JournalReport {
        records: lines.len() as u64,
        head_sha256: head,
        campaign_id: campaign_id.ok_or("missing campaign id")?,
        role: role.ok_or("missing role")?,
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
