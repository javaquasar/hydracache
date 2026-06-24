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

    /// Serialize the current canonical simulator snapshot as JSON.
    pub fn snapshot_json(&self) -> String {
        self.world.snapshot_json()
    }

    /// Serialize the latest canonical invariant verdict as JSON.
    pub fn verdict_json(&self) -> String {
        self.world.verdict_json()
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
}
