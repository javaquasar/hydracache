#[path = "support/bootstrap_enrollment_guard.rs"]
mod bootstrap_guard;
#[path = "support/bootstrap_build_guard.rs"]
mod build_guard;
#[path = "support/bootstrap_image_guard.rs"]
mod image_guard;
fn source(name: &str) -> String {
    let raw = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tools/long-run-supervisor-074/src")
            .join(name),
    )
    .unwrap()
    .replace('\r', "");
    image_guard::provider_view(name, &raw)
}
fn baseline(name: &str) -> String {
    let output = std::process::Command::new("git")
        .args([
            "--no-replace-objects",
            "show",
            &format!("aa58bad7:tools/long-run-supervisor-074/src/{name}"),
        ])
        .current_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().replace('\r', "")
}
#[test]
fn activation_ingress_is_explicit_and_keeps_the_complete_old_kernel() {
    let main = source("bin/diagnostic_held_launcher.rs");
    assert!(main.contains("activation::prepare()"));
    assert_eq!(
        source("diagnostic_held_launcher_kernel.rs"),
        baseline("diagnostic_held_launcher_kernel.rs")
    );
    for file in [
        "diagnostic_held_unit_material.rs",
        "diagnostic_held_role_census.rs",
        "diagnostic_held_unit_projection.rs",
        "diagnostic_consuming_held_join.rs",
        "diagnostic_manager_channel.rs",
        "diagnostic_original_manager_message.rs",
    ] {
        let current = source(file);
        let current = if file == "diagnostic_held_unit_material.rs" {
            build_guard::material_view(&current)
        } else {
            current
        };
        assert_eq!(current, baseline(file), "{file}");
    }
}

const MODULE: &str = "\n#[cfg(all(target_os = \"linux\", target_arch = \"x86_64\"))]\n#[path = \"../diagnostic_held_activation_ingress.rs\"]\nmod activation;\n";
const ENTRY: &str = "    if std::env::args_os().len() == 1 || activation::requested() {\n        if activation::requested() {\n            if let Err(error) = activation::prepare() {\n                eprintln!(\"Held activation refused: {error}; never retry through legacy entry.\");\n                std::process::exit(2);\n            }\n        }\n";
fn restore(main: &str) -> String {
    assert_eq!(main.matches(MODULE).count(), 1);
    assert_eq!(main.matches(ENTRY).count(), 1);
    main.replacen(MODULE, "", 1)
        .replacen(ENTRY, "    if std::env::args_os().len() == 1 {\n", 1)
}
#[test]
fn activation_ingress_only_two_exact_hooks_change_the_binary_entry() {
    let main = source("bin/diagnostic_held_launcher.rs");
    assert_eq!(restore(&main), baseline("bin/diagnostic_held_launcher.rs"));
    for wrong in [
        main.replace(MODULE, ""),
        format!("{main}{MODULE}"),
        main.replace("mod activation;", "pub mod activation;"),
        main.replace(ENTRY, ""),
    ] {
        assert!(std::panic::catch_unwind(|| restore(&wrong)).is_err());
    }
    assert_ne!(
        restore(&main.replace("kernel::run()", "kernel::skip()")),
        baseline("bin/diagnostic_held_launcher.rs")
    );
}
#[test]
fn activation_ingress_has_no_clock_descriptor_manager_or_unknown_environment_cleanup() {
    let adapter = source("diagnostic_held_activation_ingress.rs");
    let runtime = adapter.split("#[cfg(test)]").next().unwrap();
    let code: String = runtime
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect();
    for word in [
        "unsafe",
        "libc",
        "clock",
        "Command",
        "Connection",
        "Message",
        "env_clear",
        "clearenv",
        "execve",
        "dup",
        "from_raw_fd",
        "SystemTime",
        "Instant",
        "kernel::run",
    ] {
        assert!(!code.contains(word), "{word}");
    }
    assert!(runtime.contains("inspect(&environment, std::process::id())?;"));
    assert!(
        runtime.find("inspect(&environment").unwrap()
            < runtime.find("std::env::remove_var").unwrap()
    );
    assert!(runtime.contains("need(std::env::vars_os().next().is_none())"));
}

#[test]
fn activation_ingress_compatibility_registration_preserves_every_old_format_byte() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let raw = std::fs::read_to_string(root.join("docs/COMPAT.md"))
        .unwrap()
        .replace('\r', "");
    let raw = bootstrap_guard::compat_view(&build_guard::compat_view(&raw));
    let marker = "\n<!-- BEGIN HELD ACTIVATION INGRESS V1 -->\n";
    assert_eq!(raw.matches(marker).count(), 1);
    assert!(raw.ends_with("<!-- END HELD ACTIVATION INGRESS V1 -->\n"));
    let previous = std::process::Command::new("git")
        .args(["--no-replace-objects", "show", "aa58bad7:docs/COMPAT.md"])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(previous.status.success());
    assert_eq!(
        raw.split_once(marker).unwrap().0,
        String::from_utf8(previous.stdout)
            .unwrap()
            .replace('\r', "")
    );
    assert!(raw.contains("release:worker:config:creator:root-a:root-b:alarm:bootstrap"));
    assert!(raw.contains("--systemd-activation-v1"));
}
