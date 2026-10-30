//! Unsigned development inventory, reduced from actual compiler/resolver inputs.
//! Inventory validity is deliberately independent of dependency-policy success.
use crate::imap_extended_receipt_075::{artifact, Artifact};
use crate::imap_qualification_075::{digest, hex, read_relative};
use crate::imap_sbom_capture_075::{Capture, REPOSITORY, SPY};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
#[cfg(test)]
#[path = "../tests/support/imap_sbom_075.rs"]
mod tests;
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

pub const LATEST: &str = "target/imap-075/development-inventory.json";
const MAX: usize = 8 * 1024 * 1024;
const PROFILE: &str = "imap075-native-host-build-and-test-inputs-v1";
type Result<T> = std::result::Result<T, Box<dyn Error>>;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Component {
    pub id: String,
    pub ecosystem: String,
    pub name: String,
    pub version: String,
    pub source: String,
    pub source_checksum: String,
    pub upstream_source: String,
    pub upstream_origin: String,
    pub licenses: Vec<String>,
    pub license_origin: String,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub kind: String,
    pub target: String,
    pub stage: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Compilation {
    pub component: String,
    pub stage: String,
    pub features: Vec<String>,
    pub target: Value,
    pub profile: Value,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RootArtifact {
    pub component: String,
    pub artifact: Artifact,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Inventory {
    pub schema: String,
    pub profile: String,
    pub target: String,
    pub source_sha: String,
    pub cargo_lock_sha256: String,
    pub roots: Vec<RootArtifact>,
    pub components: Vec<Component>,
    pub edges: Vec<Edge>,
    pub compilations: Vec<Compilation>,
    pub tools: Vec<Value>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyState {
    Passed,
    Failed,
    Unknown,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub state: PolicyState,
    pub rust_state: PolicyState,
    pub java_state: PolicyState,
    pub scope: String,
    pub report: Artifact,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    pub schema: String,
    pub source_sha: String,
    pub source_clean: bool,
    pub production_enabled: bool,
    pub signature: String,
    pub inventory_valid: bool,
    pub capture: Artifact,
    pub inputs: Vec<Artifact>,
    pub inventory: Artifact,
    pub sbom: Artifact,
    pub producer: Artifact,
    pub dependency_policy: Policy,
}
fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value[key]
        .as_str()
        .filter(|v| v.len() <= 4096)
        .ok_or_else(|| format!("missing/bounded string: {key}").into())
}
fn array<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>> {
    value[key]
        .as_array()
        .filter(|a| a.len() <= 20000)
        .ok_or_else(|| format!("missing/bounded array: {key}").into())
}
fn read(root: &Path, path: &str) -> Result<Vec<u8>> {
    read_relative(root, path, MAX)
}
fn load(root: &Path, path: &str) -> Result<Value> {
    Ok(serde_json::from_slice(&read(root, path)?)?)
}
fn relative(root: &Path, path: &Path) -> Result<String> {
    Ok(path
        .strip_prefix(root)?
        .to_string_lossy()
        .replace('\\', "/"))
}
fn insert(components: &mut BTreeMap<String, Component>, component: Component) -> Result<()> {
    if let Some(old) = components.get(&component.id) {
        if old != &component {
            return Err("component identity has substituted source/checksum/license".into());
        }
    }
    components.insert(component.id.clone(), component);
    if components.len() > 8192 {
        return Err("component work bound".into());
    }
    Ok(())
}
fn cargo_identity(package: &Value) -> Result<String> {
    let source = package["source"].as_str().unwrap_or("workspace");
    Ok(format!(
        "cargo:{}@{}#{}",
        text(package, "name")?,
        text(package, "version")?,
        source
    ))
}
fn lock_checksums(lock: &str) -> Result<BTreeMap<(String, String, String), String>> {
    let lock: toml::Value = toml::from_str(lock)?;
    let mut result = BTreeMap::new();
    for package in lock["package"].as_array().ok_or("lock packages missing")? {
        if let Some(source) = package.get("source").and_then(toml::Value::as_str) {
            let key = (
                package["name"].as_str().ok_or("lock name")?.into(),
                package["version"].as_str().ok_or("lock version")?.into(),
                source.into(),
            );
            let checksum = package
                .get("checksum")
                .and_then(toml::Value::as_str)
                .unwrap_or("git-revision-in-source")
                .to_owned();
            if source.starts_with("registry+") && !hex(&checksum, 64) {
                return Err("registry checksum missing".into());
            }
            if result.insert(key, checksum).is_some() {
                return Err("duplicate locked identity".into());
            }
        }
    }
    Ok(result)
}
#[derive(Default)]
struct CargoContext {
    features: BTreeSet<String>,
    test: bool,
    build_script: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CargoTarget {
    pub triple: String,
    pub cfg: BTreeSet<String>,
}
impl CargoTarget {
    pub fn current() -> Result<Self> {
        fn output(args: &[&str]) -> Result<String> {
            let mut command = Command::new("rustc");
            command.args(args);
            let output = crate::imap_proof_process_075::execute(
                command,
                &[],
                crate::imap_proof_process_075::ProcessBounds {
                    stdout_bytes: 8192,
                    ..Default::default()
                },
            )?;
            Ok(String::from_utf8(output.stdout)?)
        }
        let version = output(&["-vV"])?;
        let triple = version
            .lines()
            .find_map(|l| l.strip_prefix("host: "))
            .ok_or("Rust target host missing")?
            .to_owned();
        let cfg = output(&["--print", "cfg"])?
            .lines()
            .map(str::to_owned)
            .collect();
        Ok(Self { triple, cfg })
    }
    pub fn applies(&self, predicate: &str) -> Result<bool> {
        if predicate.len() > 4096
            || self.cfg.len() > 512
            || self.triple.len() > 128
            || self.cfg.iter().any(|s| s.len() > 256)
        {
            return Err("Cargo target/cfg bound".into());
        }
        if !predicate.contains('(') {
            return Ok(predicate == self.triple);
        }
        let mut depth = 0usize;
        for byte in predicate.bytes() {
            if byte == b'(' {
                depth += 1;
                if depth > 32 {
                    return Err("Cargo cfg nesting bound".into());
                }
            } else if byte == b')' {
                depth = depth.checked_sub(1).ok_or("Cargo cfg parentheses")?;
            }
        }
        fn evaluate(meta: &syn::Meta, cfg: &BTreeSet<String>) -> Result<bool> {
            use syn::{Expr, Lit, Meta};
            match meta {
                Meta::Path(path) => {
                    Ok(cfg.contains(&path.get_ident().ok_or("Cargo cfg path")?.to_string()))
                }
                Meta::NameValue(value) => {
                    let name = value.path.get_ident().ok_or("Cargo cfg key")?;
                    let Expr::Lit(value) = &value.value else {
                        return Err("Cargo cfg literal required".into());
                    };
                    let Lit::Str(value) = &value.lit else {
                        return Err("Cargo cfg string required".into());
                    };
                    Ok(cfg.contains(&format!("{name}={:?}", value.value())))
                }
                Meta::List(list) => {
                    let values = list.parse_args_with(
                        syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated,
                    )?;
                    let name = list
                        .path
                        .get_ident()
                        .ok_or("Cargo cfg operator")?
                        .to_string();
                    match name.as_str() {
                        "cfg" if values.len() == 1 => evaluate(&values[0], cfg),
                        "not" if values.len() == 1 => Ok(!evaluate(&values[0], cfg)?),
                        "all" => {
                            let mut result = true;
                            for v in values {
                                result &= evaluate(&v, cfg)?;
                            }
                            Ok(result)
                        }
                        "any" => {
                            let mut result = false;
                            for v in values {
                                result |= evaluate(&v, cfg)?;
                            }
                            Ok(result)
                        }
                        _ => Err("Cargo cfg operator/arity unsupported".into()),
                    }
                }
            }
        }
        evaluate(&syn::parse_str(predicate)?, &self.cfg)
    }
}
fn activated_dependencies(package: &Value, context: &CargoContext) -> Result<BTreeSet<String>> {
    let definitions = package["features"]
        .as_object()
        .ok_or("Cargo feature definitions missing")?;
    let mut pending = context.features.iter().cloned().collect::<Vec<_>>();
    let mut seen = BTreeSet::new();
    let mut active = BTreeSet::new();
    let mut work = 0;
    while let Some(feature) = pending.pop() {
        work += 1;
        if work > 20000 || pending.len() > 20000 {
            return Err("Cargo feature closure work bound".into());
        }
        if !seen.insert(feature.clone()) {
            continue;
        }
        let values = definitions
            .get(&feature)
            .and_then(Value::as_array)
            .ok_or("compiler feature missing from package declaration")?;
        for value in values {
            let name = value
                .as_str()
                .filter(|s| s.len() <= 4096)
                .ok_or("Cargo feature definition string bound")?;
            if let Some(alias) = name.strip_prefix("dep:") {
                active.insert(alias.to_owned());
            } else if let Some((alias, _)) = name.split_once('/') {
                if !alias.ends_with('?') {
                    active.insert(alias.to_owned());
                }
            } else if definitions.contains_key(name) {
                pending.push(name.to_owned());
            } else {
                return Err("unknown Cargo feature closure member".into());
            }
        }
    }
    Ok(active)
}
/// Compiler instances are retained separately: features are never inferred from all root-lock packages.
pub fn cargo_projection(
    metadata: &Value,
    transcript: &[u8],
    stage: &str,
    lock: &str,
    source: &str,
) -> Result<(Vec<Component>, Vec<Edge>, Vec<Compilation>)> {
    cargo_projection_for_target(
        metadata,
        transcript,
        stage,
        lock,
        source,
        &CargoTarget::current()?,
    )
}
pub fn cargo_projection_for_target(
    metadata: &Value,
    transcript: &[u8],
    stage: &str,
    lock: &str,
    source: &str,
    target_cfg: &CargoTarget,
) -> Result<(Vec<Component>, Vec<Edge>, Vec<Compilation>)> {
    if transcript.len() > MAX || stage.len() > 128 || !hex(source, 40) {
        return Err("Cargo projection bound/source".into());
    }
    let checksums = lock_checksums(lock)?;
    let mut packages = BTreeMap::new();
    for package in array(metadata, "packages")? {
        if packages.insert(text(package, "id")?, package).is_some() {
            return Err("duplicate Cargo package ID".into());
        }
    }
    let mut compiled = BTreeSet::new();
    let mut instances = BTreeMap::new();
    let mut contexts: BTreeMap<String, CargoContext> = BTreeMap::new();
    let mut finished = false;
    for (record, line) in transcript
        .split(|b| *b == b'\n')
        .filter(|line| line.first() == Some(&b'{'))
        .enumerate()
    {
        if line.len() > 2 * 1024 * 1024 || record >= 20000 || finished {
            return Err("Cargo compiler record bound".into());
        }
        let row: Value = serde_json::from_slice(line)?;
        match row["reason"].as_str() {
            Some("compiler-artifact") => {
                let id = text(&row, "package_id")?;
                let package = packages
                    .get(id)
                    .ok_or("compiled package absent from resolved metadata")?;
                let component = cargo_identity(package)?;
                compiled.insert(id.to_owned());
                let mut features = array(&row, "features")?
                    .iter()
                    .map(|f| {
                        f.as_str()
                            .filter(|f| f.len() <= 256)
                            .map(str::to_owned)
                            .ok_or("invalid compiler feature")
                    })
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                features.sort();
                if features.windows(2).any(|w| w[0] == w[1]) {
                    return Err("duplicate compiler feature".into());
                }
                let context = contexts.entry(id.into()).or_default();
                context.features.extend(features.iter().cloned());
                context.test |= row["profile"]["test"] == true;
                context.build_script |= array(&row["target"], "kind")?
                    .iter()
                    .any(|kind| kind == "custom-build");
                let mut target = row["target"].clone();
                // Absolute source paths and temporary consumer directory names are not inventory identities.
                target
                    .as_object_mut()
                    .ok_or("compiler target missing")?
                    .remove("src_path");
                let instance = Compilation {
                    component,
                    stage: stage.into(),
                    features,
                    target,
                    profile: row["profile"].clone(),
                };
                let key = serde_json::to_string(&instance)?;
                instances.insert(key, instance);
            }
            Some("build-finished") => {
                if finished || row["success"] != true {
                    return Err("Cargo build terminal duplicate/failed".into());
                }
                finished = true;
            }
            Some("compiler-message" | "build-script-executed") => {}
            _ => return Err("unknown Cargo compiler record".into()),
        }
    }
    if !finished || compiled.is_empty() {
        return Err("incomplete compiler transcript".into());
    }
    let mut components = Vec::new();
    let mut canonical_ids = BTreeSet::new();
    for id in &compiled {
        let p = packages[id.as_str()];
        if !canonical_ids.insert(cargo_identity(p)?) {
            return Err("ambiguous compiled package/source identity".into());
        }
        let name = text(p, "name")?.to_owned();
        let version = text(p, "version")?.to_owned();
        let source_identity = p["source"].as_str().unwrap_or("workspace").to_owned();
        let checksum = if source_identity == "workspace" {
            format!("git:{source}")
        } else {
            checksums
                .get(&(name.clone(), version.clone(), source_identity.clone()))
                .ok_or("compiled source not pinned in root lock")?
                .clone()
        };
        let license = p["license"].as_str().filter(|license| !license.is_empty());
        components.push(Component {
            id: cargo_identity(p)?,
            ecosystem: "cargo".into(),
            name,
            version,
            source: source_identity,
            source_checksum: checksum,
            upstream_source: p["repository"]
                .as_str()
                .filter(|s| !s.is_empty())
                .unwrap_or("unknown")
                .into(),
            upstream_origin: if p["repository"].as_str().is_some_and(|s| !s.is_empty()) {
                "cargo-package-metadata"
            } else {
                "unknown"
            }
            .into(),
            licenses: license.into_iter().map(str::to_owned).collect(),
            license_origin: if license.is_some() {
                "cargo-package-metadata"
            } else {
                "unknown"
            }
            .into(),
        });
    }
    let mut edges = BTreeSet::new();
    let nodes = array(&metadata["resolve"], "nodes")?;
    let mut node_ids = BTreeSet::new();
    for node in nodes {
        let id = text(node, "id")?;
        if !node_ids.insert(id) {
            return Err("duplicate Cargo resolved node".into());
        }
        if !compiled.contains(id) {
            continue;
        }
        let context = contexts.get(id).ok_or("compiled context missing")?;
        let active = activated_dependencies(packages[id], context)?;
        for dep in array(node, "deps")? {
            let to = text(dep, "pkg")?;
            let destination = packages
                .get(to)
                .ok_or("resolved dependency package missing")?;
            for kind in array(dep, "dep_kinds")? {
                let kind_name = match kind["kind"].as_str() {
                    None if kind["kind"].is_null() => "normal",
                    Some("build") => "build",
                    Some("dev") => "dev",
                    _ => return Err("unknown Cargo dependency kind".into()),
                };
                let target = kind["target"].as_str().unwrap_or("all").to_owned();
                if target != "all" && !target_cfg.applies(&target)? {
                    continue;
                }
                if kind_name == "dev" && !context.test
                    || kind_name == "build" && !context.build_script
                {
                    continue;
                }
                let declarations = array(packages[id], "dependencies")?
                    .iter()
                    .filter(|d| {
                        d["name"] == destination["name"]
                            && d["rename"]
                                .as_str()
                                .or(d["name"].as_str())
                                .is_some_and(|alias| {
                                    alias.replace('-', "_") == dep["name"].as_str().unwrap_or("")
                                })
                            && d["kind"] == kind["kind"]
                            && d["target"] == kind["target"]
                    })
                    .collect::<Vec<_>>();
                if declarations.is_empty() {
                    return Err("resolved dependency has no matching declaration".into());
                }
                if declarations.iter().any(|d| !d["optional"].is_boolean()) {
                    return Err("Cargo optional declaration missing".into());
                }
                let enabled = declarations.iter().any(|d| {
                    d["optional"] == false
                        || d["optional"] == true
                            && d["rename"]
                                .as_str()
                                .or(d["name"].as_str())
                                .is_some_and(|alias| active.contains(alias))
                });
                if !enabled {
                    continue;
                }
                if !compiled.contains(to) {
                    return Err(format!(
                        "active transitive dependency missing from compiler input: {to}"
                    )
                    .into());
                }
                edges.insert(Edge {
                    from: cargo_identity(packages[id])?,
                    to: cargo_identity(packages[to])?,
                    kind: kind_name.into(),
                    target,
                    stage: stage.into(),
                });
            }
        }
    }
    if compiled.iter().any(|id| !node_ids.contains(id.as_str())) {
        return Err("compiled package missing its resolved node".into());
    }
    Ok((
        components,
        edges.into_iter().collect(),
        instances.into_values().collect(),
    ))
}

/// A Maven build must close its actual resolver journal; swallowed EventSpy errors cannot pass.
pub fn maven_journal(bytes: &[u8], nonce: &str) -> Result<Vec<Value>> {
    if bytes.len() > MAX || nonce.is_empty() || nonce.len() > 128 {
        return Err("Maven journal bound".into());
    }
    let mut rows = Vec::new();
    for line in bytes.split(|b| *b == b'\n').filter(|l| !l.is_empty()) {
        if line.len() > 16384 || rows.len() >= 20000 {
            return Err("Maven journal record/work bound".into());
        }
        rows.push(serde_json::from_slice::<Value>(line)?);
    }
    let first = rows.first().ok_or("empty Maven journal")?;
    let last = rows.last().ok_or("missing Maven terminal")?;
    if first["event"] != "init"
        || first["schema"] != "imap075-maven-resolution-v1"
        || first["nonce"] != nonce
        || last["event"] != "terminal"
        || last["nonce"] != nonce
        || last["stage"] != first["stage"]
        || last["complete"] != true
        || last["records"].as_u64() != Some((rows.len() - 1) as u64)
    {
        return Err("Maven incomplete/foreign journal".into());
    }
    let mut materials = BTreeSet::new();
    let mut tools = BTreeSet::new();
    for row in &rows[1..rows.len() - 1] {
        match row["event"].as_str() {
            Some("material") => {
                let id = text(row, "id")?;
                let path = text(row, "path")?;
                if !materials.insert((id, path))
                    || !hex(text(row, "sha256")?, 64)
                    || row["bytes"]
                        .as_u64()
                        .is_none_or(|b| b == 0 || b > 512 * 1024 * 1024)
                    || !matches!(row["location"].as_str(), Some("workspace" | "repository"))
                {
                    return Err("duplicate/invalid Maven material".into());
                }
            }
            Some("tool") => {
                if !tools.insert((text(row, "kind")?, text(row, "path")?))
                    || !hex(text(row, "sha256")?, 64)
                {
                    return Err("duplicate/invalid Maven tool".into());
                }
            }
            Some("mojo") => {
                for field in ["group", "name", "version", "goal"] {
                    text(row, field)?;
                }
            }
            _ => return Err("unexpected Maven journal record".into()),
        }
    }
    if materials.is_empty()
        || tools.is_empty()
        || last["materials"].as_u64() != Some(materials.len() as u64)
    {
        return Err("Maven input closure empty/count differs".into());
    }
    Ok(rows)
}
fn maven_id(row: &Value) -> Result<String> {
    Ok(format!("maven:{}", text(row, "id")?))
}
fn tree_edges(tree: &Value, stage: &str, known: &BTreeMap<String, Component>) -> Result<Vec<Edge>> {
    fn id(node: &Value) -> Result<String> {
        Ok(format!(
            "maven:{}:{}:{}:{}:{}",
            text(node, "groupId")?,
            text(node, "artifactId")?,
            text(node, "type")?,
            node["classifier"].as_str().unwrap_or(""),
            text(node, "version")?
        ))
    }
    let mut pending = vec![(tree, 0)];
    let mut work = 0;
    let mut edges = BTreeSet::new();
    while let Some((node, depth)) = pending.pop() {
        work += 1;
        if work > 4096 || depth > 128 {
            return Err("Maven tree work/depth bound".into());
        }
        let from = id(node)?;
        if !known.contains_key(&from) {
            return Err(format!("Maven transitive root/material missing: {from}").into());
        }
        if let Some(children) = node["children"].as_array() {
            for child in children {
                let to = id(child)?;
                if !known.contains_key(&to) {
                    return Err(format!("Maven transitive dependency missing: {to}").into());
                }
                let scope = text(child, "scope")?;
                if !matches!(
                    scope,
                    "compile" | "runtime" | "test" | "provided" | "system"
                ) {
                    return Err("Maven unknown dependency scope".into());
                }
                edges.insert(Edge {
                    from: from.clone(),
                    to,
                    kind: scope.into(),
                    target: "jvm-release-17".into(),
                    stage: stage.into(),
                });
                pending.push((child, depth + 1));
            }
        }
    }
    Ok(edges.into_iter().collect())
}
pub fn validate_inventory(inventory: &Inventory) -> Result<()> {
    if inventory.schema != "imap075-development-inventory-v1"
        || inventory.profile != PROFILE
        || !hex(&inventory.source_sha, 40)
        || !hex(&inventory.cargo_lock_sha256, 64)
        || inventory.target.is_empty()
        || inventory.target.len() > 128
        || inventory.components.is_empty()
        || inventory.components.len() > 8192
        || inventory.compilations.is_empty()
        || inventory.compilations.len() > 20000
        || inventory.edges.len() > 100000
        || inventory.tools.is_empty()
    {
        return Err("inventory identity/profile/shape differs".into());
    }
    let mut ids = BTreeSet::new();
    let mut previous = "";
    for c in &inventory.components {
        if c.id.as_str() <= previous
            || !ids.insert(c.id.as_str())
            || c.id.len() > 4096
            || c.source.is_empty()
            || c.source_checksum.is_empty()
            || c.upstream_source.is_empty()
            || !matches!(
                c.upstream_origin.as_str(),
                "cargo-package-metadata" | "direct-pom-declaration" | "unknown"
            )
            || (c.upstream_origin == "unknown") != (c.upstream_source == "unknown")
            || c.licenses.len() > 64
            || !matches!(c.ecosystem.as_str(), "cargo" | "maven")
            || !matches!(
                c.license_origin.as_str(),
                "unknown" | "cargo-package-metadata" | "direct-pom-declaration"
            )
            || (c.license_origin == "unknown") != c.licenses.is_empty()
        {
            return Err("duplicate/invalid component/license origin".into());
        }
        previous = &c.id;
    }
    if inventory.roots.len() != 5 {
        return Err("development artifact root missing".into());
    }
    let mut roots = BTreeSet::new();
    for root in &inventory.roots {
        if !ids.contains(root.component.as_str())
            || !roots.insert(&root.artifact.path)
            || !hex(&root.artifact.sha256, 64)
            || root.artifact.bytes == 0
        {
            return Err("foreign/duplicate root artifact".into());
        }
    }
    let mut edges = BTreeSet::new();
    for edge in &inventory.edges {
        if !ids.contains(edge.from.as_str())
            || !ids.contains(edge.to.as_str())
            || !edges.insert(edge)
            || !matches!(
                edge.kind.as_str(),
                "normal"
                    | "build"
                    | "dev"
                    | "compile"
                    | "runtime"
                    | "test"
                    | "provided"
                    | "system"
                    | "build-input"
            )
        {
            return Err("missing transitive/duplicate/unknown edge".into());
        }
    }
    let mut compilations = BTreeSet::new();
    for c in &inventory.compilations {
        if !ids.contains(c.component.as_str())
            || !compilations.insert(serde_json::to_string(c)?)
            || !c.target.is_object()
            || !c.profile.is_object()
            || c.features.windows(2).any(|w| w[0] >= w[1])
        {
            return Err("duplicate/foreign compiler instance/features".into());
        }
    }
    Ok(())
}
pub fn compare_inventory(expected: &Inventory, actual: &Inventory) -> Result<()> {
    validate_inventory(expected)?;
    validate_inventory(actual)?;
    if actual != expected {
        return Err("resolved closure/features/target/artifact inventory differs".into());
    }
    Ok(())
}

pub fn cyclone_dx(inventory: &Inventory) -> Result<Value> {
    validate_inventory(inventory)?;
    let components=inventory.components.iter().map(|c|{
        let mut properties=vec![json!({"name":"hydracache:ecosystem","value":c.ecosystem}),json!({"name":"hydracache:source","value":c.source}),json!({"name":"hydracache:source-checksum","value":c.source_checksum}),json!({"name":"hydracache:license-origin","value":c.license_origin})];
        properties.push(json!({"name":"hydracache:upstream-source","value":c.upstream_source}));
        properties.push(json!({"name":"hydracache:upstream-origin","value":c.upstream_origin}));
        let artifacts=inventory.roots.iter().filter(|r|r.component==c.id).map(|r|json!({"path":r.artifact.path,"sha256":r.artifact.sha256,"bytes":r.artifact.bytes})).collect::<Vec<_>>();
        if !artifacts.is_empty(){properties.push(json!({"name":"hydracache:development-artifacts","value":serde_json::to_string(&artifacts).unwrap()}));}
        let mut v=json!({"type":"library","bom-ref":c.id,"name":c.name,"version":c.version,"properties":properties});
        if !c.licenses.is_empty(){v["licenses"]=json!(c.licenses.iter().map(|name|json!({"license":{"name":name}})).collect::<Vec<_>>());}v
    }).collect::<Vec<_>>();
    let mut dependencies: BTreeMap<&str, BTreeSet<&str>> = inventory
        .components
        .iter()
        .map(|c| (c.id.as_str(), BTreeSet::new()))
        .collect();
    for edge in &inventory.edges {
        dependencies
            .get_mut(edge.from.as_str())
            .ok_or("BOM edge owner")?
            .insert(&edge.to);
    }
    let bom = json!({"bomFormat":"CycloneDX","specVersion":"1.6","version":1,"metadata":{"properties":[{"name":"hydracache:scope","value":"unsigned-development-build-input-inventory"},{"name":"hydracache:source-sha","value":inventory.source_sha},{"name":"hydracache:profile","value":inventory.profile},{"name":"hydracache:target","value":inventory.target}]},"components":components,"dependencies":dependencies.into_iter().map(|(r,ds)|json!({"ref":r,"dependsOn":ds})).collect::<Vec<_>>()});
    validate_cyclone_dx(&bom)?;
    Ok(bom)
}
struct OfflineSchemas;
impl jsonschema::Retrieve for OfflineSchemas {
    fn retrieve(
        &self,
        uri: &jsonschema::Uri<String>,
    ) -> std::result::Result<Value, Box<dyn Error + Send + Sync>> {
        let uri = uri.as_str();
        let source = match uri {
            "http://cyclonedx.org/schema/spdx.schema.json"
            | "https://cyclonedx.org/schema/spdx.schema.json" => include_str!(
                "../../../docs/testing/imap/0.75/schemas/vendor/cyclonedx-1.6/spdx.schema.json"
            ),
            "http://cyclonedx.org/schema/jsf-0.82.schema.json"
            | "https://cyclonedx.org/schema/jsf-0.82.schema.json" => include_str!(
                "../../../docs/testing/imap/0.75/schemas/vendor/cyclonedx-1.6/jsf-0.82.schema.json"
            ),
            _ => return Err("unregistered offline schema reference".into()),
        };
        Ok(serde_json::from_str(source)?)
    }
}
pub fn validate_cyclone_dx(bom: &Value) -> Result<()> {
    if bom["bomFormat"] != "CycloneDX" || bom["specVersion"] != "1.6" {
        return Err("pinned CycloneDX format/version differs".into());
    }
    let schema: Value = serde_json::from_str(include_str!(
        "../../../docs/testing/imap/0.75/schemas/vendor/cyclonedx-1.6/bom-1.6.schema.json"
    ))?;
    let validator = jsonschema::options()
        .with_retriever(OfflineSchemas)
        .build(&schema)?;
    if !validator.is_valid(bom) {
        return Err(format!(
            "CycloneDX schema rejected: {}",
            validator
                .iter_errors(bom)
                .take(3)
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("; ")
        )
        .into());
    }
    Ok(())
}
fn current_roots(
    root: &Path,
    components: &BTreeMap<String, Component>,
) -> Result<Vec<RootArtifact>> {
    let suffix = if cfg!(windows) { ".exe" } else { "" };
    let mut result = Vec::new();
    for (name, path) in [
        (
            "hydracache-server",
            format!("target/debug/hydracache-server{suffix}"),
        ),
        (
            "imap-contract-consumer-075",
            format!("target/debug/imap-contract-consumer-075{suffix}"),
        ),
        (
            "xtask",
            format!("target/debug/imap-soak-worker-075{suffix}"),
        ),
        (
            "java-imap-benchmark",
            "tests/java-imap-benchmark/target/java-imap-benchmark-0.75.0-SNAPSHOT.jar".into(),
        ),
        (
            "java-imap-hazelcast-075",
            "tests/java-imap-hazelcast-075/target/java-imap-hazelcast-075-0.75.0-SNAPSHOT.jar"
                .into(),
        ),
    ] {
        let candidates = components
            .values()
            .filter(|c| c.name == name && (c.ecosystem != "maven" || c.id.contains(":jar::")))
            .collect::<Vec<_>>();
        if candidates.len() != 1 {
            return Err(format!("ambiguous/missing artifact component {name}").into());
        }
        result.push(RootArtifact {
            component: candidates[0].id.clone(),
            artifact: artifact(root, &root.join(path), 512 * 1024 * 1024)?,
        });
    }
    Ok(result)
}
fn inputs(root: &Path, capture: &Capture) -> Result<Vec<Artifact>> {
    let mut paths = BTreeSet::new();
    for stage in &capture.stages {
        paths.insert(stage.stdout.clone());
        if let Some(journal) = &stage.journal {
            paths.insert(journal.clone());
        }
    }
    for name in ["contract-tree.json", "native-tree.json"] {
        paths.insert(format!("{}/{name}", capture.directory));
    }
    for path in [
        "Cargo.lock",
        crate::imap_sbom_capture_075::SETTINGS,
        crate::imap_sbom_capture_075::CLASSWORLDS,
        SPY,
        "docs/testing/imap/0.75/development-sbom-profile.json",
        "docs/testing/imap/0.75/schemas/development-provenance.schema.json",
        "docs/testing/imap/0.75/schemas/vendor/cyclonedx-1.6/bom-1.6.schema.json",
        "docs/testing/imap/0.75/schemas/vendor/cyclonedx-1.6/spdx.schema.json",
        "docs/testing/imap/0.75/schemas/vendor/cyclonedx-1.6/jsf-0.82.schema.json",
    ] {
        paths.insert(path.into());
    }
    paths
        .into_iter()
        .map(|p| artifact(root, &root.join(p), MAX))
        .collect()
}
fn pom_metadata(root: &Path, paths: &BTreeSet<String>) -> Result<Value> {
    let scratch = root.join("target/imap-075");
    if !scratch.canonicalize()?.starts_with(root.canonicalize()?) {
        return Err("foreign POM-list scratch refused".into());
    }
    let mut list = tempfile::NamedTempFile::new_in(scratch)?;
    list.write_all(
        paths
            .iter()
            .map(|p| format!("{p}\n"))
            .collect::<String>()
            .as_bytes(),
    )?;
    list.flush()?;
    let mut command = Command::new(crate::imap_api_differential::java_executable());
    command
        .args([
            "-Xms16m",
            "-Xmx128m",
            "-XX:ActiveProcessorCount=2",
            "-XX:ReservedCodeCacheSize=32m",
            "-XX:CompressedClassSpaceSize=64m",
        ])
        .arg(format!(
            "-XX:ErrorFile={}",
            root.join("target/imap-075/pom-metadata-jvm-%p.log")
                .display()
        ))
        .arg("-cp")
        .arg(root.join(SPY))
        .arg("io.hydracache.imap.provenance.MavenInventoryMain075")
        .arg(root)
        .arg(list.path());
    let output = crate::imap_proof_process_075::execute(
        command,
        &[],
        crate::imap_proof_process_075::ProcessBounds {
            timeout: Duration::from_secs(60),
            ..Default::default()
        },
    )?;
    Ok(serde_json::from_slice(&output.stdout)?)
}
pub fn reduce(root: &Path, capture: &Capture, source: &str) -> Result<Inventory> {
    let target_cfg = CargoTarget::current()?;
    if target_cfg.triple != capture.host {
        return Err("current Cargo target host differs".into());
    }
    if crate::imap_sbom_capture_075::rust_tools()?
        != (capture.rustc_sha256.clone(), capture.cargo_sha256.clone())
    {
        return Err("current Rust compiler/Cargo substituted".into());
    }
    if crate::imap_sbom_capture_075::rust_materials(&capture.host)? != capture.rust_inputs {
        return Err("Rust compiler/stdlib distribution inputs substituted".into());
    }
    let mut version = Command::new("rustc");
    version.arg("-vV");
    let output = crate::imap_proof_process_075::execute(version, &[], Default::default())?;
    if String::from_utf8(output.stdout)? != capture.rust_version
        || !capture
            .rust_version
            .lines()
            .any(|l| l == format!("host: {}", capture.host))
    {
        return Err("current Rust host/version differs".into());
    }
    let lock = String::from_utf8(read(root, "Cargo.lock")?)?;
    let mut components = BTreeMap::new();
    let mut edges = BTreeSet::new();
    let mut compilations = BTreeMap::new();
    let mut tools = BTreeMap::new();
    let stage = |name: &str| {
        capture
            .stages
            .iter()
            .find(|s| s.name == name)
            .ok_or("required fixed capture stage missing")
    };
    let root_metadata = load(root, &stage("root-metadata")?.stdout)?;
    let consumer_metadata = load(root, &stage("consumer-metadata")?.stdout)?;
    for metadata in [&root_metadata, &consumer_metadata] {
        for package in array(metadata, "packages")?
            .iter()
            .filter(|p| p["source"].is_null())
        {
            if !Path::new(text(package, "manifest_path")?).starts_with(root) {
                return Err("foreign path dependency in development closure".into());
            }
        }
    }
    let mut poms = BTreeSet::from([
        "tests/java-imap-benchmark/pom.xml".into(),
        "tests/java-imap-hazelcast-075/pom.xml".into(),
    ]);
    let mut materials = Vec::new();
    for s in &capture.stages {
        if let Some(journal) = &s.journal {
            let rows = maven_journal(
                &read(root, journal)?,
                s.nonce.as_deref().ok_or("Maven nonce missing")?,
            )?;
            for row in rows {
                match row["event"].as_str() {
                    Some("material") => {
                        let path = if row["location"] == "repository" {
                            format!("{REPOSITORY}/{}", text(&row, "path")?)
                        } else {
                            text(&row, "path")?.into()
                        };
                        let actual = artifact(root, &root.join(&path), 512 * 1024 * 1024)?;
                        if actual.sha256 != text(&row, "sha256")?
                            || actual.bytes
                                != row["bytes"].as_u64().ok_or("Maven material size missing")?
                        {
                            return Err("Maven material substituted after build".into());
                        }
                        if text(&row, "extension")? == "pom" {
                            poms.insert(path.clone());
                        }
                        materials.push((s.name.clone(), row, path));
                    }
                    Some("tool") => {
                        let key = format!("{}:{}", text(&row, "kind")?, text(&row, "path")?);
                        if let Some(old) = tools.insert(key, row.clone()) {
                            if old != row {
                                return Err("tool input changed across Maven invocations".into());
                            }
                        }
                    }
                    Some("init") => {
                        let version = json!({"kind":"runtime-versions","maven":row["maven_version"],"java":row["java_version"],"vendor":row["java_vendor"]});
                        if let Some(old) = tools.insert("runtime-versions".into(), version.clone())
                        {
                            if old != version {
                                return Err("Maven/JDK runtime changed".into());
                            }
                        }
                    }
                    _ => {}
                }
            }
        } else if !s.name.ends_with("metadata") {
            let metadata = if s.name.starts_with("consumer-") {
                &consumer_metadata
            } else {
                &root_metadata
            };
            let (cs, es, ins) = cargo_projection_for_target(
                metadata,
                &read(root, &s.stdout)?,
                &s.name,
                &lock,
                source,
                &target_cfg,
            )?;
            for c in cs {
                insert(&mut components, c)?;
            }
            edges.extend(es);
            for i in ins {
                compilations.insert(serde_json::to_string(&i)?, i);
            }
        }
    }
    let java = crate::imap_api_differential::java_executable();
    let jdk = java.parent().and_then(Path::parent).ok_or("JDK layout")?;
    let mut maven = crate::imap_sbom_capture_075::maven_command(root)?;
    maven.arg("--version");
    let output = crate::imap_proof_process_075::execute(maven, &[], Default::default())?;
    let version = String::from_utf8(output.stdout)?;
    let home = version
        .lines()
        .find_map(|l| l.strip_prefix("Maven home: "))
        .ok_or("current Maven home missing")?;
    if Path::new(home.trim()).canonicalize()? != Path::new(&capture.maven_home).canonicalize()? {
        return Err("current Maven distribution differs".into());
    }
    for row in tools
        .values()
        .filter(|row| matches!(row["kind"].as_str(), Some("maven" | "jdk")))
    {
        let base = if row["kind"] == "jdk" {
            jdk
        } else {
            Path::new(&capture.maven_home)
        };
        let path = text(row, "path")?;
        if path.contains('\\')
            || path.contains(':')
            || Path::new(path)
                .components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return Err("foreign tool material path".into());
        }
        let file = base.join(path);
        if crate::imap_daemon_probe_075::binary_digest(&file.canonicalize()?)?
            != text(row, "sha256")?
        {
            return Err("current Maven/JDK input hash differs".into());
        }
    }
    let metadata = pom_metadata(root, &poms)?;
    for (stage, row, path) in materials {
        let id = maven_id(&row)?;
        let pom = if row["extension"] == "pom" {
            metadata.get(&path)
        } else {
            let group = text(&row, "group")?.replace('.', "/");
            let name = text(&row, "name")?;
            let version = text(&row, "version")?;
            metadata.get(format!(
                "{REPOSITORY}/{group}/{name}/{version}/{name}-{version}.pom"
            ))
        };
        let licenses = pom
            .and_then(|p| p["licenses"].as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|l| l["name"].as_str().map(str::to_owned))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let origin = if licenses.is_empty() {
            "unknown"
        } else {
            "direct-pom-declaration"
        };
        insert(
            &mut components,
            Component {
                id: id.clone(),
                ecosystem: "maven".into(),
                name: text(&row, "name")?.into(),
                version: text(&row, "version")?.into(),
                source: format!(
                    "registry+https://repo.maven.apache.org/maven2/{};pom-sha256={};scm={}",
                    text(&row, "group")?.replace('.', "/"),
                    pom.and_then(|p| p["sha256"].as_str()).unwrap_or("unknown"),
                    pom.and_then(|p| p["source_url"].as_str())
                        .filter(|s| !s.is_empty())
                        .unwrap_or("unknown")
                ),
                source_checksum: text(&row, "sha256")?.into(),
                upstream_source: pom
                    .and_then(|p| p["source_url"].as_str())
                    .filter(|s| !s.is_empty())
                    .unwrap_or("unknown")
                    .into(),
                upstream_origin: if pom
                    .and_then(|p| p["source_url"].as_str())
                    .is_some_and(|s| !s.is_empty())
                {
                    "direct-pom-declaration"
                } else {
                    "unknown"
                }
                .into(),
                licenses,
                license_origin: origin.into(),
            },
        )?;
        // Build-input edges describe resolver input closure, not runtime linkage of Maven plugins.
        let owner = if stage == "JavaContract" {
            "maven:io.hydracache.test:java-imap-benchmark:jar::0.75.0-SNAPSHOT"
        } else if stage == "NativeJava" {
            "maven:io.hydracache.test:java-imap-hazelcast-075:jar::0.75.0-SNAPSHOT"
        } else {
            continue;
        };
        edges.insert(Edge {
            from: owner.into(),
            to: id,
            kind: "build-input".into(),
            target: "jvm-release-17".into(),
            stage,
        });
    }
    // Built roots need not be emitted as repository resolution events. Their source is our POM/revision.
    for (name, module) in [
        ("java-imap-benchmark", "java-imap-benchmark"),
        ("java-imap-hazelcast-075", "java-imap-hazelcast-075"),
    ] {
        let id = format!("maven:io.hydracache.test:{name}:jar::0.75.0-SNAPSHOT");
        let hash = crate::imap_daemon_probe_075::binary_digest(
            &root.join(format!("tests/{module}/target/{name}-0.75.0-SNAPSHOT.jar")),
        )?;
        let declarations = &metadata[format!("tests/{module}/pom.xml")];
        let licenses = array(declarations, "licenses")?
            .iter()
            .map(|v| text(v, "name").map(str::to_owned))
            .collect::<Result<Vec<_>>>()?;
        let c = Component {
            id,
            ecosystem: "maven".into(),
            name: name.into(),
            version: "0.75.0-SNAPSHOT".into(),
            source: format!("workspace:git:{source}"),
            source_checksum: hash,
            upstream_source: declarations["source_url"]
                .as_str()
                .filter(|s| !s.is_empty())
                .unwrap_or("unknown")
                .into(),
            upstream_origin: if declarations["source_url"]
                .as_str()
                .is_some_and(|s| !s.is_empty())
            {
                "direct-pom-declaration"
            } else {
                "unknown"
            }
            .into(),
            licenses,
            license_origin: text(declarations, "license_origin")?.into(),
        };
        // Installed contract JAR and workspace JAR are byte-identical; repository location is not source authority.
        if let Some(old) = components.remove(&c.id) {
            if old.source_checksum != c.source_checksum {
                return Err("installed Java root differs".into());
            }
        }
        insert(&mut components, c)?;
    }
    for name in ["contract", "native"] {
        edges.extend(tree_edges(
            &load(root, &format!("{}/{name}-tree.json", capture.directory))?,
            name,
            &components,
        )?);
    }
    tools.insert("rustc".into(),json!({"kind":"rustc","version":capture.rust_version,"target":capture.host,"target_cfg":target_cfg.cfg,"rustc_sha256":capture.rustc_sha256,"cargo_sha256":capture.cargo_sha256}));
    for input in &capture.rust_inputs {
        tools.insert(
            format!("rust-distribution:{}", text(input, "path")?),
            input.clone(),
        );
    }
    tools.insert(
        "maven-event-spy".into(),
        json!({"kind":"maven-event-spy","artifact":artifact(root,&root.join(SPY),MAX)?}),
    );
    let inventory = Inventory {
        schema: "imap075-development-inventory-v1".into(),
        profile: PROFILE.into(),
        target: capture.host.clone(),
        source_sha: source.into(),
        cargo_lock_sha256: digest(lock.as_bytes()),
        roots: current_roots(root, &components)?,
        components: components.into_values().collect(),
        edges: edges.into_iter().collect(),
        compilations: compilations.into_values().collect(),
        tools: tools.into_values().collect(),
    };
    validate_inventory(&inventory)?;
    Ok(inventory)
}

pub fn policy_state(success: bool, complete: bool, diagnostics: &[u8]) -> PolicyState {
    let mut rejected = false;
    let mut infrastructure = false;
    let mut count = 0;
    for line in diagnostics.split(|b| *b == b'\n').filter(|l| !l.is_empty()) {
        let Ok(row) = serde_json::from_slice::<Value>(line) else {
            infrastructure = true;
            continue;
        };
        count += 1;
        let diagnostic = row.get("fields").unwrap_or(&row);
        let code = diagnostic["code"].as_str().unwrap_or("");
        if diagnostic["severity"] == "error" || row["level"] == "ERROR" {
            if matches!(
                code,
                "vulnerability"
                    | "unsound"
                    | "unmaintained"
                    | "yanked"
                    | "denied"
                    | "not-allowed"
                    | "unlicensed"
                    | "license-not-encountered"
                    | "banned"
                    | "duplicate"
                    | "unmatched-organization"
                    | "unknown-registry"
                    | "unknown-git"
                    | "rejected"
                    | "source-not-allowed"
                    | "git-source-underspecified"
                    | "build-script-not-allowed"
                    | "exact-features-mismatch"
                    | "feature-not-explicitly-allowed"
                    | "feature-banned"
                    | "default-feature-enabled"
                    | "checksum-mismatch"
            ) {
                rejected = true;
            } else {
                infrastructure = true;
            }
        }
    }
    if rejected {
        PolicyState::Failed
    } else if success && complete && !infrastructure && count > 0 {
        PolicyState::Passed
    } else {
        PolicyState::Unknown
    }
}
fn capture_policy(root: &Path, directory: &Path) -> Result<Policy> {
    use crate::imap_build_process_075::{BuildBounds, BuildController, BuildStage, Cancellation};
    let mut controller = BuildController::with_budget(
        root,
        Duration::from_secs(65),
        BuildBounds {
            phase_timeout: Duration::from_secs(55),
            cleanup_reserve: Duration::from_secs(5),
            stdout_bytes: 64 * 1024,
            stderr_bytes: 256 * 1024,
            ..Default::default()
        },
        Cancellation::default(),
    )?;
    let mut command = Command::new("cargo");
    command
        .current_dir(root)
        .args(["deny", "--format", "json", "check"]);
    let (success, report) = match controller.run(BuildStage::ProvenanceMetadata, command) {
        Ok(report) => (true, report),
        Err(error) => (false, *error.report),
    };
    let stdout = fs::read(&report.stdout_path).unwrap_or_default();
    let stderr = fs::read(&report.stderr_path).unwrap_or_default();
    let rust_state = policy_state(success, report.logs_complete && report.root_reaped, &stderr);
    let raw = json!({"schema":"imap075-dependency-policy-v1","scope":"cargo-deny-root-workspace-default-features; Java advisory/license policy not run","rust_state":rust_state,"java_state":"unknown","build_report":report,"stdout":String::from_utf8_lossy(&stdout),"stderr":String::from_utf8_lossy(&stderr)});
    let path = directory.join("dependency-policy.json");
    fs::write(&path, serde_json::to_vec_pretty(&raw)?)?;
    Ok(Policy {
        state: if rust_state == PolicyState::Failed {
            PolicyState::Failed
        } else {
            PolicyState::Unknown
        },
        rust_state,
        java_state: PolicyState::Unknown,
        scope: "development-input-inventory; not security/release admission".into(),
        report: artifact(root, &path, MAX)?,
    })
}
pub fn validate_provenance_schema(value: &Value) -> Result<()> {
    let schema: Value = serde_json::from_str(include_str!(
        "../../../docs/testing/imap/0.75/schemas/development-provenance.schema.json"
    ))?;
    if !jsonschema::validator_for(&schema)?.is_valid(value) {
        return Err("development provenance schema rejected".into());
    }
    Ok(())
}
pub fn generate(root: &Path, capture: &Capture) -> Result<PathBuf> {
    let source = crate::imap_ci_075::source_state(root)?;
    if source != capture.source_sha {
        return Err("source changed during managed build capture".into());
    }
    let inventory = reduce(root, capture, &source)?;
    let bom = cyclone_dx(&inventory)?;
    let directory = root.join(&capture.directory);
    fs::write(
        directory.join("inventory.json"),
        serde_json::to_vec_pretty(&inventory)?,
    )?;
    fs::write(
        directory.join("bom.cdx.json"),
        serde_json::to_vec_pretty(&bom)?,
    )?;
    let provenance = Provenance {
        schema: "imap075-development-provenance-v1".into(),
        source_sha: source,
        source_clean: true,
        production_enabled: false,
        signature: "unsigned".into(),
        inventory_valid: true,
        capture: artifact(root, &directory.join("capture.json"), MAX)?,
        inputs: inputs(root, capture)?,
        inventory: artifact(root, &directory.join("inventory.json"), MAX)?,
        sbom: artifact(root, &directory.join("bom.cdx.json"), MAX)?,
        producer: artifact(root, &std::env::current_exe()?, 512 * 1024 * 1024)?,
        dependency_policy: capture_policy(root, &directory)?,
    };
    let bytes = serde_json::to_vec_pretty(&provenance)?;
    validate_provenance_schema(&serde_json::to_value(&provenance)?)?;
    fs::write(directory.join("provenance.json"), &bytes)?;
    verify(root, &directory.join("provenance.json"))?;
    fs::write(root.join(LATEST), &bytes)?;
    println!("imap-sbom: inventory valid; {} components; dependency policy {:?}; unsigned development scope",inventory.components.len(),provenance.dependency_policy.state);
    Ok(directory.join("provenance.json"))
}
fn check_artifact(root: &Path, expected: &Artifact, limit: usize) -> Result<Vec<u8>> {
    let bytes = read_relative(root, &expected.path, limit)?;
    if digest(&bytes) != expected.sha256 || bytes.len() as u64 != expected.bytes {
        return Err("provenance input checksum/size differs".into());
    }
    Ok(bytes)
}
pub fn verify(root: &Path, path: &Path) -> Result<Provenance> {
    let path = if path.is_absolute() {
        relative(root, path)?
    } else {
        path.to_string_lossy().replace('\\', "/")
    };
    let provenance: Provenance = serde_json::from_slice(&read(root, &path)?)?;
    validate_provenance_schema(&serde_json::to_value(&provenance)?)?;
    if provenance.schema != "imap075-development-provenance-v1"
        || provenance.signature != "unsigned"
        || !provenance.inventory_valid
        || provenance.production_enabled
        || !provenance.source_clean
        || provenance.source_sha != crate::imap_ci_075::source_state(root)?
        || provenance.producer != artifact(root, &std::env::current_exe()?, 512 * 1024 * 1024)?
    {
        return Err("foreign/stale/production provenance".into());
    }
    let capture: Capture =
        serde_json::from_slice(&check_artifact(root, &provenance.capture, MAX)?)?;
    if capture.source_sha != provenance.source_sha {
        return Err("build-start source differs from provenance".into());
    }
    if provenance.inputs != inputs(root, &capture)? {
        return Err("missing/substituted capture inputs".into());
    }
    let observed: Inventory =
        serde_json::from_slice(&check_artifact(root, &provenance.inventory, MAX)?)?;
    let expected = reduce(root, &capture, &provenance.source_sha)?;
    compare_inventory(&expected, &observed)?;
    let bom: Value = serde_json::from_slice(&check_artifact(root, &provenance.sbom, MAX)?)?;
    if bom != cyclone_dx(&expected)? {
        return Err("CycloneDX does not represent current resolved inventory".into());
    }
    let raw: Value = serde_json::from_slice(&check_artifact(
        root,
        &provenance.dependency_policy.report,
        MAX,
    )?)?;
    let p = &provenance.dependency_policy;
    let report: crate::imap_build_process_075::BuildReport =
        serde_json::from_value(raw["build_report"].clone())?;
    let stderr = raw["stderr"]
        .as_str()
        .filter(|s| s.len() <= 256 * 1024)
        .ok_or("bounded policy stderr missing")?;
    let stdout = raw["stdout"]
        .as_str()
        .filter(|s| s.len() <= 64 * 1024)
        .ok_or("bounded policy stdout missing")?;
    if digest(stderr.as_bytes()) != report.stderr_sha256
        || digest(stdout.as_bytes()) != report.stdout_sha256
    {
        return Err("policy raw report hash differs".into());
    }
    let rust = policy_state(
        report.outcome == crate::imap_build_process_075::BuildOutcome::Completed,
        report.logs_complete && report.root_reaped,
        stderr.as_bytes(),
    );
    if serde_json::to_value(&rust)? != raw["rust_state"]
        || rust != p.rust_state
        || p.java_state != PolicyState::Unknown
        || p.state
            != if rust == PolicyState::Failed {
                PolicyState::Failed
            } else {
                PolicyState::Unknown
            }
    {
        return Err("policy promoted unknown/failure".into());
    }
    Ok(provenance)
}
pub fn run(args: Vec<String>, check: bool) -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or("workspace root")?;
    if check {
        if args.len() != 4 || args[0] != "--release" || args[1] != "0.75" || args[2] != "--receipt"
        {
            return Err("expected --release 0.75 --receipt <provenance>".into());
        }
        let p = verify(root, Path::new(&args[3]))?;
        println!(
            "imap-sbom-check: inventory valid; dependency policy {:?}; unsigned",
            p.dependency_policy.state
        );
    } else {
        if args != ["--release", "0.75"] {
            return Err("expected --release 0.75".into());
        }
        crate::imap_pre074_075::prepare_tools(root)?;
    }
    Ok(())
}
