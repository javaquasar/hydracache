use std::path::Path;
#[path = "support/bootstrap_build_guard.rs"]
mod build_guard;
#[path = "support/bootstrap_image_guard.rs"]
mod image_guard;
fn root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn read(path: &str) -> String {
    let raw = std::fs::read_to_string(root().join(path))
        .unwrap()
        .replace('\r', "");
    if let Some(name) = path.strip_prefix("tools/long-run-supervisor-074/src/") {
        image_guard::provider_view(name, &raw)
    } else {
        raw
    }
}
#[test]
fn rejected_bind_alias_does_not_install_unproved_runtime_or_change_old_bytes() {
    for file in [
        "diagnostic_held_role_files.rs",
        "diagnostic_held_unit_material.rs",
        "diagnostic_held_role_census.rs",
        "diagnostic_held_unit_projection.rs",
        "diagnostic_consuming_held_join.rs",
        "diagnostic_held_launcher_kernel.rs",
        "diagnostic_held_activation_ingress.rs",
        "diagnostic_manager_channel.rs",
        "diagnostic_original_manager_message.rs",
        "bin/diagnostic_held_launcher.rs",
    ] {
        let path = format!("tools/long-run-supervisor-074/src/{file}");
        let old = std::process::Command::new("git")
            .args(["--no-replace-objects", "show", &format!("17b9c689:{path}")])
            .current_dir(root())
            .output()
            .unwrap();
        assert!(old.status.success());
        assert_eq!(
            if file == "diagnostic_held_unit_material.rs" {
                build_guard::material_view(&read(&path))
            } else {
                read(&path)
            },
            String::from_utf8(old.stdout).unwrap().replace('\r', ""),
            "{path}"
        );
    }
    for file in [
        "diagnostic_launcher_alias_path.rs",
        "diagnostic_launcher_bind_alias.rs",
    ] {
        assert!(!root()
            .join("tools/long-run-supervisor-074/src")
            .join(file)
            .exists());
        assert!(read(&format!(
            "docs/testing/performance/0.74/rejected-candidates/held-bind-alias-20261023/{file}.txt"
        ))
        .contains("REJECTED UNFINISHED CANDIDATE"));
    }
}
#[test]
fn memfd_probe_has_no_retry_copy_fallback_or_automatic_promotion() {
    let probe = read("tools/held-bind-alias-074/probe.py");
    assert_eq!(probe.matches("tree = syscall(428,").count(), 1);
    assert!(
        probe.contains("os.stat('/proc/self/ns/mnt').st_ino != os.stat('/proc/1/ns/mnt').st_ino")
    );
    assert!(probe.contains("fcntl.fcntl(source, fcntl.F_ADD_SEALS, 15)"));
    let control = read("tools/held-bind-alias-074/regular_control.py");
    assert!(control.contains("original_candidate_pass=False"));
    assert!(control.contains("eligible_original_launcher=False"));
    assert!(control.contains("os.kill(child, signal.SIGKILL)"));
    for word in ["systemctl", "ssh", "retry", "copyfile", "subprocess"] {
        assert!(!probe.contains(word), "{word}");
    }
}
