#[path = "support/bootstrap_build_guard.rs"]
mod build_guard;
#[path = "../../../tools/historical-blob-batch-074/mod.rs"]
pub mod census_runtime;
#[path = "support/bootstrap_entry_guard.rs"]
mod entry_guard;
#[path = "support/held_role_census_guard.rs"]
mod guard;
#[path = "support/census_provider_input.rs"]
mod provider_input;

#[test]
fn census_standalone_panic_api_signatures_do_not_add_source_reads() {
    let _: fn() -> String = provider_input::original;
    let _: fn(&str, &str) -> bool = provider_input::accepts;
    let _: fn(&str, &str) -> String = provider_input::view;
}

fn read(name: &str) -> String {
    let raw = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tools/long-run-supervisor-074/src")
            .join(name),
    )
    .unwrap()
    .replace('\r', "");
    provider_input::view(name, &raw)
}

fn baseline(name: &str) -> String {
    let result = std::process::Command::new("git")
        .arg("--no-replace-objects")
        .args([
            "show",
            &format!("798400d4:tools/long-run-supervisor-074/src/{name}"),
        ])
        .current_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .output()
        .unwrap();
    assert!(result.status.success());
    String::from_utf8(result.stdout).unwrap().replace('\r', "")
}

#[test]
fn complete_role_census_keeps_prior_owner_and_provider_bodies_exact_after_additive_hooks() {
    let source = build_guard::material_view(&read("diagnostic_held_unit_material.rs"));
    assert_eq!(
        guard::material_view(&source),
        baseline("diagnostic_held_unit_material.rs")
    );
    for bad in [
        source.replace("mod role_census;", "pub mod role_census;"),
        source.replace("            retained_roles,\n", ""),
        format!("{source}\n#[path = \"diagnostic_material_role_census.rs\"]\nmod role_census;\n"),
    ] {
        assert!(std::panic::catch_unwind(|| guard::material_view(&bad)).is_err());
    }
    assert_ne!(
        guard::material_view(&source.replace("owner.full_frame()?;", "owner.prepare()?;")),
        baseline("diagnostic_held_unit_material.rs")
    );
    for (file, hook) in [
        (
            "diagnostic_artifacts_linux.rs",
            "\n#[path = \"diagnostic_held_role_census.rs\"]\npub(crate) mod held_role_census;\n",
        ),
        (
            "diagnostic_bundle_projection.rs",
            "\n#[path = \"diagnostic_bundle_role_census.rs\"]\nmod role_census;\n",
        ),
        (
            "diagnostic_root_guardian_projection.rs",
            "\n#[path = \"diagnostic_guardian_role_census.rs\"]\nmod role_census;\n",
        ),
    ] {
        let source = read(file);
        assert_eq!(source.matches(hook).count(), 1);
        assert_eq!(source.replacen(hook, "", 1), baseline(file));
    }
    for file in [
        "diagnostic_consuming_held_join.rs",
        "diagnostic_held_unit_projection.rs",
        "diagnostic_manager_channel.rs",
        "diagnostic_original_manager_message.rs",
        "diagnostic_original_watchdog_context.rs",
        "diagnostic_root_guardian.rs",
        "diagnostic_root_guardian_raw.rs",
        "diagnostic_root_guardian_original_kernel.rs",
        "diagnostic_held_launcher_kernel.rs",
    ] {
        assert_eq!(
            entry_guard::source_view(file, &read(file)),
            baseline(file),
            "{file}"
        );
    }
}

#[test]
fn complete_role_census_uses_owned_contributions_before_the_existing_closing_owner_check() {
    let material = read("diagnostic_held_unit_material.rs");
    let capture = material
        .find("role_census::capture(self, &context)?")
        .unwrap();
    assert!(
        capture
            < material
                .find(".inspect(&bindings(&context), original)?")
                .unwrap()
    );
    let kernel = read("diagnostic_held_unit_material_kernel.rs");
    assert!(kernel.contains("let descriptors = fds.into_iter().map(stamp).collect::<io::Result<_>>()?;\n        owner(b)?;\n        Ok(descriptors)"));
    let census = read("diagnostic_held_role_census.rs");
    let runtime = census.split("#[cfg(test)]").next().unwrap();
    for forbidden in [
        "SYS_pidfd_open",
        "try_clone",
        "into_raw_fd",
        "fn as_raw_fd",
        "BorrowedFd",
        "Message",
        "Serialize",
        "Deserialize",
        "Debug",
        "Clone",
        "sendmsg",
        "execve",
        "clock_gettime",
    ] {
        // Comments do not supply an executable capability.
        let code: String = runtime
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect();
        assert!(!code.contains(forbidden), "{forbidden}");
    }
    assert!(runtime.contains("roles: [i32; 11]"));
    assert!(runtime.contains("let mut fds = [0; 23]"));
    let bundle = read("diagnostic_bundle_role_census.rs");
    assert!(bundle.contains("contribute(&self.files, census)"));
    assert!(bundle.contains(".get(\"timing-controls-074\")"));
    let guardian: String = read("diagnostic_guardian_role_census.rs")
        .split_whitespace()
        .collect();
    assert!(guardian.contains("self.pair.guards.each_ref()"));
}

#[test]
fn complete_role_census_is_inside_the_same_original_material_inventory() {
    let source = read("diagnostic_held_unit_material.rs");
    assert!(source.contains("retained_roles: role_census::CompleteBindings"));
    assert!(source.contains("role_census::capture(self, &context)?"));
    assert!(source.contains("kernel::complete_roles(&roles, &descriptors, retained_roles)?"));
    assert_eq!(source.matches("owner.full_frame()?;").count(), 2);
    assert_eq!(source.matches("pub(crate) fn ").count(), 4);
}
