use std::sync::Arc;
use std::time::Duration;

use hydracache_core::{CacheEventOptions, CacheEventOrigin, CacheOptions, PostcardCodec};
use tokio::time::{sleep, timeout};

use crate::tests::common::{user, User};
use crate::{
    CacheError, CacheEventSubscriber, CacheInvalidation, CacheInvalidationBus,
    CacheInvalidationMessage, ChitchatStyleDiscovery, ClusterCandidate, ClusterControlPlane,
    ClusterDiagnostics, ClusterDiscovery, ClusterDiscoveryEvent, ClusterEpoch, ClusterGeneration,
    ClusterLifecycleDiagnostics, ClusterMembershipEvent, ClusterNodeId, ClusterRole, HydraCache,
    InMemoryCluster, InMemoryClusterDiscovery, InMemoryInvalidationBus, RaftMetadataCommand,
    RaftStyleMetadataControlPlane,
};

async fn wait_until_absent(cache: &HydraCache, key: &str) {
    timeout(Duration::from_secs(2), async {
        loop {
            if cache.get::<User>(key).await.unwrap().is_none() {
                return;
            }

            sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("cache entry should be removed by cluster invalidation");
}

async fn wait_for_distributed_mutation(subscriber: &mut CacheEventSubscriber) {
    let event = timeout(Duration::from_secs(2), subscriber.recv())
        .await
        .expect("distributed mutation event should arrive")
        .expect("distributed mutation event stream should remain open");

    assert_eq!(event.origin(), CacheEventOrigin::DistributedBus);
}

#[tokio::test]
async fn local_cache_has_no_cluster_diagnostics() {
    let cache = HydraCache::local().build();

    assert!(cache.cluster_diagnostics().is_none());
    assert!(cache.cluster_discovery_diagnostics().is_none());
    assert!(cache.leave_cluster().await.unwrap().is_none());
}

#[tokio::test]
async fn member_and_client_builders_connect_to_shared_cluster() {
    let cluster = Arc::new(InMemoryCluster::new("orders-prod"));
    let discovery = Arc::new(InMemoryClusterDiscovery::new());

    let member = HydraCache::member()
        .cluster("orders-prod")
        .shared_cluster(cluster.clone())
        .shared_discovery(discovery.clone())
        .node_id("member-a")
        .generation(ClusterGeneration::new(1))
        .bind("127.0.0.1:7000")
        .diagnostics_endpoint("http://127.0.0.1:3000")
        .start()
        .await
        .unwrap();

    let client = HydraCache::client()
        .cluster("orders-prod")
        .shared_cluster(cluster)
        .shared_discovery(discovery.clone())
        .node_id("client-a")
        .generation(ClusterGeneration::new(4))
        .bootstrap("127.0.0.1:7000")
        .near_cache_capacity(128)
        .connect()
        .await
        .unwrap();

    let member_diag = member.cluster_diagnostics().unwrap();
    assert_eq!(member_diag.cluster_name, "orders-prod");
    assert_eq!(member_diag.role, ClusterRole::Member);
    assert_eq!(member_diag.member_count, 1);
    assert_eq!(member_diag.client_count, 1);
    assert_eq!(member_diag.epoch.value(), 1);
    assert!(member_diag.is_member_role());
    assert_eq!(member_diag.participant_count(), 2);
    assert_eq!(member_diag.bootstrap_count(), 0);
    assert!(member_diag.has_members());
    assert!(member_diag.has_clients());
    assert!(!member_diag.has_bootstrap());
    assert!(member_diag.has_multiple_participants());
    assert!(member_diag.is_operational());
    assert_eq!(member_diag.lifecycle.component, "cluster-runtime:member");
    assert!(member_diag.lifecycle.is_running());
    assert_eq!(member_diag.lifecycle.start_count, 1);

    let client_diag = client.cluster_diagnostics().unwrap();
    assert_eq!(client_diag.role, ClusterRole::Client);
    assert_eq!(client_diag.bootstrap, vec!["127.0.0.1:7000".to_owned()]);
    assert!(client_diag.is_client_role());
    assert_eq!(client_diag.bootstrap_count(), 1);
    assert!(client_diag.has_bootstrap());
    assert_eq!(client_diag.lifecycle.component, "cluster-runtime:client");
    assert!(client_diag.lifecycle.is_running());
    assert_eq!(client.invalidation_node_id(), "client-a");
    assert!(client_diag.invalidation_subscribers >= 2);

    let discovered = discovery.candidates();
    assert_eq!(discovered.len(), 2);
    assert!(discovered
        .iter()
        .any(|candidate| candidate.node_id.as_str() == "member-a"));
    assert!(discovered
        .iter()
        .any(|candidate| candidate.node_id.as_str() == "client-a"));
    assert_eq!(
        discovery
            .events()
            .iter()
            .filter(|event| matches!(event, ClusterDiscoveryEvent::CandidateSeen(_)))
            .count(),
        2
    );

    let member_discovery = member.cluster_discovery_diagnostics().unwrap();
    assert_eq!(member_discovery.local_node_id.as_str(), "member-a");
    assert_eq!(member_discovery.candidate_count(), 2);
    assert_eq!(member_discovery.event_count(), 2);
    assert!(member_discovery.has_candidates());
    assert!(member_discovery.has_events());
}

#[tokio::test]
async fn member_and_client_builders_apply_cache_tuning_and_endpoint_metadata() {
    let cluster = Arc::new(InMemoryCluster::new("orders-tuned"));

    let member = HydraCache::member()
        .shared_cluster(cluster.clone())
        .node_id("member-tuned")
        .generation(ClusterGeneration::new(3))
        .bind("127.0.0.1:7100")
        .diagnostics_endpoint("http://127.0.0.1:3100")
        .cache_capacity(2)
        .max_entry_bytes(128)
        .default_ttl(Duration::from_millis(50))
        .enable_access_events(true)
        .event_buffer_capacity(4)
        .codec(PostcardCodec)
        .start()
        .await
        .unwrap();

    let client = HydraCache::client()
        .shared_cluster(cluster.clone())
        .node_id("client-tuned")
        .generation(ClusterGeneration::new(5))
        .bootstrap("127.0.0.1:7100")
        .control_endpoint("127.0.0.1:7200")
        .diagnostics_endpoint("http://127.0.0.1:3200")
        .near_cache_capacity(2)
        .max_entry_bytes(128)
        .default_ttl(Duration::from_millis(50))
        .enable_access_events(true)
        .event_buffer_capacity(4)
        .codec(PostcardCodec)
        .connect()
        .await
        .unwrap();

    member
        .put("member-key", user(1), CacheOptions::new())
        .await
        .unwrap();
    client
        .put("client-key", user(2), CacheOptions::new())
        .await
        .unwrap();

    let members = cluster.members();
    let member_record = members
        .iter()
        .find(|member| member.node_id.as_str() == "member-tuned")
        .unwrap();
    assert_eq!(
        member_record.endpoints.control.as_deref(),
        Some("127.0.0.1:7100")
    );
    assert_eq!(
        member_record.endpoints.invalidation.as_deref(),
        Some("127.0.0.1:7100")
    );
    assert_eq!(
        member_record.endpoints.diagnostics.as_deref(),
        Some("http://127.0.0.1:3100")
    );

    let clients = cluster.clients();
    let client_record = clients
        .iter()
        .find(|client| client.node_id.as_str() == "client-tuned")
        .unwrap();
    assert_eq!(
        client_record.endpoints.control.as_deref(),
        Some("127.0.0.1:7200")
    );
    assert_eq!(
        client_record.endpoints.diagnostics.as_deref(),
        Some("http://127.0.0.1:3200")
    );
    assert_eq!(
        client.cluster_diagnostics().unwrap().bootstrap,
        vec!["127.0.0.1:7100".to_owned()]
    );
}

#[tokio::test]
async fn cache_runtime_can_subscribe_to_cluster_membership_events() {
    let local = HydraCache::local().build();
    assert!(local.subscribe_cluster_membership().is_none());

    let cluster = Arc::new(InMemoryCluster::new("orders"));
    let member = HydraCache::member()
        .shared_cluster(cluster.clone())
        .node_id("member-a")
        .start()
        .await
        .unwrap();
    let mut events = member
        .subscribe_cluster_membership()
        .expect("member runtime exposes membership events");

    let client = HydraCache::client()
        .shared_cluster(cluster)
        .node_id("client-a")
        .connect()
        .await
        .unwrap();

    assert!(matches!(
        events.recv().await.unwrap(),
        ClusterMembershipEvent::ClientConnected(joined)
            if joined.node_id.as_str() == "client-a"
    ));
    assert_eq!(
        client
            .cluster_diagnostics()
            .expect("cluster diagnostics")
            .membership_subscribers,
        1
    );
}

#[tokio::test]
async fn member_invalidation_reaches_client_near_cache() {
    let cluster = Arc::new(InMemoryCluster::new("orders"));

    let member = HydraCache::member()
        .shared_cluster(cluster.clone())
        .node_id("member-a")
        .start()
        .await
        .unwrap();
    let client = HydraCache::client()
        .shared_cluster(cluster)
        .node_id("client-a")
        .connect()
        .await
        .unwrap();

    client
        .put("user:42", user(42), CacheOptions::new().tag("user:42"))
        .await
        .unwrap();
    assert!(client.get::<User>("user:42").await.unwrap().is_some());

    let mut client_events =
        client.subscribe(CacheEventOptions::mutations().origin(CacheEventOrigin::DistributedBus));
    member.invalidate_tag("user:42").await.unwrap();

    let event = timeout(Duration::from_secs(2), client_events.recv())
        .await
        .expect("client should observe distributed invalidation")
        .expect("client event stream should remain open");

    assert_eq!(event.origin(), CacheEventOrigin::DistributedBus);
    wait_until_absent(&client, "user:42").await;
    assert_eq!(member.stats().distributed_invalidations_published, 1);
    assert_eq!(client.stats().distributed_invalidations_applied, 1);
}

#[tokio::test]
async fn client_invalidation_reaches_member_cache() {
    let cluster = Arc::new(InMemoryCluster::new("orders"));

    let member = HydraCache::member()
        .shared_cluster(cluster.clone())
        .node_id("member-a")
        .start()
        .await
        .unwrap();
    let client = HydraCache::client()
        .shared_cluster(cluster)
        .node_id("client-a")
        .connect()
        .await
        .unwrap();

    member
        .put("user:7", user(7), CacheOptions::new().tag("user:7"))
        .await
        .unwrap();
    assert!(member.get::<User>("user:7").await.unwrap().is_some());

    client.invalidate_key("user:7").await.unwrap();

    wait_until_absent(&member, "user:7").await;
    assert_eq!(client.stats().distributed_invalidations_published, 1);
    assert_eq!(member.stats().distributed_invalidations_applied, 1);
}

#[tokio::test]
async fn multi_node_cluster_propagates_invalidations_and_tracks_membership_changes() {
    let cluster = Arc::new(InMemoryCluster::new("orders"));
    let discovery = Arc::new(InMemoryClusterDiscovery::new());

    let member_a = HydraCache::member()
        .cluster("orders")
        .shared_cluster(cluster.clone())
        .shared_discovery(discovery.clone())
        .node_id("member-a")
        .generation(ClusterGeneration::new(1))
        .bind("127.0.0.1:7000")
        .start()
        .await
        .unwrap();
    let member_b = HydraCache::member()
        .cluster("orders")
        .shared_cluster(cluster.clone())
        .shared_discovery(discovery.clone())
        .node_id("member-b")
        .generation(ClusterGeneration::new(1))
        .bind("127.0.0.1:7001")
        .start()
        .await
        .unwrap();
    let client_a = HydraCache::client()
        .cluster("orders")
        .shared_cluster(cluster.clone())
        .shared_discovery(discovery.clone())
        .node_id("client-a")
        .generation(ClusterGeneration::new(1))
        .bootstrap("127.0.0.1:7000")
        .connect()
        .await
        .unwrap();
    let client_b = HydraCache::client()
        .cluster("orders")
        .shared_cluster(cluster.clone())
        .shared_discovery(discovery.clone())
        .node_id("client-b")
        .generation(ClusterGeneration::new(1))
        .bootstrap("127.0.0.1:7001")
        .connect()
        .await
        .unwrap();

    assert_eq!(cluster.members().len(), 2);
    assert_eq!(cluster.clients().len(), 2);
    assert_eq!(discovery.candidates().len(), 4);
    assert_eq!(member_a.cluster_diagnostics().unwrap().epoch.value(), 2);

    for cache in [&member_a, &member_b, &client_a, &client_b] {
        cache
            .put("user:42", user(42), CacheOptions::new().tag("users"))
            .await
            .unwrap();
    }

    let mut member_b_tag_events =
        member_b.subscribe(CacheEventOptions::mutations().origin(CacheEventOrigin::DistributedBus));
    let mut client_a_tag_events =
        client_a.subscribe(CacheEventOptions::mutations().origin(CacheEventOrigin::DistributedBus));
    let mut client_b_tag_events =
        client_b.subscribe(CacheEventOptions::mutations().origin(CacheEventOrigin::DistributedBus));

    assert_eq!(member_a.invalidate_tag("users").await.unwrap(), 1);
    wait_for_distributed_mutation(&mut member_b_tag_events).await;
    wait_for_distributed_mutation(&mut client_a_tag_events).await;
    wait_for_distributed_mutation(&mut client_b_tag_events).await;
    drop((
        member_b_tag_events,
        client_a_tag_events,
        client_b_tag_events,
    ));

    for cache in [&member_a, &member_b, &client_a, &client_b] {
        wait_until_absent(cache, "user:42").await;
    }

    for cache in [&member_a, &member_b, &client_a, &client_b] {
        cache
            .put("user:99", user(99), CacheOptions::new())
            .await
            .unwrap();
    }

    let mut member_a_key_events =
        member_a.subscribe(CacheEventOptions::mutations().origin(CacheEventOrigin::DistributedBus));
    let mut member_b_key_events =
        member_b.subscribe(CacheEventOptions::mutations().origin(CacheEventOrigin::DistributedBus));
    let mut client_a_key_events =
        client_a.subscribe(CacheEventOptions::mutations().origin(CacheEventOrigin::DistributedBus));

    assert!(client_b.invalidate_key("user:99").await.unwrap());
    wait_for_distributed_mutation(&mut member_a_key_events).await;
    wait_for_distributed_mutation(&mut member_b_key_events).await;
    wait_for_distributed_mutation(&mut client_a_key_events).await;

    for cache in [&member_a, &member_b, &client_a, &client_b] {
        wait_until_absent(cache, "user:99").await;
    }

    assert_eq!(member_a.stats().distributed_invalidations_published, 1);
    assert_eq!(client_b.stats().distributed_invalidations_published, 1);
    assert_eq!(member_b.stats().distributed_invalidations_applied, 2);
    assert_eq!(client_a.stats().distributed_invalidations_applied, 2);

    let upgraded_client_b = HydraCache::client()
        .cluster("orders")
        .shared_cluster(cluster.clone())
        .node_id("client-b")
        .generation(ClusterGeneration::new(2))
        .connect()
        .await
        .unwrap();
    assert_eq!(cluster.clients().len(), 2);
    assert_eq!(
        cluster
            .clients()
            .into_iter()
            .find(|client| client.node_id.as_str() == "client-b")
            .unwrap()
            .generation
            .value(),
        2
    );

    let stale_member = HydraCache::member()
        .shared_cluster(cluster.clone())
        .node_id("member-a")
        .generation(ClusterGeneration::new(0))
        .start()
        .await
        .unwrap_err();
    assert!(stale_member
        .to_string()
        .contains("stale cluster generation"));

    let stale_client = HydraCache::client()
        .shared_cluster(cluster.clone())
        .node_id("client-b")
        .generation(ClusterGeneration::new(1))
        .connect()
        .await
        .unwrap_err();
    assert!(stale_client
        .to_string()
        .contains("stale cluster generation"));

    let client_left = client_a.leave_cluster().await.unwrap().unwrap();
    assert!(matches!(
        client_left,
        ClusterMembershipEvent::NodeLeft {
            role: ClusterRole::Client,
            ..
        }
    ));
    let member_left = member_b.leave_cluster().await.unwrap().unwrap();
    assert!(matches!(
        member_left,
        ClusterMembershipEvent::NodeLeft {
            role: ClusterRole::Member,
            ..
        }
    ));

    let diagnostics = upgraded_client_b.cluster_diagnostics().unwrap();
    assert_eq!(diagnostics.member_count, 1);
    assert_eq!(diagnostics.client_count, 1);
    assert_eq!(diagnostics.epoch.value(), 3);
    assert_eq!(cluster.members()[0].node_id.as_str(), "member-a");
    assert_eq!(cluster.clients()[0].node_id.as_str(), "client-b");
}

#[tokio::test]
async fn cluster_rejects_stale_generation_for_same_node() {
    let cluster = Arc::new(InMemoryCluster::new("orders"));

    HydraCache::member()
        .shared_cluster(cluster.clone())
        .node_id("member-a")
        .generation(ClusterGeneration::new(2))
        .start()
        .await
        .unwrap();

    let error = HydraCache::member()
        .shared_cluster(cluster.clone())
        .node_id("member-a")
        .generation(ClusterGeneration::new(1))
        .start()
        .await
        .unwrap_err();

    assert!(error.to_string().contains("stale cluster generation"));
    assert_eq!(cluster.members().len(), 1);
    assert_eq!(cluster.members()[0].generation.value(), 2);
}

#[tokio::test]
async fn stale_runtime_cannot_leave_newer_generation() {
    let cluster = Arc::new(InMemoryCluster::new("orders"));
    let stale = HydraCache::member()
        .shared_cluster(cluster.clone())
        .node_id("member-a")
        .generation(ClusterGeneration::new(1))
        .start()
        .await
        .unwrap();

    let current = HydraCache::member()
        .shared_cluster(cluster.clone())
        .node_id("member-a")
        .generation(ClusterGeneration::new(2))
        .start()
        .await
        .unwrap();

    let error = stale.leave_cluster().await.unwrap_err();

    assert!(error.to_string().contains("stale cluster generation"));
    assert_eq!(cluster.members().len(), 1);
    assert_eq!(cluster.members()[0].generation.value(), 2);
    assert!(stale.cluster_diagnostics().unwrap().lifecycle.is_running());
    assert_eq!(current.cluster_diagnostics().unwrap().member_count, 1);
}

#[tokio::test]
async fn stale_runtime_cannot_publish_cluster_invalidation() {
    let cluster = Arc::new(InMemoryCluster::new("orders"));
    let stale = HydraCache::member()
        .shared_cluster(cluster.clone())
        .node_id("member-a")
        .generation(ClusterGeneration::new(1))
        .start()
        .await
        .unwrap();
    let _current = HydraCache::member()
        .shared_cluster(cluster.clone())
        .node_id("member-a")
        .generation(ClusterGeneration::new(2))
        .start()
        .await
        .unwrap();
    let observer = HydraCache::client()
        .shared_cluster(cluster)
        .node_id("client-a")
        .connect()
        .await
        .unwrap();

    observer
        .put("user:42", user(42), CacheOptions::new().tag("users"))
        .await
        .unwrap();

    let error = stale.invalidate_tag("users").await.unwrap_err();

    assert!(error.to_string().contains("stale cluster generation"));
    assert_eq!(stale.stats().distributed_invalidations_published, 0);
    assert_eq!(stale.stats().distributed_invalidation_publish_failures, 1);
    assert!(observer.get::<User>("user:42").await.unwrap().is_some());
}

#[tokio::test]
async fn stale_bus_message_generation_is_rejected_by_cluster_receivers() {
    let cluster = Arc::new(InMemoryCluster::new("orders"));
    let bus = cluster.invalidation_bus();
    HydraCache::member()
        .shared_cluster(cluster.clone())
        .node_id("member-a")
        .generation(ClusterGeneration::new(2))
        .start()
        .await
        .unwrap();
    let observer = HydraCache::client()
        .shared_cluster(cluster)
        .node_id("client-a")
        .connect()
        .await
        .unwrap();

    observer
        .put("user:42", user(42), CacheOptions::new().tag("users"))
        .await
        .unwrap();

    bus.publish(
        CacheInvalidationMessage::new("member-a", CacheInvalidation::tag("users"))
            .with_source_generation(ClusterGeneration::new(1)),
    )
    .await
    .unwrap();
    sleep(Duration::from_millis(50)).await;

    assert!(observer.get::<User>("user:42").await.unwrap().is_some());
    assert_eq!(observer.stats().distributed_invalidations_applied, 0);
}

#[tokio::test]
async fn client_builder_can_create_isolated_cluster_runtime() {
    let client = HydraCache::client()
        .cluster("isolated")
        .node_id("client-a")
        .bootstrap("127.0.0.1:7000")
        .connect()
        .await
        .unwrap();

    let diagnostics = client.cluster_diagnostics().unwrap();
    assert_eq!(diagnostics.cluster_name, "isolated");
    assert_eq!(diagnostics.role, ClusterRole::Client);
    assert_eq!(diagnostics.member_count, 0);
    assert_eq!(diagnostics.client_count, 1);
    assert_eq!(diagnostics.bootstrap, vec!["127.0.0.1:7000".to_owned()]);
    assert!(client.cluster_discovery_diagnostics().is_none());
}

#[tokio::test]
async fn client_and_member_can_leave_cluster_without_clearing_local_cache() {
    let cluster = Arc::new(InMemoryCluster::new("orders"));
    let member = HydraCache::member()
        .shared_cluster(cluster.clone())
        .node_id("member-a")
        .start()
        .await
        .unwrap();
    let client = HydraCache::client()
        .shared_cluster(cluster.clone())
        .node_id("client-a")
        .connect()
        .await
        .unwrap();

    client
        .put("user:42", user(42), CacheOptions::new().tag("user:42"))
        .await
        .unwrap();
    assert_eq!(cluster.members().len(), 1);
    assert_eq!(cluster.clients().len(), 1);

    let client_left = client.leave_cluster().await.unwrap().unwrap();
    assert!(matches!(
        client_left,
        ClusterMembershipEvent::NodeLeft {
            role: ClusterRole::Client,
            epoch,
            ..
        } if epoch.value() == 1
    ));
    assert_eq!(cluster.clients().len(), 0);
    assert_eq!(cluster.members().len(), 1);
    assert!(client.get::<User>("user:42").await.unwrap().is_some());
    let client_diagnostics = client.cluster_diagnostics().unwrap();
    assert!(client_diagnostics.lifecycle.is_stopped());
    assert_eq!(client_diagnostics.lifecycle.stop_count, 1);
    assert!(!client_diagnostics.is_operational());

    let member_left = member.leave_cluster().await.unwrap().unwrap();
    assert!(matches!(
        member_left,
        ClusterMembershipEvent::NodeLeft {
            role: ClusterRole::Member,
            epoch,
            ..
        } if epoch.value() == 2
    ));
    assert_eq!(cluster.members().len(), 0);
    assert!(member.leave_cluster().await.unwrap().is_none());
    let member_diagnostics = member.cluster_diagnostics().unwrap();
    assert_eq!(member_diagnostics.member_count, 0);
    assert!(member_diagnostics.lifecycle.is_stopped());
    assert_eq!(member_diagnostics.lifecycle.stop_count, 1);
}

#[tokio::test]
async fn builders_accept_control_plane_trait_objects() {
    let control_plane: Arc<dyn ClusterControlPlane> = Arc::new(InMemoryCluster::new("orders"));

    let member = HydraCache::member()
        .control_plane(control_plane.clone())
        .node_id("member-a")
        .start()
        .await
        .unwrap();
    let client = HydraCache::client()
        .control_plane(control_plane)
        .node_id("client-a")
        .connect()
        .await
        .unwrap();

    assert_eq!(member.cluster_diagnostics().unwrap().member_count, 1);
    assert_eq!(client.cluster_diagnostics().unwrap().client_count, 1);
}

#[tokio::test]
async fn raft_style_metadata_control_plane_records_committed_membership_commands() {
    let control_plane = Arc::new(RaftStyleMetadataControlPlane::new("orders").with_term(7));
    let member = HydraCache::member()
        .control_plane(control_plane.clone())
        .node_id("member-a")
        .generation(ClusterGeneration::new(1))
        .start()
        .await
        .unwrap();
    let client = HydraCache::client()
        .control_plane(control_plane.clone())
        .node_id("client-a")
        .generation(ClusterGeneration::new(1))
        .connect()
        .await
        .unwrap();

    let snapshot = control_plane.snapshot();
    assert_eq!(snapshot.term, 7);
    assert_eq!(snapshot.commit_index, 2);
    assert_eq!(snapshot.epoch.value(), 1);
    assert_eq!(snapshot.member_count, 1);
    assert_eq!(snapshot.client_count, 1);
    assert!(matches!(
        snapshot.last_command,
        Some(RaftMetadataCommand::ClientUpsert { ref node_id, .. })
            if node_id.as_str() == "client-a"
    ));
    assert!(matches!(
        &control_plane.commands()[0],
        RaftMetadataCommand::MemberUpsert {
            node_id,
            generation,
            epoch,
        } if node_id.as_str() == "member-a"
            && generation.value() == 1
            && epoch.value() == 1
    ));

    let stale = HydraCache::member()
        .control_plane(control_plane.clone())
        .node_id("member-a")
        .generation(ClusterGeneration::new(0))
        .start()
        .await
        .unwrap_err();
    assert!(stale.to_string().contains("stale cluster generation"));
    assert_eq!(control_plane.snapshot().commit_index, 2);

    let client_left = client.leave_cluster().await.unwrap().unwrap();
    assert!(matches!(
        client_left,
        ClusterMembershipEvent::NodeLeft {
            role: ClusterRole::Client,
            ..
        }
    ));
    let member_left = member.leave_cluster().await.unwrap().unwrap();
    assert!(matches!(
        member_left,
        ClusterMembershipEvent::NodeLeft {
            role: ClusterRole::Member,
            ..
        }
    ));

    let snapshot = control_plane.snapshot();
    assert_eq!(snapshot.commit_index, 4);
    assert_eq!(snapshot.epoch.value(), 2);
    assert_eq!(snapshot.member_count, 0);
    assert_eq!(snapshot.client_count, 0);
    assert!(matches!(
        snapshot.last_command,
        Some(RaftMetadataCommand::NodeLeft {
            role: ClusterRole::Member,
            epoch,
            ..
        }) if epoch.value() == 2
    ));
}

#[tokio::test]
async fn raft_style_metadata_control_plane_exposes_trait_paths_and_noop_leave() {
    let control_plane = RaftStyleMetadataControlPlane::default().with_term(9);
    assert_eq!(control_plane.name(), "hydracache");
    assert_eq!(control_plane.snapshot().term, 9);
    assert!(control_plane.commands().is_empty());
    assert_eq!(
        control_plane
            .leave(&ClusterNodeId::from("missing"), ClusterGeneration::new(1))
            .await
            .unwrap(),
        None
    );

    let _bus_subscriber = control_plane.invalidation_bus().subscribe();
    let diagnostics = control_plane.diagnostics_for(
        ClusterRole::Client,
        ClusterNodeId::from("client-a"),
        ClusterGeneration::new(1),
        vec!["seed-a".to_owned()],
    );
    assert_eq!(diagnostics.cluster_name, "hydracache");
    assert_eq!(diagnostics.role, ClusterRole::Client);
    assert_eq!(diagnostics.bootstrap, vec!["seed-a".to_owned()]);
    assert_eq!(diagnostics.invalidation_subscribers, 1);

    let ownership = control_plane.ownership_diagnostics();
    assert_eq!(ownership.resolver, "rendezvous");
    assert_eq!(ownership.resolutions, 0);
    assert_eq!(ownership.no_owner, 0);
    assert_eq!(ownership.owner_found_ratio(), None);
    let _membership_subscriber = control_plane.subscribe_membership();
}

#[tokio::test]
async fn builders_accept_discovery_trait_objects() {
    let cluster = Arc::new(InMemoryCluster::new("orders"));
    let discovery: Arc<dyn ClusterDiscovery> = Arc::new(InMemoryClusterDiscovery::new());

    HydraCache::member()
        .shared_cluster(cluster.clone())
        .discovery(discovery.clone())
        .node_id("member-a")
        .start()
        .await
        .unwrap();
    HydraCache::client()
        .shared_cluster(cluster)
        .discovery(discovery.clone())
        .node_id("client-a")
        .connect()
        .await
        .unwrap();

    assert_eq!(discovery.candidates().len(), 2);
    assert_eq!(
        discovery
            .events()
            .iter()
            .filter(|event| matches!(event, ClusterDiscoveryEvent::CandidateSeen(_)))
            .count(),
        2
    );

    let diagnostics = HydraCache::client()
        .shared_cluster(Arc::new(InMemoryCluster::new("another-orders")))
        .discovery(discovery.clone())
        .node_id("client-b")
        .connect()
        .await
        .unwrap()
        .cluster_discovery_diagnostics()
        .unwrap();
    assert_eq!(diagnostics.local_node_id.as_str(), "client-b");
    assert_eq!(diagnostics.candidate_count(), 3);
}

#[tokio::test]
async fn chitchat_style_discovery_records_seed_metadata_and_liveness_events() {
    let cluster = Arc::new(InMemoryCluster::new("orders"));
    let discovery = Arc::new(ChitchatStyleDiscovery::new(["seed-a:7000", "seed-b:7000"]));

    assert_eq!(discovery.adapter_name(), "chitchat-style");
    assert_eq!(discovery.seed_count(), 2);
    assert!(discovery.has_seeds());
    assert_eq!(discovery.seeds(), ["seed-a:7000", "seed-b:7000"]);

    let discovery_trait: Arc<dyn ClusterDiscovery> = discovery.clone();
    let member = HydraCache::member()
        .shared_cluster(cluster.clone())
        .discovery(discovery_trait.clone())
        .node_id("member-a")
        .start()
        .await
        .unwrap();
    let client = HydraCache::client()
        .shared_cluster(cluster)
        .discovery(discovery_trait)
        .node_id("client-a")
        .connect()
        .await
        .unwrap();

    discovery.mark_live("member-a");
    discovery.mark_suspect("client-a");
    discovery.mark_dead("client-a");

    let candidates = discovery.candidates();
    assert_eq!(candidates.len(), 2);
    for candidate in &candidates {
        assert_eq!(
            candidate.metadata.get("discovery.adapter").unwrap(),
            "chitchat-style"
        );
        assert_eq!(
            candidate.metadata.get("discovery.seeds").unwrap(),
            "seed-a:7000,seed-b:7000"
        );
    }

    let diagnostics = client.cluster_discovery_diagnostics().unwrap();
    assert_eq!(diagnostics.local_node_id.as_str(), "client-a");
    assert_eq!(diagnostics.candidate_count(), 2);
    assert_eq!(diagnostics.event_count(), 5);
    assert!(diagnostics
        .events
        .iter()
        .any(|event| matches!(event, ClusterDiscoveryEvent::MemberLive(node) if node.as_str() == "member-a")));
    assert!(diagnostics
        .events
        .iter()
        .any(|event| matches!(event, ClusterDiscoveryEvent::MemberSuspect(node) if node.as_str() == "client-a")));
    assert!(diagnostics
        .events
        .iter()
        .any(|event| matches!(event, ClusterDiscoveryEvent::MemberDead(node) if node.as_str() == "client-a")));

    assert_eq!(member.cluster_diagnostics().unwrap().member_count, 1);
}

#[derive(Debug)]
struct RejectingControlPlane {
    bus: Arc<InMemoryInvalidationBus>,
}

impl RejectingControlPlane {
    fn new() -> Self {
        Self {
            bus: Arc::new(InMemoryInvalidationBus::default()),
        }
    }
}

#[async_trait::async_trait]
impl ClusterControlPlane for RejectingControlPlane {
    fn name(&self) -> String {
        "rejecting".to_owned()
    }

    fn invalidation_bus(&self) -> Arc<dyn CacheInvalidationBus> {
        self.bus.clone()
    }

    async fn join_member(
        &self,
        _candidate: ClusterCandidate,
    ) -> crate::CacheResult<crate::ClusterMember> {
        Err(CacheError::Backend(
            "admission denied for member".to_owned(),
        ))
    }

    async fn join_client(
        &self,
        _candidate: ClusterCandidate,
    ) -> crate::CacheResult<crate::ClusterMember> {
        Err(CacheError::Backend(
            "admission denied for client".to_owned(),
        ))
    }

    async fn validate_generation(
        &self,
        _node_id: &ClusterNodeId,
        _generation: ClusterGeneration,
    ) -> crate::CacheResult<()> {
        Ok(())
    }

    async fn leave(
        &self,
        _node_id: &ClusterNodeId,
        _generation: ClusterGeneration,
    ) -> crate::CacheResult<Option<ClusterMembershipEvent>> {
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
            epoch: ClusterEpoch::default(),
            member_count: 0,
            client_count: 0,
            bootstrap,
            connected: false,
            invalidation_subscribers: self.bus.receiver_count(),
            membership_subscribers: 0,
            lifecycle: ClusterLifecycleDiagnostics::idle("rejecting-control-plane"),
        }
    }
}

#[tokio::test]
async fn diagnostics_helpers_describe_disconnected_control_plane_views() {
    let control_plane = RejectingControlPlane::new();
    let diagnostics = control_plane.diagnostics_for(
        ClusterRole::Client,
        ClusterNodeId::from("client-a"),
        ClusterGeneration::new(3),
        vec!["seed-a:7000".to_owned()],
    );

    assert!(diagnostics.is_client_role());
    assert!(!diagnostics.is_member_role());
    assert!(!diagnostics.is_local_role());
    assert_eq!(diagnostics.participant_count(), 0);
    assert_eq!(diagnostics.bootstrap_count(), 1);
    assert!(!diagnostics.has_members());
    assert!(!diagnostics.has_clients());
    assert!(diagnostics.has_bootstrap());
    assert!(!diagnostics.has_multiple_participants());
    assert!(!diagnostics.is_operational());
}

#[tokio::test]
async fn builders_return_custom_control_plane_admission_errors() {
    let control_plane = Arc::new(RejectingControlPlane::new());

    let client_error = HydraCache::client()
        .control_plane(control_plane.clone())
        .node_id("client-a")
        .connect()
        .await
        .unwrap_err();
    assert!(client_error
        .to_string()
        .contains("admission denied for client"));

    let member_error = HydraCache::member()
        .control_plane(control_plane)
        .node_id("member-a")
        .start()
        .await
        .unwrap_err();
    assert!(member_error
        .to_string()
        .contains("admission denied for member"));
}

#[derive(Debug, Default)]
struct RejectingDiscovery;

#[async_trait::async_trait]
impl ClusterDiscovery for RejectingDiscovery {
    async fn announce(&self, _candidate: ClusterCandidate) -> crate::CacheResult<()> {
        Err(CacheError::Backend("discovery announce failed".to_owned()))
    }

    async fn mark_live(&self, _node_id: ClusterNodeId) -> crate::CacheResult<()> {
        Ok(())
    }

    async fn mark_suspect(&self, _node_id: ClusterNodeId) -> crate::CacheResult<()> {
        Ok(())
    }

    async fn mark_dead(&self, _node_id: ClusterNodeId) -> crate::CacheResult<()> {
        Ok(())
    }

    fn candidates(&self) -> Vec<ClusterCandidate> {
        Vec::new()
    }

    fn events(&self) -> Vec<ClusterDiscoveryEvent> {
        Vec::new()
    }
}

#[tokio::test]
async fn builders_return_custom_discovery_errors_before_admission() {
    let cluster = Arc::new(InMemoryCluster::new("orders"));
    let discovery = Arc::new(RejectingDiscovery);

    let error = HydraCache::client()
        .shared_cluster(cluster.clone())
        .discovery(discovery.clone())
        .node_id("client-a")
        .connect()
        .await
        .unwrap_err();
    assert!(error.to_string().contains("discovery announce failed"));
    assert_eq!(cluster.clients().len(), 0);

    let error = HydraCache::member()
        .shared_cluster(cluster.clone())
        .discovery(discovery)
        .node_id("member-a")
        .start()
        .await
        .unwrap_err();
    assert!(error.to_string().contains("discovery announce failed"));
    assert_eq!(cluster.members().len(), 0);
}
