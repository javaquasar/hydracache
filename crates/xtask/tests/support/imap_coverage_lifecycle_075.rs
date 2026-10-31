use super::*;
use std::io::{Error as IoError, ErrorKind};

fn controller() -> crate::imap_soak_075::WorkerIdentity {
    crate::imap_soak_075::WorkerIdentity {
        pid: 123,
        platform_birth: "unit-birth".into(),
        incarnation: "coverage-123".into(),
    }
}

#[test]
fn only_an_observed_exit_or_pid_reuse_closes_the_coverage_controller() {
    let identity = controller();
    assert!(dead_using(&identity, |pid| {
        assert_eq!(pid, identity.pid);
        Ok(identity.platform_birth.clone())
    })
    .unwrap_err()
    .to_string()
    .contains("still lives"));
    dead_using(&identity, |_| Ok("another-birth".into())).unwrap();
    for kind in [ErrorKind::NotFound, ErrorKind::InvalidInput] {
        dead_using(&identity, |_| Err(IoError::from(kind).into())).unwrap();
    }
    for kind in [ErrorKind::PermissionDenied, ErrorKind::Other] {
        let error = dead_using(&identity, |_| Err(IoError::from(kind).into())).unwrap_err();
        assert_eq!(error.downcast_ref::<IoError>().unwrap().kind(), kind);
    }
    assert_eq!(
        dead_using(&identity, |_| Err("unclassified identity failure".into()))
            .unwrap_err()
            .to_string(),
        "unclassified identity failure"
    );
    // The Windows invalid-PID code is already covered by InvalidInput above.
    #[cfg(windows)]
    {
        assert_eq!(
            IoError::from_raw_os_error(87).kind(),
            ErrorKind::InvalidInput
        );
        dead_using(&identity, |_| Err(IoError::from_raw_os_error(87).into())).unwrap();
    }
}

#[test]
fn malformed_controller_identity_refuses_before_any_process_lookup() {
    for mode in 0..4 {
        let mut identity = controller();
        match mode {
            0 => identity.pid = 0,
            1 => identity.platform_birth.clear(),
            2 => identity.platform_birth = "x".repeat(257),
            _ => identity.incarnation = "foreign".into(),
        }
        assert!(
            dead_using(&identity, |_| panic!("invalid identity must not be read"))
                .unwrap_err()
                .to_string()
                .contains("invalid coverage controller identity")
        );
    }
    let mut identity = controller();
    identity.platform_birth = "x".repeat(256);
    dead_using(&identity, |_| Ok("reused".into())).unwrap();
}

#[test]
fn unavailable_cfg_spans_never_remove_changed_line_obligations() {
    let source = Source {
        path: "crates/xtask/src/unit.rs".into(),
        sha256: "0".repeat(64),
        bytes: 1,
        added: [2].into_iter().collect(),
        has_executable: true,
    };
    for (first, last) in [(0, 0), (0, 2), (3, 2)] {
        let mut ranges = vec![];
        let mut failure = None;
        assert!(!record_excluded_cfg(
            &source,
            "unit-disabled-cfg".into(),
            first,
            last,
            &mut ranges,
            &mut failure
        ));
        assert!(ranges.is_empty());
        assert_eq!(
            failure.as_deref(),
            Some("cfg source span locations unavailable")
        );
    }
    let mut ranges = vec![];
    let mut failure = None;
    assert!(!record_excluded_cfg(
        &source,
        "unit-disabled-cfg".into(),
        3,
        4,
        &mut ranges,
        &mut failure
    ));
    assert!(ranges.is_empty() && failure.is_none());
    assert!(!record_excluded_cfg(
        &source,
        "unit-disabled-cfg".into(),
        1,
        2,
        &mut ranges,
        &mut failure
    ));
    assert_eq!(
        ranges,
        vec![CfgRange {
            source: source.path,
            first: 1,
            last: 2,
            predicate: "unit-disabled-cfg".into()
        }]
    );
    assert!(failure.is_none());
}

#[test]
fn scoped_git_inventory_refuses_malformed_diff_encoding_and_foreign_source_names() {
    let root = tempfile::tempdir().unwrap();
    for mode in 0..3 {
        let error = source_snapshot_using(root.path(), BASELINE, |observed, args, _| {
            assert_eq!(observed, root.path());
            if args.contains(&"rev-list") {
                return Ok("0".repeat(40).into_bytes());
            }
            if args.contains(&"--name-only") {
                return Ok(if mode == 0 {
                    b"crates/xtask/src/unit.rs\0".to_vec()
                } else {
                    vec![]
                });
            }
            if args.contains(&"--unified=0") {
                return Ok(vec![0xff]);
            }
            if args.contains(&"ls-files") {
                return Ok(if mode == 1 {
                    vec![0xff, 0]
                } else {
                    b"foreign.rs\0".to_vec()
                });
            }
            panic!("unexpected scoped Git command: {args:?}")
        })
        .unwrap_err();
        if mode == 2 {
            assert!(error.to_string().contains("foreign source"));
        }
    }
    assert!(
        source_snapshot_using(root.path(), "bad-baseline", |_, _, _| panic!(
            "invalid baseline must not execute Git"
        ))
        .is_err()
    );
    assert!(!root.path().join("report.json").exists());
}
