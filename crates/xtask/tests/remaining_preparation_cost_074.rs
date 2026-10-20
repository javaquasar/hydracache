#[path = "support/document_scratch_guard.rs"]
mod guard;
fn root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
#[test]
fn remaining_preparation_marks_restore_exact_preceding_bodies_and_reject_drift() {
    for (file, marker) in [
        ("diagnostic_original_output_join.rs", "outputs-starting"),
        ("diagnostic_owned_root_manager.rs", "owned-context"),
        ("diagnostic_original_root_guardians.rs", "owned-controls"),
    ] {
        let path = format!("tools/long-run-supervisor-074/src/{file}");
        let raw = std::fs::read_to_string(root().join(&path))
            .unwrap()
            .replace('\r', "");
        let output = std::process::Command::new("git")
            .args([
                "show",
                &format!("dea214a80aa59d3af7b8456a43c0763573a04546:{path}"),
            ])
            .current_dir(root())
            .output()
            .unwrap();
        assert!(output.status.success());
        let old = String::from_utf8(output.stdout).unwrap().replace('\r', "");
        assert_eq!(
            crate::hc074_remaining_preparation_view!(path.as_str(), &raw),
            old
        );
        for changed in [raw.replace(marker, "missing-marker"), format!("{raw}{raw}")] {
            assert!(
                std::panic::catch_unwind(|| crate::hc074_remaining_preparation_view!(
                    path.as_str(),
                    &changed
                ))
                .is_err()
            );
        }
        let changed = raw.replacen("?;", ";", 1);
        assert_ne!(
            crate::hc074_remaining_preparation_view!(path.as_str(), &changed),
            old
        );
    }
    // Keep the shared predecessor adapter compiled and exercised as well.
    assert_eq!(guard::normalize("unrelated", "unchanged"), "unchanged");
}
