#[path = "support/fresh_status_guard.rs"]
mod guard;
const BASE: &str = "94e53f29bab80734862673063bb3e9e114838ef0";
fn read(path: &str) -> String {
    std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path),
    )
    .unwrap()
    .replace('\r', "")
}
fn old(path: &str) -> String {
    let out = std::process::Command::new("git")
        .args(["show", &format!("{BASE}:{path}")])
        .output()
        .unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout).unwrap().replace('\r', "")
}
#[test]
fn fresh_status_reconstructs_all_changed_old_bodies_and_hooks() {
    for path in guard::CHANGES
        .iter()
        .map(|c| c.0)
        .chain(guard::HOOKS.iter().map(|h| h.0))
    {
        assert_eq!(guard::normalize(path, &read(path)), old(path), "{path}");
    }
}
#[test]
fn fresh_status_adapter_refuses_missing_duplicate_and_other_validation_drift() {
    for (path, token) in guard::CHANGES
        .iter()
        .map(|c| (c.0, c.1))
        .chain(guard::HOOKS.iter().map(|h| (h.0, h.1)))
    {
        let raw = read(path);
        for bad in [raw.replacen(token, "", 1), format!("{raw}{token}")] {
            assert!(std::panic::catch_unwind(|| guard::normalize(path, &bad)).is_err());
        }
    }
    for (path, token) in [
        (guard::CHANGES[0].0, "filesystem(&file, 0x9fa0)?;"),
        (guard::CHANGES[0].0, "libc::O_NOFOLLOW"),
        (guard::CHANGES[1].0, "origin.revalidate()?;"),
        (
            guard::CHANGES[1].0,
            "alive(&self.pidfd, self.credentials.pid)?;",
        ),
        (
            guard::CHANGES[1].0,
            "projection(&bounded(file)?, &self.credentials)? != self.projection",
        ),
        (guard::CHANGES[1].0, "const HOLD_NS: u64 = 60_000_000_000;"),
    ] {
        let raw = read(path);
        let bad = raw.replacen(token, "/* validation changed */", 1);
        assert_ne!(bad, raw);
        assert_ne!(guard::normalize(path, &bad), old(path));
    }
}
