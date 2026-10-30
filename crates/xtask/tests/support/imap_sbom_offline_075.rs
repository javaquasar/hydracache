use super::*;

#[test]
fn offline_schema_retrieval_has_a_closed_catalog_and_never_fetches_unknown_uris() {
    use jsonschema::Retrieve;
    for uri in [
        "http://cyclonedx.org/schema/spdx.schema.json",
        "https://cyclonedx.org/schema/spdx.schema.json",
        "http://cyclonedx.org/schema/jsf-0.82.schema.json",
        "https://cyclonedx.org/schema/jsf-0.82.schema.json",
    ] {
        let uri = jsonschema::Uri::parse(uri.to_owned()).unwrap();
        assert!(OfflineSchemas.retrieve(&uri).unwrap().is_object());
    }
    for uri in [
        "https://example.invalid/foreign.json",
        "file:///foreign.json",
    ] {
        let uri = jsonschema::Uri::parse(uri.to_owned()).unwrap();
        assert_eq!(
            OfflineSchemas.retrieve(&uri).unwrap_err().to_string(),
            "unregistered offline schema reference"
        );
    }
}

#[test]
fn missing_root_components_refuse_before_attempting_to_read_any_product_binary() {
    let root = tempfile::tempdir().unwrap();
    assert!(current_roots(root.path(), &BTreeMap::new())
        .unwrap_err()
        .to_string()
        .contains("ambiguous/missing artifact component hydracache-server"));
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
}

fn policy_report(root: &Path, stderr: &[u8]) -> crate::imap_build_process_075::BuildReport {
    use crate::imap_build_process_075::*;
    let out = root.join("unit.stdout");
    let err = root.join("unit.stderr");
    fs::write(&out, b"").unwrap();
    fs::write(&err, stderr).unwrap();
    BuildReport {
        bounds: BuildBounds::default(),
        scope: "unit-policy-transport-only".into(),
        stage: BuildStage::ProvenanceMetadata,
        outcome: BuildOutcome::Completed,
        elapsed_ms: 1,
        effective_timeout_ms: 100,
        stdout_path: out.to_string_lossy().into_owned(),
        stderr_path: err.to_string_lossy().into_owned(),
        stdout_bytes: 0,
        stderr_bytes: stderr.len(),
        stdout_sha256: digest(b""),
        stderr_sha256: digest(stderr),
        pid: None,
        root_reaped: true,
        remaining_live_members: Some(0),
        containment: "unit-no-child".into(),
        journal_complete: true,
        logs_complete: true,
    }
}

#[test]
fn policy_transport_never_promotes_unknown_java_or_incomplete_process_observations() {
    use crate::imap_build_process_075::{BuildError, BuildOutcome};
    for mode in 0..8 {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("target/imap-075/unit-policy");
        fs::create_dir_all(&directory).unwrap();
        let mut report = policy_report(root.path(), b"{\"type\":\"summary\"}\n");
        match mode {
            1 => report.logs_complete = false,
            2 => report.root_reaped = false,
            3 => report.remaining_live_members = Some(1),
            4 => report.outcome = BuildOutcome::Failed,
            5 => {
                fs::remove_file(&report.stderr_path).unwrap();
            }
            6 => {
                fs::write(
                    &report.stderr_path,
                    b"{\"fields\":{\"severity\":\"error\",\"code\":\"vulnerability\"}}\n",
                )
                .unwrap();
                report.outcome = BuildOutcome::Failed;
            }
            7 => report.outcome = BuildOutcome::TimedOut,
            _ => {}
        }
        let outcome = if mode >= 6 {
            Err(BuildError {
                report: Box::new(report),
                detail: "unit scanner refusal".into(),
            })
        } else {
            Ok(report)
        };
        let policy = policy_from_outcome(root.path(), &directory, outcome).unwrap();
        assert_eq!(policy.java_state, PolicyState::Unknown);
        let expected = if mode == 0 {
            PolicyState::Passed
        } else if mode == 6 {
            PolicyState::Failed
        } else {
            PolicyState::Unknown
        };
        assert_eq!(policy.rust_state, expected, "mode {mode}");
        assert_eq!(
            policy.state,
            if mode == 6 {
                PolicyState::Failed
            } else {
                PolicyState::Unknown
            }
        );
        assert!(!directory.join("provenance.json").exists());
        assert!(!directory.join("report.json").exists());
    }
}

#[test]
fn policy_report_missing_stdout_stays_unknown_and_real_publication_errors_propagate() {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("target/imap-075/unit-policy");
    fs::create_dir_all(&directory).unwrap();
    let report = policy_report(root.path(), b"");
    fs::remove_file(&report.stdout_path).unwrap();
    assert_eq!(
        policy_from_outcome(root.path(), &directory, Ok(report))
            .unwrap()
            .state,
        PolicyState::Unknown
    );
    let blocked = directory.join("blocked");
    fs::write(&blocked, b"a file, not an output directory").unwrap();
    let report = policy_report(root.path(), b"{\"type\":\"summary\"}\n");
    assert!(policy_from_outcome(root.path(), &blocked, Ok(report)).is_err());
}

#[test]
fn pinned_provenance_validator_refuses_malformed_schema_and_values() {
    assert!(validate_provenance_with_schema("{", &json!(false)).is_err());
    assert!(
        validate_provenance_with_schema(r#"{"type":"future-unknown-type"}"#, &json!(false))
            .is_err()
    );
    validate_provenance_with_schema(r#"{"type":"boolean"}"#, &json!(false)).unwrap();
    assert_eq!(
        validate_provenance_with_schema(r#"{"type":"boolean"}"#, &json!(42))
            .unwrap_err()
            .to_string(),
        "development provenance schema rejected"
    );
}
#[test]
fn independent_policy_replay_keeps_unknown_and_refuses_raw_or_state_substitution() {
    for mode in 0..5 {
        let root = tempfile::tempdir().unwrap();
        let mut p = policy_from_outcome(
            root.path(),
            root.path(),
            Ok(policy_report(root.path(), b"")),
        )
        .unwrap();
        assert_eq!(p.state, PolicyState::Unknown);
        assert_eq!(p.rust_state, PolicyState::Unknown);
        verify_policy(root.path(), &p).unwrap();
        let path = root.path().join(&p.report.path);
        match mode {
            0 => {
                fs::write(&path, b"{").unwrap();
                p.report = artifact(root.path(), &path, MAX).unwrap();
            }
            1 => {
                fs::remove_file(&path).unwrap();
            }
            2 | 4 => {
                let mut raw: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
                if mode == 2 {
                    raw["rust_state"] = json!("passed");
                } else {
                    raw["stdout"] = json!("tampered");
                }
                fs::write(&path, serde_json::to_vec(&raw).unwrap()).unwrap();
                p.report = artifact(root.path(), &path, MAX).unwrap();
            }
            _ => p.state = PolicyState::Passed,
        }
        assert!(verify_policy(root.path(), &p).is_err());
        assert!(!root.path().join("provenance.json").exists());
    }
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("not-directory");
    fs::write(&file, b"unit").unwrap();
    assert!(capture_policy(&file, root.path()).is_err());
}
