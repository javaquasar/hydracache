use std::path::Path;
#[path = "support/bootstrap_entry_guard.rs"]
mod entry_guard;
#[path = "support/bootstrap_build_guard.rs"]
mod guard;
fn read(name: &str) -> String {
    let raw = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(name),
    )
    .unwrap()
    .replace('\r', "");
    if name == "docs/testing/gated-test-registry.toml" {
        entry_guard::registry_view(&raw)
    } else {
        raw
    }
}
#[test]
fn bootstrap_build_has_separate_policy_and_receipt_domains_without_execution() {
    let raw = read("tools/long-run-supervisor-074/src/diagnostic_bootstrap_build.rs");
    for token in [
        "POLICY_DOMAIN",
        "RECEIPT_DOMAIN",
        "verify_strict",
        "deny_unknown_fields",
        "MAX_RECEIPT_BYTES",
    ] {
        assert!(raw.contains(token), "{token}");
    }
}
#[test]
fn bootstrap_original_wrapper_consumes_material_and_keeps_closing_frames() {
    let raw = read("tools/long-run-supervisor-074/src/diagnostic_bootstrap_owner.rs");
    assert!(raw.contains("MaterializedHeldUnit<'root"));
    assert_eq!(raw.matches("owner.full_frame()?;").count(), 3);
    assert!(raw.contains("owner.refuse();"));
    assert!(raw.contains("inspect_bootstrap_build"));
}

fn baseline(path: &str) -> String {
    let out = std::process::Command::new("git")
        .args(["--no-replace-objects", "show", &format!("dd820049:{path}")])
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .output()
        .unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout).unwrap().replace('\r', "")
}
#[test]
fn bootstrap_hooks_restore_entire_old_bodies_and_compatibility_bytes() {
    for (file,hook) in [
        ("diagnostic_builder.rs","#[path = \"diagnostic_bootstrap_build.rs\"]\npub mod bootstrap_build;\n"),
        ("diagnostic_bootstrap_enrollment.rs","\n#[path = \"diagnostic_bootstrap_selected_build.rs\"]\npub(crate) mod selected_build;\n"),
        ("diagnostic_held_unit_material.rs","\n#[path = \"diagnostic_bootstrap_owner.rs\"]\nmod bootstrap_owner;\n"),
    ] {
        let path=format!("tools/long-run-supervisor-074/src/{file}");let raw=read(&path);assert_eq!(raw.matches(hook).count(),1);
        assert_eq!(raw.replacen(hook,"",1),baseline(&path));
    }
    let raw = read("docs/COMPAT.md");
    assert_eq!(guard::compat_view(&raw), baseline("docs/COMPAT.md"));
    for bad in [
        raw.replace(guard::BLOCK, ""),
        format!("{raw}{}", guard::BLOCK),
        raw.replace(
            "Maximum receipt size is 65536 bytes.",
            "Maximum receipt size is unlimited.",
        ),
    ] {
        assert!(std::panic::catch_unwind(|| guard::compat_view(&bad)).is_err());
    }
    let raw = read("tools/long-run-supervisor-074/src/diagnostic_held_unit_material.rs");
    for bad in [
        raw.replace("mod bootstrap_owner;", "pub mod bootstrap_owner;"),
        format!("{raw}\n#[path = \"diagnostic_bootstrap_owner.rs\"]\nmod bootstrap_owner;\n"),
    ] {
        assert!(std::panic::catch_unwind(|| guard::material_view(&bad)).is_err());
    }
}
#[test]
fn bootstrap_owner_adds_no_execution_or_clock_capability_and_keeps_exact_original_context() {
    for file in [
        "diagnostic_bootstrap_owner.rs",
        "diagnostic_bootstrap_selected_build.rs",
        "diagnostic_bootstrap_sealed.rs",
    ] {
        let raw = read(&format!("tools/long-run-supervisor-074/src/{file}"));
        let code: String = raw
            .lines()
            .filter(|s| !s.trim_start().starts_with("//"))
            .collect();
        for token in [
            "execve",
            "sendmsg",
            "Connection",
            "Message",
            "Command",
            "clock_gettime",
            "Instant",
            "SystemTime",
            "File::open",
            "IntoRawFd",
            "try_clone",
            "Serialize",
            "Deserialize",
            "Deref",
        ] {
            assert!(!code.contains(token), "{file}: {token}");
        }
    }
    let raw = read("tools/long-run-supervisor-074/src/diagnostic_bootstrap_owner.rs");
    assert!(raw.contains("self.owner.revalidate()?"));
    let compact: String = raw.split_whitespace().collect();
    assert!(compact.contains("self.owner.owner.owner.context()"));
    assert!(raw.contains("self.owner.owner.owner.trust.inspect_bootstrap_build("));
    for token in [
        "context.documents_sha256[0]",
        "context.documents_sha256[2]",
        "context.host.sha256",
        "context.launcher.sha256",
        "self.owner.refuse();",
        "self.owner.owner.owner.trust.refuse_late_join();",
    ] {
        assert!(raw.contains(token), "{token}");
    }
    for file in [
        "diagnostic_consuming_held_join.rs",
        "diagnostic_held_unit_projection.rs",
        "diagnostic_held_late_trust.rs",
        "diagnostic_original_manager_message.rs",
        "diagnostic_held_launcher_kernel.rs",
        "diagnostic_root_guardian_raw.rs",
    ] {
        let path = format!("tools/long-run-supervisor-074/src/{file}");
        assert_eq!(read(&path), baseline(&path));
    }
    for path in [
        "Cargo.lock",
        "docs/testing/gated-test-registry.toml",
        "docs/testing/performance/0.74/qualification-manifest.toml",
    ] {
        assert_eq!(read(path), baseline(path));
    }
}
