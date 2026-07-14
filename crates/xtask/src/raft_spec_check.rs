use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const TOOLCHAIN_PATH: &str = "docs/specs/tla-toolchain.toml";
const MAIN_SPEC: &str = "docs/specs/RaftElection.tla";
const CANARY_SPEC: &str = "docs/specs/RaftElectionCanary.tla";
const README: &str = "docs/specs/README.md";

const TRACEABILITY: &[(&str, &str, &str, &str)] = &[
    (
        "AtMostOneLeaderPerTerm",
        "HC-RAFT-INV-ONE-LEADER-PER-TERM",
        "crates/hydracache-cluster-testkit/tests/invariants.rs",
        "invariant_catalog_flags_each_seeded_violation",
    ),
    (
        "TermsNeverDecrease",
        "HC-RAFT-INV-TERM-MONOTONIC",
        "crates/hydracache-cluster-raft/tests/raft_corpus_vectors.rs",
        "raft_corpus_stale_term_install_snapshot_is_rejected",
    ),
    (
        "CommittedIndexNeverDecreases",
        "HC-RAFT-INV-COMMIT-MONOTONIC",
        "crates/hydracache-cluster-raft/tests/leadership_handoff.rs",
        "leadership_handoff_preserves_committed_prefix_and_exactly_once_proposal_outcome",
    ),
    (
        "CommittedPrefixNeverConflicts",
        "HC-RAFT-INV-COMMITTED-PREFIX",
        "crates/hydracache-cluster-raft/tests/raft_message_filter.rs",
        "reordered_appends_do_not_corrupt_committed_prefix",
    ),
    (
        "AppliedNeverExceedsCommit",
        "HC-RAFT-INV-APPLIED-LE-COMMIT",
        "crates/hydracache-cluster-raft/tests/model_check.rs",
        "bounded_model_check_membership_and_commit_invariants_hold_for_up_to_4_nodes",
    ),
    (
        "SnapshotIdentityMatches",
        "HC-RAFT-INV-SNAPSHOT-IDENTITY",
        "crates/hydracache-cluster-raft/tests/snapshot_corruption.rs",
        "misdirected_snapshot_with_valid_checksum_is_rejected_on_identity_mismatch",
    ),
    (
        "SnapshotIndexNeverDecreases",
        "HC-RAFT-INV-SNAPSHOT-MONOTONIC",
        "crates/hydracache-cluster-raft/tests/snapshot_delivery_chaos.rs",
        "newer_snapshot_then_delayed_older_snapshot_never_rolls_state_back",
    ),
    (
        "RemovedNodeCannotRegainAuthority",
        "HC-RAFT-INV-REMOVED-NODE-NO-AUTHORITY",
        "crates/hydracache-cluster-raft/tests/raft_message_filter.rs",
        "leader_promotion_does_not_resurrect_draining_member",
    ),
];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Toolchain {
    version: String,
    url: String,
    sha256: String,
    minimum_java: u32,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum Scope {
    Fast,
    Canary,
    Nightly,
}

impl Scope {
    fn parse(value: &str) -> Result<Self, Box<dyn Error>> {
        match value {
            "fast" => Ok(Self::Fast),
            "canary" => Ok(Self::Canary),
            "nightly" => Ok(Self::Nightly),
            _ => Err(format!("unsupported raft spec scope: {value}").into()),
        }
    }

    fn config(self) -> &'static str {
        match self {
            Self::Fast => "raft-election-fast.cfg",
            Self::Canary => "raft-election-canary.cfg",
            Self::Nightly => "raft-election-nightly.cfg",
        }
    }

    fn module(self) -> &'static str {
        match self {
            Self::Canary => "RaftElectionCanary.tla",
            Self::Fast | Self::Nightly => "RaftElection.tla",
        }
    }

    fn artifact(self) -> &'static str {
        match self {
            Self::Fast => "raft-spec-fast.json",
            Self::Canary => "raft-spec-canary.json",
            Self::Nightly => "raft-spec-nightly.json",
        }
    }
}

#[derive(Debug, Serialize)]
struct SpecEvidence<'a> {
    schema_version: u32,
    release: &'a str,
    scope: Scope,
    source_commit: String,
    tool_version: &'a str,
    tool_sha256: &'a str,
    java: String,
    expected_canary_failure: bool,
    states_generated: Option<u64>,
    distinct_states: Option<u64>,
    stdout_sha256: String,
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let options = Options::parse(args)?;
    let problems = structural_check(&options.root)?;
    if !problems.is_empty() {
        for problem in &problems {
            eprintln!("raft-spec-check: {problem}");
        }
        return Err(format!(
            "raft-spec-check found {} structural problem(s)",
            problems.len()
        )
        .into());
    }
    println!(
        "raft-spec-check: structural OK ({} invariants)",
        TRACEABILITY.len()
    );
    if options.structural_only {
        return Ok(());
    }
    run_tlc(
        &options.root,
        options.scope.expect("scope required"),
        options.jar.as_deref(),
    )
}

pub fn structural_check(root: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let mut problems = Vec::new();
    let toolchain: Toolchain = toml::from_str(&fs::read_to_string(root.join(TOOLCHAIN_PATH))?)?;
    if toolchain.version != "1.7.4"
        || toolchain.url
            != "https://github.com/tlaplus/tlaplus/releases/download/v1.7.4/tla2tools.jar"
        || toolchain.sha256 != "936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88"
        || toolchain.minimum_java != 17
    {
        problems.push("TLA+ toolchain pin differs from the reviewed 1.7.4 surface".to_owned());
    }
    let main = fs::read_to_string(root.join(MAIN_SPEC))?;
    let canary = fs::read_to_string(root.join(CANARY_SPEC))?;
    let readme = fs::read_to_string(root.join(README))?;
    let catalog =
        fs::read_to_string(root.join("crates/hydracache-cluster-testkit/src/invariants.rs"))?;
    if main.contains("RaftElectionCanary") || main.contains("UnsafeSecondLeader") {
        problems.push("main model imports or embeds the negative canary".to_owned());
    }
    if !canary.contains("UnsafeSecondLeader") || !canary.contains("AtMostOneLeaderPerTerm") {
        problems
            .push("negative model does not expose its unsafe transition and invariant".to_owned());
    }
    for (invariant, id, source, function) in TRACEABILITY {
        if !main.contains(&format!("{invariant} ==")) {
            problems.push(format!("main model is missing invariant {invariant}"));
        }
        if !catalog.contains(id) {
            problems.push(format!("Rust invariant catalog is missing {id}"));
        }
        if !readme.contains(invariant)
            || !readme.contains(id)
            || !readme.contains(source)
            || !readme.contains(function)
        {
            problems.push(format!("traceability row is incomplete for {invariant}"));
        }
        let implementation = fs::read_to_string(root.join(source))?;
        if !implementation.contains(&format!("fn {function}")) {
            problems.push(format!(
                "mapped Rust test {source}::{function} does not exist"
            ));
        }
    }
    for config in [
        "docs/specs/raft-election-fast.cfg",
        "docs/specs/raft-election-nightly.cfg",
        "docs/specs/raft-election-canary.cfg",
    ] {
        let text = fs::read_to_string(root.join(config))?;
        if !text.contains("SPECIFICATION Spec") || !text.contains("AtMostOneLeaderPerTerm") {
            problems.push(format!(
                "{config} does not execute Spec and the leader invariant"
            ));
        }
    }
    Ok(problems)
}

fn run_tlc(root: &Path, scope: Scope, explicit_jar: Option<&Path>) -> Result<(), Box<dyn Error>> {
    let toolchain: Toolchain = toml::from_str(&fs::read_to_string(root.join(TOOLCHAIN_PATH))?)?;
    let jar = explicit_jar
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HYDRACACHE_TLA2TOOLS_JAR").map(PathBuf::from))
        .unwrap_or_else(|| {
            root.join("target/tools/tla2tools")
                .join(&toolchain.version)
                .join("tla2tools.jar")
        });
    let require = std::env::var("HYDRACACHE_REQUIRE_TLC").as_deref() == Ok("1");
    if !jar.is_file() {
        return unavailable(
            require,
            format!("pinned tla2tools.jar is missing at {}", jar.display()),
        );
    }
    let actual_sha = hex_digest(&fs::read(&jar)?);
    if actual_sha != toolchain.sha256 {
        return Err(format!(
            "tla2tools.jar checksum mismatch: expected {}, got {actual_sha}",
            toolchain.sha256
        )
        .into());
    }
    let java_version = match Command::new("java").arg("-version").output() {
        Ok(output) if output.status.success() => {
            format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            )
        }
        Ok(output) => {
            return unavailable(
                require,
                format!(
                    "java -version failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ),
            )
        }
        Err(error) => return unavailable(require, format!("java is unavailable: {error}")),
    };
    let major = parse_java_major(&java_version).ok_or("cannot parse java major version")?;
    if major < toolchain.minimum_java {
        return unavailable(
            require,
            format!(
                "Java {major} is older than required {}",
                toolchain.minimum_java
            ),
        );
    }
    let output = Command::new("java")
        .args(["-XX:+UseParallelGC", "-jar"])
        .arg(&jar)
        .args(["-workers", "2", "-metadir"])
        .arg(
            root.join("target/tlc-states")
                .join(format!("{scope:?}").to_ascii_lowercase()),
        )
        .args(["-config", scope.config(), scope.module()])
        .current_dir(root.join("docs/specs"))
        .output()?;
    let stdout = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let expected_canary = matches!(scope, Scope::Canary);
    let valid = if expected_canary {
        !output.status.success()
            && stdout.contains("Invariant AtMostOneLeaderPerTerm is violated")
            && stdout.contains("UnsafeSecondLeader")
    } else {
        output.status.success()
            && stdout.contains("Model checking completed. No error has been found")
    };
    if !valid {
        eprintln!("{stdout}");
        return Err(format!("TLC produced an unexpected result for {scope:?}").into());
    }
    let artifact = SpecEvidence {
        schema_version: 1,
        release: "0.64",
        scope,
        source_commit: git_commit(root),
        tool_version: &toolchain.version,
        tool_sha256: &toolchain.sha256,
        java: java_version.lines().next().unwrap_or("unknown").to_owned(),
        expected_canary_failure: expected_canary,
        states_generated: parse_counter(&stdout, "states generated"),
        distinct_states: parse_counter(&stdout, "distinct states found"),
        stdout_sha256: hex_digest(stdout.as_bytes()),
    };
    let artifact_path = root
        .join("target/test-evidence/0.64")
        .join(scope.artifact());
    fs::create_dir_all(artifact_path.parent().expect("artifact parent"))?;
    fs::write(&artifact_path, serde_json::to_vec_pretty(&artifact)?)?;
    println!(
        "raft-spec-check: {scope:?} OK artifact={}",
        artifact_path.display()
    );
    Ok(())
}

fn unavailable(require: bool, message: String) -> Result<(), Box<dyn Error>> {
    if require {
        Err(message.into())
    } else {
        println!("raft-spec-check: SKIP {message}; set HYDRACACHE_REQUIRE_TLC=1 to fail");
        Ok(())
    }
}

fn parse_java_major(version: &str) -> Option<u32> {
    let quoted = version.split('"').nth(1)?;
    let first = quoted.split('.').next()?.parse::<u32>().ok()?;
    if first == 1 {
        quoted.split('.').nth(1)?.parse().ok()
    } else {
        Some(first)
    }
}

fn parse_counter(output: &str, suffix: &str) -> Option<u64> {
    output.lines().rev().find_map(|line| {
        let prefix = line.split(suffix).next()?;
        prefix
            .split_whitespace()
            .last()?
            .replace(',', "")
            .parse()
            .ok()
    })
}

fn hex_digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn git_commit(root: &Path) -> String {
    Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_owned())
        .unwrap_or_else(|| "unknown".to_owned())
}

struct Options {
    root: PathBuf,
    structural_only: bool,
    scope: Option<Scope>,
    jar: Option<PathBuf>,
}

impl Options {
    fn parse(args: Vec<String>) -> Result<Self, Box<dyn Error>> {
        let mut root = crate::doc_check::find_repo_root()?;
        let mut structural_only = false;
        let mut scope = None;
        let mut jar = None;
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--root" => root = PathBuf::from(args.next().ok_or("--root requires a path")?),
                "--structural" => structural_only = true,
                "--scope" => {
                    scope = Some(Scope::parse(
                        &args.next().ok_or("--scope requires a value")?,
                    )?)
                }
                "--jar" => jar = Some(PathBuf::from(args.next().ok_or("--jar requires a path")?)),
                other => return Err(format!("unknown raft-spec-check argument: {other}").into()),
            }
        }
        if structural_only == scope.is_some() {
            return Err("raft-spec-check requires exactly one of --structural or --scope".into());
        }
        Ok(Self {
            root,
            structural_only,
            scope,
            jar,
        })
    }
}
