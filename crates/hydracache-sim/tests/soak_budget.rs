use std::time::Duration;

use hydracache_sim::{run_soak, SimConfig, SoakConfig, SoakReport};
use serde_json::Value;

#[test]
fn bounded_ci_soak_is_deterministic_and_fast() {
    let cfg = SoakConfig::new(
        0x50_40,
        Duration::from_millis(500),
        64,
        SimConfig::default(),
    )
    .with_max_seeds(4);

    let first = run_soak(&cfg);
    let second = run_soak(&cfg);

    assert_eq!(first, second, "fixed master seed must be deterministic");
    assert_eq!(first.seeds_run, 4);
    assert_eq!(first.total_steps, 256);
    assert!(first.first_failure.is_none(), "{first:?}");

    let report = serde_json::to_value(SoakReport::from(&first)).expect("report serializes");
    assert_eq!(report["outcome"]["status"], "clean");
    assert_eq!(report["resource_bounds_ok"], true);
    assert_eq!(report["wall_clock_secs"], 0);
    for forbidden in [
        "score",
        "self_score",
        "health_score",
        "throughput",
        "ops_per_sec",
    ] {
        assert!(
            !contains_key(&report, forbidden),
            "SOAK_REPORT must not contain score-like field '{forbidden}': {report}"
        );
    }
}

fn contains_key(value: &Value, needle: &str) -> bool {
    match value {
        Value::Object(map) => map
            .iter()
            .any(|(key, value)| key == needle || contains_key(value, needle)),
        Value::Array(values) => values.iter().any(|value| contains_key(value, needle)),
        _ => false,
    }
}
