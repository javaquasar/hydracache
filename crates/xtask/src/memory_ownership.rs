use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use syn::visit::Visit;
use syn::{Attribute, ItemMacro, ItemMod, ItemStatic, ItemStruct, ItemType, Type};

const REGISTRY_PATH: &str = "docs/testing/memory/0.71/ownership-registry.toml";
const INVENTORY_PATH: &str = "target/memory-evidence/0.71/ownership-inventory.json";
const OWNING_TYPES: [&str; 18] = [
    "Arc",
    "Weak",
    "Box",
    "Vec",
    "VecDeque",
    "HashMap",
    "BTreeMap",
    "HashSet",
    "BTreeSet",
    "Bytes",
    "BytesMut",
    "Sender",
    "Receiver",
    "Semaphore",
    "JoinHandle",
    "DashMap",
    "Slab",
    "Arena",
];
const EXTERNAL_BOUNDARY_SUFFIXES: [&str; 6] = [
    "Handle",
    "Pool",
    "Client",
    "Connection",
    "Producer",
    "Consumer",
];
const OWNER_SYMBOL_MARKERS: [&str; 39] = [
    "Cache",
    "Store",
    "State",
    "Registry",
    "Manager",
    "Pool",
    "Runtime",
    "Server",
    "Client",
    "Session",
    "Connection",
    "Cluster",
    "Raft",
    "Node",
    "Log",
    "Snapshot",
    "Index",
    "Map",
    "Queue",
    "Coordinator",
    "Tracker",
    "Membership",
    "Transport",
    "Buffer",
    "Table",
    "History",
    "Pending",
    "Inflight",
    "InFlight",
    "Watch",
    "Channel",
    "Engine",
    "Shard",
    "Partition",
    "Replica",
    "Lease",
    "Limiter",
    "Worker",
    "Task",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct OwnershipCandidate {
    pub candidate_id: String,
    pub source: String,
    pub symbol: String,
    pub candidate_kind: String,
    pub allocation_sites: Vec<String>,
    pub owning_types: Vec<String>,
    pub uncertainty: Option<String>,
}

#[derive(Debug, Serialize)]
struct Inventory<'a> {
    schema_version: u32,
    release: &'a str,
    discovery_scope: &'static str,
    source_sha: String,
    tested_sha: String,
    baseline_sha: String,
    scenario_digest: String,
    host_fingerprint: &'static str,
    toolchain: String,
    started_at: String,
    finished_at: String,
    result: &'static str,
    ship_evidence_eligible: bool,
    candidate_count: usize,
    uncertainty_count: usize,
    candidates: &'a [OwnershipCandidate],
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Registry {
    schema_version: u32,
    release: String,
    discovery_scope: String,
    #[serde(default)]
    owner: Vec<OwnerRecord>,
    #[serde(default)]
    exemption: Vec<Exemption>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OwnerRecord {
    owner_id: String,
    subsystem: String,
    owning_type: String,
    source: String,
    symbol: String,
    allocation_sites: Vec<String>,
    keying_dimension: String,
    retained_unit: String,
    creation_transition: String,
    terminal_transitions: Vec<String>,
    cleanup: String,
    count_bound: String,
    byte_bound: String,
    age_bound: String,
    overflow_behavior: String,
    snapshot_fields: Vec<String>,
    security_classification: String,
    focused_tests: Vec<String>,
    slow_evidence_ids: Vec<String>,
    disposition: String,
    disposition_reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Exemption {
    source: String,
    symbol: String,
    candidate_kind: String,
    reviewer: String,
    reason: String,
    expires_after_release: String,
    runtime_evidence: Vec<String>,
    terminal_transition_test: String,
}

#[derive(Default)]
struct AliasCollector {
    aliases: BTreeMap<String, Type>,
}

impl<'ast> Visit<'ast> for AliasCollector {
    fn visit_item_type(&mut self, item: &'ast ItemType) {
        self.aliases
            .insert(item.ident.to_string(), (*item.ty).clone());
        syn::visit::visit_item_type(self, item);
    }
}

struct CandidateCollector<'a> {
    source: &'a str,
    aliases: &'a BTreeSet<String>,
    candidates: Vec<OwnershipCandidate>,
    macro_index: usize,
}

impl<'ast> Visit<'ast> for CandidateCollector<'_> {
    fn visit_item_mod(&mut self, item: &'ast ItemMod) {
        if item.ident == "tests" || cfg_test(&item.attrs) {
            return;
        }
        syn::visit::visit_item_mod(self, item);
    }

    fn visit_item_struct(&mut self, item: &'ast ItemStruct) {
        if cfg_test(&item.attrs) {
            return;
        }
        let symbol = item.ident.to_string();
        if !owner_like_symbol(&symbol) {
            syn::visit::visit_item_struct(self, item);
            return;
        }
        let mut allocation_sites = Vec::new();
        let mut owning_types = BTreeSet::new();
        let mut external_types = BTreeSet::new();
        for (index, field) in item.fields.iter().enumerate() {
            let field_name = field
                .ident
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| format!("field_{index}"));
            let mut type_names = BTreeSet::new();
            collect_type_names(&field.ty, &mut type_names);
            let owns = type_names
                .iter()
                .any(|name| OWNING_TYPES.contains(&name.as_str()) || self.aliases.contains(name));
            if owns {
                allocation_sites.push(field_name);
                owning_types.extend(
                    type_names
                        .iter()
                        .filter(|name| {
                            OWNING_TYPES.contains(&name.as_str()) || self.aliases.contains(*name)
                        })
                        .cloned(),
                );
            } else {
                external_types.extend(
                    type_names
                        .iter()
                        .filter(|name| {
                            EXTERNAL_BOUNDARY_SUFFIXES
                                .iter()
                                .any(|suffix| name.ends_with(suffix))
                        })
                        .cloned(),
                );
            }
        }
        if !allocation_sites.is_empty() || !external_types.is_empty() {
            let uncertainty = (!external_types.is_empty()).then(|| "external_type".to_owned());
            owning_types.extend(external_types);
            self.candidates.push(candidate(
                self.source,
                &symbol,
                "struct",
                allocation_sites,
                owning_types.into_iter().collect(),
                uncertainty,
            ));
        }
        syn::visit::visit_item_struct(self, item);
    }

    fn visit_item_static(&mut self, item: &'ast ItemStatic) {
        if cfg_test(&item.attrs) {
            return;
        }
        let mut type_names = BTreeSet::new();
        collect_type_names(&item.ty, &mut type_names);
        let owning_types: Vec<_> = type_names
            .into_iter()
            .filter(|name| OWNING_TYPES.contains(&name.as_str()) || self.aliases.contains(name))
            .collect();
        if !owning_types.is_empty() {
            self.candidates.push(candidate(
                self.source,
                &item.ident.to_string(),
                "static",
                vec![item.ident.to_string()],
                owning_types,
                None,
            ));
        }
        syn::visit::visit_item_static(self, item);
    }

    fn visit_item_macro(&mut self, item: &'ast ItemMacro) {
        if cfg_test(&item.attrs) {
            return;
        }
        let tokens = item.mac.tokens.to_string();
        if OWNING_TYPES.iter().any(|name| tokens.contains(name)) {
            self.macro_index += 1;
            let path = item
                .mac
                .path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");
            let symbol = item
                .ident
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| format!("{path}#{}", self.macro_index));
            self.candidates.push(candidate(
                self.source,
                &symbol,
                "macro",
                vec![path],
                Vec::new(),
                Some("opaque_macro".to_owned()),
            ));
        }
        syn::visit::visit_item_macro(self, item);
    }
}

pub fn run_inventory(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let options = Options::parse(args)?;
    let started_at = now();
    let candidates = scan_repo(&options.root)?;
    let output = options
        .output
        .unwrap_or_else(|| options.root.join(INVENTORY_PATH));
    write_inventory(
        &options.root,
        &output,
        &options.release,
        &started_at,
        &candidates,
    )?;
    println!(
        "memory-owner-inventory: wrote {} candidates ({} uncertainties) to {}",
        candidates.len(),
        candidates
            .iter()
            .filter(|candidate| candidate.uncertainty.is_some())
            .count(),
        output.display()
    );
    Ok(())
}

pub fn run_check(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let options = Options::parse(args)?;
    let problems = check(&options.root, &options.release)?;
    if problems.is_empty() {
        println!("memory-ownership-check: OK (release {})", options.release);
        Ok(())
    } else {
        Err(format!(
            "memory-ownership-check failed:\n- {}",
            problems.join("\n- ")
        )
        .into())
    }
}

pub fn check(root: &Path, release: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let registry_text = fs::read_to_string(root.join(REGISTRY_PATH))?;
    let registry: Registry = toml::from_str(&registry_text)?;
    let candidates = scan_repo(root)?;
    let mut problems = validate_registry(&registry, &candidates, release);
    let focused_tests: BTreeSet<_> = registry
        .owner
        .iter()
        .flat_map(|owner| owner.focused_tests.iter())
        .collect();
    for test in focused_tests {
        let symbol = test.rsplit("::").next().unwrap_or(test);
        if !repo_contains_function(root, symbol)? {
            problems.push(format!(
                "focused test {test} does not resolve in the workspace"
            ));
        }
    }
    Ok(problems)
}

pub fn validate_registry_document(
    registry: &str,
    candidates: &[OwnershipCandidate],
    release: &str,
) -> Result<Vec<String>, Box<dyn Error>> {
    let registry: Registry = toml::from_str(registry)?;
    Ok(validate_registry(&registry, candidates, release))
}

fn validate_registry(
    registry: &Registry,
    candidates: &[OwnershipCandidate],
    release: &str,
) -> Vec<String> {
    let mut problems = Vec::new();
    if registry.schema_version != 1 || registry.release != release {
        problems.push(format!(
            "registry identity must be schema 1/release {release}, found schema {}/release {}",
            registry.schema_version, registry.release
        ));
    }
    if registry.discovery_scope != "publishable-workspace-production-rust-v1" {
        problems.push("registry discovery_scope is not the reviewed production scope".to_owned());
    }

    let candidate_keys: BTreeSet<_> = candidates
        .iter()
        .map(|candidate| {
            (
                candidate.source.as_str(),
                candidate.symbol.as_str(),
                candidate.candidate_kind.as_str(),
            )
        })
        .collect();
    let mut closed = BTreeSet::new();
    let mut owner_ids = BTreeSet::new();
    for owner in &registry.owner {
        if !owner_ids.insert(owner.owner_id.as_str()) {
            problems.push(format!("duplicate owner_id {}", owner.owner_id));
        }
        let matching: Vec<_> = candidates
            .iter()
            .filter(|candidate| {
                candidate.source == owner.source && candidate.symbol == owner.symbol
            })
            .collect();
        if matching.is_empty() {
            problems.push(format!(
                "stale owner {}: {}::{} no longer resolves",
                owner.owner_id, owner.source, owner.symbol
            ));
            continue;
        }
        for candidate in matching {
            closed.insert((
                candidate.source.as_str(),
                candidate.symbol.as_str(),
                candidate.candidate_kind.as_str(),
            ));
            let missing_sites: Vec<_> = candidate
                .allocation_sites
                .iter()
                .filter(|site| !owner.allocation_sites.contains(site))
                .collect();
            if !missing_sites.is_empty() {
                problems.push(format!(
                    "owner {} omits allocation sites {:?}",
                    owner.owner_id, missing_sites
                ));
            }
        }
        validate_owner(owner, &mut problems);
    }

    for exemption in &registry.exemption {
        let key = (
            exemption.source.as_str(),
            exemption.symbol.as_str(),
            exemption.candidate_kind.as_str(),
        );
        if !candidate_keys.contains(&key) {
            problems.push(format!(
                "stale exemption {}::{} ({})",
                exemption.source, exemption.symbol, exemption.candidate_kind
            ));
        } else {
            closed.insert(key);
        }
        if exemption.reviewer.trim().is_empty()
            || exemption.reason.trim().is_empty()
            || exemption.runtime_evidence.is_empty()
            || exemption.terminal_transition_test.trim().is_empty()
        {
            problems.push(format!(
                "exemption {}::{} lacks reviewed runtime closure",
                exemption.source, exemption.symbol
            ));
        }
        if !release_is_after(&exemption.expires_after_release, release) {
            problems.push(format!(
                "exemption {}::{} expired after {}",
                exemption.source, exemption.symbol, exemption.expires_after_release
            ));
        }
    }

    for candidate in candidates {
        let key = (
            candidate.source.as_str(),
            candidate.symbol.as_str(),
            candidate.candidate_kind.as_str(),
        );
        if !closed.contains(&key) {
            problems.push(format!(
                "unreviewed candidate {} {}::{} ({})",
                candidate.candidate_id,
                candidate.source,
                candidate.symbol,
                candidate
                    .uncertainty
                    .as_deref()
                    .unwrap_or("syntactic owner")
            ));
        }
    }
    problems
}

fn validate_owner(owner: &OwnerRecord, problems: &mut Vec<String>) {
    for (label, value) in [
        ("subsystem", owner.subsystem.as_str()),
        ("owning_type", owner.owning_type.as_str()),
        ("keying_dimension", owner.keying_dimension.as_str()),
        ("retained_unit", owner.retained_unit.as_str()),
        ("creation_transition", owner.creation_transition.as_str()),
        ("cleanup", owner.cleanup.as_str()),
        (
            "security_classification",
            owner.security_classification.as_str(),
        ),
        ("disposition_reason", owner.disposition_reason.as_str()),
    ] {
        if value.trim().is_empty() {
            problems.push(format!("owner {} has empty {label}", owner.owner_id));
        }
    }
    if owner.terminal_transitions.is_empty() || owner.focused_tests.is_empty() {
        problems.push(format!(
            "owner {} must cite terminal transitions and focused tests",
            owner.owner_id
        ));
    }
    let valid_disposition = matches!(
        owner.disposition.as_str(),
        "bounded" | "ephemeral" | "external_allocator" | "file_backed" | "not_applicable"
    );
    if !valid_disposition {
        problems.push(format!(
            "owner {} has invalid disposition {}",
            owner.owner_id, owner.disposition
        ));
    }
    if owner.disposition == "bounded"
        && (owner.count_bound.trim().is_empty()
            || owner.byte_bound.trim().is_empty()
            || owner.age_bound.trim().is_empty()
            || owner.overflow_behavior.trim().is_empty()
            || owner.snapshot_fields.is_empty())
    {
        problems.push(format!(
            "bounded owner {} lacks count/byte/age/overflow/snapshot closure",
            owner.owner_id
        ));
    }
    if owner.disposition == "bounded"
        && owner
            .snapshot_fields
            .iter()
            .any(|field| !field.starts_with(&format!("{}.", owner.owner_id)))
    {
        problems.push(format!(
            "bounded owner {} has a snapshot field outside its stable owner namespace",
            owner.owner_id
        ));
    }
    let _ = &owner.slow_evidence_ids;
}

fn scan_repo(root: &Path) -> Result<Vec<OwnershipCandidate>, Box<dyn Error>> {
    let mut roots = Vec::new();
    for entry in fs::read_dir(root.join("crates"))? {
        let entry = entry?;
        let crate_root = entry.path();
        let manifest = crate_root.join("Cargo.toml");
        let source = crate_root.join("src");
        if !manifest.is_file() || !source.is_dir() || !is_publishable(&manifest)? {
            continue;
        }
        roots.push(source);
    }
    scan_source_roots(root, &roots)
}

pub fn scan_source_roots(
    root: &Path,
    source_roots: &[PathBuf],
) -> Result<Vec<OwnershipCandidate>, Box<dyn Error>> {
    let mut files = Vec::new();
    for source_root in source_roots {
        collect_rust_files(source_root, &mut files)?;
    }
    files.sort();
    let mut candidates = Vec::new();
    for file in files {
        let source = file
            .strip_prefix(root)?
            .to_string_lossy()
            .replace('\\', "/");
        let syntax = syn::parse_file(&fs::read_to_string(&file)?)?;
        let mut aliases = AliasCollector::default();
        aliases.visit_file(&syntax);
        let mut owning_aliases = BTreeSet::new();
        loop {
            let before = owning_aliases.len();
            for (name, ty) in &aliases.aliases {
                let mut names = BTreeSet::new();
                collect_type_names(ty, &mut names);
                if names.iter().any(|candidate| {
                    OWNING_TYPES.contains(&candidate.as_str()) || owning_aliases.contains(candidate)
                }) {
                    owning_aliases.insert(name.clone());
                }
            }
            if owning_aliases.len() == before {
                break;
            }
        }
        let mut collector = CandidateCollector {
            source: &source,
            aliases: &owning_aliases,
            candidates: Vec::new(),
            macro_index: 0,
        };
        collector.visit_file(&syntax);
        candidates.extend(collector.candidates);
    }
    candidates.sort();
    candidates.dedup();
    Ok(candidates)
}

fn is_publishable(manifest: &Path) -> Result<bool, Box<dyn Error>> {
    let value: toml::Value = toml::from_str(&fs::read_to_string(manifest)?)?;
    let publish = value
        .get("package")
        .and_then(|package| package.get("publish"));
    Ok(!matches!(publish, Some(toml::Value::Boolean(false)))
        && !matches!(publish, Some(toml::Value::Array(values)) if values.is_empty()))
}

fn collect_rust_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(path)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_rust_files(&path, files)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some("rs") {
            files.push(path);
        }
    }
    Ok(())
}

fn repo_contains_function(root: &Path, symbol: &str) -> Result<bool, Box<dyn Error>> {
    let mut files = Vec::new();
    collect_rust_files(&root.join("crates"), &mut files)?;
    let needle = format!("fn {symbol}");
    for file in files {
        if fs::read_to_string(file)?.contains(&needle) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn collect_type_names(ty: &Type, names: &mut BTreeSet<String>) {
    match ty {
        Type::Array(value) => collect_type_names(&value.elem, names),
        Type::BareFn(value) => {
            for input in &value.inputs {
                collect_type_names(&input.ty, names);
            }
            if let syn::ReturnType::Type(_, ty) = &value.output {
                collect_type_names(ty, names);
            }
        }
        Type::Group(value) => collect_type_names(&value.elem, names),
        Type::Paren(value) => collect_type_names(&value.elem, names),
        Type::Path(value) => {
            for segment in &value.path.segments {
                names.insert(segment.ident.to_string());
                if let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments {
                    for argument in &arguments.args {
                        if let syn::GenericArgument::Type(ty) = argument {
                            collect_type_names(ty, names);
                        }
                    }
                }
            }
        }
        Type::Ptr(value) => collect_type_names(&value.elem, names),
        Type::Reference(value) => collect_type_names(&value.elem, names),
        Type::Slice(value) => collect_type_names(&value.elem, names),
        Type::Tuple(value) => {
            for element in &value.elems {
                collect_type_names(element, names);
            }
        }
        _ => {}
    }
}

fn candidate(
    source: &str,
    symbol: &str,
    candidate_kind: &str,
    allocation_sites: Vec<String>,
    owning_types: Vec<String>,
    uncertainty: Option<String>,
) -> OwnershipCandidate {
    let identity = format!("{source}\0{symbol}\0{candidate_kind}");
    let digest = Sha256::digest(identity.as_bytes());
    OwnershipCandidate {
        candidate_id: format!("owner-candidate-{}", hex(&digest[..8])),
        source: source.to_owned(),
        symbol: symbol.to_owned(),
        candidate_kind: candidate_kind.to_owned(),
        allocation_sites,
        owning_types,
        uncertainty,
    }
}

fn owner_like_symbol(symbol: &str) -> bool {
    OWNER_SYMBOL_MARKERS
        .iter()
        .any(|marker| symbol.contains(marker))
}

fn cfg_test(attributes: &[Attribute]) -> bool {
    attributes.iter().any(|attribute| {
        attribute.path().is_ident("cfg")
            && attribute
                .parse_args::<syn::Ident>()
                .is_ok_and(|ident| ident == "test")
    })
}

fn write_inventory(
    root: &Path,
    path: &Path,
    release: &str,
    started_at: &str,
    candidates: &[OwnershipCandidate],
) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let source_sha = command_text(root, "git", &["rev-parse", "HEAD"])?;
    let baseline_sha = command_text(root, "git", &["rev-list", "-n", "1", "v0.70.0"])?;
    let candidate_bytes = serde_json::to_vec(candidates)?;
    let scenario_digest = format!("sha256:{}", hex(&Sha256::digest(candidate_bytes)));
    let toolchain = command_text(root, "rustc", &["--version"])?;
    let inventory = Inventory {
        schema_version: 1,
        release,
        discovery_scope: "publishable-workspace-production-rust-v1",
        source_sha: source_sha.clone(),
        tested_sha: source_sha,
        baseline_sha,
        scenario_digest,
        host_fingerprint: "not-applicable(structural-source-inventory)",
        toolchain,
        started_at: started_at.to_owned(),
        finished_at: now(),
        result: "success",
        ship_evidence_eligible: true,
        candidate_count: candidates.len(),
        uncertainty_count: candidates
            .iter()
            .filter(|candidate| candidate.uncertainty.is_some())
            .count(),
        candidates,
    };
    fs::write(path, serde_json::to_vec_pretty(&inventory)?)?;
    Ok(())
}

fn command_text(root: &Path, program: &str, args: &[&str]) -> Result<String, Box<dyn Error>> {
    let output = Command::new(program)
        .args(args)
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "{program} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn now() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "unavailable".to_owned())
}

fn release_is_after(candidate: &str, current: &str) -> bool {
    release_parts(candidate) > release_parts(current)
}

fn release_parts(release: &str) -> Vec<u64> {
    release
        .split('.')
        .map(|part| part.parse::<u64>().unwrap_or_default())
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

struct Options {
    root: PathBuf,
    release: String,
    output: Option<PathBuf>,
}

impl Options {
    fn parse(args: Vec<String>) -> Result<Self, Box<dyn Error>> {
        let mut root = None;
        let mut release = None;
        let mut output = None;
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--root" => root = Some(PathBuf::from(take(&args, &mut index, "--root")?)),
                "--release" => release = Some(take(&args, &mut index, "--release")?),
                "--output" => output = Some(PathBuf::from(take(&args, &mut index, "--output")?)),
                other => {
                    return Err(format!("unsupported memory ownership argument: {other}").into())
                }
            }
            index += 1;
        }
        Ok(Self {
            root: root.unwrap_or(crate::doc_check::find_repo_root()?),
            release: release.ok_or("memory ownership command requires --release")?,
            output,
        })
    }
}

fn take(args: &[String], index: &mut usize, flag: &str) -> Result<String, Box<dyn Error>> {
    *index += 1;
    args.get(*index)
        .cloned()
        .ok_or_else(|| format!("{flag} requires a value").into())
}
