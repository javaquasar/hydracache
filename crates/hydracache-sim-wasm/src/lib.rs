//! Browser bindings for the real deterministic HydraCache simulator.

use hydracache_sim::{SimConfig, SimWorld};
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

    /// Enable or disable the built-in workload generator.
    pub fn set_workload_enabled(&mut self, enabled: bool) {
        self.world.set_workload_enabled(enabled);
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

    /// Crash a simulator node.
    pub fn crash_node(&mut self, node_id: String) -> Result<(), JsValue> {
        if self.world.crash_node(node_id.clone()) {
            Ok(())
        } else {
            Err(JsValue::from_str(&format!("unknown node '{node_id}'")))
        }
    }

    /// Restart a simulator node.
    pub fn restart_node(&mut self, node_id: String) -> Result<(), JsValue> {
        if self.world.restart_node(node_id.clone()) {
            Ok(())
        } else {
            Err(JsValue::from_str(&format!(
                "node '{node_id}' is not crashed"
            )))
        }
    }

    /// Apply an interactive link action.
    pub fn inject(
        &mut self,
        action: String,
        from: String,
        to: String,
        delay_millis: u64,
    ) -> Result<(), JsValue> {
        let applied = match action.as_str() {
            "partition" => self.world.partition_link(from.clone(), to.clone()),
            "heal" => self.world.heal_link(from.clone(), to.clone()),
            "drop" => self.world.drop_next_on_link(from.clone(), to.clone()),
            "delay" => self
                .world
                .delay_next_on_link_millis(from.clone(), to.clone(), delay_millis),
            other => return Err(JsValue::from_str(&format!("unknown link action '{other}'"))),
        };
        if applied {
            Ok(())
        } else {
            Err(JsValue::from_str(&format!(
                "could not apply '{action}' to link {from}->{to}"
            )))
        }
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

    /// Apply a versioned replay script through the shared control surface.
    pub fn apply_control_script_json(&mut self, script_json: String) -> Result<(), JsValue> {
        let script = hydracache_sim::ReplayScriptV1::from_json(&script_json)
            .map_err(|error| JsValue::from_str(&error.to_string()))?;
        self.world
            .apply_replay_script(&script)
            .map_err(|error| JsValue::from_str(&error.to_string()))
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
        handle.set_workload_enabled(false);
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
        handle.set_workload_enabled(false);
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
}
