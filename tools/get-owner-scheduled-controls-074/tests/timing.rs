//! Semantic fixtures, not finite A/A-A/B numerical attempts.
use get_owner_scheduled_controls_074::{
    native::Dataset,
    timing::{run, Input, TimedObservation},
};

#[test]
fn rental_pilot_draft_inputs_validate_without_starting_a_fixture() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/testing/performance/0.74/rental-pilot-draft");
    let mut digests = std::collections::BTreeSet::new();
    for (surface, expected) in [
        (
            "embedded",
            "06d84110ec8fe56f986ef5cbef75b3e9df559449fdf32cb2c42680b683363a3c",
        ),
        (
            "direct",
            "ee3dde1ca3322617ac08b6609bca5d151e9d49a8595a1743b3b3c321b22a1bc4",
        ),
        (
            "resp2",
            "e2ead8f4d1e61f1a1ab4246bb308ce7b00a2532bed52dc2c5ea1effb236fc11d",
        ),
        (
            "resp3",
            "642dd2671bdfb92c7d0d9400b10b9bd37dbbdd5b40ec2d0a50e8c4c1ea66c938",
        ),
    ] {
        let data = std::fs::read(root.join(format!("{surface}.json"))).unwrap();
        let config: Input = serde_json::from_slice(&data).unwrap();
        config.validate().unwrap();
        assert_eq!(
            config.dataset_sha256,
            Dataset::new(16, 4096).unwrap().digest()
        );
        assert_eq!(config.schedule.operations, 10_000);
        assert_eq!(config.schedule.offered_rate_per_second, 5_000);
        assert_eq!(config.slots, 8);
        assert_eq!(config.warmup_calls, 64);
        assert_eq!(config.minimum_usable_cpu_ns, 1_000_000_000);
        assert_eq!(config.minimum_usable_measurement_wall_ns, 1_000_000_000);
        assert_eq!(config.workload_sha256(), expected);
        assert!(digests.insert(config.workload_sha256()));
    }
    assert_eq!(digests.len(), 4); // Surface identity is intentionally not pooled away.
}

fn input(surface: &str, operation: &str) -> Input {
    serde_json::from_value(serde_json::json!({
        "schema_version": 1,
        "profile_id": "unprofiled-timing-controls-074-v1",
        "surface": surface, "operation": operation,
        "seed": 740074, "keyspace": 16, "payload_bytes": 256,
        "dataset_sha256": Dataset::new(16, 256).unwrap().digest(),
        "slots": 8, "pipeline_depth": if surface.starts_with("resp") { 10 } else { 0 },
        "warmup_calls": 16,
        "minimum_usable_cpu_ns": 1_000_000_000_u64,
        "minimum_usable_measurement_wall_ns": 1_000_000_000_u64,
        "schedule": {
            "operations": 16, "offered_rate_per_second": 10000,
            "concurrency": 8, "maximum_queued": 128,
            "operation_timeout_ns": 5_000_000_000_u64,
            "drain_timeout_ns": 5_000_000_000_u64,
            "slo_ns": 5_000_000_000_u64,
            "highest_trackable_ns": 10_000_000_000_u64
        }
    }))
    .unwrap()
}

#[test]
fn timing_input_refuses_unknown_missing_drifted_or_out_of_budget_fields() {
    let base = input("resp2", "get");
    base.validate().unwrap();
    let mut extra = serde_json::to_value(&base).unwrap();
    extra["retry"] = serde_json::json!(true);
    assert!(serde_json::from_value::<Input>(extra).is_err());
    let mut missing = serde_json::to_value(&base).unwrap();
    missing.as_object_mut().unwrap().remove("warmup_calls");
    assert!(serde_json::from_value::<Input>(missing).is_err());
    let mut nested = serde_json::to_value(&base).unwrap();
    nested["schedule"]["refresh_deadline"] = serde_json::json!(true);
    assert!(serde_json::from_value::<Input>(nested).is_err());
    for field in [
        "seed",
        "slots",
        "minimum_usable_cpu_ns",
        "minimum_usable_measurement_wall_ns",
    ] {
        let mut value = serde_json::to_value(&base).unwrap();
        value[field] = serde_json::json!(0);
        assert!(
            serde_json::from_value::<Input>(value)
                .unwrap()
                .validate()
                .is_err(),
            "{field}"
        );
    }
    let mut value = base.clone();
    value.warmup_calls = 65;
    assert!(value.validate().is_err());
    value = base.clone();
    value.dataset_sha256 = "0".repeat(64);
    assert!(value.validate().is_err());
    value = input("embedded", "get");
    value.pipeline_depth = 10;
    assert!(value.validate().is_err());
    value = base;
    value.schedule.operations = 10001;
    assert!(value.validate().is_err());
    assert!(
        serde_json::from_value::<Input>(serde_json::json!({"surface": "redis-fallback"})).is_err()
    );
}

#[test]
fn workload_digest_is_canonical_and_binds_the_entire_input() {
    let base = input("resp2", "get");
    let digest = base.workload_sha256();
    assert_eq!(digest.len(), 64);
    let text = serde_json::to_string_pretty(&base).unwrap();
    assert_eq!(
        serde_json::from_str::<Input>(&text)
            .unwrap()
            .workload_sha256(),
        digest
    );
    let mut changed = base.clone();
    changed.schedule.slo_ns -= 1;
    assert_ne!(changed.workload_sha256(), digest);
    changed = base.clone();
    changed.warmup_calls -= 1;
    assert_ne!(changed.workload_sha256(), digest);
    assert_ne!(input("resp3", "get").workload_sha256(), digest);
    assert_ne!(input("resp2", "put").workload_sha256(), digest);
}

#[tokio::test]
async fn every_surface_get_put_warmup_keeps_original_offers_and_closes_owners() {
    for surface in [
        "embedded",
        "direct",
        "hc1",
        "hc2-mtls",
        "resp2",
        "resp3",
        "resp2-mtls",
        "resp3-mtls",
    ] {
        for operation in ["get", "put"] {
            let config = input(surface, operation);
            let digest = config.workload_sha256();
            let report = run(config).await;
            assert!(
                report.error.is_none(),
                "{surface}/{operation}: {:?}",
                report.error
            );
            assert!(report.shutdown_verified);
            assert!(report.final_dataset_verified);
            assert_eq!(report.workload_sha256, digest);
            assert_eq!(report.warmup_completed, 16);
            assert!(!report.admission_allowed);
            assert!(!report.cross_surface_numeric_comparison_allowed);
            let observed = report.observation.as_ref().unwrap();
            let operations = observed.operations();
            assert_eq!(operations.offered, 16);
            assert_eq!(operations.successes, 16);
            assert_eq!(operations.samples.first().unwrap().sequence, 0);
            assert!(operations.owned_tasks_drained);
            if let TimedObservation::Resp(wire) = observed {
                assert_eq!(wire.wire_samples.len(), 16);
                assert!(wire
                    .wire_samples
                    .iter()
                    .all(|r| r.sequence < 16 && r.byte_oracle_verified));
            }
            let cpu = report.cpu.as_ref().unwrap();
            assert!(cpu.wall_elapsed_ns > 0);
            assert!(cpu.clock.unit_resolution_ns > 0);
            // Assert the quality formula, never a real-clock speed threshold.
            assert_eq!(
                cpu.usable_for_ratio,
                cpu.process_cpu_ns >= 1_000_000_000
                    && cpu.wall_elapsed_ns >= 1_000_000_000
                    && operations.scheduled_response_latency.overflow_count == 0
                    && operations.service_response_latency.overflow_count == 0
            );
        }
    }
}

#[tokio::test]
async fn invalid_input_is_reported_before_any_owner_or_warmup() {
    let mut config = input("direct", "get");
    config.seed += 1;
    let report = run(config).await;
    assert!(report.error.is_some());
    assert!(!report.shutdown_verified);
    assert_eq!(report.warmup_completed, 0);
    assert!(report.observation.is_none());
    assert!(report.cpu.is_none());
}

#[tokio::test]
async fn resp_warmup_is_bounded_single_use_and_never_reserves_original_offer_ids() {
    use get_owner_scheduled_controls_074::resp::{Operation, RespControl};
    use std::sync::Arc;
    let config = input("resp2", "get");
    let control = Arc::new(
        RespControl::start(Dataset::new(16, 256).unwrap(), 1)
            .await
            .unwrap(),
    );
    assert!(control.warmup(65).await.is_err());
    control.warmup(64).await.unwrap();
    assert!(control.warmup(0).await.is_err());
    let observed = control.run(&config.schedule).await.unwrap();
    assert_eq!(observed.operation, Operation::Get);
    assert_eq!(observed.operations.successes, 16);
    assert_eq!(observed.wire_samples.len(), 16);
    assert!(observed.wire_samples.iter().all(|r| r.sequence < 16));
    assert!(control.warmup(1).await.is_err());
    control.verify().await.unwrap();
    Arc::try_unwrap(control)
        .unwrap_or_else(|_| panic!("warmup owner leaked"))
        .shutdown()
        .await
        .unwrap();
}
