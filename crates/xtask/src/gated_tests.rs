use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use syn::visit::{self, Visit};
use syn::{Attribute, Expr, ItemFn, Lit, Meta};

use crate::doc_check;

pub const REGISTRY_PATH: &str = "docs/testing/gated-test-registry.toml";
const RELEASE: &str = "0.64.0";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GatedTestRegistry {
    pub schema_version: u32,
    pub release: String,
    #[serde(default)]
    pub gate: Vec<GateEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GateEntry {
    pub id: String,
    pub kind: GateKind,
    pub source: String,
    pub package: String,
    pub target: String,
    #[serde(default)]
    pub test: String,
    #[serde(default)]
    pub cfg: String,
    #[serde(default)]
    pub env: String,
    pub reason: String,
    pub tier: GateTier,
    #[serde(default)]
    pub required_features: Vec<String>,
    #[serde(default)]
    pub required_env: Vec<String>,
    #[serde(default)]
    pub required_tools: Vec<String>,
    pub timeout_seconds: u64,
    pub owner_release: String,
    pub ship_mandatory: bool,
    #[serde(default)]
    pub artifacts: Vec<String>,
    pub ci: CiRegistration,
    pub command: CommandSpec,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GateKind {
    IgnoredTest,
    CfgTestTarget,
    EnvGate,
    ExternalTool,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GateTier {
    Fast,
    Nightly,
    Manual,
    External,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CiRegistration {
    pub workflow: String,
    pub job: String,
    pub step: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CommandSpec {
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default = "default_cwd")]
    pub cwd: String,
    #[serde(default = "default_platform")]
    pub platform: String,
}

fn default_cwd() -> String {
    ".".to_owned()
}

fn default_platform() -> String {
    "any".to_owned()
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiscoveredGate {
    IgnoredTest {
        package: String,
        target: String,
        source: String,
        test: String,
        reason: String,
    },
    CfgTestTarget {
        package: String,
        target: String,
        source: String,
        cfg: String,
    },
    EnvGate {
        source: String,
        env: String,
    },
}

#[derive(Debug, Deserialize)]
struct CargoMetadata {
    packages: Vec<MetadataPackage>,
    workspace_members: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct MetadataPackage {
    id: String,
    name: String,
    manifest_path: PathBuf,
    targets: Vec<MetadataTarget>,
}

#[derive(Debug, Deserialize)]
struct MetadataTarget {
    name: String,
    kind: Vec<String>,
    src_path: PathBuf,
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let (root, emit_template) = parse_args(args)?;
    if let Some(path) = emit_template {
        let registry = registry_template(&discover_gates(&root)?);
        let text = toml::to_string_pretty(&registry)?;
        let path = root.join(path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, text)?;
        println!("gated-test-check: wrote template to {}", path.display());
        return Ok(());
    }
    let problems = check_registry(&root)?;
    if problems.is_empty() {
        let discovered = discover_gates(&root)?;
        println!(
            "gated-test-check: OK ({} discovered gates, all registered)",
            discovered.len()
        );
        Ok(())
    } else {
        for problem in &problems {
            eprintln!("gated-test-check: {problem}");
        }
        Err(format!("gated-test-check found {} problem(s)", problems.len()).into())
    }
}

pub fn check_registry(root: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let registry = load_registry(root)?;
    let discovered = discover_gates(root)?;
    Ok(validate_registry(&registry, &discovered))
}

pub fn load_registry(root: &Path) -> Result<GatedTestRegistry, Box<dyn Error>> {
    let path = root.join(REGISTRY_PATH);
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("reading {}: {error}", path.display()))?;
    toml::from_str(&text).map_err(|error| format!("parsing {REGISTRY_PATH}: {error}").into())
}

pub fn discover_gates(root: &Path) -> Result<BTreeSet<DiscoveredGate>, Box<dyn Error>> {
    let metadata = cargo_metadata(root)?;
    let members: BTreeSet<_> = metadata.workspace_members.into_iter().collect();
    let mut discovered = BTreeSet::new();
    let mut package_roots = BTreeSet::new();
    let mut env_sources = BTreeMap::<String, String>::new();

    for package in metadata
        .packages
        .into_iter()
        .filter(|package| members.contains(&package.id))
    {
        if let Some(package_root) = package.manifest_path.parent() {
            package_roots.insert(package_root.to_path_buf());
        }
        for target in package
            .targets
            .iter()
            .filter(|target| target.kind.iter().any(|kind| kind == "test"))
        {
            discover_test_target(root, &package.name, target, &mut discovered)?;
        }
    }

    let mut rust_files = BTreeSet::new();
    for package_root in package_roots {
        collect_rust_files(&package_root, &mut rust_files)?;
    }
    for path in rust_files {
        for env in discover_env_gates(&path)? {
            env_sources
                .entry(env)
                .or_insert_with(|| repo_relative(root, &path));
        }
    }
    for (env, source) in env_sources {
        discovered.insert(DiscoveredGate::EnvGate { source, env });
    }
    Ok(discovered)
}

fn cargo_metadata(root: &Path) -> Result<CargoMetadata, Box<dyn Error>> {
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into());
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}

fn discover_test_target(
    root: &Path,
    package: &str,
    target: &MetadataTarget,
    discovered: &mut BTreeSet<DiscoveredGate>,
) -> Result<(), Box<dyn Error>> {
    let text = fs::read_to_string(&target.src_path)?;
    let syntax = syn::parse_file(&text)
        .map_err(|error| format!("parsing {}: {error}", target.src_path.display()))?;
    let source = repo_relative(root, &target.src_path);
    for attr in &syntax.attrs {
        if attr.path().is_ident("cfg") {
            discovered.insert(DiscoveredGate::CfgTestTarget {
                package: package.to_owned(),
                target: target.name.clone(),
                source: source.clone(),
                cfg: meta_list_tokens(attr),
            });
        }
    }
    let mut visitor = TestAttributeVisitor {
        package,
        target: &target.name,
        source: &source,
        discovered,
    };
    visitor.visit_file(&syntax);
    Ok(())
}

struct TestAttributeVisitor<'a> {
    package: &'a str,
    target: &'a str,
    source: &'a str,
    discovered: &'a mut BTreeSet<DiscoveredGate>,
}

impl<'ast> Visit<'ast> for TestAttributeVisitor<'_> {
    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        if is_test_function(&node.attrs) {
            let ignored = node.attrs.iter().any(is_ignore_attr);
            if ignored {
                self.discovered.insert(DiscoveredGate::IgnoredTest {
                    package: self.package.to_owned(),
                    target: self.target.to_owned(),
                    source: self.source.to_owned(),
                    test: node.sig.ident.to_string(),
                    reason: ignore_reason(&node.attrs),
                });
            }
        }
        visit::visit_item_fn(self, node);
    }
}

fn is_test_function(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path()
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "test")
    })
}

fn is_ignore_attr(attr: &Attribute) -> bool {
    attr.path().is_ident("ignore")
        || (attr.path().is_ident("cfg_attr") && meta_list_tokens(attr).contains("ignore"))
}

fn ignore_reason(attrs: &[Attribute]) -> String {
    attrs
        .iter()
        .find(|attr| attr.path().is_ident("ignore"))
        .and_then(|attr| match &attr.meta {
            Meta::NameValue(value) => match &value.value {
                Expr::Lit(expr) => match &expr.lit {
                    Lit::Str(value) => Some(value.value()),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        })
        .unwrap_or_else(|| "ignored test requires explicit gated execution".to_owned())
}

fn meta_list_tokens(attr: &Attribute) -> String {
    match &attr.meta {
        Meta::List(list) => normalize_tokens(&list.tokens.to_string()),
        _ => String::new(),
    }
}

fn normalize_tokens(tokens: &str) -> String {
    tokens.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn collect_rust_files(dir: &Path, files: &mut BTreeSet<PathBuf>) -> Result<(), Box<dyn Error>> {
    if dir.ends_with("target") || dir.ends_with(".git") {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            if !matches!(
                path.file_name().and_then(|name| name.to_str()),
                Some("target" | ".git")
            ) {
                collect_rust_files(&path, files)?;
            }
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.insert(path);
        }
    }
    Ok(())
}

fn discover_env_gates(path: &Path) -> Result<BTreeSet<String>, Box<dyn Error>> {
    let text = fs::read_to_string(path)?;
    let Ok(syntax) = syn::parse_file(&text) else {
        return Ok(BTreeSet::new());
    };
    let mut visitor = EnvLiteralVisitor::default();
    visitor.visit_file(&syntax);
    Ok(visitor.names)
}

#[derive(Default)]
struct EnvLiteralVisitor {
    names: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for EnvLiteralVisitor {
    fn visit_lit(&mut self, node: &'ast Lit) {
        if let Lit::Str(value) = node {
            let value = value.value();
            if is_gate_env(&value) {
                self.names.insert(value);
            }
        }
        visit::visit_lit(self, node);
    }
}

fn is_gate_env(value: &str) -> bool {
    has_gate_suffix(value, "HYDRACACHE_RUN_")
        || has_gate_suffix(value, "HYDRACACHE_REQUIRE_")
        || has_gate_suffix(value, "HYDRACACHE_FORCE_")
        || has_gate_suffix(value, "HYDRACACHE_TEST_")
        || matches!(value, "HYDRACACHE_OPERATOR_KIND" | "HYDRACACHE_GRID_SCOPE")
}

fn has_gate_suffix(value: &str, prefix: &str) -> bool {
    value.strip_prefix(prefix).is_some_and(|suffix| {
        !suffix.is_empty()
            && suffix
                .bytes()
                .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
    })
}

fn registry_template(discovered: &BTreeSet<DiscoveredGate>) -> GatedTestRegistry {
    let cfg_by_source: BTreeMap<_, _> = discovered
        .iter()
        .filter_map(|gate| match gate {
            DiscoveredGate::CfgTestTarget { source, cfg, .. } => {
                Some((source.as_str(), cfg.as_str()))
            }
            _ => None,
        })
        .collect();
    let gate = discovered
        .iter()
        .map(|gate| {
            let mut entry = template_entry(gate);
            if entry.kind == GateKind::IgnoredTest {
                if let Some(feature) = cfg_by_source
                    .get(entry.source.as_str())
                    .and_then(|cfg| cfg_feature(cfg))
                {
                    entry.required_features.push(feature.clone());
                    insert_cargo_option(
                        &mut entry.command.args,
                        ["--features".to_owned(), feature],
                    );
                }
                for env in &entry.required_env {
                    if !env.starts_with("HYDRACACHE_TEST_") {
                        entry.command.env.insert(env.clone(), "1".to_owned());
                    }
                }
            }
            entry
        })
        .collect();
    GatedTestRegistry {
        schema_version: 1,
        release: RELEASE.to_owned(),
        gate,
    }
}

fn template_entry(discovered: &DiscoveredGate) -> GateEntry {
    let ci = CiRegistration {
        workflow: ".github/workflows/ci.yml".to_owned(),
        job: "gated-proof-registry".to_owned(),
        step: "Run registered gated proofs".to_owned(),
    };
    match discovered {
        DiscoveredGate::IgnoredTest {
            package,
            target,
            source,
            test,
            reason,
        } => {
            let tier = tier_from_reason(reason);
            GateEntry {
                id: format!(
                    "ignored.{}.{}.{}",
                    sanitize_id(package),
                    sanitize_id(target),
                    sanitize_id(test)
                ),
                kind: GateKind::IgnoredTest,
                source: source.clone(),
                package: package.clone(),
                target: target.clone(),
                test: test.clone(),
                cfg: String::new(),
                env: String::new(),
                reason: reason.clone(),
                tier,
                required_features: Vec::new(),
                required_env: env_names_in_text(reason),
                required_tools: tools_from_reason(reason),
                timeout_seconds: timeout_for_tier(tier),
                owner_release: RELEASE.to_owned(),
                ship_mandatory: false,
                artifacts: Vec::new(),
                ci: ci.clone(),
                command: cargo_test_command(package, target, Some(test)),
            }
        }
        DiscoveredGate::CfgTestTarget {
            package,
            target,
            source,
            cfg,
        } => {
            let features = cfg_feature(cfg).into_iter().collect::<Vec<_>>();
            let mut command = cargo_test_command(package, target, None);
            if let Some(feature) = features.first() {
                command
                    .args
                    .extend(["--features".to_owned(), feature.clone()]);
            } else if cfg.contains("hydracache_loom") {
                command
                    .env
                    .insert("RUSTFLAGS".to_owned(), "--cfg hydracache_loom".to_owned());
            }
            GateEntry {
                id: format!("cfg.{}.{}", sanitize_id(package), sanitize_id(target)),
                kind: GateKind::CfgTestTarget,
                source: source.clone(),
                package: package.clone(),
                target: target.clone(),
                test: String::new(),
                cfg: cfg.clone(),
                env: String::new(),
                reason: format!("Cargo test target is compiled only under cfg({cfg})"),
                tier: GateTier::Nightly,
                required_features: features,
                required_env: Vec::new(),
                required_tools: Vec::new(),
                timeout_seconds: timeout_for_tier(GateTier::Nightly),
                owner_release: RELEASE.to_owned(),
                ship_mandatory: false,
                artifacts: Vec::new(),
                ci,
                command,
            }
        }
        DiscoveredGate::EnvGate { source, env } => {
            let (package, target, command, tools, tier) = env_gate_command(env);
            let (ci, owner_release) = if env == "HYDRACACHE_RUN_REDIS_RESP_MULTINODE_E2E" {
                (
                    CiRegistration {
                        workflow: ".github/workflows/ci.yml".to_owned(),
                        job: "dst-nightly-soak".to_owned(),
                        step: "Redis RESP multinode debt sentinels".to_owned(),
                    },
                    "0.65.0",
                )
            } else {
                (ci, RELEASE)
            };
            GateEntry {
                id: format!("env.{}", sanitize_id(env)),
                kind: GateKind::EnvGate,
                source: source.clone(),
                package,
                target,
                test: String::new(),
                cfg: String::new(),
                env: env.clone(),
                reason: format!("Execution is explicitly gated by {env}"),
                tier,
                required_features: Vec::new(),
                required_env: vec![env.clone()],
                required_tools: tools,
                timeout_seconds: timeout_for_tier(tier),
                owner_release: owner_release.to_owned(),
                ship_mandatory: matches!(
                    env.as_str(),
                    "HYDRACACHE_RUN_RAFT_NEMESIS_SOAK"
                        | "HYDRACACHE_RUN_CANCELLATION_RAFT"
                        | "HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS"
                        | "HYDRACACHE_RUN_REDIS_RESP_MULTINODE_E2E"
                        | "HYDRACACHE_REQUIRE_REDIS_ORACLE"
                ),
                artifacts: Vec::new(),
                ci,
                command,
            }
        }
    }
}

fn cargo_test_command(package: &str, target: &str, test: Option<&str>) -> CommandSpec {
    let mut args = vec![
        "test".to_owned(),
        "-p".to_owned(),
        package.to_owned(),
        "--test".to_owned(),
        target.to_owned(),
        "--locked".to_owned(),
    ];
    if let Some(test) = test {
        args.push(test.to_owned());
    }
    if test.is_some() {
        args.extend([
            "--".to_owned(),
            "--ignored".to_owned(),
            "--nocapture".to_owned(),
        ]);
    }
    CommandSpec {
        program: "cargo".to_owned(),
        args,
        env: BTreeMap::new(),
        cwd: ".".to_owned(),
        platform: "any".to_owned(),
    }
}

fn insert_cargo_option<const N: usize>(args: &mut Vec<String>, option: [String; N]) {
    let position = args
        .iter()
        .position(|arg| arg == "--locked")
        .unwrap_or(args.len());
    args.splice(position..position, option);
}

fn env_gate_command(env: &str) -> (String, String, CommandSpec, Vec<String>, GateTier) {
    let redis_client = env.contains("REDIS_CLIENT") || env.contains("REDIS_ORACLE");
    let (package, target, mut command, tools, tier) = if redis_client {
        let package = "hydracache-redis-compat";
        let target = "redis_clients";
        (
            package,
            target,
            cargo_test_command(package, target, None),
            vec!["docker".to_owned()],
            GateTier::External,
        )
    } else {
        match env {
            "HYDRACACHE_RUN_REDIS_COMPAT_RESOURCE_SMOKE" => (
                "hydracache-redis-compat",
                "resp_resource_smoke",
                cargo_test_command("hydracache-redis-compat", "resp_resource_smoke", None),
                Vec::new(),
                GateTier::Nightly,
            ),
            "HYDRACACHE_RUN_REDIS_RESP_MULTINODE_E2E" => (
                "hydracache-server",
                "redis_resp_multinode",
                cargo_test_command("hydracache-server", "redis_resp_multinode", None),
                Vec::new(),
                GateTier::Nightly,
            ),
            "HYDRACACHE_RUN_DAEMON_PROCESS_E2E" => (
                "hydracache-server",
                "daemon_process_cluster",
                cargo_test_command("hydracache-server", "daemon_process_cluster", None),
                Vec::new(),
                GateTier::Nightly,
            ),
            "HYDRACACHE_RUN_NETWORKED_DAEMON_E2E" => (
                "hydracache-server",
                "grid_host",
                cargo_test_command("hydracache-server", "grid_host", None),
                Vec::new(),
                GateTier::Nightly,
            ),
            "HYDRACACHE_RUN_PREVOTE_NIGHTLY_SOAK" => (
                "hydracache-cluster-raft",
                "prevote_nightly_soak",
                cargo_test_command("hydracache-cluster-raft", "prevote_nightly_soak", None),
                Vec::new(),
                GateTier::Nightly,
            ),
            "HYDRACACHE_RUN_RAFT_NEMESIS_SOAK" => (
                "hydracache-cluster-raft",
                "nemesis_membership",
                cargo_test_command("hydracache-cluster-raft", "nemesis_membership", None),
                Vec::new(),
                GateTier::Nightly,
            ),
            "HYDRACACHE_RUN_CANCELLATION_RAFT" => {
                let mut command =
                    cargo_test_command("hydracache-cluster-raft", "cancellation_safety", None);
                command.args.extend([
                    "--".to_owned(),
                    "--ignored".to_owned(),
                    "--nocapture".to_owned(),
                ]);
                (
                    "hydracache-cluster-raft",
                    "cancellation_safety",
                    command,
                    Vec::new(),
                    GateTier::Nightly,
                )
            }
            "HYDRACACHE_RUN_RAFT_MUTANTS" => (
                "xtask",
                "mutants",
                CommandSpec {
                    program: "cargo".to_owned(),
                    args: vec!["xtask".to_owned(), "mutants".to_owned()],
                    env: BTreeMap::new(),
                    cwd: ".".to_owned(),
                    platform: "any".to_owned(),
                },
                vec!["cargo-mutants".to_owned()],
                GateTier::Nightly,
            ),
            "HYDRACACHE_GRID_SCOPE" => (
                "hydracache-cluster-raft",
                "snapshot_exhaustive_grid",
                cargo_test_command("hydracache-cluster-raft", "snapshot_exhaustive_grid", None),
                Vec::new(),
                GateTier::Nightly,
            ),
            "HYDRACACHE_OPERATOR_KIND" => (
                "hydracache-operator",
                "soak_kind",
                cargo_test_command("hydracache-operator", "soak_kind", None),
                vec!["docker".to_owned(), "kind".to_owned(), "kubectl".to_owned()],
                GateTier::External,
            ),
            "HYDRACACHE_TEST_MYSQL_URL" => (
                "hydracache-db",
                "mysql_hooks",
                cargo_test_command("hydracache-db", "mysql_hooks", None),
                vec!["mysql".to_owned()],
                GateTier::External,
            ),
            "HYDRACACHE_TEST_POSTGRES_URL" => (
                "hydracache-db",
                "postgres_hooks",
                cargo_test_command("hydracache-db", "postgres_hooks", None),
                vec!["postgres".to_owned()],
                GateTier::External,
            ),
            _ => (
                "workspace",
                "environment",
                CommandSpec {
                    program: "cargo".to_owned(),
                    args: vec!["xtask".to_owned(), "gated-test-check".to_owned()],
                    env: BTreeMap::new(),
                    cwd: ".".to_owned(),
                    platform: "any".to_owned(),
                },
                Vec::new(),
                GateTier::Manual,
            ),
        }
    };
    command.env.insert(env.to_owned(), "1".to_owned());
    (package.to_owned(), target.to_owned(), command, tools, tier)
}

fn tier_from_reason(reason: &str) -> GateTier {
    let reason = reason.to_ascii_lowercase();
    if [
        "docker",
        "kind",
        "postgres",
        "mysql",
        "networked",
        "live grid",
    ]
    .iter()
    .any(|needle| reason.contains(needle))
    {
        GateTier::External
    } else if ["nightly", "chaos", "soak", "fault-injection"]
        .iter()
        .any(|needle| reason.contains(needle))
    {
        GateTier::Nightly
    } else {
        GateTier::Manual
    }
}

fn tools_from_reason(reason: &str) -> Vec<String> {
    let reason = reason.to_ascii_lowercase();
    ["docker", "kind", "kubectl", "postgres", "mysql"]
        .into_iter()
        .filter(|tool| reason.contains(tool))
        .map(str::to_owned)
        .collect()
}

fn timeout_for_tier(tier: GateTier) -> u64 {
    match tier {
        GateTier::Fast => 300,
        GateTier::Manual => 900,
        GateTier::Nightly => 1_800,
        GateTier::External => 3_600,
    }
}

fn cfg_feature(cfg: &str) -> Option<String> {
    let prefix = "feature = \"";
    cfg.strip_prefix(prefix)
        .and_then(|value| value.strip_suffix('"'))
        .map(str::to_owned)
}

fn env_names_in_text(text: &str) -> Vec<String> {
    text.split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .filter(|value| is_gate_env(value))
        .map(str::to_owned)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn sanitize_id(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_owned()
}

fn validate_registry(
    registry: &GatedTestRegistry,
    discovered: &BTreeSet<DiscoveredGate>,
) -> Vec<String> {
    let mut problems = Vec::new();
    if registry.schema_version != 1 {
        problems.push(format!(
            "{REGISTRY_PATH}: schema_version must be 1, got {}",
            registry.schema_version
        ));
    }
    if registry.release != RELEASE {
        problems.push(format!(
            "{REGISTRY_PATH}: release must be {RELEASE}, got {}",
            registry.release
        ));
    }

    let mut ids = BTreeSet::new();
    for gate in &registry.gate {
        if !ids.insert(gate.id.as_str()) {
            problems.push(format!("{REGISTRY_PATH}: duplicate gate id `{}`", gate.id));
        }
        validate_entry_shape(gate, &mut problems);
        if gate.kind != GateKind::ExternalTool
            && !discovered.iter().any(|item| entry_matches(gate, item))
        {
            problems.push(format!(
                "{REGISTRY_PATH}: stale gate `{}` does not resolve to a discovered gate",
                gate.id
            ));
        }
    }

    for item in discovered {
        let matches = registry
            .gate
            .iter()
            .filter(|entry| entry_matches(entry, item))
            .collect::<Vec<_>>();
        if matches.is_empty() {
            problems.push(format!(
                "{REGISTRY_PATH}: unregistered {}",
                describe_discovered(item)
            ));
        } else if matches.len() > 1 && !entries_form_complete_shard_set(item, &matches) {
            problems.push(format!(
                "{REGISTRY_PATH}: {} is covered by {} entries that do not form a complete --shard set",
                describe_discovered(item),
                matches.len()
            ));
        }
    }
    problems
}

fn entries_form_complete_shard_set(discovered: &DiscoveredGate, entries: &[&GateEntry]) -> bool {
    if !matches!(discovered, DiscoveredGate::EnvGate { .. }) {
        return false;
    }
    let shards = entries
        .iter()
        .filter_map(|entry| command_shard(&entry.command.args))
        .collect::<Vec<_>>();
    let Some(total) = shards.first().map(|(_, total)| *total) else {
        return false;
    };
    total == entries.len()
        && shards.len() == entries.len()
        && shards.iter().all(|(_, candidate)| *candidate == total)
        && shards
            .iter()
            .map(|(index, _)| *index)
            .collect::<BTreeSet<_>>()
            == (0..total).collect()
}

fn command_shard(args: &[String]) -> Option<(usize, usize)> {
    let values = args
        .windows(2)
        .filter(|pair| pair[0] == "--shard")
        .map(|pair| pair[1].as_str())
        .collect::<Vec<_>>();
    let [value] = values.as_slice() else {
        return None;
    };
    let (index, total) = value.split_once('/')?;
    let index = index.parse::<usize>().ok()?;
    let total = total.parse::<usize>().ok()?;
    (total > 0 && index < total).then_some((index, total))
}

fn validate_entry_shape(gate: &GateEntry, problems: &mut Vec<String>) {
    for (field, value) in [
        ("id", gate.id.as_str()),
        ("source", gate.source.as_str()),
        ("package", gate.package.as_str()),
        ("target", gate.target.as_str()),
        ("reason", gate.reason.as_str()),
        ("owner_release", gate.owner_release.as_str()),
        ("ci.workflow", gate.ci.workflow.as_str()),
        ("ci.job", gate.ci.job.as_str()),
        ("ci.step", gate.ci.step.as_str()),
        ("command.program", gate.command.program.as_str()),
        ("command.cwd", gate.command.cwd.as_str()),
        ("command.platform", gate.command.platform.as_str()),
    ] {
        if value.trim().is_empty() {
            problems.push(format!(
                "{REGISTRY_PATH}: gate `{}` is missing {field}",
                gate.id
            ));
        }
    }
    if gate.timeout_seconds == 0 {
        problems.push(format!(
            "{REGISTRY_PATH}: gate `{}` timeout_seconds must be positive",
            gate.id
        ));
    }
    match gate.kind {
        GateKind::IgnoredTest if gate.test.trim().is_empty() => problems.push(format!(
            "{REGISTRY_PATH}: ignored-test gate `{}` is missing test",
            gate.id
        )),
        GateKind::CfgTestTarget if gate.cfg.trim().is_empty() => problems.push(format!(
            "{REGISTRY_PATH}: cfg-test-target gate `{}` is missing cfg",
            gate.id
        )),
        GateKind::EnvGate if gate.env.trim().is_empty() => problems.push(format!(
            "{REGISTRY_PATH}: env-gate `{}` is missing env",
            gate.id
        )),
        GateKind::ExternalTool if gate.required_tools.is_empty() => problems.push(format!(
            "{REGISTRY_PATH}: external-tool gate `{}` is missing required_tools",
            gate.id
        )),
        _ => {}
    }
    problems.extend(redis_multinode_gate_contract_problems(gate));
}

pub fn redis_multinode_gate_contract_problems(gate: &GateEntry) -> Vec<String> {
    const ID: &str = "env.hydracache-run-redis-resp-multinode-e2e";
    const ENV: &str = "HYDRACACHE_RUN_REDIS_RESP_MULTINODE_E2E";
    if gate.id != ID && gate.env != ENV {
        return Vec::new();
    }

    let expected_args = [
        "test",
        "-p",
        "hydracache-server",
        "--test",
        "redis_resp_multinode",
        "--locked",
    ]
    .map(str::to_owned)
    .to_vec();
    let exact = gate.id == ID
        && gate.kind == GateKind::EnvGate
        && gate.source == "crates/hydracache-server/tests/support/daemon_cluster.rs"
        && gate.package == "hydracache-server"
        && gate.target == "redis_resp_multinode"
        && gate.env == ENV
        && gate.tier == GateTier::Nightly
        && gate.required_env == [ENV]
        && gate.owner_release == "0.65.0"
        && gate.ship_mandatory
        && gate.ci.workflow == ".github/workflows/ci.yml"
        && gate.ci.job == "dst-nightly-soak"
        && gate.ci.step == "Redis RESP multinode debt sentinels"
        && gate.command.program == "cargo"
        && gate.command.args == expected_args
        && gate.command.env.len() == 1
        && gate.command.env.get(ENV).is_some_and(|value| value == "1")
        && gate.command.cwd == "."
        && gate.command.platform == "any";
    (!exact)
        .then(|| {
            format!(
                "{REGISTRY_PATH}: Redis multinode gate must retain the exact dedicated target, env, CI step, owner release, and command"
            )
        })
        .into_iter()
        .collect()
}

fn entry_matches(entry: &GateEntry, discovered: &DiscoveredGate) -> bool {
    match (entry.kind, discovered) {
        (
            GateKind::IgnoredTest,
            DiscoveredGate::IgnoredTest {
                package,
                target,
                source,
                test,
                ..
            },
        ) => {
            entry.package == *package
                && entry.target == *target
                && normalize_path(&entry.source) == *source
                && entry.test == *test
        }
        (
            GateKind::CfgTestTarget,
            DiscoveredGate::CfgTestTarget {
                package,
                target,
                source,
                cfg,
            },
        ) => {
            entry.package == *package
                && entry.target == *target
                && normalize_path(&entry.source) == *source
                && normalize_tokens(&entry.cfg) == *cfg
        }
        (GateKind::EnvGate, DiscoveredGate::EnvGate { env, .. }) => entry.env == *env,
        _ => false,
    }
}

fn describe_discovered(gate: &DiscoveredGate) -> String {
    match gate {
        DiscoveredGate::IgnoredTest {
            package,
            target,
            source,
            test,
            ..
        } => format!("ignored test {package}/{target}::{test} in {source}"),
        DiscoveredGate::CfgTestTarget {
            package,
            target,
            source,
            cfg,
        } => format!("cfg-gated target {package}/{target} ({cfg}) in {source}"),
        DiscoveredGate::EnvGate { source, env } => {
            format!("environment gate {env:?} referenced by {source}")
        }
    }
}

fn repo_relative(root: &Path, path: &Path) -> String {
    normalize_path(
        path.strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .as_ref(),
    )
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}

fn parse_args(args: Vec<String>) -> Result<(PathBuf, Option<PathBuf>), Box<dyn Error>> {
    let mut root = None;
    let mut emit_template = None;
    let mut it = args.into_iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--root" => {
                root = Some(PathBuf::from(
                    it.next().ok_or("--root requires a path argument")?,
                ));
            }
            "--emit-template" => {
                emit_template = Some(PathBuf::from(
                    it.next()
                        .ok_or("--emit-template requires a path argument")?,
                ));
            }
            other => return Err(format!("unknown gated-test-check argument: {other}").into()),
        }
    }
    let root = root.map(Ok).unwrap_or_else(doc_check::find_repo_root)?;
    Ok((root, emit_template))
}
