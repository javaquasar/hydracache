use futures_util::Stream;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use hydracache_cluster_testkit::value_plane_cluster_admission_075::*;
use hydracache_cluster_testkit::value_plane_cluster_auth_075::*;
use hydracache_cluster_testkit::value_plane_cluster_health_075::*;
use hydracache_cluster_testkit::value_plane_cluster_runtime_075::*;
use hydracache_cluster_testkit::value_plane_collection_075::CollectionBounds;
use hydracache_cluster_testkit::value_plane_model_075::MutationIdentity;
use hydracache_cluster_testkit::value_plane_security_075::{
    ReplayDisposition, RouteClaims, SecurityBounds, TrustEpochWindow,
};
use hydracache_cluster_testkit::value_plane_service_075::*;
use hydracache_cluster_testkit::value_plane_service_cluster_075::*;
use hydracache_cluster_testkit::value_plane_service_registry_075::*;
use hydracache_cluster_testkit::value_plane_stream_075::{DemandStream, DemandStreamError};
use hydracache_cluster_testkit::value_plane_surface_075::{
    IngressPrincipal, SurfaceId, SurfaceOperation, VerifiedMapCommand,
};
use hydracache_cluster_transport_axum::{ClusterRoute, DenyRouteAuthorizer, NodeCredential};

fn collection() -> CollectionBounds {
    CollectionBounds {
        max_entries: 32,
        max_key_bytes: 64,
        max_value_bytes: 128,
        max_page_items: 16,
        max_scan_items: 64,
        max_response_bytes: 4096,
        max_bulk_mutation_items: 32,
    }
}
fn security() -> SecurityBounds {
    SecurityBounds {
        max_envelope_bytes: 1024 * 1024,
        max_key_bytes: 64,
        max_value_bytes: 128,
        ..SecurityBounds::default()
    }
}
fn config() -> ClusterRuntimeConfig {
    ClusterRuntimeConfig {
        cluster: "cluster-private".into(),
        members: ["a", "b", "c"]
            .into_iter()
            .map(|node| RuntimeMember {
                node: node.into(),
                generation: 1,
                credential: NodeCredential::new("key-1", format!("secret-{node}")),
            })
            .collect(),
        entry_node: "c".into(),
        bounds: ServiceClusterBounds {
            partitions: 2,
            max_frame_bytes: 1024 * 1024,
            ..ServiceClusterBounds::default()
        },
        admission: ClusterAdmissionBounds {
            max_replica_bytes: 64 * 1024 * 1024,
            ..ClusterAdmissionBounds::default()
        },
        max_replays: 4096,
    }
}
fn authenticated(tenant: &str, namespace: &str, generation: u64) -> AuthenticatedNamespace {
    authenticated_epoch(tenant, namespace, generation, 1, 0)
}
fn authenticated_epoch(
    tenant: &str,
    namespace: &str,
    generation: u64,
    epoch: u64,
    now: u64,
) -> AuthenticatedNamespace {
    let principal = IngressPrincipal {
        tenant: tenant.into(),
    };
    let verified = VerifiedMapCommand::new(
        SurfaceId::Hc2Rust,
        &principal,
        tenant,
        namespace,
        generation,
        b"k".to_vec(),
        SurfaceOperation::Read,
    )
    .unwrap();
    let claims = RouteClaims {
        authenticated: true,
        authenticated_tenant: tenant,
        claimed_tenant: tenant,
        expected_namespace_generation: generation,
        actual_namespace_generation: generation,
        expected_topology_epoch: 1,
        actual_topology_epoch: 1,
        proxy_hops: 0,
        deadline_remaining_ticks: 1000,
        trust_epoch: epoch,
        encoded_envelope_bytes: 64,
        key_bytes: 1,
        value_bytes: 0,
    };
    AuthenticatedNamespace::verify(
        &principal,
        &verified,
        &claims,
        security(),
        TrustEpochWindow::new(epoch, 3).unwrap(),
        now,
    )
    .unwrap()
}
fn registry(config: ClusterRuntimeConfig) -> UnifiedServiceRegistry {
    UnifiedServiceRegistry::new_clustered(
        RegistryBounds {
            max_cleanup_work: 64,
            ..RegistryBounds::default()
        },
        collection(),
        ServiceLimits::default(),
        security(),
        TrustEpochWindow::new(1, 3).unwrap(),
        config,
    )
    .unwrap()
}
fn open(
    registry: &UnifiedServiceRegistry,
    tenant: &str,
    namespace: &str,
) -> (AuthenticatedNamespace, RegistryHandle) {
    let context = authenticated(tenant, namespace, 1);
    let handle = registry.open(&context, context.address()).unwrap();
    (context, handle)
}
fn command(operation: &str, sequence: u64) -> ServiceCommand {
    let mut command = ServiceCommand::point(operation, &[0, 255]);
    command.value = Some(b"v".to_vec());
    if sequence != 0 {
        command.identity = Some(MutationIdentity::new("client", sequence));
    }
    command
}
fn run(
    registry: &UnifiedServiceRegistry,
    handle: &RegistryHandle,
    context: &AuthenticatedNamespace,
    command: ServiceCommand,
) -> Result<ServiceClusterExecution, RegistryError> {
    let ticket = registry.enqueue(handle, context, SurfaceId::Hc2Rust, command)?;
    let record = registry.run_next()?.unwrap();
    assert_eq!(record.ticket, ticket);
    match record.result? {
        RegistryResult::ClusterCommand(result) => Ok(result),
        other => panic!("unexpected runtime result {other:?}"),
    }
}

#[test]
fn clustered_registry_routes_actual_journals_owners_backups_and_isolates_binary_keys() {
    let registry = registry(config());
    let (a, ah) = open(&registry, "tenant-a", "same");
    let (b, bh) = open(&registry, "tenant-b", "same");
    let inserted = run(&registry, &ah, &a, command("put", 1)).unwrap();
    assert_eq!(inserted.result.kind, "inserted");
    assert!(inserted.backup_proved);
    assert!(!inserted.routes.is_empty());
    assert!(inserted.routes.iter().all(|route| route.owner != "c"));
    let owners = registry.diagnostics(&ah, &a).unwrap().owners;
    assert_eq!(owners.values, 2);
    assert!(owners.completed_outcomes >= 4);
    assert_eq!(
        run(&registry, &bh, &b, command("get", 0))
            .unwrap()
            .result
            .kind,
        "absent"
    );
    let mut other = command("put", 1);
    other.value = Some(b"other".to_vec());
    run(&registry, &bh, &b, other).unwrap();
    assert_eq!(
        run(&registry, &ah, &a, command("get", 0))
            .unwrap()
            .result
            .value
            .as_deref(),
        Some(b"v".as_slice())
    );
    let replay = run(&registry, &ah, &a, command("put", 1)).unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.result, inserted.result);
    let observed = registry.cluster_admission_observations().unwrap();
    assert_eq!(observed.queued, [0; 3]);
    assert_eq!(observed.inflight, [0; 3]);
    assert_eq!(observed.replica_bytes, 0);
    assert_eq!(registry.usage().reserved_growth, 0);
    assert!(registry.cluster_health(&ah, &a).unwrap().ready);
    assert!(!registry.cluster_health(&ah, &a).unwrap().production_enabled);
}

#[test]
fn forged_ingress_envelopes_fail_before_actual_admission_mutation_or_proof() {
    for fault in [
        EnvelopeFault::Tenant,
        EnvelopeFault::Namespace,
        EnvelopeFault::NamespaceGeneration,
        EnvelopeFault::Cluster,
        EnvelopeFault::SourceGeneration,
        EnvelopeFault::Target,
        EnvelopeFault::Credential,
        EnvelopeFault::RequestDigest,
        EnvelopeFault::MutationIdentity,
        EnvelopeFault::Deadline,
    ] {
        let registry = registry(config());
        let (context, handle) = open(&registry, "tenant-a", "map");
        let before = registry.diagnostics(&handle, &context).unwrap();
        registry
            .enqueue_cluster(
                &handle,
                &context,
                SurfaceId::Resp,
                command("put", 1),
                RuntimeCallOptions {
                    auth_fault: Some(AuthFault {
                        stage: ServiceMessageKind::Request,
                        fault,
                    }),
                    ..RuntimeCallOptions::default()
                },
            )
            .unwrap();
        let result = registry.run_next().unwrap().unwrap().result;
        assert!(
            matches!(
                result,
                Err(RegistryError::Cluster(ServiceClusterError::Authorization(
                    _
                )))
            ),
            "{fault:?}: {result:?}"
        );
        assert_eq!(registry.diagnostics(&handle, &context).unwrap(), before);
        assert_eq!(registry.usage().reserved_growth, 0);
        assert_eq!(
            registry.cluster_admission_observations().unwrap().released,
            1
        );
    }
}

#[test]
fn forged_replica_or_ack_never_becomes_a_proved_client_success() {
    for stage in [ServiceMessageKind::ReplicaApply, ServiceMessageKind::Ack] {
        let registry = registry(config());
        let (context, handle) = open(&registry, "tenant-a", "map");
        registry
            .enqueue_cluster(
                &handle,
                &context,
                SurfaceId::Hc2Java,
                command("put", 1),
                RuntimeCallOptions {
                    auth_fault: Some(AuthFault {
                        stage,
                        fault: EnvelopeFault::AckDigest,
                    }),
                    ..RuntimeCallOptions::default()
                },
            )
            .unwrap();
        match registry.run_next().unwrap().unwrap().result {
            Ok(RegistryResult::ClusterCommand(result)) => {
                assert!(!result.backup_proved);
                assert!(result.failure.is_some());
                assert_ne!(result.result.kind, "inserted");
            }
            Err(RegistryError::Cluster(ServiceClusterError::Authorization(_))) => {}
            other => panic!("forged proof escaped: {other:?}"),
        }
        let health = registry.cluster_health(&handle, &context).unwrap();
        assert!(!health.ready);
        assert!(health.reasons.contains(&ReadinessReason::BackupUnproved));
        assert_eq!(
            registry
                .cluster_admission_observations()
                .unwrap()
                .replica_bytes,
            0
        );
    }
}

#[derive(Default)]
struct Capture {
    messages: Mutex<Vec<ServiceMessageContext>>,
}
impl ServiceMessageGuard for Capture {
    fn authorize(&self, message: &ServiceMessageContext) -> Result<(), ServiceClusterError> {
        let mut messages = self.messages.lock().unwrap();
        if messages.len() == 64 {
            return Err(ServiceClusterError::InvalidBound);
        }
        messages.push(message.clone());
        Ok(())
    }
}

#[test]
fn authenticated_replay_binds_actual_message_fields_and_redacts_credentials() {
    let mut cfg = config();
    cfg.bounds.partitions = 1;
    let mut cluster = ServiceClusterHarness::new(
        &["a", "b", "c"],
        cfg.bounds,
        collection(),
        ServiceLimits::default(),
        1,
    )
    .unwrap();
    let capture = Arc::new(Capture::default());
    cluster
        .execute_guarded(
            "c",
            command("put", 1),
            ServiceClusterCall::default(),
            capture.clone(),
        )
        .unwrap();
    let mut next = command("put", 2);
    next.value = Some(b"next".to_vec());
    cluster
        .execute_guarded("c", next, ServiceClusterCall::default(), capture.clone())
        .unwrap();
    let messages = capture.messages.lock().unwrap();
    let first = messages
        .iter()
        .find(|message| message.kind == ServiceMessageKind::Request)
        .unwrap();
    let auth = RuntimeAuthentication::new(
        ClusterAuthConfig {
            cluster: cfg.cluster.clone(),
            tenant: "tenant-a".into(),
            namespace: "map".into(),
            generation: 1,
            members: cfg.members.clone(),
            max_replays: 64,
        },
        security(),
        TrustEpochWindow::new(1, 3).unwrap(),
    )
    .unwrap();
    let principal = authenticated("tenant-a", "map", 1);
    let envelope = auth.make_envelope(&principal, first, 1).unwrap();
    assert_eq!(
        auth.verify_envelope(&envelope, first, 1).unwrap(),
        ReplayDisposition::New
    );
    assert_eq!(
        auth.verify_envelope(&envelope, first, 1).unwrap(),
        ReplayDisposition::ExactReplay
    );
    let changed = messages
        .iter()
        .find(|message| {
            message.kind == first.kind
                && message.source == first.source
                && message.request_digest != first.request_digest
        })
        .unwrap();
    let mut reused = auth.make_envelope(&principal, changed, 1).unwrap();
    reused.nonce = envelope.nonce;
    assert_eq!(
        auth.verify_envelope(&reused, changed, 1).unwrap_err(),
        ServiceClusterError::Authorization("replay")
    );
    assert_eq!(
        auth.verify_envelope(&envelope, first, 2).unwrap_err(),
        ServiceClusterError::Authorization("request_binding")
    );
    let printed = format!("{envelope:?}");
    for secret in [
        "secret-a",
        "secret-b",
        "secret-c",
        "tenant-a",
        "cluster-private",
    ] {
        assert!(!printed.contains(secret));
    }
    auth.update_member_generation(&first.source, 2).unwrap();
    assert!(auth.verify_envelope(&envelope, first, 1).is_err());
    assert!(auth.update_member_generation(&first.source, 1).is_err());
    let denied = RuntimeAuthentication::with_authorizer(
        ClusterAuthConfig {
            cluster: cfg.cluster,
            tenant: "tenant-a".into(),
            namespace: "map".into(),
            generation: 1,
            members: cfg.members,
            max_replays: 1,
        },
        security(),
        TrustEpochWindow::new(1, 3).unwrap(),
        Arc::new(DenyRouteAuthorizer::new(ClusterRoute::OwnerLoad)),
    )
    .unwrap();
    let denied_envelope = denied.make_envelope(&principal, first, 1).unwrap();
    assert_eq!(
        denied
            .verify_envelope(&denied_envelope, first, 1)
            .unwrap_err(),
        ServiceClusterError::Authorization("authorization")
    );
}

#[test]
fn cancelled_cluster_queue_credits_release_once_without_dispatching() {
    let registry = registry(config());
    let (context, handle) = open(&registry, "tenant-a", "map");
    let first = registry
        .enqueue(&handle, &context, SurfaceId::Resp, command("put", 1))
        .unwrap();
    let second = registry
        .enqueue(&handle, &context, SurfaceId::Resp, command("put", 2))
        .unwrap();
    assert!(registry
        .cancel_cluster_queued(&handle, &context, first)
        .unwrap());
    assert!(!registry
        .cancel_cluster_queued(&handle, &context, first)
        .unwrap());
    assert_eq!(
        registry
            .diagnostics(&handle, &context)
            .unwrap()
            .owners
            .total(),
        0
    );
    let record = registry.run_next().unwrap().unwrap();
    assert_eq!(record.ticket, second);
    record.result.unwrap();
    let observations = registry.cluster_admission_observations().unwrap();
    assert_eq!(observations.released, 2);
    assert_eq!(observations.cancelled, 1);
    assert_eq!(observations.owned_bytes, [0; 3]);
    assert_eq!(registry.usage().queued_requests, 0);
}

#[test]
fn clustered_delete_reclaims_actual_replicas_and_fences_old_generation() {
    let registry = registry(config());
    let (context, handle) = open(&registry, "tenant-a", "map");
    run(&registry, &handle, &context, command("put", 1)).unwrap();
    let before = registry.diagnostics(&handle, &context).unwrap().owners;
    assert_eq!(before.values, 2);
    assert!(before.completed_outcomes >= 4);
    registry.enqueue_delete(&handle, &context).unwrap();
    assert_eq!(
        registry.run_next().unwrap().unwrap().result.unwrap(),
        RegistryResult::DeleteStarted
    );
    assert!(registry
        .enqueue(&handle, &context, SurfaceId::Resp, command("get", 0))
        .is_err());
    for _ in 0..64 {
        registry.enqueue_reclaim(&handle, &context, 1).unwrap();
        let progress = registry.run_next().unwrap().unwrap().result.unwrap();
        if matches!(progress, RegistryResult::Cleanup(NamespaceCleanup { phase: hydracache_cluster_testkit::value_plane_admission_075::NamespacePhase::Reclaimable, .. })) { break; }
    }
    assert_eq!(
        registry
            .diagnostics(&handle, &context)
            .unwrap()
            .owners
            .total(),
        0
    );
    let next = registry.recreate(&handle, &context).unwrap();
    let fresh = authenticated("tenant-a", "map", 2);
    assert_eq!(
        run(&registry, &next, &fresh, command("get", 0))
            .unwrap()
            .result
            .kind,
        "absent"
    );
    assert!(registry
        .enqueue(&handle, &context, SurfaceId::Resp, command("get", 0))
        .is_err());
}

#[test]
fn readiness_and_bounded_drain_follow_actual_backup_and_owner_state() {
    let registry = registry(config());
    let (context, handle) = open(&registry, "tenant-a", "map");
    run(&registry, &handle, &context, command("put", 1)).unwrap();
    registry
        .enqueue_maintenance(
            &handle,
            &context,
            RuntimeMaintenance::SetNodeLive {
                node: "b".into(),
                live: false,
            },
        )
        .unwrap();
    registry.run_next().unwrap().unwrap().result.unwrap();
    let unavailable = registry.cluster_health(&handle, &context).unwrap();
    assert!(!unavailable.ready);
    assert!(unavailable.reasons.iter().any(|reason| matches!(
        reason,
        ReadinessReason::OwnerUnavailable | ReadinessReason::BackupUnavailable
    )));
    registry
        .enqueue_maintenance(
            &handle,
            &context,
            RuntimeMaintenance::SetNodeLive {
                node: "b".into(),
                live: true,
            },
        )
        .unwrap();
    registry.run_next().unwrap().unwrap().result.unwrap();
    registry
        .enqueue_maintenance(
            &handle,
            &context,
            RuntimeMaintenance::BeginDrain { node: "a".into() },
        )
        .unwrap();
    registry.run_next().unwrap().unwrap().result.unwrap();
    assert!(registry
        .cluster_health(&handle, &context)
        .unwrap()
        .reasons
        .contains(&ReadinessReason::Draining));
    registry
        .enqueue_cluster(
            &handle,
            &context,
            SurfaceId::Resp,
            command("get", 0),
            RuntimeCallOptions {
                entry_node: Some("a".into()),
                ..RuntimeCallOptions::default()
            },
        )
        .unwrap();
    assert!(registry.run_next().unwrap().unwrap().result.is_err());
    let mut done = false;
    for _ in 0..64 {
        registry
            .enqueue_maintenance(
                &handle,
                &context,
                RuntimeMaintenance::DrainStep {
                    node: "a".into(),
                    maximum_partitions: 1,
                    maximum_reclaim: 16,
                },
            )
            .unwrap();
        if let RegistryResult::Maintenance(MaintenanceResult::Drain(progress)) =
            registry.run_next().unwrap().unwrap().result.unwrap()
        {
            if progress.complete {
                done = true;
                break;
            }
        }
    }
    assert!(done);
    let observations = registry.cluster_admission_observations().unwrap();
    assert_eq!(observations.inflight, [0; 3]);
    assert_eq!(observations.owned_bytes, [0; 3]);
    assert_eq!(registry.usage().reserved_growth, 0);
    assert_eq!(
        run(&registry, &handle, &context, command("get", 0))
            .unwrap()
            .result
            .value
            .as_deref(),
        Some(b"v".as_slice())
    );
}

fn rotated_members() -> Vec<RuntimeMember> {
    config()
        .members
        .into_iter()
        .map(|mut member| {
            member.credential =
                NodeCredential::new("key-2", format!("fresh-secret-{}", member.node));
            member
        })
        .collect()
}

#[test]
fn member_credentials_rotate_atomically_and_old_secrets_cannot_claim_a_new_epoch() {
    let mut cfg = config();
    cfg.bounds.partitions = 1;
    let mut core = ServiceClusterHarness::new(
        &["a", "b", "c"],
        cfg.bounds,
        collection(),
        ServiceLimits::default(),
        1,
    )
    .unwrap();
    let capture = Arc::new(Capture::default());
    core.execute_guarded(
        "c",
        command("put", 1),
        ServiceClusterCall::default(),
        capture.clone(),
    )
    .unwrap();
    let mut expected = capture
        .messages
        .lock()
        .unwrap()
        .iter()
        .find(|message| message.kind == ServiceMessageKind::Request)
        .unwrap()
        .clone();
    let auth_config = ClusterAuthConfig {
        cluster: cfg.cluster,
        tenant: "tenant-a".into(),
        namespace: "map".into(),
        generation: 1,
        members: cfg.members.clone(),
        max_replays: 32,
    };
    let auth = RuntimeAuthentication::new(
        auth_config.clone(),
        security(),
        TrustEpochWindow::new(1, 3).unwrap(),
    )
    .unwrap();
    let principal = authenticated("tenant-a", "map", 1);
    let old = auth.make_envelope(&principal, &expected, 1).unwrap();
    auth.verify_envelope(&old, &expected, 1).unwrap();
    let mut invalid = rotated_members();
    invalid[1].credential.token = cfg.members[1].credential.token.clone();
    assert!(auth.rotate_credentials(2, 0, &invalid).is_err());
    assert_eq!(
        auth.current_members()
            .iter()
            .map(|member| &member.credential)
            .collect::<Vec<_>>(),
        cfg.members
            .iter()
            .map(|member| &member.credential)
            .collect::<Vec<_>>()
    );
    auth.rotate_credentials(2, 0, &rotated_members()).unwrap();
    expected.now = 1;
    assert_eq!(
        auth.verify_envelope(&old, &expected, 1).unwrap(),
        ReplayDisposition::ExactReplay
    );
    let new = auth.make_envelope(&principal, &expected, 1).unwrap();
    assert_eq!(new.member_trust_epoch, 2);
    auth.verify_envelope(&new, &expected, 1).unwrap();
    expected.now = 4;
    let mut forged = old.clone();
    forged.trust_epoch = 2;
    forged.member_trust_epoch = 2;
    assert_eq!(
        auth.verify_envelope(&forged, &expected, 1).unwrap_err(),
        ServiceClusterError::Authorization("member_trust")
    );
    forged.member_trust_epoch = 1;
    assert_eq!(
        auth.verify_envelope(&forged, &expected, 1).unwrap_err(),
        ServiceClusterError::Authorization("member_trust")
    );
    let current_principal = authenticated_epoch("tenant-a", "map", 1, 2, 4);
    let current = auth
        .make_envelope(&current_principal, &expected, 1)
        .unwrap();
    auth.verify_envelope(&current, &expected, 1).unwrap();
    let mut oversized = current.clone();
    oversized.tenant = "x".repeat(129);
    assert_eq!(
        auth.verify_envelope(&oversized, &expected, 1).unwrap_err(),
        ServiceClusterError::Authorization("envelope_bound")
    );
    let generations = RuntimeAuthentication::new(
        auth_config.clone(),
        security(),
        TrustEpochWindow::new(1, 3).unwrap(),
    )
    .unwrap();
    generations.update_member_generation("a", 2).unwrap();
    assert!(generations
        .rotate_credentials(2, 0, &rotated_members())
        .is_err());
    assert_eq!(
        generations
            .current_members()
            .iter()
            .find(|member| member.node == "a")
            .unwrap()
            .generation,
        2
    );
    assert!(generations
        .current_members()
        .iter()
        .all(|member| member.credential.key_id == "key-1"));
    let mut duplicate = auth_config;
    duplicate.members[1].credential.token = duplicate.members[0].credential.token.clone();
    assert!(RuntimeAuthentication::new(
        duplicate,
        security(),
        TrustEpochWindow::new(1, 3).unwrap()
    )
    .is_err());
}

#[test]
fn clustered_operator_rotation_preserves_overlap_and_recreate_without_resurrecting_old_keys() {
    let registry = registry(config());
    let mut invalid = rotated_members();
    invalid.pop();
    assert!(registry.rotate_cluster_credentials(2, invalid).is_err());
    let (old, handle) = open(&registry, "tenant-a", "map");
    assert!(registry.rotate_trust(2).is_err());
    run(&registry, &handle, &old, command("put", 1)).unwrap();
    registry
        .rotate_cluster_credentials(2, rotated_members())
        .unwrap();
    assert_eq!(
        run(&registry, &handle, &old, command("get", 0))
            .unwrap()
            .result
            .value
            .as_deref(),
        Some(b"v".as_slice())
    );
    registry.advance_clock(4).unwrap();
    assert!(registry
        .enqueue(&handle, &old, SurfaceId::Resp, command("get", 0))
        .is_err());
    let fresh = authenticated_epoch("tenant-a", "map", 1, 2, 4);
    assert_eq!(
        run(&registry, &handle, &fresh, command("get", 0))
            .unwrap()
            .result
            .kind,
        "present"
    );
    registry.enqueue_delete(&handle, &fresh).unwrap();
    registry.run_next().unwrap().unwrap().result.unwrap();
    for _ in 0..64 {
        registry.enqueue_reclaim(&handle, &fresh, 16).unwrap();
        registry.run_next().unwrap().unwrap().result.unwrap();
        if registry
            .diagnostics(&handle, &fresh)
            .unwrap()
            .owners
            .total()
            == 0
        {
            break;
        }
    }
    let next = registry.recreate(&handle, &fresh).unwrap();
    let recreated = authenticated_epoch("tenant-a", "map", 2, 2, 4);
    assert_eq!(
        run(&registry, &next, &recreated, command("get", 0))
            .unwrap()
            .result
            .kind,
        "absent"
    );
}

#[test]
fn clustered_subscription_cuts_preserve_metadata_and_hold_real_buffer_and_snapshot_leases() {
    let registry = registry(config());
    let (context, handle) = open(&registry, "tenant-a", "map");
    let options = ClusterSubscriptionOptions {
        listener: ServiceListenerOptions {
            include_value: true,
            max_value_bytes: 128,
            queue_capacity: 8,
            ..ServiceListenerOptions::default()
        },
        max_queued_events: 16,
        max_queued_bytes: 1024,
    };
    let source = registry
        .cluster_listener_source(&handle, &context, options)
        .unwrap();
    let id = source.subscription_id();
    let mut stream = DemandStream::new(source, 2).unwrap();
    run(&registry, &handle, &context, command("put", 1)).unwrap();
    let mut other = command("put", 2);
    other.key = vec![0, 0];
    run(&registry, &handle, &context, other).unwrap();
    let mut cx = Context::from_waker(Waker::noop());
    assert!(matches!(
        Pin::new(&mut stream).poll_next(&mut cx),
        Poll::Ready(Some(Ok(ClusterSubscriptionEvent::Entry { .. })))
    ));
    assert_eq!(stream.retained_items(), 1);
    let actual = registry.diagnostics(&handle, &context).unwrap();
    assert_eq!(actual.owners.streams, 1);
    assert!(actual.resources.streams >= 1024);
    let mut cut = registry
        .resync_cluster_listener(&handle, &context, id, 32, 512)
        .unwrap();
    assert_eq!(cut.cuts.len(), 2);
    assert_eq!(
        cut.cuts
            .iter()
            .map(|cut| cut.snapshot.entries.len())
            .sum::<usize>(),
        2
    );
    assert!(Pin::new(&mut stream).poll_next(&mut cx).is_pending());
    assert_eq!(stream.retained_items(), 0);
    registry.enqueue_delete(&handle, &context).unwrap();
    registry.run_next().unwrap().unwrap().result.unwrap();
    registry.enqueue_reclaim(&handle, &context, 64).unwrap();
    registry.run_next().unwrap().unwrap().result.unwrap();
    assert!(
        registry
            .diagnostics(&handle, &context)
            .unwrap()
            .owners
            .blocked_owners()
            >= 3
    );
    assert!(registry.recreate(&handle, &context).is_err());
    assert!(matches!(
        Pin::new(&mut stream).poll_next(&mut cx),
        Poll::Ready(Some(Err(DemandStreamError::Source(
            RegistryError::Namespace(_)
        ))))
    ));
    cut.close();
    registry.enqueue_reclaim(&handle, &context, 64).unwrap();
    registry.run_next().unwrap().unwrap().result.unwrap();
    assert_eq!(
        registry
            .diagnostics(&handle, &context)
            .unwrap()
            .owners
            .total(),
        0
    );
    assert_eq!(registry.usage().reserved_growth, 0);
}

struct ReentrantHealthWake {
    registry: UnifiedServiceRegistry,
    handle: RegistryHandle,
    context: AuthenticatedNamespace,
    wakes: AtomicUsize,
    blocked: AtomicBool,
}
impl std::task::Wake for ReentrantHealthWake {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.wakes.fetch_add(1, Ordering::SeqCst);
        let registry = self.registry.clone();
        let handle = self.handle.clone();
        let context = self.context.clone();
        let (send, receive) = std::sync::mpsc::sync_channel(1);
        // A bounded callback probe also catches an accidentally held registry metadata lock.
        std::thread::spawn(move || {
            let _ = send.send(registry.cluster_health(&handle, &context));
        });
        if !matches!(
            receive.recv_timeout(std::time::Duration::from_secs(2)),
            Ok(Ok(_))
        ) {
            self.blocked.store(true, Ordering::SeqCst);
        }
    }
}

#[test]
fn owner_down_and_promotion_wake_parked_sources_after_runtime_locks_are_released() {
    let registry = registry(config());
    let (context, handle) = open(&registry, "tenant-a", "map");
    run(&registry, &handle, &context, command("put", 1)).unwrap();
    let source = registry
        .cluster_listener_source(
            &handle,
            &context,
            ClusterSubscriptionOptions {
                listener: ServiceListenerOptions {
                    max_value_bytes: 128,
                    queue_capacity: 8,
                    ..ServiceListenerOptions::default()
                },
                max_queued_events: 16,
                max_queued_bytes: 1024,
            },
        )
        .unwrap();
    let mut stream = DemandStream::new(source, 2).unwrap();
    let observer = Arc::new(ReentrantHealthWake {
        registry: registry.clone(),
        handle: handle.clone(),
        context: context.clone(),
        wakes: AtomicUsize::new(0),
        blocked: AtomicBool::new(false),
    });
    let waker = Waker::from(Arc::clone(&observer));
    let mut cx = Context::from_waker(&waker);
    assert!(Pin::new(&mut stream).poll_next(&mut cx).is_pending());
    registry
        .enqueue_maintenance(
            &handle,
            &context,
            RuntimeMaintenance::SetNodeLive {
                node: "a".into(),
                live: false,
            },
        )
        .unwrap();
    registry.run_next().unwrap().unwrap().result.unwrap();
    assert!(observer.wakes.load(Ordering::SeqCst) > 0);
    assert!(!observer.blocked.load(Ordering::SeqCst));
    assert!(matches!(
        Pin::new(&mut stream).poll_next(&mut cx),
        Poll::Ready(Some(Ok(ClusterSubscriptionEvent::Gap { .. })))
    ));
    assert!(Pin::new(&mut stream).poll_next(&mut cx).is_pending());
    let before = observer.wakes.load(Ordering::SeqCst);
    registry
        .enqueue_maintenance(
            &handle,
            &context,
            RuntimeMaintenance::Promote { partition: 0 },
        )
        .unwrap();
    registry.run_next().unwrap().unwrap().result.unwrap();
    assert!(observer.wakes.load(Ordering::SeqCst) > before);
    assert!(!observer.blocked.load(Ordering::SeqCst));
    assert!(matches!(
        Pin::new(&mut stream).poll_next(&mut cx),
        Poll::Ready(Some(Ok(ClusterSubscriptionEvent::Gap { .. })))
    ));
    stream.cancel();
    assert_eq!(
        registry
            .diagnostics(&handle, &context)
            .unwrap()
            .owners
            .streams,
        0
    );
}

#[test]
fn local_registry_treats_typed_listener_expected_as_a_key_before_dispatch() {
    let narrow_values = SecurityBounds {
        max_value_bytes: 1,
        ..security()
    };
    let registry = UnifiedServiceRegistry::new(
        RegistryBounds::default(),
        collection(),
        ServiceLimits::default(),
        narrow_values,
        TrustEpochWindow::new(1, 3).unwrap(),
    )
    .unwrap();
    let (context, handle) = open(&registry, "tenant-a", "map");
    let mut listener = ServiceCommand::point("entry_listener", &[]);
    listener.expected = Some(vec![7; 32]);
    listener.limit = 4;
    registry
        .enqueue(&handle, &context, SurfaceId::Hc2Rust, listener.clone())
        .unwrap();
    assert!(
        matches!(registry.run_next().unwrap().unwrap().result.unwrap(), RegistryResult::Command(ServiceResult { kind, .. }) if kind == "listener_added")
    );
    let before = registry.diagnostics(&handle, &context).unwrap();
    listener.expected = Some(vec![7; 65]);
    assert!(matches!(
        registry.enqueue(&handle, &context, SurfaceId::Hc2Rust, listener),
        Err(RegistryError::Security(
            hydracache_cluster_testkit::value_plane_security_075::SecurityError::KeyTooLarge { .. }
        ))
    ));
    assert_eq!(registry.diagnostics(&handle, &context).unwrap(), before);
    let mut condition = ServiceCommand::point("conditional_remove", b"k");
    condition.expected = Some(vec![7; 2]);
    condition.identity = Some(MutationIdentity::new("client", 1));
    assert!(matches!(
        registry.enqueue(&handle, &context, SurfaceId::Hc2Rust, condition),
        Err(RegistryError::Security(
            hydracache_cluster_testkit::value_plane_security_075::SecurityError::ValueTooLarge { .. }
        ))
    ));
}
