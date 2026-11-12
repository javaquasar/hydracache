use hydracache_cluster_testkit::value_plane_model_075::{
    AuthorityModelBounds, AuthorityModelExplorer, AUTHORITY_INVARIANT_IDS, AUTHORITY_MODEL_ID,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const RELEASE: &str = "0.75";
pub const RECEIPT_SCHEMA: &str = "hydracache.imap.model-receipt.v1";
pub const MODEL_SOURCE: &str = "crates/hydracache-cluster-testkit/src/value_plane_model_075.rs";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DigestRef {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReceiptBounds {
    pub max_depth: usize,
    pub max_states: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AuthorityModelReceipt {
    pub schema: String,
    pub release: String,
    pub status: String,
    pub evidence_kind: String,
    pub source_sha: String,
    pub model_id: String,
    pub model_source: DigestRef,
    pub seed: u64,
    pub bounds: ReceiptBounds,
    pub explored_states: usize,
    pub explored_transitions: usize,
    pub max_depth_reached: usize,
    pub truncated: bool,
    pub invariant_ids: Vec<String>,
    pub result: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Args {
    seed: u64,
    max_depth: usize,
    max_states: usize,
    output: Option<PathBuf>,
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let args = parse_args(&args)?;
    let root = repository_root()?;
    let receipt = execute_at_root(&root, &args)?;
    let encoded = serde_json::to_string_pretty(&receipt)? + "\n";
    if let Some(output) = args.output {
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&output, encoded)?;
        println!(
            "imap-value-plane-model {}: {} ({} states; receipt {})",
            RELEASE,
            receipt.result,
            receipt.explored_states,
            output.display()
        );
    } else {
        print!("{encoded}");
    }
    if receipt.result != "passed" {
        return Err("bounded authority model did not pass".into());
    }
    Ok(())
}

pub fn execute_for_test(
    root: &Path,
    seed: u64,
    max_depth: usize,
    max_states: usize,
) -> Result<AuthorityModelReceipt, Box<dyn Error>> {
    execute_at_root(
        root,
        &Args {
            seed,
            max_depth,
            max_states,
            output: None,
        },
    )
}

fn execute_at_root(root: &Path, args: &Args) -> Result<AuthorityModelReceipt, Box<dyn Error>> {
    let bounds = AuthorityModelBounds {
        max_depth: args.max_depth,
        max_states: args.max_states,
    };
    let report = AuthorityModelExplorer::new(bounds)?.explore();
    let source = root.join(MODEL_SOURCE);
    let source_sha256 = sha256_file(&source)?;
    let source_sha = git_head(root)?;
    Ok(AuthorityModelReceipt {
        schema: RECEIPT_SCHEMA.into(),
        release: RELEASE.into(),
        status: "provisional".into(),
        evidence_kind: "authority_model".into(),
        source_sha,
        model_id: AUTHORITY_MODEL_ID.into(),
        model_source: DigestRef {
            path: MODEL_SOURCE.into(),
            sha256: source_sha256,
        },
        seed: args.seed,
        bounds: ReceiptBounds {
            max_depth: args.max_depth,
            max_states: args.max_states,
        },
        explored_states: report.explored_states,
        explored_transitions: report.explored_transitions,
        max_depth_reached: report.max_depth_reached,
        truncated: report.truncated,
        invariant_ids: AUTHORITY_INVARIANT_IDS
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
        result: if report.passed() { "passed" } else { "failed" }.into(),
    })
}

fn parse_args(values: &[String]) -> Result<Args, Box<dyn Error>> {
    let mut release = None;
    let mut seed = 0x075_u64;
    let mut max_depth = AuthorityModelBounds::default().max_depth;
    let mut max_states = AuthorityModelBounds::default().max_states;
    let mut output = None;
    let mut index = 0;
    while index < values.len() {
        let name = values[index].as_str();
        index += 1;
        let value = values
            .get(index)
            .ok_or_else(|| format!("{name} requires a value"))?;
        match name {
            "--release" => release = Some(value.clone()),
            "--seed" => seed = value.parse()?,
            "--max-depth" => max_depth = value.parse()?,
            "--max-states" => max_states = value.parse()?,
            "--output" => output = Some(PathBuf::from(value)),
            other => return Err(format!("unknown imap-value-plane-model argument: {other}").into()),
        }
        index += 1;
    }
    if release.as_deref() != Some(RELEASE) {
        return Err(format!("imap-value-plane-model requires --release {RELEASE}").into());
    }
    if max_depth == 0 || max_states == 0 {
        return Err("model bounds must be finite non-zero values".into());
    }
    Ok(Args {
        seed,
        max_depth,
        max_states,
        output,
    })
}

pub fn sha256_file(path: &Path) -> Result<String, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    Ok(hex_digest(Sha256::digest(bytes).as_slice()))
}

pub fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn git_head(root: &Path) -> Result<String, Box<dyn Error>> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        return Err("git rev-parse HEAD failed".into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
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
