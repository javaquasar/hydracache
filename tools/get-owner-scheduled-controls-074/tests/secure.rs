use get_owner_scheduled_controls_074::{
    native::{Dataset, NativeControl, Operation as NativeOperation},
    resp::{Dialect, Operation, RespControl},
    scheduled::Config,
    security::{validate_transport_material_match, MtlsFixture},
};
use std::sync::Arc;

fn config(operations: u64) -> Config {
    Config {
        operations,
        offered_rate_per_second: 10000,
        concurrency: 128,
        maximum_queued: 128,
        operation_timeout_ns: 5_000_000_000,
        drain_timeout_ns: 5_000_000_000,
        slo_ns: 5_000_000_000,
        highest_trackable_ns: 10_000_000_000,
    }
}

#[tokio::test]
async fn secure_resp_grid_preserves_wire_owners_auth_and_logical_retention() {
    let material = MtlsFixture::new().unwrap();
    for dialect in [Dialect::Resp2, Dialect::Resp3] {
        for connections in [1, 8, 32, 128] {
            for depth in [1, 10, 50] {
                if connections >= 32 && depth != 10 {
                    continue;
                }
                for operation in [
                    Operation::Get,
                    Operation::GetMissing,
                    Operation::Set,
                    Operation::Mget { batch_size: 8 },
                    Operation::Mset { batch_size: 8 },
                    Operation::Exists { batch_size: 8 },
                    Operation::DelMissing { batch_size: 8 },
                ] {
                    let control = Arc::new(
                        RespControl::start_mtls(
                            Dataset::new(4, 256).unwrap(),
                            depth,
                            connections,
                            operation,
                            dialect,
                            &material,
                        )
                        .await
                        .unwrap(),
                    );
                    let result = control
                        .run(&config((2 * connections) as u64))
                        .await
                        .unwrap();
                    assert_eq!(result.operations.successes, (2 * connections) as u64);
                    assert_eq!(result.wire_samples.len(), 2 * connections);
                    assert_eq!(result.authenticated_connections, connections);
                    assert_eq!(
                        result.transport_security.as_ref(),
                        Some(&material.receipt())
                    );
                    assert!(!result.product_performance_claim);
                    assert_eq!(control.retained_entries(), 4);
                    assert_eq!(control.retained_value_bytes(), 1024);
                    Arc::try_unwrap(control)
                        .unwrap_or_else(|_| panic!("wire owner leaked"))
                        .shutdown()
                        .await
                        .unwrap();
                }
            }
        }
    }
}

#[tokio::test]
async fn shared_secure_material_and_dataset_guard_does_not_waive_authorization_differences() {
    let material = MtlsFixture::new().unwrap();
    let foreign = MtlsFixture::new().unwrap();
    let native = NativeControl::start_hc2_mtls(
        8,
        Dataset::new(4, 256).unwrap(),
        NativeOperation::Get,
        &material,
    )
    .await
    .unwrap();
    let resp = RespControl::start_mtls(
        Dataset::new(4, 256).unwrap(),
        10,
        8,
        Operation::Get,
        Dialect::Resp3,
        &material,
    )
    .await
    .unwrap();
    validate_transport_material_match(
        native.transport_security().unwrap(),
        resp.transport_security().unwrap(),
        &native.verify().await.unwrap(),
        &resp.verify().await.unwrap(),
    )
    .unwrap();
    assert!(validate_transport_material_match(
        &foreign.receipt(),
        resp.transport_security().unwrap(),
        &native.dataset_digest(),
        &resp.verify().await.unwrap()
    )
    .is_err());
    assert!(validate_transport_material_match(
        &material.receipt(),
        &material.receipt(),
        "wrong-dataset",
        &native.dataset_digest()
    )
    .is_err());
    assert!(!material.receipt().cross_surface_numeric_comparison_allowed);
    native.shutdown().await.unwrap();
    resp.shutdown().await.unwrap();
}

#[tokio::test]
async fn secure_large_payload_and_multikey_boundaries_preserve_delete_refill_oracles() {
    let material = MtlsFixture::new().unwrap();
    for dialect in [Dialect::Resp2, Dialect::Resp3] {
        for (payload, operation) in [
            (1_048_576, Operation::Get),
            (1_048_576, Operation::Set),
            (256, Operation::Mget { batch_size: 1 }),
            (256, Operation::Mget { batch_size: 32 }),
            (256, Operation::Mget { batch_size: 128 }),
            (256, Operation::Mset { batch_size: 128 }),
        ] {
            let control = Arc::new(
                RespControl::start_mtls(
                    Dataset::new(4, payload).unwrap(),
                    10,
                    8,
                    operation,
                    dialect,
                    &material,
                )
                .await
                .unwrap(),
            );
            assert_eq!(
                control.run(&config(16)).await.unwrap().operations.successes,
                16
            );
            control.delete_dataset().await.unwrap();
            assert_eq!(control.retained_entries(), 0);
            assert_eq!(control.retained_value_bytes(), 0);
            control.refill_dataset().await.unwrap();
            control.verify().await.unwrap();
            Arc::try_unwrap(control)
                .unwrap_or_else(|_| panic!("control owned"))
                .shutdown()
                .await
                .unwrap();
        }
    }
}

#[tokio::test]
async fn native_delete_refill_uses_each_public_transport_and_leaves_no_values() {
    use get_owner_scheduled_controls_074::native::Surface;
    for surface in [
        Surface::DirectClientSurface,
        Surface::Hc1Http,
        Surface::Hc2GrpcMtls,
    ] {
        let control = NativeControl::start(
            surface,
            8,
            Dataset::new(16, 256).unwrap(),
            NativeOperation::Get,
        )
        .await
        .unwrap();
        control.delete_dataset().await.unwrap();
        assert_eq!(control.retained_entries(), 0);
        assert_eq!(control.retained_value_bytes(), 0);
        control.refill_dataset().await.unwrap();
        control.verify().await.unwrap();
        control.shutdown().await.unwrap();
    }
}
