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
    GrantKeys, GrantRights, NamespaceGrant, ReplayDisposition, ReplayWindow, RouteClaims,
    SecurityBounds, SecurityError, TrustEpochWindow,
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
        EnvelopeFault::AdmissionDigest,
        EnvelopeFault::GrantDigest,
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

#[test]
fn stepped_maintenance_retains_transfer_progress_under_repair_permits() {
    use hydracache_cluster_testkit::value_plane_loopback_075::LoopbackFault;
    let registry = registry(config());
    let (context, handle) = open(&registry, "tenant-a", "stepped");
    let first = run(&registry, &handle, &context, command("put", 1)).unwrap();
    let partition = first.routes[0].partition;
    let before = registry
        .cluster_health(&handle, &context)
        .unwrap()
        .partitions[partition as usize]
        .clone();
    let mut transfer = None;
    let mut repaired = false;
    let mut wrote = false;
    for index in 0..64 {
        registry
            .enqueue_maintenance(
                &handle,
                &context,
                RuntimeMaintenance::RepairStep {
                    partition,
                    backup: "c".into(),
                    faults: if index == 0 {
                        ServiceLinkFaults {
                            ack: LoopbackFault::Drop,
                            ..ServiceLinkFaults::default()
                        }
                    } else {
                        ServiceLinkFaults::default()
                    },
                    maximum_work: 1,
                },
            )
            .unwrap();
        let RegistryResult::Maintenance(MaintenanceResult::Online(step)) =
            registry.run_next().unwrap().unwrap().result.unwrap()
        else {
            panic!("online repair result")
        };
        assert!(step.work <= 1);
        if let Some(id) = transfer {
            assert_eq!(step.transfer, Some(id));
        } else {
            transfer = step.transfer;
        }
        let health = registry.cluster_health(&handle, &context).unwrap();
        assert_eq!(health.partitions[partition as usize].epoch, before.epoch);
        if !step.complete {
            assert!(health.reasons.contains(&ReadinessReason::TransferPending));
        }
        let admission = registry.cluster_admission_observations().unwrap();
        assert_eq!(admission.inflight, [0; 3]);
        assert_eq!(admission.owned_bytes, [0; 3]);
        if index == 1 {
            let mut put = command("put", 2);
            put.value = Some(b"foreground".to_vec());
            put.ttl = ServiceTtl::ExpireAfter(20);
            assert!(
                run(&registry, &handle, &context, put)
                    .unwrap()
                    .backup_proved
            );
            wrote = true;
        }
        if step.complete {
            assert!(health.partitions[partition as usize].backup_proved);
            repaired = true;
            break;
        }
    }
    assert!(transfer.is_some() && repaired && wrote);
    registry
        .enqueue_maintenance(&handle, &context, RuntimeMaintenance::Promote { partition })
        .unwrap();
    registry.run_next().unwrap().unwrap().result.unwrap();
    assert_eq!(
        run(&registry, &handle, &context, command("get", 0))
            .unwrap()
            .result
            .value
            .as_deref(),
        Some(b"foreground".as_slice())
    );
    registry
        .enqueue_maintenance(
            &handle,
            &context,
            RuntimeMaintenance::BeginDrain { node: "b".into() },
        )
        .unwrap();
    registry.run_next().unwrap().unwrap().result.unwrap();
    let mut drained = false;
    for _ in 0..128 {
        registry
            .enqueue_maintenance(
                &handle,
                &context,
                RuntimeMaintenance::DrainOnlineStep {
                    node: "b".into(),
                    maximum_partitions: 1,
                    maximum_reclaim: 1,
                    maximum_transfer_work: 1,
                },
            )
            .unwrap();
        let RegistryResult::Maintenance(MaintenanceResult::Drain(progress)) =
            registry.run_next().unwrap().unwrap().result.unwrap()
        else {
            panic!("online drain result")
        };
        assert!(progress.moved <= 1 && progress.reclaimed <= 1);
        assert_eq!(
            registry.cluster_admission_observations().unwrap().inflight,
            [0; 3]
        );
        if progress.complete {
            drained = true;
            break;
        }
    }
    assert!(drained);
    assert_eq!(registry.usage().reserved_growth, 0);
    assert_eq!(
        run(&registry, &handle, &context, command("get", 0))
            .unwrap()
            .result
            .value
            .as_deref(),
        Some(b"foreground".as_slice())
    );
    for operation in [
        RuntimeMaintenance::RepairStep {
            partition,
            backup: "c".into(),
            faults: ServiceLinkFaults::default(),
            maximum_work: 0,
        },
        RuntimeMaintenance::DrainOnlineStep {
            node: "b".into(),
            maximum_partitions: 1,
            maximum_reclaim: 1,
            maximum_transfer_work: 0,
        },
    ] {
        registry
            .enqueue_maintenance(&handle, &context, operation)
            .unwrap();
        assert!(registry.run_next().unwrap().unwrap().result.is_err());
        assert_eq!(
            registry.cluster_admission_observations().unwrap().inflight,
            [0; 3]
        );
    }
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

fn granted(epoch: u64, rights: GrantRights, keys: GrantKeys) -> AuthenticatedNamespace {
    authenticated("tenant-a", "map", 1)
        .with_grant(NamespaceGrant::new("tenant-a", "map", 1, epoch, rights, keys).unwrap())
        .unwrap()
}

#[test]
fn replay_window_retires_monotonically_and_never_revives_old_or_conflicting_nonces() {
    let mut window = ReplayWindow::new(3, 2).unwrap();
    assert_eq!(
        window.admit("a:1:1", 3, b"three").unwrap(),
        ReplayDisposition::New
    );
    assert_eq!(
        window.admit("a:1:1", 1, b"one").unwrap(),
        ReplayDisposition::New
    );
    assert_eq!(
        window.admit("a:1:1", 2, b"two").unwrap(),
        ReplayDisposition::New
    );
    assert_eq!(
        window.admit("a:1:1", 2, b"two").unwrap(),
        ReplayDisposition::ExactReplay
    );
    assert_eq!(
        window.admit("a:1:1", 2, b"wrong"),
        Err(SecurityError::ReplayDigestConflict)
    );
    for nonce in 4u64..128 {
        window.admit("a:1:1", nonce, &nonce.to_le_bytes()).unwrap();
        assert!(window.observations().retained <= 3);
    }
    assert_eq!(
        window.admit("a:1:1", 1, b"one"),
        Err(SecurityError::ReplayTooOld)
    );
    window.admit("b:1:1", 200, b"b").unwrap();
    assert_eq!(
        window.admit("c:1:1", 201, b"c"),
        Err(SecurityError::ReplayCapacityExhausted)
    );
    assert!(window.observations().retired > 100);
    assert_eq!(window.observations().lanes, 2);
    assert!(ReplayWindow::new(0, 1).is_err());
    assert!(ReplayWindow::new(1, 129).is_err());
    assert!(window.admit("", 1, b"").is_err());
}

#[test]
fn admission_receipt_keeps_credits_until_every_attempt_owner_drops() {
    let scheduler = ClusterAdmission::new(ClusterAdmissionBounds::default()).unwrap();
    scheduler
        .admit(ClusterWork {
            tenant: "tenant-a".into(),
            partition: 0,
            key: b"k".to_vec(),
            lane: ClusterLane::Data,
            bytes: 64,
            replica_bytes: 128,
        })
        .unwrap();
    let permit = scheduler.pop().unwrap().unwrap();
    let binding = AdmissionBinding {
        namespace: "map".into(),
        namespace_generation: 1,
        request_digest: [1; 32],
        identity: Some(MutationIdentity::new("client", 1)),
        deadline: 100,
        grant_digest: None,
    };
    let attempt = permit.begin_invocation(binding.clone()).unwrap();
    assert!(matches!(
        permit.begin_invocation(binding.clone()),
        Err(ClusterAdmissionError::InvocationInFlight)
    ));
    let mut other = binding.clone();
    other.namespace_generation = 2;
    assert!(matches!(
        permit.begin_invocation(other),
        Err(ClusterAdmissionError::ReceiptConflict)
    ));
    drop(attempt);
    let attempt = permit.begin_invocation(binding).unwrap();
    drop(permit);
    assert_eq!(scheduler.observations().replica_bytes, 128);
    assert_eq!(scheduler.observations().released, 0);
    drop(attempt);
    assert_eq!(scheduler.observations().replica_bytes, 0);
    assert_eq!(scheduler.observations().released, 1);
}

#[test]
fn actual_runtime_receipt_rejects_changed_mutation_and_allows_identical_unknown_retry() {
    let context = authenticated("tenant-a", "map", 1);
    let runtime = ClusterNamespaceRuntime::new(
        config(),
        context.address(),
        collection(),
        ServiceLimits::default(),
        security(),
        TrustEpochWindow::new(1, 3).unwrap(),
    )
    .unwrap();
    let mut put = command("put", 1);
    put.lose_response = true;
    let scheduler = ClusterAdmission::new(ClusterAdmissionBounds {
        max_replica_bytes: 64 * 1024 * 1024,
        ..ClusterAdmissionBounds::default()
    })
    .unwrap();
    scheduler
        .admit_for_resources(
            ClusterWork {
                tenant: "tenant-a".into(),
                partition: runtime.partition_for(&put.key),
                key: put.key.clone(),
                lane: ClusterLane::Data,
                bytes: 4096,
                replica_bytes: runtime.command_replica_bound(&put).unwrap() * 2,
            },
            &runtime.affected_partitions(&put),
            &[put.key.clone()],
        )
        .unwrap();
    let permit = scheduler.pop().unwrap().unwrap();
    runtime
        .execute(&context, &permit, put.clone(), None)
        .unwrap();
    let mut changed = put.clone();
    changed.value = Some(b"other".to_vec());
    assert_eq!(
        runtime
            .execute(&context, &permit, changed, None)
            .unwrap_err(),
        ServiceClusterError::Authorization("admission_receipt")
    );
    put.lose_response = false;
    let replay = runtime.execute(&context, &permit, put, None).unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.result.kind, "inserted");
    assert!(replay.backup_proved);
    drop(permit);
    assert_eq!(scheduler.observations().released, 1);
}

#[test]
fn immutable_grants_fail_closed_before_bulk_proxy_and_privilege_amplification() {
    let registry = registry(config());
    registry.advance_policy_epoch(1).unwrap();
    let legacy = authenticated("tenant-a", "map", 1);
    assert!(registry.open(&legacy, legacy.address()).is_err());
    let admin = granted(1, GrantRights::ALL, GrantKeys::All);
    let handle = registry.open(&admin, admin.address()).unwrap();
    let reader = admin
        .attenuate(
            GrantRights {
                read: true,
                ..GrantRights::default()
            },
            GrantKeys::Prefixes(vec![b"a".to_vec()]),
        )
        .unwrap();
    assert!(reader.attenuate(GrantRights::ALL, GrantKeys::All).is_err());
    let mut read = command("get", 0);
    read.key = b"allowed".to_vec();
    assert_eq!(
        run(&registry, &handle, &reader, read).unwrap().result.kind,
        "absent"
    );
    assert!(registry
        .enqueue(&handle, &reader, SurfaceId::Resp, command("put", 1))
        .is_err());
    let writer = admin
        .attenuate(
            GrantRights {
                write: true,
                ..GrantRights::default()
            },
            GrantKeys::Prefixes(vec![b"a".to_vec()]),
        )
        .unwrap();
    let mut bulk = command("put_all_detailed", 1);
    bulk.entries = vec![
        hydracache_cluster_testkit::value_plane_collection_075::ReplacementEntry {
            key: b"a".to_vec(),
            value: b"1".to_vec(),
        },
        hydracache_cluster_testkit::value_plane_collection_075::ReplacementEntry {
            key: b"b".to_vec(),
            value: b"2".to_vec(),
        },
    ];
    let before = registry.diagnostics(&handle, &admin).unwrap();
    assert!(registry
        .enqueue_cluster(
            &handle,
            &writer,
            SurfaceId::Hc2Java,
            bulk,
            RuntimeCallOptions::default()
        )
        .is_err());
    assert_eq!(registry.diagnostics(&handle, &admin).unwrap(), before);
    assert!(registry.enqueue_delete(&handle, &writer).is_err());
    assert!(NamespaceGrant::new(
        "t",
        "n",
        1,
        1,
        GrantRights::ALL,
        GrantKeys::Prefixes(vec![vec![0]; 17])
    )
    .is_err());
    let wrong =
        NamespaceGrant::new("other", "map", 1, 1, GrantRights::ALL, GrantKeys::All).unwrap();
    assert!(legacy.with_grant(wrong).is_err());
}

#[test]
fn policy_revocation_discards_queued_writes_and_buffered_subscription_values() {
    let registry = registry(config());
    registry.advance_policy_epoch(1).unwrap();
    let admin = granted(1, GrantRights::ALL, GrantKeys::All);
    let handle = registry.open(&admin, admin.address()).unwrap();
    let source = registry
        .cluster_listener_source(
            &handle,
            &admin,
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
    run(&registry, &handle, &admin, command("put", 1)).unwrap();
    let mut next = command("put", 2);
    next.key = vec![0, 0];
    run(&registry, &handle, &admin, next).unwrap();
    let mut cx = Context::from_waker(Waker::noop());
    assert!(Pin::new(&mut stream).poll_next(&mut cx).is_ready());
    assert_eq!(stream.retained_items(), 1);
    registry
        .enqueue(&handle, &admin, SurfaceId::Hc1, command("put", 3))
        .unwrap();
    registry.advance_policy_epoch(2).unwrap();
    assert_eq!(registry.usage().queued_requests, 0);
    assert_eq!(registry.usage().reserved_growth, 0);
    assert!(registry.run_next().unwrap().is_none());
    assert!(matches!(
        Pin::new(&mut stream).poll_next(&mut cx),
        Poll::Ready(Some(Err(DemandStreamError::Source(
            RegistryError::Security(SecurityError::PolicyEpochMismatch)
        ))))
    ));
    assert_eq!(stream.retained_items(), 0);
    let current = granted(2, GrantRights::ALL, GrantKeys::All);
    assert_eq!(
        registry
            .diagnostics(&handle, &current)
            .unwrap()
            .owners
            .streams,
        0
    );
    assert!(registry.advance_policy_epoch(2).is_err());
}

#[test]
fn cluster_invocation_cancellation_has_distinct_pre_admission_and_replicated_reservation_states() {
    use hydracache_cluster_testkit::value_plane_async_075::{
        AsyncOutcome, CancellationDisposition, InvocationRejection,
    };
    use std::future::Future;
    let registry = registry(config());
    let (context, handle) = open(&registry, "tenant-a", "map");
    let mut cx = Context::from_waker(Waker::noop());
    let (mut before, cancel) = registry
        .cluster_invocation(
            &handle,
            &context,
            SurfaceId::Resp,
            command("put", 1),
            RuntimeCallOptions::default(),
        )
        .unwrap();
    assert_eq!(cancel.cancel(), CancellationDisposition::NotApplied);
    assert!(matches!(
        Pin::new(&mut before).poll(&mut cx),
        Poll::Ready(AsyncOutcome::NotApplied(InvocationRejection::Cancelled))
    ));
    assert_eq!(
        registry
            .diagnostics(&handle, &context)
            .unwrap()
            .owners
            .total(),
        0
    );
    let (mut after, cancel) = registry
        .cluster_invocation(
            &handle,
            &context,
            SurfaceId::Resp,
            command("put", 2),
            RuntimeCallOptions::default(),
        )
        .unwrap();
    assert!(Pin::new(&mut after).poll(&mut cx).is_pending());
    assert_eq!(cancel.phase(), ClusterInvocationPhase::Claimed);
    let reserved = Pin::new(&mut after).poll(&mut cx);
    assert!(
        reserved.is_pending(),
        "{reserved:?}; error={:?}",
        cancel.last_error()
    );
    assert_eq!(cancel.phase(), ClusterInvocationPhase::Reserved);
    assert_eq!(
        registry
            .diagnostics(&handle, &context)
            .unwrap()
            .owners
            .pending,
        2
    );
    assert_eq!(cancel.cancel(), CancellationDisposition::OutcomeUnknown);
    assert!(matches!(
        Pin::new(&mut after).poll(&mut cx),
        Poll::Ready(AsyncOutcome::OutcomeUnknown)
    ));
    assert_eq!(cancel.cancel(), CancellationDisposition::AlreadyFinished);
    assert_eq!(
        registry
            .diagnostics(&handle, &context)
            .unwrap()
            .owners
            .pending,
        0
    );
    assert_eq!(
        registry
            .diagnostics(&handle, &context)
            .unwrap()
            .owners
            .values,
        0
    );
    assert_eq!(registry.usage().reserved_growth, 0);
    assert_eq!(
        registry.cluster_admission_observations().unwrap().inflight,
        [0; 3]
    );
}

#[test]
fn cluster_invocation_completes_once_and_cancelled_retry_recovers_the_actual_receipt() {
    use hydracache_cluster_testkit::value_plane_async_075::AsyncOutcome;
    use std::future::Future;
    let registry = registry(config());
    let (context, handle) = open(&registry, "tenant-a", "map");
    let mut cx = Context::from_waker(Waker::noop());
    let (mut call, control) = registry
        .cluster_invocation(
            &handle,
            &context,
            SurfaceId::Hc2Rust,
            command("put", 1),
            RuntimeCallOptions::default(),
        )
        .unwrap();
    assert!(registry.run_next().unwrap().is_none());
    assert!(Pin::new(&mut call).poll(&mut cx).is_pending());
    let reserved = Pin::new(&mut call).poll(&mut cx);
    assert!(
        reserved.is_pending(),
        "{reserved:?}; error={:?}",
        control.last_error()
    );
    assert!(matches!(
        Pin::new(&mut call).poll(&mut cx),
        Poll::Ready(AsyncOutcome::Completed(Ok(ServiceClusterExecution {
            backup_proved: true,
            ..
        })))
    ));
    assert_eq!(control.phase(), ClusterInvocationPhase::Finished);
    let before = registry.diagnostics(&handle, &context).unwrap();
    let (mut retry, cancel) = registry
        .cluster_invocation(
            &handle,
            &context,
            SurfaceId::Hc2Java,
            command("put", 1),
            RuntimeCallOptions::default(),
        )
        .unwrap();
    cancel.cancel();
    assert!(matches!(
        Pin::new(&mut retry).poll(&mut cx),
        Poll::Ready(AsyncOutcome::Completed(Ok(ServiceClusterExecution {
            replayed: true,
            ..
        })))
    ));
    assert_eq!(registry.diagnostics(&handle, &context).unwrap(), before);
    assert_eq!(registry.usage().queued_requests, 0);
    assert_eq!(registry.usage().reserved_growth, 0);
}

#[test]
fn cooperative_invocation_drop_wakes_fair_waiters_outside_locks_and_deadlines_release_credits() {
    use hydracache_cluster_testkit::value_plane_async_075::{AsyncOutcome, InvocationRejection};
    use std::future::Future;
    let registry = registry(config());
    let (context, handle) = open(&registry, "tenant-a", "map");
    let observer = Arc::new(ReentrantHealthWake {
        registry: registry.clone(),
        handle: handle.clone(),
        context: context.clone(),
        wakes: AtomicUsize::new(0),
        blocked: AtomicBool::new(false),
    });
    let waker = Waker::from(Arc::clone(&observer));
    let mut cx = Context::from_waker(&waker);
    let (mut first, _) = registry
        .cluster_invocation(
            &handle,
            &context,
            SurfaceId::Resp,
            command("put", 1),
            RuntimeCallOptions::default(),
        )
        .unwrap();
    let (mut second, control) = registry
        .cluster_invocation(
            &handle,
            &context,
            SurfaceId::Resp,
            command("put", 2),
            RuntimeCallOptions::default(),
        )
        .unwrap();
    assert!(Pin::new(&mut first).poll(&mut cx).is_pending());
    assert!(Pin::new(&mut second).poll(&mut cx).is_pending());
    let before = observer.wakes.load(Ordering::SeqCst);
    drop(first);
    assert!(observer.wakes.load(Ordering::SeqCst) > before);
    assert!(!observer.blocked.load(Ordering::SeqCst));
    assert!(Pin::new(&mut second).poll(&mut cx).is_pending());
    let reserved = Pin::new(&mut second).poll(&mut cx);
    assert!(
        reserved.is_pending(),
        "{reserved:?} {:?}",
        control.last_error()
    );
    drop(second);
    assert_eq!(
        registry
            .diagnostics(&handle, &context)
            .unwrap()
            .owners
            .pending,
        0
    );
    assert_eq!(
        registry
            .diagnostics(&handle, &context)
            .unwrap()
            .owners
            .values,
        0
    );
    let mut late = command("put", 3);
    late.deadline = 1;
    let (mut late, _) = registry
        .cluster_invocation(
            &handle,
            &context,
            SurfaceId::Resp,
            late,
            RuntimeCallOptions::default(),
        )
        .unwrap();
    registry.advance_clock(1).unwrap();
    assert!(matches!(
        Pin::new(&mut late).poll(&mut cx),
        Poll::Ready(AsyncOutcome::NotApplied(InvocationRejection::Deadline))
    ));
    assert!(!observer.blocked.load(Ordering::SeqCst));
    assert_eq!(registry.usage().reserved_growth, 0);
    assert_eq!(registry.usage().queued_requests, 0);
    assert_eq!(
        registry.cluster_admission_observations().unwrap().released,
        3
    );
}

#[test]
fn revoking_a_reserved_invocation_keeps_terminal_unknown_and_releases_real_pending_owners() {
    use hydracache_cluster_testkit::value_plane_async_075::AsyncOutcome;
    use std::future::Future;
    let registry = registry(config());
    registry.advance_policy_epoch(1).unwrap();
    let admin = granted(1, GrantRights::ALL, GrantKeys::All);
    let handle = registry.open(&admin, admin.address()).unwrap();
    let mut cx = Context::from_waker(Waker::noop());
    let (mut invocation, control) = registry
        .cluster_invocation(
            &handle,
            &admin,
            SurfaceId::Hc1,
            command("put", 1),
            RuntimeCallOptions::default(),
        )
        .unwrap();
    assert!(Pin::new(&mut invocation).poll(&mut cx).is_pending());
    let reserved = Pin::new(&mut invocation).poll(&mut cx);
    assert!(
        reserved.is_pending(),
        "{reserved:?} {:?}",
        control.last_error()
    );
    registry.advance_policy_epoch(2).unwrap();
    assert!(registry.usage().reserved_growth > 0);
    assert!(matches!(
        Pin::new(&mut invocation).poll(&mut cx),
        Poll::Ready(AsyncOutcome::OutcomeUnknown)
    ));
    let current = granted(2, GrantRights::ALL, GrantKeys::All);
    assert_eq!(
        registry
            .diagnostics(&handle, &current)
            .unwrap()
            .owners
            .pending,
        0
    );
    assert_eq!(
        registry
            .diagnostics(&handle, &current)
            .unwrap()
            .owners
            .values,
        0
    );
    let mut outcome = command("query_operation_outcome", 0);
    outcome.identity = Some(MutationIdentity::new("client", 1));
    assert_eq!(
        run(&registry, &handle, &current, outcome)
            .unwrap()
            .result
            .kind,
        "completed"
    );
    assert_eq!(registry.usage().reserved_growth, 0);
    assert_eq!(
        registry.cluster_admission_observations().unwrap().inflight,
        [0; 3]
    );
}

#[test]
fn actual_authentication_survives_window_turnover_and_health_reports_bounded_replay_owners() {
    let mut cfg = config();
    cfg.max_replays = 3;
    let registry = registry(cfg);
    let (context, handle) = open(&registry, "tenant-a", "map");
    for sequence in 1..33 {
        run(&registry, &handle, &context, command("put", sequence)).unwrap();
    }
    let health = registry.cluster_health(&handle, &context).unwrap();
    assert!(health.authentication_ready);
    assert!(health.replay.retired > 32);
    assert!(health.replay.retained <= 3);
    assert!(health.replay.lanes <= 48);
    registry
        .rotate_cluster_credentials(2, rotated_members())
        .unwrap();
    registry.advance_clock(4).unwrap();
    let current = authenticated_epoch("tenant-a", "map", 1, 2, 4);
    run(&registry, &handle, &current, command("get", 0)).unwrap();
    assert!(
        registry
            .cluster_health(&handle, &current)
            .unwrap()
            .authentication_ready
    );
}

#[test]
fn scoped_listener_grants_require_read_for_values_and_cannot_widen_the_filter() {
    let registry = registry(config());
    registry.advance_policy_epoch(1).unwrap();
    let admin = granted(1, GrantRights::ALL, GrantKeys::All);
    let handle = registry.open(&admin, admin.address()).unwrap();
    let listener = admin
        .attenuate(
            GrantRights {
                listen: true,
                ..GrantRights::default()
            },
            GrantKeys::Prefixes(vec![b"a".to_vec()]),
        )
        .unwrap();
    let mut options = ClusterSubscriptionOptions {
        listener: ServiceListenerOptions {
            max_value_bytes: 128,
            queue_capacity: 8,
            ..ServiceListenerOptions::default()
        },
        max_queued_events: 16,
        max_queued_bytes: 1024,
    };
    assert!(registry
        .cluster_listener_source(&handle, &listener, options.clone())
        .is_err());
    options.listener.exact_key = Some(b"a".to_vec());
    options.listener.include_value = true;
    assert!(registry
        .cluster_listener_source(&handle, &listener, options.clone())
        .is_err());
    options.listener.include_value = false;
    options.listener.exact_key = Some(b"b".to_vec());
    assert!(registry
        .cluster_listener_source(&handle, &listener, options.clone())
        .is_err());
    options.listener.exact_key = Some(b"a".to_vec());
    let source = registry
        .cluster_listener_source(&handle, &listener, options)
        .unwrap();
    let mut stream = DemandStream::new(source, 2).unwrap();
    let mut a = command("put", 1);
    a.key = b"a".to_vec();
    run(&registry, &handle, &admin, a).unwrap();
    let mut b = command("put", 2);
    b.key = b"b".to_vec();
    run(&registry, &handle, &admin, b).unwrap();
    let mut cx = Context::from_waker(Waker::noop());
    match Pin::new(&mut stream).poll_next(&mut cx) {
        Poll::Ready(Some(Ok(ClusterSubscriptionEvent::Entry { event, .. }))) => {
            assert_eq!(event.key, b"a");
            assert!(event.value.is_none());
            assert!(event.old_value.is_none());
        }
        other => panic!("unexpected scoped event {other:?}"),
    }
    assert!(Pin::new(&mut stream).poll_next(&mut cx).is_pending());
    stream.cancel();
    assert_eq!(
        registry
            .diagnostics(&handle, &admin)
            .unwrap()
            .owners
            .streams,
        0
    );
}

#[test]
fn lost_response_invocation_retry_recovers_the_actual_decision_without_reexecution() {
    use hydracache_cluster_testkit::value_plane_async_075::AsyncOutcome;
    use std::future::Future;
    let registry = registry(config());
    let (context, handle) = open(&registry, "tenant-a", "map");
    let mut cx = Context::from_waker(Waker::noop());
    let mut lost = command("put", 1);
    lost.lose_response = true;
    let (mut invocation, control) = registry
        .cluster_invocation(
            &handle,
            &context,
            SurfaceId::Resp,
            lost,
            RuntimeCallOptions::default(),
        )
        .unwrap();
    assert!(Pin::new(&mut invocation).poll(&mut cx).is_pending());
    let reserved = Pin::new(&mut invocation).poll(&mut cx);
    assert!(
        reserved.is_pending(),
        "{reserved:?} {:?}",
        control.last_error()
    );
    assert!(matches!(
        Pin::new(&mut invocation).poll(&mut cx),
        Poll::Ready(AsyncOutcome::OutcomeUnknown)
    ));
    let before = registry.diagnostics(&handle, &context).unwrap();
    assert_eq!(before.owners.values, 2);
    assert_eq!(before.owners.pending, 0);
    let (mut retry, control) = registry
        .cluster_invocation(
            &handle,
            &context,
            SurfaceId::Hc2Java,
            command("put", 1),
            RuntimeCallOptions::default(),
        )
        .unwrap();
    assert!(Pin::new(&mut retry).poll(&mut cx).is_pending());
    let reserved = Pin::new(&mut retry).poll(&mut cx);
    assert!(
        reserved.is_pending(),
        "{reserved:?} {:?}",
        control.last_error()
    );
    assert!(matches!(
        Pin::new(&mut retry).poll(&mut cx),
        Poll::Ready(AsyncOutcome::Completed(Ok(ServiceClusterExecution {
            replayed: true,
            backup_proved: true,
            ..
        })))
    ));
    assert_eq!(registry.diagnostics(&handle, &context).unwrap(), before);
    assert_eq!(registry.usage().reserved_growth, 0);
}

#[test]
fn forged_reservation_fails_before_root_owners_and_invalid_async_calls_never_queue() {
    use hydracache_cluster_testkit::value_plane_async_075::AsyncOutcome;
    use std::future::Future;
    let registry = registry(config());
    let (context, handle) = open(&registry, "tenant-a", "map");
    let mut cx = Context::from_waker(Waker::noop());
    assert!(registry
        .cluster_invocation(
            &handle,
            &context,
            SurfaceId::Resp,
            command("get", 0),
            RuntimeCallOptions::default()
        )
        .is_err());
    assert!(registry
        .cluster_invocation(
            &handle,
            &context,
            SurfaceId::Resp,
            command("put", 0),
            RuntimeCallOptions::default()
        )
        .is_err());
    assert!(registry
        .cluster_invocation(
            &handle,
            &context,
            SurfaceId::Resp,
            command("destroy", 1),
            RuntimeCallOptions::default()
        )
        .is_err());
    let before = registry.diagnostics(&handle, &context).unwrap();
    let (mut invocation, control) = registry
        .cluster_invocation(
            &handle,
            &context,
            SurfaceId::Resp,
            command("put", 1),
            RuntimeCallOptions {
                auth_fault: Some(AuthFault {
                    stage: ServiceMessageKind::Request,
                    fault: EnvelopeFault::AdmissionDigest,
                }),
                ..RuntimeCallOptions::default()
            },
        )
        .unwrap();
    assert!(Pin::new(&mut invocation).poll(&mut cx).is_pending());
    assert!(matches!(
        Pin::new(&mut invocation).poll(&mut cx),
        Poll::Ready(AsyncOutcome::OutcomeUnknown)
    ));
    assert_eq!(
        control.last_error(),
        Some(RegistryError::Cluster(ServiceClusterError::Authorization(
            "receipt_binding"
        )))
    );
    assert_eq!(registry.diagnostics(&handle, &context).unwrap(), before);
    assert_eq!(registry.usage().queued_requests, 0);
    assert_eq!(registry.usage().reserved_growth, 0);
    assert_eq!(
        registry
            .cluster_admission_observations()
            .unwrap()
            .replica_bytes,
        0
    );
}

#[test]
fn write_only_grants_cannot_extract_prior_values_from_mutation_results() {
    let registry = registry(config());
    registry.advance_policy_epoch(1).unwrap();
    let admin = granted(1, GrantRights::ALL, GrantKeys::All);
    let handle = registry.open(&admin, admin.address()).unwrap();
    let writer = admin
        .attenuate(
            GrantRights {
                write: true,
                ..GrantRights::default()
            },
            GrantKeys::Prefixes(vec![b"a".to_vec()]),
        )
        .unwrap();
    let mut initial = command("put", 1);
    initial.key = b"a".to_vec();
    initial.value = Some(b"private".to_vec());
    run(&registry, &handle, &admin, initial).unwrap();
    let before = registry.diagnostics(&handle, &admin).unwrap();
    for operation in [
        "get_and_put",
        "get_and_remove",
        "replace_if_present",
        "conditional_replace",
        "conditional_remove",
        "replace_if_version",
        "put_if_absent",
        "remove_all_detailed",
    ] {
        let mut denied = command(operation, 2);
        denied.key = b"a".to_vec();
        denied.keys = vec![b"a".to_vec()];
        assert!(matches!(
            registry.enqueue(&handle, &writer, SurfaceId::Resp, denied),
            Err(RegistryError::Security(SecurityError::PermissionDenied))
        ));
    }
    assert_eq!(registry.diagnostics(&handle, &admin).unwrap(), before);
    let mut overwrite = command("put", 2);
    overwrite.key = b"a".to_vec();
    overwrite.value = Some(b"public".to_vec());
    assert!(run(&registry, &handle, &writer, overwrite)
        .unwrap()
        .result
        .value
        .is_none());
}

struct PollOtherInvocationWake {
    future: Arc<Mutex<Option<RegistryClusterInvocation>>>,
    pumping: AtomicBool,
    blocked: AtomicBool,
    terminal_error: AtomicBool,
}
impl std::task::Wake for PollOtherInvocationWake {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        if self.pumping.swap(true, Ordering::SeqCst) {
            return;
        }
        let observer = Arc::clone(self);
        let (send, receive) = std::sync::mpsc::sync_channel(1);
        std::thread::spawn(move || {
            use hydracache_cluster_testkit::value_plane_async_075::AsyncOutcome;
            use std::future::Future;
            let waker = Waker::from(Arc::clone(&observer));
            let mut cx = Context::from_waker(&waker);
            let mut future = observer.future.lock().unwrap();
            if let Some(future) = future.as_mut() {
                if matches!(
                    Pin::new(future).poll(&mut cx),
                    Poll::Ready(AsyncOutcome::Completed(Err(_)))
                ) {
                    observer.terminal_error.store(true, Ordering::SeqCst);
                }
            }
            let _ = send.send(());
        });
        if receive
            .recv_timeout(std::time::Duration::from_secs(2))
            .is_err()
        {
            self.blocked.store(true, Ordering::SeqCst);
        }
        self.pumping.store(false, Ordering::SeqCst);
    }
}

#[test]
fn completion_wakes_another_actual_invocation_after_dispatch_guard_is_released() {
    use hydracache_cluster_testkit::value_plane_async_075::AsyncOutcome;
    use std::future::Future;
    let registry = registry(config());
    let (context, handle) = open(&registry, "tenant-a", "map");
    let (mut first, control) = registry
        .cluster_invocation(
            &handle,
            &context,
            SurfaceId::Resp,
            command("put", 1),
            RuntimeCallOptions::default(),
        )
        .unwrap();
    let (second, second_control) = registry
        .cluster_invocation(
            &handle,
            &context,
            SurfaceId::Resp,
            command("put", 2),
            RuntimeCallOptions::default(),
        )
        .unwrap();
    let future = Arc::new(Mutex::new(Some(second)));
    let observer = Arc::new(PollOtherInvocationWake {
        future: Arc::clone(&future),
        pumping: AtomicBool::new(false),
        blocked: AtomicBool::new(false),
        terminal_error: AtomicBool::new(false),
    });
    let waker = Waker::from(Arc::clone(&observer));
    let mut cx = Context::from_waker(&waker);
    let mut noop = Context::from_waker(Waker::noop());
    assert!(Pin::new(&mut first).poll(&mut noop).is_pending());
    assert!(Pin::new(future.lock().unwrap().as_mut().unwrap())
        .poll(&mut cx)
        .is_pending());
    let reserved = Pin::new(&mut first).poll(&mut noop);
    assert!(
        reserved.is_pending(),
        "{reserved:?} {:?}",
        control.last_error()
    );
    assert!(matches!(
        Pin::new(&mut first).poll(&mut noop),
        Poll::Ready(AsyncOutcome::Completed(Ok(_)))
    ));
    assert!(!observer.blocked.load(Ordering::SeqCst));
    assert!(!observer.terminal_error.load(Ordering::SeqCst));
    assert_eq!(second_control.phase(), ClusterInvocationPhase::Claimed);
    drop(future.lock().unwrap().take());
    assert_eq!(registry.usage().queued_requests, 0);
    assert_eq!(registry.usage().reserved_growth, 0);
}

#[test]
fn cancelled_retained_retry_cannot_turn_a_missing_coordinator_ack_into_success() {
    use hydracache_cluster_testkit::value_plane_async_075::AsyncOutcome;
    use hydracache_cluster_testkit::value_plane_loopback_075::LoopbackFault;
    use std::future::Future;
    let registry = registry(config());
    let (context, handle) = open(&registry, "tenant-a", "map");
    let mut call = ServiceClusterCall::default();
    call.faults.coordinator.ack = LoopbackFault::Drop;
    registry
        .enqueue_cluster(
            &handle,
            &context,
            SurfaceId::Resp,
            command("put", 1),
            RuntimeCallOptions {
                call,
                ..RuntimeCallOptions::default()
            },
        )
        .unwrap();
    let RegistryResult::ClusterCommand(execution) =
        registry.run_next().unwrap().unwrap().result.unwrap()
    else {
        panic!("expected actual cluster execution");
    };
    assert!(!execution.backup_proved);
    let before = registry.diagnostics(&handle, &context).unwrap();
    assert_eq!(before.owners.values, 2);
    let (mut retry, cancel) = registry
        .cluster_invocation(
            &handle,
            &context,
            SurfaceId::Hc2Java,
            command("put", 1),
            RuntimeCallOptions::default(),
        )
        .unwrap();
    cancel.cancel();
    let mut cx = Context::from_waker(Waker::noop());
    assert!(matches!(
        Pin::new(&mut retry).poll(&mut cx),
        Poll::Ready(AsyncOutcome::OutcomeUnknown)
    ));
    assert_eq!(registry.diagnostics(&handle, &context).unwrap(), before);
    assert_eq!(registry.usage().reserved_growth, 0);
}

fn run_typed_helper(
    registry: &UnifiedServiceRegistry,
    handle: &RegistryHandle,
    context: &AuthenticatedNamespace,
    request: ClusterHelperRequest,
) -> Result<ClusterHelperExecution, RegistryError> {
    let ticket = registry.enqueue_helper(
        handle,
        context,
        SurfaceId::Hc2Rust,
        request,
        RuntimeCallOptions::default(),
    )?;
    let record = registry.run_next()?.expect("typed helper dispatch");
    assert_eq!(record.ticket, ticket);
    match record.result? {
        RegistryResult::ClusterHelper(execution) => Ok(execution),
        other => panic!("unexpected typed helper result {other:?}"),
    }
}

#[test]
fn typed_outcome_query_ignores_unrelated_down_root_and_expired_query_keeps_outcome_ownership() {
    let setup = config();
    // This independent harness is used only to select identity partitions. Values and all
    // query results below come exclusively from the actual registry owner graph.
    let routing = ServiceClusterHarness::new(
        &["a", "b", "c"],
        setup.bounds,
        collection(),
        ServiceLimits::default(),
        1,
    )
    .unwrap();
    let existing = (1..=64)
        .map(|n| MutationIdentity::new("queried", n))
        .find(|id| routing.coordinator_for(id) == 0)
        .unwrap();
    let unrelated = (1..=64)
        .map(|n| MutationIdentity::new("query-request", n))
        .find(|id| routing.coordinator_for(id) == 1)
        .unwrap();
    let registry = registry(setup);
    let (context, handle) = open(&registry, "tenant-a", "map");
    let mut put = command("put", 1);
    put.identity = Some(existing.clone());
    let original = run(&registry, &handle, &context, put).unwrap().result;
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
    let before = registry.diagnostics(&handle, &context).unwrap();
    let query = ClusterHelperRequest::new(
        unrelated.clone(),
        ClusterHelperCommand::QueryOutcomes {
            identities: vec![existing.clone()],
            max_bytes: 4096,
        },
    );
    let execution = run_typed_helper(&registry, &handle, &context, query.clone()).unwrap();
    assert!(!execution.replayed);
    assert!(execution.failure.is_none());
    assert!(execution.routes.iter().all(|route| route.partition == 0));
    let ClusterHelperResult::Outcomes(outcomes) = execution.result else {
        panic!("read-only outcome query");
    };
    assert_eq!(
        outcomes,
        vec![OperationOutcomeQuery::Completed(Box::new(original))]
    );
    let after = registry.diagnostics(&handle, &context).unwrap();
    assert_eq!(
        after.statistics.retained_outcomes,
        before.statistics.retained_outcomes
    );
    assert_eq!(after.resources, before.resources);
    assert_eq!(after.owners, before.owners);
    registry.advance_clock(1).unwrap();
    let mut expired = query;
    expired.deadline = 1;
    let refused = run_typed_helper(&registry, &handle, &context, expired).unwrap();
    assert_eq!(refused.result, ClusterHelperResult::NotApplied);
    assert!(refused.routes.is_empty());
    assert_eq!(
        registry
            .diagnostics(&handle, &context)
            .unwrap()
            .statistics
            .retained_outcomes,
        before.statistics.retained_outcomes
    );
    assert_eq!(registry.usage().queued_requests, 0);
    assert_eq!(registry.usage().reserved_growth, 0);
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
    let query_request = run_typed_helper(
        &registry,
        &handle,
        &context,
        ClusterHelperRequest::new(
            MutationIdentity::new("observer", 1),
            ClusterHelperCommand::QueryOutcomes {
                identities: vec![unrelated],
                max_bytes: 4096,
            },
        ),
    )
    .unwrap();
    assert_eq!(
        query_request.result,
        ClusterHelperResult::Outcomes(vec![OperationOutcomeQuery::Unknown])
    );
}

#[test]
fn typed_helpers_enforce_per_key_write_map_wide_read_and_admin_grants_before_queue_admission() {
    let registry = registry(config());
    registry.advance_policy_epoch(1).unwrap();
    let admin = granted(1, GrantRights::ALL, GrantKeys::All);
    let handle = registry.open(&admin, admin.address()).unwrap();
    let mut put = command("put", 1);
    put.key = b"a".to_vec();
    run(&registry, &handle, &admin, put).unwrap();
    let mut view = command("get_entry_view", 0);
    view.key = b"a".to_vec();
    let token = run(&registry, &handle, &admin, view)
        .unwrap()
        .result
        .token
        .unwrap();
    let writer = admin
        .attenuate(
            GrantRights {
                write: true,
                ..GrantRights::default()
            },
            GrantKeys::Prefixes(vec![b"a".to_vec()]),
        )
        .unwrap();
    let reader = admin
        .attenuate(
            GrantRights {
                read: true,
                ..GrantRights::default()
            },
            GrantKeys::All,
        )
        .unwrap();
    let scoped_reader = admin
        .attenuate(
            GrantRights {
                read: true,
                ..GrantRights::default()
            },
            GrantKeys::Prefixes(vec![b"a".to_vec()]),
        )
        .unwrap();
    let administrator = admin
        .attenuate(
            GrantRights {
                admin: true,
                ..GrantRights::default()
            },
            GrantKeys::All,
        )
        .unwrap();
    let allowed = VersionBulkEntry {
        key: b"a".to_vec(),
        token,
        value: Some(b"changed".to_vec()),
        ttl: ServiceTtl::Preserve,
    };
    let version = ClusterHelperRequest::new(
        MutationIdentity::new("version-helper", 1),
        ClusterHelperCommand::VersionBulk {
            operation: VersionBulkOperation::Replace,
            entries: vec![allowed.clone()],
        },
    );
    let mut forbidden_key = allowed.clone();
    forbidden_key.key = b"b".to_vec();
    let denied_bulk = ClusterHelperRequest::new(
        MutationIdentity::new("version-helper", 2),
        ClusterHelperCommand::VersionBulk {
            operation: VersionBulkOperation::Replace,
            entries: vec![allowed, forbidden_key],
        },
    );
    let query = ClusterHelperRequest::new(
        MutationIdentity::new("query-helper", 1),
        ClusterHelperCommand::QueryOutcomes {
            identities: vec![MutationIdentity::new("client", 1)],
            max_bytes: 4096,
        },
    );
    let expiry = ClusterHelperRequest::new(
        MutationIdentity::new("expiry-helper", 1),
        ClusterHelperCommand::ExpireStep { max_work: 1 },
    );
    let retirement = ClusterHelperRequest::new(
        MutationIdentity::new("retire-helper", 1),
        ClusterHelperCommand::RetireOutcomes {
            client: "unused-client".into(),
            through: 1,
        },
    );
    let before = registry.usage();
    let owners = registry.diagnostics(&handle, &admin).unwrap();
    for (principal, request) in [
        (&reader, version.clone()),
        (&writer, denied_bulk),
        (&scoped_reader, query.clone()),
        (&writer, query.clone()),
        (&administrator, query.clone()),
        (&reader, expiry.clone()),
        (&writer, expiry.clone()),
        (&reader, retirement.clone()),
        (&writer, retirement.clone()),
    ] {
        assert!(matches!(
            registry.enqueue_helper(
                &handle,
                principal,
                SurfaceId::Resp,
                request,
                RuntimeCallOptions::default()
            ),
            Err(RegistryError::Security(SecurityError::PermissionDenied))
        ));
        assert_eq!(registry.usage(), before);
        assert_eq!(registry.diagnostics(&handle, &admin).unwrap(), owners);
    }
    let changed = run_typed_helper(&registry, &handle, &writer, version).unwrap();
    let ClusterHelperResult::Versions(receipt) = changed.result else {
        panic!("version helper receipt");
    };
    assert_eq!(receipt.items.len(), 1);
    assert_eq!(receipt.items[0].kind, "replaced");
    assert!(receipt.items[0].value.is_none());
    assert!(matches!(
        run_typed_helper(&registry, &handle, &reader, query)
            .unwrap()
            .result,
        ClusterHelperResult::Outcomes(_)
    ));
    assert!(matches!(
        run_typed_helper(&registry, &handle, &administrator, expiry)
            .unwrap()
            .result,
        ClusterHelperResult::Expiry(_)
    ));
    assert!(matches!(
        run_typed_helper(&registry, &handle, &administrator, retirement)
            .unwrap()
            .result,
        ClusterHelperResult::Retirement(_)
    ));
    let mut get = command("get", 0);
    get.key = b"a".to_vec();
    assert_eq!(
        run(&registry, &handle, &admin, get).unwrap().result.value,
        Some(b"changed".to_vec())
    );
    assert_eq!(registry.usage().queued_requests, 0);
    assert_eq!(registry.usage().reserved_growth, 0);
}

#[test]
fn malformed_typed_helper_limits_and_shapes_cannot_charge_queue_growth_or_create_outcomes() {
    let registry = registry(config());
    let (context, handle) = open(&registry, "tenant-a", "map");
    run(&registry, &handle, &context, command("put", 1)).unwrap();
    let token = run(&registry, &handle, &context, command("get_entry_view", 0))
        .unwrap()
        .result
        .token
        .unwrap();
    let entry = VersionBulkEntry {
        key: vec![0, 255],
        token,
        value: Some(b"new".to_vec()),
        ttl: ServiceTtl::Preserve,
    };
    let mut missing = entry.clone();
    missing.value = None;
    let invalid = vec![
        ClusterHelperCommand::VersionBulk {
            operation: VersionBulkOperation::Replace,
            entries: vec![],
        },
        ClusterHelperCommand::VersionBulk {
            operation: VersionBulkOperation::Replace,
            entries: vec![entry.clone(), entry.clone()],
        },
        ClusterHelperCommand::VersionBulk {
            operation: VersionBulkOperation::Replace,
            entries: vec![entry.clone(); 257],
        },
        ClusterHelperCommand::VersionBulk {
            operation: VersionBulkOperation::Replace,
            entries: vec![missing.clone()],
        },
        ClusterHelperCommand::VersionBulk {
            operation: VersionBulkOperation::Remove,
            entries: vec![entry],
        },
        ClusterHelperCommand::VersionBulk {
            operation: VersionBulkOperation::SetTtl,
            entries: vec![missing],
        },
        ClusterHelperCommand::QueryOutcomes {
            identities: vec![],
            max_bytes: 4096,
        },
        ClusterHelperCommand::QueryOutcomes {
            identities: vec![MutationIdentity::new("client", 1)],
            max_bytes: 0,
        },
        ClusterHelperCommand::QueryOutcomes {
            identities: vec![MutationIdentity::new("client", 1)],
            max_bytes: ServiceLimits::default().query_bytes + 1,
        },
        ClusterHelperCommand::ExpireStep { max_work: 0 },
        ClusterHelperCommand::ExpireStep {
            max_work: collection().max_scan_items + 1,
        },
        ClusterHelperCommand::RetireOutcomes {
            client: "".into(),
            through: 1,
        },
        ClusterHelperCommand::RetireOutcomes {
            client: "client".into(),
            through: 0,
        },
    ];
    let before = registry.usage();
    let owners = registry.diagnostics(&handle, &context).unwrap();
    for (index, command) in invalid.into_iter().enumerate() {
        let request = ClusterHelperRequest::new(
            MutationIdentity::new("bad-helper", index as u64 + 1),
            command,
        );
        assert!(registry
            .enqueue_helper(
                &handle,
                &context,
                SurfaceId::Hc2Java,
                request,
                RuntimeCallOptions::default()
            )
            .is_err());
        assert_eq!(registry.usage(), before);
        assert_eq!(registry.diagnostics(&handle, &context).unwrap(), owners);
    }
    let self_retirement = ClusterHelperRequest::new(
        MutationIdentity::new("same-client", 1),
        ClusterHelperCommand::RetireOutcomes {
            client: "same-client".into(),
            through: 1,
        },
    );
    assert!(registry
        .enqueue_helper(
            &handle,
            &context,
            SurfaceId::Resp,
            self_retirement,
            RuntimeCallOptions::default()
        )
        .is_err());
    assert_eq!(registry.usage(), before);
    assert_eq!(registry.diagnostics(&handle, &context).unwrap(), owners);
}

#[test]
fn retirement_helper_preflights_shared_root_and_child_outcome_budget_before_any_fence() {
    let setup = config();
    let routing = ServiceClusterHarness::new(
        &["a", "b", "c"],
        setup.bounds,
        collection(),
        ServiceLimits::default(),
        1,
    )
    .unwrap();
    let identity = (1..=64)
        .map(|sequence| MutationIdentity::new("retirement-request", sequence))
        .find(|identity| routing.coordinator_for(identity) == 1)
        .unwrap();
    // A data retirement group fits (374 bytes), and a journal group fits (315).
    // The journal group at the root must also own the 130-byte root reservation;
    // together they exceed 425. All groups must be refused before the first fence.
    let registry = UnifiedServiceRegistry::new_clustered(
        RegistryBounds {
            max_cleanup_work: 64,
            ..RegistryBounds::default()
        },
        collection(),
        ServiceLimits {
            outcome_bytes: 425,
            ..ServiceLimits::default()
        },
        security(),
        TrustEpochWindow::new(1, 3).unwrap(),
        setup,
    )
    .unwrap();
    let (context, handle) = open(&registry, "tenant-a", "map");
    let before = registry.diagnostics(&handle, &context).unwrap();
    let usage = registry.usage();
    let request = ClusterHelperRequest::new(
        identity.clone(),
        ClusterHelperCommand::RetireOutcomes {
            client: "unused-client".into(),
            through: 1,
        },
    );
    assert!(matches!(
        run_typed_helper(&registry, &handle, &context, request),
        Err(RegistryError::Cluster(ServiceClusterError::Model(
            MapServiceError::ResourceBudget(_)
        )))
    ));
    assert_eq!(registry.diagnostics(&handle, &context).unwrap(), before);
    assert_eq!(registry.usage(), usage);
    let root = run_typed_helper(
        &registry,
        &handle,
        &context,
        ClusterHelperRequest::new(
            MutationIdentity::new("query-after-refusal", 1),
            ClusterHelperCommand::QueryOutcomes {
                identities: vec![identity],
                max_bytes: 4096,
            },
        ),
    )
    .unwrap();
    assert_eq!(
        root.result,
        ClusterHelperResult::Outcomes(vec![OperationOutcomeQuery::Unknown])
    );
    let mut old_identity = command("put", 1);
    old_identity.identity = Some(MutationIdentity::new("unused-client", 1));
    let applied = run(&registry, &handle, &context, old_identity).unwrap();
    assert_eq!(applied.result.kind, "inserted");
    assert!(applied.backup_proved);
    assert!(applied.failure.is_none());
    assert_eq!(registry.usage().queued_requests, 0);
    assert_eq!(registry.usage().reserved_growth, 0);
}
