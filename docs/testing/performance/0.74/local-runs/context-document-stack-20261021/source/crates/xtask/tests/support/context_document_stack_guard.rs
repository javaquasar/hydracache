//! Reverse one storage constructor and exact hooks; never exclude a live body.
#[macro_export]
macro_rules! hc074_context_document_stack_view {
    ($path:expr,$source:expr) => {{
        let path: &str = $path;
        let mut raw: String = $source.to_owned();
        if path == "tools/long-run-supervisor-074/src/diagnostic_worker_context.rs"
            && (raw.contains("diagnostic_context_document_buffer.rs") || raw.contains("document_buffer::Scratch"))
        {
            let hook = "\n#[path = \"diagnostic_context_document_buffer.rs\"]\nmod document_buffer;\n\n#[cfg(all(test, feature = \"fixed-leaf-alloc-probe\"))]\n#[path = \"../tests/support/context_document_stack_adapter.rs\"]\npub(crate) mod document_stack_controls;\n";
            let new = "let mut buffer = document_buffer::Scratch::new(length);";
            assert_eq!(raw.matches(hook).count(),1);
            assert_eq!(raw.matches(new).count(),1);
            raw=raw.replacen(hook,"",1).replacen(new,"let mut buffer = vec![0; length + 1];",1);
        }
        if path == "tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs"
            && raw.contains("mod context_document_stack;")
        {
            let hook="\n#[path = \"../tests/support/context_document_stack.rs\"]\nmod context_document_stack;\n";
            assert_eq!(raw.matches(hook).count(),1);
            raw=raw.replacen(hook,"",1);
        }
        raw
    }};
}
