use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::doc_check;

const REGISTRY: &str = "docs/testing/management-center/0.72/claims.toml";
const TAXONOMY: &str = "docs/testing/management-center/0.72/failure-taxonomy.toml";
const SOURCE_MAP: &str = "docs/testing/management-center/0.72/source-map.toml";
const CANARIES: &str = "docs/testing/canaries/0.72-management-center.toml";

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ClaimRegistry {
    schema_version: u32,
    release: String,
    claim: Vec<Claim>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Claim {
    id: String,
    work_item: String,
    claim: String,
    implementation: Vec<String>,
    behavior_tests: Vec<String>,
    canaries: Vec<String>,
    #[serde(default)]
    routes: Vec<String>,
    #[serde(default)]
    compat_artifacts: Vec<String>,
    #[serde(default)]
    bounds: Vec<String>,
    #[serde(default)]
    privacy_rules: Vec<String>,
    receipts: Vec<String>,
    status: ClaimStatus,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ClaimStatus {
    Planned,
    Implemented,
    Evidenced,
    Deferred,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FailureTaxonomy {
    schema_version: u32,
    release: String,
    row: Vec<TaxonomyRow>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TaxonomyRow {
    id: String,
    status: TaxonomyStatus,
    source_tests: Vec<String>,
    management_tests: Vec<String>,
    canaries: Vec<String>,
    receipts: Vec<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum TaxonomyStatus {
    Covered,
    Partial,
    Missing,
}

#[derive(Debug, Deserialize)]
struct CanaryDocument {
    release: String,
    canary: Vec<CanaryRow>,
}

#[derive(Debug, Deserialize)]
struct CanaryRow {
    id: String,
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let options = Options::parse(args)?;
    let problems = check(&options.root, options.require_evidence)?;
    if problems.is_empty() {
        println!("management-center-check: OK");
        Ok(())
    } else {
        for problem in &problems {
            eprintln!("management-center-check: {problem}");
        }
        Err(format!(
            "management-center-check found {} problem(s)",
            problems.len()
        )
        .into())
    }
}

pub fn check(root: &Path, require_evidence: bool) -> Result<Vec<String>, Box<dyn Error>> {
    check_documents(
        root,
        &fs::read_to_string(root.join(REGISTRY))?,
        &fs::read_to_string(root.join(TAXONOMY))?,
        &fs::read_to_string(root.join(CANARIES))?,
        &fs::read_to_string(root.join(SOURCE_MAP))?,
        require_evidence,
    )
}

/// Validate explicit registry documents against the repository. This seam lets
/// meta-tests prove that a tampered document turns the admission decision red.
pub fn check_documents(
    root: &Path,
    registry_text: &str,
    taxonomy_text: &str,
    canary_text: &str,
    source_map_text: &str,
    require_evidence: bool,
) -> Result<Vec<String>, Box<dyn Error>> {
    let registry: ClaimRegistry = toml::from_str(registry_text)?;
    let taxonomy: FailureTaxonomy = toml::from_str(taxonomy_text)?;
    let canaries: CanaryDocument = toml::from_str(canary_text)?;
    let source_map: toml::Value = toml::from_str(source_map_text)?;
    let mut problems = Vec::new();

    if registry.schema_version != 1 || registry.release != "0.72.0" {
        problems.push("claim registry must be schema 1 for release 0.72.0".to_owned());
    }
    if taxonomy.schema_version != 1 || taxonomy.release != registry.release {
        problems.push("failure taxonomy version/release mismatch".to_owned());
    }
    if canaries.release != registry.release {
        problems.push("canary release does not match claim registry".to_owned());
    }
    let canary_ids = canaries
        .canary
        .iter()
        .map(|row| row.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut claim_ids = BTreeSet::new();
    let mut claimed_routes = BTreeSet::new();
    for claim in &registry.claim {
        if !claim_ids.insert(claim.id.as_str()) {
            problems.push(format!("duplicate claim id {}", claim.id));
        }
        if !matches!(
            claim.work_item.as_str(),
            "W0" | "W1"
                | "W2"
                | "W3"
                | "W4"
                | "W5"
                | "W6"
                | "W7"
                | "W8"
                | "W9"
                | "W10"
                | "W11"
                | "W12"
                | "W13"
                | "W14"
        ) {
            problems.push(format!(
                "{} has unknown work item {}",
                claim.id, claim.work_item
            ));
        }
        if claim.claim.trim().is_empty()
            || claim.implementation.is_empty()
            || claim.behavior_tests.is_empty()
            || claim.canaries.is_empty()
        {
            problems.push(format!(
                "{} lacks claim, implementation, test, or canary",
                claim.id
            ));
        }
        for reference in &claim.implementation {
            validate_reference(root, reference, false, &claim.id, &mut problems);
        }
        for reference in &claim.behavior_tests {
            validate_reference(root, reference, true, &claim.id, &mut problems);
        }
        for canary in &claim.canaries {
            if !canary_ids.contains(canary.as_str()) {
                problems.push(format!("{} references unknown canary {canary}", claim.id));
            }
        }
        claimed_routes.extend(claim.routes.iter().map(String::as_str));
        for receipt in &claim.receipts {
            if !safe_relative(receipt) {
                problems.push(format!("{} has unsafe receipt path {receipt}", claim.id));
            } else if claim.status == ClaimStatus::Evidenced && !root.join(receipt).is_file() {
                problems.push(format!(
                    "{} evidenced receipt is missing: {receipt}",
                    claim.id
                ));
            }
        }
        if matches!(claim.status, ClaimStatus::Planned | ClaimStatus::Deferred) {
            problems.push(format!(
                "{} is still {:?} while registered as implemented UI/API",
                claim.id, claim.status
            ));
        }
        if require_evidence && claim.status != ClaimStatus::Evidenced {
            problems.push(format!("{} lacks exact-candidate evidence", claim.id));
        }
        if claim
            .compat_artifacts
            .iter()
            .any(|value| value.trim().is_empty())
            || claim.bounds.iter().any(|value| value.trim().is_empty())
            || claim
                .privacy_rules
                .iter()
                .any(|value| value.trim().is_empty())
        {
            problems.push(format!(
                "{} contains an empty compatibility/bound/privacy rule",
                claim.id
            ));
        }
    }
    for route in source_routes(&source_map) {
        if !claimed_routes.contains(route.as_str()) {
            problems.push(format!("source-map route has no claim: {route}"));
        }
    }

    let required_taxonomy = BTreeSet::from([
        "reopen",
        "missing-derivative",
        "bit-rot",
        "torn-artifact",
        "enospc-write-error",
        "uncommitted-wal",
        "corrupt-authoritative-snapshot",
        "commit-before-apply",
        "deletion-stale-peer",
        "foreign-identity-disk",
        "interrupted-reconciliation",
        "concurrent-snapshot-aggregation",
        "bounded-resource-pressure",
    ]);
    let mut taxonomy_ids = BTreeSet::new();
    for row in &taxonomy.row {
        taxonomy_ids.insert(row.id.as_str());
        if row.source_tests.is_empty()
            || row.management_tests.is_empty()
            || row.canaries.is_empty()
            || row.receipts.is_empty()
        {
            problems.push(format!("taxonomy {} lacks a proof mapping", row.id));
        }
        for reference in row.source_tests.iter().chain(&row.management_tests) {
            validate_reference(root, reference, true, &row.id, &mut problems);
        }
        for canary in &row.canaries {
            if !canary_ids.contains(canary.as_str()) {
                problems.push(format!(
                    "taxonomy {} references unknown canary {canary}",
                    row.id
                ));
            }
        }
        if require_evidence && row.status != TaxonomyStatus::Covered {
            problems.push(format!("taxonomy {} is not covered", row.id));
        }
        if require_evidence {
            for receipt in &row.receipts {
                if !root.join(receipt).is_file() {
                    problems.push(format!("taxonomy {} receipt is missing: {receipt}", row.id));
                }
            }
        }
    }
    for missing in required_taxonomy.difference(&taxonomy_ids) {
        problems.push(format!("failure taxonomy row is missing: {missing}"));
    }
    if let Some(extra) = taxonomy_ids.difference(&required_taxonomy).next() {
        problems.push(format!("unknown failure taxonomy row: {extra}"));
    }
    Ok(problems)
}

fn validate_reference(
    root: &Path,
    reference: &str,
    function: bool,
    owner: &str,
    problems: &mut Vec<String>,
) {
    let (path, symbol) = reference.split_once('#').unwrap_or((reference, ""));
    if !safe_relative(path) {
        problems.push(format!("{owner} has unsafe reference {reference}"));
        return;
    }
    match fs::read_to_string(root.join(path)) {
        Ok(_source) if symbol.is_empty() && !function => {}
        Ok(source) if source.contains(symbol) => {}
        Ok(_) => problems.push(format!("{owner} symbol/test is missing: {reference}")),
        Err(error) => problems.push(format!("{owner} source is missing: {reference}: {error}")),
    }
}

fn source_routes(value: &toml::Value) -> Vec<String> {
    let mut routes = Vec::new();
    fn visit(value: &toml::Value, routes: &mut Vec<String>) {
        match value {
            toml::Value::Table(table) => {
                for (key, value) in table {
                    if matches!(key.as_str(), "route" | "routes") {
                        if let Some(route) = value.as_str() {
                            routes.push(route.to_owned());
                        }
                    }
                    visit(value, routes);
                }
            }
            toml::Value::Array(values) => values.iter().for_each(|value| visit(value, routes)),
            _ => {}
        }
    }
    visit(value, &mut routes);
    routes.sort();
    routes.dedup();
    routes
}

fn safe_relative(value: &str) -> bool {
    let path = Path::new(value);
    !value.trim().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| !matches!(component, Component::ParentDir | Component::Prefix(_)))
}

struct Options {
    root: PathBuf,
    require_evidence: bool,
}

impl Options {
    fn parse(args: Vec<String>) -> Result<Self, Box<dyn Error>> {
        let mut root = None;
        let mut release = "0.72".to_owned();
        let mut require_evidence = false;
        let mut it = args.into_iter();
        while let Some(arg) = it.next() {
            match arg.as_str() {
                "--root" => root = Some(PathBuf::from(it.next().ok_or("--root requires a path")?)),
                "--release" => release = it.next().ok_or("--release requires a value")?,
                "--require-evidence" | "--require-ship" => require_evidence = true,
                other => {
                    return Err(format!("unknown management-center-check argument: {other}").into())
                }
            }
        }
        if !matches!(release.as_str(), "0.72" | "0.72.0") {
            return Err("management-center-check currently supports only release 0.72".into());
        }
        Ok(Self {
            root: root.unwrap_or(doc_check::find_repo_root()?),
            require_evidence,
        })
    }
}
