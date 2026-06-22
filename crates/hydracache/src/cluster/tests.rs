use std::sync::Arc;
use std::time::Duration;

use super::{
    ChitchatStyleDiscovery, ClusterAdmissionBridge, ClusterAdmissionBridgeConfig,
    ClusterAdmissionBridgeDiagnostics, ClusterAdmissionBridgeEvent, ClusterAdmissionIgnoreReason,
    ClusterAdmissionRejectReason, ClusterCandidate, ClusterComponent, ClusterControlPlane,
    ClusterDiagnostics, ClusterDiscovery, ClusterDiscoveryDiagnostics, ClusterDiscoveryEvent,
    ClusterEndpoints, ClusterEpoch, ClusterFillCounters, ClusterGeneration, ClusterHealthReason,
    ClusterHealthState, ClusterLifecycleComponent, ClusterLifecycleDiagnostics,
    ClusterLifecycleStatus, ClusterLoadReport, ClusterMember, ClusterMembershipEvent,
    ClusterMembershipEventBus, ClusterMembershipRecvError, ClusterNodeId, ClusterOwnershipDecision,
    ClusterOwnershipResolver, ClusterPeerFetch, ClusterPeerFetchGenerationMismatch,
    ClusterPeerFetchRequest, ClusterPeerFetchResponse, ClusterRole, ClusterStagingCounters,
    ClusterStagingHealth, InMemoryCluster, InMemoryClusterDiscovery, InMemoryPeerFetch,
    RendezvousClusterOwnership, CLUSTER_PEER_FETCH_BASE_URL_METADATA_KEY,
};
use crate::HydraCache;
use bytes::Bytes;
use hydracache_core::CacheStats;

#[test]
fn node_id_formats_and_converts_from_strings() {
    let id = ClusterNodeId::from("node-a");
    assert_eq!(id.as_str(), "node-a");
    assert_eq!(id.to_string(), "node-a");

    let owned = ClusterNodeId::from("node-b".to_owned());
    assert!(owned > id);
}

#[test]
fn generation_ordering_tracks_restarts() {
    let first = ClusterGeneration::new(7);
    let second = first.next();

    assert_eq!(first.value(), 7);
    assert_eq!(second.value(), 8);
    assert!(second > first);
}

#[test]
fn lifecycle_diagnostics_track_start_stop_and_helpers() {
    let mut lifecycle = ClusterLifecycleDiagnostics::idle("admission-bridge");

    assert_eq!(lifecycle.component, "admission-bridge");
    assert_eq!(lifecycle.status, ClusterLifecycleStatus::Idle);
    assert!(!lifecycle.is_terminal());

    lifecycle.record_start();
    assert!(lifecycle.is_running());
    assert_eq!(lifecycle.start_count, 1);
    assert_eq!(lifecycle.stop_count, 0);
    assert!(!lifecycle.shutdown_requested);

    lifecycle.record_shutdown_requested();
    assert!(lifecycle.is_stopping());
    assert!(lifecycle.shutdown_requested);

    lifecycle.record_graceful_stop();
    assert!(lifecycle.is_stopped());
    assert!(lifecycle.is_terminal());
    assert_eq!(lifecycle.stop_count, 1);

    lifecycle.record_start();
    assert!(lifecycle.is_running());
    assert_eq!(lifecycle.start_count, 2);
    assert!(!lifecycle.shutdown_requested);
}

#[test]
fn lifecycle_diagnostics_report_failure_and_running_constructor() {
    let mut lifecycle = ClusterLifecycleDiagnostics::running("peer-fetch");

    assert_eq!(lifecycle.status, ClusterLifecycleStatus::Running);
    assert_eq!(lifecycle.start_count, 1);

    lifecycle.record_failure("socket closed");
    assert!(lifecycle.has_failed());
    assert!(lifecycle.is_terminal());
    assert_eq!(lifecycle.last_error.as_deref(), Some("socket closed"));

    lifecycle.record_shutdown_requested();
    assert!(lifecycle.has_failed());
    assert!(lifecycle.shutdown_requested);
}

#[tokio::test]
async fn component_start_is_idempotent() {
    let component = ClusterLifecycleComponent::new("invalidation-pump");

    component.start().await.unwrap();
    component.start().await.unwrap();

    let diagnostics = component.diagnostics();
    assert_eq!(component.name(), "invalidation-pump");
    assert!(diagnostics.is_running());
    assert_eq!(diagnostics.start_count, 1);
}

#[tokio::test]
async fn component_stop_records_graceful_stop() {
    let component = ClusterLifecycleComponent::new("transport-server");

    component.start().await.unwrap();
    component.stop().await.unwrap();
    component.stop().await.unwrap();

    let diagnostics = component.diagnostics();
    assert!(diagnostics.is_stopped());
    assert_eq!(diagnostics.stop_count, 1);
    assert!(component.last_error().is_none());
}

#[tokio::test]
async fn component_failure_sets_last_error_and_failed_status() {
    let component = ClusterLifecycleComponent::new("discovery-bridge");

    component.start().await.unwrap();
    component.fail("listener closed");

    let diagnostics = component.diagnostics();
    assert!(diagnostics.has_failed());
    assert_eq!(component.last_error().as_deref(), Some("listener closed"));
}

#[test]
fn staging_health_healthy_cluster_is_healthy() {
    let health = ClusterStagingHealth::from_parts(
        staging_diagnostics(ClusterLifecycleDiagnostics::running("cluster-runtime")),
        CacheStats::default(),
        ClusterFillCounters::default(),
        ClusterStagingCounters::default(),
    );

    assert_eq!(health.state, ClusterHealthState::Healthy);
    assert!(health.state.ready_for_staging());
    assert!(health.state.reasons().is_empty());
}

#[test]
fn staging_health_lagged_receiver_is_degraded() {
    let health = ClusterStagingHealth::from_parts(
        staging_diagnostics(ClusterLifecycleDiagnostics::running("cluster-runtime")),
        CacheStats {
            distributed_invalidation_lagged: 1,
            ..CacheStats::default()
        },
        ClusterFillCounters::default(),
        ClusterStagingCounters::default(),
    );

    assert_eq!(
        health.state,
        ClusterHealthState::Degraded {
            reasons: vec![ClusterHealthReason::LaggedReceivers { count: 1 }]
        }
    );
    assert!(!health.state.ready_for_staging());
}

#[test]
fn staging_health_decode_error_is_not_ready() {
    let health = ClusterStagingHealth::from_parts(
        staging_diagnostics(ClusterLifecycleDiagnostics::running("cluster-runtime")),
        CacheStats {
            distributed_invalidation_decode_errors: 1,
            ..CacheStats::default()
        },
        ClusterFillCounters::default(),
        ClusterStagingCounters::default(),
    );

    assert_eq!(
        health.state,
        ClusterHealthState::NotReady {
            reasons: vec![ClusterHealthReason::DecodeErrors { count: 1 }]
        }
    );
}

#[test]
fn staging_health_stale_generation_alone_stays_healthy() {
    let health = ClusterStagingHealth::from_parts(
        staging_diagnostics(ClusterLifecycleDiagnostics::running("cluster-runtime")),
        CacheStats::default(),
        ClusterFillCounters::default(),
        ClusterStagingCounters {
            stale_generation_rejected: 1,
            ..ClusterStagingCounters::default()
        },
    );

    assert_eq!(health.stale_generation_rejected, 1);
    assert_eq!(health.state, ClusterHealthState::Healthy);
}

#[test]
fn staging_health_stopped_lifecycle_is_not_ready() {
    let mut lifecycle = ClusterLifecycleDiagnostics::running("cluster-runtime");
    lifecycle.record_shutdown_requested();
    lifecycle.record_graceful_stop();

    let health = ClusterStagingHealth::from_parts(
        staging_diagnostics(lifecycle),
        CacheStats::default(),
        ClusterFillCounters::default(),
        ClusterStagingCounters::default(),
    );

    assert_eq!(
        health.state,
        ClusterHealthState::NotReady {
            reasons: vec![ClusterHealthReason::LifecycleNotRunning]
        }
    );
}

#[test]
fn recent_gossip_reset_downgrades_health_to_degraded() {
    let health = ClusterStagingHealth::from_parts(
        staging_diagnostics(ClusterLifecycleDiagnostics::running("cluster-runtime")),
        CacheStats::default(),
        ClusterFillCounters::default(),
        ClusterStagingCounters {
            tombstone_age_ms: 42,
            gossip_reset_count: 1,
            ..ClusterStagingCounters::default()
        },
    );

    assert_eq!(
        health.state,
        ClusterHealthState::Degraded {
            reasons: vec![ClusterHealthReason::GossipResetRecent {
                tombstone_age_ms: 42,
                reset_count: 1,
            }]
        }
    );
}

#[test]
fn staging_health_local_role_returns_none() {
    let cache = HydraCache::local().build();

    assert!(cache.cluster_staging_health().is_none());
}

#[test]
fn fill_owner_load_increments_only_owner_counter() {
    let cache = HydraCache::local().build();

    cache.record_cluster_owner_load_success();

    assert_eq!(
        cache.cluster_fill_counters(),
        ClusterFillCounters {
            owner_load_success: 1,
            ..ClusterFillCounters::default()
        }
    );
}

#[test]
fn fill_remote_fetch_increments_only_remote_counter() {
    let cache = HydraCache::local().build();

    cache.record_cluster_remote_fetch_success();

    assert_eq!(
        cache.cluster_fill_counters(),
        ClusterFillCounters {
            remote_fetch_success: 1,
            ..ClusterFillCounters::default()
        }
    );
}

#[test]
fn fill_hot_cache_hit_increments_only_hot_counter() {
    let cache = HydraCache::local().build();

    cache.record_cluster_hot_cache_hit();

    assert_eq!(
        cache.cluster_fill_counters(),
        ClusterFillCounters {
            hot_cache_hits: 1,
            ..ClusterFillCounters::default()
        }
    );
}

#[test]
fn fill_counters_are_mutually_exclusive_per_event() {
    let owner = HydraCache::local().build();
    let remote = HydraCache::local().build();
    let hot = HydraCache::local().build();

    owner.record_cluster_owner_load_success();
    remote.record_cluster_remote_fetch_success();
    hot.record_cluster_hot_cache_hit();

    assert_eq!(owner.cluster_fill_counters().successful_events(), 1);
    assert_eq!(remote.cluster_fill_counters().successful_events(), 1);
    assert_eq!(hot.cluster_fill_counters().successful_events(), 1);
}

#[test]
fn load_report_totals_health_and_json_shape_are_stable() {
    let report = ClusterLoadReport {
        nodes: 4,
        requests: 240,
        read_ops: 228,
        invalidation_ops: 12,
        published: 12,
        received: 24,
        applied: 24,
        lagged: 0,
        decode_errors: 0,
        publish_failures: 0,
        receiver_closed: 0,
        stale_generation_rejected: 1,
        peer_fetch_auth_failures: 1,
        wire_version_rejections: 1,
        owner_load_success: 5,
        remote_fetch_success: 3,
        hot_cache_hits: 7,
        elapsed_ms: 320,
    };

    assert!(report.totals_match_requests());
    assert!(report.has_clean_invalidation_health());

    let value = serde_json::to_value(&report).unwrap();
    assert_eq!(value["nodes"], 4);
    assert_eq!(value["requests"], 240);
    assert_eq!(value["published"], 12);
    assert_eq!(value["stale_generation_rejected"], 1);
    assert_eq!(value["peer_fetch_auth_failures"], 1);
    assert_eq!(value["wire_version_rejections"], 1);
    assert_eq!(value["elapsed_ms"], 320);
}

fn staging_diagnostics(lifecycle: ClusterLifecycleDiagnostics) -> ClusterDiagnostics {
    ClusterDiagnostics {
        cluster_name: "orders".to_owned(),
        role: ClusterRole::Member,
        node_id: ClusterNodeId::from("member-a"),
        generation: ClusterGeneration::new(3),
        epoch: ClusterEpoch::new(7),
        member_count: 1,
        client_count: 1,
        bootstrap: vec!["127.0.0.1:7000".to_owned()],
        connected: true,
        invalidation_subscribers: 1,
        membership_subscribers: 1,
        lifecycle,
    }
}

#[test]
fn role_marks_only_members_as_future_voters() {
    assert!(!ClusterRole::Local.can_vote());
    assert!(!ClusterRole::Client.can_vote());
    assert!(ClusterRole::Member.can_vote());
}

#[test]
fn endpoints_builder_sets_advertised_addresses() {
    let endpoints = ClusterEndpoints::new()
        .control("127.0.0.1:7000")
        .invalidation("127.0.0.1:7001")
        .diagnostics("http://127.0.0.1:3000");

    assert_eq!(endpoints.control.as_deref(), Some("127.0.0.1:7000"));
    assert_eq!(endpoints.invalidation.as_deref(), Some("127.0.0.1:7001"));
    assert_eq!(
        endpoints.diagnostics.as_deref(),
        Some("http://127.0.0.1:3000")
    );
}

#[test]
fn candidate_carries_generation_endpoints_and_metadata() {
    let candidate = ClusterCandidate::member("member-a")
        .generation(ClusterGeneration::new(3))
        .endpoints(ClusterEndpoints::new().control("127.0.0.1:7000"))
        .metadata("version", "0.20.0");

    assert_eq!(candidate.node_id.as_str(), "member-a");
    assert_eq!(candidate.role, ClusterRole::Member);
    assert_eq!(candidate.generation.value(), 3);
    assert_eq!(
        candidate.endpoints.control.as_deref(),
        Some("127.0.0.1:7000")
    );
    assert_eq!(
        candidate.metadata.get("version").map(String::as_str),
        Some("0.20.0")
    );
}

#[test]
fn peer_fetch_endpoint_metadata_is_carried_to_member() {
    let candidate =
        ClusterCandidate::member("member-a").peer_fetch_base_url("http://127.0.0.1:3000");

    assert_eq!(
        candidate.peer_fetch_base_url_value(),
        Some("http://127.0.0.1:3000")
    );
    assert_eq!(
        candidate
            .metadata
            .get(CLUSTER_PEER_FETCH_BASE_URL_METADATA_KEY)
            .map(String::as_str),
        Some("http://127.0.0.1:3000")
    );

    let member = ClusterMember::from_candidate(candidate, ClusterEpoch::new(1));

    assert_eq!(member.peer_fetch_base_url(), Some("http://127.0.0.1:3000"));
}

#[test]
fn rendezvous_ownership_resolver_selects_stable_member_owner() {
    let resolver = RendezvousClusterOwnership;
    let first = ClusterMember::from_candidate(
        ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)),
        ClusterEpoch::new(1),
    );
    let second = ClusterMember::from_candidate(
        ClusterCandidate::member("member-b").generation(ClusterGeneration::new(1)),
        ClusterEpoch::new(1),
    );
    let client = ClusterMember::from_candidate(
        ClusterCandidate::client("client-a").generation(ClusterGeneration::new(1)),
        ClusterEpoch::new(1),
    );
    let participants = vec![first.clone(), second.clone(), client];
    let reversed = vec![second, first];

    let decision = resolver.resolve_owner("user:42", &participants);
    let reversed_decision = resolver.resolve_owner("user:42", &reversed);

    assert_eq!(decision.resolver, "rendezvous");
    assert_eq!(decision.key, "user:42");
    assert_eq!(decision.member_count, 2);
    assert!(decision.has_owner());
    assert_eq!(decision.owner_node_id(), reversed_decision.owner_node_id());
    assert_eq!(decision.owner_generation(), Some(ClusterGeneration::new(1)));
}

#[test]
fn rendezvous_ownership_resolver_reports_no_owner_without_members() {
    let resolver = RendezvousClusterOwnership;
    let participants = vec![ClusterMember::from_candidate(
        ClusterCandidate::client("client-a"),
        ClusterEpoch::default(),
    )];

    let decision = resolver.resolve_owner("user:42", &participants);

    assert_eq!(decision.member_count, 0);
    assert!(!decision.has_owner());
    assert!(decision.owner_node_id().is_none());
    assert!(decision.owner_generation().is_none());
    assert!(decision.peer_fetch_request().is_none());
}

#[tokio::test]
async fn ownership_decision_builds_peer_fetch_request_for_owner() {
    let member = ClusterMember::from_candidate(
        ClusterCandidate::member("member-a").generation(ClusterGeneration::new(3)),
        ClusterEpoch::new(1),
    );
    let decision = ClusterOwnershipDecision {
        key: "user:42".to_owned(),
        owner: Some(member),
        member_count: 1,
        resolver: "test",
    };

    let request = decision.peer_fetch_request().expect("owner exists");

    assert_eq!(request.owner.as_str(), "member-a");
    assert_eq!(request.key, "user:42");
    assert_eq!(request.generation, Some(ClusterGeneration::new(3)));
}

#[test]
fn ownership_decision_without_owner_cannot_build_peer_fetch_request() {
    let decision = ClusterOwnershipDecision {
        key: "user:42".to_owned(),
        owner: None,
        member_count: 0,
        resolver: "test",
    };

    assert!(!decision.has_owner());
    assert!(decision.peer_fetch_request().is_none());
}

#[test]
fn peer_fetch_request_reports_generation_mismatch() {
    let request =
        ClusterPeerFetchRequest::new("member-a", "user:42").generation(ClusterGeneration::new(3));

    assert!(request.has_generation());
    assert!(request.matches_generation(ClusterGeneration::new(3)));
    assert_eq!(
        request.generation_mismatch(ClusterGeneration::new(4)),
        Some(ClusterPeerFetchGenerationMismatch {
            requested: ClusterGeneration::new(3),
            current: ClusterGeneration::new(4),
        })
    );
    assert!(!request.matches_generation(ClusterGeneration::new(4)));

    let generationless = ClusterPeerFetchRequest::new("member-a", "user:42");
    assert!(!generationless.has_generation());
    assert!(generationless.matches_generation(ClusterGeneration::new(99)));
    assert!(generationless
        .generation_mismatch(ClusterGeneration::new(99))
        .is_none());
}

#[tokio::test]
async fn in_memory_peer_fetch_returns_hits_misses_and_removes_values() {
    let fetch = InMemoryPeerFetch::new();
    let owner = ClusterNodeId::from("member-a");

    assert!(fetch.is_empty());
    fetch.put(owner.clone(), "user:42", Bytes::from_static(b"encoded"));
    assert_eq!(fetch.len(), 1);

    let hit = fetch
        .fetch(ClusterPeerFetchRequest::new(owner.clone(), "user:42"))
        .await
        .unwrap();
    assert_eq!(
        hit,
        ClusterPeerFetchResponse::hit(owner.clone(), "user:42", Bytes::from_static(b"encoded"))
    );
    assert!(hit.is_hit());
    assert!(!hit.is_miss());

    let missing = fetch
        .fetch(ClusterPeerFetchRequest::new(owner.clone(), "user:99"))
        .await
        .unwrap();
    assert!(missing.is_miss());
    assert_eq!(
        missing,
        ClusterPeerFetchResponse::miss(owner.clone(), "user:99")
    );

    assert_eq!(
        fetch.remove(&owner, "user:42"),
        Some(Bytes::from_static(b"encoded"))
    );
    assert!(fetch.is_empty());

    let removed = fetch
        .fetch(ClusterPeerFetchRequest::new(owner.clone(), "user:42"))
        .await
        .unwrap();
    assert!(removed.is_miss());
    assert_eq!(
        fetch.remove(&owner, "user:42"),
        None,
        "removing an already removed value is a no-op"
    );

    let diagnostics = fetch.diagnostics();
    assert_eq!(diagnostics.stored_values, 0);
    assert_eq!(diagnostics.hits, 1);
    assert_eq!(diagnostics.misses, 2);
    assert_eq!(diagnostics.total_requests(), 3);
    assert_eq!(diagnostics.hit_ratio(), Some(1.0 / 3.0));
}

#[test]
fn discovery_events_keep_candidate_and_liveness_information() {
    let candidate = ClusterCandidate::client("client-a");

    assert_eq!(
        ClusterDiscoveryEvent::CandidateSeen(candidate.clone()),
        ClusterDiscoveryEvent::CandidateSeen(candidate)
    );
    assert_eq!(
        ClusterDiscoveryEvent::MemberLive(ClusterNodeId::from("member-a")),
        ClusterDiscoveryEvent::MemberLive(ClusterNodeId::from("member-a"))
    );
    assert_ne!(
        ClusterDiscoveryEvent::MemberSuspect(ClusterNodeId::from("member-a")),
        ClusterDiscoveryEvent::MemberDead(ClusterNodeId::from("member-a"))
    );
}

#[test]
fn discovery_diagnostics_helpers_report_candidate_and_event_counts() {
    let diagnostics = ClusterDiscoveryDiagnostics {
        local_node_id: ClusterNodeId::from("client-a"),
        candidates: vec![ClusterCandidate::client("client-a")],
        events: vec![ClusterDiscoveryEvent::MemberLive(ClusterNodeId::from(
            "client-a",
        ))],
    };

    assert_eq!(diagnostics.candidate_count(), 1);
    assert_eq!(diagnostics.event_count(), 1);
    assert!(diagnostics.has_candidates());
    assert!(diagnostics.has_events());
}

#[test]
fn admission_bridge_diagnostics_record_events_without_double_counting_failures() {
    let candidate = ClusterCandidate::member("member-a").generation(ClusterGeneration::new(3));
    let admitted = ClusterMember::from_candidate(candidate.clone(), Default::default());
    let mut diagnostics = ClusterAdmissionBridgeDiagnostics::default();

    diagnostics.record_event(&ClusterAdmissionBridgeEvent::CandidateSeen(
        candidate.clone(),
    ));
    diagnostics.record_event(&ClusterAdmissionBridgeEvent::CandidateIgnored {
        candidate: candidate.clone(),
        reason: ClusterAdmissionIgnoreReason::AlreadyCurrent,
    });
    diagnostics.record_event(&ClusterAdmissionBridgeEvent::CandidateAdmitted(admitted));
    diagnostics.record_event(&ClusterAdmissionBridgeEvent::CandidateRejected {
        candidate: candidate.clone(),
        reason: ClusterAdmissionRejectReason::AdmissionError("raft unavailable".to_owned()),
    });
    diagnostics.record_event(&ClusterAdmissionBridgeEvent::BridgeStopped);

    assert_eq!(diagnostics.candidates_seen, 1);
    assert_eq!(diagnostics.candidates_ignored, 1);
    assert_eq!(diagnostics.candidates_admitted, 1);
    assert_eq!(diagnostics.candidates_rejected, 1);
    assert_eq!(diagnostics.admission_failures, 1);
    assert_eq!(diagnostics.total_decisions(), 3);
    assert!(diagnostics.has_seen_candidates());
    assert!(diagnostics.has_admissions());
    assert!(diagnostics.has_issues());
    assert_eq!(diagnostics.last_candidate, Some(candidate.node_id.clone()));
    assert_eq!(diagnostics.last_admitted, Some(candidate.node_id));
    assert_eq!(diagnostics.last_error.as_deref(), Some("raft unavailable"));
}

#[tokio::test]
async fn admission_bridge_run_once_admits_candidates_and_deduplicates_generation() {
    let discovery = Arc::new(InMemoryClusterDiscovery::new());
    let control_plane = Arc::new(InMemoryCluster::new("orders"));
    let bridge = ClusterAdmissionBridge::new(discovery.clone(), control_plane.clone());

    discovery.announce(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)));

    assert_eq!(bridge.run_once().await, 1);
    assert_eq!(control_plane.members().len(), 1);
    assert_eq!(control_plane.events().len(), 1);

    assert_eq!(bridge.run_once().await, 1);
    assert_eq!(control_plane.events().len(), 1);

    let diagnostics = bridge.diagnostics();
    assert_eq!(diagnostics.candidates_seen, 2);
    assert_eq!(diagnostics.candidates_admitted, 1);
    assert_eq!(diagnostics.candidates_ignored, 1);
    assert_eq!(diagnostics.total_decisions(), 2);
    assert!(matches!(
        bridge.events().last(),
        Some(ClusterAdmissionBridgeEvent::CandidateIgnored {
            reason: ClusterAdmissionIgnoreReason::AlreadyCurrent,
            ..
        })
    ));
}

#[tokio::test]
async fn admission_bridge_allows_role_transition_for_same_generation() {
    let discovery = Arc::new(InMemoryClusterDiscovery::new());
    let control_plane = Arc::new(InMemoryCluster::new("orders"));
    let bridge = ClusterAdmissionBridge::new(discovery.clone(), control_plane.clone());

    discovery.announce(ClusterCandidate::client("node-a").generation(ClusterGeneration::new(1)));
    assert_eq!(bridge.run_once().await, 1);
    assert_eq!(control_plane.clients().len(), 1);

    discovery.announce(ClusterCandidate::member("node-a").generation(ClusterGeneration::new(1)));
    assert_eq!(bridge.run_once().await, 1);

    assert_eq!(control_plane.clients().len(), 0);
    assert_eq!(control_plane.members().len(), 1);
    assert_eq!(control_plane.events().len(), 2);
    assert_eq!(bridge.diagnostics().candidates_admitted, 2);
}

#[tokio::test]
async fn admission_bridge_rejects_stale_candidate_before_control_plane_write() {
    let discovery = Arc::new(InMemoryClusterDiscovery::new());
    let control_plane = Arc::new(InMemoryCluster::new("orders"));
    let bridge = ClusterAdmissionBridge::new(discovery.clone(), control_plane.clone());

    discovery.announce(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(2)));
    assert_eq!(bridge.run_once().await, 1);

    discovery.announce(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)));
    assert_eq!(bridge.run_once().await, 1);

    assert_eq!(control_plane.members()[0].generation.value(), 2);
    assert_eq!(control_plane.events().len(), 1);
    assert!(matches!(
        bridge.events().last(),
        Some(ClusterAdmissionBridgeEvent::CandidateRejected {
            reason: ClusterAdmissionRejectReason::StaleGeneration { existing, attempted },
            ..
        }) if existing.value() == 2 && attempted.value() == 1
    ));
}

#[tokio::test]
async fn admission_bridge_respects_role_filters_and_ignores_local_candidates() {
    let discovery = Arc::new(InMemoryClusterDiscovery::new());
    let control_plane = Arc::new(InMemoryCluster::new("orders"));
    let bridge = ClusterAdmissionBridge::with_config(
        discovery.clone(),
        control_plane.clone(),
        ClusterAdmissionBridgeConfig::default().admit_clients(false),
    );
    let mut local_candidate = ClusterCandidate::client("local-a");
    local_candidate.role = ClusterRole::Local;

    discovery.announce(ClusterCandidate::client("client-a"));
    discovery.announce(local_candidate);

    assert_eq!(bridge.run_once().await, 2);
    assert!(control_plane.clients().is_empty());
    assert!(control_plane.members().is_empty());

    let diagnostics = bridge.diagnostics();
    assert_eq!(diagnostics.candidates_seen, 2);
    assert_eq!(diagnostics.candidates_ignored, 2);
    assert!(bridge.events().iter().any(|event| matches!(
        event,
        ClusterAdmissionBridgeEvent::CandidateIgnored {
            reason: ClusterAdmissionIgnoreReason::RoleDisabled,
            ..
        }
    )));
    assert!(bridge.events().iter().any(|event| matches!(
        event,
        ClusterAdmissionBridgeEvent::CandidateIgnored {
            reason: ClusterAdmissionIgnoreReason::LocalRole,
            ..
        }
    )));
}

#[test]
fn admission_bridge_config_normalizes_zero_interval_and_member_filter() {
    let config = ClusterAdmissionBridgeConfig::default()
        .poll_interval(Duration::ZERO)
        .admit_members(false);

    assert_eq!(config.normalized_poll_interval(), Duration::from_millis(1));
    assert!(!config.admit_members);
    assert!(config.admit_clients);
}

#[tokio::test]
async fn admission_bridge_background_loop_can_shutdown_gracefully() {
    let discovery = Arc::new(InMemoryClusterDiscovery::new());
    let control_plane = Arc::new(InMemoryCluster::new("orders"));
    let bridge = ClusterAdmissionBridge::with_config(
        discovery.clone(),
        control_plane.clone(),
        ClusterAdmissionBridgeConfig::default().poll_interval(Duration::from_millis(1)),
    );

    discovery.announce(ClusterCandidate::member("member-a"));
    assert_eq!(
        bridge.lifecycle_diagnostics().status,
        ClusterLifecycleStatus::Idle
    );
    let handle = bridge.start();
    assert!(bridge.lifecycle_diagnostics().is_running());
    assert_eq!(bridge.lifecycle_diagnostics().start_count, 1);

    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if control_plane.members().len() == 1 {
                return;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .expect("background bridge should admit the candidate");

    handle.shutdown().await;

    assert_eq!(control_plane.members().len(), 1);
    let lifecycle = bridge.lifecycle_diagnostics();
    assert!(lifecycle.is_stopped());
    assert_eq!(lifecycle.stop_count, 1);
    assert!(lifecycle.shutdown_requested);
    assert!(matches!(
        bridge.events().last(),
        Some(ClusterAdmissionBridgeEvent::BridgeStopped)
    ));
}

#[tokio::test]
async fn admission_bridge_handle_drop_requests_shutdown_and_records_stop() {
    let discovery = Arc::new(InMemoryClusterDiscovery::new());
    let control_plane = Arc::new(InMemoryCluster::new("orders"));
    let bridge = ClusterAdmissionBridge::with_config(
        discovery,
        control_plane,
        ClusterAdmissionBridgeConfig::default().poll_interval(Duration::from_millis(1)),
    );

    let handle = bridge.start();
    assert!(bridge.lifecycle_diagnostics().is_running());

    drop(handle);
    let lifecycle = bridge.lifecycle_diagnostics();
    assert!(lifecycle.is_stopping() || lifecycle.is_stopped());
    assert!(lifecycle.shutdown_requested);

    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if bridge.lifecycle_diagnostics().is_stopped() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .expect("dropped bridge handle should stop the task");

    assert_eq!(bridge.lifecycle_diagnostics().stop_count, 1);
}

#[test]
fn in_memory_discovery_records_candidates_and_liveness_events() {
    let discovery = InMemoryClusterDiscovery::new();
    let first = ClusterCandidate::member("member-a")
        .generation(ClusterGeneration::new(1))
        .metadata("zone", "eu");
    let second = ClusterCandidate::member("member-a").generation(ClusterGeneration::new(2));

    discovery.announce(first);
    discovery.announce(second);
    discovery.mark_live("member-a");
    discovery.mark_suspect("member-a");
    discovery.mark_dead("member-a");

    let candidates = discovery.candidates();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].generation.value(), 2);
    assert_eq!(discovery.events().len(), 5);
    assert!(matches!(
        discovery.events().last(),
        Some(ClusterDiscoveryEvent::MemberDead(node_id)) if node_id.as_str() == "member-a"
    ));
}

#[tokio::test]
async fn in_memory_discovery_satisfies_discovery_contract() {
    let discovery: Arc<dyn ClusterDiscovery> = Arc::new(InMemoryClusterDiscovery::new());

    discovery
        .announce(ClusterCandidate::client("client-a"))
        .await
        .unwrap();
    discovery
        .mark_live(ClusterNodeId::from("client-a"))
        .await
        .unwrap();
    discovery
        .mark_suspect(ClusterNodeId::from("client-a"))
        .await
        .unwrap();
    discovery
        .mark_dead(ClusterNodeId::from("client-a"))
        .await
        .unwrap();

    assert_eq!(discovery.candidates().len(), 1);
    assert_eq!(discovery.events().len(), 4);
    assert!(matches!(
        discovery.events().last(),
        Some(ClusterDiscoveryEvent::MemberDead(node_id)) if node_id.as_str() == "client-a"
    ));
}

#[tokio::test]
async fn chitchat_style_discovery_satisfies_trait_and_seed_metadata_paths() {
    let empty = ChitchatStyleDiscovery::default();
    assert_eq!(empty.seed_count(), 0);
    assert!(!empty.has_seeds());
    assert_eq!(empty.adapter_name(), "chitchat-style");

    let discovery = ChitchatStyleDiscovery::new(["127.0.0.1:7000", "127.0.0.1:7001"]);
    assert_eq!(discovery.seed_count(), 2);
    assert!(discovery.has_seeds());
    assert_eq!(discovery.seeds()[0], "127.0.0.1:7000");

    let discovery: Arc<dyn ClusterDiscovery> = Arc::new(discovery);
    discovery
        .announce(ClusterCandidate::member("member-a"))
        .await
        .unwrap();
    discovery
        .mark_live(ClusterNodeId::from("member-a"))
        .await
        .unwrap();
    discovery
        .mark_suspect(ClusterNodeId::from("member-a"))
        .await
        .unwrap();
    discovery
        .mark_dead(ClusterNodeId::from("member-a"))
        .await
        .unwrap();

    let candidates = discovery.candidates();
    assert_eq!(candidates.len(), 1);
    assert_eq!(
        candidates[0]
            .metadata
            .get("discovery.adapter")
            .map(String::as_str),
        Some("chitchat-style")
    );
    assert_eq!(
        candidates[0]
            .metadata
            .get("discovery.seeds")
            .map(String::as_str),
        Some("127.0.0.1:7000,127.0.0.1:7001")
    );
    assert_eq!(discovery.events().len(), 4);
}

#[tokio::test]
async fn closed_membership_subscriber_and_display_errors_are_observable() {
    assert_eq!(
        ClusterMembershipRecvError::Closed.to_string(),
        "cluster membership subscription closed"
    );
    assert_eq!(
        ClusterMembershipRecvError::Lagged(3).to_string(),
        "cluster membership subscriber lagged by 3 events"
    );

    let mut subscriber = super::ClusterMembershipSubscriber::closed();
    assert_eq!(
        subscriber.recv().await.unwrap_err(),
        ClusterMembershipRecvError::Closed
    );
    assert_eq!(subscriber.next_event().await, None);
}

#[test]
fn in_memory_cluster_admits_members_and_clients() {
    let cluster = InMemoryCluster::new("orders");

    let member = cluster
        .join_member(ClusterCandidate::member("member-a"))
        .unwrap();
    let client = cluster
        .join_client(ClusterCandidate::client("client-a"))
        .unwrap();

    assert_eq!(cluster.name(), "orders");
    assert!(member.is_member());
    assert!(client.is_client());
    assert_eq!(cluster.epoch().value(), 1);
    assert_eq!(cluster.members().len(), 1);
    assert_eq!(cluster.clients().len(), 1);
    assert_eq!(cluster.events().len(), 2);
}

#[tokio::test]
async fn membership_subscriber_receives_join_leave_and_stale_rejection_events() {
    let cluster = InMemoryCluster::new("orders");
    let mut events = cluster.subscribe_membership();
    let member_id = ClusterNodeId::from("member-a");

    cluster
        .join_member(
            ClusterCandidate::member(member_id.clone()).generation(ClusterGeneration::new(2)),
        )
        .unwrap();
    assert!(matches!(
        events.recv().await.unwrap(),
        ClusterMembershipEvent::MemberJoined(member) if member.node_id == member_id
    ));

    let error = cluster
        .join_member(
            ClusterCandidate::member(member_id.clone()).generation(ClusterGeneration::new(1)),
        )
        .unwrap_err();
    assert!(error.to_string().contains("stale cluster generation"));
    assert!(matches!(
        events.recv().await.unwrap(),
        ClusterMembershipEvent::StaleGenerationRejected {
            node_id,
            role: ClusterRole::Member,
            existing,
            attempted,
            reason,
        } if node_id == member_id
            && existing.value() == 2
            && attempted.value() == 1
            && reason == "stale-generation"
    ));

    cluster
        .leave(&member_id, ClusterGeneration::new(2))
        .unwrap()
        .unwrap();
    assert!(matches!(
        events.recv().await.unwrap(),
        ClusterMembershipEvent::NodeLeft {
            node_id,
            role: ClusterRole::Member,
            ..
        } if node_id == member_id
    ));
}

#[tokio::test]
async fn membership_subscriber_reports_lag_for_slow_consumers() {
    let bus = ClusterMembershipEventBus::new(1);
    let mut events = bus.subscribe();
    let first =
        ClusterMember::from_candidate(ClusterCandidate::member("member-a"), ClusterEpoch::new(1));
    let second =
        ClusterMember::from_candidate(ClusterCandidate::member("member-b"), ClusterEpoch::new(2));

    bus.publish(ClusterMembershipEvent::MemberJoined(first));
    bus.publish(ClusterMembershipEvent::MemberJoined(second));

    assert!(matches!(
        events.recv().await,
        Err(ClusterMembershipRecvError::Lagged(1))
    ));
    assert!(matches!(
        events.recv().await.unwrap(),
        ClusterMembershipEvent::MemberJoined(member) if member.node_id.as_str() == "member-b"
    ));
}

#[test]
fn in_memory_cluster_rejects_stale_generation() {
    let cluster = InMemoryCluster::new("orders");
    cluster
        .join_member(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(2)))
        .unwrap();

    let error = cluster
        .join_member(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)))
        .unwrap_err();

    assert!(error.to_string().contains("stale cluster generation"));
    assert!(matches!(
        cluster.events().last(),
        Some(ClusterMembershipEvent::StaleGenerationRejected { .. })
    ));
}

#[test]
fn in_memory_cluster_allows_generation_upgrade_and_advances_epoch() {
    let cluster = InMemoryCluster::new("orders");
    cluster
        .join_member(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)))
        .unwrap();
    cluster
        .join_member(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(2)))
        .unwrap();

    assert_eq!(cluster.epoch().value(), 2);
    assert_eq!(cluster.members()[0].generation.value(), 2);
}

#[test]
fn client_to_member_promotion_moves_node_between_role_sets() {
    let cluster = InMemoryCluster::new("orders");
    cluster
        .join_client(ClusterCandidate::client("node-a"))
        .unwrap();
    cluster
        .join_member(ClusterCandidate::member("node-a"))
        .unwrap();

    assert_eq!(cluster.clients().len(), 0);
    assert_eq!(cluster.members().len(), 1);
    assert_eq!(cluster.members()[0].role, ClusterRole::Member);
}

#[test]
fn leave_removes_clients_without_advancing_epoch_and_members_with_epoch() {
    let cluster = InMemoryCluster::new("orders");
    let member_id = ClusterNodeId::from("member-a");
    let client_id = ClusterNodeId::from("client-a");
    cluster
        .join_member(ClusterCandidate::member(member_id.clone()))
        .unwrap();
    cluster
        .join_client(ClusterCandidate::client(client_id.clone()))
        .unwrap();

    let client_left = cluster
        .leave(&client_id, ClusterGeneration::default())
        .unwrap()
        .unwrap();
    assert_eq!(cluster.epoch().value(), 1);
    assert!(matches!(
        client_left,
        ClusterMembershipEvent::NodeLeft {
            role: ClusterRole::Client,
            ..
        }
    ));

    let member_left = cluster
        .leave(&member_id, ClusterGeneration::default())
        .unwrap()
        .unwrap();
    assert_eq!(cluster.epoch().value(), 2);
    assert!(matches!(
        member_left,
        ClusterMembershipEvent::NodeLeft {
            role: ClusterRole::Member,
            ..
        }
    ));
    assert!(cluster
        .leave(&member_id, ClusterGeneration::default())
        .unwrap()
        .is_none());
}

#[test]
fn leave_rejects_stale_generation_without_removing_newer_node() {
    let cluster = InMemoryCluster::new("orders");
    let node_id = ClusterNodeId::from("member-a");

    cluster
        .join_member(
            ClusterCandidate::member(node_id.clone()).generation(ClusterGeneration::new(1)),
        )
        .unwrap();
    cluster
        .join_member(
            ClusterCandidate::member(node_id.clone()).generation(ClusterGeneration::new(2)),
        )
        .unwrap();

    let error = cluster
        .leave(&node_id, ClusterGeneration::new(1))
        .unwrap_err();

    assert!(error.to_string().contains("stale cluster generation"));
    assert_eq!(cluster.members().len(), 1);
    assert_eq!(cluster.members()[0].generation.value(), 2);
    assert!(matches!(
        cluster.events().last(),
        Some(ClusterMembershipEvent::StaleGenerationRejected { .. })
    ));
}

#[test]
fn diagnostics_report_counts_bootstrap_and_subscribers() {
    let cluster = Arc::new(InMemoryCluster::new("orders"));
    cluster
        .join_member(ClusterCandidate::member("member-a"))
        .unwrap();
    let _subscriber = cluster.invalidation_bus().subscribe();

    let diagnostics = cluster.diagnostics_for(
        ClusterRole::Member,
        ClusterNodeId::from("member-a"),
        ClusterGeneration::default(),
        vec!["seed-a:7000".to_owned()],
    );

    assert_eq!(diagnostics.cluster_name, "orders");
    assert_eq!(diagnostics.role, ClusterRole::Member);
    assert_eq!(diagnostics.node_id.as_str(), "member-a");
    assert_eq!(diagnostics.member_count, 1);
    assert_eq!(diagnostics.client_count, 0);
    assert_eq!(diagnostics.bootstrap, ["seed-a:7000".to_owned()]);
    assert!(diagnostics.connected);
    assert_eq!(diagnostics.invalidation_subscribers, 1);
    assert!(diagnostics.is_member_role());
    assert!(!diagnostics.is_client_role());
    assert!(!diagnostics.is_local_role());
    assert_eq!(diagnostics.participant_count(), 1);
    assert_eq!(diagnostics.bootstrap_count(), 1);
    assert!(diagnostics.has_members());
    assert!(!diagnostics.has_clients());
    assert!(diagnostics.has_bootstrap());
    assert!(diagnostics.has_invalidation_subscribers());
    assert!(!diagnostics.has_membership_subscribers());
    assert!(!diagnostics.has_multiple_participants());
    assert!(diagnostics.is_operational());

    let ownership = cluster.ownership_diagnostics();
    assert_eq!(ownership.resolver, "rendezvous");
    assert_eq!(ownership.resolutions, 0);
    assert_eq!(ownership.no_owner, 0);
    assert_eq!(ownership.owner_found(), 0);
    assert!(!ownership.has_resolutions());
    assert_eq!(ownership.owner_found_ratio(), None);
}

#[test]
fn in_memory_cluster_resolves_key_owner_from_admitted_members() {
    let cluster = InMemoryCluster::new("orders");

    let empty = cluster.owner_for_key("user:42");
    assert!(!empty.has_owner());
    assert_eq!(empty.member_count, 0);

    cluster
        .join_member(ClusterCandidate::member("member-a"))
        .unwrap();
    cluster
        .join_member(ClusterCandidate::member("member-b"))
        .unwrap();
    cluster
        .join_client(ClusterCandidate::client("client-a"))
        .unwrap();

    let first = cluster.owner_for_key("user:42");
    let second = cluster.owner_for_key("user:42");
    let different_key = cluster.owner_for_key("user:99");

    assert_eq!(first.resolver, "rendezvous");
    assert_eq!(first.member_count, 2);
    assert!(first.has_owner());
    assert_eq!(first.owner_node_id(), second.owner_node_id());
    assert!(different_key.has_owner());
    assert!(
        ["member-a", "member-b"].contains(&different_key.owner_node_id().expect("owner").as_str())
    );

    let diagnostics = cluster.ownership_diagnostics();
    assert_eq!(diagnostics.resolutions, 4);
    assert_eq!(diagnostics.no_owner, 1);
    assert_eq!(diagnostics.owner_found(), 3);
    assert_eq!(diagnostics.owner_found_ratio(), Some(0.75));
}

#[test]
fn ownership_ignores_client_join_and_leave() {
    let cluster = InMemoryCluster::new("orders");
    cluster
        .join_member(ClusterCandidate::member("member-a"))
        .unwrap();
    cluster
        .join_member(ClusterCandidate::member("member-b"))
        .unwrap();

    let before_clients = cluster.owner_for_key("user:42");
    cluster
        .join_client(ClusterCandidate::client("client-a"))
        .unwrap();
    cluster
        .join_client(ClusterCandidate::client("client-b"))
        .unwrap();
    let after_client_join = cluster.owner_for_key("user:42");

    cluster
        .leave(
            &ClusterNodeId::from("client-a"),
            ClusterGeneration::default(),
        )
        .unwrap();
    let after_client_leave = cluster.owner_for_key("user:42");

    assert_eq!(before_clients.member_count, 2);
    assert_eq!(
        before_clients.owner_node_id(),
        after_client_join.owner_node_id()
    );
    assert_eq!(
        before_clients.owner_node_id(),
        after_client_leave.owner_node_id()
    );
    assert_eq!(cluster.clients().len(), 1);
}

#[test]
fn ownership_moves_when_owner_member_leaves_and_returns_on_rejoin() {
    let cluster = InMemoryCluster::new("orders");
    cluster
        .join_member(ClusterCandidate::member("member-a"))
        .unwrap();
    cluster
        .join_member(ClusterCandidate::member("member-b"))
        .unwrap();

    let initial = cluster.owner_for_key("user:42");
    let initial_owner = initial.owner.clone().expect("initial owner");
    let initial_owner_id = initial_owner.node_id.clone();
    let survivor = cluster
        .members()
        .into_iter()
        .find(|member| member.node_id != initial_owner_id)
        .expect("surviving member");

    cluster
        .leave(&initial_owner.node_id, initial_owner.generation)
        .unwrap();
    let after_leave = cluster.owner_for_key("user:42");

    assert_eq!(after_leave.member_count, 1);
    assert_eq!(after_leave.owner_node_id(), Some(&survivor.node_id));

    let rejoined_generation = initial_owner.generation.next();
    cluster
        .join_member(
            ClusterCandidate::member(initial_owner_id.as_str()).generation(rejoined_generation),
        )
        .unwrap();
    let after_rejoin = cluster.owner_for_key("user:42");

    assert_eq!(after_rejoin.member_count, 2);
    assert_eq!(after_rejoin.owner_node_id(), Some(&initial_owner_id));
    assert_eq!(after_rejoin.owner_generation(), Some(rejoined_generation));
}

#[test]
fn stale_member_candidate_does_not_replace_owner_generation() {
    let cluster = InMemoryCluster::new("orders");
    cluster
        .join_member(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(2)))
        .unwrap();

    let stale = cluster
        .join_member(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)));
    let owner = cluster.owner_for_key("user:42");

    assert!(stale
        .unwrap_err()
        .to_string()
        .contains("stale cluster generation"));
    assert_eq!(owner.member_count, 1);
    assert_eq!(
        owner.owner_node_id().map(ClusterNodeId::as_str),
        Some("member-a")
    );
    assert_eq!(owner.owner_generation(), Some(ClusterGeneration::new(2)));
    assert_eq!(cluster.members().len(), 1);
}

#[tokio::test]
async fn in_memory_cluster_satisfies_control_plane_contract() {
    let control_plane: Arc<dyn ClusterControlPlane> = Arc::new(InMemoryCluster::new("orders"));

    let member = control_plane
        .join_member(ClusterCandidate::member("member-a"))
        .await
        .unwrap();
    let client = control_plane
        .join_client(ClusterCandidate::client("client-a"))
        .await
        .unwrap();

    assert_eq!(control_plane.name(), "orders");
    assert!(member.is_member());
    assert!(client.is_client());
    let _receiver = control_plane.invalidation_bus().subscribe();

    let diagnostics = control_plane.diagnostics_for(
        ClusterRole::Client,
        ClusterNodeId::from("client-a"),
        ClusterGeneration::default(),
        vec!["seed-a:7000".to_owned()],
    );
    assert_eq!(diagnostics.member_count, 1);
    assert_eq!(diagnostics.client_count, 1);
    assert_eq!(diagnostics.bootstrap, ["seed-a:7000".to_owned()]);

    let left = control_plane
        .leave(
            &ClusterNodeId::from("client-a"),
            ClusterGeneration::default(),
        )
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        left,
        ClusterMembershipEvent::NodeLeft {
            role: ClusterRole::Client,
            ..
        }
    ));
}
