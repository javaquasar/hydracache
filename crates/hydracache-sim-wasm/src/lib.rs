//! Browser bindings for the real deterministic HydraCache simulator.

use hydracache_sim::{ControlActionV1, SimConfig, SimWorld};
use wasm_bindgen::prelude::*;

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

    /// Enable or disable the built-in workload generator (replay-recorded).
    pub fn set_workload_enabled(&mut self, enabled: bool) -> Result<(), JsValue> {
        let at_step = self.world.outcome().steps;
        self.control(ControlActionV1::Workload { at_step, enabled })
    }

    /// Set the visible lab mode.
    pub fn set_mode(&mut self, mode: String) -> Result<(), JsValue> {
        let mode = parse_mode(&mode)?;
        self.world
            .apply_control_action(hydracache_sim::ControlActionV1::ModeChange {
                at_step: self.world.outcome().steps,
                mode,
            })
            .map_err(|error| JsValue::from_str(&error.to_string()))
    }

    /// Crash a simulator node (replay-recorded).
    pub fn crash_node(&mut self, node_id: String) -> Result<(), JsValue> {
        let at_step = self.world.outcome().steps;
        self.control(ControlActionV1::Crash {
            at_step,
            node: node_id,
        })
    }

    /// Restart a crashed simulator node (replay-recorded).
    pub fn restart_node(&mut self, node_id: String) -> Result<(), JsValue> {
        let at_step = self.world.outcome().steps;
        self.control(ControlActionV1::Restart {
            at_step,
            node: node_id,
        })
    }

    /// Apply an interactive link action (replay-recorded).
    pub fn inject(
        &mut self,
        action: String,
        from: String,
        to: String,
        delay_millis: u64,
    ) -> Result<(), JsValue> {
        let at_step = self.world.outcome().steps;
        let action = match action.as_str() {
            "partition" => ControlActionV1::Partition { at_step, from, to },
            "heal" => ControlActionV1::Heal { at_step, from, to },
            "drop" => ControlActionV1::DropNext { at_step, from, to },
            "delay" => ControlActionV1::DelayNext {
                at_step,
                from,
                to,
                millis: delay_millis,
            },
            other => return Err(JsValue::from_str(&format!("unknown link action '{other}'"))),
        };
        self.control(action)
    }

    /// Isolate one simulator node from all peers.
    pub fn isolate_node(&mut self, node_id: String) -> Result<(), JsValue> {
        let at_step = self.world.outcome().steps;
        self.world
            .apply_control_action(hydracache_sim::ControlActionV1::Isolate {
                at_step,
                node: node_id,
            })
            .map_err(|error| JsValue::from_str(&error.to_string()))
    }

    /// Rejoin one isolated simulator node.
    pub fn rejoin_node(&mut self, node_id: String) -> Result<(), JsValue> {
        let at_step = self.world.outcome().steps;
        self.world
            .apply_control_action(hydracache_sim::ControlActionV1::Rejoin {
                at_step,
                node: node_id,
            })
            .map_err(|error| JsValue::from_str(&error.to_string()))
    }

    /// Disable one simulator node.
    pub fn disable_node(&mut self, node_id: String) -> Result<(), JsValue> {
        let at_step = self.world.outcome().steps;
        self.world
            .apply_control_action(hydracache_sim::ControlActionV1::Disable {
                at_step,
                node: node_id,
            })
            .map_err(|error| JsValue::from_str(&error.to_string()))
    }

    /// Enable one simulator node.
    pub fn enable_node(&mut self, node_id: String) -> Result<(), JsValue> {
        let at_step = self.world.outcome().steps;
        self.world
            .apply_control_action(hydracache_sim::ControlActionV1::Enable {
                at_step,
                node: node_id,
            })
            .map_err(|error| JsValue::from_str(&error.to_string()))
    }

    /// Add one deterministic simulator node.
    pub fn add_node(&mut self) -> Result<(), JsValue> {
        let at_step = self.world.outcome().steps;
        self.world
            .apply_control_action(hydracache_sim::ControlActionV1::AddNode { at_step })
            .map_err(|error| JsValue::from_str(&error.to_string()))
    }

    /// Subscribe a manual-mode client to namespace cache events.
    pub fn subscribe(&mut self, client: String, namespace: String) -> Result<(), JsValue> {
        let at_step = self.world.outcome().steps;
        self.world
            .apply_control_action(hydracache_sim::ControlActionV1::Subscribe {
                at_step,
                client,
                ns: namespace,
            })
            .map_err(|error| JsValue::from_str(&error.to_string()))
    }

    /// Register a manual-mode writer client without writing.
    pub fn add_client(&mut self, client: String, namespace: String) -> Result<(), JsValue> {
        let at_step = self.world.outcome().steps;
        self.world
            .apply_control_action(hydracache_sim::ControlActionV1::AddClient {
                at_step,
                client,
                ns: namespace,
            })
            .map_err(|error| JsValue::from_str(&error.to_string()))
    }

    /// Remove a manual-mode writer client.
    pub fn remove_client(&mut self, client: String) -> Result<(), JsValue> {
        let at_step = self.world.outcome().steps;
        self.world
            .apply_control_action(hydracache_sim::ControlActionV1::RemoveClient { at_step, client })
            .map_err(|error| JsValue::from_str(&error.to_string()))
    }

    /// Remove a manual-mode subscriber.
    pub fn unsubscribe(&mut self, client: String, namespace: String) -> Result<(), JsValue> {
        let at_step = self.world.outcome().steps;
        self.world
            .apply_control_action(hydracache_sim::ControlActionV1::Unsubscribe {
                at_step,
                client,
                ns: namespace,
            })
            .map_err(|error| JsValue::from_str(&error.to_string()))
    }

    /// Push a manual-mode cache event through the shared control surface.
    pub fn push_event(
        &mut self,
        client: String,
        namespace: String,
        key: String,
        value: String,
    ) -> Result<(), JsValue> {
        let at_step = self.world.outcome().steps;
        self.world
            .apply_control_action(hydracache_sim::ControlActionV1::PushEvent {
                at_step,
                client,
                ns: namespace,
                key,
                value,
            })
            .map_err(|error| JsValue::from_str(&error.to_string()))
    }

    /// Replace the current world with a replay of a versioned control script.
    ///
    /// A script naming a curated scenario starts from that preset's world; the
    /// script's actions and `target_step` are then replayed. The current world
    /// is only replaced when the whole replay succeeds.
    pub fn apply_control_script_json(&mut self, script_json: String) -> Result<(), JsValue> {
        let script = hydracache_sim::ReplayScriptV1::from_json(&script_json)
            .map_err(|error| JsValue::from_str(&error.to_string()))?;
        self.world =
            hydracache_sim::replay_world(&script, |seed| SimWorld::new(seed, SimConfig::default()))
                .map_err(|error| JsValue::from_str(&error.to_string()))?;
        Ok(())
    }

    /// Replace the current world with a curated simulator scenario.
    pub fn apply_scenario(&mut self, name: String) -> Result<(), JsValue> {
        let run = hydracache_sim::run_scenario(&name)
            .map_err(|error| JsValue::from_str(&error.to_string()))?;
        self.world = run.world;
        Ok(())
    }

    /// Serialize the current canonical simulator snapshot as JSON.
    pub fn snapshot_json(&self) -> String {
        self.world.snapshot_json()
    }

    /// Serialize the replay script accumulated by the shared control surface.
    pub fn replay_script_json(&self) -> String {
        self.world.replay_script().to_json()
    }

    /// Serialize the latest canonical invariant verdict as JSON.
    pub fn verdict_json(&self) -> String {
        self.world.verdict_json()
    }
}

impl SimHandle {
    /// Apply one action through the shared, replay-recorded control surface.
    fn control(&mut self, action: ControlActionV1) -> Result<(), JsValue> {
        self.world
            .apply_control_action(action)
            .map_err(|error| JsValue::from_str(&error.to_string()))
    }
}

fn parse_mode(mode: &str) -> Result<hydracache_sim::SimMode, JsValue> {
    match mode {
        "manual" => Ok(hydracache_sim::SimMode::Manual),
        "scripted" => Ok(hydracache_sim::SimMode::Scripted),
        "mixed" => Ok(hydracache_sim::SimMode::Mixed),
        other => Err(JsValue::from_str(&format!(
            "unknown simulator mode '{other}'"
        ))),
    }
}

/// Build the same W2 snapshot JSON used by the browser binding.
pub fn snapshot_json(world: &SimWorld) -> String {
    world.snapshot_json()
}

/// Build the same W2 verdict JSON used by the browser binding.
pub fn verdict_json(world: &SimWorld) -> String {
    world.verdict_json()
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydracache_sim::{SimSnapshot, VerdictView, SIM_SNAPSHOT_SCHEMA_VERSION};

    #[test]
    fn snapshot_json_reports_canonical_sim_snapshot() {
        let mut world = SimWorld::new(42, SimConfig::default());
        world.run(8);

        let snapshot = SimSnapshot::from_json(&snapshot_json(&world)).expect("valid snapshot json");

        assert_eq!(snapshot.schema_version, SIM_SNAPSHOT_SCHEMA_VERSION);
        assert_eq!(snapshot.seed, 42);
        assert_eq!(snapshot.step, 8);
        assert_eq!(snapshot.nodes.len(), 3);
        assert_eq!(snapshot.links.len(), 6);
        assert!(matches!(snapshot.verdict, VerdictView::Holding));
    }

    #[test]
    fn wasm_default_reports_validated_sim_model() {
        let mut handle = SimHandle::new(42);
        handle.run(8);

        let snapshot = SimSnapshot::from_json(&handle.snapshot_json()).expect("valid snapshot");

        assert_eq!(snapshot.election_source, "sim-model");
        assert!(snapshot
            .election_disclosure
            .contains("not a product consensus claim"));
    }

    #[test]
    fn verdict_json_reports_canonical_verdict() {
        let mut handle = SimHandle::new(7);
        handle.run(3);

        let verdict: VerdictView =
            serde_json::from_str(&handle.verdict_json()).expect("valid verdict json");

        assert_eq!(verdict, VerdictView::Holding);
    }

    #[test]
    fn wasm_controls_update_canonical_snapshot() {
        let mut handle = SimHandle::new(9);

        handle
            .inject(
                "partition".to_owned(),
                "node-0".to_owned(),
                "node-1".to_owned(),
                0,
            )
            .expect("partition applies");
        handle
            .crash_node("node-2".to_owned())
            .expect("node crash applies");

        let snapshot = SimSnapshot::from_json(&handle.snapshot_json()).expect("valid snapshot");
        assert!(snapshot.links.iter().any(|link| link.from == "node-0"
            && link.to == "node-1"
            && link.state == hydracache_sim::LinkStateView::Partitioned));
        assert!(snapshot
            .nodes
            .iter()
            .any(|node| node.id == "node-2" && node.crashed && !node.up));
    }

    #[test]
    fn wasm_can_apply_curated_scenario() {
        let mut handle = SimHandle::new(1);
        handle
            .apply_scenario("minority_partition_cannot_commit".to_owned())
            .expect("scenario applies");

        let snapshot = SimSnapshot::from_json(&handle.snapshot_json()).expect("valid snapshot");
        assert_eq!(snapshot.seed, 5_001);
        assert_eq!(snapshot.step, 6);
        assert_eq!(snapshot.progress.committed_entries, 0);
        assert_eq!(snapshot.verdict, VerdictView::Holding);
    }

    #[test]
    fn wasm_control_actions_match_native() {
        let mut handle = SimHandle::new(0x5333);
        handle
            .set_workload_enabled(false)
            .expect("workload toggle applies");
        handle.run(8);
        handle
            .subscribe("client-a".to_owned(), "profiles".to_owned())
            .expect("subscribe applies");
        handle
            .push_event(
                "client-a".to_owned(),
                "profiles".to_owned(),
                "profile-42".to_owned(),
                "fresh".to_owned(),
            )
            .expect("push applies");
        handle.run(2);

        let snapshot = SimSnapshot::from_json(&handle.snapshot_json()).expect("valid snapshot");
        assert!(snapshot
            .subscribers
            .iter()
            .any(|subscriber| subscriber.last_event.is_some()));
        let replay = hydracache_sim::ReplayScriptV1::from_json(&handle.replay_script_json())
            .expect("wasm replay script decodes");
        assert!(replay
            .actions
            .iter()
            .any(|action| matches!(action, hydracache_sim::ControlActionV1::Subscribe { .. })));
    }

    #[test]
    fn wasm_topology_controls_match_native() {
        let mut handle = SimHandle::new(0x5340);
        handle
            .set_workload_enabled(false)
            .expect("workload toggle applies");
        handle.run(8);
        handle
            .isolate_node("node-0".to_owned())
            .expect("isolate applies");
        handle.step();
        handle
            .rejoin_node("node-0".to_owned())
            .expect("rejoin applies");
        handle.add_node().expect("add-node applies");

        let snapshot = SimSnapshot::from_json(&handle.snapshot_json()).expect("valid snapshot");
        assert_eq!(snapshot.nodes.len(), 4);
        assert!(snapshot.rebalance.is_some());
    }

    #[test]
    fn wasm_mode_change_is_visible_in_snapshot() {
        let mut handle = SimHandle::new(0x5354);
        handle.set_mode("mixed".to_owned()).expect("mode applies");

        let snapshot = SimSnapshot::from_json(&handle.snapshot_json()).expect("valid snapshot");

        assert_eq!(snapshot.mode, "mixed");
        assert_eq!(snapshot.intervention_count, 1);
    }

    #[test]
    fn wasm_faults_are_recorded_and_replay_identically() {
        let mut handle = SimHandle::new(0x5400);
        handle.run(6);
        handle
            .set_workload_enabled(false)
            .expect("workload toggle applies");
        handle
            .crash_node("node-1".to_owned())
            .expect("crash applies");
        handle.run(3);
        handle
            .inject(
                "partition".to_owned(),
                "node-0".to_owned(),
                "node-2".to_owned(),
                0,
            )
            .expect("partition applies");
        handle
            .inject(
                "delay".to_owned(),
                "node-2".to_owned(),
                "node-0".to_owned(),
                250,
            )
            .expect("delay applies");
        handle.run(2);
        handle
            .restart_node("node-1".to_owned())
            .expect("restart applies");
        handle.run(4);

        let script_json = handle.replay_script_json();
        let script =
            hydracache_sim::ReplayScriptV1::from_json(&script_json).expect("script decodes");
        assert_eq!(script.actions.len(), 5);
        assert_eq!(script.target_step, Some(15));

        let mut replayed = SimHandle::new(0);
        replayed
            .apply_control_script_json(script_json)
            .expect("replay applies");
        assert_eq!(replayed.snapshot_json(), handle.snapshot_json());
    }

    #[test]
    fn wasm_scenario_with_interventions_replays_from_preset() {
        let mut handle = SimHandle::new(0);
        handle
            .apply_scenario("minority_partition_cannot_commit".to_owned())
            .expect("scenario applies");
        handle.run(2);
        handle
            .crash_node("node-0".to_owned())
            .expect("crash applies");
        handle.run(3);

        let script_json = handle.replay_script_json();
        let script =
            hydracache_sim::ReplayScriptV1::from_json(&script_json).expect("script decodes");
        assert_eq!(
            script.scenario.as_deref(),
            Some("minority_partition_cannot_commit")
        );

        let mut replayed = SimHandle::new(0);
        replayed
            .apply_control_script_json(script_json)
            .expect("replay applies");
        assert_eq!(replayed.snapshot_json(), handle.snapshot_json());
    }
}
