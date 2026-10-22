#[path = "support/bootstrap_build_guard.rs"]
mod build_guard;
#[path = "support/held_role_census_guard.rs"]
mod census_guard;

fn read(name: &str) -> String {
    std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tools/long-run-supervisor-074/src")
            .join(name),
    )
    .unwrap()
    .replace('\r', "")
}

#[test]
fn held_material_roles_are_private_and_share_the_full_frame_inventory() {
    let material = read("diagnostic_held_unit_material.rs");
    assert!(material.contains("roles: kernel::RoleBindings"));
    assert!(material.contains("kernel::bind_roles(&bindings(&context), &descriptors)?"));
    assert_eq!(material.matches("owner.full_frame()?;").count(), 2);
    assert_eq!(material.matches("pub(crate) fn ").count(), 4);
    assert!(material.contains("self.owner.revalidate()?"));
    assert!(material.contains("self.owner.owner.refuse();"));
}

fn baseline(name: &str) -> String {
    let path = format!("563caf77:tools/long-run-supervisor-074/src/{name}");
    let out = std::process::Command::new("git")
        .arg("--no-replace-objects")
        .args(["show", &path])
        .current_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .output()
        .unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout).unwrap().replace('\r', "")
}

#[test]
fn held_material_roles_only_add_values_and_leave_complete_frame_and_retirement_unchanged() {
    let source = census_guard::material_view(&build_guard::material_view(&read(
        "diagnostic_held_unit_material.rs",
    )));
    let hook = "        // Pure binding of the SAME fresh census. No new FD, bus/time read or\n        // authority is introduced; the context pins the original channel names.\n        let roles = kernel::bind_roles(&bindings(&context), &descriptors)?;\n";
    assert_eq!(source.matches(hook).count(), 1);
    let restored = source
        .replace("    roles: kernel::RoleBindings,\n", "")
        .replace(hook, "")
        .replace("            roles,\n", "");
    assert_eq!(restored, baseline("diagnostic_held_unit_material.rs"));
    for name in [
        "diagnostic_consuming_held_join.rs",
        "diagnostic_held_unit_projection.rs",
        "diagnostic_original_manager_message.rs",
        "diagnostic_original_manager_join.rs",
        "diagnostic_original_watchdog_context.rs",
        "diagnostic_manager_channel.rs",
        "diagnostic_held_launcher_kernel.rs",
        "diagnostic_root_guardian.rs",
    ] {
        assert_eq!(read(name), baseline(name), "{name}");
    }
}

#[test]
fn held_material_roles_leaf_has_no_new_live_query_effect_or_transport() {
    let source = read("diagnostic_held_unit_material_kernel.rs");
    let leaf = source
        .split("pub(super) fn bind_roles(")
        .nth(1)
        .unwrap()
        .split("\nfn stamp(")
        .next()
        .unwrap();
    for forbidden in [
        "unsafe",
        "libc::fcntl",
        "libc::fstat",
        "clock_gettime",
        "now()",
        "owner(b)",
        "try_clone",
        "BorrowedFd",
        "File::",
        "send",
        "Message",
        "Connection",
        "execve",
        "fork",
        "dup2",
        "dup3",
    ] {
        assert!(!leaf.contains(forbidden), "{forbidden}");
    }
    assert!(leaf.contains("inventory.len() == 20"));
    assert!(leaf.contains("inventory[at].fd"));
}
