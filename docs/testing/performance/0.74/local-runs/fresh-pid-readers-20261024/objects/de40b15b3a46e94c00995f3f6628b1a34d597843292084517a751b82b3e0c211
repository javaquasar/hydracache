#[path = "support/pid_path_guard.rs"]
mod guard;
const BASE: &str = "b63a0e81ad808d6f65e8c8d818ee4c090c3718ff";
fn read(path: &str) -> String {
    let raw = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path),
    )
    .unwrap()
    .replace('\r', "");
    guard::custody_cost::document_scratch::normalize(path, &raw)
}
fn old(commit: &str, path: &str) -> String {
    let out = std::process::Command::new("git")
        .args(["show", &format!("{commit}:{path}")])
        .output()
        .unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout).unwrap().replace('\r', "")
}
#[test]
fn pid_path_reconstructs_every_changed_old_body_and_test_hook() {
    for path in [guard::ORIGIN, guard::ADMISSION, guard::HOOKS[1].0] {
        assert_eq!(
            guard::normalize(path, &read(path)),
            old(BASE, path),
            "{path}"
        );
    }
}
#[test]
fn pid_path_adopted_formatter_is_exactly_the_measured_frozen_prototype() {
    let raw = old(
        "0d7717686a06cec7178a1965e02363fc43a63da8",
        "tools/long-run-supervisor-074/tests/support/pid_path.rs",
    );
    let start = raw.find("// BEGIN STACK PID NAME").unwrap();
    let end = raw.find("// END STACK PID NAME").unwrap() + "// END STACK PID NAME".len();
    assert_eq!(guard::STACK.trim(), &raw[start..end]);
    assert_eq!(read(guard::ORIGIN).matches(guard::STACK).count(), 1);
}
#[test]
fn pid_path_adapter_refuses_missing_duplicate_and_unrelated_live_drift() {
    for (path, token) in guard::HOOKS
        .into_iter()
        .chain([(guard::ORIGIN, guard::STACK)])
        .chain(guard::CHANGES.into_iter().map(|(p, n, _)| (p, n)))
    {
        let raw = read(path);
        for bad in [raw.replacen(token, "", 1), format!("{raw}{token}")] {
            assert!(std::panic::catch_unwind(|| guard::normalize(path, &bad)).is_err());
        }
    }
    for (path, token) in [
        (guard::ORIGIN, "filesystem(&file, 0x9fa0)?;"),
        (guard::ORIGIN, "libc::O_NOFOLLOW"),
        (
            guard::ADMISSION,
            "alive(&self.pidfd, self.credentials.pid)?;",
        ),
        (guard::ADMISSION, "origin.revalidate()?;"),
        (guard::ADMISSION, "const HOLD_NS: u64 = 60_000_000_000;"),
    ] {
        let raw = read(path);
        let bad = raw.replacen(token, "/* changed live check */", 1);
        assert_ne!(raw, bad);
        assert_ne!(guard::normalize(path, &bad), old(BASE, path));
    }
}
