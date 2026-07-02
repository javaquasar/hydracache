use std::sync::atomic::{AtomicU64, Ordering};

use hydracache_core::CacheStats;

#[derive(Debug, Default)]
pub(crate) struct StatsCounters {
    pub(crate) hits: AtomicU64,
    pub(crate) misses: AtomicU64,
    pub(crate) loads: AtomicU64,
    pub(crate) single_flight_joins: AtomicU64,
    pub(crate) stale_load_discards: AtomicU64,
    pub(crate) load_breaker_open_total: AtomicU64,
    pub(crate) load_breaker_half_open_total: AtomicU64,
    pub(crate) load_breaker_recovered_total: AtomicU64,
    pub(crate) load_breaker_rejected_total: AtomicU64,
    pub(crate) invalidations: AtomicU64,
    pub(crate) evictions: AtomicU64,
    pub(crate) oversize_rejections: AtomicU64,
    pub(crate) events_published: AtomicU64,
    pub(crate) event_subscriber_lagged: AtomicU64,
    pub(crate) distributed_invalidations_published: AtomicU64,
    pub(crate) distributed_invalidations_received: AtomicU64,
    pub(crate) distributed_invalidations_applied: AtomicU64,
    pub(crate) distributed_invalidation_lagged: AtomicU64,
    pub(crate) distributed_invalidation_decode_errors: AtomicU64,
    pub(crate) distributed_invalidation_publish_failures: AtomicU64,
    pub(crate) distributed_invalidation_receiver_closed: AtomicU64,
    pub(crate) cluster_owner_load_success: AtomicU64,
    pub(crate) cluster_owner_load_errors: AtomicU64,
    pub(crate) cluster_remote_fetch_success: AtomicU64,
    pub(crate) cluster_remote_fetch_errors: AtomicU64,
    pub(crate) cluster_hot_cache_hits: AtomicU64,
    pub(crate) cluster_peer_fetch_auth_failures: AtomicU64,
    pub(crate) cluster_wire_version_rejections: AtomicU64,
    pub(crate) cluster_stale_generation_rejected: AtomicU64,
    pub(crate) cluster_gossip_tombstone_age_ms: AtomicU64,
    pub(crate) cluster_gossip_reset_count: AtomicU64,
    pub(crate) cluster_barrier_timeouts: AtomicU64,
    pub(crate) cluster_near_cache_conservative_invalidations: AtomicU64,
    pub(crate) cluster_lifecycle_stop_count: AtomicU64,
    pub(crate) cluster_lifecycle_restart_count: AtomicU64,
    pub(crate) cluster_replication_success_total: AtomicU64,
    pub(crate) cluster_replication_failure_total: AtomicU64,
    pub(crate) cluster_bytes_replicated_total: AtomicU64,
    pub(crate) cluster_replication_backpressure_total: AtomicU64,
    pub(crate) cluster_replication_oversized_rejected_total: AtomicU64,
    pub(crate) cluster_replication_decrypt_failure_total: AtomicU64,
    pub(crate) cluster_under_replicated_keys: AtomicU64,
    pub(crate) cluster_failover_total: AtomicU64,
    pub(crate) cluster_repair_task_total: AtomicU64,
    pub(crate) cluster_repair_failure_total: AtomicU64,
    pub(crate) cluster_rebalance_plan_total: AtomicU64,
    pub(crate) cluster_rebalance_task_ack_total: AtomicU64,
    pub(crate) cluster_topology_fence_rejected_total: AtomicU64,
    pub(crate) cluster_tombstone_repair_debt: AtomicU64,
    pub(crate) consistency_wait_successes: AtomicU64,
    pub(crate) consistency_wait_timeouts: AtomicU64,
    pub(crate) consistency_degraded_reads: AtomicU64,
    pub(crate) consistency_fail_closed: AtomicU64,
}

impl StatsCounters {
    pub(crate) fn snapshot(&self) -> CacheStats {
        CacheStats {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            loads: self.loads.load(Ordering::Relaxed),
            single_flight_joins: self.single_flight_joins.load(Ordering::Relaxed),
            stale_load_discards: self.stale_load_discards.load(Ordering::Relaxed),
            load_breaker_open_total: self.load_breaker_open_total.load(Ordering::Relaxed),
            load_breaker_half_open_total: self.load_breaker_half_open_total.load(Ordering::Relaxed),
            load_breaker_recovered_total: self.load_breaker_recovered_total.load(Ordering::Relaxed),
            load_breaker_rejected_total: self.load_breaker_rejected_total.load(Ordering::Relaxed),
            invalidations: self.invalidations.load(Ordering::Relaxed),
            evictions: self.evictions.load(Ordering::Relaxed),
            oversize_rejections: self.oversize_rejections.load(Ordering::Relaxed),
            events_published: self.events_published.load(Ordering::Relaxed),
            event_subscriber_lagged: self.event_subscriber_lagged.load(Ordering::Relaxed),
            distributed_invalidations_published: self
                .distributed_invalidations_published
                .load(Ordering::Relaxed),
            distributed_invalidations_received: self
                .distributed_invalidations_received
                .load(Ordering::Relaxed),
            distributed_invalidations_applied: self
                .distributed_invalidations_applied
                .load(Ordering::Relaxed),
            distributed_invalidation_lagged: self
                .distributed_invalidation_lagged
                .load(Ordering::Relaxed),
            distributed_invalidation_decode_errors: self
                .distributed_invalidation_decode_errors
                .load(Ordering::Relaxed),
            distributed_invalidation_publish_failures: self
                .distributed_invalidation_publish_failures
                .load(Ordering::Relaxed),
            distributed_invalidation_receiver_closed: self
                .distributed_invalidation_receiver_closed
                .load(Ordering::Relaxed),
            consistency_wait_successes: self.consistency_wait_successes.load(Ordering::Relaxed),
            consistency_wait_timeouts: self.consistency_wait_timeouts.load(Ordering::Relaxed),
            consistency_degraded_reads: self.consistency_degraded_reads.load(Ordering::Relaxed),
            consistency_fail_closed: self.consistency_fail_closed.load(Ordering::Relaxed),
        }
    }
}
