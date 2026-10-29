use get_owner_scheduled_controls_074::{
    embedded::EmbeddedControl,
    native::{Dataset, Operation},
    scheduled::{run, Config},
    target::{Target, TargetOutcome, TargetRequest},
};
use std::sync::Arc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn embedded_scheduled_get_put_keep_same_corpus_and_original_offers() {
    let digest = Dataset::new(16, 4096).unwrap().digest();
    for slots in [1, 8, 32, 128] {
        for operation in [Operation::Get, Operation::Put] {
            let control = Arc::new(
                EmbeddedControl::start(slots, Dataset::new(16, 4096).unwrap(), operation)
                    .await
                    .unwrap(),
            );
            assert_eq!(control.dataset_digest(), digest);
            assert_eq!(control.client_slots(), slots);
            let observed = run(
                Arc::clone(&control),
                &Config {
                    operations: (2 * slots) as u64,
                    offered_rate_per_second: 10000,
                    concurrency: slots,
                    maximum_queued: 128,
                    operation_timeout_ns: 5_000_000_000,
                    drain_timeout_ns: 5_000_000_000,
                    slo_ns: 5_000_000_000,
                    highest_trackable_ns: 10_000_000_000,
                },
            )
            .await
            .unwrap();
            assert_eq!(observed.successes, (2 * slots) as u64);
            assert_eq!(observed.samples.len(), 2 * slots);
            assert!(observed.owned_tasks_drained);
            assert_eq!(control.verify().await.unwrap(), digest);
            Arc::try_unwrap(control)
                .unwrap_or_else(|_| panic!("embedded owner leaked"))
                .shutdown()
                .await
                .unwrap();
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn embedded_high_concurrency_and_public_delete_refill_release_oracles() {
    for slots in [32, 128] {
        for operation in [Operation::Get, Operation::Put] {
            let control = Arc::new(
                EmbeddedControl::start(slots, Dataset::new(16, 256).unwrap(), operation)
                    .await
                    .unwrap(),
            );
            let barrier = Arc::new(tokio::sync::Barrier::new(slots));
            let mut owners = tokio::task::JoinSet::new();
            for sequence in 0..slots {
                let c = Arc::clone(&control);
                let gate = Arc::clone(&barrier);
                owners.spawn(async move {
                    gate.wait().await;
                    c.execute(TargetRequest {
                        sequence: sequence as u64,
                    })
                    .await
                });
            }
            while let Some(result) = owners.join_next().await {
                assert_eq!(result.unwrap(), TargetOutcome::Success);
            }
            control.delete_dataset().await.unwrap();
            control.refill_dataset().await.unwrap();
            control.verify().await.unwrap();
            Arc::try_unwrap(control)
                .unwrap_or_else(|_| panic!("embedded owner leaked"))
                .shutdown()
                .await
                .unwrap();
        }
    }
}

#[tokio::test]
async fn embedded_large_binary_values_use_the_public_encoded_api() {
    let control = EmbeddedControl::start(1, Dataset::new(16, 1_048_576).unwrap(), Operation::Get)
        .await
        .unwrap();
    assert_eq!(
        control.execute(TargetRequest { sequence: 15 }).await,
        TargetOutcome::Success
    );
    control.verify().await.unwrap();
    control.delete_dataset().await.unwrap();
    control.refill_dataset().await.unwrap();
    control.shutdown().await.unwrap();
}

#[tokio::test]
async fn embedded_unsupported_slots_and_batches_fail_before_cache_setup() {
    for slots in [0, 2, 256] {
        assert!(
            EmbeddedControl::start(slots, Dataset::new(1, 256).unwrap(), Operation::Get)
                .await
                .is_err()
        );
    }
    assert!(EmbeddedControl::start(
        1,
        Dataset::new(1, 256).unwrap(),
        Operation::BatchPut { batch_size: 8 }
    )
    .await
    .is_err());
}
