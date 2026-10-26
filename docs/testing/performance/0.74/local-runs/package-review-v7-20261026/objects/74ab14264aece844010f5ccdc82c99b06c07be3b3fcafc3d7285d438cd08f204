#[test]
fn bootstrap_child_complete_census_exists_before_authentication() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    assert!(
        root.join("tools/long-run-supervisor-074/src/diagnostic_bootstrap_child_roles.rs")
            .is_file(),
        "complete child role census absent"
    );
}

#[path = "support/bootstrap_roles_guard.rs"]
mod guard;
fn source(name: &str) -> String {
    bootstrap_docs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tools/long-run-supervisor-074/src")
            .join(name),
    )
    .unwrap()
    .replace('\r', "")
}
fn baseline(path: &str) -> String {
    let out = std::process::Command::new("git")
        .args(["--no-replace-objects", "show", &format!("851763a0:{path}")])
        .current_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .output()
        .unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout).unwrap().replace('\r', "")
}
#[test]
fn bootstrap_roles_exact_hooks_keep_whole_old_entry_mode_and_original_frames() {
    for name in [
        "bin/diagnostic_held_bootstrap.rs",
        "diagnostic_bootstrap_child_reader.rs",
        "diagnostic_bootstrap_child_reader_tests.rs",
        "diagnostic_bootstrap_child_ingress.rs",
        "diagnostic_bootstrap_owner.rs",
        "diagnostic_bootstrap_entry_format.rs",
        "diagnostic_held_launcher_kernel.rs",
        "diagnostic_held_unit_material_kernel.rs",
        "diagnostic_original_manager_message.rs",
        "diagnostic_root_guardian_raw.rs",
    ] {
        let raw = source(name);
        let old = baseline(&format!("tools/long-run-supervisor-074/src/{name}"));
        assert_eq!(guard::source_view(name, &raw), old);
        if name == "bin/diagnostic_held_bootstrap.rs"
            || name == "diagnostic_bootstrap_child_reader.rs"
        {
            assert!(
                std::panic::catch_unwind(|| guard::source_view(name, &format!("{raw}{raw}")))
                    .is_err()
            );
            assert_ne!(
                guard::source_view(name, &format!("{raw}\n// unrelated drift\n")),
                old
            );
        }
    }
}
#[test]
fn bootstrap_roles_additive_compat_and_qualification_are_exact() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let raw = bootstrap_docs::read_to_string(root.join("docs/COMPAT.md"))
        .unwrap()
        .replace('\r', "");
    assert_eq!(guard::compat_view(&raw), baseline("docs/COMPAT.md"));
    for bad in [
        raw.replace(guard::BLOCK, ""),
        format!("{raw}{}", guard::BLOCK),
        raw.replace("exactly 0..14 plus", "exactly 0..15 plus"),
    ] {
        assert!(std::panic::catch_unwind(|| guard::compat_view(&bad)).is_err());
    }
    for name in [
        "Cargo.lock",
        "tools/long-run-supervisor-074/Cargo.toml",
        "docs/testing/gated-test-registry.toml",
        "docs/testing/performance/0.74/qualification-manifest.toml",
    ] {
        let raw = bootstrap_docs::read_to_string(root.join(name))
            .unwrap()
            .replace('\r', "");
        assert_eq!(
            if name == "docs/testing/gated-test-registry.toml" {
                guard::registry_view(&raw)
            } else {
                raw
            },
            baseline(name)
        );
    }
}
#[test]
fn bootstrap_roles_add_no_execution_repair_or_authority_and_close_extra_fds() {
    for name in [
        "diagnostic_bootstrap_census_entry.rs",
        "diagnostic_bootstrap_child_roles.rs",
        "diagnostic_bootstrap_child_role_kernel.rs",
        "diagnostic_bootstrap_closed_inventory.rs",
    ] {
        let raw = source(name);
        let runtime = raw.split("#[cfg(test)]").next().unwrap();
        let code: String = runtime
            .lines()
            .filter(|s| !s.trim_start().starts_with("//"))
            .collect();
        for token in [
            "execveat",
            "libc::send",
            "libc::dup",
            "libc::fork",
            "F_SETFD",
            "F_SETFL",
            "setsockopt",
            "timerfd_create",
            "timerfd_settime",
            "Command::",
            "remove_var",
            "set_var",
            "FromRawFd",
            "BorrowedFd",
        ] {
            assert!(!code.contains(token), "{name}/{token}");
        }
    }
    let kernel = source("diagnostic_bootstrap_child_role_kernel.rs");
    assert!(kernel.contains("deadline - after <= 60_000_000_000"));
    assert!(source("diagnostic_bootstrap_closed_inventory.rs").contains("seen == 0xffff"));
    assert!(source("diagnostic_bootstrap_census_entry.rs")
        .contains("original bootstrap authentication is closed"));
}

#[path = "../../../tools/bootstrap-doc-reader-074/io.rs"]
mod bootstrap_docs;
