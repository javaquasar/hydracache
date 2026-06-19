use std::process::Command;

#[test]
fn sqlparser_absent_from_runtime_graph() {
    for package in [
        "hydracache-core",
        "hydracache",
        "hydracache-db",
        "hydracache-sqlx",
    ] {
        let output = Command::new("cargo")
            .args([
                "tree", "-p", package, "--edges", "normal", "--prefix", "none",
            ])
            .output()
            .expect("cargo tree should run");

        assert!(
            output.status.success(),
            "cargo tree failed for {package}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            !stdout.contains("sqlparser"),
            "sqlparser leaked into normal dependency graph for {package}:\n{stdout}"
        );
    }
}

#[test]
fn cargo_deny_runtime_ban_passes() {
    let output = Command::new("cargo")
        .args(["deny", "check", "bans"])
        .output()
        .expect("cargo deny should run");

    assert!(
        output.status.success(),
        "cargo deny check bans failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
