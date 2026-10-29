//! Reverse one allocation expression and exact test hooks; no body exemption.
#[macro_export]
macro_rules! hc074_collector_peer_capacity_view {
    ($path:expr,$source:expr) => {{
        let path:&str=$path;let mut raw:String=$source.to_owned();
        if path=="tools/long-run-supervisor-074/src/diagnostic_kernel_collector_inputs.rs" && (raw.contains("capacity_controls") || raw.contains("Vec::with_capacity(supplementary.len()")) {
            let hook="\n#[cfg(all(test, feature = \"fixed-leaf-alloc-probe\"))]\n#[path = \"../tests/support/collector_peer_capacity_adapter.rs\"]\npub(crate) mod capacity_controls;\n";
            let new="let mut gids = Vec::with_capacity(supplementary.len() + 1);\n    gids.extend_from_slice(supplementary);";
            assert_eq!(raw.matches(hook).count(),1);assert_eq!(raw.matches(new).count(),1);
            raw=raw.replacen(hook,"",1).replacen(new,"let mut gids = supplementary.to_vec();",1);
        }
        if path=="tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs" && raw.contains("mod collector_peer_capacity;") {
            let hook="\n#[path = \"../tests/support/collector_peer_capacity.rs\"]\nmod collector_peer_capacity;\n";
            assert_eq!(raw.matches(hook).count(),1);raw=raw.replacen(hook,"",1);
        }
        raw
    }};
}
