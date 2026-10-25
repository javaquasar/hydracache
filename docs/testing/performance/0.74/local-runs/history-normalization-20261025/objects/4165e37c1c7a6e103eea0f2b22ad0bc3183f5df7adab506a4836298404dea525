//! Remove one opt-in baseline hook only. No production body is excluded.
#[path = "context_document_stack_guard.rs"]
mod stack_candidate;
#[macro_export]
macro_rules! hc074_context_document_baseline_view {
    ($path:expr,$source:expr) => {{
        let path: &str = $path;
        let mut raw: String = $crate::hc074_context_document_stack_view!(path,$source);
        if path == "tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs"
            && raw.contains("mod context_document_buffer_baseline;")
        {
            let hook = "\n#[path = \"../tests/support/context_document_buffer_baseline.rs\"]\nmod context_document_buffer_baseline;\n";
            assert_eq!(raw.matches(hook).count(),1);
            raw = raw.replacen(hook,"",1);
        }
        raw
    }};
}
