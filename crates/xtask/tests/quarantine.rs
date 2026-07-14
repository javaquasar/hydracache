use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

fn temp_root(name: &str) -> PathBuf {
    let serial = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "hydracache-quarantine-{name}-{}-{serial}",
        std::process::id()
    ));
    fs::create_dir_all(root.join("docs/testing")).unwrap();
    root
}

fn write_gate_registry(root: &Path, ship_mandatory: bool) {
    let registry = xtask::gated_tests::GatedTestRegistry {
        schema_version: 1,
        release: "0.64.0".to_owned(),
        gate: vec![xtask::gated_tests::GateEntry {
            id: "ignored.pkg.target.test".to_owned(),
            kind: xtask::gated_tests::GateKind::IgnoredTest,
            source: "crates/pkg/tests/target.rs".to_owned(),
            package: "pkg".to_owned(),
            target: "target".to_owned(),
            test: "test".to_owned(),
            cfg: String::new(),
            env: String::new(),
            reason: "temporarily flaky".to_owned(),
            tier: xtask::gated_tests::GateTier::Nightly,
            required_features: vec![],
            required_env: vec![],
            required_tools: vec![],
            timeout_seconds: 60,
            owner_release: "0.64.0".to_owned(),
            ship_mandatory,
            artifacts: vec![],
            ci: xtask::gated_tests::CiRegistration {
                workflow: ".github/workflows/ci.yml".to_owned(),
                job: "test".to_owned(),
                step: "test".to_owned(),
            },
            command: command(),
        }],
    };
    fs::write(
        root.join(xtask::gated_tests::REGISTRY_PATH),
        toml::to_string_pretty(&registry).unwrap(),
    )
    .unwrap();
}

fn command() -> xtask::gated_tests::CommandSpec {
    xtask::gated_tests::CommandSpec {
        program: "cargo".to_owned(),
        args: vec!["test".to_owned()],
        env: BTreeMap::new(),
        cwd: ".".to_owned(),
        platform: "any".to_owned(),
    }
}

fn timestamp(value: &str) -> OffsetDateTime {
    OffsetDateTime::parse(value, &Rfc3339).unwrap()
}

#[test]
fn empty_quarantine_registry_is_valid() {
    let root = temp_root("empty");
    write_gate_registry(&root, false);
    fs::write(
        root.join(xtask::quarantine::QUARANTINE_PATH),
        "schema_version = 1\nrelease = \"0.64.0\"\n",
    )
    .unwrap();

    let report =
        xtask::quarantine::check_at(&root, "0.64", timestamp("2026-07-14T12:00:00Z")).unwrap();
    assert!(report.problems.is_empty(), "{:?}", report.problems);
    assert!(report.active.is_empty());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn quarantine_check_rejects_overdue_or_incomplete_entries_and_ship_rejects_all_active_entries() {
    let root = temp_root("invalid");
    write_gate_registry(&root, true);
    fs::write(
        root.join(xtask::quarantine::QUARANTINE_PATH),
        r#"
schema_version = 1
release = "0.64.0"

[[quarantine]]
gate_id = "ignored.pkg.target.test"
issue = "HC-640"
owner = ""
reason = "reproducing a scheduler-sensitive failure"
created_at = "2026-07-13T10:00:00Z"
expiry_at = "2026-07-14T10:00:00Z"
[quarantine.replay]
program = "cargo"
args = ["test"]
cwd = "."
platform = "any"
[quarantine.replay.env]
"#,
    )
    .unwrap();
    let report =
        xtask::quarantine::check_at(&root, "0.64", timestamp("2026-07-14T12:00:00Z")).unwrap();
    assert!(report
        .problems
        .iter()
        .any(|problem| problem.contains("empty owner")));
    assert!(report
        .problems
        .iter()
        .any(|problem| problem.contains("expired")));

    fs::write(
        root.join(xtask::quarantine::QUARANTINE_PATH),
        r#"
schema_version = 1
release = "0.64.0"

[[quarantine]]
gate_id = "ignored.pkg.target.test"
issue = "HC-640"
owner = "release-engineering"
reason = "bounded investigation"
created_at = "2026-07-14T10:00:00Z"
expiry_at = "2026-07-15T10:00:00Z"
[quarantine.replay]
program = "cargo"
args = ["test"]
cwd = "."
platform = "any"
[quarantine.replay.env]
"#,
    )
    .unwrap();
    let report =
        xtask::quarantine::check_at(&root, "0.64", timestamp("2026-07-14T12:00:00Z")).unwrap();
    assert!(report.problems.is_empty(), "{:?}", report.problems);
    assert_eq!(report.active.len(), 1);
    assert!(report.active[0].ship_mandatory);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn quarantine_rejects_unknown_gates_duplicate_ids_and_windows_over_24_hours() {
    let root = temp_root("shape");
    write_gate_registry(&root, false);
    fs::write(
        root.join(xtask::quarantine::QUARANTINE_PATH),
        r#"
schema_version = 1
release = "0.64.0"

[[quarantine]]
gate_id = "unknown"
issue = "HC-1"
owner = "owner"
reason = "reason"
created_at = "2026-07-14T00:00:00Z"
expiry_at = "2026-07-15T01:00:00Z"
[quarantine.replay]
program = "cargo"
cwd = "."
platform = "any"
[quarantine.replay.env]

[[quarantine]]
gate_id = "unknown"
issue = "HC-2"
owner = "owner"
reason = "reason"
created_at = "2026-07-14T00:00:00+01:00"
expiry_at = "2026-07-14T01:00:00Z"
[quarantine.replay]
program = "cargo"
cwd = "."
platform = "any"
[quarantine.replay.env]
"#,
    )
    .unwrap();
    let report =
        xtask::quarantine::check_at(&root, "0.64", timestamp("2026-07-14T00:30:00Z")).unwrap();
    assert!(report
        .problems
        .iter()
        .any(|problem| problem.contains("unknown gate")));
    assert!(report
        .problems
        .iter()
        .any(|problem| problem.contains("duplicate")));
    assert!(report
        .problems
        .iter()
        .any(|problem| problem.contains("longer than 24 hours")));
    assert!(report
        .problems
        .iter()
        .any(|problem| problem.contains("UTC Z offset")));

    fs::remove_dir_all(root).unwrap();
}
