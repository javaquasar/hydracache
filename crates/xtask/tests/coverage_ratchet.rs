#[test]
fn coverage_floor_matches_post_064_measured_baseline_without_decreasing_88() {
    let root = xtask::doc_check::find_repo_root().unwrap();
    let config = xtask::coverage_ratchet::load_config(&root).unwrap();
    let problems = xtask::coverage_ratchet::validate_contract(&root, &config).unwrap();
    assert!(problems.is_empty(), "{problems:#?}");
    assert!(config.configured_floor_percent >= 88.0);
    if config.baseline_status == xtask::coverage_ratchet::BaselineStatus::Measured {
        assert_eq!(
            config.configured_floor_percent,
            88.0_f64.max(config.baseline_lines_percent.floor())
        );
    }
}

#[test]
fn coverage_contract_rejects_floor_below_88_or_mismatched_measured_baseline() {
    let root = xtask::doc_check::find_repo_root().unwrap();
    let mut config = xtask::coverage_ratchet::load_config(&root).unwrap();
    config.configured_floor_percent = 87.0;
    let problems = xtask::coverage_ratchet::validate_contract(&root, &config).unwrap();
    assert!(problems.iter().any(|problem| problem.contains("below 88")));

    config.configured_floor_percent = 88.0;
    config.baseline_status = xtask::coverage_ratchet::BaselineStatus::Measured;
    config.baseline_lines_percent = 91.7;
    config.baseline_commit = "f".repeat(40);
    config.baseline_toolchain = "rustc test".to_owned();
    let problems = xtask::coverage_ratchet::validate_contract(&root, &config).unwrap();
    assert!(problems
        .iter()
        .any(|problem| problem.contains("floor=max(88,floor(lines))")));
}
