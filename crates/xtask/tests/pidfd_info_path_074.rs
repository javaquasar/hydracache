#[path = "support/pidfd_info_path_guard.rs"]
mod guard;

#[test]
fn pidfd_info_path_only_exact_allocation_hook_is_added_at_baseline() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = "tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs";
    let raw = std::fs::read_to_string(root.join(path))
        .unwrap()
        .replace('\r', "");
    let prior = std::process::Command::new("git")
        .args(["show", &format!("f9e7cada:{path}")])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(prior.status.success());
    let prior = String::from_utf8(prior.stdout).unwrap().replace('\r', "");
    assert_eq!(crate::hc074_pidfd_info_path_view!(path, &raw), prior);
    let bad = raw.replace("mod pidfd_info_path;", "pub mod pidfd_info_path;");
    assert!(std::panic::catch_unwind(|| crate::hc074_pidfd_info_path_view!(path, &bad)).is_err());
    assert_ne!(
        crate::hc074_pidfd_info_path_view!(path, &format!("{raw}\n// drift\n")),
        prior
    );
}
