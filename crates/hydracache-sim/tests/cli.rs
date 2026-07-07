use std::process::Command;

use serde_json::Value;

fn run_vopr(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_vopr"))
        .args(args)
        .output()
        .expect("vopr binary runs")
}

fn stdout_utf8(output: std::process::Output) -> String {
    String::from_utf8(output.stdout).expect("stdout is utf8")
}

fn stderr_utf8(output: &std::process::Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr is utf8")
}

#[test]
fn vopr_soak_subcommand_exits_2_on_failure() {
    let output = run_vopr(&[
        "soak",
        "--master-seed",
        "22530",
        "--budget-ms",
        "0",
        "--steps-per-seed",
        "4",
        "--max-seeds",
        "1",
        "--synthetic-failure-after-seeds",
        "1",
    ]);

    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let stdout = stdout_utf8(output);
    assert!(stdout.contains("\"status\":\"failed\""), "{stdout}");
    assert!(stdout.contains("synthetic_soak_failure"), "{stdout}");
    assert!(stdout.contains("vopr --seed"), "{stdout}");
}

#[test]
fn vopr_single_shot_prints_machine_readable_summary() {
    let output = run_vopr(&["--seed", "44", "--steps", "1"]);

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let stdout = stdout_utf8(output);
    assert!(stdout.contains("seed=44"), "{stdout}");
    assert!(stdout.contains("steps=1"), "{stdout}");
    assert!(stdout.contains("accepted_ops="), "{stdout}");
    assert!(stdout.contains("delivered_messages="), "{stdout}");
    assert!(stdout.contains("history_hash="), "{stdout}");
    assert!(stdout.contains("invariant_violations=0"), "{stdout}");
}

#[test]
fn vopr_soak_success_prints_score_free_json_report() {
    let output = run_vopr(&[
        "soak",
        "--master-seed",
        "22530",
        "--budget-ms",
        "0",
        "--steps-per-seed",
        "1",
        "--max-seeds",
        "1",
    ]);

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let stdout = stdout_utf8(output);
    let json = serde_json::from_str::<Value>(stdout.trim()).expect("soak report is JSON");
    assert_eq!(json["master_seed"], 22530);
    assert_eq!(json["seeds_run"], 1);
    assert_eq!(json["total_steps"], 1);
    assert_eq!(json["resource_bounds_ok"], true);
    assert_eq!(json["outcome"]["status"], "clean");
    assert!(
        !json_contains_key(&json, "score") && !json_contains_key(&json, "percent"),
        "soak report stays score-free: {json}"
    );
}

#[test]
fn vopr_rejects_invalid_arguments_with_usage() {
    let output = run_vopr(&["soak", "--wat"]);

    assert_eq!(output.status.code(), Some(64), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let stderr = stderr_utf8(&output);
    assert!(stderr.contains("unknown argument '--wat'"), "{stderr}");
    assert!(stderr.contains("usage:"), "{stderr}");
    assert!(stderr.contains("vopr soak"), "{stderr}");
}

fn json_contains_key(value: &Value, needle: &str) -> bool {
    match value {
        Value::Object(map) => map
            .iter()
            .any(|(key, value)| key == needle || json_contains_key(value, needle)),
        Value::Array(values) => values.iter().any(|value| json_contains_key(value, needle)),
        _ => false,
    }
}
