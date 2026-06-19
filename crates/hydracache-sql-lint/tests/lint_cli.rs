use std::fs;
use std::process::Command;

#[test]
fn check_baseline_passes_empty_current() {
    let root = unique_temp_dir("check_baseline_passes_empty_current");
    fs::create_dir_all(&root).unwrap();
    let baseline = root.join("lint-baseline.json");
    fs::write(&baseline, r#"{"entries":[]}"#).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_lint"))
        .arg("--check-baseline")
        .arg("--baseline")
        .arg(&baseline)
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stdout).contains("baseline passed"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn check_baseline_fails_stale_entry() {
    let root = unique_temp_dir("check_baseline_fails_stale_entry");
    fs::create_dir_all(&root).unwrap();
    let baseline = root.join("lint-baseline.json");
    fs::write(&baseline, r#"{"entries":["stale-fingerprint"]}"#).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_lint"))
        .arg("--check-baseline")
        .arg("--baseline")
        .arg(&baseline)
        .output()
        .unwrap();

    assert!(!output.status.success(), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stderr).contains("stale SQL lint baseline entries"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn update_baseline_uses_diagnostics() {
    let root = unique_temp_dir("update_baseline_uses_diagnostics");
    fs::create_dir_all(&root).unwrap();
    let baseline = root.join("lint-baseline.json");
    let diagnostics = root.join("diagnostics.json");
    fs::write(
        &diagnostics,
        r#"[{"policy":"load-user-roles","finding":{"MissingDependencies":[{"schema":null,"name":"roles"}]},"fingerprint":"known-finding"}]"#,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_lint"))
        .arg("--update-baseline")
        .arg("--baseline")
        .arg(&baseline)
        .arg("--diagnostics")
        .arg(&diagnostics)
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let baseline_json = fs::read_to_string(&baseline).unwrap();
    assert!(baseline_json.contains("known-finding"));
    let _ = fs::remove_dir_all(root);
}

fn unique_temp_dir(name: &str) -> std::path::PathBuf {
    let path =
        std::env::temp_dir().join(format!("hydracache_sql_lint_{name}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    path
}
