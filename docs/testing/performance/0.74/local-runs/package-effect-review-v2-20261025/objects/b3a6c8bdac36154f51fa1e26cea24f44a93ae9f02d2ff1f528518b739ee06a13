//! Exact additive diagnostic transformation, never a source-admission waiver.
pub const SOURCE: &str = "crates/xtask/tests/diagnostic_artifact_fixture_074.rs";
pub const BASELINE: &str = include_str!("../../docs/testing/performance/0.74/local-runs/transport-package-20261025/objects/5819f89ec58a899e03d7c0f14c6af91f4be6248a26f8578186cfc47b5900a805");
pub const HOOK: &str = "#[path = \"../../../tools/xtask-package-attribution-074/helper_trace.rs\"]\nmod helper_trace;\n";
pub const OLD_READ: &str = r#"fn read(path: &str) -> String {
    runtime_io::read_to_string(root().join(path))
        .unwrap()
        .replace('\r', "")
}
"#;
pub const NEW_READ: &str = r#"fn read(path: &str) -> String {
    let helper = helper_trace::read(path);
    let result = runtime_io::read_to_string(root().join(path))
        .unwrap()
        .replace('\r', "");
    helper_trace::finish(helper, result.len(), 0);
    result
}
"#;
pub const OLD_GIT: &str = r#"fn git(args: &[&str]) -> String {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(root())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let raw = String::from_utf8(output.stdout).unwrap().replace('\r', "");
    if args == ["ls-files", "--", inventory::SCOPE] {
        inventory::project(&raw).expect("current inventory refused before historical phase")
    } else {
        raw
    }
}
"#;
pub const NEW_GIT: &str = r#"fn git(args: &[&str]) -> String {
    let helper = helper_trace::git(args);
    let output_span = helper_trace::output();
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(root())
        .output()
        .unwrap();
    helper_trace::finish(output_span, output.stdout.len(), output.stderr.len());
    let decode = helper_trace::decode();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let raw = String::from_utf8(output.stdout).unwrap().replace('\r', "");
    let result = if args == ["ls-files", "--", inventory::SCOPE] {
        inventory::project(&raw).expect("current inventory refused before historical phase")
    } else {
        raw
    };
    helper_trace::finish(decode, result.len(), output.stderr.len());
    helper_trace::finish(helper, result.len(), output.stderr.len());
    result
}
"#;
pub fn instrument(raw: &str) -> String {
    assert_eq!(raw, BASELINE, "unreviewed helper baseline");
    assert_eq!(raw.matches(OLD_READ).count(), 1);
    assert_eq!(raw.matches(OLD_GIT).count(), 1);
    let transformed = raw
        .replacen(OLD_READ, NEW_READ, 1)
        .replacen(OLD_GIT, NEW_GIT, 1);
    let after_header = transformed.find('\n').unwrap() + 1;
    format!(
        "{}{HOOK}{}",
        &transformed[..after_header],
        &transformed[after_header..]
    )
}
pub fn erase(raw: &str) -> String {
    for exact in [HOOK, NEW_READ, NEW_GIT] {
        assert_eq!(raw.matches(exact).count(), 1);
    }
    let restored = raw
        .replacen(HOOK, "", 1)
        .replacen(NEW_READ, OLD_READ, 1)
        .replacen(NEW_GIT, OLD_GIT, 1);
    assert_eq!(
        restored, BASELINE,
        "observer cannot waive body/assertion drift"
    );
    restored
}
