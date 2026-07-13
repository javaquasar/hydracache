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

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if !args.is_empty() {
        return Err("cargo xtask mutants does not accept arguments".into());
    }

    let root = doc_check::find_repo_root()?;
    check_mutation_baseline(&root)?;

    if env::var_os(RUN_ENV).is_some() {
        run_cargo_mutants(&root)?;
    } else {
        println!("mutants: cargo-mutants execution skipped; set {RUN_ENV}=1 for the slow lane");
    }

    Ok(())
}

pub fn check_mutation_baseline(root: &Path) -> Result<(), String> {
    let config_path = root.join(CONFIG_PATH);
    let baseline_path = root.join(BASELINE_PATH);
    let report_path = root.join(REPORT_PATH);

    let config = read_required(&config_path)?;
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

fn run_cargo_mutants(root: &Path) -> Result<(), Box<dyn Error>> {
    if Command::new("cargo")
        .args(["mutants", "--version"])
        .current_dir(root)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
    {
        fs::create_dir_all(root.join("target/hydracache-mutants"))?;
        let status = Command::new("cargo")
            .args(["mutants", "--config", CONFIG_PATH])
            .current_dir(root)
            .status()?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("cargo mutants exited with {status}").into())
        }
    } else {
        Err("cargo-mutants is required for HYDRACACHE_RUN_RAFT_MUTANTS=1; install it with `cargo install cargo-mutants --locked`".into())
    }
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
