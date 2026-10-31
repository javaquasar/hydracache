use super::*;
#[test]
fn reference_round_validation_requires_the_observed_value_callback_and_retirement() {
    let value = (117u64 ^ 9).to_be_bytes();
    assert_eq!(
        validate_reference_round(&value, 117, 9, 1, 0).unwrap(),
        state_digest(&value)
    );
    for (bytes, callbacks, retained) in [
        (b"wrong".as_slice(), 1, 0),
        (&value, 0, 0),
        (&value, 2, 0),
        (&value, 1, 1),
    ] {
        assert!(validate_reference_round(bytes, 117, 9, callbacks, retained).is_err());
    }
}
#[test]
fn worker_stop_policy_preserves_cancel_duration_and_hard_work_limits() {
    let bounds = SoakBounds::smoke();
    assert_eq!(stop_reason(&bounds, 0, 0, false), None);
    assert_eq!(stop_reason(&bounds, 1, bounds.active_ms, false), None);
    assert_eq!(
        stop_reason(&bounds, 2, bounds.active_ms, false),
        Some("duration_complete")
    );
    assert_eq!(
        stop_reason(&bounds, bounds.maximum_rounds, 0, false),
        Some("round_limit")
    );
    assert_eq!(
        stop_reason(&bounds, bounds.maximum_rounds + 1, 0, false),
        Some("round_limit")
    );
    assert_eq!(
        stop_reason(&bounds, bounds.maximum_rounds, bounds.active_ms, true),
        Some("cancelled")
    );
}
#[test]
fn worker_configuration_requires_an_owned_absolute_directory_and_a_known_canary() {
    let root = tempfile::tempdir().unwrap();
    let args = vec![
        "120".into(),
        "10".into(),
        "32".into(),
        "2".into(),
        "1000".into(),
        "117".into(),
        "a".repeat(40),
        root.path().to_string_lossy().into_owned(),
    ];
    WorkerConfig::parse(&args).unwrap();
    let mut changed = args.clone();
    changed[7] = "relative".into();
    assert!(WorkerConfig::parse(&changed).is_err());
    let mut changed = vec!["--canary=foreign".into()];
    changed.extend(args);
    assert!(WorkerConfig::parse(&changed).is_err());
}
#[test]
fn hung_worker_checkpoint_is_attempted_once_and_errors_propagate_without_retries() {
    let mut done = false;
    let mut calls = 0;
    checkpoint_once(&mut done, || {
        calls += 1;
        Ok(())
    })
    .unwrap();
    assert!(done);
    assert_eq!(calls, 1);
    checkpoint_once(&mut done, || {
        panic!("a completed checkpoint must not run again")
    })
    .unwrap();
    let mut failed = false;
    assert!(
        checkpoint_once(&mut failed, || Err("unit checkpoint failure".into()))
            .unwrap_err()
            .to_string()
            .contains("unit checkpoint failure")
    );
    assert!(failed);
    checkpoint_once(&mut failed, || {
        panic!("the fault worker does not retry failed checkpoints")
    })
    .unwrap();
}
