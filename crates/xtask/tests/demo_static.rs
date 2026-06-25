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
    let readme = fs::read_to_string(root.join("demo/README.md")).expect("demo README exists");
    let workflow =
        fs::read_to_string(root.join(".github/workflows/demo.yml")).expect("demo workflow exists");
    let root_readme = fs::read_to_string(root.join("README.md")).expect("root README exists");
    let positioning =
        fs::read_to_string(root.join("docs/POSITIONING.md")).expect("positioning doc exists");
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
    assert!(html.contains("data-testid=\"copy-status\""));
    assert!(html.contains("data-testid=\"snapshot-hash\""));
    assert!(html.contains("actual invariant checker"));

    assert!(js.contains("from \"./share.js\""));
    assert!(js.contains("from \"./scenarios.js\""));
    assert!(js.contains("./pkg/hydracache_sim_wasm.js"));
    assert!(js.contains("new state.SimHandle"));
    assert!(js.contains("WasmSimSession"));
    assert!(js.contains("ServerSimSession"));
    assert!(js.contains("/sim/new"));
    assert!(js.contains("/sim/step"));
    assert!(js.contains("/sim/inject"));
    assert!(js.contains("snapshot_json()"));
    assert!(js.contains("set_workload_enabled"));
    assert!(js.contains("crash_node"));
    assert!(js.contains("restart_node"));
    assert!(js.contains("state.sim.inject(action"));
    assert!(js.contains("snapshot.verdict.status"));
    assert!(js.contains("snapshot.formation_phase"));
    assert!(js.contains("snapshot.election_source"));
    assert!(js.contains("snapshot.nodes"));
    assert!(js.contains("snapshot.links"));
    assert!(js.contains("snapshot.keys"));
    assert!(js.contains("writeUrlState(window.history"));
    assert!(js.contains("snapshotHash(snapshot)"));
    assert!(js.contains("reproducerCommand("));
    assert!(js.contains("state.sim.apply_scenario(state.scenario)"));
    assert!(js.contains("actual invariant checker"));
    assert!(js.contains("el.copyStatus.textContent = command"));

    assert!(share.contains("readInitialState"));
    assert!(share.contains("writeUrlState"));
    assert!(share.contains("reproducerCommand"));
    assert!(share.contains("snapshotHash"));
    assert!(share.contains("engine"));
    assert!(share.contains("apiBase"));

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
    assert!(seed_spec.contains("copy-status"));

    assert!(readme.contains("real deterministic"));
    assert!(readme
        .contains("cargo build -p hydracache-sim-wasm --target wasm32-unknown-unknown --locked"));
    assert!(readme.contains("wasm-pack build crates/hydracache-sim-wasm"));
    assert!(readme.contains("cargo xtask verify"));
    assert!(readme.contains("engine=server"));
    assert!(readme.contains("cargo run -p hydracache-sandbox -- --backend memory"));

    assert!(workflow
        .contains("cargo build -p hydracache-sim-wasm --target wasm32-unknown-unknown --locked"));
    assert!(workflow.contains("wasm-pack build crates/hydracache-sim-wasm"));
    assert!(workflow
        .contains("npx playwright test demo/tests/ui_smoke.spec.js demo/tests/seed_share.spec.js"));
    assert!(workflow.contains("actions/upload-pages-artifact"));
    assert!(workflow.contains("actions/deploy-pages"));
    assert!(!workflow.contains("pull_request:"));

    assert!(root_readme.contains("demo/README.md"));
    assert!(root_readme.contains("javaquasar.github.io/hydracache"));
    assert!(positioning.contains("../demo/README.md"));
    assert!(positioning.contains("javaquasar.github.io/hydracache"));
}

#[test]
fn demo_wasm_pack_output_is_gitignored() {
    let root = repo_root();
    let gitignore = fs::read_to_string(root.join(".gitignore")).expect("gitignore exists");

    assert!(gitignore.lines().any(|line| line.trim() == "demo/pkg/"));
}
