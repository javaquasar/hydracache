#[path = "support/held_material_guard.rs"]
mod guard;
#[path = "support/authority_history_io.rs"]
mod history;

#[test]
fn held_material_inventory_requires_fresh_original_owner_and_deadline_after_census() {
    let source = read("tools/long-run-supervisor-074/src/diagnostic_held_unit_material_kernel.rs");
    assert!(source.contains("let descriptors = fds.into_iter().map(stamp).collect::<io::Result<_>>()?;\n        owner(b)?;\n        Ok(descriptors)"));
}
fn read(path: &str) -> String {
    history::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path),
    )
    .unwrap()
    .replace('\r', "")
}
fn old(path: &str) -> String {
    let out = history::command()
        .args(["show", &format!("c9cc5a2e:{path}")])
        .output()
        .unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout).unwrap().replace('\r', "")
}
#[test]
fn held_material_only_two_exact_hooks_restore_complete_projected_owner_and_bundle_bodies() {
    for (file, child) in [
        (
            "diagnostic_held_unit_projection.rs",
            "diagnostic_held_unit_material.rs",
        ),
        (
            "diagnostic_bundle_projection.rs",
            "diagnostic_bundle_material.rs",
        ),
    ] {
        let path = format!("tools/long-run-supervisor-074/src/{file}");
        let raw = read(&path);
        assert_eq!(crate::hc074_held_material_view!(&path, &raw), old(&path));
        let hook = format!("\n#[path = \"{child}\"]\nmod material;\n");
        for bad in [
            raw.replacen("mod material;", "pub mod material;", 1),
            format!("{raw}{hook}"),
        ] {
            assert!(
                std::panic::catch_unwind(|| crate::hc074_held_material_view!(&path, &bad)).is_err()
            );
        }
        let drift = format!("{raw}\n// unrelated drift\n");
        assert_ne!(crate::hc074_held_material_view!(&path, &drift), old(&path));
        assert_eq!(
            crate::hc074_held_material_view!(&path, &old(&path)),
            old(&path)
        );
    }
}
#[test]
fn held_material_keeps_original_frames_retirement_launcher_format_and_qualification_frozen() {
    for file in [
        "diagnostic_consuming_held_join.rs",
        "diagnostic_held_late_trust.rs",
        "diagnostic_fixed_placement.rs",
        "diagnostic_owned_placement.rs",
        "diagnostic_owned_root_manager.rs",
        "diagnostic_original_root_guardians.rs",
        "diagnostic_original_manager_message.rs",
        "diagnostic_original_watchdog_context.rs",
        "diagnostic_original_enrollment_frame.rs",
        "diagnostic_original_descriptor_plan.rs",
        "diagnostic_held_launcher_kernel.rs",
        "diagnostic_root_guardian.rs",
        "diagnostic_root_guardian_raw.rs",
        "diagnostic_held_role_projection.rs",
        "diagnostic_root_guardian_projection.rs",
    ] {
        let path = format!("tools/long-run-supervisor-074/src/{file}");
        assert_eq!(read(&path), old(&path), "{path}");
    }
    for path in [
        "Cargo.lock",
        "docs/COMPAT.md",
        "docs/testing/gated-test-registry.toml",
        "docs/testing/performance/0.74/qualification-manifest.toml",
        "tools/long-run-supervisor-074/tests/support/consuming_held_live.rs",
    ] {
        assert_eq!(
            crate::hc074_held_material_live_view!(path, &read(path)),
            old(path),
            "{path}"
        );
    }
}
fn ids(source: &str) -> Vec<String> {
    fn visit(stream: proc_macro2::TokenStream, out: &mut Vec<String>) {
        for token in stream {
            match token {
                proc_macro2::TokenTree::Ident(i) => out.push(i.to_string()),
                proc_macro2::TokenTree::Group(g) => visit(g.stream(), out),
                _ => {}
            }
        }
    }
    let mut out = vec![];
    visit(source.parse().unwrap(), &mut out);
    out
}
#[test]
fn held_material_closed_api_contains_no_transport_mapping_fork_exec_or_serialization() {
    for file in [
        "diagnostic_held_unit_material.rs",
        "diagnostic_held_unit_material_kernel.rs",
        "diagnostic_bundle_material.rs",
    ] {
        let source = read(&format!("tools/long-run-supervisor-074/src/{file}"));
        let tokens = ids(&source);
        for forbidden in [
            "send",
            "sendmsg",
            "execve",
            "execveat",
            "fork",
            "dup2",
            "dup3",
            "Message",
            "Connection",
            "TransientUnitSpec",
            "Serialize",
            "Deserialize",
            "IntoRawFd",
            "Deref",
            "release_barrier",
            "start_once",
            "into_raw_fd",
        ] {
            assert!(!tokens.iter().any(|i| i == forbidden), "{file}/{forbidden}");
        }
    }
    let source = read("tools/long-run-supervisor-074/src/diagnostic_held_unit_material.rs");
    assert_eq!(source.matches("owner.full_frame()?;").count(), 2);
    assert!(source.contains("self.owner.revalidate()?"));
    assert!(source.contains("self.owner.owner.refuse();"));
    assert!(source.contains("self.resources.is_some()"));
    assert!(source.contains("owner.retained().is_some_and"));
    assert_eq!(source.matches("pub(crate) fn ").count(), 4); // materialize/revalidate/error/is_refused only.
}
