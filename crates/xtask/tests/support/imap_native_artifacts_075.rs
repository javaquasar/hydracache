use super::*;
fn inputs(root: &Path) -> [std::path::PathBuf; 3] {
    let paths = [
        root.join("unit-client"),
        root.join("unit-adapter"),
        root.join("unit-dependency"),
    ];
    for path in &paths {
        std::fs::write(path, b"unit material; not an executable or JAR").unwrap();
    }
    paths
}
#[test]
fn native_pin_and_launch_errors_never_yield_an_observation() {
    let root = tempfile::tempdir().unwrap();
    let paths = inputs(root.path());
    assert!(execute_pinned(&paths, "wrong-pin", || panic!(
        "foreign dependency must not launch"
    ))
    .unwrap_err()
    .to_string()
    .contains("native dependency pin differs"));
    let pin = crate::imap_daemon_probe_075::binary_digest(&paths[2]).unwrap();
    assert_eq!(
        execute_pinned(&paths, &pin, || Err("unit launch refusal".into()))
            .unwrap_err()
            .to_string(),
        "unit launch refusal"
    );
    std::fs::remove_file(&paths[0]).unwrap();
    assert!(execute_pinned(&paths, &pin, || panic!("missing input must not launch")).is_err());
}
#[test]
fn every_native_input_is_rehashed_after_launch_and_no_fixture_is_qualified() {
    for changed in 0..3 {
        let root = tempfile::tempdir().unwrap();
        let paths = inputs(root.path());
        let pin = crate::imap_daemon_probe_075::binary_digest(&paths[2]).unwrap();
        assert!(execute_pinned(&paths, &pin, || {
            std::fs::write(&paths[changed], b"substituted during unit launch")?;
            Ok(b"unit-invalid-observation".to_vec())
        })
        .unwrap_err()
        .to_string()
        .contains("native artifact changed while running"));
    }
    let root = tempfile::tempdir().unwrap();
    let paths = inputs(root.path());
    let pin = crate::imap_daemon_probe_075::binary_digest(&paths[2]).unwrap();
    let bytes = execute_pinned(&paths, &pin, || Ok(b"unit-invalid-observation".to_vec())).unwrap();
    assert!(serde_json::from_slice::<NativeProbe>(&bytes).is_err());
    assert!(!root.path().join("report.json").exists());
}
