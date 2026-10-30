use super::*;
#[test]
fn maven_materials_are_bound_to_real_files_for_repository_and_workspace_locations() {
    let root = tempfile::tempdir().unwrap();
    for location in ["repository", "workspace"] {
        let relative = if location == "repository" {
            format!("{REPOSITORY}/unit.pom")
        } else {
            "tests/unit.pom".into()
        };
        let file = root.path().join(&relative);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, b"unit POM bytes").unwrap();
        let actual = artifact(root.path(), &file, MAX).unwrap();
        let row = json!({"event":"material","location":location,"path":if location=="repository" {"unit.pom"}else{"tests/unit.pom"},"sha256":actual.sha256,"bytes":actual.bytes,"extension":"pom"});
        let mut inputs = MavenInputs::default();
        inputs
            .accept(root.path(), "NativeJava", row.clone())
            .unwrap();
        assert!(inputs.poms.contains(&relative));
        assert_eq!(inputs.materials[0].2, relative);
        for field in ["sha256", "bytes"] {
            let mut wrong = row.clone();
            wrong[field] = if field == "sha256" {
                json!("0".repeat(64))
            } else {
                json!(actual.bytes + 1)
            };
            assert!(MavenInputs::default()
                .accept(root.path(), "NativeJava", wrong)
                .unwrap_err()
                .to_string()
                .contains("substituted after build"));
        }
        fs::remove_file(file).unwrap();
        assert!(MavenInputs::default()
            .accept(root.path(), "NativeJava", row)
            .is_err());
    }
    assert!(!root.path().join("provenance.json").exists());
}
#[test]
fn repeated_tool_and_runtime_inputs_must_remain_identical_across_maven_invocations() {
    let root = tempfile::tempdir().unwrap();
    for row in [
        json!({"event":"tool","kind":"jdk","path":"bin/java","sha256":"0".repeat(64)}),
        json!({"event":"init","maven_version":"3.9","java_version":"21","java_vendor":"unit"}),
    ] {
        let mut inputs = MavenInputs::default();
        inputs
            .accept(root.path(), "JavaContract", row.clone())
            .unwrap();
        inputs
            .accept(root.path(), "NativeJava", row.clone())
            .unwrap();
        let mut changed = row.clone();
        if row["event"] == "tool" {
            changed["sha256"] = json!("1".repeat(64));
        } else {
            changed["java_version"] = json!("17");
        }
        assert!(inputs.accept(root.path(), "NativeJava", changed).is_err());
    }
}
#[test]
fn workspace_metadata_refuses_foreign_or_missing_path_identities() {
    let root = tempfile::tempdir().unwrap();
    let valid = json!({"packages":[{"source":null,"manifest_path":root.path().join("Cargo.toml")},{"source":"registry+unit","manifest_path":"foreign"}]});
    validate_workspace_metadata(root.path(), &valid).unwrap();
    for manifest in [json!("foreign/Cargo.toml"), json!(null)] {
        let mut value = valid.clone();
        value["packages"][0]["manifest_path"] = manifest;
        assert!(validate_workspace_metadata(root.path(), &value).is_err());
    }
    assert!(validate_workspace_metadata(root.path(), &json!({})).is_err());
}
#[test]
fn fixed_tool_materials_refuse_aliases_missing_files_and_changed_hashes() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("bin")).unwrap();
    let tool = root.path().join("bin/tool");
    fs::write(&tool, b"unit bytes; never executed").unwrap();
    let row = json!({"path":"bin/tool","sha256":crate::imap_daemon_probe_075::binary_digest(&tool).unwrap()});
    verify_tool_material(root.path(), &row).unwrap();
    for path in ["../foreign", "a\\b", "C:foreign", "/foreign"] {
        let mut changed = row.clone();
        changed["path"] = json!(path);
        assert!(verify_tool_material(root.path(), &changed)
            .unwrap_err()
            .to_string()
            .contains("foreign tool material path"));
    }
    let mut changed = row.clone();
    changed["sha256"] = json!("0".repeat(64));
    assert!(verify_tool_material(root.path(), &changed)
        .unwrap_err()
        .to_string()
        .contains("input hash differs"));
    fs::remove_file(tool).unwrap();
    assert!(verify_tool_material(root.path(), &row).is_err());
}
