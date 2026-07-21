use std::collections::BTreeSet;
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::doc_check;

const CONFIG_PATH: &str = ".cargo/mutants.toml";
const BASELINE_PATH: &str = "docs/testing/mutation-baseline.md";
const REPORT_PATH: &str = "target/hydracache-mutants/report.txt";
const RUN_ENV: &str = "HYDRACACHE_RUN_RAFT_MUTANTS";
const PROOF_CONFIG_PATH: &str = ".cargo/mutants-proof-oracles.toml";
const PROOF_BASELINE_PATH: &str = "docs/testing/mutation-proof-oracle-baseline.md";
const PROOF_REPORT_PATH: &str = "target/hydracache-mutants-proof-oracles/report.txt";
const PROOF_RUN_ENV: &str = "HYDRACACHE_RUN_PROOF_ORACLE_MUTANTS";
const CARGO_MUTANTS_VERSION: &str = "27.1.0";

const REQUIRED_SCOPES: &[&str] = &[
    "crates/hydracache-cluster-raft/src/lib.rs",
    "crates/hydracache-cluster-raft/src/log_store.rs",
];

const REQUIRED_TESTS: &[&str] = &[
    "cargo test -p hydracache-cluster-raft snapshot_immutability --locked",
    "cargo test -p hydracache-cluster-raft --test raft_snapshot_membership --locked",
    "cargo test -p hydracache-cluster-raft --features test-failpoints snapshot_apply --locked -- --test-threads=1",
    "cargo test -p hydracache-cluster-raft --test rejoin_after_compaction --features test-failpoints --locked -- --test-threads=1",
    "cargo test -p hydracache-cluster-raft --test proposal_idempotency --locked",
];

const PROOF_REQUIRED_SCOPES: &[&str] = &[
    "crates/hydracache-sim/src/linearizability.rs",
    "crates/hydracache-cluster-testkit/src/invariants.rs",
    "crates/hydracache-cluster-testkit/src/client_surface_conformance.rs",
];

const PROOF_REQUIRED_TESTS: &[&str] = &[
    "cargo test -p hydracache-sim --test linearizability_oracle --locked",
    "cargo test -p hydracache-cluster-testkit --test invariants --locked",
    "cargo test -p hydracache-cluster-testkit --test client_surface_conformance_oracle --locked",
    "cargo test -p hydracache-client-transport-axum --test client_surface_conformance --locked",
];

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let root = doc_check::find_repo_root()?;
    let options = parse_options(args)?;
    let (config, run_env) = match options.scope {
        MutationScope::Product => {
            check_mutation_baseline(&root)?;
            (CONFIG_PATH, RUN_ENV)
        }
        MutationScope::ProofOracles => {
            check_proof_oracle_mutation_baseline(&root)?;
            (PROOF_CONFIG_PATH, PROOF_RUN_ENV)
        }
    };

    if env::var_os(run_env).is_some() {
        run_cargo_mutants(&root, config, options.shard.as_deref())?;
    } else {
        println!("mutants: cargo-mutants execution skipped; set {run_env}=1 for the slow lane");
    }

    Ok(())
}

#[derive(Clone, Copy)]
enum MutationScope {
    Product,
    ProofOracles,
}

struct MutationOptions {
    scope: MutationScope,
    shard: Option<String>,
}

fn parse_options(args: Vec<String>) -> Result<MutationOptions, Box<dyn Error>> {
    let mut scope = MutationScope::Product;
    let mut shard = None;
    let mut args = args.into_iter();
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--scope" => {
                scope = match args.next().as_deref() {
                    Some("product") => MutationScope::Product,
                    Some("proof-oracles") => MutationScope::ProofOracles,
                    _ => return Err(usage().into()),
                };
            }
            "--shard" => {
                if shard.is_some() {
                    return Err("mutants accepts --shard only once".into());
                }
                let value = args.next().ok_or_else(usage)?;
                validate_shard(&value)?;
                shard = Some(value);
            }
            _ => return Err(usage().into()),
        }
    }
    Ok(MutationOptions { scope, shard })
}

fn usage() -> String {
    "usage: cargo xtask mutants [--scope product|proof-oracles] [--shard INDEX/TOTAL]".to_owned()
}

fn validate_shard(value: &str) -> Result<(), Box<dyn Error>> {
    let (index, total) = value
        .split_once('/')
        .ok_or("mutation shard must use INDEX/TOTAL syntax")?;
    let index = index.parse::<usize>()?;
    let total = total.parse::<usize>()?;
    if total == 0 || index >= total {
        return Err(
            format!("invalid mutation shard {value}: require TOTAL > 0 and INDEX < TOTAL").into(),
        );
    }
    Ok(())
}

pub fn check_mutation_baseline(root: &Path) -> Result<(), String> {
    let config_path = root.join(CONFIG_PATH);
    let baseline_path = root.join(BASELINE_PATH);
    let report_path = root.join(REPORT_PATH);

    let config = read_required(&config_path)?;
    if config.contains("[hydracache]") {
        return Err(format!(
            "{CONFIG_PATH} must stay a native cargo-mutants config; do not add HydraCache-only tables"
        ));
    }
    toml::from_str::<toml::Value>(&config)
        .map_err(|error| format!("parsing {CONFIG_PATH}: {error}"))?;
    for scope in REQUIRED_SCOPES {
        if !config.contains(scope) {
            return Err(format!(
                "{CONFIG_PATH} is missing required mutation scope `{scope}`"
            ));
        }
    }
    for command in REQUIRED_TESTS {
        if !config.contains(command) {
            return Err(format!(
                "{CONFIG_PATH} is missing required test command `{command}`"
            ));
        }
    }

    let baseline = read_required(&baseline_path)?;
    if !baseline.contains("## Allowed Survivors") {
        return Err(format!(
            "{BASELINE_PATH} is missing the `Allowed Survivors` section"
        ));
    }
    if !baseline.contains("No allowed survivors.") && allowed_survivors(&baseline).is_empty() {
        return Err(format!(
            "{BASELINE_PATH} must either say `No allowed survivors.` or list `SURVIVED ...` entries"
        ));
    }
    for scope in REQUIRED_SCOPES {
        if !baseline.contains(scope) {
            return Err(format!("{BASELINE_PATH} is missing scoped path `{scope}`"));
        }
    }

    if !report_path.is_file() {
        println!(
            "mutants: no cached report at {}; baseline metadata OK, skipping survivor diff",
            report_path.display()
        );
        return Ok(());
    }

    let report = read_required(&report_path)?;
    let allowed = allowed_survivors(&baseline);
    let survivors = survived_lines(&report);
    let untriaged: Vec<_> = survivors.difference(&allowed).cloned().collect();
    if !untriaged.is_empty() {
        return Err(format!(
            "untriaged mutation survivor(s) in {}: {}",
            report_path.display(),
            untriaged.join("; ")
        ));
    }

    println!(
        "mutants: cached report at {} has no untriaged survivors",
        report_path.display()
    );
    Ok(())
}

pub fn check_proof_oracle_mutation_baseline(root: &Path) -> Result<(), String> {
    let config = read_required(&root.join(PROOF_CONFIG_PATH))?;
    if config.contains("[hydracache]") {
        return Err(format!(
            "{PROOF_CONFIG_PATH} must stay a native cargo-mutants config"
        ));
    }
    let parsed: toml::Value =
        toml::from_str(&config).map_err(|error| format!("parsing {PROOF_CONFIG_PATH}: {error}"))?;
    let globs = parsed
        .get("examine_globs")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| format!("{PROOF_CONFIG_PATH} is missing examine_globs"))?;
    for glob in globs.iter().filter_map(toml::Value::as_str) {
        if glob.replace('\\', "/").contains("/tests/") {
            return Err(format!(
                "{PROOF_CONFIG_PATH} must target reusable decision modules, not integration-test glue: {glob}"
            ));
        }
    }
    for scope in PROOF_REQUIRED_SCOPES {
        if !config.contains(scope) {
            return Err(format!(
                "{PROOF_CONFIG_PATH} is missing required proof-oracle scope `{scope}`"
            ));
        }
    }
    for command in PROOF_REQUIRED_TESTS {
        if !config.contains(command) {
            return Err(format!(
                "{PROOF_CONFIG_PATH} is missing required test command `{command}`"
            ));
        }
    }
    check_baseline_and_report(
        root,
        PROOF_BASELINE_PATH,
        PROOF_REPORT_PATH,
        PROOF_REQUIRED_SCOPES,
    )
}

fn check_baseline_and_report(
    root: &Path,
    baseline_path: &str,
    report_path: &str,
    scopes: &[&str],
) -> Result<(), String> {
    let baseline = read_required(&root.join(baseline_path))?;
    if !baseline.contains("## Allowed Survivors") {
        return Err(format!(
            "{baseline_path} is missing the `Allowed Survivors` section"
        ));
    }
    if !baseline.contains("No allowed survivors.") && allowed_survivors(&baseline).is_empty() {
        return Err(format!(
            "{baseline_path} must either say `No allowed survivors.` or list `SURVIVED ...` entries"
        ));
    }
    for scope in scopes {
        if !baseline.contains(scope) {
            return Err(format!("{baseline_path} is missing scoped path `{scope}`"));
        }
    }
    let report_path = root.join(report_path);
    if !report_path.is_file() {
        println!(
            "mutants: no cached report at {}; baseline metadata OK, skipping survivor diff",
            report_path.display()
        );
        return Ok(());
    }
    let report = read_required(&report_path)?;
    let allowed = allowed_survivors(&baseline);
    let survivors = survived_lines(&report);
    let untriaged = survivors.difference(&allowed).cloned().collect::<Vec<_>>();
    if !untriaged.is_empty() {
        return Err(format!(
            "untriaged mutation survivor(s) in {}: {}",
            report_path.display(),
            untriaged.join("; ")
        ));
    }
    Ok(())
}

fn run_cargo_mutants(root: &Path, config: &str, shard: Option<&str>) -> Result<(), Box<dyn Error>> {
    let version = Command::new("cargo")
        .args(["mutants", "--version"])
        .current_dir(root)
        .output();
    if let Ok(version) = version {
        if !version.status.success() {
            return Err(format!(
                "cargo-mutants {CARGO_MUTANTS_VERSION} is required for the mutation lane"
            )
            .into());
        }
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&version.stdout),
            String::from_utf8_lossy(&version.stderr)
        );
        if !text.contains(CARGO_MUTANTS_VERSION) {
            return Err(format!(
                "cargo-mutants {CARGO_MUTANTS_VERSION} is required, got {}",
                text.trim()
            )
            .into());
        }
        let args = cargo_mutants_args(config, shard)?;
        let status = Command::new("cargo")
            .args(&args)
            .current_dir(root)
            .status()?;
        if status.success() {
            let config: toml::Value = toml::from_str(&fs::read_to_string(root.join(config))?)?;
            let output = config
                .get("output")
                .and_then(toml::Value::as_str)
                .ok_or("cargo-mutants config is missing output")?;
            fs::create_dir_all(root.join(output))?;
            fs::write(
                root.join(output).join("report.txt"),
                format!(
                    "cargo-mutants {CARGO_MUTANTS_VERSION}\nSHARD {}\nCOMPLETED no survived mutants\n",
                    shard.unwrap_or("all")
                ),
            )?;
            Ok(())
        } else {
            Err(format!(
                "cargo mutants exited with {status}; inspect the configured mutants.out/log directory"
            )
            .into())
        }
    } else {
        Err(
            format!("cargo-mutants {CARGO_MUTANTS_VERSION} is required for the mutation lane")
                .into(),
        )
    }
}

pub fn cargo_mutants_args(
    config: &str,
    shard: Option<&str>,
) -> Result<Vec<String>, Box<dyn Error>> {
    if let Some(shard) = shard {
        validate_shard(shard)?;
    }
    let mut args = vec![
        "mutants".to_owned(),
        "--config".to_owned(),
        config.to_owned(),
        "--in-place".to_owned(),
    ];
    if let Some(shard) = shard {
        args.extend(["--shard".to_owned(), shard.to_owned()]);
    }
    Ok(args)
}

fn read_required(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("reading {}: {error}", path.display()))
}

fn allowed_survivors(text: &str) -> BTreeSet<String> {
    survived_lines(text)
}

fn survived_lines(text: &str) -> BTreeSet<String> {
    text.lines()
        .map(str::trim)
        .filter_map(|line| {
            line.strip_prefix("- ")
                .unwrap_or(line)
                .strip_prefix("SURVIVED ")
        })
        .map(|line| format!("SURVIVED {}", line.trim()))
        .collect()
}

#[allow(dead_code)]
fn _repo_relative(path: &Path, root: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}
