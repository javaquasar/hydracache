use std::fmt;

use crate::{
    ControlActionV1, ConvergenceView, ReplayScriptV1, SimConfig, SimMode, SimSnapshot, SimWorld,
    VerdictView,
};

/// Curated simulator scenario set version.
pub const SIM_SCENARIO_SET_VERSION: u16 = 1;

/// A named deterministic scenario preset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioPreset {
    /// Stable scenario name used by URLs and the demo UI.
    pub name: &'static str,
    /// Human-readable title.
    pub title: &'static str,
    /// Short operational summary.
    pub summary: &'static str,
    /// Seed used to build the scenario world.
    pub seed: u64,
    /// Expected final step after applying the script.
    pub steps: u64,
    /// Scripted actions applied to the real simulator.
    pub actions: Vec<ScenarioAction>,
    /// Expected invariant verdict.
    pub expected_verdict: ExpectedScenarioVerdict,
    /// Expected progress shape.
    pub expected_progress: ExpectedScenarioProgress,
}

/// One deterministic scenario action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScenarioAction {
    /// Enable or disable the built-in workload.
    Workload(bool),
    /// Run scheduler steps.
    Run(u64),
    /// Crash a node.
    Crash(&'static str),
    /// Restart a node.
    Restart(&'static str),
    /// Partition one directed link.
    Partition(&'static str, &'static str),
    /// Heal one directed link.
    Heal(&'static str, &'static str),
    /// Delay the next packet on one directed link.
    Delay(&'static str, &'static str, u64),
    /// Drop the next packet on one directed link.
    Drop(&'static str, &'static str),
}

/// Expected invariant verdict for a scenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpectedScenarioVerdict {
    /// The real invariant checker should hold.
    Holding,
    /// The real invariant checker should report a violation.
    Violated,
}

/// Expected progress shape for a scenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpectedScenarioProgress {
    /// Scenario should not commit any workload entry.
    NoProgress,
    /// Scenario should commit at least one workload entry.
    SomeProgress,
}

/// Completed scenario run.
#[derive(Debug, Clone)]
pub struct ScenarioRun {
    /// Preset that was applied.
    pub preset: ScenarioPreset,
    /// World after applying the preset.
    pub world: SimWorld,
    /// Snapshot after applying the preset.
    pub snapshot: SimSnapshot,
}

/// Scenario lookup/run errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScenarioError {
    /// Unknown scenario name.
    UnknownScenario { name: String },
    /// Script referenced a node or link that does not exist.
    InvalidAction { scenario: String, action: String },
}

impl fmt::Display for ScenarioError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownScenario { name } => write!(formatter, "unknown scenario '{name}'"),
            Self::InvalidAction { scenario, action } => {
                write!(
                    formatter,
                    "scenario '{scenario}' has invalid action {action}"
                )
            }
        }
    }
}

impl std::error::Error for ScenarioError {}

/// Return all curated scenario presets.
pub fn scenario_presets() -> Vec<ScenarioPreset> {
    vec![
        ScenarioPreset {
            name: "minority_partition_cannot_commit",
            title: "Minority partition cannot commit",
            summary: "A partitioned minority makes no workload progress while invariants hold.",
            seed: 5_001,
            steps: 6,
            actions: vec![
                ScenarioAction::Workload(false),
                ScenarioAction::Partition("node-0", "node-1"),
                ScenarioAction::Partition("node-0", "node-2"),
                ScenarioAction::Run(6),
            ],
            expected_verdict: ExpectedScenarioVerdict::Holding,
            expected_progress: ExpectedScenarioProgress::NoProgress,
        },
        ScenarioPreset {
            name: "leader_crash_failover_no_committed_loss",
            title: "Leader crash, no committed loss",
            summary: "A node crash and restart keeps the deterministic history valid.",
            seed: 5_002,
            steps: 12,
            actions: vec![
                ScenarioAction::Run(4),
                ScenarioAction::Crash("node-0"),
                ScenarioAction::Run(4),
                ScenarioAction::Restart("node-0"),
                ScenarioAction::Run(4),
            ],
            expected_verdict: ExpectedScenarioVerdict::Holding,
            expected_progress: ExpectedScenarioProgress::SomeProgress,
        },
        ScenarioPreset {
            name: "symmetric_partition_heal_converges",
            title: "Symmetric partition heals",
            summary: "Partitioned links heal and the latest invariant verdict remains green.",
            seed: 5_003,
            steps: 10,
            actions: vec![
                ScenarioAction::Partition("node-0", "node-1"),
                ScenarioAction::Partition("node-1", "node-0"),
                ScenarioAction::Run(4),
                ScenarioAction::Heal("node-0", "node-1"),
                ScenarioAction::Heal("node-1", "node-0"),
                ScenarioAction::Run(6),
            ],
            expected_verdict: ExpectedScenarioVerdict::Holding,
            expected_progress: ExpectedScenarioProgress::SomeProgress,
        },
        ScenarioPreset {
            name: "each_quorum_region_loss_fails_loud",
            title: "EachQuorum under region loss refuses progress",
            summary: "Region-loss posture is presented as halted progress, not silent success.",
            seed: 5_004,
            steps: 5,
            actions: vec![
                ScenarioAction::Workload(false),
                ScenarioAction::Crash("node-1"),
                ScenarioAction::Crash("node-2"),
                ScenarioAction::Run(5),
            ],
            expected_verdict: ExpectedScenarioVerdict::Holding,
            expected_progress: ExpectedScenarioProgress::NoProgress,
        },
        ScenarioPreset {
            name: "delete_vs_concurrent_write_no_resurrection",
            title: "Delete versus concurrent write",
            summary: "Delete/write stress remains inside the real invariant checker.",
            seed: 5_005,
            steps: 16,
            actions: vec![
                ScenarioAction::Run(5),
                ScenarioAction::Delay("node-0", "node-1", 250),
                ScenarioAction::Drop("node-1", "node-0"),
                ScenarioAction::Run(11),
            ],
            expected_verdict: ExpectedScenarioVerdict::Holding,
            expected_progress: ExpectedScenarioProgress::SomeProgress,
        },
    ]
}

/// Return scripted lab scenarios as replayable control scripts.
pub fn scripted_lab_catalog() -> Vec<ReplayScriptV1> {
    vec![
        ReplayScriptV1 {
            version: crate::REPLAY_SCRIPT_VERSION,
            seed: 0x5350,
            mode: SimMode::Scripted,
            scenario: Some("cold-start-formation".to_owned()),
            actions: vec![ControlActionV1::Step { at_step: 0, n: 8 }],
        },
        ReplayScriptV1 {
            version: crate::REPLAY_SCRIPT_VERSION,
            seed: 0x5351,
            mode: SimMode::Scripted,
            scenario: Some("leader-loss-reelection".to_owned()),
            actions: vec![
                ControlActionV1::Step { at_step: 0, n: 8 },
                ControlActionV1::Isolate {
                    at_step: 8,
                    node: "node-0".to_owned(),
                },
                ControlActionV1::Step { at_step: 8, n: 4 },
                ControlActionV1::Rejoin {
                    at_step: 12,
                    node: "node-0".to_owned(),
                },
            ],
        },
        ReplayScriptV1 {
            version: crate::REPLAY_SCRIPT_VERSION,
            seed: 0x5352,
            mode: SimMode::Scripted,
            scenario: Some("manual-push-convergence".to_owned()),
            actions: vec![
                ControlActionV1::Step { at_step: 0, n: 8 },
                ControlActionV1::Subscribe {
                    at_step: 8,
                    client: "client-a".to_owned(),
                    ns: "profiles".to_owned(),
                },
                ControlActionV1::PushEvent {
                    at_step: 8,
                    client: "client-a".to_owned(),
                    ns: "profiles".to_owned(),
                    key: "profile-42".to_owned(),
                    value: "fresh".to_owned(),
                },
                ControlActionV1::Step { at_step: 8, n: 2 },
            ],
        },
        ReplayScriptV1 {
            version: crate::REPLAY_SCRIPT_VERSION,
            seed: 0x5353,
            mode: SimMode::Scripted,
            scenario: Some("scale-out".to_owned()),
            actions: vec![
                ControlActionV1::Step { at_step: 0, n: 8 },
                ControlActionV1::AddNode { at_step: 8 },
            ],
        },
    ]
}

/// Run a named scenario.
pub fn run_scenario(name: &str) -> Result<ScenarioRun, ScenarioError> {
    let preset = scenario_presets()
        .into_iter()
        .find(|preset| preset.name == name)
        .ok_or_else(|| ScenarioError::UnknownScenario {
            name: name.to_owned(),
        })?;
    let mut world = SimWorld::new(preset.seed, SimConfig::default());
    for action in &preset.actions {
        apply_action(&mut world, preset.name, action)?;
    }
    let snapshot = world.snapshot();
    Ok(ScenarioRun {
        preset,
        world,
        snapshot,
    })
}

fn apply_action(
    world: &mut SimWorld,
    scenario: &str,
    action: &ScenarioAction,
) -> Result<(), ScenarioError> {
    let applied = match action {
        ScenarioAction::Workload(enabled) => {
            world.set_workload_enabled(*enabled);
            true
        }
        ScenarioAction::Run(steps) => {
            world.run(*steps);
            true
        }
        ScenarioAction::Crash(node) => world.crash_node(*node),
        ScenarioAction::Restart(node) => world.restart_node(*node),
        ScenarioAction::Partition(from, to) => world.partition_link(*from, *to),
        ScenarioAction::Heal(from, to) => world.heal_link(*from, *to),
        ScenarioAction::Delay(from, to, millis) => {
            world.delay_next_on_link_millis(*from, *to, *millis)
        }
        ScenarioAction::Drop(from, to) => world.drop_next_on_link(*from, *to),
    };
    if applied {
        Ok(())
    } else {
        Err(ScenarioError::InvalidAction {
            scenario: scenario.to_owned(),
            action: format!("{action:?}"),
        })
    }
}

/// Return whether a snapshot satisfies a preset's expected outcome.
pub fn scenario_matches_expectation(preset: &ScenarioPreset, snapshot: &SimSnapshot) -> bool {
    let verdict_matches = matches!(
        (preset.expected_verdict, &snapshot.verdict),
        (ExpectedScenarioVerdict::Holding, VerdictView::Holding)
            | (
                ExpectedScenarioVerdict::Violated,
                VerdictView::Violated { .. }
            )
    );
    let progress_matches = match preset.expected_progress {
        ExpectedScenarioProgress::NoProgress => snapshot.progress.committed_entries == 0,
        ExpectedScenarioProgress::SomeProgress => snapshot.progress.committed_entries > 0,
    };
    verdict_matches
        && progress_matches
        && snapshot.progress.convergence == ConvergenceView::Converged
}
