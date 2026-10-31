use super::*;
fn layout(root: &Path) {
    for directory in ["bin", "lib", "lib/rustlib/unit-host/lib"] {
        fs::create_dir_all(root.join(directory)).unwrap();
    }
}
#[test]
fn rust_hosts_and_material_budgets_refuse_unbounded_or_foreign_inputs() {
    for host in ["", "../host", "x/y", "space host", &"a".repeat(129)] {
        assert!(validate_rust_host(host).is_err());
    }
    validate_rust_host("x86_64-pc-windows-msvc").unwrap();
    validate_rust_host(&"a".repeat(128)).unwrap();
    let limit = 2 * 1024 * 1024 * 1024u64;
    assert_eq!(rust_material_budget(limit - 1, 1, 511).unwrap(), limit);
    for (total, size, count) in [(limit, 1, 0), (0, 1, 512), (u64::MAX, 1, 0)] {
        assert!(rust_material_budget(total, size, count).is_err());
    }
    assert!(
        rust_distribution(Path::new("unit-must-not-be-read"), "../bad")
            .unwrap_err()
            .to_string()
            .contains("host bound")
    );
}
#[test]
fn distribution_collects_only_bound_materials_and_rejects_missing_or_empty_inputs() {
    let root = tempfile::tempdir().unwrap();
    assert!(rust_distribution(root.path(), "unit-host").is_err());
    layout(root.path());
    assert!(rust_distribution(root.path(), "unit-host")
        .unwrap_err()
        .to_string()
        .contains("inputs missing"));
    fs::write(root.path().join("bin/ignored.txt"), b"ignored").unwrap();
    fs::write(
        root.path().join("bin/compiler.dll"),
        b"unit bytes; not a compiler",
    )
    .unwrap();
    fs::write(
        root.path().join("lib/rustlib/unit-host/lib/libstd.rlib"),
        b"unit bytes; not a standard library",
    )
    .unwrap();
    let materials = rust_distribution(root.path(), "unit-host").unwrap();
    assert_eq!(materials.len(), 2);
    assert_eq!(materials[0]["kind"], "rust-distribution");
    assert_eq!(materials[0]["path"], "bin/compiler.dll");
    assert!(!root.path().join("capture.json").exists());
    assert!(!root.path().join("report.json").exists());
}
#[test]
fn actual_distribution_directory_entry_bound_refuses_before_unbounded_enumeration() {
    let root = tempfile::tempdir().unwrap();
    layout(root.path());
    for i in 0..1025 {
        fs::write(root.path().join(format!("bin/ignored-{i}.txt")), b"x").unwrap();
    }
    assert!(rust_distribution(root.path(), "unit-host")
        .unwrap_err()
        .to_string()
        .contains("directory bound"));
}

#[test]
fn maven_launcher_spellings_and_search_roots_do_not_change_the_process_environment() {
    assert_eq!(
        launcher_spelling(Path::new(r"\\?\UNC\server\share\maven\bin\mvn.cmd")),
        PathBuf::from(r"\\server\share\maven\bin\mvn.cmd")
    );
    assert_eq!(
        launcher_spelling(Path::new(r"\\?\C:\maven\bin\mvn.cmd")),
        PathBuf::from(r"C:\maven\bin\mvn.cmd")
    );
    assert!(maven_home_in(Vec::<PathBuf>::new())
        .unwrap_err()
        .to_string()
        .contains("launcher missing"));
    let root = tempfile::tempdir().unwrap();
    for directory in ["bin", "lib", "boot"] {
        fs::create_dir(root.path().join(directory)).unwrap();
    }
    let launcher = root
        .path()
        .join("bin")
        .join(if cfg!(windows) { "mvn.cmd" } else { "mvn" });
    fs::write(&launcher, b"unit layout; never executed").unwrap();
    assert_eq!(
        maven_home_in([root.path().join("absent"), root.path().join("bin")]).unwrap(),
        launcher_spelling(&root.path().canonicalize().unwrap())
    );
}
#[test]
fn maven_boot_inventory_requires_one_launcher_and_bounds_actual_directory_entries() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("maven");
    fs::create_dir_all(home.join("boot")).unwrap();
    assert!(maven_command_at(root.path(), &home)
        .unwrap_err()
        .to_string()
        .contains("ambiguous/missing"));
    for name in ["plexus-classworlds-a.jar", "plexus-classworlds-b.jar"] {
        fs::write(home.join("boot").join(name), b"unit; not a JAR").unwrap();
    }
    assert!(maven_command_at(root.path(), &home)
        .unwrap_err()
        .to_string()
        .contains("ambiguous/missing"));
    fs::remove_file(home.join("boot/plexus-classworlds-b.jar")).unwrap();
    let command = maven_command_at(root.path(), &home).unwrap();
    assert_eq!(command.get_current_dir(), Some(root.path()));
    for i in 0..256 {
        fs::write(home.join("boot").join(format!("ignored-{i}.txt")), b"x").unwrap();
    }
    assert!(maven_command_at(root.path(), &home)
        .unwrap_err()
        .to_string()
        .contains("boot directory bound"));
    assert!(!root.path().join("report.json").exists());
}
#[test]
fn rust_version_host_requires_one_bounded_path_free_host() {
    assert_eq!(
        rust_version_host("rustc 1.94\nhost: unit-host\n").unwrap(),
        "unit-host"
    );
    for version in [
        "rustc 1.94".into(),
        "host: ".into(),
        "host: ../foreign".into(),
        "host: bad host".into(),
        format!("host: {}", "a".repeat(129)),
    ] {
        assert!(rust_version_host(&version).is_err());
    }
    assert_eq!(
        rust_version_host(&format!("host: {}", "a".repeat(128)))
            .unwrap()
            .len(),
        128
    );
}
#[test]
fn installed_maven_home_requires_an_existing_absolute_directory_and_preserves_its_spelling() {
    let root = tempfile::tempdir().unwrap();
    let valid = format!("Apache Maven unit\nMaven home: {}\n", root.path().display());
    assert_eq!(installed_maven_home(&valid).unwrap(), root.path());
    let file = root.path().join("plain-file");
    fs::write(&file, b"not a directory").unwrap();
    for output in [
        "missing home line".into(),
        "Maven home: relative".into(),
        format!("Maven home: {}", root.path().join("missing").display()),
        format!("Maven home: {}", file.display()),
    ] {
        assert!(installed_maven_home(&output).is_err());
    }
}
#[test]
fn canonical_repository_scope_refuses_a_foreign_sibling() {
    let root = tempfile::tempdir().unwrap();
    let canonical = root.path().canonicalize().unwrap();
    verify_repository_scope(&canonical.join(REPOSITORY), &canonical).unwrap();
    let foreign = tempfile::tempdir().unwrap();
    assert!(
        verify_repository_scope(&foreign.path().canonicalize().unwrap(), &canonical)
            .unwrap_err()
            .to_string()
            .contains("foreign Maven repository refused")
    );
}
