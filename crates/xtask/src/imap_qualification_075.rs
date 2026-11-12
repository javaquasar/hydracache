//! Provisional comparison contracts and exact-source release admission.
//! Structural success never promotes local/model evidence into production evidence.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::io::Read;
use std::path::{Component, Path};
use std::process::Command;

const DIR: &str = "docs/testing/imap/0.75";
pub const MAX_ARTIFACT_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProductIdentity {
    pub id: String,
    pub version: String,
    pub source_sha: String,
    pub artifact_sha256: String,
    pub dependency_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Semantics {
    pub backups: u32,
    pub persistence: String,
    pub acknowledgement: String,
    pub tls: bool,
    pub key_codec: String,
    pub ttl: String,
    pub previous_value: bool,
    pub retry: String,
    pub listeners: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ComparisonCell {
    pub id: String,
    pub cohort: String,
    pub operation: String,
    pub members: u32,
    pub disposition: String,
    pub reason: String,
    pub hydra: Semantics,
    pub hazelcast: Semantics,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComparisonContract {
    pub release: String,
    pub contract_version: u32,
    pub status: String,
    pub production_enabled: bool,
    pub claims_enabled: bool,
    pub minimum_pairs: u32,
    pub minimum_goodput_advantage_percent: u32,
    pub maximum_p99_regression_percent: u32,
    pub require_balanced_order: bool,
    pub retain_negative_results: bool,
    pub products: Vec<ProductIdentity>,
    pub cells: Vec<ComparisonCell>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReleasePolicy {
    pub release: String,
    pub contract_version: u32,
    pub predecessor_tag: String,
    pub required_evidence: Vec<String>,
    pub maximum_receipts: usize,
    pub maximum_nested_artifacts: usize,
    pub require_zero_skips: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactRef {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProductionReceipt {
    pub release: String,
    pub kind: String,
    pub scope: String,
    pub source_sha: String,
    pub tooling_sha: String,
    pub predecessor_sha: String,
    pub schema_sha256: String,
    pub sdk_sha256: String,
    pub config_sha256: String,
    pub producer_command: String,
    pub coverage: Vec<String>,
    pub passed: bool,
    pub complete: bool,
    pub fixture: bool,
    pub total: u64,
    pub executed: u64,
    pub failed: u64,
    pub skipped: u64,
    pub artifacts: Vec<ArtifactRef>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseManifest {
    pub release: String,
    pub contract_version: u32,
    pub candidate_sha: String,
    pub tooling_sha: String,
    pub predecessor_sha: String,
    pub predecessor_tag: String,
    pub schema_sha256: String,
    pub sdk_sha256: String,
    pub config_sha256: String,
    pub comparison_sha256: String,
    pub receipts: Vec<ArtifactRef>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdmissionReport {
    pub release: &'static str,
    pub scope: &'static str,
    pub admitted: bool,
    pub blockers: Vec<String>,
}

pub fn comparison_run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let options = options(&args, "--require-ready")?;
    if options.manifest.is_some() {
        return Err("comparison check does not accept --manifest".into());
    }
    let root = crate::doc_check::find_repo_root()?;
    let blockers = check_comparison_at_root(&root)?;
    if options.require && !blockers.is_empty() {
        return Err(format!("comparison not ready: {blockers:?}").into());
    }
    println!("imap-comparison-check 0.75: valid provisional contract; ready={}, numerical claims disabled", blockers.is_empty());
    Ok(())
}

pub fn release_run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let options = options(&args, "--require-ship")?;
    let root = crate::doc_check::find_repo_root()?;
    let report = check_release_at_root(&root, options.manifest.as_deref())?;
    println!("{}", serde_json::to_string(&report)?);
    if options.require && !report.admitted {
        return Err(
            "0.75 release admission is blocked; foundation receipts cannot authorize shipping"
                .into(),
        );
    }
    Ok(())
}

struct Options {
    require: bool,
    manifest: Option<String>,
}
fn options(args: &[String], flag: &str) -> Result<Options, Box<dyn Error>> {
    let mut result = Options {
        require: false,
        manifest: None,
    };
    let mut release = None;
    let mut index = 0;
    while index < args.len() {
        let name = args[index].as_str();
        index += 1;
        if name == flag {
            if result.require {
                return Err("duplicate admission flag".into());
            }
            result.require = true;
            continue;
        }
        let value = args.get(index).ok_or("option requires a value")?.clone();
        index += 1;
        match name {
            "--release" if release.is_none() => release = Some(value),
            "--manifest" if result.manifest.is_none() => result.manifest = Some(value),
            _ => return Err(format!("unknown or duplicate option {name}").into()),
        }
    }
    if release.as_deref() != Some("0.75") {
        return Err("requires --release 0.75".into());
    }
    Ok(result)
}

pub fn check_comparison_at_root(root: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let contract: ComparisonContract = toml::from_str(std::str::from_utf8(&read_relative(
        root,
        &format!("{DIR}/hazelcast-comparison-contract.toml"),
        MAX_ARTIFACT_BYTES,
    )?)?)?;
    let mut blockers = validate_comparison(&contract)?;
    blockers.push(
        "qualified 0.75 real-cluster producer is unavailable; S2 execution is not admitted".into(),
    );
    Ok(blockers)
}

pub fn validate_comparison(contract: &ComparisonContract) -> Result<Vec<String>, Box<dyn Error>> {
    if contract.release != "0.75"
        || contract.contract_version != 1
        || contract.status != "provisional"
        || contract.production_enabled
        || contract.claims_enabled
        || contract.minimum_pairs < 5
        || contract.minimum_goodput_advantage_percent < 10
        || contract.maximum_p99_regression_percent > 5
        || !contract.require_balanced_order
        || !contract.retain_negative_results
        || contract.products.len() != 2
        || contract.cells.is_empty()
        || contract.cells.len() > 32
    {
        return Err("invalid provisional comparison policy or weakened thresholds".into());
    }
    let mut blockers = Vec::new();
    let mut products = BTreeSet::new();
    for product in &contract.products {
        if !matches!(product.id.as_str(), "hydra" | "hazelcast")
            || !products.insert(&product.id)
            || product.version.trim().is_empty()
        {
            return Err("comparison must contain exactly one identity per product".into());
        }
        for (label, value, size) in [
            ("source", &product.source_sha, 40),
            ("artifact", &product.artifact_sha256, 64),
            ("dependency", &product.dependency_sha256, 64),
        ] {
            if value == "UNRESOLVED" {
                blockers.push(format!("{} {label} identity is unresolved", product.id));
            } else if !hex(value, size) {
                return Err(format!("invalid {} {label} digest", product.id).into());
            }
        }
    }
    let mut ids = BTreeSet::new();
    let mut cohorts = BTreeSet::new();
    for cell in &contract.cells {
        if cell.id.trim().is_empty()
            || cell.id.len() > 128
            || !ids.insert(&cell.id)
            || !matches!(cell.cohort.as_str(), "S1" | "S2")
            || !matches!(
                cell.operation.as_str(),
                "get" | "put" | "delete" | "conditional_replace" | "get_all" | "put_all_detailed"
            )
            || cell.members != if cell.cohort == "S1" { 1 } else { 3 }
        {
            return Err("invalid or duplicate comparison cell".into());
        }
        for semantics in [&cell.hydra, &cell.hazelcast] {
            if semantics.backups >= cell.members
                || !matches!(semantics.persistence.as_str(), "memory" | "durable")
                || !matches!(
                    semantics.acknowledgement.as_str(),
                    "owner" | "owner_and_backup" | "durable_flush"
                )
                || semantics.key_codec != "canonical_bytes"
                || !matches!(semantics.ttl.as_str(), "eternal" | "expire_after")
                || semantics.retry != "same_retained_identity"
            {
                return Err("invalid comparison semantics or impossible replica profile".into());
            }
            if (semantics.backups > 0 && semantics.acknowledgement == "owner")
                || (semantics.persistence == "durable"
                    && semantics.acknowledgement != "durable_flush")
            {
                return Err("comparison acknowledgement weakens requested semantics".into());
            }
            if cell.cohort == "S2"
                && (semantics.backups < 1 || semantics.acknowledgement == "owner")
            {
                return Err("S2 requires a proved backup; three independent local maps are not a cluster cohort".into());
            }
        }
        match cell.disposition.as_str() {
            "comparable" if cell.hydra == cell.hazelcast && cell.reason.is_empty() => { cohorts.insert(cell.cohort.as_str()); }
            "not_comparable" if !cell.reason.trim().is_empty() => blockers.push(format!("{} not comparable: {}", cell.id, cell.reason)),
            _ => return Err("mismatched backup/persistence/TLS/TTL/return/retry/listener semantics require an explicit not-comparable disposition".into()),
        }
    }
    if cohorts != BTreeSet::from(["S1", "S2"]) {
        return Err("comparison requires comparable single-member and three-member cohorts".into());
    }
    Ok(blockers)
}

pub fn check_release_at_root(
    root: &Path,
    manifest_path: Option<&str>,
) -> Result<AdmissionReport, Box<dyn Error>> {
    let policy: ReleasePolicy = toml::from_str(std::str::from_utf8(&read_relative(
        root,
        &format!("{DIR}/release-admission-contract.toml"),
        MAX_ARTIFACT_BYTES,
    )?)?)?;
    validate_release_policy(&policy)?;
    let mut blockers = check_comparison_at_root(root)?;
    blockers.push("production receipt verifier adapters are unavailable; structural manifests are not executed production evidence".into());
    let status: serde_json::Value = serde_json::from_slice(&read_relative(
        root,
        &format!("{DIR}/status.json"),
        MAX_ARTIFACT_BYTES,
    )?)?;
    if status["release"] != "0.75" || status["contract_version"] != 1 {
        return Err("release status has an unsupported identity/version".into());
    }
    if status["production_capability"] != "enabled_qualified"
        || status["w0_disposition"] != "closed"
    {
        blockers.push("production capability is disabled or W0 remains open".into());
    }
    if let Some(path) = manifest_path {
        let manifest: ReleaseManifest =
            serde_json::from_slice(&read_relative(root, path, MAX_ARTIFACT_BYTES)?)?;
        validate_release_manifest(root, &policy, &manifest)?;
        if git_output(root, &["rev-parse", "HEAD"]).as_deref()
            != Some(manifest.candidate_sha.as_str())
        {
            blockers.push("candidate is not this exact HEAD".into());
        }
        if git_output(root, &["status", "--porcelain"]).as_deref() != Some("") {
            blockers.push("workspace is dirty or source status is unavailable".into());
        }
        let tag = format!("refs/tags/{}^{{commit}}", policy.predecessor_tag);
        if git_output(root, &["rev-parse", &tag]).as_deref()
            != Some(manifest.predecessor_sha.as_str())
        {
            blockers.push("published predecessor tag/commit is absent or mismatched".into());
        }
    } else {
        blockers.push("exact-candidate production evidence manifest is absent".into());
    }
    Ok(AdmissionReport {
        release: "0.75",
        scope: "release_admission",
        admitted: blockers.is_empty(),
        blockers,
    })
}

pub fn validate_release_policy(policy: &ReleasePolicy) -> Result<(), Box<dyn Error>> {
    let required = BTreeSet::from([
        "compatibility",
        "real_cluster",
        "security",
        "sdk_interop",
        "non_regression_074",
        "long_run",
        "migration",
        "supply_chain",
    ]);
    let actual = policy
        .required_evidence
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if policy.release != "0.75"
        || policy.contract_version != 1
        || policy.predecessor_tag != "v0.74.0"
        || actual != required
        || actual.len() != policy.required_evidence.len()
        || !policy.require_zero_skips
        || !(8..=32).contains(&policy.maximum_receipts)
        || !(1..=32).contains(&policy.maximum_nested_artifacts)
    {
        return Err("invalid release policy or missing mandatory production evidence".into());
    }
    Ok(())
}

pub fn validate_release_manifest(
    root: &Path,
    policy: &ReleasePolicy,
    manifest: &ReleaseManifest,
) -> Result<(), Box<dyn Error>> {
    validate_release_policy(policy)?;
    if manifest.release != "0.75"
        || manifest.contract_version != 1
        || manifest.predecessor_tag != policy.predecessor_tag
        || [
            &manifest.candidate_sha,
            &manifest.tooling_sha,
            &manifest.predecessor_sha,
        ]
        .iter()
        .any(|sha| !hex(sha, 40))
        || [
            &manifest.schema_sha256,
            &manifest.sdk_sha256,
            &manifest.config_sha256,
            &manifest.comparison_sha256,
        ]
        .iter()
        .any(|digest| !hex(digest, 64))
        || manifest.receipts.len() > policy.maximum_receipts
    {
        return Err("invalid exact-source release manifest".into());
    }
    let comparison = read_relative(
        root,
        &format!("{DIR}/hazelcast-comparison-contract.toml"),
        MAX_ARTIFACT_BYTES,
    )?;
    if digest(&comparison) != manifest.comparison_sha256 {
        return Err("comparison contract digest mismatch".into());
    }
    let mut kinds = BTreeSet::new();
    let mut paths = BTreeSet::new();
    for reference in &manifest.receipts {
        if !paths.insert(reference.path.clone()) {
            return Err("duplicate production receipt path".into());
        }
        let bytes = verify_artifact(root, reference)?;
        let receipt: ProductionReceipt = serde_json::from_slice(&bytes)?;
        if !kinds.insert(receipt.kind.clone())
            || !policy.required_evidence.contains(&receipt.kind)
            || receipt.release != "0.75"
            || receipt.scope != "production"
            || !receipt.passed
            || !receipt.complete
            || receipt.fixture
            || receipt.source_sha != manifest.candidate_sha
            || receipt.tooling_sha != manifest.tooling_sha
            || receipt.predecessor_sha != manifest.predecessor_sha
            || receipt.schema_sha256 != manifest.schema_sha256
            || receipt.sdk_sha256 != manifest.sdk_sha256
            || receipt.config_sha256 != manifest.config_sha256
            || receipt.producer_command != expected_producer(&receipt.kind)
            || receipt.coverage.iter().cloned().collect::<BTreeSet<_>>()
                != required_coverage(&receipt.kind).into_iter().collect()
            || receipt.coverage.len() != receipt.coverage.iter().collect::<BTreeSet<_>>().len()
            || receipt.total == 0
            || receipt.executed != receipt.total
            || receipt.failed != 0
            || receipt.skipped != 0
            || receipt.artifacts.is_empty()
            || receipt.artifacts.len() > policy.maximum_nested_artifacts
        {
            return Err(format!(
                "invalid, incomplete, model-only or mixed-source {} receipt",
                receipt.kind
            )
            .into());
        }
        for artifact in &receipt.artifacts {
            if !paths.insert(artifact.path.clone()) {
                return Err("duplicate nested evidence artifact".into());
            }
            verify_artifact(root, artifact)?;
        }
    }
    if kinds != policy.required_evidence.iter().cloned().collect() {
        return Err("production receipt set is incomplete".into());
    }
    Ok(())
}

pub fn expected_producer(kind: &str) -> String {
    format!("imap075-qualified-{kind}")
}
pub fn required_coverage(kind: &str) -> Vec<String> {
    let labels: Vec<&str> = match kind {
        "compatibility" => crate::imap_contract::REQUIRED_OPERATIONS.to_vec(),
        "sdk_interop" => crate::imap_contract::REQUIRED_OPERATIONS
            .into_iter()
            .chain(["rust_consumer", "java_17_consumer", "java_21_consumer"])
            .collect(),
        "real_cluster" => vec![
            "three_members",
            "routing",
            "required_backup_ack",
            "promotion",
            "rejoin",
            "rebalance",
            "bulk",
            "listener",
            "namespace_reclaim",
        ],
        "security" => vec![
            "member_auth",
            "tenant_substitution",
            "stale_epoch",
            "replay",
            "false_ack",
            "trust_rotation",
            "audit_redaction",
        ],
        "non_regression_074" => vec![
            "native",
            "hc1",
            "hc2",
            "resp",
            "batch",
            "expiry",
            "listeners",
            "memory",
        ],
        "long_run" => vec!["six_hour", "24_hour", "cleanup", "zero_lost_acknowledged"],
        "migration" => vec!["codec", "cold_repopulation", "cutover", "rollback"],
        "supply_chain" => vec!["sbom", "lockfile", "licenses", "provenance"],
        _ => Vec::new(),
    };
    labels.into_iter().map(str::to_owned).collect()
}
pub fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
pub fn hex(value: &str, size: usize) -> bool {
    value.len() == size
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn verify_artifact(root: &Path, reference: &ArtifactRef) -> Result<Vec<u8>, Box<dyn Error>> {
    if !hex(&reference.sha256, 64) {
        return Err("invalid artifact digest".into());
    }
    let bytes = read_relative(root, &reference.path, MAX_ARTIFACT_BYTES)?;
    if digest(&bytes) != reference.sha256 {
        return Err(format!("artifact digest mismatch: {}", reference.path).into());
    }
    Ok(bytes)
}
pub fn read_relative(
    root: &Path,
    relative: &str,
    maximum: usize,
) -> Result<Vec<u8>, Box<dyn Error>> {
    if maximum == 0 || maximum > MAX_ARTIFACT_BYTES {
        return Err("artifact byte limit is outside its supported bound".into());
    }
    let path = Path::new(relative);
    if relative.is_empty()
        || relative.contains('\\')
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err("artifact path must stay within its workspace".into());
    }
    let base = root.canonicalize()?;
    let mut resolved = base.clone();
    for component in path.components() {
        resolved.push(component);
        if fs::symlink_metadata(&resolved)?.file_type().is_symlink() {
            return Err("symlinked evidence is refused".into());
        }
    }
    if !resolved.canonicalize()?.starts_with(&base)
        || !resolved.is_file()
        || fs::metadata(&resolved)?.len() > maximum as u64
    {
        return Err("artifact is outside the workspace or exceeds its byte bound".into());
    }
    let mut bytes = Vec::new();
    fs::File::open(resolved)?
        .take(maximum as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > maximum {
        return Err("artifact grew beyond its byte bound".into());
    }
    Ok(bytes)
}
fn git_output(root: &Path, args: &[&str]) -> Option<String> {
    let result = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .ok()?;
    result
        .status
        .success()
        .then(|| String::from_utf8_lossy(&result.stdout).trim().to_owned())
}
