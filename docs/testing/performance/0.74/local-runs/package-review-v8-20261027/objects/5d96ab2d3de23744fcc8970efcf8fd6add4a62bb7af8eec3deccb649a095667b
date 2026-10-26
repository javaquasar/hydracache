#[path = "support/fresh_status_guard.rs"]
mod guard;
#[path = "../../../tools/authority-runtime-074/gate.rs"]
mod runtime;
#[path = "../../../tools/authority-runtime-074/io.rs"]
mod runtime_io;
#[path = "../../../tools/authority-storage-source-074/contract.rs"]
mod storage_contract;
const BASE: &str = "94e53f29bab80734862673063bb3e9e114838ef0";
fn read(path: &str) -> String {
    let loader = if path == runtime::status_fields::SOURCE {
        runtime_io::read_to_string
    } else {
        std::fs::read_to_string
    };
    let raw = loader(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path),
    )
    .unwrap()
    .replace('\r', "");
    let raw = if path == "tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs" {
        runtime::successor::project(path, raw)
    } else {
        raw
    };
    guard::pid_path::custody_cost::document_scratch::normalize(path, &raw)
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
