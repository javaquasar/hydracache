#[path = "support/document_scratch_guard.rs"]
mod guard;
fn root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
#[test]
fn bounded_projection_recovers_all_473_preceding_bodies_and_frozen_parsers() {
    let git = |args: &[&str]| {
        let out = std::process::Command::new("git")
            .args(args)
            .current_dir(root())
            .output()
            .unwrap();
        assert!(out.status.success());
        String::from_utf8(out.stdout).unwrap().replace('\r', "")
    };
    let read = |path: &str| {
        std::fs::read_to_string(root().join(path))
            .unwrap()
            .replace('\r', "")
    };
    let paths = git(&[
        "ls-tree",
        "-r",
        "--name-only",
        "dea214a80aa59d3af7b8456a43c0763573a04546",
        "--",
        "tools/long-run-supervisor-074/src",
        "tools/long-run-supervisor-074/tests/support",
    ]);
    assert_eq!(paths.lines().count(), 473);
    for path in paths.lines() {
        assert_eq!(
            crate::hc074_remaining_preparation_view!(path, &read(path)),
            git(&[
                "show",
                &format!("dea214a80aa59d3af7b8456a43c0763573a04546:{path}")
            ]),
            "{path}"
        );
    }
    let path = "tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs";
    let raw = read(path);
    let candidate = read("tools/long-run-supervisor-074/tests/support/projection_candidate.rs");
    let exact = format!("fn projection(bytes: &[u8], credentials: &libc::ucred) -> Result<Projection, KernelError> {{{}", candidate.split_once(") -> Result<Projection, KernelError> {").unwrap().1.trim_end());
    assert_eq!(raw.matches(&exact).count(), 1);
    for changed in [
        raw.replacen("values[len] = value", "values[len] = skipped", 1),
        format!("{raw}{exact}"),
    ] {
        assert!(
            std::panic::catch_unwind(|| crate::hc074_remaining_preparation_view!(path, &changed))
                .is_err()
        );
    }
    let reference = read("tools/long-run-supervisor-074/tests/support/projection_reference.rs");
    let old = git(&[
        "show",
        &format!("dea214a80aa59d3af7b8456a43c0763573a04546:{path}"),
    ]);
    let original = old
        .split_once("fn projection(")
        .unwrap()
        .1
        .split_once("\nstruct Sender {")
        .unwrap()
        .0;
    assert_eq!(
        original
            .split_once(") -> Result<Projection, KernelError> {")
            .unwrap()
            .1
            .trim_end(),
        reference
            .split_once(") -> Result<Projection, KernelError> {")
            .unwrap()
            .1
            .trim_end()
    );
    let control = read("tools/long-run-supervisor-074/tests/support/projection_cost.rs");
    let start = control.find("        #[cfg(test)]").unwrap();
    let end = control[start..]
        .find("\n}\npub(in crate::diagnostic_kernel_admission) fn compare")
        .unwrap()
        + start;
    let restored = control[start..end].replace(
        "parse(scratch.read(file)?, &self.credentials)?",
        "projection(scratch.read(file)?, &self.credentials)?",
    );
    let original_sender = old
        .split_once("    fn revalidate(&self, origin:")
        .unwrap()
        .1;
    let original_sender = &original_sender[original_sender.find("        #[cfg(test)]").unwrap()
        ..original_sender.find("\n}\nfn alive").unwrap()];
    assert_eq!(
        restored.trim_end(),
        original_sender.trim_end(),
        "complete Sender control changes only projection selection"
    );
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
