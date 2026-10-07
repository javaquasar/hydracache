use get_owner_scheduled_controls_074::target::Target;
use get_owner_scheduled_controls_074::{
    native::{Dataset, NativeControl, Operation, Surface},
    scheduled::{run, Config},
};
use std::sync::Arc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn independent_hc1_hc2_get_put_use_real_transports_and_same_binary_oracle() {
    let mut digest = None;
    for surface in [
        Surface::DirectClientSurface,
        Surface::Hc1Http,
        Surface::Hc2GrpcMtls,
    ] {
        for slots in [1, 8] {
            for operation in [Operation::Get, Operation::Put] {
                let control = Arc::new(
                    NativeControl::start(surface, slots, Dataset::new(4, 1024).unwrap(), operation)
                        .await
                        .unwrap(),
                );
                let expected = control.dataset_digest();
                let preload = control.preload().await.unwrap();
                assert_eq!(preload.operations, 4);
                assert_eq!(preload.state_digest, expected);
                assert_eq!(control.reset().await.unwrap(), expected);
                if let Some(digest) = &digest {
                    assert_eq!(digest, &expected);
                } else {
                    digest = Some(expected.clone());
                }
                let result = run(
                    Arc::clone(&control),
                    &Config {
                        operations: 8,
                        offered_rate_per_second: 100,
                        concurrency: slots,
                        maximum_queued: 8,
                        operation_timeout_ns: 2_000_000_000,
                        drain_timeout_ns: 2_000_000_000,
                        slo_ns: 2_000_000_000,
                        highest_trackable_ns: 5_000_000_000,
                    },
                )
                .await
                .unwrap();
                // Semantic fixture only: no real-clock percentile/goodput assertions.
                assert_eq!(result.successes, 8);
                assert_eq!(result.target_completed, 8);
                assert!(!result.promotable);
                assert_eq!(control.verify().await.unwrap(), expected);
                assert_eq!(control.retained_entries(), 4);
                assert_eq!(control.retained_value_bytes(), 4096);
                Arc::try_unwrap(control)
                    .unwrap_or_else(|_| panic!("control still owned"))
                    .shutdown()
                    .await
                    .unwrap();
            }
        }
    }
}

#[test]
fn dataset_and_slot_bounds_are_explicit() {
    assert!(Dataset::new(0, 1).is_err());
    assert!(Dataset::new(17, 1).is_err());
    assert!(Dataset::new(1, 0).is_err());
    assert!(Dataset::new(1, 1_048_577).is_err());
    assert_ne!(
        Dataset::new(1, 256).unwrap().digest(),
        Dataset::new(2, 256).unwrap().digest()
    );
}

#[tokio::test]
async fn large_binary_payload_crosses_real_native_frames_without_truncation() {
    for surface in [Surface::Hc1Http, Surface::Hc2GrpcMtls] {
        let control = NativeControl::start(
            surface,
            1,
            Dataset::new(1, 1_048_576).unwrap(),
            Operation::Get,
        )
        .await
        .unwrap();
        assert_eq!(control.retained_value_bytes(), 1_048_576);
        assert_eq!(control.verify().await.unwrap(), control.dataset_digest());
        control.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn invalid_slots_fail_before_starting_any_listener() {
    assert!(NativeControl::start(
        Surface::Hc1Http,
        2,
        Dataset::new(1, 1).unwrap(),
        Operation::Get
    )
    .await
    .is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn native_batches_keep_one_offer_and_separate_dispatch_semantics() {
    let mut shared_digest = None;
    for surface in [
        Surface::DirectClientSurface,
        Surface::Hc1Http,
        Surface::Hc2GrpcMtls,
    ] {
        for slots in [1, 8] {
            for batch_size in [1, 8, 32, 128] {
                for operation in [
                    Operation::BatchGet { batch_size },
                    Operation::BatchPut { batch_size },
                ] {
                    let control = Arc::new(
                        NativeControl::start(
                            surface,
                            slots,
                            Dataset::new(4, 256).unwrap(),
                            operation,
                        )
                        .await
                        .unwrap(),
                    );
                    let digest = control.dataset_digest();
                    if let Some(expected) = &shared_digest {
                        assert_eq!(expected, &digest);
                    } else {
                        shared_digest = Some(digest.clone());
                    }
                    let before = control.dispatch_attempts();
                    let result = run(
                        Arc::clone(&control),
                        &Config {
                            operations: (2 * slots) as u64,
                            offered_rate_per_second: 10_000,
                            concurrency: slots,
                            maximum_queued: slots,
                            operation_timeout_ns: 5_000_000_000,
                            drain_timeout_ns: 5_000_000_000,
                            slo_ns: 5_000_000_000,
                            highest_trackable_ns: 5_000_000_000,
                        },
                    )
                    .await
                    .unwrap();
                    assert_eq!(result.offered, (2 * slots) as u64);
                    assert_eq!(result.successes, result.offered);
                    assert_eq!(result.samples.len(), result.offered as usize);
                    assert_eq!(result.scheduled_response_latency.samples, result.offered);
                    assert_eq!(result.service_response_latency.samples, result.offered);
                    for sample in &result.samples {
                        assert_eq!(sample.scheduled_ns, sample.sequence * 100_000);
                    }
                    assert!(!result.promotable);
                    assert!(!result.product_performance_claim);
                    let per_command = if matches!(surface, Surface::Hc2GrpcMtls) {
                        batch_size
                    } else {
                        1
                    };
                    assert_eq!(
                        control.dispatch_attempts() - before,
                        result.offered * per_command as u64
                    );
                    assert_eq!(control.verify().await.unwrap(), digest);
                    Arc::try_unwrap(control)
                        .unwrap_or_else(|_| panic!("batch control still owned"))
                        .shutdown()
                        .await
                        .unwrap();
                }
            }
        }
    }
}

#[tokio::test]
async fn unsupported_native_batch_size_and_payload_fail_before_setup() {
    for surface in [
        Surface::DirectClientSurface,
        Surface::Hc1Http,
        Surface::Hc2GrpcMtls,
    ] {
        for batch_size in [0, 2, 256, usize::MAX] {
            assert!(NativeControl::start(
                surface,
                1,
                Dataset::new(1, 256).unwrap(),
                Operation::BatchGet { batch_size }
            )
            .await
            .is_err());
        }
        for operation in [
            Operation::BatchGet { batch_size: 8 },
            Operation::BatchPut { batch_size: 8 },
        ] {
            assert!(NativeControl::start(
                surface,
                1,
                Dataset::new(1, 1_048_576).unwrap(),
                operation
            )
            .await
            .is_err());
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn high_concurrency_native_clients_start_together_and_release_resources() {
    for surface in [Surface::Hc1Http, Surface::Hc2GrpcMtls] {
        for slots in [32, 128] {
            for operation in [Operation::Get, Operation::Put] {
                let control = Arc::new(
                    NativeControl::start(surface, slots, Dataset::new(4, 1024).unwrap(), operation)
                        .await
                        .unwrap(),
                );
                assert_eq!(control.client_slots(), slots);
                if let Some(active) = control.hc2_active_connections() {
                    assert_eq!(active, slots as u64);
                }
                let barrier = Arc::new(tokio::sync::Barrier::new(slots));
                let mut tasks = tokio::task::JoinSet::new();
                for sequence in 0..slots {
                    let control = Arc::clone(&control);
                    let barrier = Arc::clone(&barrier);
                    tasks.spawn(async move {
                        barrier.wait().await;
                        control
                            .execute(get_owner_scheduled_controls_074::target::TargetRequest {
                                sequence: sequence as u64,
                            })
                            .await
                    });
                }
                while let Some(result) = tasks.join_next().await {
                    assert_eq!(
                        result.unwrap(),
                        get_owner_scheduled_controls_074::target::TargetOutcome::Success
                    );
                }
                control.verify().await.unwrap();
                Arc::try_unwrap(control)
                    .unwrap_or_else(|_| panic!("native clients still owned"))
                    .shutdown()
                    .await
                    .unwrap();
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn native_batch_concurrency_boundaries_release_all_owners() {
    for surface in [
        Surface::DirectClientSurface,
        Surface::Hc1Http,
        Surface::Hc2GrpcMtls,
    ] {
        for slots in [32, 128] {
            for operation in [
                Operation::BatchGet { batch_size: 8 },
                Operation::BatchPut { batch_size: 8 },
            ] {
                let control = Arc::new(
                    NativeControl::start(surface, slots, Dataset::new(4, 256).unwrap(), operation)
                        .await
                        .unwrap(),
                );
                if let Some(active) = control.hc2_active_connections() {
                    assert_eq!(active, slots as u64);
                }
                let before = control.dispatch_attempts();
                let barrier = Arc::new(tokio::sync::Barrier::new(slots));
                let mut tasks = tokio::task::JoinSet::new();
                for sequence in 0..slots {
                    let control = Arc::clone(&control);
                    let barrier = Arc::clone(&barrier);
                    tasks.spawn(async move {
                        barrier.wait().await;
                        control
                            .execute(get_owner_scheduled_controls_074::target::TargetRequest {
                                sequence: sequence as u64,
                            })
                            .await
                    });
                }
                while let Some(result) = tasks.join_next().await {
                    assert_eq!(
                        result.unwrap(),
                        get_owner_scheduled_controls_074::target::TargetOutcome::Success
                    );
                }
                let per_command = if matches!(surface, Surface::Hc2GrpcMtls) {
                    8
                } else {
                    1
                };
                assert_eq!(
                    control.dispatch_attempts() - before,
                    (slots * per_command) as u64
                );
                control.verify().await.unwrap();
                Arc::try_unwrap(control)
                    .unwrap_or_else(|_| panic!("batch clients still owned"))
                    .shutdown()
                    .await
                    .unwrap();
            }
        }
    }
}
