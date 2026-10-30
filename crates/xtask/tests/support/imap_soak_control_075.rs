use super::*;
#[test]
fn atomic_control_markers_are_closed_idempotent_and_never_replace_foreign_data() {
    let root = tempfile::tempdir().unwrap();
    assert!(complete_marker(
        root.path(),
        "cancel",
        Err(std::io::Error::from(std::io::ErrorKind::AlreadyExists))
    )
    .is_err());
    for name in ["start", "cancel"] {
        assert!(!marker(root.path(), name).unwrap());
        publish_marker(root.path(), name).unwrap();
        publish_marker(root.path(), name).unwrap();
        assert!(marker(root.path(), name).unwrap());
        fs::write(root.path().join(name), b"foreign marker").unwrap();
        assert!(publish_marker(root.path(), name).is_err());
        assert_eq!(fs::read(root.path().join(name)).unwrap(), b"foreign marker");
    }
    for name in ["", "foreign", "../cancel", "start\0"] {
        assert!(marker(root.path(), name).is_err());
        assert!(publish_marker(root.path(), name).is_err());
    }
    assert!(complete_marker(
        root.path(),
        "start",
        Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
    )
    .is_err());
    fs::remove_file(root.path().join("cancel")).unwrap();
    fs::create_dir(root.path().join("cancel")).unwrap();
    assert!(marker(root.path(), "cancel").is_err());
}
#[test]
fn control_directories_reject_redirects_and_traversal_before_creating_any_directory() {
    let root = tempfile::tempdir().unwrap();
    let canonical = root.path().canonicalize().unwrap();
    plain_control_component(false).unwrap();
    assert!(plain_control_component(true).is_err());
    control_scope(&canonical, &canonical.join("owned")).unwrap();
    assert!(control_scope(&canonical, canonical.parent().unwrap()).is_err());
    assert!(prepare_control(root.path(), &root.path().join("../escaped")).is_err());
    let owned = root.path().join("owned");
    prepare_control(root.path(), &owned).unwrap();
    assert!(owned.is_dir());
    assert!(prepare_control(root.path(), &owned).is_err());
}
#[test]
fn a_real_directory_redirect_cannot_create_soak_controls_in_another_root() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let link = root.path().join("redirect");
    #[cfg(windows)]
    {
        let mut command = Command::new("powershell.exe");
        command.args(["-NoLogo","-NoProfile","-NonInteractive","-Command",
            "$ErrorActionPreference='Stop'; New-Item -ItemType Junction -Path $env:IMAP075_TEST_JUNCTION -Value $env:IMAP075_TEST_DESTINATION | Out-Null"])
            .env("IMAP075_TEST_JUNCTION",&link).env("IMAP075_TEST_DESTINATION",outside.path());
        process::execute(
            command,
            &[],
            process::ProcessBounds {
                timeout: Duration::from_secs(20),
                ..Default::default()
            },
        )
        .unwrap();
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(outside.path(), &link).unwrap();
    assert!(prepare_control(root.path(), &link.join("must-not-create")).is_err());
    assert!(!outside.path().join("must-not-create").exists());
    #[cfg(windows)]
    fs::remove_dir(link).unwrap();
    #[cfg(unix)]
    fs::remove_file(link).unwrap();
}
