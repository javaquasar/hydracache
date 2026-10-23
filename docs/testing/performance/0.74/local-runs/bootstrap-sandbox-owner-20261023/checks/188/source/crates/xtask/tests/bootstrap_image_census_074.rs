#[path = "support/bootstrap_entry_guard.rs"]
mod entry_guard;
#[path = "support/bootstrap_image_guard.rs"]
mod guard;
fn source(name: &str) -> String {
    std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tools/long-run-supervisor-074/src")
            .join(name),
    )
    .unwrap()
    .replace('\r', "")
}
#[test]
fn bootstrap_image_census_has_closed_twenty_six_descriptors_and_fourteen_roles() {
    let raw = source("diagnostic_bootstrap_image_census.rs");
    assert!(raw.contains("roles: [i32; 14]"));
    assert!(raw.contains("fds: [i32; 26]"));
}
#[test]
fn bootstrap_image_census_is_inside_the_same_existing_validation_observation() {
    let raw = source("diagnostic_bootstrap_owner.rs");
    assert!(raw.contains("census: BootstrapImageBindings"));
    assert_eq!(raw.matches("owner.full_frame()?;").count(), 3);
}
fn baseline(name: &str) -> String {
    let path = format!("e40d4732:tools/long-run-supervisor-074/src/{name}");
    let out = std::process::Command::new("git")
        .args(["--no-replace-objects", "show", &path])
        .current_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .output()
        .unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout).unwrap().replace('\r', "")
}
#[test]
fn bootstrap_image_census_exact_hooks_preserve_whole_old_owner_and_all_old_queries() {
    for name in [
        "diagnostic_held_role_files.rs",
        "diagnostic_held_role_census.rs",
        "diagnostic_held_unit_material_kernel.rs",
    ] {
        let raw = source(name);
        assert_eq!(guard::provider_view(name, &raw), baseline(name));
        assert!(
            std::panic::catch_unwind(|| guard::provider_view(name, &format!("{raw}{raw}")))
                .is_err()
        );
        assert_ne!(
            guard::provider_view(name, &format!("{raw}\n// unrelated drift\n")),
            baseline(name)
        );
    }
    let raw = source("diagnostic_bootstrap_owner.rs");
    assert_eq!(
        guard::owner_view(&raw),
        baseline("diagnostic_bootstrap_owner.rs")
    );
    for bad in [
        raw.replace("    census: BootstrapImageBindings,\n", ""),
        raw.replace("&original.retained_roles,", "&other.retained_roles,"),
        raw.replace("census,\n", "other_census,\n"),
    ] {
        assert!(std::panic::catch_unwind(|| guard::owner_view(&bad)).is_err());
    }
    assert_ne!(
        guard::owner_view(&raw.replace("owner.full_frame()?;", "owner.inspect()?;")),
        baseline("diagnostic_bootstrap_owner.rs")
    );
}
#[test]
fn bootstrap_image_census_exports_no_fd_mapping_transport_or_new_clock() {
    for name in [
        "diagnostic_bootstrap_image_census.rs",
        "diagnostic_bootstrap_image_provider.rs",
        "diagnostic_bootstrap_original_inventory.rs",
    ] {
        let raw = source(name);
        let runtime = raw.split("#[cfg(test)]").next().unwrap();
        let code: String = runtime
            .lines()
            .filter(|s| !s.trim_start().starts_with("//"))
            .collect();
        for token in [
            "pub fn",
            "fn as_raw_fd",
            "IntoRawFd",
            "BorrowedFd",
            "try_clone",
            "File::open",
            "OpenOptions",
            "from_raw_fd",
            "dup2",
            "dup3",
            "sendmsg",
            "execve",
            "Command",
            "Message",
            "Connection",
            "clock_gettime",
            "Instant",
            "Serialize",
            "Deserialize",
            "Debug",
            "Clone",
            "Deref",
        ] {
            assert!(!code.contains(token), "{name}/{token}");
        }
    }
    for name in [
        "diagnostic_held_unit_material.rs",
        "diagnostic_consuming_held_join.rs",
        "diagnostic_held_unit_projection.rs",
        "diagnostic_held_launcher_kernel.rs",
        "diagnostic_held_activation_ingress.rs",
        "diagnostic_held_runtime_trust.rs",
        "diagnostic_bootstrap_selected_build.rs",
        "diagnostic_bootstrap_build.rs",
        "diagnostic_original_manager_message.rs",
        "diagnostic_root_guardian_raw.rs",
    ] {
        assert_eq!(
            entry_guard::source_view(name, &source(name)),
            baseline(name),
            "{name}"
        );
    }
    for name in [
        "Cargo.lock",
        "docs/COMPAT.md",
        "docs/testing/gated-test-registry.toml",
        "docs/testing/performance/0.74/qualification-manifest.toml",
    ] {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let raw = std::fs::read_to_string(root.join(name))
            .unwrap()
            .replace('\r', "");
        let raw = if name == "docs/COMPAT.md" {
            entry_guard::compat_view(&raw)
        } else if name == "docs/testing/gated-test-registry.toml" {
            entry_guard::registry_view(&raw)
        } else {
            raw
        };
        let out = std::process::Command::new("git")
            .args(["--no-replace-objects", "show", &format!("e40d4732:{name}")])
            .current_dir(root)
            .output()
            .unwrap();
        assert!(out.status.success());
        assert_eq!(
            raw,
            String::from_utf8(out.stdout).unwrap().replace('\r', "")
        );
    }
}
