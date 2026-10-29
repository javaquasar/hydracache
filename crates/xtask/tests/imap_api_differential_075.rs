use std::path::{Path, PathBuf};

use hydracache_cluster_testkit::value_plane_concurrent_075::{
    generated_schedule, ConcurrentSchedule,
};
use hydracache_cluster_testkit::value_plane_differential_075::{
    generated_trace, ApiTrace, TraceStep,
};
use xtask::imap_api_differential::{
    check_actual_at_root, execute_java, execute_java_concurrent, parse_args, parse_observations,
    prepare_java_at_root, retain_failure_schedule, retain_failure_trace,
};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf()
}

#[test]
fn actual_process_differential_compares_every_observation() {
    prepare_java_at_root(&root()).unwrap();
    let report = check_actual_at_root(&root(), 117, 128).unwrap();
    assert_eq!(report.seed, 117);
    assert!(report.steps >= 128 + 40);
    assert_eq!(report.comparisons, report.steps);
    assert!(report.canary_detected);
    assert_eq!(report.concurrent_rounds, 27);
    assert!(report.concurrent_canary_detected);
    assert_eq!(report.extended_steps, 64);
    assert_eq!(report.extended_process_canaries, 8);
}

#[test]
fn differential_options_and_process_boundaries_fail_loud() {
    assert_eq!(
        parse_args(&["--release".into(), "0.75".into()]).unwrap(),
        (117, 128)
    );
    for args in [
        vec!["--release", "0.74"],
        vec!["--release"],
        vec!["--unknown", "0.75"],
        vec!["--release", "0.75", "--seed", "bad"],
        vec!["--release", "0.75", "--steps", "10000"],
        vec!["--release", "0.75", "--seed", "1", "--seed", "2"],
    ] {
        assert!(parse_args(&args.into_iter().map(str::to_owned).collect::<Vec<_>>()).is_err());
    }
    assert_eq!(
        parse_observations(b"one\ntwo\n", 2).unwrap(),
        vec!["one", "two"]
    );
    assert!(parse_observations(b"one\n", 2).is_err());
    assert!(parse_observations(b"one\nextra\n", 1).is_err());
    assert!(parse_observations(b"\xff", 1).is_err());
    assert!(parse_observations(b"\n", 1).is_err());
    let temp = tempfile::tempdir().unwrap();
    assert!(execute_java(temp.path(), &generated_trace(117, 0).unwrap(), false).is_err());
    let oversized = ApiTrace {
        seed: 1,
        steps: vec![TraceStep::point("get", b"k", None); 513],
    };
    assert!(execute_java(temp.path(), &oversized, false).is_err());
    assert!(
        execute_java_concurrent(temp.path(), &generated_schedule(117, 0).unwrap(), false).is_err()
    );
    assert!(execute_java_concurrent(
        temp.path(),
        &ConcurrentSchedule {
            seed: 117,
            rounds: Vec::new()
        },
        false
    )
    .is_err());
}

#[test]
fn minimized_failure_artifacts_never_overwrite_conflicting_files() {
    let root = tempfile::tempdir().unwrap();
    let trace = generated_trace(117, 0).unwrap();
    let path = retain_failure_trace(root.path(), &trace).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), trace.encode());
    assert_eq!(retain_failure_trace(root.path(), &trace).unwrap(), path);
    std::fs::write(&path, "other source").unwrap();
    assert!(retain_failure_trace(root.path(), &trace).is_err());
    assert_eq!(std::fs::read_to_string(path).unwrap(), "other source");
    let schedule = generated_schedule(117, 0).unwrap();
    let path = retain_failure_schedule(root.path(), &schedule).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), schedule.encode());
    assert_eq!(
        retain_failure_schedule(root.path(), &schedule).unwrap(),
        path
    );
    std::fs::write(&path, "conflicting schedule").unwrap();
    assert!(retain_failure_schedule(root.path(), &schedule).is_err());
    assert!(retain_failure_schedule(
        root.path(),
        &ConcurrentSchedule {
            seed: 1,
            rounds: Vec::new()
        }
    )
    .is_err());
}
