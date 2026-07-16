#[cfg(test)]
mod tests {
    use hydracache::{CacheKeyBuilder, CacheOptions, HydraCache, TagSet};
    use hydracache::{ClusterCandidate, ClusterGeneration, InMemoryCluster};
    use hydracache_actuator_axum::HydraCacheActuator;
    use hydracache_cluster::HydraCluster;
    use hydracache_cluster_chitchat::{ChitchatDiscovery, ChitchatDiscoveryConfig};
    use hydracache_cluster_raft::RaftMetadataRuntime;
    use hydracache_cluster_transport_axum::{
        AxumPeerFetchService, HttpPeerFetch, MemoryPeerFetchStore, PeerFetchHttpRequest,
        PeerFetchHttpResponse, PeerFetchReadThrough, PeerFetchReadThroughPolicy,
        PeerFetchReadThroughStatus, PeerFetchRouter, PeerFetchRouterStatus,
        DEFAULT_PEER_FETCH_PATH,
    };
    use hydracache_db::DbCache;
    use hydracache_diesel::{DieselCache, DieselQueryExt};
    use hydracache_observability::HydraCacheRegistry;
    use hydracache_seaorm::{SeaOrmCache, SeaOrmQueryExt};
    use hydracache_sqlx::SqlxCache;
    use serde::{Deserialize, Serialize};
    use std::sync::Arc;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct User {
        id: u64,
        name: String,
    }

    #[tokio::test]
    async fn published_crate_smoke_test() -> hydracache::CacheResult<()> {
        let cache = HydraCache::local().build();
        let users = cache.typed::<User>("users");

        let key = CacheKeyBuilder::new()
            .tenant(7)
            .entity("user", 42)
            .build_string();

        let tags = TagSet::new().tenant(7).entity("user", 42);

        let user = users
            .get_or_insert_with(&key, CacheOptions::new().tag_set(tags), || async {
                User {
                    id: 42,
                    name: "Ada".to_owned(),
                }
            })
            .await?;

        assert_eq!(user.id, 42);
        assert!(users.contains_key(&key).await);

        assert_eq!(users.invalidate_tag("user:42").await?, 1);
        assert_eq!(users.get(&key).await?, None);

        Ok(())
    }

    #[tokio::test]
    async fn published_cluster_adapter_smoke_test() -> hydracache::CacheResult<()> {
        let discovery = ChitchatDiscovery::spawn_udp(ChitchatDiscoveryConfig::new(
            "orders",
            "member-a",
            ClusterGeneration::new(1),
            "127.0.0.1:0".parse().unwrap(),
        ))
        .await?;

        hydracache::ClusterDiscovery::announce(
            &discovery,
            hydracache::ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)),
        )
        .await?;
        assert_eq!(discovery.candidates().len(), 1);

        let control_plane = Arc::new(RaftMetadataRuntime::single_node("orders", 1)?);
        let member = HydraCache::member()
            .control_plane(control_plane.clone())
            .node_id("member-a")
            .generation(ClusterGeneration::new(1))
            .start()
            .await?;

        assert_eq!(member.cluster_diagnostics().unwrap().member_count, 1);
        assert_eq!(control_plane.snapshot().commands_committed, 1);

        Ok(())
    }

    #[tokio::test]
    async fn published_cluster_composition_smoke_test() -> hydracache::CacheResult<()> {
        let cluster = HydraCluster::builder("orders")
            .node_id("member-b")
            .generation(ClusterGeneration::new(2))
            .raft_single_node(1)
            .build()
            .await?;

        let member = cluster.member_cache().start().await?;
        let diagnostics = member.cluster_diagnostics().unwrap();

        assert_eq!(cluster.cluster_name(), "orders");
        assert_eq!(diagnostics.node_id.as_str(), "member-b");
        assert_eq!(diagnostics.member_count, 1);
        assert_eq!(cluster.raft().snapshot().commands_committed, 1);

        Ok(())
    }

    #[tokio::test]
    async fn published_cluster_ownership_and_peer_fetch_smoke_test() -> hydracache::CacheResult<()>
    {
        let empty = hydracache::InMemoryCluster::new("empty");
        let no_owner = empty.owner_for_key("user:42");
        assert!(!no_owner.has_owner());
        assert!(no_owner.peer_fetch_request().is_none());

        let cluster = Arc::new(hydracache::InMemoryCluster::new("orders"));
        let _member_a = HydraCache::member()
            .shared_cluster(cluster.clone())
            .node_id("member-a")
            .start()
            .await?;
        let _member_b = HydraCache::member()
            .shared_cluster(cluster.clone())
            .node_id("member-b")
            .start()
            .await?;

        let decision = cluster.owner_for_key("user:42");
        assert!(decision.has_owner());
        assert_eq!(decision.member_count, 2);

        let request = decision.peer_fetch_request().unwrap();
        assert!(request.has_generation());
        assert!(request.matches_generation(request.generation.unwrap()));
        assert!(request
            .generation_mismatch(request.generation.unwrap().next())
            .is_some());

        let ownership_diagnostics = cluster.ownership_diagnostics();
        assert_eq!(ownership_diagnostics.resolutions, 1);
        assert_eq!(ownership_diagnostics.no_owner, 0);

        let fetch = hydracache::InMemoryPeerFetch::new();
        fetch.put(
            request.owner.clone(),
            request.key.clone(),
            b"encoded-user".to_vec(),
        );

        let response = hydracache::ClusterPeerFetch::fetch(&fetch, request.clone()).await?;
        assert!(response.is_hit());
        assert_eq!(response.value.unwrap().as_ref(), b"encoded-user");

        let missing = hydracache::ClusterPeerFetch::fetch(
            &fetch,
            hydracache::ClusterPeerFetchRequest::new(request.owner, "missing"),
        )
        .await?;
        assert!(missing.is_miss());

        let fetch_diagnostics = fetch.diagnostics();
        assert_eq!(fetch_diagnostics.hits, 1);
        assert_eq!(fetch_diagnostics.misses, 1);
        assert_eq!(fetch_diagnostics.total_requests(), 2);
        assert_eq!(fetch_diagnostics.hit_ratio(), Some(0.5));

        Ok(())
    }

    #[tokio::test]
    async fn published_http_peer_fetch_transport_smoke_test() -> hydracache::CacheResult<()> {
        let store = MemoryPeerFetchStore::new();
        store.put("user:42", Vec::from("encoded-user"));
        let service =
            AxumPeerFetchService::new("member-a", ClusterGeneration::new(7), Arc::new(store));
        let _routes = service.routes();

        let request = hydracache::ClusterPeerFetchRequest::new("member-a", "user:42")
            .generation(ClusterGeneration::new(7));
        let http_request = PeerFetchHttpRequest::from_peer_request(&request);
        assert_eq!(http_request.owner, "member-a");
        assert_eq!(http_request.generation, Some(7));

        let peer_response = hydracache::ClusterPeerFetchResponse::hit(
            "member-a",
            "user:42",
            Vec::from("encoded-user").into(),
        );
        let http_response = PeerFetchHttpResponse::from_peer_response(&peer_response);
        assert_eq!(
            http_response.decode_value()?.unwrap().as_ref(),
            b"encoded-user"
        );

        let peer_fetch = HttpPeerFetch::for_base_url("http://127.0.0.1:3000/");
        assert_eq!(
            peer_fetch.endpoint(),
            format!("http://127.0.0.1:3000{DEFAULT_PEER_FETCH_PATH}")
        );

        let router = PeerFetchRouter::new();
        let empty = InMemoryCluster::new("empty");
        let no_owner = router
            .fetch_owner_value(empty.owner_for_key("user:42"))
            .await;
        assert_eq!(no_owner.status, PeerFetchRouterStatus::NoOwner);
        assert!(no_owner.did_not_route());

        let cluster = InMemoryCluster::new("orders");
        cluster.join_member(ClusterCandidate::member("member-a"))?;
        let missing_endpoint = router
            .fetch_owner_value(cluster.owner_for_key("user:42"))
            .await;
        assert_eq!(
            missing_endpoint.status,
            PeerFetchRouterStatus::MissingEndpoint
        );
        assert!(missing_endpoint.did_not_route());

        let diagnostics = router.diagnostics();
        assert_eq!(diagnostics.attempts, 2);
        assert_eq!(diagnostics.no_owner, 1);
        assert_eq!(diagnostics.missing_endpoint, 1);
        assert_eq!(diagnostics.routed_requests(), 0);
        assert!(diagnostics.has_failures());

        Ok(())
    }

    #[tokio::test]
    async fn published_read_through_smoke_test() -> hydracache::CacheResult<()> {
        let source = HydraCache::local().build();
        let near_cache = HydraCache::local().build();
        source.put("answer", 42_u64, CacheOptions::new()).await?;
        let encoded = source.get_encoded("answer").await?.expect("source value");
        near_cache
            .put_encoded("answer", encoded, CacheOptions::new().tag("answers"))
            .await?;

        let cluster = InMemoryCluster::new("orders");
        cluster.join_member(
            ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)),
        )?;

        let read_through = PeerFetchReadThrough::new(near_cache.clone());
        let local = read_through
            .fetch_encoded(cluster.owner_for_key("answer"), CacheOptions::new())
            .await?;
        assert_eq!(local.status, PeerFetchReadThroughStatus::LocalHit);
        assert!(local.is_local_hit());
        assert_eq!(near_cache.get::<u64>("answer").await?, Some(42));

        let owner_only = PeerFetchReadThrough::new(HydraCache::local().build())
            .policy(PeerFetchReadThroughPolicy::OwnerOnly)
            .without_hydration();
        let missing_endpoint = owner_only
            .fetch_encoded(cluster.owner_for_key("answer"), CacheOptions::new())
            .await?;
        assert_eq!(
            missing_endpoint.status,
            PeerFetchReadThroughStatus::MissingEndpoint
        );
        assert!(missing_endpoint.is_router_error());

        let diagnostics = owner_only.diagnostics();
        assert_eq!(diagnostics.attempts, 1);
        assert_eq!(diagnostics.router_errors, 1);
        assert!(diagnostics.has_router_errors());

        Ok(())
    }

    #[tokio::test]
    async fn published_sqlx_adapter_smoke_test() -> hydracache_sqlx::Result<()> {
        let cache = HydraCache::local().build();
        let queries = DbCache::new(cache, "db");

        let user = queries
            .cached::<User>()
            .key("user:42")
            .tag("user:42")
            .fetch_with(|| async {
                Ok::<_, std::io::Error>(User {
                    id: 42,
                    name: "Ada".to_owned(),
                })
            })
            .await?;

        assert_eq!(user.name, "Ada");

        Ok(())
    }

    #[tokio::test]
    async fn published_sqlx_alias_smoke_test() -> hydracache_sqlx::Result<()> {
        let cache = HydraCache::local().build();
        let queries = SqlxCache::new(cache, "sqlx");

        let user = queries
            .cached::<User>()
            .key("user:7")
            .fetch_with(|| async {
                Ok::<_, std::io::Error>(User {
                    id: 7,
                    name: "Grace".to_owned(),
                })
            })
            .await?;

        assert_eq!(user.id, 7);

        Ok(())
    }

    #[tokio::test]
    async fn published_diesel_adapter_smoke_test() -> hydracache_diesel::Result<()> {
        let cache = HydraCache::local().build();
        let queries = DieselCache::new(cache, "diesel");

        let user = queries
            .cached::<User>()
            .key("user:42")
            .diesel_one(|| {
                Ok::<_, hydracache_diesel::diesel::result::Error>(User {
                    id: 42,
                    name: "Ada".to_owned(),
                })
            })
            .await?;

        assert_eq!(user.name, "Ada");

        Ok(())
    }

    #[tokio::test]
    async fn published_seaorm_adapter_smoke_test() -> hydracache_seaorm::Result<()> {
        let cache = HydraCache::local().build();
        let queries = SeaOrmCache::new(cache, "seaorm");

        let user = queries
            .cached::<User>()
            .key("user:42")
            .sea_one(|| async {
                Ok::<_, hydracache_seaorm::sea_orm::DbErr>(User {
                    id: 42,
                    name: "Ada".to_owned(),
                })
            })
            .await?;

        assert_eq!(user.name, "Ada");

        Ok(())
    }

    #[tokio::test]
    async fn published_observability_and_actuator_smoke_test() -> hydracache::CacheResult<()> {
        let cache = HydraCache::local().build();
        cache
            .get_or_insert_with("answer", CacheOptions::new(), || async { 42_u64 })
            .await?;
        cache
            .get_or_insert_with("answer", CacheOptions::new(), || async { 7_u64 })
            .await?;

        let registry = HydraCacheRegistry::new().with_cache("main", cache);
        let diagnostics = registry.diagnostics("main").await.unwrap();
        assert_eq!(diagnostics.stats.loads, 1);
        assert_eq!(diagnostics.stats.hits, 1);
        assert_eq!(diagnostics.hit_ratio(), Some(0.5));

        let routes = HydraCacheActuator::new(registry).routes();
        let _ = routes;

        Ok(())
    }
}
