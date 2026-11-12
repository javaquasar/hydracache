use super::*;
use crate::imap_build_process_075::{
    BuildBounds, BuildController, BuildError, BuildOutcome, BuildStage,
};
use std::time::Duration;
#[test]
fn real_finite_package_command_failure_is_reaped_and_stops_before_second_module() {
    let root = tempfile::tempdir().unwrap();
    for module in ["java-imap-benchmark", "java-imap-hazelcast-075"] {
        let source = root.path().join("tests").join(module);
        fs::create_dir_all(source.join("src")).unwrap();
        fs::write(
            source.join("pom.xml"),
            b"unit source; never passed to Maven",
        )
        .unwrap();
    }
    let mut builds = BuildController::with_budget(
        root.path(),
        Duration::from_secs(30),
        BuildBounds::default(),
        Default::default(),
    )
    .unwrap();
    let mut calls = 0;
    let error = reproduce_packages(root.path(), &mut builds, |_| {
        calls += 1;
        let mut command = if cfg!(windows) {
            let mut c = std::process::Command::new("cmd");
            c.args(["/d", "/c", "exit 7"]);
            c
        } else {
            let mut c = std::process::Command::new("sh");
            c.args(["-c", "exit 7", "--"]);
            c
        };
        command.current_dir(root.path());
        Ok(command)
    })
    .unwrap_err();
    assert_eq!(calls, 1);
    let error = error.downcast_ref::<BuildError>().unwrap();
    assert_eq!(error.report.stage, BuildStage::ReproduceContract);
    assert_eq!(error.report.outcome, BuildOutcome::Failed);
    assert!(error.report.root_reaped);
    assert_eq!(error.report.remaining_live_members, Some(0));
    assert!(!root.path().join("provenance.json").exists());
}
