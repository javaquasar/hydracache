#[test]
fn installed_origin_exists_before_systemd_or_child_admission() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    assert!(
        root.join("tools/long-run-supervisor-074/src/diagnostic_bootstrap_installed_owner.rs")
            .is_file(),
        "installed origin owner binding is absent"
    );
}

#[path = "support/bootstrap_installed_guard.rs"]
mod guard;
#[test]
fn bootstrap_installed_entry_component_precedes_runtime() {
    assert!(
        root().join("tools/long-run-supervisor-074/src/diagnostic_bootstrap_installed_entry_format.rs").is_file(),
        "separate installed entry assertion codec is absent"
    );
}
fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path))
        .unwrap()
        .replace('\r', "")
}
fn source(name: &str) -> String {
    read(&format!("tools/long-run-supervisor-074/src/{name}"))
}
fn baseline(path: &str) -> String {
    let out = std::process::Command::new("git")
        .current_dir(root())
        .args(["--no-replace-objects", "show", &format!("c506d0cc:{path}")])
        .output()
        .unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout).unwrap().replace('\r', "")
}
#[test]
fn installed_origin_only_three_additive_hooks_preserve_old_live_frames_and_child_modes() {
    for name in [
        "diagnostic_bootstrap_build.rs",
        "diagnostic_bootstrap_owner.rs",
        "diagnostic_bootstrap_entry_census.rs",
    ] {
        let raw = source(name);
        let old = baseline(&format!("tools/long-run-supervisor-074/src/{name}"));
        assert_eq!(guard::source_view(name, &raw), old);
        assert!(
            std::panic::catch_unwind(|| guard::source_view(name, &format!("{raw}{raw}"))).is_err()
        );
        assert_ne!(
            guard::source_view(name, &format!("{raw}\n// unrelated drift\n")),
            old
        );
    }
    for name in [
        "bin/diagnostic_held_bootstrap.rs",
        "diagnostic_bootstrap_current_image.rs",
        "diagnostic_bootstrap_child_reader.rs",
        "diagnostic_bootstrap_child_roles.rs",
        "diagnostic_held_unit_projection.rs",
        "diagnostic_original_manager_message.rs",
        "diagnostic_root_guardian_raw.rs",
        "diagnostic_bootstrap_entry_binding.rs",
    ] {
        assert_eq!(
            source(name),
            baseline(&format!("tools/long-run-supervisor-074/src/{name}"))
        );
    }
}
#[test]
fn installed_origin_compat_is_additive_and_qualification_inputs_stay_frozen() {
    let raw = read("docs/COMPAT.md");
    assert_eq!(guard::compat_view(&raw), baseline("docs/COMPAT.md"));
    for bad in [
        raw.replace(guard::BLOCK, ""),
        format!("{raw}{}", guard::BLOCK),
        raw.replace("32 FDs/16 roles", "32 FDs/17 roles"),
    ] {
        assert!(std::panic::catch_unwind(|| guard::compat_view(&bad)).is_err());
    }
    for name in [
        "Cargo.lock",
        "tools/long-run-supervisor-074/Cargo.toml",
        "docs/testing/gated-test-registry.toml",
        "docs/testing/performance/0.74/qualification-manifest.toml",
    ] {
        assert_eq!(read(name), baseline(name));
    }
}
#[test]
fn installed_origin_consumes_same_owner_and_cannot_recreate_retained_source_or_export_execution() {
    let owner = source("diagnostic_bootstrap_installed_owner.rs");
    assert!(owner.contains("self.owner.revalidate()"));
    assert!(owner.contains("self.owner.refuse();"));
    assert!(owner.contains("self.owner.selection.as_ref()") || owner.contains(".selection\n"));
    assert!(owner.contains("if self.source.is_none()"));
    assert!(owner.contains("if self.retained.is_some()"));
    assert!(owner.contains("InstalledSource::open_fixed(build)"));
    for name in [
        "diagnostic_bootstrap_installed_source.rs",
        "diagnostic_bootstrap_installed_owner.rs",
        "diagnostic_bootstrap_installed_census.rs",
    ] {
        let raw = source(name);
        let code: String = raw
            .split("#[cfg(test)]")
            .next()
            .unwrap()
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect();
        for token in [
            "execve",
            "sendmsg",
            "Command::",
            "clock_gettime",
            "SystemTime",
            "Instant",
            "IntoRawFd",
            "try_clone",
            "setns",
            "unshare",
            "F_SETFD",
            "F_SETFL",
            "Serialize",
            "Deserialize",
            "Deref",
            "pub fn",
            "pub struct",
        ] {
            assert!(!code.contains(token), "{name}/{token}");
        }
    }
    let source = source("diagnostic_bootstrap_installed_source.rs");
    assert!(source.contains("O_NOFOLLOW"));
    assert!(source.contains("libc::geteuid() } == 0"));
    assert!(source.contains("fn open_under("));
    assert!(!source.contains("pub(crate) fn open_under("));
    let census = crate::source("diagnostic_bootstrap_installed_census.rs");
    assert!(census.contains("fds[..27].copy_from_slice(&self.fds)"));
    assert!(census.contains("roles[..15].copy_from_slice(&self.roles)"));
    assert!(census.contains("need(distinct(&fds))"));
}
