//! Reverse exactly one opt-in pure test hook; no production exception.
#[path = "trust_component_stack_guard.rs"]
mod stack_candidate;
#[macro_export]
macro_rules! hc074_trust_component_baseline_view {
    ($path:expr,$source:expr) => {{
        let path: &str = $path;
        let mut raw: String = $crate::hc074_trust_component_stack_view!(path,$source);
        if path == "tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs"
            && raw.contains("mod trust_component_baseline;")
        {
            let hook="\n#[path = \"../tests/support/trust_component_baseline.rs\"]\nmod trust_component_baseline;\n";
            assert_eq!(raw.matches(hook).count(),1);
            raw=raw.replacen(hook,"",1);
        }
        raw
    }};
}
