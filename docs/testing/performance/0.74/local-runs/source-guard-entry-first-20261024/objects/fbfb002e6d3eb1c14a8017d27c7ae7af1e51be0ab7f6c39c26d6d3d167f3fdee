//! Exact reviewed input versions for historical assertions, never runtime trust.
use std::path::{Path, PathBuf};
pub const REVIEWED: &str = "3c52c0aa34b8d00b2fac44375d5ba8fdf532bb9f";
pub const PREDECESSOR: &str = "33c54b0cda8c330b115009cdf092ad002077cedf";
pub const PATHS: [&str; 18] = [
    "docs/COMPAT.md",
    "docs/testing/gated-test-registry.toml",
    "tools/long-run-supervisor-074/src/bin/diagnostic_held_launcher.rs",
    "tools/long-run-supervisor-074/src/diagnostic_artifacts_linux.rs",
    "tools/long-run-supervisor-074/src/diagnostic_builder.rs",
    "tools/long-run-supervisor-074/src/diagnostic_bundle_projection.rs",
    "tools/long-run-supervisor-074/src/diagnostic_held_role_files.rs",
    "tools/long-run-supervisor-074/src/diagnostic_held_runtime_trust.rs",
    "tools/long-run-supervisor-074/src/diagnostic_held_unit_material.rs",
    "tools/long-run-supervisor-074/src/diagnostic_held_unit_material_kernel.rs",
    "tools/long-run-supervisor-074/src/diagnostic_held_unit_material_kernel_tests.rs",
    "tools/long-run-supervisor-074/src/diagnostic_held_unit_material_tests.rs",
    "tools/long-run-supervisor-074/src/diagnostic_manager_channel.rs",
    "tools/long-run-supervisor-074/src/diagnostic_process.rs",
    "tools/long-run-supervisor-074/src/diagnostic_root_guardian_projection.rs",
    "tools/long-run-supervisor-074/tests/held_launcher.rs",
    "tools/long-run-supervisor-074/tests/support/consuming_held_live.rs",
    "tools/long-run-supervisor-074/tests/support/consuming_held_live_documents.rs",
];
pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
pub fn frozen(revision: &str, path: &str) -> String {
    assert!(matches!(revision, REVIEWED | PREDECESSOR));
    assert!(PATHS.contains(&path));
    let output = std::process::Command::new("git")
        .arg("--no-replace-objects")
        .args(["show", &format!("{revision}:{path}")])
        .current_dir(root())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "missing reviewed source input: {path}"
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .replace("\r\n", "\n")
}
pub fn expected(path: &str) -> Option<String> {
    PATHS.contains(&path).then(|| frozen(REVIEWED, path))
}
pub fn accepts(path: &str, raw: &str) -> bool {
    expected(path).is_some_and(|expected| expected == raw.replace("\r\n", "\n"))
}
pub fn predecessor(path: &str, raw: &str) -> String {
    assert!(
        accepts(path, raw),
        "unreviewed input cannot enter historical assertions: {path}"
    );
    frozen(PREDECESSOR, path)
}
