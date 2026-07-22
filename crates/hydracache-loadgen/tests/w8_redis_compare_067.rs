use std::net::{Ipv4Addr, SocketAddr};

use hydracache_loadgen::compare_redis::{
    w8_boundary_canary_red, RedisComparisonScenario, W3_DAEMON_LIFECYCLE_RELATIVE_PATH,
    W3_EXTERNAL_REPORT_RELATIVE_PATH, W3_OPEN_LOOP_REPORT_RELATIVE_PATH,
    W3_SUITE_RECEIPT_RELATIVE_PATH, W8_CANARY_MARKER, W8_CLAIM_SCOPE, W8_INTERPRETATION,
    W8_MEASUREMENT_ID, W8_METHOD, W8_REPORT_RELATIVE_PATH,
};

const SCENARIO: &str =
    include_str!("../../../docs/testing/perf-scenarios/0.67/compare-redis-v1.toml");

#[test]
fn w8_contract_is_same_box_same_tool_and_immutable() {
    let scenario = RedisComparisonScenario::parse_toml(SCENARIO).unwrap();
    assert_eq!(scenario.measurement_id, W8_MEASUREMENT_ID);
    assert_eq!(scenario.methodology, W8_METHOD);
    assert_eq!(scenario.claim_scope, W8_CLAIM_SCOPE);
    assert_eq!(scenario.interpretation, W8_INTERPRETATION);
    assert_eq!(scenario.repeats, 5);
    assert_eq!(scenario.pipelines, [1, 10]);
    assert_eq!(scenario.operations, ["get", "set"]);
    assert_eq!(scenario.tool.expected_version, "redis-benchmark 7.2.5");
    assert_eq!(
        scenario.docker.image_index_digest,
        "sha256:3aaec283e6e593bde528077d60280ac1589887067a39273348860837c9346d7e"
    );
    assert_eq!(
        scenario.docker.image_platform_manifest_digest,
        "sha256:301f993bbc91d0b50b0737a97962905657d9f595e9935282b7db16a563b53d1b"
    );
    assert!(scenario.image_reference().contains("@sha256:"));
    assert!(!scenario
        .interpretation
        .to_ascii_lowercase()
        .contains("faster than redis"));
}

#[test]
fn both_targets_receive_identical_workload_dimensions() {
    let scenario = RedisComparisonScenario::parse_toml(SCENARIO).unwrap();
    let hydra = SocketAddr::from((Ipv4Addr::LOCALHOST, 6380));
    let redis = SocketAddr::from((Ipv4Addr::LOCALHOST, 6381));
    for pipeline in [1, 10] {
        let hydra_argv = scenario.benchmark_argv(hydra, pipeline);
        let redis_argv = scenario.benchmark_argv(redis, pipeline);
        let normalize_endpoint = |mut argv: Vec<String>| {
            argv[2] = "<host>".to_owned();
            argv[4] = "<port>".to_owned();
            argv
        };
        assert_eq!(
            normalize_endpoint(hydra_argv),
            normalize_endpoint(redis_argv)
        );
    }
}

#[test]
fn mutable_image_or_relaxed_method_is_rejected() {
    let exact = RedisComparisonScenario::parse_toml(SCENARIO).unwrap();

    let mut mutable_image = exact.clone();
    mutable_image.docker.image_index_digest = "redis:7.2.5".to_owned();
    assert!(mutable_image.validate().is_err());

    let mut three_repeats = exact.clone();
    three_repeats.repeats = 3;
    assert!(three_repeats.validate().is_err());

    let mut one_pipeline = exact;
    one_pipeline.pipelines = vec![1];
    assert!(one_pipeline.validate().is_err());
}

#[test]
fn unknown_contract_fields_fail_closed() {
    let mutated = SCENARIO.replace(
        "schema_version = 1",
        "schema_version = 1\ncaller_asserted_same_host = true",
    );
    assert!(RedisComparisonScenario::parse_toml(&mutated).is_err());
}

#[test]
fn local_absence_is_loud_and_mandatory_path_is_system_owned() {
    let source = include_str!("../src/compare_redis.rs");
    assert!(source.contains("w8-docker-local-skip-loud"));
    assert!(source.contains("w8-redis-benchmark-local-skip-loud"));
    assert!(source.contains("MandatoryReference"));
    assert!(source.contains("start_reference_daemon"));
    assert!(source.contains("W3ReferenceBinding::load"));
    assert!(source.contains("run_and_write_same_box_redis_comparison"));
    assert!(!source.contains("caller_asserted_same_host"));
    assert!(!source.contains("caller_asserted_pinned"));
}

#[test]
fn w8_requires_the_canonical_four_artifact_w3_predecessor() {
    assert_eq!(
        W3_OPEN_LOOP_REPORT_RELATIVE_PATH,
        "target/test-evidence/0.67/node-resp-open-loop.json"
    );
    assert_eq!(
        W3_DAEMON_LIFECYCLE_RELATIVE_PATH,
        "target/test-evidence/0.67/node-resp-daemon-lifecycle.json"
    );
    assert_eq!(
        W3_EXTERNAL_REPORT_RELATIVE_PATH,
        "target/test-evidence/0.67/node-resp-redis-benchmark.json"
    );
    assert_eq!(
        W3_SUITE_RECEIPT_RELATIVE_PATH,
        "target/test-evidence/0.67/node-resp-suite-receipt.json"
    );
    assert_eq!(
        W8_REPORT_RELATIVE_PATH,
        "target/test-evidence/0.67/compare-redis.json"
    );
    let source = include_str!("../src/compare_redis.rs");
    assert!(source.contains("RespReferenceSuiteReceipt"));
    assert!(source.contains("RespReferenceSuiteEvidence"));
    assert!(source.contains("validate_archived_lifecycle"));
    assert!(source.contains("!process_is_alive(lifecycle.pid)"));
    assert!(source.contains(".validate(&external_contract, &provenance_registry)"));
}

#[test]
fn docker_cleanup_is_armed_before_run_and_report_publication_is_atomic_last() {
    let source = include_str!("../src/compare_redis.rs");
    let start = source.split_once("fn start_redis_container(").unwrap().1;
    let guard = start.find("PendingRedisContainer::new").unwrap();
    let run = start.find("let container_run = execute_checked").unwrap();
    assert!(guard < run);
    assert!(source.contains("fs::hard_link(&temporary, path)"));
    assert!(source.contains("refusing to overwrite stale W8 evidence"));
    assert!(source.contains("no {} artifact was produced"));
}

#[test]
fn canary_same_box_comparison_accepts_a_mismatched_host_or_unpinned_redis() {
    let red = w8_boundary_canary_red().unwrap_err();
    assert!(red.contains(W8_CANARY_MARKER));
    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W8") {
        panic!("{red}");
    }
}
