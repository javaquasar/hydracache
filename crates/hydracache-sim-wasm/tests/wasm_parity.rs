#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test;

use hydracache_sim::{SimConfig, SimWorld};
use hydracache_sim_wasm::{snapshot_json, verdict_json, SimHandle};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn same_seed_native_and_wasm_match() {
    let seed = 0x50_00_00_01;
    let steps = 16;

    let mut world = SimWorld::new(seed, SimConfig::default());
    world.run(steps);

    let mut handle = SimHandle::new(seed);
    handle.run(steps);

    assert_eq!(handle.seed(), seed);
    assert_eq!(handle.snapshot_json(), snapshot_json(&world));
    assert_eq!(handle.verdict_json(), verdict_json(&world));
}
