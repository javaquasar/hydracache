#[test]
fn bootstrap_child_executable_exists_before_any_original_start() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    assert!(
        root.join("tools/long-run-supervisor-074/src/bin/diagnostic_held_bootstrap.rs")
            .is_file(),
        "bootstrap executable absent"
    );
}

#[path = "support/bootstrap_child_guard.rs"]
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
fn baseline(path: &str) -> String {
    let out = std::process::Command::new("git")
        .args(["--no-replace-objects", "show", &format!("c9654c84:{path}")])
        .current_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .output()
        .unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout).unwrap().replace('\r', "")
}
#[test]
fn bootstrap_child_keeps_entire_original_runtime_and_qualification_bytes() {
    for name in [
        "diagnostic_bootstrap_owner.rs",
        "diagnostic_bootstrap_entry_format.rs",
        "diagnostic_bootstrap_entry_binding.rs",
        "diagnostic_bootstrap_entry_resource.rs",
        "diagnostic_bootstrap_entry_census.rs",
        "diagnostic_bootstrap_build.rs",
        "diagnostic_bootstrap_enrollment.rs",
        "diagnostic_held_unit_material.rs",
        "diagnostic_held_unit_material_kernel.rs",
        "diagnostic_held_unit_projection.rs",
        "diagnostic_consuming_held_join.rs",
        "diagnostic_held_launcher_kernel.rs",
        "diagnostic_held_activation_ingress.rs",
        "diagnostic_root_guardian_raw.rs",
        "diagnostic_original_manager_message.rs",
        "bin/diagnostic_held_launcher.rs",
    ] {
        assert_eq!(
            source(name),
            baseline(&format!("tools/long-run-supervisor-074/src/{name}")),
            "{name}"
        );
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for name in [
        "Cargo.lock",
        "tools/long-run-supervisor-074/Cargo.toml",
        "docs/testing/gated-test-registry.toml",
        "docs/testing/performance/0.74/qualification-manifest.toml",
    ] {
        assert_eq!(
            std::fs::read_to_string(root.join(name))
                .unwrap()
                .replace('\r', ""),
            baseline(name)
        );
    }
}
#[test]
fn bootstrap_child_compares_exact_additive_compatibility_and_refuses_mutation() {
    let raw = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/COMPAT.md"),
    )
    .unwrap()
    .replace('\r', "");
    assert_eq!(guard::compat_view(&raw), baseline("docs/COMPAT.md"));
    for bad in [
        raw.replace(guard::BLOCK, ""),
        format!("{raw}{}", guard::BLOCK),
        raw.replace(
            "All paths, INCLUDING canonical forged records, exit 2",
            "All paths, INCLUDING canonical forged records, exit 0",
        ),
    ] {
        assert!(std::panic::catch_unwind(|| guard::compat_view(&bad)).is_err());
    }
}
#[test]
fn bootstrap_child_has_no_authority_transport_mutation_fallback_or_new_clock() {
    for name in [
        "bin/diagnostic_held_bootstrap.rs",
        "diagnostic_bootstrap_child_reader.rs",
        "diagnostic_bootstrap_child_ingress.rs",
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
            "libc::fork",
            "libc::dup",
            "F_SETFD",
            "close_range",
            "OpenOptions",
            "File::open",
            "Command::",
            "set_var",
            "remove_var",
            "clock_gettime",
            "Instant::",
            "as_raw_fd",
            "FromRawFd",
            "BorrowedFd",
        ] {
            assert!(!code.contains(token), "{name}/{token}");
        }
    }
    let main = source("bin/diagnostic_held_bootstrap.rs");
    assert!(main.contains("original bootstrap authentication is closed"));
    assert_eq!(main.matches("std::process::exit(2)").count(), 1);
    assert!(main.contains("reader::read(14)?"));
    assert!(main.contains("reader::revalidate(14, &observed)?"));
    assert!(main.contains("skip(1).take(2)"));
    assert!(main.contains("vars_os().take(4)"));
}
