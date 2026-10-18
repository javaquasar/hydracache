//! Parser-only acceptance; the two-child owner-fault candidate is withdrawn.
fn root() -> std::path::PathBuf {
    super::super::super::root()
}
fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path))
        .unwrap()
        .replace('\r', "")
}
#[test]
fn collector_notice_tests_preserve_the_entire_ordinary_kernel_and_join() {
    let prefix = "tools/long-run-supervisor-074/src/";
    for name in [
        "diagnostic_original_barred_collector_kernel.rs",
        "diagnostic_original_barred_collector.rs",
        "diagnostic_original_barred_collector_test_api.rs",
        "diagnostic_original_barred_collector_tests.rs",
    ] {
        let path = format!("{prefix}{name}");
        let out = std::process::Command::new("git")
            .args(["show", &format!("1c56d4a3:{path}")])
            .current_dir(root())
            .output()
            .unwrap();
        assert!(out.status.success());
        let frozen = String::from_utf8(out.stdout).unwrap().replace('\r', "");
        let raw = read(&path);
        let current = if name.ends_with("_kernel.rs") {
            let registration = "\n#[cfg(test)]\n#[path = \"diagnostic_original_collector_notice_tests.rs\"]\nmod notice_tests;\n";
            assert_eq!(raw.matches(registration).count(), 1);
            raw.replacen(registration, "", 1)
        } else {
            raw
        };
        assert_eq!(current, frozen, "{path}");
    }
}
#[test]
fn collector_notice_units_cover_shape_and_readiness_without_execution() {
    let raw =
        read("tools/long-run-supervisor-074/src/diagnostic_original_collector_notice_tests.rs");
    for needed in [
        "b\"BB\"",
        "b\"P\"",
        "b\"X\"",
        "drop(peer)",
        "original_collector_notice_alarm_precedes_queued_b",
        "original_collector_notice_terminal_precedes_queued_b",
        "assert_eq!(byte, b'B');",
        "pid: 0",
    ] {
        assert!(raw.contains(needed), "{needed}");
    }
    for denied in ["SYS_clone3", "SYS_pidfd_send_signal", "timerfd_settime"] {
        assert!(!raw.contains(denied));
    }
}
