use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    canary_check, canary_sweep, doc_check, evidence_run, fast_suite, gated_tests, release_evidence,
};

const REGISTRY: &str = "docs/testing/management-center/0.72/claims.toml";
const TAXONOMY: &str = "docs/testing/management-center/0.72/failure-taxonomy.toml";
const SOURCE_MAP: &str = "docs/testing/management-center/0.72/source-map.toml";
const CANARIES: &str = "docs/testing/canaries/0.72-management-center.toml";
const COVERAGE: &str = "docs/testing/management-center/0.72/coverage.toml";
const COVERAGE_RATCHET: &str = "docs/testing/coverage-ratchet.toml";
const ARCHITECTURE: &str = "docs/architecture/management-center-v2.md";
const BASELINES: &str = "docs/testing/management-center/0.72/baselines.toml";

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

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ClaimReceipt {
    schema_version: u32,
    release: String,
    claim_id: String,
    work_item: String,
    source_commit: String,
    dirty_worktree: bool,
    claim_registry_sha256: String,
    implementation: Vec<String>,
    behavior_tests: Vec<String>,
    canaries: Vec<String>,
    proof_inputs: Vec<ClaimProofInput>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ClaimProofInput {
    kind: ClaimProofKind,
    id: String,
    path: String,
    bytes: u64,
    sha256: String,
}

type ReceiptFile = (String, Vec<u8>);

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum ClaimProofKind {
    FastGate,
    GatedGate,
    Canary,
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
    work_item: String,
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

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CoverageInventory {
    schema_version: u32,
    release: String,
    workspace_line_floor_percent: f64,
    module: Vec<CoverageModule>,
    #[serde(default)]
    exclusion: Vec<CoverageExclusion>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CoverageModule {
    path: String,
    work_items: Vec<String>,
    tests: Vec<String>,
    reviewed_branches: Vec<String>,
    receipt: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CoverageExclusion {
    path: String,
    owner: String,
    rationale: String,
    expires: String,
    non_ship: bool,
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let options = Options::parse(args)?;
    if options.write_receipts {
        write_claim_receipts(&options.root, &options.receipts_dir)?;
        println!("management-center-check: wrote exact-candidate claim receipts");
        return Ok(());
    }
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
    let mut problems = check_documents(
        root,
        &fs::read_to_string(root.join(REGISTRY))?,
        &fs::read_to_string(root.join(TAXONOMY))?,
        &fs::read_to_string(root.join(CANARIES))?,
        &fs::read_to_string(root.join(SOURCE_MAP))?,
        require_evidence,
    )?;
    problems.extend(check_coverage_document(
        root,
        &fs::read_to_string(root.join(COVERAGE))?,
    )?);
    problems.extend(check_architecture_document(&fs::read_to_string(
        root.join(ARCHITECTURE),
    )?));
    problems.extend(check_baseline_document(
        root,
        &fs::read_to_string(root.join(BASELINES))?,
        require_evidence,
    )?);
    Ok(problems)
}

/// Keep the human architecture decision tied to the threat, truth, budget and
/// interaction contracts that the machine-readable registries enforce.
pub fn check_architecture_document(text: &str) -> Vec<String> {
    let required = [
        "# Management Center 2.0 architecture",
        "## Scope and non-goals",
        "## Information architecture",
        "## Authority and observation state machine",
        "## Permissions and data classification",
        "## Trust boundaries and threat model",
        "## Request and aggregation sequences",
        "## Endpoint budgets",
        "## Responsive interaction specification",
        "## Baselines and evidence boundary",
        "confused-deputy fan-out and SSRF",
        "enumeration and cross-tenant disclosure",
        "stale replay and mixed epochs",
        "response amplification and denial of service",
        "stored/reflected XSS",
        "Desktop",
        "Tablet",
        "Narrow mobile",
    ];
    required
        .into_iter()
        .filter(|marker| !text.contains(marker))
        .map(|marker| format!("management architecture is missing required marker: {marker}"))
        .collect()
}

pub fn check_baseline_document(
    root: &Path,
    text: &str,
    require_evidence: bool,
) -> Result<Vec<String>, Box<dyn Error>> {
    let value: toml::Value = toml::from_str(text)?;
    let mut problems = Vec::new();
    if value
        .get("schema_version")
        .and_then(toml::Value::as_integer)
        != Some(1)
        || value.get("release").and_then(toml::Value::as_str) != Some("0.72.0")
    {
        problems.push("baseline registry must be schema 1 for release 0.72.0".to_owned());
    }
    let pre = value.get("pre_feature");
    let previous = value.get("published_previous");
    let policy = value.get("policy");
    let pre_sha = pre
        .and_then(|row| row.get("source_commit"))
        .and_then(toml::Value::as_str)
        .unwrap_or_default();
    if pre_sha.len() != 40 || !pre_sha.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        problems.push("pre-feature baseline must name an exact 40-hex source commit".to_owned());
    }
    if previous
        .and_then(|row| row.get("tag"))
        .and_then(toml::Value::as_str)
        != Some("v0.71.0")
    {
        problems.push("published previous baseline must name v0.71.0".to_owned());
    }
    for (name, row) in [("pre-feature", pre), ("published previous", previous)] {
        let measurements = row
            .and_then(|entry| entry.get("required_measurements"))
            .and_then(toml::Value::as_array);
        if measurements.is_none_or(Vec::is_empty) {
            problems.push(format!("{name} baseline has no required measurements"));
        }
        let receipt = row
            .and_then(|entry| entry.get("receipt"))
            .and_then(toml::Value::as_str)
            .unwrap_or_default();
        if !safe_relative(receipt)
            || !Path::new(receipt).starts_with("target/release-evidence/management-center/0.72")
        {
            problems.push(format!("{name} baseline has an unsafe receipt path"));
        } else if require_evidence && !root.join(receipt).is_file() {
            problems.push(format!("{name} baseline receipt is missing: {receipt}"));
        }
    }
    for flag in [
        "candidate_may_self_baseline",
        "development_branch_may_substitute_for_published_artifact",
    ] {
        if policy
            .and_then(|entry| entry.get(flag))
            .and_then(toml::Value::as_bool)
            != Some(false)
        {
            problems.push(format!("baseline policy must keep {flag}=false"));
        }
    }
    for flag in [
        "missing_baseline_blocks_ship",
        "failed_attempts_are_append_only",
    ] {
        if policy
            .and_then(|entry| entry.get(flag))
            .and_then(toml::Value::as_bool)
            != Some(true)
        {
            problems.push(format!("baseline policy must keep {flag}=true"));
        }
    }
    Ok(problems)
}

/// Validate the management diff-coverage inventory and preserve the workspace
/// floor. Exact numeric coverage is retained as a candidate receipt; this
/// structural gate prevents a changed module or branch review from vanishing.
pub fn check_coverage_document(
    root: &Path,
    coverage_text: &str,
) -> Result<Vec<String>, Box<dyn Error>> {
    let inventory: CoverageInventory = toml::from_str(coverage_text)?;
    let ratchet: toml::Value = toml::from_str(&fs::read_to_string(root.join(COVERAGE_RATCHET))?)?;
    let configured_floor = ratchet
        .get("configured_floor_percent")
        .and_then(toml::Value::as_float)
        .unwrap_or_default();
    let minimum_floor = ratchet
        .get("minimum_floor_percent")
        .and_then(toml::Value::as_float)
        .unwrap_or_default();
    let mut problems = Vec::new();
    if inventory.schema_version != 1 || inventory.release != "0.72.0" {
        problems.push("coverage inventory must be schema 1 for release 0.72.0".to_owned());
    }
    if inventory.workspace_line_floor_percent < 88.0
        || configured_floor < inventory.workspace_line_floor_percent
        || minimum_floor < 88.0
    {
        problems.push(format!(
            "coverage floor regressed: inventory={}, configured={}, minimum={}",
            inventory.workspace_line_floor_percent, configured_floor, minimum_floor
        ));
    }

    let required_modules = BTreeSet::from([
        "crates/hydracache-observability/src/management.rs",
        "crates/hydracache-observability/src/management_health.rs",
        "crates/hydracache-server/src/management_aggregation.rs",
        "crates/hydracache-server/src/management_history.rs",
        "crates/hydracache-server/src/management_http.rs",
        "crates/hydracache-server/src/management_operations.rs",
        "crates/hydracache-server/src/management_security.rs",
        "crates/hydracache-server/src/management_topology.rs",
        "crates/hydracache-server/src/admin_http.rs",
        "crates/hydracache-server/src/generated_console_assets.rs",
        "console/src/api.ts",
        "console/src/app.tsx",
        "console/src/capabilities.ts",
        "console/src/controller.ts",
        "console/src/history.ts",
        "console/src/router.ts",
        "console/src/state.ts",
        "console/src/components/primitives.tsx",
        "console/src/components/error-boundary.tsx",
        "console/src/components/shell.tsx",
        "console/src/pages/visibility.ts",
        "console/scripts/check-static.mjs",
        "console/scripts/check-supply-chain.mjs",
        "console/scripts/embed-dist.mjs",
        "console/scripts/package-management-center.mjs",
        "console/vite.config.ts",
        "fuzz/src/lib.rs",
        "crates/xtask/src/management_center.rs",
    ]);
    let mut registered = BTreeSet::new();
    for module in &inventory.module {
        if !registered.insert(module.path.as_str()) {
            problems.push(format!("duplicate coverage module {}", module.path));
        }
        if !safe_relative(&module.path) || !root.join(&module.path).is_file() {
            problems.push(format!(
                "coverage module is missing or unsafe: {}",
                module.path
            ));
        }
        if module.work_items.is_empty()
            || module.tests.is_empty()
            || module.reviewed_branches.is_empty()
        {
            problems.push(format!(
                "coverage module {} lacks work item, test, or reviewed branch class",
                module.path
            ));
        }
        for test in &module.tests {
            validate_reference(root, test, true, &module.path, &mut problems);
        }
        if !safe_relative(&module.receipt)
            || !Path::new(&module.receipt).starts_with("target/test-evidence/0.72")
        {
            problems.push(format!(
                "coverage module {} has invalid exact-candidate receipt {}",
                module.path, module.receipt
            ));
        }
    }
    for missing in required_modules.difference(&registered) {
        problems.push(format!(
            "changed management module lacks coverage row: {missing}"
        ));
    }
    for extra in registered.difference(&required_modules) {
        problems.push(format!(
            "coverage row is not a reviewed management module: {extra}"
        ));
    }
    for exclusion in &inventory.exclusion {
        if !safe_relative(&exclusion.path)
            || exclusion.owner.trim().is_empty()
            || exclusion.rationale.trim().is_empty()
            || exclusion.expires.trim().is_empty()
            || !exclusion.non_ship
        {
            problems.push(format!(
                "coverage exclusion {} needs safe path, owner, rationale, expiry and non_ship=true",
                exclusion.path
            ));
        }
    }
    Ok(problems)
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
    let proof_context = if require_evidence {
        Some(ClaimProofContext::load(root)?)
    } else {
        None
    };

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
    let mut claim_receipts_by_work_item = BTreeMap::<&str, BTreeSet<&str>>::new();
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
        if claim.receipts.is_empty() {
            problems.push(format!("{} has no exact-candidate receipt path", claim.id));
        }
        for receipt in &claim.receipts {
            if !safe_relative(receipt)
                || !Path::new(receipt).starts_with("target/release-evidence/management-center/0.72")
                || Path::new(receipt)
                    .extension()
                    .and_then(|value| value.to_str())
                    != Some("json")
            {
                problems.push(format!("{} has unsafe receipt path {receipt}", claim.id));
            } else if let Some(context) = &proof_context {
                problems.extend(context.validate_claim_receipt(
                    root,
                    registry_text,
                    claim,
                    receipt,
                ));
            }
            claim_receipts_by_work_item
                .entry(claim.work_item.as_str())
                .or_default()
                .insert(receipt.as_str());
        }
        if matches!(claim.status, ClaimStatus::Planned | ClaimStatus::Deferred) {
            problems.push(format!(
                "{} is still {:?} while registered as implemented UI/API",
                claim.id, claim.status
            ));
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
            if !canary.starts_with(&format!("MC72-{}-", row.work_item)) {
                problems.push(format!(
                    "taxonomy {} canary {canary} is not owned by {}",
                    row.id, row.work_item
                ));
            }
        }
        let owned_receipts = claim_receipts_by_work_item
            .get(row.work_item.as_str())
            .cloned()
            .unwrap_or_default();
        for receipt in &row.receipts {
            if !safe_relative(receipt)
                || !Path::new(receipt).starts_with("target/release-evidence/management-center/0.72")
                || !owned_receipts.contains(receipt.as_str())
            {
                problems.push(format!(
                    "taxonomy {} receipt {receipt} is not a validated {} claim receipt",
                    row.id, row.work_item
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

struct ClaimProofContext {
    source_commit: String,
    dirty_worktree: bool,
    manifest: release_evidence::EvidenceManifest,
    fast: fast_suite::FastSuiteRegistry,
    gated: gated_tests::GatedTestRegistry,
    canaries: canary_check::CanaryRegistry,
}

impl ClaimProofContext {
    fn load(root: &Path) -> Result<Self, Box<dyn Error>> {
        let (source_commit, dirty_worktree) = git_identity(root)?;
        Ok(Self {
            source_commit,
            dirty_worktree,
            manifest: release_evidence::parse_manifest_text(&fs::read_to_string(
                root.join("docs/testing/release-evidence/0.72.toml"),
            )?)?,
            fast: fast_suite::load_registry(root)?,
            gated: gated_tests::load_registry(root)?,
            canaries: canary_check::load_registry_for_release(root, "0.72")?,
        })
    }

    fn validate_claim_receipt(
        &self,
        root: &Path,
        registry_text: &str,
        claim: &Claim,
        receipt_path: &str,
    ) -> Vec<String> {
        let prefix = format!("{} receipt {receipt_path}", claim.id);
        let bytes = match fs::read(root.join(receipt_path)) {
            Ok(bytes) => bytes,
            Err(error) => return vec![format!("{prefix} is missing: {error}")],
        };
        let receipt: ClaimReceipt = match serde_json::from_slice(&bytes) {
            Ok(receipt) => receipt,
            Err(error) => return vec![format!("{prefix} is invalid JSON: {error}")],
        };
        let mut problems = Vec::new();
        if receipt.schema_version != 1
            || receipt.release != "0.72.0"
            || receipt.claim_id != claim.id
            || receipt.work_item != claim.work_item
        {
            problems.push(format!(
                "{prefix} has wrong schema, release, claim or work item"
            ));
        }
        if receipt.source_commit != self.source_commit {
            problems.push(format!("{prefix} has wrong source commit"));
        }
        if receipt.dirty_worktree || self.dirty_worktree {
            problems.push(format!("{prefix} is not bound to a clean candidate"));
        }
        if receipt.claim_registry_sha256 != sha256(registry_text.as_bytes()) {
            problems.push(format!("{prefix} has stale claim-registry digest"));
        }
        if receipt.implementation != claim.implementation
            || receipt.behavior_tests != claim.behavior_tests
            || receipt.canaries != claim.canaries
        {
            problems.push(format!("{prefix} has stale claim/test/canary mapping"));
        }

        let Some(item) = self
            .manifest
            .work_item
            .iter()
            .find(|item| item.id == claim.work_item)
        else {
            problems.push(format!("{prefix} references an unregistered work item"));
            return problems;
        };
        let Some(canary) = self
            .canaries
            .entries
            .iter()
            .find(|entry| entry.w_item == claim.work_item)
        else {
            problems.push(format!("{prefix} has no release canary"));
            return problems;
        };

        let mut expected = BTreeSet::new();
        expected.extend(
            item.fast_gate_ids
                .iter()
                .map(|id| (ClaimProofKind::FastGate, id.clone())),
        );
        expected.extend(
            item.gated_gate_ids
                .iter()
                .map(|id| (ClaimProofKind::GatedGate, id.clone())),
        );
        expected.insert((ClaimProofKind::Canary, canary.defect_id.clone()));
        let mut observed = BTreeSet::new();
        for input in &receipt.proof_inputs {
            if !observed.insert((input.kind, input.id.clone())) {
                problems.push(format!("{prefix} repeats proof input {}", input.id));
                continue;
            }
            problems.extend(self.validate_proof_input(root, input, &prefix));
        }
        if observed != expected {
            problems.push(format!(
                "{prefix} proof set differs from the exact fast/gated/canary contract"
            ));
        }
        problems
    }

    fn validate_proof_input(
        &self,
        root: &Path,
        input: &ClaimProofInput,
        prefix: &str,
    ) -> Vec<String> {
        if !safe_relative(&input.path)
            || !Path::new(&input.path).starts_with("target/release-evidence")
        {
            return vec![format!("{prefix} has unsafe proof input {}", input.path)];
        }
        let bytes = match fs::read(root.join(&input.path)) {
            Ok(bytes) => bytes,
            Err(error) => {
                return vec![format!(
                    "{prefix} proof input {} is missing: {error}",
                    input.path
                )]
            }
        };
        if input.bytes != bytes.len() as u64 || input.sha256 != sha256(&bytes) {
            return vec![format!("{prefix} proof input {} hash mismatch", input.path)];
        }
        match input.kind {
            ClaimProofKind::FastGate => {
                let Some(suite) = self.fast.suite.iter().find(|suite| suite.id == input.id) else {
                    return vec![format!(
                        "{prefix} references unknown fast gate {}",
                        input.id
                    )];
                };
                match serde_json::from_slice::<evidence_run::EvidenceReceipt>(&bytes) {
                    Ok(receipt) => release_evidence::fast_receipt_problems(
                        root,
                        "0.72",
                        &self.source_commit,
                        suite,
                        &receipt,
                    )
                    .into_iter()
                    .map(|problem| format!("{prefix} fast proof {}: {problem}", input.id))
                    .collect(),
                    Err(error) => vec![format!(
                        "{prefix} fast proof {} is invalid: {error}",
                        input.id
                    )],
                }
            }
            ClaimProofKind::GatedGate => {
                let Some(gate) = self.gated.gate.iter().find(|gate| gate.id == input.id) else {
                    return vec![format!(
                        "{prefix} references unknown gated gate {}",
                        input.id
                    )];
                };
                match serde_json::from_slice::<evidence_run::EvidenceReceipt>(&bytes) {
                    Ok(receipt) => release_evidence::receipt_problems(
                        root,
                        "0.72",
                        &self.source_commit,
                        gate,
                        &receipt,
                    )
                    .into_iter()
                    .map(|problem| format!("{prefix} gated proof {}: {problem}", input.id))
                    .collect(),
                    Err(error) => vec![format!(
                        "{prefix} gated proof {} is invalid: {error}",
                        input.id
                    )],
                }
            }
            ClaimProofKind::Canary => {
                let Some(entry) = self
                    .canaries
                    .entries
                    .iter()
                    .find(|entry| entry.defect_id == input.id)
                else {
                    return vec![format!("{prefix} references unknown canary {}", input.id)];
                };
                match serde_json::from_slice::<canary_sweep::CanaryReceipt>(&bytes) {
                    Ok(receipt) => canary_sweep::receipt_problems(
                        root,
                        &self.canaries,
                        entry,
                        &receipt,
                        &self.source_commit,
                    )
                    .into_iter()
                    .map(|problem| format!("{prefix} canary proof {}: {problem}", input.id))
                    .collect(),
                    Err(error) => vec![format!(
                        "{prefix} canary proof {} is invalid: {error}",
                        input.id
                    )],
                }
            }
        }
    }
}

fn write_claim_receipts(root: &Path, receipts_dir: &Path) -> Result<(), Box<dyn Error>> {
    let registry_text = fs::read_to_string(root.join(REGISTRY))?;
    let registry: ClaimRegistry = toml::from_str(&registry_text)?;
    let context = ClaimProofContext::load(root)?;
    if context.dirty_worktree {
        return Err("claim receipts can only be written from a clean candidate worktree".into());
    }
    let evidence_files = receipt_files(root, receipts_dir)?;
    let canary_files = receipt_files(root, Path::new(canary_sweep::RECEIPTS_DIR))?;
    for claim in &registry.claim {
        let item = context
            .manifest
            .work_item
            .iter()
            .find(|item| item.id == claim.work_item)
            .ok_or_else(|| format!("missing release work item {}", claim.work_item))?;
        let canary = context
            .canaries
            .entries
            .iter()
            .find(|entry| entry.w_item == claim.work_item)
            .ok_or_else(|| format!("missing canary for {}", claim.work_item))?;
        let mut inputs = Vec::new();
        for (kind, id, files) in item
            .fast_gate_ids
            .iter()
            .map(|id| (ClaimProofKind::FastGate, id, &evidence_files))
            .chain(
                item.gated_gate_ids
                    .iter()
                    .map(|id| (ClaimProofKind::GatedGate, id, &evidence_files)),
            )
            .chain(std::iter::once((
                ClaimProofKind::Canary,
                &canary.defect_id,
                &canary_files,
            )))
        {
            let input = files
                .iter()
                .filter_map(|(path, bytes)| proof_input(kind, id, path, bytes).ok())
                .find(|input| {
                    context
                        .validate_proof_input(root, input, &claim.id)
                        .is_empty()
                })
                .ok_or_else(|| format!("no valid exact-candidate {kind:?} proof for {id}"))?;
            inputs.push(input);
        }
        let receipt = ClaimReceipt {
            schema_version: 1,
            release: registry.release.clone(),
            claim_id: claim.id.clone(),
            work_item: claim.work_item.clone(),
            source_commit: context.source_commit.clone(),
            dirty_worktree: false,
            claim_registry_sha256: sha256(registry_text.as_bytes()),
            implementation: claim.implementation.clone(),
            behavior_tests: claim.behavior_tests.clone(),
            canaries: claim.canaries.clone(),
            proof_inputs: inputs,
        };
        for relative in &claim.receipts {
            let destination = root.join(relative);
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            let temporary = destination.with_extension("json.tmp");
            fs::write(&temporary, serde_json::to_vec_pretty(&receipt)?)?;
            fs::rename(temporary, destination)?;
        }
    }
    Ok(())
}

fn receipt_files(root: &Path, directory: &Path) -> Result<Vec<ReceiptFile>, Box<dyn Error>> {
    if !safe_relative(directory.to_string_lossy().as_ref())
        || !directory.starts_with("target/release-evidence")
    {
        return Err("receipt directory must be below target/release-evidence".into());
    }
    let absolute = root.join(directory);
    if !absolute.is_dir() {
        return Err(format!("receipt directory is missing: {}", absolute.display()).into());
    }
    let mut files = Vec::new();
    for entry in fs::read_dir(absolute)? {
        let path = entry?.path();
        if path.extension().and_then(|value| value.to_str()) == Some("json") {
            let relative = path
                .strip_prefix(root)?
                .to_string_lossy()
                .replace('\\', "/");
            files.push((relative, fs::read(path)?));
        }
    }
    Ok(files)
}

fn proof_input(
    kind: ClaimProofKind,
    id: &str,
    path: &str,
    bytes: &[u8],
) -> Result<ClaimProofInput, Box<dyn Error>> {
    let actual_id = match kind {
        ClaimProofKind::FastGate | ClaimProofKind::GatedGate => {
            serde_json::from_slice::<evidence_run::EvidenceReceipt>(bytes)?.gate_id
        }
        ClaimProofKind::Canary => {
            serde_json::from_slice::<canary_sweep::CanaryReceipt>(bytes)?.defect_id
        }
    };
    if actual_id != id {
        return Err("proof identity mismatch".into());
    }
    Ok(ClaimProofInput {
        kind,
        id: id.to_owned(),
        path: path.to_owned(),
        bytes: bytes.len() as u64,
        sha256: sha256(bytes),
    })
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn git_identity(root: &Path) -> Result<(String, bool), Box<dyn Error>> {
    let commit = command_output(root, "git", &["rev-parse", "HEAD"])
        .ok_or("unable to resolve source commit")?;
    let status = command_output(
        root,
        "git",
        &["status", "--porcelain", "--untracked-files=normal"],
    )
    .ok_or("unable to inspect worktree status")?;
    Ok((commit, !status.trim().is_empty()))
}

fn command_output(root: &Path, program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program)
        .current_dir(root)
        .args(args)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
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
    write_receipts: bool,
    receipts_dir: PathBuf,
}

impl Options {
    fn parse(args: Vec<String>) -> Result<Self, Box<dyn Error>> {
        let mut root = None;
        let mut release = "0.72".to_owned();
        let mut require_evidence = false;
        let mut write_receipts = false;
        let mut receipts_dir = PathBuf::from("target/release-evidence/receipts");
        let mut it = args.into_iter();
        while let Some(arg) = it.next() {
            match arg.as_str() {
                "--root" => root = Some(PathBuf::from(it.next().ok_or("--root requires a path")?)),
                "--release" => release = it.next().ok_or("--release requires a value")?,
                "--require-evidence" | "--require-ship" => require_evidence = true,
                "--write-receipts" => write_receipts = true,
                "--receipts-dir" => {
                    receipts_dir = PathBuf::from(it.next().ok_or("--receipts-dir requires a path")?)
                }
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
            write_receipts,
            receipts_dir,
        })
    }
}
