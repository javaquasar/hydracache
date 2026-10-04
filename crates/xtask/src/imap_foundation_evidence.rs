use crate::imap_value_plane_model::{
    git_head, hex_digest, sha256_file, AuthorityModelReceipt, MODEL_SOURCE, RECEIPT_SCHEMA,
};
use hydracache_cluster_testkit::distributed_value_plane_075::{
    run_seeded_chaos_campaign, ChaosCampaignBounds,
};
use hydracache_cluster_testkit::value_plane_model_075::{
    AUTHORITY_INVARIANT_IDS, AUTHORITY_MODEL_ID,
};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const RELEASE: &str = "0.75";
const REQUIRED_SCHEMAS: [(&str, &str); 4] = [
    ("authority_model", "hydracache.imap.model-receipt.v1"),
    (
        "fault_schedule",
        "hydracache.imap.fault-schedule-receipt.v1",
    ),
    (
        "linearizability",
        "hydracache.imap.linearizability-receipt.v1",
    ),
    ("rpo_rto", "hydracache.imap.rpo-rto-receipt.v1"),
];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvidenceContract {
    release: String,
    contract_version: u32,
    status: String,
    schemas: Vec<SchemaEntry>,
    admission: String,
    production_capability: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SchemaEntry {
    kind: String,
    id: String,
    path: String,
    required_test_id: String,
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let args = parse_args(&args)?;
    let root = repository_root()?;
    let mut problems = check_at_root(&root, &args.release, args.receipt.as_deref())?;
    if let Some(receipts) = args.receipts {
        problems.extend(check_receipt_set_at_root(&root, &args.release, &receipts)?);
    }
    if problems.is_empty() {
        println!(
            "imap-foundation-evidence-check {}: OK (provisional)",
            args.release
        );
        return Ok(());
    }
    for problem in &problems {
        eprintln!("imap-foundation-evidence-check {}: {problem}", args.release);
    }
    Err(format!(
        "foundation evidence check found {} problem(s)",
        problems.len()
    )
    .into())
}

pub fn check_receipt_set_at_root(
    root: &Path,
    release: &str,
    receipts: &Path,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut problems = Vec::new();
    if !receipts.is_dir() {
        return Ok(vec![format!(
            "receipt set is not a directory: {}",
            receipts.display()
        )]);
    }
    let expected_files = [
        "authority-model.json",
        "fault-schedule.json",
        "linearizability.json",
        "rpo-rto.json",
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    let actual_files = fs::read_dir(receipts)?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            (path.extension().and_then(|value| value.to_str()) == Some("json"))
                .then(|| entry.file_name().to_string_lossy().into_owned())
        })
        .collect::<BTreeSet<_>>();
    let actual_names = actual_files
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if actual_names != expected_files {
        problems.push(format!(
            "receipt set must contain exactly the four canonical JSON files; found {actual_names:?}"
        ));
    }
    let mut schema_ids = BTreeSet::new();
    let mut source_shas = BTreeSet::new();
    let mut model_seed = None;
    let mut fault_seed = None;
    for name in &actual_files {
        let path = receipts.join(name);
        problems.extend(check_at_root(root, release, Some(&path))?);
        let value: Value = serde_json::from_slice(&fs::read(path)?)?;
        if let Some(schema) = value.get("schema").and_then(Value::as_str) {
            schema_ids.insert(schema.to_owned());
        }
        if let Some(source_sha) = value.get("source_sha").and_then(Value::as_str) {
            source_shas.insert(source_sha.to_owned());
        }
        match name.as_str() {
            "authority-model.json" => model_seed = value.get("seed").and_then(Value::as_u64),
            "fault-schedule.json" => fault_seed = value.get("seed").and_then(Value::as_u64),
            _ => {}
        }
    }
    let required_schemas = REQUIRED_SCHEMAS
        .iter()
        .map(|(_, schema)| (*schema).to_owned())
        .collect::<BTreeSet<_>>();
    if schema_ids != required_schemas {
        problems.push(
            "receipt set does not cover every registered evidence schema exactly once".into(),
        );
    }
    if source_shas.len() != 1 {
        problems.push("receipt set mixes source commits".into());
    }
    if model_seed.is_none() || model_seed != fault_seed {
        problems.push("authority model and fault schedule receipts must use the same seed".into());
    }
    Ok(problems)
}

pub fn check_at_root(
    root: &Path,
    release: &str,
    receipt: Option<&Path>,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut problems = Vec::new();
    if release != RELEASE {
        return Ok(vec![format!(
            "unsupported release {release}; expected {RELEASE}"
        )]);
    }
    let dir = root.join("docs/testing/imap/0.75");
    let contract: EvidenceContract =
        serde_json::from_slice(&fs::read(dir.join("foundation-evidence-contract.json"))?)?;
    if contract.release != RELEASE
        || contract.contract_version != 1
        || contract.status != "provisional"
    {
        problems.push("foundation evidence contract header is not provisional 0.75/v1".into());
    }
    if contract.admission != "structural_only_until_0_74_published"
        || contract.production_capability != "disabled_fail_closed"
    {
        problems.push("foundation evidence contract must remain fail-closed before 0.74".into());
    }

    let actual = contract
        .schemas
        .iter()
        .map(|entry| (entry.kind.as_str(), entry.id.as_str()))
        .collect::<BTreeSet<_>>();
    let required = REQUIRED_SCHEMAS.into_iter().collect::<BTreeSet<_>>();
    if actual != required {
        problems
            .push("foundation evidence registry must contain exactly four required schemas".into());
    }
    for entry in &contract.schemas {
        if entry.required_test_id.trim().is_empty() {
            problems.push(format!("schema {} has no required test id", entry.id));
        }
        let relative = Path::new(&entry.path);
        if relative.is_absolute()
            || relative
                .components()
                .any(|component| matches!(component, std::path::Component::ParentDir))
        {
            problems.push(format!("schema {} has unsafe path", entry.id));
            continue;
        }
        let schema_path = dir.join(relative);
        let schema: Value = match fs::read(&schema_path)
            .map_err(Box::<dyn Error>::from)
            .and_then(|bytes| serde_json::from_slice(&bytes).map_err(Box::<dyn Error>::from))
        {
            Ok(value) => value,
            Err(error) => {
                problems.push(format!(
                    "cannot load schema {}: {error}",
                    schema_path.display()
                ));
                continue;
            }
        };
        if schema.get("$id").and_then(Value::as_str) != Some(entry.id.as_str()) {
            problems.push(format!("schema {} has a mismatched $id", entry.id));
        }
        if let Err(error) = jsonschema::validator_for(&schema) {
            problems.push(format!("schema {} does not compile: {error}", entry.id));
        }
    }

    if let Some(receipt_path) = receipt {
        validate_receipt(root, &dir, &contract, receipt_path, &mut problems)?;
    }
    Ok(problems)
}

fn validate_receipt(
    root: &Path,
    contract_dir: &Path,
    contract: &EvidenceContract,
    receipt_path: &Path,
    problems: &mut Vec<String>,
) -> Result<(), Box<dyn Error>> {
    let receipt: Value = serde_json::from_slice(&fs::read(receipt_path)?)?;
    let schema_id = receipt
        .get("schema")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let Some(entry) = contract.schemas.iter().find(|entry| entry.id == schema_id) else {
        problems.push(format!("receipt uses unknown schema {schema_id:?}"));
        return Ok(());
    };
    let schema: Value = serde_json::from_slice(&fs::read(contract_dir.join(&entry.path))?)?;
    let validator = jsonschema::validator_for(&schema)?;
    for error in validator.iter_errors(&receipt) {
        problems.push(format!("receipt schema violation: {error}"));
    }
    let expected_head = git_head(root)?;
    if receipt.get("source_sha").and_then(Value::as_str) != Some(expected_head.as_str()) {
        problems.push("receipt source_sha does not match current HEAD".into());
    }
    match schema_id {
        RECEIPT_SCHEMA => validate_model_receipt(root, receipt, problems)?,
        "hydracache.imap.fault-schedule-receipt.v1" => {
            validate_array_checksum(&receipt, "checkpoints", "schedule_sha256", problems);
            validate_fault_chaos(&receipt, problems)?;
        }
        "hydracache.imap.linearizability-receipt.v1" => {
            validate_array_checksum(&receipt, "history", "history_sha256", problems)
        }
        _ => {}
    }
    Ok(())
}

fn validate_fault_chaos(receipt: &Value, problems: &mut Vec<String>) -> Result<(), Box<dyn Error>> {
    let Some(seed) = receipt.get("seed").and_then(Value::as_u64) else {
        return Ok(());
    };
    let Some(steps) = receipt.get("chaos_steps").and_then(Value::as_u64) else {
        return Ok(());
    };
    if steps != 128 {
        problems.push("fault receipt must retain the fixed 128-step chaos campaign".into());
        return Ok(());
    }
    let report = run_seeded_chaos_campaign(
        seed,
        ChaosCampaignBounds {
            steps: 128,
            max_trace_events: 128,
        },
    )?;
    if !report.passed() {
        problems.push(format!(
            "fault receipt chaos replay violates invariants: {:?}",
            report.violations
        ));
    }
    let expected = format!("{:016x}", report.trace_fingerprint);
    if receipt
        .get("chaos_trace_fingerprint")
        .and_then(Value::as_str)
        != Some(expected.as_str())
    {
        problems.push("fault receipt chaos trace fingerprint does not replay from seed".into());
    }
    Ok(())
}

fn validate_model_receipt(
    root: &Path,
    value: Value,
    problems: &mut Vec<String>,
) -> Result<(), Box<dyn Error>> {
    let receipt: AuthorityModelReceipt = match serde_json::from_value(value) {
        Ok(receipt) => receipt,
        Err(error) => {
            problems.push(format!("invalid authority model receipt: {error}"));
            return Ok(());
        }
    };
    if receipt.model_id != AUTHORITY_MODEL_ID || receipt.model_source.path != MODEL_SOURCE {
        problems.push("authority receipt identifies a different model".into());
    }
    if receipt.model_source.sha256 != sha256_file(&root.join(MODEL_SOURCE))? {
        problems.push("authority receipt model source digest is stale or tampered".into());
    }
    let expected = AUTHORITY_INVARIANT_IDS
        .iter()
        .map(|id| (*id).to_owned())
        .collect::<BTreeSet<_>>();
    if receipt.invariant_ids.into_iter().collect::<BTreeSet<_>>() != expected {
        problems.push("authority receipt invariant set is incomplete".into());
    }
    Ok(())
}

fn validate_array_checksum(
    receipt: &Value,
    array_name: &str,
    checksum_name: &str,
    problems: &mut Vec<String>,
) {
    let Some(array) = receipt.get(array_name) else {
        return;
    };
    let bytes = serde_json::to_vec(array).expect("JSON values always encode");
    let actual = hex_digest(Sha256::digest(bytes).as_slice());
    if receipt.get(checksum_name).and_then(Value::as_str) != Some(actual.as_str()) {
        problems.push(format!(
            "receipt {checksum_name} does not cover canonical {array_name}"
        ));
    }
}

struct EvidenceArgs {
    release: String,
    receipt: Option<PathBuf>,
    receipts: Option<PathBuf>,
}

fn parse_args(args: &[String]) -> Result<EvidenceArgs, Box<dyn Error>> {
    let mut release = None;
    let mut receipt = None;
    let mut receipts = None;
    let mut index = 0;
    while index < args.len() {
        let name = args[index].as_str();
        index += 1;
        let value = args
            .get(index)
            .ok_or_else(|| format!("{name} requires a value"))?;
        match name {
            "--release" => release = Some(value.clone()),
            "--receipt" => receipt = Some(PathBuf::from(value)),
            "--receipts" => receipts = Some(PathBuf::from(value)),
            other => return Err(format!("unknown evidence-check argument: {other}").into()),
        }
        index += 1;
    }
    if receipt.is_some() && receipts.is_some() {
        return Err("use either --receipt or --receipts, not both".into());
    }
    let release = release.ok_or("imap-foundation-evidence-check requires --release 0.75")?;
    Ok(EvidenceArgs {
        release,
        receipt,
        receipts,
    })
}

fn repository_root() -> Result<PathBuf, Box<dyn Error>> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()?;
    if !output.status.success() {
        return Err("current directory is not inside a git repository".into());
    }
    Ok(PathBuf::from(String::from_utf8(output.stdout)?.trim()))
}
