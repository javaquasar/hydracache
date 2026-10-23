use std::path::Path;
#[path = "support/bootstrap_build_guard.rs"]
mod build_guard;
#[path = "support/bootstrap_entry_guard.rs"]
mod entry_guard;
#[path = "support/bootstrap_enrollment_guard.rs"]
mod guard;
fn read(path: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path),
    )
    .unwrap()
    .replace('\r', "")
}

#[test]
fn bootstrap_selection_compatibility_is_exactly_additive_and_mutations_refuse() {
    let raw = build_guard::compat_view(&read("docs/COMPAT.md"));
    let old = std::process::Command::new("git")
        .args(["--no-replace-objects", "show", "17b9c689:docs/COMPAT.md"])
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .output()
        .unwrap();
    assert!(old.status.success());
    assert_eq!(
        guard::compat_view(&raw),
        String::from_utf8(old.stdout).unwrap().replace('\r', "")
    );
    for bad in [
        raw.replace(guard::BLOCK, ""),
        format!("{raw}{}", guard::BLOCK),
        raw.replace("Digests are nonzero;", "Digests are unchecked;"),
    ] {
        assert!(std::panic::catch_unwind(|| guard::compat_view(&bad)).is_err());
    }
}
#[test]
fn bootstrap_selector_keeps_original_checks_without_key_fd_or_execution_exports() {
    let raw = read("tools/long-run-supervisor-074/src/diagnostic_bootstrap_enrollment.rs");
    let runtime = raw.split("#[cfg(test)]").next().unwrap();
    assert_eq!(runtime.matches("pub fn ").count(), 3);
    assert_eq!(runtime.matches("self.revalidate()?;").count(), 2);
    assert_eq!(runtime.matches("self.held.revalidate()?;").count(), 2);
    assert!(runtime.contains("self.retained.same(&snapshot(&self.held.source)?)?;"));
    let code: String = runtime
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect();
    for token in [
        "VerifyingKey::from_bytes",
        "SigningKey",
        "pub fn open",
        "AsRawFd",
        "IntoRawFd",
        "try_clone",
        "Command",
        "Connection",
        "Message",
        "clock_gettime",
        "SystemTime",
        "Instant",
        "execve",
        "sendmsg",
        "env::",
    ] {
        assert!(!code.contains(token), "{token}");
    }
}
#[test]
fn independent_bootstrap_selection_has_only_one_additive_parent_hook() {
    let raw = read("tools/long-run-supervisor-074/src/diagnostic_held_runtime_trust.rs");
    let hook =
        "\n#[path = \"diagnostic_bootstrap_enrollment.rs\"]\npub mod bootstrap_enrollment;\n";
    assert_eq!(raw.matches(hook).count(), 1);
    let result = std::process::Command::new("git")
        .args([
            "--no-replace-objects",
            "show",
            "17b9c689:tools/long-run-supervisor-074/src/diagnostic_held_runtime_trust.rs",
        ])
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .output()
        .unwrap();
    assert!(result.status.success());
    assert_eq!(
        raw.replacen(hook, "", 1),
        String::from_utf8(result.stdout).unwrap().replace('\r', "")
    );
}
