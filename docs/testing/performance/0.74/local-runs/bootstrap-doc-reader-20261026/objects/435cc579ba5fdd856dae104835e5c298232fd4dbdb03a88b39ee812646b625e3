#[test]
fn bootstrap_current_executable_component_is_present_before_admission() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    assert!(
        root.join("tools/long-run-supervisor-074/src/diagnostic_bootstrap_current_image.rs")
            .is_file(),
        "current executable inode consistency is absent"
    );
}

#[path = "support/bootstrap_current_image_guard.rs"]
mod guard;
fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn current(path: &str) -> String {
    bootstrap_docs::read_to_string(root().join(path))
        .unwrap()
        .replace('\r', "")
}
fn baseline(path: &str) -> String {
    let out = std::process::Command::new("git")
        .args(["--no-replace-objects", "show", &format!("c3dd8469:{path}")])
        .current_dir(root())
        .output()
        .unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout).unwrap().replace('\r', "")
}
#[test]
fn current_image_only_adds_exact_hooks_and_keeps_old_modes_frames_and_deadlines() {
    for name in [
        "bin/diagnostic_held_bootstrap.rs",
        "diagnostic_bootstrap_child_reader.rs",
        "diagnostic_bootstrap_child_roles.rs",
        "diagnostic_bootstrap_census_entry.rs",
        "diagnostic_bootstrap_child_role_kernel.rs",
        "diagnostic_bootstrap_closed_inventory.rs",
        "diagnostic_bootstrap_owner.rs",
        "diagnostic_bootstrap_entry_format.rs",
        "diagnostic_held_launcher_kernel.rs",
        "diagnostic_held_unit_material_kernel.rs",
        "diagnostic_original_manager_message.rs",
        "diagnostic_root_guardian_raw.rs",
    ] {
        let path = format!("tools/long-run-supervisor-074/src/{name}");
        let raw = current(&path);
        assert_eq!(guard::source_view(name, &raw), baseline(&path));
        if name == "bin/diagnostic_held_bootstrap.rs"
            || name == "diagnostic_bootstrap_child_reader.rs"
        {
            assert!(
                std::panic::catch_unwind(|| guard::source_view(name, &format!("{raw}{raw}")))
                    .is_err()
            );
            assert_ne!(
                guard::source_view(name, &format!("{raw}\n// drift\n")),
                baseline(&path)
            );
        }
    }
    for name in [
        "bootstrap_child_census.rs",
        "bootstrap_child_reader.rs",
        "support/bootstrap_census_material.rs",
    ] {
        let path = format!("tools/long-run-supervisor-074/tests/{name}");
        assert_eq!(current(&path), baseline(&path));
    }
}
#[test]
fn current_image_compat_registry_and_qualification_remain_exact() {
    for (name, view, addition) in [
        (
            "docs/COMPAT.md",
            guard::compat_view as fn(&str) -> String,
            guard::BLOCK,
        ),
        (
            "docs/testing/gated-test-registry.toml",
            guard::registry_view as fn(&str) -> String,
            guard::GATE,
        ),
    ] {
        let raw = current(name);
        assert_eq!(view(&raw), baseline(name));
        for bad in [
            raw.replace(addition, ""),
            format!("{raw}{addition}"),
            raw.replace(addition, &format!("!{}", &addition[1..])),
        ] {
            assert!(std::panic::catch_unwind(|| view(&bad)).is_err());
        }
    }
    for name in [
        "Cargo.lock",
        "tools/long-run-supervisor-074/Cargo.toml",
        "docs/testing/performance/0.74/qualification-manifest.toml",
    ] {
        assert_eq!(current(name), baseline(name));
    }
}
#[test]
fn current_image_queries_cannot_repair_roles_send_or_execute() {
    for name in [
        "diagnostic_bootstrap_current_image.rs",
        "diagnostic_bootstrap_image_entry.rs",
    ] {
        let raw = current(&format!("tools/long-run-supervisor-074/src/{name}"));
        let code: String = raw
            .split("#[cfg(test)]")
            .next()
            .unwrap()
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect();
        for token in [
            "libc::exec",
            "libc::send",
            "libc::dup",
            "libc::fork",
            "F_SETFD",
            "F_SETFL",
            "timerfd_create",
            "timerfd_settime",
            "Command::",
            "set_var",
            "remove_var",
            "pub fn",
            "pub struct",
            "pub use",
        ] {
            assert!(!code.contains(token), "{name}/{token}");
        }
    }
    let code = current("tools/long-run-supervisor-074/src/diagnostic_bootstrap_image_entry.rs");
    let order = [
        "roles::observe",
        "current_image::observe",
        "roles::revalidate",
        "current_image::revalidate",
    ];
    let mut at = 0;
    for token in order {
        at += code[at..].find(token).unwrap() + token.len();
    }
    assert!(code.contains("original bootstrap authentication is closed"));
}

#[path = "../../../tools/bootstrap-doc-reader-074/io.rs"]
mod bootstrap_docs;
