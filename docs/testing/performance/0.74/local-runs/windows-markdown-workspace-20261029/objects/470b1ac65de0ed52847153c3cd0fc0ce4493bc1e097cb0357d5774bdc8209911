//! Reverse exactly one pure test hook; no live body is exempted.
#[path = "collector_peer_capacity_guard.rs"]
mod capacity_candidate;
#[macro_export]
macro_rules! hc074_collector_peer_baseline_view {
    ($path:expr,$source:expr) => {{
        let path:&str=$path;let mut raw:String=$crate::hc074_collector_peer_capacity_view!(path,$source);
        if path=="tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs" && raw.contains("mod collector_peer_baseline;") {
            let hook="\n#[path = \"../tests/support/collector_peer_baseline.rs\"]\nmod collector_peer_baseline;\n";
            assert_eq!(raw.matches(hook).count(),1);raw=raw.replacen(hook,"",1);
        }
        raw
    }};
}
