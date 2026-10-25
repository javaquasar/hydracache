#[path = "support/held_unit_projection_guard.rs"]
mod guard;
#[path = "support/authority_history_io.rs"]
mod history;

const BASE: &str = "392fa72188f84ca348e9f21c3e689d6546f48afb";
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
    let result = history::command()
        .args(["show", &format!("{BASE}:{path}")])
        .output()
        .unwrap();
    assert!(result.status.success());
    String::from_utf8(result.stdout).unwrap().replace('\r', "")
}

#[test]
fn held_unit_projection_only_four_exact_additive_hooks_restore_prior_sources() {
    for (file, child, module) in [
        (
            "diagnostic_consuming_held_join.rs",
            "diagnostic_held_unit_projection.rs",
            "unit_projection",
        ),
        (
            "diagnostic_held_role_files.rs",
            "diagnostic_held_role_projection.rs",
            "projection",
        ),
        (
            "diagnostic_artifacts_linux.rs",
            "diagnostic_bundle_projection.rs",
            "projection",
        ),
        (
            "diagnostic_root_guardian_original_kernel.rs",
            "diagnostic_root_guardian_projection.rs",
            "projection",
        ),
    ] {
        let path = format!("tools/long-run-supervisor-074/src/{file}");
        let raw = read(&path);
        assert_eq!(
            crate::hc074_held_unit_projection_view!(&path, &raw),
            old(&path)
        );
        let exact = format!("\n#[path = \"{child}\"]\nmod {module};\n");
        for bad in [
            raw.replacen(
                &exact,
                &format!("\n#[path = \"{child}\"]\npub mod {module};\n"),
                1,
            ),
            format!("{raw}{exact}"),
        ] {
            assert!(
                std::panic::catch_unwind(|| crate::hc074_held_unit_projection_view!(&path, &bad))
                    .is_err()
            );
        }
        let changed = format!("{raw}\n// unrelated mutation\n");
        assert!(crate::hc074_held_unit_projection_view!(&path, &changed)
            .ends_with("// unrelated mutation\n"));
        assert_eq!(
            crate::hc074_held_unit_projection_view!(&path, &old(&path)),
            old(&path)
        );
    }
}

#[test]
fn held_unit_projection_preserves_all_other_admission_transport_and_fixture_bytes() {
    for file in [
        "diagnostic_held_late_trust.rs",
        "diagnostic_held_role_provenance.rs",
        "diagnostic_fixed_placement.rs",
        "diagnostic_owned_placement.rs",
        "diagnostic_owned_root_manager.rs",
        "diagnostic_original_root_guardians.rs",
        "diagnostic_original_watchdog_context.rs",
        "diagnostic_original_manager_message.rs",
        "diagnostic_original_manager_join.rs",
        "diagnostic_original_pipe_transport.rs",
        "diagnostic_original_enrollment_frame.rs",
        "diagnostic_held_launcher_kernel.rs",
        "diagnostic_root_guardian.rs",
        "diagnostic_root_guardian_raw.rs",
        "diagnostic_collector_starting_storage.rs",
        "diagnostic_dispatch_exchange.rs",
        "diagnostic_dispatch_exchange.rs",
        "diagnostic_lease.rs",
    ] {
        let path = format!("tools/long-run-supervisor-074/src/{file}");
        assert_eq!(read(&path), old(&path), "{path}");
    }
    for path in [
        "Cargo.lock",
        "docs/COMPAT.md",
        "docs/testing/gated-test-registry.toml",
        "docs/testing/performance/0.74/qualification-manifest.toml",
        "tools/long-run-supervisor-074/tests/support/consuming_held_join.rs",
        "tools/long-run-supervisor-074/tests/support/consuming_held_live.rs",
    ] {
        assert_eq!(
            crate::hc074_held_material_live_view!(path, &read(path)),
            old(path),
            "{path}"
        );
    }
}

fn identifiers(raw: &str) -> Vec<String> {
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
    visit(raw.parse().unwrap(), &mut out);
    out
}

#[test]
fn held_unit_projection_has_no_runnable_spec_transport_or_descriptor_export() {
    let path = "tools/long-run-supervisor-074/src/diagnostic_held_unit_projection.rs";
    let source = read(path);
    let tokens = identifiers(&source);
    for forbidden in [
        "unsafe",
        "TransientUnitSpec",
        "Message",
        "Connection",
        "send",
        "sendmsg",
        "execve",
        "execveat",
        "fork",
        "socketpair",
        "try_clone",
        "AsRawFd",
        "IntoRawFd",
        "Serialize",
        "Deserialize",
        "Deref",
        "AsRef",
        "release_barrier",
        "start_once",
    ] {
        assert!(!tokens.iter().any(|i| i == forbidden), "{forbidden}");
    }
    assert_eq!(source.matches("pub(crate) fn ").count(), 2);
    assert!(source.contains("pub(crate) fn project_held_unit(\n        mut self,"));
    assert!(source.contains("pub(crate) fn revalidate(&mut self)"));
    assert_eq!(source.matches("validate(owner)?;").count(), 2);
    assert!(source.contains("owner.refuse();"));
    for name in [
        "DedicatedEmptyInputPipe",
        "AuthenticatedReleaseEndpoint",
        "SealedCopyOfOriginalConfig",
        "SealedBootstrap256",
    ] {
        assert!(tokens.iter().any(|i| i == name));
    }
}
