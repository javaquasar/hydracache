use super::*;
use crate::imap_coverage_075::{Analysis, Profile, Summary};
use serde_json::json;
use std::sync::{Mutex, MutexGuard, OnceLock};

// All modeled evidence is explicitly unqualified. Nothing here runs Cargo or
// constructs/publishes a passing CI receipt. Serialize this test-only driver;
// it never changes environment variables or production function bindings.
#[derive(Default)]
struct State {
    root: PathBuf,
    calls: Vec<String>,
    fail_at: usize,
    source_reads: usize,
    source_drift: bool,
    wrong_rust: bool,
    wrong_java: bool,
    remove_report: bool,
}
fn state() -> &'static Mutex<State> {
    static STATE: OnceLock<Mutex<State>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(State::default()))
}
fn serial() -> MutexGuard<'static, ()> {
    static SERIAL: Mutex<()> = Mutex::new(());
    SERIAL.lock().unwrap()
}
fn stage(name: &str) -> LaneResult<MutexGuard<'static, State>> {
    let mut state = state().lock().unwrap();
    state.calls.push(name.into());
    if state.fail_at == state.calls.len() {
        return Err(format!("unit stage refusal: {name}").into());
    }
    Ok(state)
}
fn reset(root: &Path, fail_at: usize) {
    *state().lock().unwrap() = State {
        root: root.into(),
        fail_at,
        ..Default::default()
    };
}
fn root() -> LaneResult<PathBuf> {
    Ok(stage("root")?.root.clone())
}
fn workflow(root: &Path) -> LaneResult<()> {
    assert_eq!(stage("workflow")?.root, root);
    Ok(())
}
fn source(root: &Path) -> LaneResult<String> {
    let mut s = stage("source")?;
    assert_eq!(s.root, root);
    s.source_reads += 1;
    Ok(if s.source_drift && s.source_reads > 1 {
        "1".repeat(40)
    } else {
        "0".repeat(40)
    })
}
fn fingerprint(program: &str, arguments: &[&str]) -> LaneResult<String> {
    assert_eq!(arguments, &["--version"]);
    let s = stage(program)?;
    Ok(match program {
        "rustc" if s.wrong_rust => "rustc 1.88.0 (unit)".into(),
        "rustc" => "rustc 1.94.0 (unit)".into(),
        "java" if s.wrong_java => "openjdk 17.0.1 unit".into(),
        "java" => "openjdk 21.0.1 unit".into(),
        _ => panic!("unexpected fingerprint"),
    })
}
fn action(args: Vec<String>) -> LaneResult<()> {
    let _s = stage("action")?;
    assert_eq!(args, &["--release", "0.75"]);
    Ok(())
}
fn tooling(root: &Path, command: Command) -> LaneResult<()> {
    assert_eq!(stage("tooling")?.root, root);
    assert_eq!(command.get_program(), std::ffi::OsStr::new("cargo"));
    let args = command
        .get_args()
        .map(|s| s.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert!(args.iter().any(|s| s == "imap_coverage_075"));
    assert_eq!(command.get_current_dir(), Some(root));
    Ok(())
}
fn owned_material(root: &Path, directory: &Path) -> LaneResult<PathBuf> {
    let directory = root.join(directory);
    fs::create_dir_all(&directory)?;
    let path = directory.join("receipt.json");
    fs::write(
        &path,
        b"{\"scope\":\"unit-unqualified\",\"qualification\":false}",
    )?;
    Ok(path)
}
fn capture(root: &Path, output: &Path) -> LaneResult<PathBuf> {
    assert_eq!(stage("capture")?.root, root);
    assert!(output.to_string_lossy().contains("coverage-1.94.0-21-0-"));
    owned_material(root, output)
}
fn coverage(root: &Path, path: &Path) -> LaneResult<Analysis> {
    let s = stage("coverage")?;
    assert_eq!(s.root, root);
    assert!(path.starts_with(root));
    if s.remove_report {
        fs::remove_file(path)?;
    }
    Ok(unqualified_analysis())
}
fn unqualified_analysis() -> Analysis {
    Analysis {
        schema: "imap075-scoped-coverage-v2".into(),
        source_sha: "0".repeat(40),
        run_id: "0".repeat(32),
        profile: Profile {
            schema: "unit-unqualified".into(),
            baseline: String::new(),
            os: "linux".into(),
            arch: "x86_64".into(),
            rust_flags: String::new(),
            cargo_config: vec![],
            feature: String::new(),
            target: crate::imap_sbom_075::CargoTarget {
                triple: "unit".into(),
                cfg: Default::default(),
            },
            tools: vec![],
        },
        sources: vec![],
        raw_profiles: vec![],
        rust_report: crate::imap_extended_receipt_075::Artifact {
            path: "unit".into(),
            sha256: "0".repeat(64),
            bytes: 0,
        },
        java_reports: vec![],
        summary: Summary {
            sources: vec![],
            executable_added: 0,
            covered_added: 0,
            compiled_out: vec![],
        },
        qualification: false,
    }
}
fn preparation(root: &Path, output: &Path) -> LaneResult<PathBuf> {
    assert_eq!(stage("preparation")?.root, root);
    assert!(output.to_string_lossy().contains("pre074-1.94.0-21-0-"));
    owned_material(root, output)
}
fn preparation_check(root: &Path, path: &Path) -> LaneResult<()> {
    assert_eq!(stage("preparation-check")?.root, root);
    assert!(path.starts_with(root) && path.is_file());
    Ok(())
}
fn differential(
    root: &Path,
    seed: u64,
    steps: usize,
) -> LaneResult<crate::imap_api_differential::DifferentialReport> {
    assert_eq!(stage("differential")?.root, root);
    assert!([117, 118].contains(&seed));
    assert_eq!(steps, 128);
    Ok(crate::imap_api_differential::DifferentialReport {
        seed,
        steps: 0,
        comparisons: 0,
        canary_detected: false,
        concurrent_rounds: 0,
        concurrent_canary_detected: false,
        extended_steps: 0,
        extended_process_canaries: 0,
    })
}
fn concurrent(
    root: &Path,
    seed: i64,
    rounds: usize,
) -> LaneResult<crate::imap_extended_concurrent_075::ExtendedConcurrentReport> {
    assert_eq!(stage("concurrent")?.root, root);
    assert!([117, 118].contains(&seed));
    assert_eq!(rounds, 15);
    Ok(
        crate::imap_extended_concurrent_075::ExtendedConcurrentReport {
            seed,
            rounds: 0,
            process_canaries: 0,
            search_budget: 0,
        },
    )
}
fn extended(root: &Path, output: &Path, seed: i64) -> LaneResult<PathBuf> {
    assert_eq!(stage("extended")?.root, root);
    assert!(output
        .to_string_lossy()
        .contains(&format!("extended-1.94.0-21-0-{seed}-")));
    owned_material(root, output)
}
fn extended_check(root: &Path, path: &Path) -> LaneResult<()> {
    assert_eq!(stage("extended-check")?.root, root);
    assert!(path.starts_with(root) && path.is_file());
    Ok(())
}
fn campaign(root: &Path, seed: u64, rounds: usize) -> LaneResult<serde_json::Value> {
    assert_eq!(stage("campaign")?.root, root);
    assert!([117, 118].contains(&seed));
    assert_eq!(rounds, 3);
    Ok(json!({"scope":"unit", "passed":false, "seed":seed}))
}
const HOST: LaneHost = LaneHost {
    root,
    workflow,
    source,
    fingerprint,
    actions: [action; 6],
    tooling,
    coverage_capture: capture,
    coverage_check: coverage,
    preparation,
    preparation_check,
    differential,
    concurrent,
    extended,
    extended_check,
    campaign,
};
fn args() -> Vec<String> {
    [
        "--release",
        "0.75",
        "--mode",
        "pr",
        "--rust",
        "1.94.0",
        "--java",
        "21",
        "--shard",
        "0",
        "--output",
        "target/imap-075/lane-unit.json",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}
fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join(".github/workflows")).unwrap();
    fs::write(
        root.path().join(WORKFLOW),
        b"unqualified unit workflow bytes",
    )
    .unwrap();
    root
}
#[test]
fn unqualified_driver_exercises_all_stages_but_cannot_publish_a_passing_ci_receipt() {
    let _serial = serial();
    let root = fixture();
    reset(root.path(), 0);
    let error = run_lane_using(args(), &HOST).unwrap_err().to_string();
    assert!(error.contains("requires independently qualified coverage"));
    let expected = state().lock().unwrap().calls.clone();
    assert_eq!(expected.len(), 27);
    assert_eq!(
        expected.iter().filter(|s| s.as_str() == "action").count(),
        6
    );
    assert_eq!(
        expected
            .iter()
            .filter(|s| s.as_str() == "differential")
            .count(),
        2
    );
    assert!(!root.path().join("target/imap-075/lane-unit.json").exists());
    for fail_at in 1..=expected.len() {
        let root = fixture();
        reset(root.path(), fail_at);
        assert!(
            run_lane_using(args(), &HOST)
                .unwrap_err()
                .to_string()
                .contains("unit stage refusal"),
            "stage {fail_at}"
        );
        assert_eq!(state().lock().unwrap().calls, expected[..fail_at]);
        assert!(!root.path().join("target/imap-075/lane-unit.json").exists());
    }
}
#[test]
fn lane_runtime_source_and_checked_report_failures_do_not_publish_receipts() {
    let _serial = serial();
    for mode in 0..5 {
        let root = fixture();
        reset(root.path(), 0);
        {
            let mut s = state().lock().unwrap();
            match mode {
                0 => s.wrong_rust = true,
                1 => s.wrong_java = true,
                2 => s.source_drift = true,
                3 => s.remove_report = true,
                _ => {
                    fs::remove_file(root.path().join(WORKFLOW)).unwrap();
                }
            }
        }
        assert!(run_lane_using(args(), &HOST).is_err(), "mode {mode}");
        let s = state().lock().unwrap();
        if mode < 2 {
            assert!(!s.calls.iter().any(|s| s == "action"));
        }
        if mode == 3 {
            assert!(!s.calls.iter().any(|s| s == "preparation"));
        }
        assert!(!root.path().join("target/imap-075/lane-unit.json").exists());
    }
    let root = fixture();
    reset(root.path(), 0);
    let mut invalid = args();
    invalid[5] = "future".into();
    assert!(run_lane_using(invalid, &HOST).is_err());
    assert_eq!(state().lock().unwrap().calls, vec!["root"]);
    assert!(run_lane(vec![]).is_err());
}
#[test]
fn actual_coverage_verifier_refuses_the_relative_unqualified_report_before_preparation() {
    fn unqualified_capture(root: &Path, output: &Path) -> LaneResult<PathBuf> {
        let path = capture(root, output)?;
        fs::write(&path, serde_json::to_vec(&unqualified_analysis())?)?;
        Ok(path.strip_prefix(root)?.to_path_buf())
    }
    let _serial = serial();
    let root = fixture();
    reset(root.path(), 0);
    let host = LaneHost {
        coverage_capture: unqualified_capture,
        coverage_check: ACTUAL_HOST.coverage_check,
        ..HOST
    };
    assert!(run_lane_using(args(), &host)
        .unwrap_err()
        .to_string()
        .contains("not finalized qualification"));
    let s = state().lock().unwrap();
    assert_eq!(s.calls.last().unwrap(), "capture");
    assert!(!s.calls.iter().any(|name| name == "preparation"));
    assert!(!root.path().join("target/imap-075/lane-unit.json").exists());
}
#[test]
fn managed_ci_leaf_has_no_recursive_builds_or_children() {
    println!("ci-owned-leaf-complete");
}
#[test]
fn actual_lane_adapters_use_owned_bounded_processes_and_refuse_missing_receipts() {
    let root = fixture();
    let missing = root.path().join("missing-receipt.json");
    assert!(check_preparation(root.path(), &missing).is_err());
    assert!(check_extended(root.path(), &missing).is_err());
    let blocked = root.path().join("blocked-root");
    fs::write(&blocked, b"file").unwrap();
    assert!(bounded_tooling(&blocked, Command::new("unit-never-spawned")).is_err());
    assert!(bounded_tooling(
        root.path(),
        Command::new(root.path().join("missing-command"))
    )
    .is_err());
    let mut command = Command::new(std::env::current_exe().unwrap());
    let (_, module) = module_path!().split_once("::").unwrap();
    command.args([
        "--exact",
        &format!("{module}::managed_ci_leaf_has_no_recursive_builds_or_children"),
        "--nocapture",
    ]);
    bounded_tooling(root.path(), command).unwrap();
    let logs = fs::read_dir(root.path().join("target/imap-075/builds"))
        .unwrap()
        .flat_map(|dir| fs::read_dir(dir.unwrap().path()).unwrap())
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "stdout"))
        .flat_map(|path| fs::read(path).unwrap())
        .collect::<Vec<_>>();
    assert!(String::from_utf8_lossy(&logs).contains("ci-owned-leaf-complete"));
    assert!(!root.path().join("target/imap-075/lane-unit.json").exists());
}
