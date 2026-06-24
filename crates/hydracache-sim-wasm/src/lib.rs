//! Browser bindings for the real deterministic HydraCache simulator.

use hydracache_sim::{InvariantReport, SimConfig, SimOutcome, SimWorld};
use serde::Serialize;
use wasm_bindgen::prelude::*;

const SNAPSHOT_SCHEMA_VERSION: u16 = 1;
const VERDICT_SCHEMA_VERSION: u16 = 1;

/// wasm-bindgen handle around a real [`SimWorld`].
#[wasm_bindgen]
pub struct SimHandle {
    world: SimWorld,
}

#[wasm_bindgen]
impl SimHandle {
    /// Create a deterministic simulator from a seed.
    #[wasm_bindgen(constructor)]
    pub fn new(seed: u64) -> Self {
        Self {
            world: SimWorld::new(seed, SimConfig::default()),
        }
    }

    /// Advance the real simulator by one scheduler step.
    pub fn step(&mut self) {
        self.world.step();
    }

    /// Advance the real simulator by `steps` scheduler steps.
    pub fn run(&mut self, steps: u64) {
        self.world.run(steps);
    }

    /// Return the seed backing this simulator.
    pub fn seed(&self) -> u64 {
        self.world.outcome().seed
    }

    /// Serialize the current simulator state as JSON.
    pub fn snapshot_json(&self) -> String {
        snapshot_json(&self.world)
    }

    /// Serialize the latest invariant verdict as JSON.
    pub fn verdict_json(&self) -> String {
        verdict_json(&self.world)
    }
}

/// Build the same W1 snapshot JSON used by the browser binding.
pub fn snapshot_json(world: &SimWorld) -> String {
    let outcome = world.outcome();
    to_json(&SnapshotDto {
        schema_version: SNAPSHOT_SCHEMA_VERSION,
        engine: "hydracache-sim",
        seed: outcome.seed,
        steps: outcome.steps,
        logical_time_millis: world.now().as_millis(),
        accepted_ops: outcome.accepted_ops,
        delivered_messages: outcome.delivered_messages,
        history_hash: outcome.history_hash,
        invariant_violations: outcome.invariant_violations,
    })
}

/// Build the same W1 verdict JSON used by the browser binding.
pub fn verdict_json(world: &SimWorld) -> String {
    let outcome = world.outcome();
    to_json(&verdict_dto(outcome, world.invariant_report()))
}

fn verdict_dto(outcome: SimOutcome, report: &InvariantReport) -> VerdictDto<'_> {
    let status = if report.violations.is_empty() {
        "ok"
    } else {
        "violation"
    };
    VerdictDto {
        schema_version: VERDICT_SCHEMA_VERSION,
        engine: "hydracache-sim",
        seed: outcome.seed,
        steps: outcome.steps,
        status,
        checked: report.checked,
        violations: report
            .violations
            .iter()
            .map(|violation| ViolationDto {
                name: violation.name,
                message: violation.message.as_str(),
            })
            .collect(),
    }
}

fn to_json(value: &impl Serialize) -> String {
    serde_json::to_string(value).expect("simulator DTO serialization is infallible")
}

#[derive(Debug, Serialize)]
struct SnapshotDto<'a> {
    schema_version: u16,
    engine: &'a str,
    seed: u64,
    steps: u64,
    logical_time_millis: u64,
    accepted_ops: u64,
    delivered_messages: u64,
    history_hash: u64,
    invariant_violations: usize,
}

#[derive(Debug, Serialize)]
struct VerdictDto<'a> {
    schema_version: u16,
    engine: &'a str,
    seed: u64,
    steps: u64,
    status: &'a str,
    checked: usize,
    violations: Vec<ViolationDto<'a>>,
}

#[derive(Debug, Serialize)]
struct ViolationDto<'a> {
    name: &'a str,
    message: &'a str,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct Snapshot {
        schema_version: u16,
        engine: String,
        seed: u64,
        steps: u64,
        logical_time_millis: u64,
        accepted_ops: u64,
        delivered_messages: u64,
        history_hash: u64,
        invariant_violations: usize,
    }

    #[derive(Debug, Deserialize)]
    struct Verdict {
        schema_version: u16,
        engine: String,
        seed: u64,
        steps: u64,
        status: String,
        checked: usize,
        violations: Vec<serde_json::Value>,
    }

    #[test]
    fn snapshot_json_reports_real_simworld_outcome() {
        let mut world = SimWorld::new(42, SimConfig::default());
        let outcome = world.run(8);

        let snapshot: Snapshot =
            serde_json::from_str(&snapshot_json(&world)).expect("valid snapshot json");

        assert_eq!(snapshot.schema_version, SNAPSHOT_SCHEMA_VERSION);
        assert_eq!(snapshot.engine, "hydracache-sim");
        assert_eq!(snapshot.seed, outcome.seed);
        assert_eq!(snapshot.steps, outcome.steps);
        assert_eq!(snapshot.logical_time_millis, world.now().as_millis());
        assert_eq!(snapshot.accepted_ops, outcome.accepted_ops);
        assert_eq!(snapshot.delivered_messages, outcome.delivered_messages);
        assert_eq!(snapshot.history_hash, outcome.history_hash);
        assert_eq!(snapshot.invariant_violations, outcome.invariant_violations);
    }

    #[test]
    fn verdict_json_reports_invariant_checker_result() {
        let mut handle = SimHandle::new(7);
        handle.run(3);

        let verdict: Verdict =
            serde_json::from_str(&handle.verdict_json()).expect("valid verdict json");

        assert_eq!(verdict.schema_version, VERDICT_SCHEMA_VERSION);
        assert_eq!(verdict.engine, "hydracache-sim");
        assert_eq!(verdict.seed, 7);
        assert_eq!(verdict.steps, 3);
        assert_eq!(verdict.status, "ok");
        assert!(verdict.checked > 0);
        assert!(verdict.violations.is_empty());
    }
}
