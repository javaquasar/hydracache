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

    config.baseline_status = xtask::coverage_ratchet::BaselineStatus::Unmeasured;
    config.baseline_lines_percent = 0.0;
    config.baseline_commit.clear();
    config.baseline_toolchain.clear();
    config.ignored_source_regex = "crates/".to_owned();
    let problems = xtask::coverage_ratchet::validate_contract(&root, &config).unwrap();
    assert!(problems
        .iter()
        .any(|problem| problem.contains("coverage exclusion must remain exactly")));
}

#[test]
fn loadgen_exclusion_requires_a_non_published_development_harness() {
    assert!(
        xtask::coverage_ratchet::loadgen_manifest_is_development_only(
            "[package]\npublish = false\n"
        )
    );
    assert!(
        !xtask::coverage_ratchet::loadgen_manifest_is_development_only(
            "[package]\npublish = true\n"
        )
    );
    assert!(
        !xtask::coverage_ratchet::loadgen_manifest_is_development_only(
            "[package]\nname = \"loadgen\"\n"
        )
    );
    assert!(!xtask::coverage_ratchet::loadgen_manifest_is_development_only("not valid toml = ["));
}

#[test]
fn measured_coverage_is_checked_by_the_ratchet_with_an_actionable_error() {
    assert!(xtask::coverage_ratchet::enforce_floor(88.0, 88.0).is_ok());
    assert!(xtask::coverage_ratchet::enforce_floor(91.25, 88.0).is_ok());

    let error = xtask::coverage_ratchet::enforce_floor(87.495, 88.0).unwrap_err();
    assert_eq!(error, "measured line coverage 87.50% is below 88.00%");
}

#[test]
fn coverage_plan_runs_default_before_additive_tiers_and_reports_once() {
    let root = xtask::doc_check::find_repo_root().unwrap();
    let config = xtask::coverage_ratchet::load_config(&root).unwrap();
    let plan = xtask::coverage_ratchet::measurement_plan(&config);

    assert!(xtask::coverage_ratchet::validate_measurement_plan(&plan, &config).is_empty());
    assert_eq!(
        plan.iter().map(|step| step.id).collect::<Vec<_>>(),
        [
            "clean",
            "default-workspace",
            "raft-sled-log-store",
            "raft-test-failpoints",
            "db-postgres-outbox",
            "server-networked-daemon",
            "report"
        ]
    );
    assert_eq!(
        plan.iter()
            .filter(|step| step.kind == xtask::coverage_ratchet::CoverageStepKind::Clean)
            .count(),
        1
    );
    let report = plan.last().unwrap();
    assert_eq!(
        report.kind,
        xtask::coverage_ratchet::CoverageStepKind::Report
    );
    assert!(report.args.iter().any(|arg| arg == "--json"));
    assert!(report
        .args
        .iter()
        .any(|arg| arg == &config.raw_report_artifact));
    let ignore = report
        .args
        .windows(2)
        .find(|window| window[0] == "--ignore-filename-regex")
        .expect("coverage report must declare its reviewed source exclusion");
    assert_eq!(ignore[1], config.ignored_source_regex);
    assert_eq!(
        config.ignored_source_regex,
        "(^|/)crates/(xtask|hydracache-loadgen)/"
    );
    assert!(!report.args.iter().any(|arg| arg == "--fail-under-lines"));
    let networked = plan
        .iter()
        .find(|step| step.id == "server-networked-daemon")
        .unwrap();
    assert_eq!(
        networked.environment,
        [("HYDRACACHE_RUN_NETWORKED_DAEMON_E2E", "1")]
    );
    for step in plan.iter().filter(|step| {
        matches!(
            step.kind,
            xtask::coverage_ratchet::CoverageStepKind::DefaultTests
                | xtask::coverage_ratchet::CoverageStepKind::AdditiveTests
        )
    }) {
        assert!(step.args.iter().any(|arg| arg == "--no-report"));
        assert!(!step.args.iter().any(|arg| arg == "--no-clean"));
    }
}

#[test]
fn coverage_plan_rejects_a_required_tier_skip_or_second_clean() {
    let root = xtask::doc_check::find_repo_root().unwrap();
    let config = xtask::coverage_ratchet::load_config(&root).unwrap();

    let mut missing_tier = xtask::coverage_ratchet::measurement_plan(&config);
    missing_tier.retain(|step| step.id != "raft-test-failpoints");
    let problems = xtask::coverage_ratchet::validate_measurement_plan(&missing_tier, &config);
    assert!(problems
        .iter()
        .any(|problem| problem.contains("required steps in order")));

    let mut second_clean = xtask::coverage_ratchet::measurement_plan(&config);
    second_clean.insert(1, second_clean[0].clone());
    let problems = xtask::coverage_ratchet::validate_measurement_plan(&second_clean, &config);
    assert!(problems
        .iter()
        .any(|problem| problem.contains("exactly one clean step")));

    let mut incompatible_flags = xtask::coverage_ratchet::measurement_plan(&config);
    incompatible_flags[1].args.push("--no-clean".to_owned());
    let problems = xtask::coverage_ratchet::validate_measurement_plan(&incompatible_flags, &config);
    assert!(problems
        .iter()
        .any(|problem| problem.contains("incompatible --no-clean and --no-report")));
}
