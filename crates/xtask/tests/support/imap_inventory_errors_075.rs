use super::*;
#[test]
fn cargo_target_stops_at_each_rustc_failure_and_keeps_fixed_arguments() {
    for fail_at in [1, 2] {
        let mut calls = 0;
        let result = CargoTarget::current_using(|args| {
            calls += 1;
            assert_eq!(
                args,
                if calls == 1 {
                    vec!["-vV"]
                } else {
                    vec!["--print", "cfg"]
                }
            );
            if calls == fail_at {
                Err("unit rustc refused".into())
            } else {
                Ok("host: unit-host".into())
            }
        });
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("unit rustc refused"));
        assert_eq!(calls, fail_at);
    }
    assert!(CargoTarget::current_using(|_| Ok("no host".into())).is_err());
    let mut calls = 0;
    let target = CargoTarget::current_using(|_| {
        calls += 1;
        Ok(if calls == 1 {
            "host: unit-host".into()
        } else {
            "unix\ntarget_os=\"linux\"\n".into()
        })
    })
    .unwrap();
    assert_eq!(target.triple, "unit-host");
    assert!(target.applies("cfg(unix)").unwrap());
    assert!(CargoTarget::rustc_output(&["--unit-imap075-invalid-option"]).is_err());
}
#[test]
fn cyclone_schema_input_refuses_malformed_json_schema_and_values() {
    let bom = json!({"bomFormat":"CycloneDX","specVersion":"1.6"});
    assert!(validate_cyclone_dx_with_schema(&bom, "{").is_err());
    assert!(validate_cyclone_dx_with_schema(&bom, r#"{"type":"not-a-schema-type"}"#).is_err());
    let schema = r#"{"type":"object","required":["unit"]}"#;
    assert!(validate_cyclone_dx_with_schema(&bom, schema).is_err());
    validate_cyclone_dx_with_schema(
        &json!({"bomFormat":"CycloneDX","specVersion":"1.6","unit":true}),
        schema,
    )
    .unwrap();
}
#[test]
fn pom_metadata_writer_and_launch_failures_never_publish_inventory() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("target/imap-075")).unwrap();
    let paths = BTreeSet::from(["tests/unit/pom.xml".into()]);
    assert!(pom_metadata_using(
        root.path(),
        &paths,
        |_, _| Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied)),
        |_| panic!("failed list write must not launch Java")
    )
    .is_err());
    assert!(pom_metadata_using(
        root.path(),
        &paths,
        |file, bytes| file.write_all(bytes),
        |command| {
            assert!(command
                .get_args()
                .any(|a| a == "io.hydracache.imap.provenance.MavenInventoryMain075"));
            Err("unit metadata command refused".into())
        }
    )
    .unwrap_err()
    .to_string()
    .contains("unit metadata command refused"));
    assert!(pom_metadata_using(
        root.path(),
        &paths,
        |file, bytes| file.write_all(bytes),
        |_| Ok(b"{".to_vec())
    )
    .is_err());
    assert_eq!(
        pom_metadata_using(
            root.path(),
            &paths,
            |file, bytes| file.write_all(bytes),
            |_| Ok(b"{}".to_vec())
        )
        .unwrap(),
        json!({})
    );
    assert!(execute_pom_command(Command::new(root.path().join("missing-java"))).is_err());
    assert!(!root.path().join("provenance.json").exists());
    assert!(!root.path().join("report.json").exists());
}
