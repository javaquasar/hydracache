#[test]
fn installed_origin_exists_before_systemd_or_child_admission() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    assert!(
        root.join("tools/long-run-supervisor-074/src/diagnostic_bootstrap_installed_owner.rs")
            .is_file(),
        "installed origin owner binding is absent"
    );
}

#[test]
fn bootstrap_held_delivery_components_precede_execution_admission() {
    for name in [
        "diagnostic_held_delivery_wire.rs",
        "diagnostic_held_child_exchange.rs",
        "diagnostic_original_held_delivery.rs",
    ] {
        assert!(
            root()
                .join("tools/long-run-supervisor-074/src")
                .join(name)
                .is_file(),
            "held delivery component absent: {name}"
        );
    }
}

#[test]
fn bootstrap_held_delivery_exact_additions_leave_old_frames_and_cli_closed() {
    let view = entry_guard::material::child::sandbox::held::source_view;
    for name in [
        "diagnostic_bootstrap_installed_entry_census.rs",
        "diagnostic_bootstrap_sandbox_owner.rs",
        "diagnostic_manager_channel.rs",
    ] {
        let path = format!("tools/long-run-supervisor-074/src/{name}");
        let old = std::process::Command::new("git")
            .current_dir(root())
            .args(["--no-replace-objects", "show", &format!("ba690541:{path}")])
            .output()
            .unwrap();
        assert!(old.status.success());
        let old = String::from_utf8(old.stdout).unwrap().replace('\r', "");
        let raw = source(name);
        assert_eq!(view(name, &raw), old);
        let hook = entry_guard::material::child::sandbox::held::hook(name).unwrap();
        for bad in [raw.replace(hook, ""), format!("{raw}{hook}")] {
            assert!(std::panic::catch_unwind(|| view(name, &bad)).is_err());
        }
        assert_ne!(view(name, &format!("{raw}\n// drift\n")), old);
    }
    let raw = read("docs/COMPAT.md");
    let block = entry_guard::material::child::sandbox::held::BLOCK;
    assert!(
        entry_guard::material::child::sandbox::held::compat_view(&raw)
            .ends_with("<!-- END BOOTSTRAP INSTALLED CHILD V1 -->\n")
    );
    for bad in [raw.replace(block, ""), format!("{raw}{block}")] {
        assert!(std::panic::catch_unwind(|| {
            entry_guard::material::child::sandbox::held::compat_view(&bad)
        })
        .is_err());
    }
    let cli = source("bin/diagnostic_held_bootstrap.rs");
    assert!(!cli.contains("held-readiness"));
    assert!(source("diagnostic_bootstrap_installed_mode.rs")
        .contains("original bootstrap authentication is closed"));
    let original = source("diagnostic_original_held_delivery.rs");
    assert!(original.contains("self.stage = Stage::Uncertain;"));
    assert!(original.contains("original.observe_managed_sandbox()"));
    assert!(original.contains("deadline: context.bounds[3]"));
    assert!(!original.contains("execve"));
    assert!(!original.contains("release_worker"));
    let parser_path = "tools/long-run-supervisor-074/src/diagnostic_kernel_ancillary.rs";
    let old = std::process::Command::new("git")
        .current_dir(root())
        .args(["show", &format!("ba690541:{parser_path}")])
        .output()
        .unwrap();
    assert!(old.status.success());
    assert_eq!(
        read(parser_path),
        String::from_utf8(old.stdout).unwrap().replace('\r', "")
    );
}

use guard::installed_entry as entry_guard;
#[test]
fn bootstrap_sandbox_owner_component_precedes_admission() {
    assert!(
        root()
            .join("tools/long-run-supervisor-074/src/diagnostic_bootstrap_sandbox_owner.rs")
            .is_file(),
        "original owner sandbox observation binding is absent"
    );
}
#[test]
fn bootstrap_installed_child_component_precedes_delivery() {
    assert!(
        root()
            .join("tools/long-run-supervisor-074/src/diagnostic_bootstrap_installed_child.rs")
            .is_file(),
        "installed child consistency reader is absent"
    );
}
#[test]
fn bootstrap_installed_material_component_precedes_delivery() {
    assert!(
        root()
            .join("tools/long-run-supervisor-074/src/diagnostic_bootstrap_installed_entry_owner.rs")
            .is_file(),
        "consuming installed entry materialization owner is absent"
    );
}
#[path = "support/bootstrap_installed_guard.rs"]
mod guard;
#[test]
fn bootstrap_installed_entry_component_precedes_runtime() {
    assert!(
        root()
            .join(
                "tools/long-run-supervisor-074/src/diagnostic_bootstrap_installed_entry_format.rs"
            )
            .is_file(),
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
            entry_guard::material::child::source_view(name, &source(name)),
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

fn installed_baseline(path: &str) -> String {
    let out = std::process::Command::new("git")
        .current_dir(root())
        .args(["--no-replace-objects", "show", &format!("70607a71:{path}")])
        .output()
        .unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout).unwrap().replace('\r', "")
}

#[test]
fn bootstrap_installed_entry_only_one_codec_hook_no_owner_or_child_activation() {
    let name = "diagnostic_bootstrap_build.rs";
    let raw = source(name);
    let old = installed_baseline(&format!("tools/long-run-supervisor-074/src/{name}"));
    assert_eq!(entry_guard::source_view(name, &raw), old);
    for bad in [
        raw.replace(entry_guard::HOOK, ""),
        format!("{raw}{}", entry_guard::HOOK),
        raw.replace(
            "pub mod installed_entry_format;",
            "pub(crate) mod installed_entry_format;",
        ),
    ] {
        assert!(std::panic::catch_unwind(|| entry_guard::source_view(name, &bad)).is_err());
    }
    assert_ne!(
        entry_guard::source_view(name, &format!("{raw}\n// drift\n")),
        old
    );
    for name in [
        "bin/diagnostic_held_bootstrap.rs",
        "diagnostic_bootstrap_entry_format.rs",
        "diagnostic_bootstrap_entry_binding.rs",
        "diagnostic_bootstrap_installed_owner.rs",
        "diagnostic_bootstrap_installed_source.rs",
        "diagnostic_bootstrap_installed_census.rs",
        "diagnostic_bootstrap_child_reader.rs",
        "diagnostic_bootstrap_child_roles.rs",
        "diagnostic_bootstrap_current_image.rs",
        "diagnostic_held_unit_projection.rs",
    ] {
        assert_eq!(
            entry_guard::source_view(name, &source(name)),
            installed_baseline(&format!("tools/long-run-supervisor-074/src/{name}"))
        );
    }
    let raw = source("diagnostic_bootstrap_installed_entry_format.rs");
    for token in [
        "unsafe",
        "File",
        "RawFd",
        "Command::",
        "SystemTime",
        "Instant",
        "execve",
        "sendmsg",
        "Deserialize",
    ] {
        assert!(!raw.contains(token), "codec must not enable {token}");
    }
}

#[test]
fn bootstrap_installed_entry_compat_is_exact_and_does_not_change_old_records() {
    let raw = read("docs/COMPAT.md");
    assert_eq!(
        entry_guard::compat_view(&raw),
        installed_baseline("docs/COMPAT.md")
    );
    for bad in [
        raw.replace(entry_guard::BLOCK, ""),
        format!("{raw}{}", entry_guard::BLOCK),
        raw.replace("HCBI0741", "HCBI0742"),
    ] {
        assert!(std::panic::catch_unwind(|| entry_guard::compat_view(&bad)).is_err());
    }
    for path in [
        "Cargo.lock",
        "tools/long-run-supervisor-074/Cargo.toml",
        "docs/testing/gated-test-registry.toml",
        "docs/testing/performance/0.74/qualification-manifest.toml",
    ] {
        assert_eq!(read(path), installed_baseline(path));
    }
}

#[test]
fn bootstrap_installed_material_only_three_hooks_keep_whole_old_owner_and_kernel() {
    let view = entry_guard::material::source_view;
    for name in [
        "diagnostic_bootstrap_installed_owner.rs",
        "diagnostic_bootstrap_installed_census.rs",
        "diagnostic_bootstrap_entry_resource.rs",
    ] {
        let path = format!("tools/long-run-supervisor-074/src/{name}");
        let output = std::process::Command::new("git")
            .current_dir(root())
            .args(["--no-replace-objects", "show", &format!("5f8467b0:{path}")])
            .output()
            .unwrap();
        assert!(output.status.success());
        let old = String::from_utf8(output.stdout).unwrap().replace('\r', "");
        let raw = source(name);
        assert_eq!(view(name, &raw), old);
        let hook = entry_guard::material::hook(name).unwrap();
        for bad in [raw.replace(hook, ""), format!("{raw}{hook}")] {
            assert!(std::panic::catch_unwind(|| view(name, &bad)).is_err());
        }
        assert_ne!(view(name, &format!("{raw}\n// unrelated drift\n")), old);
    }
    for name in [
        "diagnostic_bootstrap_installed_entry_owner.rs",
        "diagnostic_bootstrap_installed_entry_resource.rs",
        "diagnostic_bootstrap_installed_entry_census.rs",
    ] {
        let raw = source(name);
        let code = raw.split("#[cfg(test)]").next().unwrap();
        for token in [
            "execve",
            "sendmsg",
            "Command::",
            "clock_gettime",
            "SystemTime",
            "Instant",
            "IntoRawFd",
            "try_clone",
            "F_SETFD",
            "F_SETFL",
            "Deref",
            "pub fn",
            "pub struct",
        ] {
            assert!(!code.contains(token), "{name}/{token}");
        }
    }
    let owner = source("diagnostic_bootstrap_installed_entry_owner.rs");
    assert!(owner.contains("self.owner.revalidate()"));
    assert!(owner.contains("if self.resource.is_none()"));
    assert!(owner.contains("if self.retained.is_some()"));
    assert!(owner.contains("self.owner.refuse();"));
    assert!(
        owner.find("owner: InstalledBootstrapUnit").unwrap()
            < owner
                .find("resource: Option<InstalledEntryResource>")
                .unwrap()
    );
    let census = source("diagnostic_bootstrap_installed_entry_census.rs");
    assert!(census.contains("fds[..32].copy_from_slice(&self.fds)"));
    assert!(census.contains("roles[..16].copy_from_slice(&self.roles)"));
}

#[test]
fn bootstrap_installed_material_additive_compat_keeps_schema_and_qualification() {
    let raw = read("docs/COMPAT.md");
    let old = std::process::Command::new("git")
        .current_dir(root())
        .args(["--no-replace-objects", "show", "5f8467b0:docs/COMPAT.md"])
        .output()
        .unwrap();
    assert!(old.status.success());
    assert_eq!(
        entry_guard::material::compat_view(&raw),
        String::from_utf8(old.stdout).unwrap().replace('\r', "")
    );
    for bad in [
        raw.replace(entry_guard::material::BLOCK, ""),
        format!("{raw}{}", entry_guard::material::BLOCK),
        raw.replace("33/17", "34/18"),
    ] {
        assert!(std::panic::catch_unwind(|| entry_guard::material::compat_view(&bad)).is_err());
    }
    assert_eq!(source("diagnostic_bootstrap_installed_entry_format.rs"),
        String::from_utf8(std::process::Command::new("git").current_dir(root()).args(["show", "5f8467b0:tools/long-run-supervisor-074/src/diagnostic_bootstrap_installed_entry_format.rs"]).output().unwrap().stdout).unwrap().replace('\r', ""));
}

fn child_baseline(path: &str) -> String {
    let output = std::process::Command::new("git")
        .current_dir(root())
        .args(["--no-replace-objects", "show", &format!("ee81c2d2:{path}")])
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().replace('\r', "")
}

#[test]
fn bootstrap_installed_child_exact_hooks_preserve_every_old_mode_and_frame() {
    let view = entry_guard::material::child::source_view;
    for (name, path) in [
        (
            "bin/diagnostic_held_bootstrap.rs",
            "tools/long-run-supervisor-074/src/bin/diagnostic_held_bootstrap.rs",
        ),
        (
            "diagnostic_bootstrap_child_roles.rs",
            "tools/long-run-supervisor-074/src/diagnostic_bootstrap_child_roles.rs",
        ),
        (
            "bootstrap_current_image.rs",
            "tools/long-run-supervisor-074/tests/bootstrap_current_image.rs",
        ),
    ] {
        let raw = read(path);
        let old = child_baseline(path);
        assert_eq!(view(name, &raw), old);
        let hook = entry_guard::material::child::hook(name).unwrap();
        for bad in [raw.replace(hook, ""), format!("{raw}{hook}")] {
            assert!(std::panic::catch_unwind(|| view(name, &bad)).is_err());
        }
        assert_ne!(view(name, &format!("{raw}\n// unrelated drift\n")), old);
    }
    for name in [
        "diagnostic_bootstrap_child_reader.rs",
        "diagnostic_bootstrap_child_role_kernel.rs",
        "diagnostic_bootstrap_closed_inventory.rs",
        "diagnostic_bootstrap_current_image.rs",
        "diagnostic_bootstrap_installed_owner.rs",
        "diagnostic_bootstrap_installed_source.rs",
        "diagnostic_bootstrap_installed_entry_owner.rs",
        "diagnostic_bootstrap_entry_binding.rs",
        "diagnostic_original_manager_message.rs",
        "diagnostic_held_unit_projection.rs",
    ] {
        assert_eq!(
            entry_guard::material::child::sandbox::source_view(name, &source(name)),
            child_baseline(&format!("tools/long-run-supervisor-074/src/{name}"))
        );
    }
}

#[test]
fn bootstrap_installed_child_is_separate_consistency_not_authority_or_transport() {
    let raw = read("docs/COMPAT.md");
    assert_eq!(
        entry_guard::material::child::compat_view(&raw),
        child_baseline("docs/COMPAT.md")
    );
    for bad in [
        raw.replace(entry_guard::material::child::BLOCK, ""),
        format!("{raw}{}", entry_guard::material::child::BLOCK),
    ] {
        assert!(
            std::panic::catch_unwind(|| entry_guard::material::child::compat_view(&bad)).is_err()
        );
    }
    for name in [
        "Cargo.lock",
        "tools/long-run-supervisor-074/Cargo.toml",
        "docs/testing/gated-test-registry.toml",
        "docs/testing/performance/0.74/qualification-manifest.toml",
    ] {
        assert_eq!(read(name), child_baseline(name));
    }
    for name in [
        "diagnostic_bootstrap_installed_mode.rs",
        "diagnostic_bootstrap_installed_child.rs",
        "diagnostic_bootstrap_installed_executable.rs",
        "diagnostic_bootstrap_installed_inventory.rs",
    ] {
        let raw = source(name);
        let code = raw.split("#[cfg(test)]").next().unwrap();
        for forbidden in [
            "execve",
            "sendmsg",
            "Command::",
            "F_SETFD",
            "F_SETFL",
            "dup2",
            "IntoRawFd",
            "try_clone",
            "clock_gettime",
            "SystemTime",
            "Instant",
            "setns",
            "unshare",
        ] {
            assert!(!code.contains(forbidden), "{name}/{forbidden}");
        }
    }
    let mode = source("diagnostic_bootstrap_installed_mode.rs");
    assert!(mode.contains("original bootstrap authentication is closed"));
    assert!(mode.contains("installed::revalidate(&observed)?"));
    let child = source("diagnostic_bootstrap_installed_child.rs");
    assert!(child.contains("let roles = capture("));
    assert!(child.contains("need(observe()? == *prior)"));
    assert_eq!(child.matches("inventory::inspect()?").count(), 2);
    assert!(child.contains("let installed_role = kernel::snapshot(15)?"));
    assert!(child.contains("let record = record(16)?"));
    assert!(source("diagnostic_bootstrap_installed_executable.rs")
        .contains(".open(\"/proc/thread-self\")?"));
}

#[test]
fn bootstrap_sandbox_only_four_exact_additions_preserve_old_frames_and_deadlines() {
    for name in [
        "diagnostic_bootstrap_installed_entry_owner.rs",
        "diagnostic_manager_channel.rs",
        "diagnostic_process.rs",
        "diagnostic_bootstrap_installed_entry_census.rs",
    ] {
        let path = format!("tools/long-run-supervisor-074/src/{name}");
        let output = std::process::Command::new("git")
            .current_dir(root())
            .args(["show", &format!("27b47622:{path}")])
            .output()
            .unwrap();
        assert!(output.status.success());
        let old = String::from_utf8(output.stdout).unwrap().replace('\r', "");
        let raw = source(name);
        let view = entry_guard::material::child::sandbox::source_view;
        assert_eq!(view(name, &raw), old);
        let hook = entry_guard::material::child::sandbox::hook(name).unwrap();
        for bad in [raw.replace(hook, ""), format!("{raw}{hook}")] {
            assert!(std::panic::catch_unwind(|| view(name, &bad)).is_err());
        }
        assert_ne!(view(name, &format!("{raw}\n// drift\n")), old);
    }
}
#[test]
fn bootstrap_sandbox_takes_no_caller_pid_or_manager_and_cannot_start_or_release() {
    for name in [
        "diagnostic_bootstrap_sandbox_owner.rs",
        "diagnostic_manager_sandbox_query.rs",
        "diagnostic_sandbox_process.rs",
    ] {
        let raw = source(name);
        let code = raw.split("#[cfg(test)]").next().unwrap();
        for forbidden in [
            "execve",
            "sendmsg",
            "Command::",
            "setns",
            "unshare",
            "F_SETFD",
            "F_SETFL",
            "StartTransientUnit",
            "SystemTime",
            "Instant",
            "Serialize",
            "Deserialize",
        ] {
            assert!(!code.contains(forbidden), "{name}/{forbidden}");
        }
    }
    let raw = source("diagnostic_bootstrap_sandbox_owner.rs");
    let compact: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        compact.contains("fnobserve_managed_sandbox(self,)")
            || compact.contains("fnobserve_managed_sandbox(self)")
    );
    assert!(raw.contains("self.owner.revalidate()?"));
    assert!(raw.contains("self.owner.refuse()"));
    assert!(compact.contains("channel.observe_sandbox(&source.unit,&source.group)"));
    assert!(raw.contains("self.kernel_pinned"));
    assert_eq!(
        read("docs/testing/performance/0.74/qualification-manifest.toml"),
        child_baseline("docs/testing/performance/0.74/qualification-manifest.toml")
    );
}
