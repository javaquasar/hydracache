use get_owner_scheduled_controls_074::{
    native::Dataset,
    resp::{validate_wire, RespControl},
    scheduled::Config,
};
use std::sync::Arc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_resp2_pipeline_matches_each_original_offer_not_batch_average() {
    for depth in [1, 10, 50] {
        let control = Arc::new(
            RespControl::start(Dataset::new(4, 4096).unwrap(), depth)
                .await
                .unwrap(),
        );
        let digest = control.verify().await.unwrap();
        let mut observation = control
            .run(&Config {
                operations: 64,
                offered_rate_per_second: 10000,
                concurrency: 128,
                maximum_queued: 128,
                operation_timeout_ns: 5_000_000_000,
                drain_timeout_ns: 5_000_000_000,
                slo_ns: 5_000_000_000,
                highest_trackable_ns: 10_000_000_000,
            })
            .await
            .unwrap();
        assert_eq!(observation.operations.successes, 64);
        assert_eq!(observation.wire_samples.len(), 64);
        let mut sequence = observation
            .wire_samples
            .iter()
            .map(|s| s.sequence)
            .collect::<Vec<_>>();
        sequence.sort_unstable();
        assert_eq!(sequence, (0..64).collect::<Vec<_>>());
        for sample in &observation.wire_samples {
            assert_eq!(sample.scheduled_ns, sample.sequence * 100_000);
            assert!(sample.byte_oracle_verified);
            assert_eq!(sample.frame_kind, Some("bulk"));
            assert!(sample.response_complete_ns >= sample.write_completed_ns);
            assert_eq!(
                sample.scheduled_frame_latency_ns,
                Some(sample.response_complete_ns.unwrap() - sample.scheduled_ns)
            );
            let operation = &observation.operations.samples[sample.sequence as usize];
            assert!(sample.response_complete_ns.unwrap() <= operation.terminal_ns);
        }
        assert_eq!(control.verify().await.unwrap(), digest);
        let original = observation.wire_samples[0].scheduled_ns;
        observation.wire_samples[0].scheduled_ns += 1;
        assert!(validate_wire(&observation).is_err());
        observation.wire_samples[0].scheduled_ns = original;
        let sequence = observation.wire_samples[1].sequence;
        observation.wire_samples[1].sequence = observation.wire_samples[0].sequence;
        assert!(validate_wire(&observation).is_err());
        observation.wire_samples[1].sequence = sequence;
        let value = observation.wire_samples[0].response_complete_ns;
        observation.wire_samples[0].response_complete_ns = None;
        assert!(validate_wire(&observation).is_err());
        observation.wire_samples[0].response_complete_ns = value;
        assert!(control.run(&observation.operations.config).await.is_err());
        Arc::try_unwrap(control)
            .unwrap_or_else(|_| panic!("response control still owned"))
            .shutdown()
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn large_binary_resp2_frame_has_one_operation_timestamp() {
    let control = Arc::new(
        RespControl::start(Dataset::new(1, 1_048_576).unwrap(), 1)
            .await
            .unwrap(),
    );
    let result = control
        .run(&Config {
            operations: 1,
            offered_rate_per_second: 1000,
            concurrency: 1,
            maximum_queued: 0,
            operation_timeout_ns: 5_000_000_000,
            drain_timeout_ns: 5_000_000_000,
            slo_ns: 5_000_000_000,
            highest_trackable_ns: 10_000_000_000,
        })
        .await
        .unwrap();
    assert_eq!(result.operations.successes, 1);
    assert_eq!(result.wire_samples.len(), 1);
    assert!(result.wire_samples[0].byte_oracle_verified);
    Arc::try_unwrap(control)
        .unwrap_or_else(|_| panic!("control still owned"))
        .shutdown()
        .await
        .unwrap();
}
