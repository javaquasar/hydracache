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
