use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::Write as _;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use syn::visit::Visit;

use crate::doc_check;
use crate::evidence_run::{self, EvidenceOutcome, EvidenceReceipt};
use crate::gated_tests::{self, GateEntry};
use crate::quarantine;

const RELEASES_PATH: &str = "docs/plans/releases.toml";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceManifest {
    pub schema_version: u32,
    pub release: String,
    pub plan: String,
    #[serde(default)]
    pub work_item: Vec<EvidenceWorkItem>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceWorkItem {
    pub id: String,
    pub required_sources: Vec<String>,
    pub required_tests: Vec<RequiredTest>,
    pub required_artifacts: Vec<String>,
    pub fast_gate_ids: Vec<String>,
    pub gated_gate_ids: Vec<String>,
    pub ship_required: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RequiredTest {
    pub source: String,
    pub function: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceStage {
    Planned,
    Implemented,
    FastGreen,
    GatedGreen,
    ShipReady,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkItemReport {
    pub id: String,
    pub stage: EvidenceStage,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseEvidenceReport {
    pub schema_version: u32,
    pub release: String,
    pub source_commit: String,
    pub current_worktree_dirty: bool,
    pub receipts_supplied: bool,
    pub counts: BTreeMap<String, usize>,
    pub reasons: Vec<String>,
    pub work_items: Vec<WorkItemReport>,
}

#[derive(Debug, Clone)]
struct ReleaseDefinition {
    version: String,
    plan: String,
    work_items: Vec<String>,
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let options = Options::parse(args)?;
    if options.emit_template {
        let definition = load_release_definition(&options.root, &options.release)?;
        let manifest = template_manifest(&options.root, &definition)?;
        let path = manifest_path(&options.root, &options.release);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, toml::to_string_pretty(&manifest)?)?;
        println!("release-evidence: wrote template to {}", path.display());
        return Ok(());
    }

    let report = build_report(
        &options.root,
        &options.release,
        options.receipts_dir.as_deref(),
    )?;
    write_report(&options.root, &options.release, &report)?;
    println!(
        "release-evidence: planned={} implemented={} fast-green={} gated-green={} ship-ready={}",
        count(&report, EvidenceStage::Planned),
        count(&report, EvidenceStage::Implemented),
        count(&report, EvidenceStage::FastGreen),
        count(&report, EvidenceStage::GatedGreen),
        count(&report, EvidenceStage::ShipReady),
    );
    for item in &report.work_items {
        if item.stage != EvidenceStage::ShipReady {
            println!(
                "release-evidence: {} {:?}: {}",
                item.id,
                item.stage,
                item.reasons.join("; ")
            );
        }
    }
    if options.require_ship
        && (report.current_worktree_dirty
            || !report.reasons.is_empty()
            || report
                .work_items
                .iter()
                .any(|item| item.stage != EvidenceStage::ShipReady))
    {
        return Err("release-evidence: --require-ship rejected non-green evidence".into());
    }
    Ok(())
}

pub fn build_report(
    root: &Path,
    release: &str,
    receipts_dir: Option<&Path>,
) -> Result<ReleaseEvidenceReport, Box<dyn Error>> {
    let definition = load_release_definition(root, release)?;
    let manifest = load_manifest(root, release)?;
    let gates = gated_tests::load_registry(root)?;
    validate_manifest(root, &definition, &manifest, &gates)?;

    let (source_commit, current_worktree_dirty) = git_identity(root)?;
    let mut global_reasons = Vec::new();
    if current_worktree_dirty {
        global_reasons.push("current worktree is dirty".to_owned());
    }
    let receipts = load_receipts(root, receipts_dir, &mut global_reasons)?;
    let quarantine_report = quarantine::check_at(root, release, time::OffsetDateTime::now_utc())?;
    if !quarantine_report.problems.is_empty() {
        return Err(quarantine_report.problems.join("; ").into());
    }
    if quarantine_report
        .active
        .iter()
        .any(|entry| entry.ship_mandatory)
    {
        global_reasons.push("an active quarantine covers a ship-mandatory gate".to_owned());
    }
    let active_quarantines: BTreeSet<_> = quarantine_report
        .active
        .iter()
        .map(|entry| entry.gate_id.as_str())
        .collect();
    let gates_by_id: BTreeMap<_, _> = gates
        .gate
        .iter()
        .map(|gate| (gate.id.as_str(), gate))
        .collect();

    let mut work_items = Vec::new();
    for item in &manifest.work_item {
        let mut reasons = Vec::new();
        let implemented = implementation_resolves(root, item, &mut reasons);
        let mut stage = if implemented {
            EvidenceStage::Implemented
        } else {
            EvidenceStage::Planned
        };

        if item.id == "W32" && !git_is_ancestor(root, "v0.63.0", "HEAD") {
            reasons.push("v0.63.0 is not an ancestor of the candidate commit".to_owned());
        }

        let quarantined = item
            .fast_gate_ids
            .iter()
            .chain(&item.gated_gate_ids)
            .filter(|id| active_quarantines.contains(id.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        if !quarantined.is_empty() {
            reasons.push(format!("active quarantine: {}", quarantined.join(", ")));
        }

        if implemented {
            if item.fast_gate_ids.is_empty() {
                reasons.push("no fast gate receipt contract is registered".to_owned());
            } else if all_gates_green(
                root,
                release,
                &source_commit,
                &item.fast_gate_ids,
                &gates_by_id,
                &receipts,
                &mut reasons,
            ) {
                stage = EvidenceStage::FastGreen;
                if all_gates_green(
                    root,
                    release,
                    &source_commit,
                    &item.gated_gate_ids,
                    &gates_by_id,
                    &receipts,
                    &mut reasons,
                ) {
                    stage = EvidenceStage::GatedGreen;
                    if quarantined.is_empty()
                        && !current_worktree_dirty
                        && !(item.id == "W32" && !git_is_ancestor(root, "v0.63.0", "HEAD"))
                    {
                        stage = EvidenceStage::ShipReady;
                    }
                }
            }
        }
        work_items.push(WorkItemReport {
            id: item.id.clone(),
            stage,
            reasons,
        });
    }

    let mut counts = BTreeMap::new();
    for stage in [
        EvidenceStage::Planned,
        EvidenceStage::Implemented,
        EvidenceStage::FastGreen,
        EvidenceStage::GatedGreen,
        EvidenceStage::ShipReady,
    ] {
        counts.insert(
            stage_name(stage).to_owned(),
            work_items.iter().filter(|row| row.stage == stage).count(),
        );
    }
    Ok(ReleaseEvidenceReport {
        schema_version: 1,
        release: definition.version,
        source_commit,
        current_worktree_dirty,
        receipts_supplied: receipts_dir.is_some(),
        counts,
        reasons: global_reasons,
        work_items,
    })
}

pub fn parse_manifest_text(text: &str) -> Result<EvidenceManifest, Box<dyn Error>> {
    toml::from_str(text).map_err(|error| error.into())
}

pub fn receipt_problems(
    root: &Path,
    release: &str,
    source_commit: &str,
    gate: &GateEntry,
    receipt: &EvidenceReceipt,
) -> Vec<String> {
    let mut problems = Vec::new();
    if receipt.schema_version != 1 {
        problems.push("unsupported receipt schema".to_owned());
    }
    if normalize_release(&receipt.release) != normalize_release(release) {
        problems.push("wrong release".to_owned());
    }
    if receipt.gate_id != gate.id {
        problems.push("wrong gate id".to_owned());
    }
    if receipt.source_commit != source_commit {
        problems.push("wrong source commit".to_owned());
    }
    if receipt.dirty_worktree {
        problems.push("receipt was produced from a dirty worktree".to_owned());
    }
    match evidence_run::expected_digests(root, gate) {
        Ok(expected) => {
            if receipt.command_digest != expected.command {
                problems.push("stale command digest".to_owned());
            }
            if receipt.registry_digest != expected.registry {
                problems.push("stale registry digest".to_owned());
            }
            if receipt.input_digest != expected.input {
                problems.push("stale input digest".to_owned());
            }
        }
        Err(error) => problems.push(format!("cannot derive expected digests: {error}")),
    }
    if receipt.outcome != EvidenceOutcome::Pass {
        problems.push(format!("receipt outcome is {:?}", receipt.outcome));
    }
    if receipt.normalized_result.outcome != receipt.outcome
        || receipt.normalized_result.exit_code != receipt.exit_code
    {
        problems.push("normalized result does not match receipt outcome".to_owned());
    }
    if receipt.normalized_result.stdout_sha256 != sha256(receipt.stdout.as_bytes())
        || receipt.normalized_result.stderr_sha256 != sha256(receipt.stderr.as_bytes())
    {
        problems.push("captured output hash mismatch".to_owned());
    }
    if !receipt.missing_artifacts.is_empty() {
        problems.push("receipt reports missing artifacts".to_owned());
    }
    let receipt_artifacts: BTreeMap<_, _> = receipt
        .artifacts
        .iter()
        .map(|artifact| (artifact.path.as_str(), artifact))
        .collect();
    for expected in &gate.artifacts {
        let Some(recorded) = receipt_artifacts.get(expected.as_str()) else {
            problems.push(format!("missing artifact receipt for {expected}"));
            continue;
        };
        match safe_repo_path(root, expected) {
            Ok(path) => match fs::read(path) {
                Ok(bytes)
                    if sha256(&bytes) == recorded.sha256
                        && bytes.len() as u64 == recorded.bytes => {}
                Ok(_) => problems.push(format!("artifact hash mismatch for {expected}")),
                Err(_) => problems.push(format!("artifact is missing: {expected}")),
            },
            Err(error) => problems.push(error.to_string()),
        }
    }
    problems
}

fn all_gates_green(
    root: &Path,
    release: &str,
    source_commit: &str,
    gate_ids: &[String],
    gates: &BTreeMap<&str, &GateEntry>,
    receipts: &BTreeMap<String, Vec<EvidenceReceipt>>,
    reasons: &mut Vec<String>,
) -> bool {
    let mut green = true;
    for gate_id in gate_ids {
        let Some(gate) = gates.get(gate_id.as_str()) else {
            reasons.push(format!("unknown gate id {gate_id}"));
            green = false;
            continue;
        };
        let candidates = receipts.get(gate_id).map(Vec::as_slice).unwrap_or_default();
        if candidates.is_empty() {
            reasons.push(format!("missing receipt for {gate_id}"));
            green = false;
            continue;
        }
        let mut candidate_reasons = Vec::new();
        let valid = candidates.iter().any(|receipt| {
            let problems = receipt_problems(root, release, source_commit, gate, receipt);
            if problems.is_empty() {
                true
            } else {
                candidate_reasons.extend(problems);
                false
            }
        });
        if !valid {
            candidate_reasons.sort();
            candidate_reasons.dedup();
            reasons.push(format!(
                "no valid receipt for {gate_id}: {}",
                candidate_reasons.join(", ")
            ));
            green = false;
        }
    }
    green
}

fn implementation_resolves(
    root: &Path,
    item: &EvidenceWorkItem,
    reasons: &mut Vec<String>,
) -> bool {
    if item.required_sources.is_empty() || item.required_tests.is_empty() {
        reasons.push("required source/test contract is incomplete".to_owned());
        return false;
    }
    let mut resolves = true;
    for source in &item.required_sources {
        match safe_repo_path(root, source) {
            Ok(path) if path.exists() => {}
            Ok(_) => {
                reasons.push(format!("missing required source {source}"));
                resolves = false;
            }
            Err(error) => {
                reasons.push(error.to_string());
                resolves = false;
            }
        }
    }
    for test in &item.required_tests {
        match function_exists(root, test) {
            Ok(true) => {}
            Ok(false) => {
                reasons.push(format!("missing test {}::{}", test.source, test.function));
                resolves = false;
            }
            Err(error) => {
                reasons.push(error.to_string());
                resolves = false;
            }
        }
    }
    for artifact in &item.required_artifacts {
        match safe_repo_path(root, artifact) {
            Ok(path) if path.exists() => {}
            Ok(_) => {
                reasons.push(format!("missing required artifact {artifact}"));
                resolves = false;
            }
            Err(error) => {
                reasons.push(error.to_string());
                resolves = false;
            }
        }
    }
    resolves
}

fn validate_manifest(
    root: &Path,
    definition: &ReleaseDefinition,
    manifest: &EvidenceManifest,
    gates: &gated_tests::GatedTestRegistry,
) -> Result<(), Box<dyn Error>> {
    let mut problems = Vec::new();
    if manifest.schema_version != 1 {
        problems.push("evidence manifest schema_version must be 1".to_owned());
    }
    if normalize_release(&manifest.release) != definition.version {
        problems.push("evidence manifest release mismatch".to_owned());
    }
    if manifest.plan.replace('\\', "/") != definition.plan.replace('\\', "/") {
        problems.push("evidence manifest plan mismatch".to_owned());
    }
    let ids: Vec<_> = manifest
        .work_item
        .iter()
        .map(|item| item.id.clone())
        .collect();
    if ids != definition.work_items {
        problems
            .push("evidence work items must match releases.toml exactly and in order".to_owned());
    }
    let unique: BTreeSet<_> = ids.iter().collect();
    if unique.len() != ids.len() {
        problems.push("duplicate evidence work item id".to_owned());
    }
    let plan = fs::read_to_string(root.join(&definition.plan))?;
    let gate_ids: BTreeSet<_> = gates.gate.iter().map(|gate| gate.id.as_str()).collect();
    for item in &manifest.work_item {
        if !plan_has_work_item(&plan, &item.id) {
            problems.push(format!("{} has no heading in the release plan", item.id));
        }
        for gate_id in item.fast_gate_ids.iter().chain(&item.gated_gate_ids) {
            if !gate_ids.contains(gate_id.as_str()) {
                problems.push(format!("{} references unknown gate {gate_id}", item.id));
            }
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("; ").into())
    }
}

fn load_release_definition(
    root: &Path,
    release: &str,
) -> Result<ReleaseDefinition, Box<dyn Error>> {
    let value: toml::Value = toml::from_str(&fs::read_to_string(root.join(RELEASES_PATH))?)?;
    let releases = value
        .get("release")
        .and_then(toml::Value::as_array)
        .ok_or("releases.toml has no [[release]] rows")?;
    let wanted = normalize_release(release);
    let row = releases
        .iter()
        .find(|row| row.get("version").and_then(toml::Value::as_str) == Some(wanted.as_str()))
        .ok_or_else(|| format!("release {wanted} is absent from releases.toml"))?;
    let plan = row
        .get("file")
        .and_then(toml::Value::as_str)
        .ok_or("release is missing file")?
        .to_owned();
    let work_items = row
        .get("work_items")
        .and_then(toml::Value::as_array)
        .ok_or("release is missing work_items")?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .ok_or("work_items must contain strings")
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ReleaseDefinition {
        version: wanted,
        plan,
        work_items,
    })
}

fn load_manifest(root: &Path, release: &str) -> Result<EvidenceManifest, Box<dyn Error>> {
    let path = manifest_path(root, release);
    parse_manifest_text(
        &fs::read_to_string(&path)
            .map_err(|error| format!("reading {}: {error}", path.display()))?,
    )
}

fn manifest_path(root: &Path, release: &str) -> PathBuf {
    root.join(format!(
        "docs/testing/release-evidence/{}.toml",
        release.trim_end_matches(".0")
    ))
}

fn load_receipts(
    root: &Path,
    receipts_dir: Option<&Path>,
    reasons: &mut Vec<String>,
) -> Result<BTreeMap<String, Vec<EvidenceReceipt>>, Box<dyn Error>> {
    let Some(receipts_dir) = receipts_dir else {
        return Ok(BTreeMap::new());
    };
    let path = safe_target_path(root, receipts_dir)?;
    if !path.is_dir() {
        reasons.push(format!("receipts directory is missing: {}", path.display()));
        return Ok(BTreeMap::new());
    }
    let mut receipts = BTreeMap::<String, Vec<EvidenceReceipt>>::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if entry.path().extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        match serde_json::from_slice::<EvidenceReceipt>(&fs::read(entry.path())?) {
            Ok(receipt) => receipts
                .entry(receipt.gate_id.clone())
                .or_default()
                .push(receipt),
            Err(error) => reasons.push(format!(
                "invalid receipt {}: {error}",
                entry.path().display()
            )),
        }
    }
    Ok(receipts)
}

fn template_manifest(
    root: &Path,
    definition: &ReleaseDefinition,
) -> Result<EvidenceManifest, Box<dyn Error>> {
    #[derive(Deserialize)]
    struct CanaryRegistry {
        entries: Vec<CanaryEntry>,
    }
    #[derive(Deserialize)]
    struct CanaryEntry {
        w_item: String,
        guard: FunctionRef,
    }
    #[derive(Deserialize)]
    struct FunctionRef {
        file: String,
        function: String,
    }
    let canaries: CanaryRegistry = serde_json::from_str(&fs::read_to_string(
        root.join("docs/testing/canary-registry.json"),
    )?)?;
    let by_id: BTreeMap<_, _> = canaries
        .entries
        .into_iter()
        .map(|entry| (entry.w_item, entry.guard))
        .collect();
    let work_item = definition
        .work_items
        .iter()
        .map(|id| {
            let guard = by_id.get(id);
            EvidenceWorkItem {
                id: id.clone(),
                required_sources: guard.iter().map(|guard| guard.file.clone()).collect(),
                required_tests: guard
                    .iter()
                    .map(|guard| RequiredTest {
                        source: guard.file.clone(),
                        function: guard.function.clone(),
                    })
                    .collect(),
                required_artifacts: vec![],
                fast_gate_ids: vec![],
                gated_gate_ids: vec![],
                ship_required: true,
            }
        })
        .collect();
    Ok(EvidenceManifest {
        schema_version: 1,
        release: definition.version.clone(),
        plan: definition.plan.clone(),
        work_item,
    })
}

fn function_exists(root: &Path, test: &RequiredTest) -> Result<bool, Box<dyn Error>> {
    struct Functions<'a> {
        wanted: &'a str,
        found: bool,
    }
    impl<'ast> Visit<'ast> for Functions<'_> {
        fn visit_item_fn(&mut self, function: &'ast syn::ItemFn) {
            if function.sig.ident == self.wanted {
                self.found = true;
            }
            syn::visit::visit_item_fn(self, function);
        }
    }
    let path = safe_repo_path(root, &test.source)?;
    let syntax = syn::parse_file(&fs::read_to_string(path)?)?;
    let mut functions = Functions {
        wanted: &test.function,
        found: false,
    };
    functions.visit_file(&syntax);
    Ok(functions.found)
}

fn plan_has_work_item(plan: &str, id: &str) -> bool {
    plan.lines().any(|line| {
        let line = line.trim_start_matches('#').trim_start();
        line.starts_with(&format!("{id}.")) || line.starts_with(&format!("{id} "))
    })
}

fn safe_repo_path(root: &Path, value: &str) -> Result<PathBuf, Box<dyn Error>> {
    let path = Path::new(value);
    if value.trim().is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!("unsafe repository path: {value:?}").into());
    }
    Ok(root.join(path))
}

fn safe_target_path(root: &Path, value: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let text = value.to_string_lossy();
    let path = safe_repo_path(root, &text)?;
    if value.components().next() != Some(Component::Normal(std::ffi::OsStr::new("target"))) {
        return Err("receipts directory must be a relative path inside target".into());
    }
    Ok(path)
}

fn write_report(
    root: &Path,
    release: &str,
    report: &ReleaseEvidenceReport,
) -> Result<(), Box<dyn Error>> {
    let directory = root.join("target/release-evidence");
    fs::create_dir_all(&directory)?;
    let stem = release.trim_end_matches(".0");
    fs::write(
        directory.join(format!("{stem}.json")),
        serde_json::to_vec_pretty(report)?,
    )?;
    let mut markdown = format!(
        "# Release {} evidence\n\nCommit: `{}`\n\n| Work item | Stage | Reasons |\n|---|---|---|\n",
        report.release, report.source_commit
    );
    for item in &report.work_items {
        writeln!(
            markdown,
            "| {} | {} | {} |",
            item.id,
            stage_name(item.stage),
            item.reasons.join("; ").replace('|', "\\|")
        )?;
    }
    fs::write(directory.join(format!("{stem}.md")), markdown)?;
    Ok(())
}

fn count(report: &ReleaseEvidenceReport, stage: EvidenceStage) -> usize {
    report
        .work_items
        .iter()
        .filter(|item| item.stage == stage)
        .count()
}

fn stage_name(stage: EvidenceStage) -> &'static str {
    match stage {
        EvidenceStage::Planned => "planned",
        EvidenceStage::Implemented => "implemented",
        EvidenceStage::FastGreen => "fast-green",
        EvidenceStage::GatedGreen => "gated-green",
        EvidenceStage::ShipReady => "ship-ready",
    }
}

fn git_identity(root: &Path) -> Result<(String, bool), Box<dyn Error>> {
    let commit = command_output(root, &["rev-parse", "HEAD"])?;
    let status = command_output(root, &["status", "--porcelain", "--untracked-files=normal"])?;
    Ok((commit, !status.is_empty()))
}

fn git_is_ancestor(root: &Path, ancestor: &str, descendant: &str) -> bool {
    Command::new("git")
        .args(["merge-base", "--is-ancestor", ancestor, descendant])
        .current_dir(root)
        .status()
        .is_ok_and(|status| status.success())
}

fn command_output(root: &Path, args: &[&str]) -> Result<String, Box<dyn Error>> {
    let output = Command::new("git").args(args).current_dir(root).output()?;
    if !output.status.success() {
        return Err(format!("git {} failed", args.join(" ")).into());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut output, byte| {
            let _ = write!(output, "{byte:02x}");
            output
        })
}

fn normalize_release(release: &str) -> String {
    if release.matches('.').count() == 1 {
        format!("{release}.0")
    } else {
        release.to_owned()
    }
}

struct Options {
    root: PathBuf,
    release: String,
    receipts_dir: Option<PathBuf>,
    require_ship: bool,
    emit_template: bool,
}

impl Options {
    fn parse(args: Vec<String>) -> Result<Self, Box<dyn Error>> {
        let mut root = doc_check::find_repo_root()?;
        let mut release = None;
        let mut receipts_dir = None;
        let mut require_ship = false;
        let mut emit_template = false;
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--root" => root = PathBuf::from(args.next().ok_or("--root requires a path")?),
                "--release" => release = Some(args.next().ok_or("--release requires a value")?),
                "--receipts-dir" => {
                    receipts_dir = Some(PathBuf::from(
                        args.next().ok_or("--receipts-dir requires a path")?,
                    ))
                }
                "--require-ship" => require_ship = true,
                "--emit-template" => emit_template = true,
                other => return Err(format!("unknown release-evidence argument: {other}").into()),
            }
        }
        Ok(Self {
            root,
            release: release.ok_or("release-evidence requires --release")?,
            receipts_dir,
            require_ship,
            emit_template,
        })
    }
}
