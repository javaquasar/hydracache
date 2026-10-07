use get_owner_scheduled_controls_074::{
    native::Dataset,
    resp::{validate_wire, Operation, RespControl},
    scheduled::Config,
};
use std::sync::Arc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn multiple_resp_connections_keep_local_fifo_and_fixed_set_oracles() {
    for connections in [1, 8, 32, 128] {
        for depth in [1, 10, 50] {
            for operation in [Operation::Get, Operation::Set] {
                let control = Arc::new(
                    RespControl::start_connections(
                        Dataset::new(4, 256).unwrap(),
                        depth,
                        connections,
                        operation,
                    )
                    .await
                    .unwrap(),
                );
                assert_eq!(control.physical_connections(), connections);
                let digest = control.verify().await.unwrap();
                let mut observation = control
                    .run(&Config {
                        operations: (connections * 2) as u64,
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
                assert_eq!(observation.operations.successes, (connections * 2) as u64);
                assert_eq!(observation.physical_connections, connections);
                assert_eq!(observation.operation, operation);
                for connection in 0..connections {
                    let samples = observation
                        .wire_samples
                        .iter()
                        .filter(|s| s.connection_id == connection)
                        .collect::<Vec<_>>();
                    assert_eq!(samples.len(), 2);
                    assert!(samples[0].wire_ordinal < samples[1].wire_ordinal);
                    for sample in samples {
                        assert_eq!(sample.sequence as usize % connections, connection);
                        assert_eq!(
                            sample.frame_kind,
                            Some(match operation {
                                Operation::Get => "bulk",
                                Operation::Set => "simple",
                            })
                        );
                        assert!(sample.byte_oracle_verified);
                    }
                }
                assert_eq!(control.verify().await.unwrap(), digest);
                let connection = observation.wire_samples[0].connection_id;
                observation.wire_samples[0].connection_id = connections;
                assert!(validate_wire(&observation).is_err());
                observation.wire_samples[0].connection_id = connection;
                if connections > 1 {
                    observation.wire_samples[0].connection_id = (connection + 1) % connections;
                    assert!(validate_wire(&observation).is_err());
                    observation.wire_samples[0].connection_id = connection;
                }
                let ordinal = observation.wire_samples[1].wire_ordinal;
                observation.wire_samples[1].wire_ordinal = observation.wire_samples[0].wire_ordinal;
                assert!(validate_wire(&observation).is_err());
                observation.wire_samples[1].wire_ordinal = ordinal;
                validate_wire(&observation).unwrap();
                Arc::try_unwrap(control)
                    .unwrap_or_else(|_| panic!("control still owned"))
                    .shutdown()
                    .await
                    .unwrap();
            }
        }
    }
}

#[tokio::test]
async fn unsupported_resp_connection_counts_fail_before_transport_setup() {
    for connections in [0, 2, 129, usize::MAX] {
        assert!(RespControl::start_connections(
            Dataset::new(1, 16).unwrap(),
            1,
            connections,
            Operation::Get,
        )
        .await
        .is_err());
    }
}

#[tokio::test]
async fn large_fixed_set_checks_acknowledgement_and_final_bytes() {
    let control = Arc::new(
        RespControl::start_connections(Dataset::new(1, 1_048_576).unwrap(), 1, 1, Operation::Set)
            .await
            .unwrap(),
    );
    let digest = control.verify().await.unwrap();
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
    assert_eq!(result.wire_samples[0].frame_kind, Some("simple"));
    assert!(result.wire_samples[0].byte_oracle_verified);
    assert_eq!(control.verify().await.unwrap(), digest);
    Arc::try_unwrap(control)
        .unwrap_or_else(|_| panic!("control still owned"))
        .shutdown()
        .await
        .unwrap();
}

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
