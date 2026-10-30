use super::*;
#[test]
fn announcement_finishes_only_after_both_checkpoints_and_actual_pid_publication() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("pid");
    let mut calls = 0;
    announce_fixture(&file, true, || {
        calls += 1;
        assert_eq!(file.exists(), calls == 2);
        Ok(())
    })
    .unwrap();
    assert_eq!(calls, 2);
    assert_eq!(
        std::fs::read_to_string(file).unwrap(),
        std::process::id().to_string()
    );
}
#[test]
fn each_checkpoint_failure_and_actual_pid_io_failure_propagates_without_later_work() {
    for fail_at in [1, 2] {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("pid");
        let mut calls = 0;
        assert_eq!(
            announce_fixture(&file, true, || {
                calls += 1;
                if calls == fail_at {
                    Err("unit checkpoint refusal".into())
                } else {
                    Ok(())
                }
            })
            .unwrap_err()
            .to_string(),
            "unit checkpoint refusal"
        );
        assert_eq!(calls, fail_at);
        assert_eq!(file.exists(), fail_at == 2);
    }
    let root = tempfile::tempdir().unwrap();
    let mut calls = 0;
    assert!(announce_fixture(root.path(), true, || {
        calls += 1;
        Ok(())
    })
    .is_err());
    assert_eq!(calls, 1);
}
