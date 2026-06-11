use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use async_trait::async_trait;
use hydracache_core::{
    CacheError, CacheEventKind, CacheEventOptions, CacheEventOrigin, CacheOptions, Result,
};
use tokio::sync::watch;

use crate::tests::common::{user, User};
use crate::{
    CacheInvalidation, CacheInvalidationBus, CacheInvalidationMessage, CacheInvalidationReceive,
    CacheInvalidationReceiver, ClusterCandidate, ClusterControlPlane, ClusterDiagnostics,
    ClusterEpoch, ClusterGeneration, ClusterMember, ClusterMembershipEvent, ClusterNodeId,
    ClusterRole, HydraCache, InMemoryFramedInvalidationBus, InMemoryInvalidationBus,
};

#[derive(Debug, Clone)]
struct ClosingBus;

#[async_trait]
impl CacheInvalidationBus for ClosingBus {
    async fn publish(&self, _message: CacheInvalidationMessage) -> Result<()> {
        Ok(())
    }

    fn subscribe(&self) -> Box<dyn CacheInvalidationReceiver> {
        Box::new(ClosingReceiver)
    }
}

struct ClosingReceiver;

#[async_trait]
impl CacheInvalidationReceiver for ClosingReceiver {
    async fn recv(&mut self) -> CacheInvalidationReceive {
        CacheInvalidationReceive::Closed
    }
}

#[derive(Debug, Clone)]
struct FailingPublishBus;

#[async_trait]
impl CacheInvalidationBus for FailingPublishBus {
    async fn publish(&self, _message: CacheInvalidationMessage) -> Result<()> {
        Err(CacheError::Backend("publish failed".to_owned()))
    }

    fn subscribe(&self) -> Box<dyn CacheInvalidationReceiver> {
        Box::new(ClosingReceiver)
    }
}

#[derive(Debug, Clone)]
struct LagThenCloseBus;

#[async_trait]
impl CacheInvalidationBus for LagThenCloseBus {
    async fn publish(&self, _message: CacheInvalidationMessage) -> Result<()> {
        Ok(())
    }

    fn subscribe(&self) -> Box<dyn CacheInvalidationReceiver> {
        Box::new(LagThenCloseReceiver { polled: false })
    }
}

struct LagThenCloseReceiver {
    polled: bool,
}

#[async_trait]
impl CacheInvalidationReceiver for LagThenCloseReceiver {
    async fn recv(&mut self) -> CacheInvalidationReceive {
        if self.polled {
            CacheInvalidationReceive::Closed
        } else {
            self.polled = true;
            CacheInvalidationReceive::Lagged(3)
        }
    }
}

#[derive(Debug)]
struct FramedGenerationControlPlane {
    bus: Arc<InMemoryFramedInvalidationBus>,
    members: Mutex<BTreeMap<ClusterNodeId, ClusterMember>>,
    clients: Mutex<BTreeMap<ClusterNodeId, ClusterMember>>,
}

impl FramedGenerationControlPlane {
    fn new(bus: Arc<InMemoryFramedInvalidationBus>) -> Self {
        Self {
            bus,
            members: Mutex::new(BTreeMap::new()),
            clients: Mutex::new(BTreeMap::new()),
        }
    }

    fn upsert_candidate(
        table: &Mutex<BTreeMap<ClusterNodeId, ClusterMember>>,
        candidate: ClusterCandidate,
    ) -> ClusterMember {
        let member = ClusterMember {
            node_id: candidate.node_id,
            generation: candidate.generation,
            role: candidate.role,
            epoch: ClusterEpoch::new(1),
            endpoints: candidate.endpoints,
            metadata: candidate.metadata,
        };
        table
            .lock()
            .expect("test control plane table poisoned")
            .insert(member.node_id.clone(), member.clone());
        member
    }

    fn find_generation(&self, node_id: &ClusterNodeId) -> Option<ClusterGeneration> {
        if let Some(generation) = self
            .members
            .lock()
            .expect("test member table poisoned")
            .get(node_id)
            .map(|member| member.generation)
        {
            return Some(generation);
        }
        self.clients
            .lock()
            .expect("test client table poisoned")
            .get(node_id)
            .map(|member| member.generation)
    }
}

#[async_trait]
impl ClusterControlPlane for FramedGenerationControlPlane {
    fn name(&self) -> String {
        "orders".to_owned()
    }

    fn invalidation_bus(&self) -> Arc<dyn CacheInvalidationBus> {
        self.bus.clone()
    }

    async fn join_member(&self, candidate: ClusterCandidate) -> Result<ClusterMember> {
        Ok(Self::upsert_candidate(&self.members, candidate))
    }

    async fn join_client(&self, candidate: ClusterCandidate) -> Result<ClusterMember> {
        Ok(Self::upsert_candidate(&self.clients, candidate))
    }

    async fn validate_generation(
        &self,
        node_id: &ClusterNodeId,
        generation: ClusterGeneration,
    ) -> Result<()> {
        match self.find_generation(node_id) {
            Some(current) if current == generation => Ok(()),
            Some(current) => Err(CacheError::Backend(format!(
                "stale cluster generation for {node_id}: attempted {}, current {}",
                generation.value(),
                current.value()
            ))),
            None => Err(CacheError::Backend(format!(
                "unknown cluster node {node_id}"
            ))),
        }
    }

    async fn leave(
        &self,
        _node_id: &ClusterNodeId,
        _generation: ClusterGeneration,
    ) -> Result<Option<ClusterMembershipEvent>> {
        Ok(None)
    }

    fn diagnostics_for(
        &self,
        role: ClusterRole,
        node_id: ClusterNodeId,
        generation: ClusterGeneration,
        bootstrap: Vec<String>,
    ) -> ClusterDiagnostics {
        ClusterDiagnostics {
            cluster_name: self.name(),
            role,
            node_id,
            generation,
            epoch: ClusterEpoch::new(1),
            member_count: self
                .members
                .lock()
                .expect("test member table poisoned")
                .len(),
            client_count: self
                .clients
                .lock()
                .expect("test client table poisoned")
                .len(),
            bootstrap,
            connected: true,
            invalidation_subscribers: self.bus.receiver_count(),
            membership_subscribers: 0,
            ownership_resolutions: 0,
            ownership_no_owner: 0,
        }
    }
}

async fn wait_until<F>(mut condition: F)
where
    F: FnMut() -> bool,
{
    tokio::time::timeout(Duration::from_millis(500), async {
        loop {
            if condition() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("condition should become true before timeout");
}

async fn wait_until_absent(cache: &HydraCache, key: &str) {
    tokio::time::timeout(Duration::from_millis(500), async {
        loop {
            if !cache.contains_key(key).await {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("key should become absent before timeout");
}

#[tokio::test]
async fn shared_bus_propagates_tag_invalidations_between_cache_instances() {
    let bus = Arc::new(InMemoryInvalidationBus::new(16));
    let source = HydraCache::local()
        .shared_invalidation_bus(bus.clone())
        .invalidation_node_id("source")
        .build();
    let target = HydraCache::local()
        .shared_invalidation_bus(bus)
        .invalidation_node_id("target")
        .build();
    let mut target_events =
        target.subscribe(CacheEventOptions::mutations().origin(CacheEventOrigin::DistributedBus));

    source
        .put("user:42", user(42), CacheOptions::new().tag("users"))
        .await
        .unwrap();
    target
        .put("user:42", user(42), CacheOptions::new().tag("users"))
        .await
        .unwrap();

    assert_eq!(source.invalidate_tag("users").await.unwrap(), 1);
    wait_until_absent(&target, "user:42").await;

    let event = tokio::time::timeout(Duration::from_millis(500), target_events.recv())
        .await
        .expect("remote event should arrive")
        .expect("subscription should stay open");
    assert_eq!(event.kind(), CacheEventKind::TagInvalidated);
    assert_eq!(event.origin(), CacheEventOrigin::DistributedBus);
    assert_eq!(event.tag(), Some("users"));
    assert_eq!(event.affected_keys(), Some(1));

    assert_eq!(source.stats().distributed_invalidations_published, 1);
    assert_eq!(source.stats().distributed_invalidations_received, 0);
    assert_eq!(target.stats().distributed_invalidations_received, 1);
    assert_eq!(target.stats().distributed_invalidations_applied, 1);
}

#[tokio::test]
async fn framed_bus_propagates_tag_invalidations_between_independent_runtimes() {
    let bus = Arc::new(InMemoryFramedInvalidationBus::for_cluster("orders", 16));
    let source = HydraCache::local()
        .shared_invalidation_bus(bus.clone())
        .invalidation_node_id("source")
        .build();
    let target = HydraCache::local()
        .shared_invalidation_bus(bus)
        .invalidation_node_id("target")
        .build();

    source
        .put("user:42", user(42), CacheOptions::new().tag("users"))
        .await
        .unwrap();
    target
        .put("user:42", user(42), CacheOptions::new().tag("users"))
        .await
        .unwrap();

    assert_eq!(source.invalidate_tag("users").await.unwrap(), 1);
    wait_until_absent(&target, "user:42").await;

    assert_eq!(source.stats().distributed_invalidations_published, 1);
    assert_eq!(target.stats().distributed_invalidations_received, 1);
    assert_eq!(target.stats().distributed_invalidations_applied, 1);
    assert_eq!(target.stats().distributed_invalidation_decode_errors, 0);
}

#[tokio::test]
async fn framed_bus_stale_cluster_generation_is_rejected_before_apply() {
    let bus = Arc::new(InMemoryFramedInvalidationBus::for_cluster("orders", 16));
    let control_plane = Arc::new(FramedGenerationControlPlane::new(bus.clone()));
    HydraCache::member()
        .control_plane(control_plane.clone())
        .node_id("member-a")
        .generation(ClusterGeneration::new(2))
        .start()
        .await
        .unwrap();
    let target = HydraCache::client()
        .control_plane(control_plane)
        .node_id("client-a")
        .connect()
        .await
        .unwrap();

    target
        .put("user:42", user(42), CacheOptions::new().tag("users"))
        .await
        .unwrap();

    bus.publish(
        CacheInvalidationMessage::new("member-a", CacheInvalidation::tag("users"))
            .with_source_generation(ClusterGeneration::new(1)),
    )
    .await
    .unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    assert!(target.contains_key("user:42").await);
    assert_eq!(target.stats().distributed_invalidations_applied, 0);

    bus.publish(
        CacheInvalidationMessage::new("member-a", CacheInvalidation::tag("users"))
            .with_source_generation(ClusterGeneration::new(2)),
    )
    .await
    .unwrap();

    wait_until_absent(&target, "user:42").await;
    assert_eq!(target.stats().distributed_invalidations_applied, 1);
}

#[tokio::test]
async fn framed_bus_decode_errors_are_reported_without_applying_invalidation() {
    let bus = Arc::new(InMemoryFramedInvalidationBus::new(16));
    let target = HydraCache::local()
        .shared_invalidation_bus(bus.clone())
        .invalidation_node_id("target")
        .build();

    target
        .put("user:42", user(42), CacheOptions::new().tag("users"))
        .await
        .unwrap();

    bus.publish_encoded_frame(bytes::Bytes::from_static(b"corrupted-frame"))
        .unwrap();

    wait_until(|| target.stats().distributed_invalidation_decode_errors == 1).await;

    assert!(target.contains_key("user:42").await);
    assert_eq!(target.stats().distributed_invalidations_applied, 0);
    assert!(target.stats().has_distributed_invalidation_bus_issues());
}

#[tokio::test]
async fn framed_bus_close_is_reported_in_cache_stats_and_publish_fails() {
    let bus = Arc::new(InMemoryFramedInvalidationBus::new(16));
    let cache = HydraCache::local()
        .shared_invalidation_bus(bus.clone())
        .invalidation_node_id("source")
        .build();

    bus.close();

    wait_until(|| cache.stats().distributed_invalidation_receiver_closed == 1).await;
    let error = cache.invalidate_tag("users").await.unwrap_err();

    assert!(error
        .to_string()
        .contains("framed invalidation transport is closed"));
    assert_eq!(cache.stats().distributed_invalidation_publish_failures, 1);
}

#[tokio::test]
async fn framed_bus_reconnect_does_not_replay_already_applied_invalidation() {
    let bus = Arc::new(InMemoryFramedInvalidationBus::new(16));
    let source = HydraCache::local()
        .shared_invalidation_bus(bus.clone())
        .invalidation_node_id("source")
        .build();
    let target = HydraCache::local()
        .shared_invalidation_bus(bus.clone())
        .invalidation_node_id("target")
        .build();

    target
        .put("user:42", user(42), CacheOptions::new().tag("users"))
        .await
        .unwrap();
    source.invalidate_tag("users").await.unwrap();
    wait_until_absent(&target, "user:42").await;

    drop(target);

    let reconnected_target = HydraCache::local()
        .shared_invalidation_bus(bus)
        .invalidation_node_id("target")
        .build();
    reconnected_target
        .put("user:42", user(42), CacheOptions::new().tag("users"))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    assert!(reconnected_target.contains_key("user:42").await);
    assert_eq!(
        reconnected_target.stats().distributed_invalidations_applied,
        0
    );
}

#[tokio::test]
async fn key_invalidation_is_published_even_when_source_does_not_hold_key() {
    let bus = Arc::new(InMemoryInvalidationBus::new(16));
    let source = HydraCache::local()
        .shared_invalidation_bus(bus.clone())
        .invalidation_node_id("source")
        .build();
    let target = HydraCache::local()
        .shared_invalidation_bus(bus)
        .invalidation_node_id("target")
        .build();
    let mut target_events = target.subscribe(
        CacheEventOptions::mutations()
            .include_kind(CacheEventKind::KeyInvalidated)
            .origin(CacheEventOrigin::DistributedBus),
    );

    target
        .put("user:7", user(7), CacheOptions::new())
        .await
        .unwrap();

    assert!(!source.invalidate_key("user:7").await.unwrap());
    wait_until_absent(&target, "user:7").await;

    let event = tokio::time::timeout(Duration::from_millis(500), target_events.recv())
        .await
        .expect("remote key invalidation should arrive")
        .expect("subscription should stay open");
    assert_eq!(event.key(), Some("user:7"));
    assert_eq!(source.stats().distributed_invalidations_published, 1);
    assert_eq!(target.stats().distributed_invalidations_applied, 1);
}

#[tokio::test]
async fn shared_bus_propagates_flush_without_echoing_back_to_source() {
    let bus = Arc::new(InMemoryInvalidationBus::new(16));
    let source = HydraCache::local()
        .shared_invalidation_bus(bus.clone())
        .invalidation_node_id("source")
        .build();
    let target = HydraCache::local()
        .shared_invalidation_bus(bus)
        .invalidation_node_id("target")
        .build();

    source
        .put("source-only", user(1), CacheOptions::new())
        .await
        .unwrap();
    target
        .put("target-only", user(2), CacheOptions::new())
        .await
        .unwrap();

    source.flush().await.unwrap();
    wait_until_absent(&target, "target-only").await;
    wait_until(|| source.stats().distributed_invalidations_published == 1).await;

    assert!(!source.contains_key("source-only").await);
    assert_eq!(source.stats().distributed_invalidations_received, 0);
    assert_eq!(target.stats().distributed_invalidations_received, 1);
    assert_eq!(target.stats().distributed_invalidations_applied, 1);
}

#[tokio::test]
async fn typed_cache_invalidation_uses_physical_keys_on_the_shared_bus() {
    let bus = Arc::new(InMemoryInvalidationBus::new(16));
    let source = HydraCache::local()
        .shared_invalidation_bus(bus.clone())
        .invalidation_node_id("source")
        .build();
    let target = HydraCache::local()
        .shared_invalidation_bus(bus)
        .invalidation_node_id("target")
        .build();
    let source_users = source.typed::<User>("users");
    let target_users = target.typed::<User>("users");

    target_users
        .put("42", user(42), CacheOptions::new())
        .await
        .unwrap();

    assert!(!source_users.invalidate_key("42").await.unwrap());
    wait_until_absent(&target, "users:42").await;

    assert_eq!(target_users.get("42").await.unwrap(), None);
    assert_eq!(source.stats().distributed_invalidations_published, 1);
    assert_eq!(target.stats().distributed_invalidations_applied, 1);
}

#[tokio::test]
async fn owned_bus_builder_generates_observable_node_id_and_handles_closed_receiver() {
    let cache = HydraCache::local().invalidation_bus(ClosingBus).build();

    assert!(cache.invalidation_node_id().starts_with("hydracache-node-"));

    wait_until(|| cache.stats().distributed_invalidation_receiver_closed == 1).await;

    assert_eq!(cache.stats().distributed_invalidations_received, 0);
    assert_eq!(cache.stats().distributed_invalidations_applied, 0);
    assert_eq!(cache.stats().distributed_invalidation_receiver_closed, 1);
    assert!(cache.stats().has_distributed_invalidation_bus_issues());
}

#[tokio::test]
async fn listener_spawn_without_bus_is_a_noop() {
    let cache = HydraCache::local().build();
    let (_shutdown, receiver) = watch::channel(false);

    cache.spawn_invalidation_listener(receiver);

    assert!(cache.invalidation_node_id().starts_with("hydracache-node-"));
}

#[tokio::test]
async fn manual_listener_can_shutdown_without_processing_messages() {
    let bus = Arc::new(InMemoryInvalidationBus::new(16));
    let cache = HydraCache::local()
        .shared_invalidation_bus(bus.clone())
        .invalidation_node_id("manual")
        .build();
    let (shutdown, receiver) = watch::channel(false);

    cache.spawn_invalidation_listener(receiver);
    tokio::task::yield_now().await;
    shutdown.send(true).unwrap();
    tokio::task::yield_now().await;

    assert_eq!(cache.stats().distributed_invalidations_received, 0);
}

#[tokio::test]
async fn listener_exits_when_cache_inner_is_dropped() {
    let bus = Arc::new(InMemoryInvalidationBus::new(16));
    let cache = HydraCache::local()
        .shared_invalidation_bus(bus.clone())
        .invalidation_node_id("transient")
        .build();
    let (_shutdown, receiver) = watch::channel(false);

    cache.spawn_invalidation_listener(receiver);
    drop(cache);

    bus.publish(CacheInvalidationMessage::new(
        "remote",
        CacheInvalidation::flush(),
    ))
    .await
    .unwrap();
    tokio::task::yield_now().await;
}

#[tokio::test]
async fn self_originated_bus_messages_are_ignored_by_listener() {
    let cache = HydraCache::local()
        .invalidation_bus(InMemoryInvalidationBus::new(16))
        .invalidation_node_id("self")
        .build();

    cache
        .put("self-key", user(9), CacheOptions::new())
        .await
        .unwrap();
    assert!(cache.remove("self-key").await.unwrap());
    tokio::time::sleep(Duration::from_millis(20)).await;

    assert_eq!(cache.stats().distributed_invalidations_published, 1);
    assert_eq!(cache.stats().distributed_invalidations_received, 0);
    assert_eq!(cache.stats().distributed_invalidations_applied, 0);
}

#[tokio::test]
async fn publish_failures_are_reported_in_stats_and_returned_to_caller() {
    let cache = HydraCache::local()
        .invalidation_bus(FailingPublishBus)
        .invalidation_node_id("failing")
        .build();

    let error = cache.invalidate_tag("users").await.unwrap_err();

    assert!(error.to_string().contains("publish failed"));
    assert_eq!(cache.stats().distributed_invalidations_published, 0);
    assert_eq!(cache.stats().distributed_invalidation_publish_failures, 1);
    assert!(cache.stats().has_distributed_invalidation_bus_issues());
}

#[tokio::test]
async fn receiver_lag_and_close_are_reported_in_stats() {
    let cache = HydraCache::local()
        .invalidation_bus(LagThenCloseBus)
        .invalidation_node_id("lagging")
        .build();

    wait_until(|| {
        let stats = cache.stats();
        stats.distributed_invalidation_lagged == 3
            && stats.distributed_invalidation_receiver_closed == 1
    })
    .await;

    let stats = cache.stats();
    assert_eq!(stats.distributed_invalidation_lagged, 3);
    assert_eq!(stats.distributed_invalidation_receiver_closed, 1);
    assert!(stats.has_distributed_invalidation_bus_issues());
}
