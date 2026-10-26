#[test]
fn bootstrap_entry_binding_exists_before_child_delivery() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    assert!(
        root.join("tools/long-run-supervisor-074/src/diagnostic_bootstrap_entry_format.rs")
            .is_file(),
        "missing sealed child-entry format"
    );
}

#[path = "support/bootstrap_entry_guard.rs"]
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
        .args(["--no-replace-objects", "show", &format!("eedd826c:{path}")])
        .current_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .output()
        .unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout).unwrap().replace('\r', "")
}
#[test]
fn bootstrap_entry_exact_additions_restore_whole_old_frames_and_kernel_bodies() {
    for name in [
        "diagnostic_bootstrap_owner.rs",
        "diagnostic_bootstrap_build.rs",
        "diagnostic_bootstrap_image_census.rs",
        "diagnostic_held_unit_material_kernel.rs",
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
    let owner = source("diagnostic_bootstrap_owner.rs");
    assert!(std::panic::catch_unwind(|| guard::source_view(
        "diagnostic_bootstrap_owner.rs",
        &owner.replace("&census,", "&other_census,")
    ))
    .is_err());
    assert_ne!(
        guard::source_view(
            "diagnostic_bootstrap_owner.rs",
            &owner.replace("owner.full_frame()?;", "owner.inspect()?;")
        ),
        baseline("tools/long-run-supervisor-074/src/diagnostic_bootstrap_owner.rs")
    );
}
#[test]
fn bootstrap_entry_compat_is_exact_additive_and_qualification_stays_frozen() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let raw = bootstrap_docs::read_to_string(root.join("docs/COMPAT.md"))
        .unwrap()
        .replace('\r', "");
    assert_eq!(guard::compat_view(&raw), baseline("docs/COMPAT.md"));
    for bad in [
        raw.replace("HCBT0741 is exactly 1024", "HCBT0741 is exactly 2048"),
        raw.replace(guard::BLOCK, ""),
        format!("{raw}{}", guard::BLOCK),
    ] {
        assert!(std::panic::catch_unwind(|| guard::compat_view(&bad)).is_err());
    }
    for name in [
        "Cargo.lock",
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
fn bootstrap_entry_uses_original_sealed_config_and_no_transport_or_new_time() {
    let factory = source("diagnostic_bootstrap_entry_binding.rs");
    assert!(factory.contains("legacy.config"));
    assert!(factory.contains("legacy.legacy"));
    assert!(!factory.contains("c.config.identity"));
    for name in [
        "diagnostic_bootstrap_entry_binding.rs",
        "diagnostic_bootstrap_entry_census.rs",
        "diagnostic_bootstrap_entry_resource.rs",
    ] {
        let raw = source(name);
        let runtime = raw.split("#[cfg(test)]").next().unwrap();
        for token in [
            "execve",
            "sendmsg",
            "dup2",
            "dup3",
            "Command::",
            "clock_gettime",
            "Instant::",
            "try_clone",
            "pub fn as_raw_fd",
            "IntoRawFd",
        ] {
            assert!(!runtime.contains(token), "{name}/{token}");
        }
    }
}

#[path = "../../../tools/bootstrap-doc-reader-074/io.rs"]
mod bootstrap_docs;
