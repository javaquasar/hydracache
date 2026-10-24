#[path = "support/document_scratch_guard.rs"]
mod guard;

fn root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn starting_cost_marks_restore_exact_effects_checks_and_order() {
    for file in [
        "diagnostic_original_output_join.rs",
        "diagnostic_collector_starting_storage.rs",
    ] {
        let path = format!("tools/long-run-supervisor-074/src/{file}");
        let raw = std::fs::read_to_string(root().join(&path))
            .unwrap()
            .replace('\r', "");
        let old = std::process::Command::new("git")
            .args([
                "show",
                &format!("4d0f461c638d158ff62dbd3e0f8f9914d523581e:{path}"),
            ])
            .current_dir(root())
            .output()
            .unwrap();
        assert!(old.status.success());
        let old = String::from_utf8(old.stdout).unwrap().replace('\r', "");
        let restored = crate::hc074_starting_effects_view!(path.as_str(), &raw);
        assert_eq!(restored, old, "all runtime bodies unchanged: {path}");
        assert_eq!(
            crate::hc074_starting_effects_view!(path.as_str(), &restored),
            old
        );
        let marks: Vec<_> = raw
            .lines()
            .filter(|line| line.contains("::mark(\"st-"))
            .collect();
        assert_eq!(
            marks.len(),
            if file.contains("output_join") { 11 } else { 4 }
        );
        for mark in marks {
            let name = mark
                .split("mark(\"")
                .nth(1)
                .unwrap()
                .split('"')
                .next()
                .unwrap();
            assert_eq!(raw.matches(&format!("mark(\"{name}\")")).count(), 1);
            for changed in [raw.replacen(name, "st-missing", 1), format!("{raw}{raw}")] {
                assert!(
                    std::panic::catch_unwind(|| crate::hc074_starting_effects_view!(
                        path.as_str(),
                        &changed
                    ))
                    .is_err()
                );
            }
        }
        let drift = raw.replacen("?;", ";", 1);
        assert_ne!(
            crate::hc074_starting_effects_view!(path.as_str(), &drift),
            old
        );
    }
    assert_eq!(guard::normalize("unrelated", "unchanged"), "unchanged");
}

#[test]
fn starting_sync_sites_and_stage_contract_are_not_optimization_waivers() {
    let path = "tools/long-run-supervisor-074/src/diagnostic_collector_starting_storage.rs";
    let raw = std::fs::read_to_string(root().join(path)).unwrap();
    assert_eq!(raw.matches(".sync_all()?").count(), 2);
    // One directory-sync arm covers two distinct stages, plus one file-sync arm.
    assert!(raw.contains("CollectorStartingStage::CreateDirectorySynced"));
    assert!(raw.contains("| CollectorStartingStage::DirectorySynced =>"));
    assert!(raw.contains("pub(crate) const STAGES: [CollectorStartingStage; 9]"));
    for name in [
        "self.inspect_starting()?;",
        "self.inspect_starting()",
        "self.revalidate()?;",
    ] {
        assert!(raw.contains(name));
    }
}
