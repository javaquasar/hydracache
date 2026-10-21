//! Reverse one constructor/import and exact hooks; never exclude a live body.
#[macro_export]
macro_rules! hc074_trust_component_stack_view {
    ($path:expr,$source:expr) => {{
        let path: &str=$path;
        let mut raw:String=$source.to_owned();
        if path=="tools/long-run-supervisor-074/src/diagnostic_worker_trust_input.rs"
            && (raw.contains("diagnostic_trust_component_name.rs")||raw.contains("component_name::Name"))
        {
            let hook="\n#[path = \"diagnostic_trust_component_name.rs\"]\nmod component_name;\n\n#[cfg(all(test, feature = \"fixed-leaf-alloc-probe\"))]\n#[path = \"../tests/support/trust_component_stack_adapter.rs\"]\npub(crate) mod component_stack_controls;\n";
            let new="let name = component_name::Name::new(name).map_err(|_| TrustInputError::Invalid)?;";
            let fs="use std::fs::{File, Metadata, OpenOptions};";
            assert_eq!(raw.matches(hook).count(),1);
            assert_eq!(raw.matches(new).count(),1);
            assert_eq!(raw.matches(fs).count(),1);
            assert!(!raw.contains("use std::ffi::CString;"));
            raw=raw.replacen(hook,"",1).replacen(new,"let name = CString::new(name).map_err(|_| TrustInputError::Invalid)?;",1).replacen(fs,&format!("use std::ffi::CString;\n{fs}"),1);
        }
        if path=="tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs"
            && raw.contains("mod trust_component_stack;")
        {
            let hook="\n#[path = \"../tests/support/trust_component_stack.rs\"]\nmod trust_component_stack;\n";
            assert_eq!(raw.matches(hook).count(),1);
            raw=raw.replacen(hook,"",1);
        }
        raw
    }};
}
