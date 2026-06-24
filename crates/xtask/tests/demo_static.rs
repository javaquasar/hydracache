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
    let spec = fs::read_to_string(root.join("demo/tests/ui_smoke.spec.js"))
        .expect("nightly UI smoke spec exists");

    assert!(html.contains("data-testid=\"verdict\""));
    assert!(html.contains("data-testid=\"partition-link\""));
    assert!(html.contains("data-testid=\"nodes-panel\""));

    assert!(js.contains("./pkg/hydracache_sim_wasm.js"));
    assert!(js.contains("new state.SimHandle"));
    assert!(js.contains("snapshot_json()"));
    assert!(js.contains("set_workload_enabled"));
    assert!(js.contains("crash_node"));
    assert!(js.contains("restart_node"));
    assert!(js.contains("state.sim.inject(action"));
    assert!(js.contains("snapshot.verdict.status"));
    assert!(js.contains("snapshot.nodes"));
    assert!(js.contains("snapshot.links"));
    assert!(js.contains("snapshot.keys"));

    assert!(spec.contains("loads_steps_and_renders_verdict"));
    assert!(spec.contains("clicking_partition_updates_link_state"));
}

#[test]
fn demo_wasm_pack_output_is_gitignored() {
    let root = repo_root();
    let gitignore = fs::read_to_string(root.join(".gitignore")).expect("gitignore exists");

    assert!(gitignore.lines().any(|line| line.trim() == "demo/pkg/"));
}
