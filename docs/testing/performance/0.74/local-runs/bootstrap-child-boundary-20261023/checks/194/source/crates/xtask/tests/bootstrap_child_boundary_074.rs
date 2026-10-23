//! Audit guards only: no new child authentication, IPC or execution path.
use std::path::{Path, PathBuf};
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn source(name: &str) -> String {
    std::fs::read_to_string(root().join("tools/long-run-supervisor-074/src").join(name))
        .unwrap()
        .replace('\r', "")
}
fn root_gate_before_open(raw: &str, function: &str, opener: &str) -> bool {
    let Some((_, body)) = raw.split_once(&format!("pub fn {function}()")) else {
        return false;
    };
    let guard = "if unsafe { libc::geteuid() } != 0 || unsafe { libc::getegid() } != 0 {";
    match (body.find(guard), body.find(opener)) {
        (Some(check), Some(open)) => check < open,
        _ => false,
    }
}
#[test]
fn child_cannot_borrow_root_only_worker_or_held_trust_by_weakening_the_opener() {
    for (file, function, opener) in [
        (
            "diagnostic_worker_trust_input.rs",
            "open_fixed_worker_trust",
            "Source::open(Path::new(\"/\"), 0",
        ),
        (
            "diagnostic_held_runtime_trust.rs",
            "open_fixed_held_runtime_trust",
            "HeldSource::open(Path::new(\"/\"), 0",
        ),
    ] {
        let raw = source(file);
        assert!(root_gate_before_open(&raw, function, opener));
        for changed in [
            raw.replace("libc::geteuid() } != 0", "libc::geteuid() } == 0"),
            raw.replace("libc::getegid() } != 0", "libc::getegid() } == 0"),
            raw.replace(opener, "unreviewed_open("),
        ] {
            assert!(!root_gate_before_open(&changed, function, opener));
        }
    }
}
#[test]
fn sandbox_hardening_is_not_a_host_mount_namespace_enrollment() {
    let raw = source("diagnostic_unit.rs");
    for property in [
        "(\"User\", UnitProperty::Text(\"hydracache-perf\".into()))",
        "(\"Group\", UnitProperty::Text(\"hydracache-perf\".into()))",
        "(\"NoNewPrivileges\", UnitProperty::Boolean(true))",
        "(\"PrivateTmp\", UnitProperty::Boolean(true))",
        "(\"PrivateDevices\", UnitProperty::Boolean(true))",
        "(\"ProtectSystem\", UnitProperty::Text(\"strict\".into()))",
    ] {
        assert!(raw.contains(property), "{property}");
    }
    let projection = source("diagnostic_held_unit_projection.rs");
    assert!(
        projection.contains("crate::diagnostic_unit::build_diagnostic_unit_spec(&context.state)")
    );
    assert!(projection.contains(
        "\"Environment\" | \"ReadWritePaths\" => *value = UnitProperty::Strings(vec![])"
    ));
    let tests = source("diagnostic_held_unit_projection_tests.rs");
    assert!(tests.contains("p.properties.contains(&(*name, value.clone()))"));
    let mount = source("diagnostic_mount_namespace.rs");
    assert!(mount.contains("if namespace_id(&worker)? != original"));
    assert!(mount.contains("FileId(policy.mount_namespace.device, policy.mount_namespace.inode)"));
}
#[test]
fn executable_consistency_does_not_supply_a_systemd_address_for_a_sealed_copy() {
    let image = source("diagnostic_bootstrap_current_image.rs");
    assert!(image.contains("need(current == role)?"));
    assert!(image.contains("libc::F_GET_SEALS) } == SEALS"));
    let parent = source("diagnostic_bootstrap_sealed.rs");
    assert!(parent.contains("libc::F_GET_SEALS"));
    let projection = source("diagnostic_held_unit_projection.rs");
    assert!(projection.contains("\"ExecStart\""));
    assert!(!projection.contains("/proc/self/fd/13"));
    assert!(source("diagnostic_bootstrap_image_entry.rs")
        .contains("original bootstrap authentication is closed"));
}
#[test]
fn boundary_audit_preserves_entire_runtime_and_frozen_qualification() {
    let names = [
        "diagnostic_unit.rs",
        "diagnostic_held_unit_projection.rs",
        "diagnostic_held_unit_projection_tests.rs",
        "diagnostic_mount_namespace.rs",
        "diagnostic_worker_policy.rs",
        "diagnostic_worker_trust_input.rs",
        "diagnostic_held_runtime_trust.rs",
        "diagnostic_bootstrap_sealed.rs",
        "diagnostic_bootstrap_current_image.rs",
        "diagnostic_bootstrap_image_entry.rs",
        "diagnostic_bootstrap_owner.rs",
        "diagnostic_bootstrap_entry_binding.rs",
    ];
    let mut paths: Vec<String> = names
        .iter()
        .map(|n| format!("tools/long-run-supervisor-074/src/{n}"))
        .collect();
    paths.extend(
        [
            "Cargo.lock",
            "docs/COMPAT.md",
            "docs/testing/gated-test-registry.toml",
            "docs/testing/performance/0.74/qualification-manifest.toml",
        ]
        .map(String::from),
    );
    for path in paths {
        let out = std::process::Command::new("git")
            .current_dir(root())
            .args(["--no-replace-objects", "show", &format!("cb7f639d:{path}")])
            .output()
            .unwrap();
        assert!(out.status.success(), "{path}");
        let original = String::from_utf8(out.stdout).unwrap().replace('\r', "");
        assert_eq!(
            std::fs::read_to_string(root().join(&path))
                .unwrap()
                .replace('\r', ""),
            original,
            "{path}"
        );
    }
}
