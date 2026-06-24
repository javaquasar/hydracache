use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use hydracache_sim::scenario_presets;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("hydracache-sim crate lives under crates/hydracache-sim")
        .to_path_buf()
}

#[test]
fn demo_presets_have_engine_seeds() {
    let root = repo_root();
    let scenarios_js =
        fs::read_to_string(root.join("demo/scenarios.js")).expect("demo scenarios helper exists");

    let ui_names = extract_scenario_names(&scenarios_js);
    assert!(
        ui_names.contains("default"),
        "demo scenarios must keep the direct seeded run option"
    );

    let ui_presets = ui_names
        .iter()
        .filter(|name| name.as_str() != "default")
        .cloned()
        .collect::<BTreeSet<_>>();
    let engine_presets = scenario_presets();
    let mut engine_names = BTreeSet::new();
    for preset in &engine_presets {
        assert!(
            preset.seed > 0,
            "scenario {} has no engine seed",
            preset.name
        );
        assert!(
            preset.steps > 0,
            "scenario {} has no engine steps",
            preset.name
        );
        assert!(
            engine_names.insert(preset.name.to_owned()),
            "duplicate engine scenario {}",
            preset.name
        );
    }

    assert_eq!(
        ui_presets, engine_names,
        "demo must not define UI-only scenarios or hide engine scenarios"
    );
}

fn extract_scenario_names(source: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for line in source.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("name: \"") else {
            continue;
        };
        let Some(name) = rest.split('"').next() else {
            continue;
        };
        assert!(
            names.insert(name.to_owned()),
            "duplicate demo scenario {name}"
        );
    }
    assert!(!names.is_empty(), "demo scenarios list is empty");
    names
}
