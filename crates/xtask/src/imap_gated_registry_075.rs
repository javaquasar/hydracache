//! Closed discovery metadata for prepared 0.75 tests, executed by the 0.75 capture.
use crate::gated_tests::{GateEntry, GateKind, GateTier, GatedTestRegistry};
use std::{collections::BTreeSet, error::Error, path::Path};

pub const REGISTRY_PATH: &str = "docs/testing/imap/0.75/gated-test-registry.toml";
pub fn load(root: &Path) -> Result<Vec<GateEntry>, Box<dyn Error>> {
    if !root.join(REGISTRY_PATH).try_exists()? {
        return Ok(Vec::new());
    }
    let bytes = crate::imap_qualification_075::read_relative(root, REGISTRY_PATH, 64 * 1024)?;
    let registry: GatedTestRegistry = toml::from_str(std::str::from_utf8(&bytes)?)?;
    if registry.schema_version != 1 || registry.release != "0.75.0" || registry.gate.len() != 4 {
        return Err("0.75 gated-test extension schema/release/complete set differs".into());
    }
    let mut ids = BTreeSet::new();
    for gate in &registry.gate {
        let (target, test, environment, timeout, java) = match gate.id.as_str() {
            "ignored.xtask.imap075.prepared-native-daemon-smoke" => (
                "imap_soak_075",
                "actual_prepared_native_and_daemon_segments_preserve_workers_and_verify_rounds",
                None,
                180,
                true,
            ),
            "ignored.xtask.imap075.actual-cargo-closure" => (
                "imap_sbom_075",
                "actual_host_filtered_cargo_graph_replays_compiler_feature_and_dependency_closure",
                Some("IMAP075_CARGO_FIXTURE"),
                180,
                false,
            ),
            "ignored.xtask.imap075.actual-provenance-mutations" => (
                "imap_sbom_075",
                "actual_provenance_reconstruction_rejects_archive_and_policy_substitution",
                Some("IMAP075_PROVENANCE_FIXTURE"),
                600,
                true,
            ),
            "ignored.xtask.imap075.actual-soak-archive-mutations" => (
                "imap_soak_075",
                "actual_saved_soak_and_restart_reject_substituted_and_partial_archives",
                Some("IMAP075_SOAK_FIXTURE"),
                180,
                true,
            ),
            _ => return Err("foreign 0.75 gated-test extension entry".into()),
        };
        let expected_env = environment
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>();
        let expected_tools = if java {
            vec!["cargo", "java"]
        } else {
            vec!["cargo"]
        };
        if !ids.insert(gate.id.clone())
            || gate.kind != GateKind::IgnoredTest
            || gate.package != "xtask"
            || gate.target != target
            || gate.test != test
            || gate.source != format!("crates/xtask/tests/{target}.rs")
            || gate.owner_release != "0.75.0"
            || !gate.ship_mandatory
            || gate.tier != GateTier::Fast
            || gate.timeout_seconds != timeout
            || gate.reason.trim().is_empty()
            || !gate.cfg.is_empty()
            || !gate.env.is_empty()
            || !gate.artifacts.is_empty()
            || !gate.required_features.is_empty()
            || gate.required_env != expected_env
            || gate.required_tools != expected_tools
            || gate.command.program != "cargo"
            || gate.command.cwd != "."
            || gate.command.platform != "any"
            || !gate.command.env.is_empty()
            || gate.command.args
                != [
                    "test",
                    "-p",
                    "xtask",
                    "--test",
                    target,
                    "--locked",
                    "--",
                    test,
                    "--ignored",
                    "--exact",
                ]
            || gate.ci.workflow != ".github/workflows/imap-075.yml"
            || gate.ci.job != "qualification"
            || gate.ci.step != "Run bounded local proofs"
        {
            return Err("0.75 gated-test extension owner/command/budget/wiring differs".into());
        }
    }
    let problems = crate::release_governance::ci_wiring_problems(root, &registry.gate)?;
    if !problems.is_empty() {
        return Err(
            format!("0.75 gated-test extension workflow wiring differs: {problems:?}").into(),
        );
    }
    Ok(registry.gate)
}
