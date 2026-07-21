use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use hydracache_cluster_testkit::client_surface_conformance::{
    self as conformance, ClientSurfaceBackend, ClientSurfaceBackendFactory,
    ClientSurfaceConformanceConfig, ConformanceResult,
};

#[derive(Clone, Default)]
struct RecordingFailureFactory {
    configs: Arc<Mutex<Vec<ClientSurfaceConformanceConfig>>>,
}

impl RecordingFailureFactory {
    fn configs(&self) -> Vec<ClientSurfaceConformanceConfig> {
        self.configs.lock().unwrap().clone()
    }
}

#[async_trait]
impl ClientSurfaceBackendFactory for RecordingFailureFactory {
    async fn create(
        &self,
        config: ClientSurfaceConformanceConfig,
    ) -> ConformanceResult<Arc<dyn ClientSurfaceBackend>> {
        self.configs.lock().unwrap().push(config);
        Err(anyhow::anyhow!("intentional backend creation failure"))
    }
}

#[tokio::test]
async fn every_conformance_oracle_executes_and_preserves_boundary_configuration() {
    let factory = RecordingFailureFactory::default();

    assert!(
        conformance::assert_conditional_put_if_absent_is_atomic_under_n_concurrent_acquirers(
            &factory
        )
        .await
        .is_err()
    );
    assert!(
        conformance::assert_conditional_put_treats_expired_key_as_absent(&factory)
            .await
            .is_err()
    );
    assert!(
        conformance::assert_compare_value_invalidate_is_token_safe_and_returns_applied_count(
            &factory
        )
        .await
        .is_err()
    );
    assert!(
        conformance::assert_compare_value_expire_add_to_remaining_and_replace_if_expiring_and_persistent_guard(
            &factory,
        )
        .await
        .is_err()
    );
    assert!(
        conformance::assert_batch_put_is_all_or_nothing_under_prevalidation_failure(&factory)
            .await
            .is_err()
    );
    assert!(
        conformance::assert_ttl_states_missing_persistent_expiring_round_trip(&factory)
            .await
            .is_err()
    );
    assert!(
        conformance::assert_local_ttl_and_lock_contracts_survive_backward_wall_clock_step(&factory)
            .await
            .is_err()
    );
    assert!(
        conformance::assert_expired_key_absent_for_get_and_batch_get(&factory)
            .await
            .is_err()
    );
    assert!(
        conformance::assert_enforces_value_bytes_batch_and_tenant_quota_limits(&factory)
            .await
            .is_err()
    );
    assert!(
        conformance::assert_rejected_conditionals_and_batches_do_not_reserve_quota(&factory)
            .await
            .is_err()
    );
    assert!(
        conformance::assert_delete_and_expiry_release_tenant_quota(&factory)
            .await
            .is_err()
    );
    assert!(
        conformance::assert_duplicate_batch_keys_account_last_write_only(&factory)
            .await
            .is_err()
    );
    assert!(
        conformance::assert_tenant_binding_and_same_namespace_keys_are_isolated(&factory)
            .await
            .is_err()
    );
    assert!(
        conformance::assert_batch_entry_and_byte_limits_reject_at_boundary_plus_one_without_mutation(
            &factory,
        )
        .await
        .is_err()
    );

    let configs = factory.configs();
    assert_eq!(configs.len(), 14, "every oracle must construct its backend");
    assert_eq!(configs[4].limits.max_value_bytes, 4);
    assert_eq!(configs[9].limits.max_value_bytes, 6);
    assert_eq!(configs[11].limits.max_value_bytes, 4);
}
