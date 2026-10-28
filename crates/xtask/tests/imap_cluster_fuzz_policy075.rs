use serde_yaml::Value;
use std::fs;
use std::path::Path;
use xtask::imap_cluster_fuzz_policy075::{check_at_root, check_corpus_at_root};

#[test]
fn retained_corpus_runs_in_pr_lanes_and_refusals_cannot_be_relabelled_success() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    assert_eq!(check_corpus_at_root(&root).unwrap(), 4);
    let temp = tempfile::tempdir().unwrap();
    let relative = "fuzz/corpus/fuzz_imap_cluster_075";
    std::fs::create_dir_all(temp.path().join(relative)).unwrap();
    for name in [
        "point-replay.json",
        "future-version.json",
        "early-response.json",
        "bad-hex.json",
    ] {
        std::fs::copy(
            root.join(relative).join(name),
            temp.path().join(relative).join(name),
        )
        .unwrap();
    }
    assert_eq!(check_corpus_at_root(temp.path()).unwrap(), 4);
    std::fs::write(
        temp.path().join(relative).join("future-version.json"),
        br#"{"version":1,"steps":[]}"#,
    )
    .unwrap();
    assert!(check_corpus_at_root(temp.path()).is_err());
    std::fs::remove_file(temp.path().join(relative).join("point-replay.json")).unwrap();
    assert!(check_corpus_at_root(temp.path()).is_err());
}

const WORKFLOW: &str = include_str!("../../../.github/workflows/ci.yml");
const RUN_STEP: &str = "Run bounded 0.75 IMap cluster fuzz foundation";
const UPLOAD_STEP: &str = "Upload 0.75 IMap cluster fuzz artifacts";

fn check_fixture(workflow: &Value) -> Vec<String> {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join(".github/workflows")).unwrap();
    fs::write(
        root.path().join(".github/workflows/ci.yml"),
        serde_yaml::to_string(workflow).unwrap(),
    )
    .unwrap();
    check_at_root(root.path()).unwrap()
}

#[test]
fn scheduled_cluster_fuzz_keeps_exact_bounds_order_and_failure_artifacts() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    assert!(check_at_root(root).unwrap().is_empty());
    let workflow: Value = serde_yaml::from_str(WORKFLOW).unwrap();
    assert!(check_fixture(&workflow).is_empty());
}

#[test]
fn policy_canaries_reject_removed_unbounded_misplaced_ignored_and_unretained_fuzz() {
    for defect in [
        "command_removed",
        "unbounded_maxlen",
        "misplaced",
        "ignored_failure",
        "missing_artifact",
        "missing_corpus",
        "skipped",
        "unpinned_upload",
        "upload_skipped",
        "job_ignored",
        "duplicate_run",
        "late_cleanup",
        "timeout",
        "shell_success",
    ] {
        let mut workflow: Value = serde_yaml::from_str(WORKFLOW).unwrap();
        if defect == "job_ignored" {
            workflow["jobs"]["fuzz-nightly"]["continue-on-error"] = Value::Bool(true);
        } else {
            let steps = workflow["jobs"]["fuzz-nightly"]["steps"]
                .as_sequence_mut()
                .unwrap();
            let run = steps
                .iter()
                .position(|step| step["name"].as_str() == Some(RUN_STEP))
                .unwrap();
            let upload = steps
                .iter()
                .position(|step| step["name"].as_str() == Some(UPLOAD_STEP))
                .unwrap();
            match defect {
                "command_removed" => {
                    steps[run]["run"] = Value::String("echo fuzz omitted".into());
                }
                "unbounded_maxlen" => {
                    steps[run]["run"] = Value::String(
                        steps[run]["run"]
                            .as_str()
                            .unwrap()
                            .replace("-max_len=65536", "-max_len=0"),
                    );
                }
                "misplaced" => {
                    let step = steps.remove(run);
                    steps.insert(0, step);
                }
                "ignored_failure" => {
                    steps[run]["continue-on-error"] = Value::Bool(true);
                }
                "missing_artifact" => {
                    steps[upload]["with"]["path"] =
                        Value::String("fuzz/corpus/fuzz_imap_cluster_075/**".into());
                }
                "missing_corpus" => {
                    steps[upload]["with"]["path"] =
                        Value::String("fuzz/artifacts/fuzz_imap_cluster_075/**".into());
                }
                "skipped" => {
                    steps[run]["if"] = Value::String("false".into());
                }
                "unpinned_upload" => {
                    steps[upload]["uses"] = Value::String("actions/upload-artifact@v6".into());
                }
                "upload_skipped" => {
                    steps[upload]["if"] = Value::String("success()".into());
                }
                "duplicate_run" => {
                    let step = steps[run].clone();
                    steps.push(step);
                }
                "late_cleanup" => {
                    let step: Value = serde_yaml::from_str(
                        "name: late cleanup\nrun: git clean -fd -- fuzz/corpus\n",
                    )
                    .unwrap();
                    steps.push(step);
                }
                "timeout" => {
                    steps[run]["timeout-minutes"] = Value::Number(11.into());
                }
                "shell_success" => {
                    steps[run]["run"] =
                        Value::String(format!("{} || true", steps[run]["run"].as_str().unwrap()));
                }
                _ => unreachable!(),
            }
        }
        assert!(
            !check_fixture(&workflow).is_empty(),
            "accepted canary: {defect}"
        );
    }
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join(".github/workflows")).unwrap();
    let file = root.path().join(".github/workflows/ci.yml");
    for invalid in [
        "jobs: [".to_owned(),
        "x".repeat(512 * 1024 + 1),
        "jobs: {}".to_owned(),
    ] {
        fs::write(&file, invalid).unwrap();
        assert!(check_at_root(root.path()).is_err());
    }
}
