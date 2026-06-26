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
    let css = fs::read_to_string(root.join("demo/style.css")).expect("demo css exists");
    let package = fs::read_to_string(root.join("demo/package.json")).expect("demo package exists");
    let demo_playwright = fs::read_to_string(root.join("demo/playwright.config.mjs"))
        .expect("demo playwright config exists");
    let root_playwright = fs::read_to_string(root.join("playwright.config.mjs"))
        .expect("root playwright shim exists");
    let static_check = fs::read_to_string(root.join("demo/scripts/check-static.mjs"))
        .expect("demo static check exists");
    let static_server = fs::read_to_string(root.join("demo/scripts/serve-static.mjs"))
        .expect("demo static server exists");
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
    assert!(html.contains("data-testid=\"signals-panel\""));
    assert!(html.contains("data-testid=\"push-event-button\""));
    assert!(html.contains("data-testid=\"subscribe-button\""));
    assert!(html.contains("data-testid=\"clients-panel\""));
    assert!(html.contains("data-testid=\"subscribers-panel\""));
    assert!(html.contains("data-testid=\"add-node-button\""));
    assert!(html.contains("data-testid=\"mode-select\""));
    assert!(html.contains("data-testid=\"intervention-status\""));
    assert!(html.contains("data-testid=\"graph-legend\""));
    assert!(html.contains("glass-panel"));
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
    assert!(js.contains("snapshot.in_flight"));
    assert!(js.contains("snapshot.over_budget"));
    assert!(js.contains("snapshot.clients"));
    assert!(js.contains("snapshot.subscribers"));
    assert!(js.contains("snapshot.rebalance") || js.contains("rebalance"));
    assert!(js.contains("snapshot.mode"));
    assert!(js.contains("snapshot.active_scenario"));
    assert!(js.contains("snapshot.intervention_count"));
    assert!(js.contains("bindPreferenceState"));
    assert!(js.contains("prefers-reduced-motion"));
    assert!(js.contains("prefers-reduced-transparency"));
    assert!(js.contains("rebuildPackets"));
    assert!(js.contains("packet.dot.setAttribute(\"cx\", x)"));
    assert!(js.contains("graphSim.packets.push"));
    assert!(js.contains("node.disabled"));
    assert!(js.contains("add_node("));
    assert!(js.contains("isolate_node("));
    assert!(js.contains("rejoin_node("));
    assert!(js.contains("snapshot.keys"));
    assert!(js.contains("push_event("));
    assert!(js.contains("subscribe("));
    assert!(js.contains("writeUrlState("));
    assert!(js.contains("window.history"));
    assert!(js.contains("snapshotHash(snapshot)"));
    assert!(js.contains("reproducerCommand("));
    assert!(js.contains("loadReplayScript"));
    assert!(js.contains("apply_control_script_json"));
    assert!(js.contains("state.sim.apply_scenario(state.scenario)"));
    assert!(js.contains("actual invariant checker"));
    assert!(js.contains("el.copyStatus.textContent = command"));

    assert!(share.contains("readInitialState"));
    assert!(share.contains("writeUrlState"));
    assert!(share.contains("reproducerCommand"));
    assert!(share.contains("encodeReplayScript"));
    assert!(share.contains("decodeReplayScript"));
    assert!(share.contains("script"));
    assert!(share.contains("MAX_REPLAY_ACTIONS") || share.contains("actions.length > 256"));
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
    assert!(spec.contains("manual_push_shows_diverge_converge_and_listener_receipt"));
    assert!(spec.contains("node_controls_show_reelection_resync_and_scale_out"));
    assert!(spec.contains("modes_switch_and_topology_is_clickable_in_each"));
    assert!(spec.contains("glass_theme_renders_and_controls_remain_operable"));
    assert!(spec.contains("reduced_motion_and_transparency_fallbacks_apply"));
    assert!(spec.contains("contrastRatio"));
    assert!(seed_spec.contains("url_seed_reproduces_identical_run"));
    assert!(seed_spec.contains("copy_reproducer_roundtrips_mode_and_actions"));
    assert!(seed_spec.contains("script="));
    assert!(seed_spec.contains("copy-status"));

    assert!(css.contains("--glass"));
    assert!(css.contains("backdrop-filter"));
    assert!(css.contains("prefers-reduced-motion"));
    assert!(css.contains("prefers-reduced-transparency"));
    assert!(css.contains(".packet-trail"));
    assert!(css.contains("html[data-reduced-transparency=\"true\"]"));
    assert!(css.contains("html[data-reduced-motion=\"true\"]"));

    assert!(package.contains("\"@playwright/test\""));
    assert!(package.contains("\"build\": \"node ./scripts/check-static.mjs\""));
    assert!(package.contains("\"serve\": \"node ./scripts/serve-static.mjs\""));
    assert!(demo_playwright.contains("desktop-1440x900"));
    assert!(demo_playwright.contains("mobile-390x844"));
    assert!(demo_playwright.contains("npm run serve"));
    assert!(root_playwright.contains("testDir: \"./demo/tests\""));
    assert!(root_playwright.contains("npm --prefix demo run serve"));
    assert!(static_check.contains("demo static checks passed"));
    assert!(static_server.contains("HydraCache demo served"));

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
    assert!(workflow.contains("npm --prefix demo ci"));
    assert!(workflow.contains("npm --prefix demo run build"));
    assert!(workflow.contains("npx --prefix demo playwright install --with-deps chromium"));
    assert!(workflow.contains(
        "npx --prefix demo playwright test demo/tests/ui_smoke.spec.js demo/tests/seed_share.spec.js",
    ));
    assert!(workflow.contains("branches:"));
    assert!(workflow.contains("- main"));
    assert!(!workflow.contains("name: Serve demo"));
    assert!(!workflow.contains("demo-server.pid"));
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
