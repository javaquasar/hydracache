use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime, UtcOffset};

use crate::doc_check;
use crate::gated_tests::{self, CommandSpec};

pub const QUARANTINE_PATH: &str = "docs/testing/test-quarantine.toml";
const MAX_QUARANTINE: Duration = Duration::hours(24);

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QuarantineRegistry {
    pub schema_version: u32,
    pub release: String,
    #[serde(default)]
    pub quarantine: Vec<QuarantineEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QuarantineEntry {
    pub gate_id: String,
    pub issue: String,
    pub owner: String,
    pub reason: String,
    pub created_at: String,
    pub expiry_at: String,
    pub replay: CommandSpec,
}

#[derive(Debug, Clone)]
pub struct ActiveQuarantine {
    pub gate_id: String,
    pub ship_mandatory: bool,
    pub expiry_at: OffsetDateTime,
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let (root, release) = parse_args(args)?;
    let now = OffsetDateTime::now_utc();
    let report = check_at(&root, &release, now)?;

    if report.problems.is_empty() {
        println!(
            "quarantine-check: OK ({} active, {} ship-mandatory)",
            report.active.len(),
            report
                .active
                .iter()
                .filter(|entry| entry.ship_mandatory)
                .count()
        );
        Ok(())
    } else {
        for problem in &report.problems {
            eprintln!("quarantine-check: {problem}");
        }
        Err(format!(
            "quarantine-check found {} problem(s)",
            report.problems.len()
        )
        .into())
    }
}

#[derive(Debug, Default)]
pub struct QuarantineReport {
    pub active: Vec<ActiveQuarantine>,
    pub problems: Vec<String>,
}

pub fn check_at(
    root: &Path,
    release: &str,
    now: OffsetDateTime,
) -> Result<QuarantineReport, Box<dyn Error>> {
    let registry = load_registry(root)?;
    let gates = gated_tests::load_registry(root)?;
    Ok(validate_at(&registry, &gates.gate, release, now))
}

pub fn load_registry(root: &Path) -> Result<QuarantineRegistry, Box<dyn Error>> {
    let path = root.join(QUARANTINE_PATH);
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("reading {}: {error}", path.display()))?;
    toml::from_str(&text).map_err(|error| format!("parsing {QUARANTINE_PATH}: {error}").into())
}

fn validate_at(
    registry: &QuarantineRegistry,
    gates: &[gated_tests::GateEntry],
    release: &str,
    now: OffsetDateTime,
) -> QuarantineReport {
    let mut report = QuarantineReport::default();
    if registry.schema_version != 1 {
        report.problems.push(format!(
            "unsupported schema_version {}; expected 1",
            registry.schema_version
        ));
    }
    if !release_matches(&registry.release, release) {
        report.problems.push(format!(
            "registry release {} does not match requested release {release}",
            registry.release
        ));
    }

    let gate_by_id: BTreeMap<_, _> = gates.iter().map(|gate| (gate.id.as_str(), gate)).collect();
    let mut seen = BTreeSet::new();
    for entry in &registry.quarantine {
        let label = if entry.gate_id.is_empty() {
            "<empty>"
        } else {
            &entry.gate_id
        };
        if !seen.insert(entry.gate_id.as_str()) {
            report
                .problems
                .push(format!("duplicate quarantine gate_id {label}"));
        }
        let gate = gate_by_id.get(entry.gate_id.as_str());
        if gate.is_none() {
            report
                .problems
                .push(format!("quarantine {label} references an unknown gate"));
        }

        for (field, value) in [
            ("issue", entry.issue.as_str()),
            ("owner", entry.owner.as_str()),
            ("reason", entry.reason.as_str()),
            ("replay.program", entry.replay.program.as_str()),
            ("replay.cwd", entry.replay.cwd.as_str()),
        ] {
            if value.trim().is_empty() {
                report
                    .problems
                    .push(format!("quarantine {label} has empty {field}"));
            }
        }

        let created = parse_utc(&entry.created_at, label, "created_at", &mut report.problems);
        let expiry = parse_utc(&entry.expiry_at, label, "expiry_at", &mut report.problems);
        let (Some(created), Some(expiry)) = (created, expiry) else {
            continue;
        };
        if expiry <= created {
            report.problems.push(format!(
                "quarantine {label} expiry_at must be later than created_at"
            ));
        } else if expiry - created > MAX_QUARANTINE {
            report
                .problems
                .push(format!("quarantine {label} lasts longer than 24 hours"));
        }
        if expiry <= now {
            report
                .problems
                .push(format!("quarantine {label} expired at {}", entry.expiry_at));
        } else if let Some(gate) = gate {
            report.active.push(ActiveQuarantine {
                gate_id: entry.gate_id.clone(),
                ship_mandatory: gate.ship_mandatory,
                expiry_at: expiry,
            });
        }
    }
    report
}

fn parse_utc(
    value: &str,
    gate_id: &str,
    field: &str,
    problems: &mut Vec<String>,
) -> Option<OffsetDateTime> {
    match OffsetDateTime::parse(value, &Rfc3339) {
        Ok(timestamp) if timestamp.offset() == UtcOffset::UTC => Some(timestamp),
        Ok(_) => {
            problems.push(format!(
                "quarantine {gate_id} {field} must use the UTC Z offset"
            ));
            None
        }
        Err(error) => {
            problems.push(format!(
                "quarantine {gate_id} has invalid RFC3339 {field}: {error}"
            ));
            None
        }
    }
}

fn release_matches(document_release: &str, requested: &str) -> bool {
    document_release == requested
        || document_release
            .strip_suffix(".0")
            .is_some_and(|release| release == requested)
}

fn parse_args(args: Vec<String>) -> Result<(PathBuf, String), Box<dyn Error>> {
    let mut root = doc_check::find_repo_root()?;
    let mut release = None;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => root = PathBuf::from(args.next().ok_or("--root requires a path")?),
            "--release" => release = Some(args.next().ok_or("--release requires a value")?),
            other => return Err(format!("unknown quarantine-check argument: {other}").into()),
        }
    }
    Ok((root, release.ok_or("quarantine-check requires --release")?))
}
