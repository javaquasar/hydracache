//! Fresh synthetic ingress only; no descriptors, manager, worker or service.
use std::process::{Command, Output, Stdio};
const NAMES: &str = "release:worker:config:creator:root-a:root-b:alarm:bootstrap";
fn attempt(mode: &str, override_value: Option<(&str, &str)>) -> Output {
    let mut command = Command::new("/bin/sh");
    command
        .args([
            "-c",
            "unset PWD; LISTEN_PID=$$; export LISTEN_PID; exec \"$@\"",
            "hc074-activation-control",
        ])
        .arg(env!("CARGO_BIN_EXE_diagnostic_held_launcher"));
    if !mode.is_empty() {
        command.arg(mode);
    }
    command
        .env_clear()
        .env("LISTEN_FDS", "8")
        .env("LISTEN_FDNAMES", NAMES)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    if let Some((key, value)) = override_value {
        command.env(key, value);
    }
    command.output().unwrap()
}
pub(super) fn controls() {
    let output = attempt("--systemd-activation-v1", None);
    assert_eq!(output.status.code(), Some(2));
    // Exact ingress reaches the UNCHANGED core, which cannot proceed without
    // its inherited role objects. This is not a healthy held launch.
    let text = String::from_utf8_lossy(&output.stderr);
    assert!(
        text.contains("Held launcher refused:") && text.contains("held FD protocol"),
        "{text}"
    );
    for override_value in [
        ("LISTEN_FDS", "9"),
        ("LISTEN_FDNAMES", "wrong"),
        ("UNEXPECTED", "value"),
    ] {
        let output = attempt("--systemd-activation-v1", Some(override_value));
        let text = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(2));
        assert!(text.contains("Held activation refused:"), "{text}");
        assert!(!text.contains("Held launcher refused:"));
    }
    let output = attempt("", None);
    assert!(String::from_utf8_lossy(&output.stderr).contains("Held launcher refused:"));
    assert_eq!(output.status.code(), Some(2));
    let output = attempt("--systemd-activation-v2", None);
    assert!(String::from_utf8_lossy(&output.stderr).contains("no command/path fallback"));
    assert_eq!(output.status.code(), Some(2));
}
