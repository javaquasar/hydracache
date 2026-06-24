use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("xtask crate lives under crates/xtask")
        .to_path_buf()
}

#[test]
fn demo_static_files_are_wired_to_real_wasm_snapshot() {
    let root = repo_root();
    let html = fs::read_to_string(root.join("demo/index.html")).expect("demo html exists");
    let js = fs::read_to_string(root.join("demo/app.js")).expect("demo app exists");
    let share = fs::read_to_string(root.join("demo/share.js")).expect("demo share helper exists");
    let scenarios =
        fs::read_to_string(root.join("demo/scenarios.js")).expect("demo scenarios helper exists");
    let spec = fs::read_to_string(root.join("demo/tests/ui_smoke.spec.js"))
        .expect("nightly UI smoke spec exists");
    let seed_spec = fs::read_to_string(root.join("demo/tests/seed_share.spec.js"))
        .expect("nightly seed-share spec exists");

    assert!(html.contains("data-testid=\"verdict\""));
    assert!(html.contains("data-testid=\"partition-link\""));
    assert!(html.contains("data-testid=\"nodes-panel\""));
    assert!(html.contains("data-testid=\"scenario-select\""));
    assert!(html.contains("data-testid=\"load-scenario\""));
    assert!(html.contains("data-testid=\"copy-reproducer\""));
    assert!(html.contains("data-testid=\"snapshot-hash\""));

    assert!(js.contains("from \"./share.js\""));
    assert!(js.contains("from \"./scenarios.js\""));
    assert!(js.contains("./pkg/hydracache_sim_wasm.js"));
    assert!(js.contains("new state.SimHandle"));
    assert!(js.contains("state.sim.run(BigInt(steps))"));
    assert!(js.contains("snapshot_json()"));
    assert!(js.contains("set_workload_enabled"));
    assert!(js.contains("crash_node"));
    assert!(js.contains("restart_node"));
    assert!(js.contains("state.sim.inject(action"));
    assert!(js.contains("snapshot.verdict.status"));
    assert!(js.contains("snapshot.nodes"));
    assert!(js.contains("snapshot.links"));
    assert!(js.contains("snapshot.keys"));
    assert!(js.contains("writeUrlState(window.history"));
    assert!(js.contains("snapshotHash(snapshot)"));
    assert!(js.contains("reproducerCommand("));
    assert!(js.contains("state.sim.apply_scenario(state.scenario)"));

    assert!(share.contains("readInitialState"));
    assert!(share.contains("writeUrlState"));
    assert!(share.contains("reproducerCommand"));
    assert!(share.contains("snapshotHash"));

    for scenario in [
        "minority_partition_cannot_commit",
        "leader_crash_failover_no_committed_loss",
        "symmetric_partition_heal_converges",
        "each_quorum_region_loss_fails_loud",
        "delete_vs_concurrent_write_no_resurrection",
    ] {
        assert!(scenarios.contains(scenario), "missing scenario {scenario}");
    }

    assert!(spec.contains("loads_steps_and_renders_verdict"));
    assert!(spec.contains("clicking_partition_updates_link_state"));
    assert!(spec.contains("loading_scenario_uses_curated_engine_preset"));
    assert!(seed_spec.contains("url_seed_reproduces_identical_run"));
}

#[test]
fn demo_wasm_pack_output_is_gitignored() {
    let root = repo_root();
    let gitignore = fs::read_to_string(root.join(".gitignore")).expect("gitignore exists");

    assert!(gitignore.lines().any(|line| line.trim() == "demo/pkg/"));
}
