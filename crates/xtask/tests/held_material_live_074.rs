#[path = "support/held_material_live_guard.rs"]
mod guard;

#[test]
fn held_material_live_adapter_restores_exact_previous_bodies_and_rejects_hook_drift() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for path in [
        "tools/long-run-supervisor-074/src/diagnostic_held_unit_material.rs",
        "tools/long-run-supervisor-074/tests/support/consuming_held_live.rs",
    ] {
        let raw = std::fs::read_to_string(root.join(path))
            .unwrap()
            .replace('\r', "");
        let prior = std::process::Command::new("git")
            .args(["show", &format!("215626cb:{path}")])
            .current_dir(&root)
            .output()
            .unwrap();
        assert!(prior.status.success());
        let prior = String::from_utf8(prior.stdout).unwrap().replace('\r', "");
        assert_eq!(crate::hc074_held_material_live_view!(path, &raw), prior);
        assert_ne!(
            crate::hc074_held_material_live_view!(path, &format!("{raw}\n// drift\n")),
            prior
        );
        let bad = if path.ends_with("consuming_held_live.rs") {
            raw.replace(
                "assert_eq!(case, \"healthy\");",
                "assert_eq!(case, \"revalidate\");",
            )
        } else {
            raw.replace("mod live_api;", "pub mod live_api;")
        };
        assert!(
            std::panic::catch_unwind(|| crate::hc074_held_material_live_view!(path, &bad)).is_err()
        );
    }
}
