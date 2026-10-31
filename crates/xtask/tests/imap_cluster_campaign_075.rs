use xtask::imap_cluster_campaign_075::*;

#[test]
fn actual_cluster_campaign_retains_seeded_proof_and_negative_control() {
    let report = check_seed(117, 2).unwrap();
    assert_eq!(report["scope"], "local_cluster_schedule");
    assert_eq!(report["production_enabled"], false);
    assert_eq!(report["canary_detected"], true);
    assert_eq!(report["bulk_checks"], 2);
    assert_eq!(report["expiry_checks"], 2);
    assert_eq!(report, check_seed(117, 2).unwrap());
}

#[test]
fn cluster_campaign_cli_rejects_ambiguous_or_out_of_budget_requests() {
    for args in [
        vec![],
        vec!["--release"],
        vec!["--release", "0.74"],
        vec!["--release", "0.75", "--rounds", "7"],
        vec!["--release", "0.75", "--seed", "bad"],
        vec!["--release", "0.75", "--unknown", "1"],
        vec!["--release", "0.75", "--seed", "1", "--seed", "2"],
    ] {
        assert!(run(args.into_iter().map(str::to_owned).collect()).is_err());
    }
    run(vec![
        "--release".into(),
        "0.75".into(),
        "--rounds".into(),
        "1".into(),
    ])
    .unwrap();
}

#[test]
fn counterexamples_keep_original_minimized_seed_and_immutable_fixture_scope() {
    use hydracache_cluster_testkit::value_plane_cluster_chaos_075::*;
    let temp = tempfile::tempdir().unwrap();
    let steps = generated_cluster_schedule(117, 1).unwrap();
    let broken = |candidate: &[ClusterScheduleStep]| {
        replay_cluster_schedule(candidate, Default::default())
            .is_ok_and(|report| history_canary_detected(&report.history))
    };
    let first = retain_failure(
        temp.path(),
        117,
        &steps,
        true,
        "injected observation canary",
        broken,
    )
    .unwrap();
    let original = std::fs::read(first.join("original.json")).unwrap();
    let second = retain_failure(
        temp.path(),
        117,
        &steps,
        true,
        "injected observation canary",
        broken,
    )
    .unwrap();
    assert_ne!(first, second);
    assert_eq!(
        std::fs::read(first.join("original.json")).unwrap(),
        original
    );
    let minimal =
        decode_cluster_schedule(&std::fs::read(first.join("minimized.json")).unwrap()).unwrap();
    assert!(broken(&minimal));
    assert!(minimal.len() < steps.len());
    let metadata: serde_json::Value =
        serde_json::from_slice(&std::fs::read(first.join("metadata.json")).unwrap()).unwrap();
    assert_eq!(metadata["seed"], 117);
    assert_eq!(metadata["fixture"], true);
    assert_eq!(metadata["production_enabled"], false);
    assert_eq!(metadata["scope"], "local_cluster_failure");
    let invalid = tempfile::tempdir().unwrap();
    std::fs::write(invalid.path().join("target"), b"file").unwrap();
    assert!(retain_failure(invalid.path(), 117, &steps, true, "canary", broken).is_err());
    assert!(retain_failure(temp.path(), 117, &steps, true, &"x".repeat(2049), broken).is_err());
    assert!(retain_failure(
        temp.path(),
        117,
        &steps,
        true,
        "not a counterexample",
        |_| false
    )
    .is_err());
    assert!(
        check_script_at_root(temp.path(), 117, &steps, true).unwrap()["linearizable"]
            .as_bool()
            .unwrap()
    );
    let bad = [ClusterScheduleStep::Advance {
        id: 99,
        phase: ClusterPhase::Apply,
    }];
    let error = check_script_at_root(temp.path(), 118, &bad, true)
        .unwrap_err()
        .to_string();
    assert!(error.contains("seed=118") && error.contains("failure artifacts"));
    assert!(!error.contains("retention failed"));
    assert!(check_script_at_root(temp.path(), 118, &[], true).is_ok());
}
