#[test]
fn bootstrap_entry_binding_exists_before_child_delivery() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    assert!(root.join("tools/long-run-supervisor-074/src/diagnostic_bootstrap_entry_format.rs").is_file(), "missing sealed child-entry format");
}
