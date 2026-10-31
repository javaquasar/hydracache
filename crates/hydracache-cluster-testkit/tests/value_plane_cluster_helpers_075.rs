use hydracache_cluster_testkit::value_plane_cluster_admission_075::*;
use hydracache_cluster_testkit::value_plane_cluster_auth_075::*;
use hydracache_cluster_testkit::value_plane_cluster_health_075::{
    MaintenanceResult, RuntimeMaintenance,
};
use hydracache_cluster_testkit::value_plane_cluster_runtime_075::*;
use hydracache_cluster_testkit::value_plane_collection_075::CollectionBounds;
use hydracache_cluster_testkit::value_plane_loopback_075::LoopbackFault;
use hydracache_cluster_testkit::value_plane_model_075::MutationIdentity;
use hydracache_cluster_testkit::value_plane_security_075::*;
use hydracache_cluster_testkit::value_plane_service_075::*;
use hydracache_cluster_testkit::value_plane_service_cluster_075::*;
use hydracache_cluster_testkit::value_plane_service_registry_075::*;
use hydracache_cluster_testkit::value_plane_surface_075::*;
use hydracache_cluster_transport_axum::NodeCredential;

fn fixture() -> (
    UnifiedServiceRegistry,
    AuthenticatedNamespace,
    RegistryHandle,
) {
    fixture_with_limits(
        RegistryBounds {
            partitions: 2,
            ..RegistryBounds::default()
        },
        ClusterAdmissionBounds {
            max_replica_bytes: 64 * 1_024 * 1_024,
            ..ClusterAdmissionBounds::default()
        },
    )
}
fn fixture_with_limits(
    registry_bounds: RegistryBounds,
    admission: ClusterAdmissionBounds,
) -> (
    UnifiedServiceRegistry,
    AuthenticatedNamespace,
    RegistryHandle,
) {
    let principal = IngressPrincipal {
        tenant: "tenant".into(),
    };
    let verified = VerifiedMapCommand::new(
        SurfaceId::Hc2Rust,
        &principal,
        "tenant",
        "map",
        1,
        b"k".to_vec(),
        SurfaceOperation::Read,
    )
    .unwrap();
    let security = SecurityBounds::default();
    let claims = RouteClaims {
        authenticated: true,
        authenticated_tenant: "tenant",
        claimed_tenant: "tenant",
        expected_namespace_generation: 1,
        actual_namespace_generation: 1,
        expected_topology_epoch: 1,
        actual_topology_epoch: 1,
        proxy_hops: 0,
        deadline_remaining_ticks: 10_000,
        trust_epoch: 1,
        encoded_envelope_bytes: 64,
        key_bytes: 1,
        value_bytes: 0,
    };
    let authenticated = AuthenticatedNamespace::verify(
        &principal,
        &verified,
        &claims,
        security,
        TrustEpochWindow::new(1, 3).unwrap(),
        0,
    )
    .unwrap();
    let config = ClusterRuntimeConfig {
        cluster: "test".into(),
        members: ["a", "b", "c"]
            .into_iter()
            .map(|node| RuntimeMember {
                node: node.into(),
                generation: 1,
                credential: NodeCredential::new("key", format!("secret-{node}")),
            })
            .collect(),
        entry_node: "c".into(),
        bounds: ServiceClusterBounds {
            partitions: 2,
            ..ServiceClusterBounds::default()
        },
        admission,
        max_replays: 4_096,
    };
    let registry = UnifiedServiceRegistry::new_clustered(
        registry_bounds,
        CollectionBounds::default(),
        ServiceLimits::default(),
        security,
        TrustEpochWindow::new(1, 3).unwrap(),
        config,
    )
    .unwrap();
    let handle = registry
        .open(&authenticated, authenticated.address())
        .unwrap();
    (registry, authenticated, handle)
}
fn command(
    registry: &UnifiedServiceRegistry,
    auth: &AuthenticatedNamespace,
    handle: &RegistryHandle,
    operation: &str,
    key: &[u8],
    sequence: u64,
    ttl: ServiceTtl,
) -> ServiceClusterExecution {
    let mut command = ServiceCommand::point(operation, key);
    command.value = Some(b"old".to_vec());
    command.ttl = ttl;
    if sequence != 0 {
        command.identity = Some(MutationIdentity::new("values", sequence));
    }
    let ticket = registry
        .enqueue(handle, auth, SurfaceId::Hc2Rust, command)
        .unwrap();
    let result = registry.run_next().unwrap().unwrap();
    assert_eq!(result.ticket, ticket);
    match result.result.unwrap() {
        RegistryResult::ClusterCommand(result) => result,
        other => panic!("{other:?}"),
    }
}
fn helper(
    registry: &UnifiedServiceRegistry,
    auth: &AuthenticatedNamespace,
    handle: &RegistryHandle,
    request: ClusterHelperRequest,
    options: RuntimeCallOptions,
) -> ClusterHelperExecution {
    let ticket = registry
        .enqueue_helper(handle, auth, SurfaceId::Hc2Rust, request, options)
        .unwrap();
    let record = registry.run_next().unwrap().unwrap();
    assert_eq!(record.ticket, ticket);
    match record.result.unwrap() {
        RegistryResult::ClusterHelper(result) => result,
        other => panic!("{other:?}"),
    }
}
fn request(client: &str, sequence: u64, command: ClusterHelperCommand) -> ClusterHelperRequest {
    ClusterHelperRequest::new(MutationIdentity::new(client, sequence), command)
}

#[test]
fn authenticated_cluster_helpers_share_real_owners_receipts_and_backup_proof() {
    let (registry, auth, handle) = fixture();
    let keys = [b"a".to_vec(), b"b".to_vec(), b"c".to_vec(), b"d".to_vec()];
    let mut entries = Vec::new();
    for (index, key) in keys.iter().enumerate() {
        assert!(
            command(
                &registry,
                &auth,
                &handle,
                "put",
                key,
                index as u64 + 1,
                ServiceTtl::Eternal
            )
            .backup_proved
        );
        let token = command(
            &registry,
            &auth,
            &handle,
            "get_entry_view",
            key,
            0,
            ServiceTtl::Preserve,
        )
        .result
        .token
        .unwrap();
        entries.push(VersionBulkEntry {
            key: key.clone(),
            token,
            value: Some(b"new".to_vec()),
            ttl: ServiceTtl::Preserve,
        });
    }
    let bulk = request(
        "helpers",
        1,
        ClusterHelperCommand::VersionBulk {
            operation: VersionBulkOperation::Replace,
            entries,
        },
    );
    let first = helper(
        &registry,
        &auth,
        &handle,
        bulk.clone(),
        RuntimeCallOptions::default(),
    );
    assert!(first.backup_proved);
    assert!(first.failure.is_none());
    let ClusterHelperResult::Versions(result) = &first.result else {
        panic!("{first:?}")
    };
    assert_eq!(
        result
            .items
            .iter()
            .map(|item| item.index)
            .collect::<Vec<_>>(),
        vec![0, 1, 2, 3]
    );
    assert!(result.items.iter().all(|item| item.kind == "replaced"));
    let replay = helper(
        &registry,
        &auth,
        &handle,
        bulk,
        RuntimeCallOptions::default(),
    );
    assert!(replay.replayed);
    assert_eq!(replay.result, first.result);
    for key in &keys {
        assert_eq!(
            command(
                &registry,
                &auth,
                &handle,
                "get",
                key,
                0,
                ServiceTtl::Preserve
            )
            .result
            .value
            .as_deref(),
            Some(b"new".as_slice())
        );
    }
    let queried = helper(
        &registry,
        &auth,
        &handle,
        request(
            "query",
            1,
            ClusterHelperCommand::QueryOutcomes {
                identities: vec![
                    MutationIdentity::new("helpers", 1),
                    MutationIdentity::new("missing", 1),
                ],
                max_bytes: 4_096,
            },
        ),
        RuntimeCallOptions::default(),
    );
    let ClusterHelperResult::Outcomes(outcomes) = queried.result else {
        panic!("{queried:?}")
    };
    assert!(
        matches!(&outcomes[0], OperationOutcomeQuery::Completed(result) if result.items.len() == 4)
    );
    assert_eq!(outcomes[1], OperationOutcomeQuery::Unknown);
    let before = registry
        .diagnostics(&handle, &auth)
        .unwrap()
        .owners
        .completed_outcomes;
    let retirement = helper(
        &registry,
        &auth,
        &handle,
        request(
            "admin",
            1,
            ClusterHelperCommand::RetireOutcomes {
                client: "helpers".into(),
                through: 1,
            },
        ),
        RuntimeCallOptions::default(),
    );
    assert!(retirement.backup_proved);
    assert!(
        matches!(retirement.result, ClusterHelperResult::Retirement(receipt) if receipt.removed >= 2)
    );
    // Both original root and per-owner helper receipts were retired, not just the root row.
    let after = registry
        .diagnostics(&handle, &auth)
        .unwrap()
        .owners
        .completed_outcomes;
    assert!(after < before + 12);
    let query = helper(
        &registry,
        &auth,
        &handle,
        request(
            "query",
            2,
            ClusterHelperCommand::QueryOutcomes {
                identities: vec![MutationIdentity::new("helpers", 1)],
                max_bytes: 4_096,
            },
        ),
        RuntimeCallOptions::default(),
    );
    assert_eq!(
        query.result,
        ClusterHelperResult::Outcomes(vec![OperationOutcomeQuery::Unknown])
    );
}

#[test]
fn helper_faults_recover_original_decisions_without_reexecution() {
    let (registry, auth, handle) = fixture();
    command(
        &registry,
        &auth,
        &handle,
        "put",
        b"key",
        1,
        ServiceTtl::Eternal,
    );
    let token = command(
        &registry,
        &auth,
        &handle,
        "get_entry_view",
        b"key",
        0,
        ServiceTtl::Preserve,
    )
    .result
    .token
    .unwrap();
    let request = request(
        "lost",
        1,
        ClusterHelperCommand::VersionBulk {
            operation: VersionBulkOperation::Remove,
            entries: vec![VersionBulkEntry {
                key: b"key".to_vec(),
                token,
                value: None,
                ttl: ServiceTtl::Preserve,
            }],
        },
    );
    let mut options = RuntimeCallOptions::default();
    options.call.faults.data.ack = LoopbackFault::Drop;
    let unknown = helper(&registry, &auth, &handle, request.clone(), options);
    assert!(
        matches!(&unknown.result, ClusterHelperResult::Versions(result) if result.kind == "partial" && result.items[0].kind == "outcome_unknown")
    );
    assert!(!unknown.backup_proved);
    let recovered = helper(
        &registry,
        &auth,
        &handle,
        request.clone(),
        RuntimeCallOptions::default(),
    );
    assert!(recovered.backup_proved);
    assert!(
        matches!(recovered.result, ClusterHelperResult::Versions(ref result) if result.items[0].kind == "removed")
    );
    let final_replay = helper(
        &registry,
        &auth,
        &handle,
        request,
        RuntimeCallOptions::default(),
    );
    assert_eq!(final_replay.result, recovered.result);
    assert_eq!(
        command(
            &registry,
            &auth,
            &handle,
            "get",
            b"key",
            0,
            ServiceTtl::Preserve
        )
        .result
        .kind,
        "absent"
    );
}

#[test]
fn lost_helper_response_keeps_the_original_completed_receipt() {
    let (registry, auth, handle) = fixture();
    command(
        &registry,
        &auth,
        &handle,
        "put",
        b"key",
        1,
        ServiceTtl::Eternal,
    );
    let token = command(
        &registry,
        &auth,
        &handle,
        "get_entry_view",
        b"key",
        0,
        ServiceTtl::Preserve,
    )
    .result
    .token
    .unwrap();
    let original = request(
        "response",
        1,
        ClusterHelperCommand::VersionBulk {
            operation: VersionBulkOperation::Replace,
            entries: vec![VersionBulkEntry {
                key: b"key".to_vec(),
                token,
                value: Some(b"changed".to_vec()),
                ttl: ServiceTtl::Preserve,
            }],
        },
    );
    let mut options = RuntimeCallOptions::default();
    options.call.faults.response = LoopbackFault::Drop;
    let lost = helper(&registry, &auth, &handle, original.clone(), options);
    assert_eq!(lost.result, ClusterHelperResult::OutcomeUnknown);
    assert!(lost.backup_proved);
    let result = helper(
        &registry,
        &auth,
        &handle,
        original,
        RuntimeCallOptions::default(),
    );
    assert!(result.replayed && result.backup_proved);
    assert!(
        matches!(result.result, ClusterHelperResult::Versions(result) if result.items[0].kind == "replaced")
    );
}

#[test]
fn partial_version_bulk_reports_only_proved_items_and_retry_preserves_their_versions() {
    let (registry, auth, handle) = fixture();
    let mut by_partition = std::collections::BTreeMap::new();
    for sequence in 1..32 {
        let key = vec![sequence as u8];
        let inserted = command(
            &registry,
            &auth,
            &handle,
            "put",
            &key,
            sequence,
            ServiceTtl::Eternal,
        );
        let partition = inserted.routes.last().unwrap().partition;
        let token = command(
            &registry,
            &auth,
            &handle,
            "get_entry_view",
            &key,
            0,
            ServiceTtl::Preserve,
        )
        .result
        .token
        .unwrap();
        by_partition.entry(partition).or_insert(VersionBulkEntry {
            key,
            token,
            value: Some(b"typed".to_vec()),
            ttl: ServiceTtl::Preserve,
        });
        if by_partition.len() == 2 {
            break;
        }
    }
    let first = by_partition.remove(&0).unwrap();
    let second = by_partition.remove(&1).unwrap();
    let original = request(
        "partial",
        1,
        ClusterHelperCommand::VersionBulk {
            operation: VersionBulkOperation::Replace,
            entries: vec![second, first],
        },
    );
    let mut options = RuntimeCallOptions::default();
    options.call.faults.partitions.insert(
        1,
        ServiceLinkFaults {
            ack: LoopbackFault::Drop,
            ..ServiceLinkFaults::default()
        },
    );
    let partial = helper(&registry, &auth, &handle, original.clone(), options);
    assert!(!partial.backup_proved && partial.failure.is_some());
    let ClusterHelperResult::Versions(partial) = partial.result else {
        panic!("{partial:?}")
    };
    assert_eq!(partial.kind, "partial");
    assert_eq!(
        partial
            .items
            .iter()
            .map(|item| item.index)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
    assert_eq!(partial.items[0].kind, "outcome_unknown");
    assert!(partial.items[0].version.is_none());
    assert_eq!(partial.items[1].kind, "replaced");
    let known_version = partial.items[1].version;
    let before = registry.diagnostics(&handle, &auth).unwrap();
    let progress = helper(
        &registry,
        &auth,
        &handle,
        request(
            "partial-progress",
            1,
            ClusterHelperCommand::QueryProgress {
                original: Box::new(original.clone()),
                original_digest: original.request_digest(),
                max_bytes: 16_384,
            },
        ),
        RuntimeCallOptions::default(),
    );
    let ClusterHelperResult::Progress(progress) = progress.result else {
        panic!("{progress:?}")
    };
    assert_eq!(progress.root, OperationOutcomeQuery::Pending);
    assert!(progress.groups.iter().all(|group| matches!(&group.outcome, OperationOutcomeQuery::Completed(result) if result.items[0].kind == "replaced")));
    assert!(progress
        .proofs
        .iter()
        .any(|proof| proof.partition == 0 && !proof.journal && proof.proved));
    assert!(progress
        .proofs
        .iter()
        .any(|proof| proof.partition == 1 && !proof.journal && !proof.proved));
    let after = registry.diagnostics(&handle, &auth).unwrap();
    assert_eq!(after.statistics, before.statistics);
    assert_eq!(after.resources, before.resources);
    assert_eq!(after.owners, before.owners);
    assert_eq!(after.logical_time, before.logical_time);
    let replay = helper(
        &registry,
        &auth,
        &handle,
        original,
        RuntimeCallOptions::default(),
    );
    assert!(replay.backup_proved);
    let ClusterHelperResult::Versions(replay) = replay.result else {
        panic!("{replay:?}")
    };
    assert!(replay.items.iter().all(|item| item.kind == "replaced"));
    assert_eq!(replay.items[1].version, known_version);
}

#[test]
fn typed_ttl_and_expiry_share_a_global_bounded_work_budget() {
    let (registry, auth, handle) = fixture();
    let mut entries = Vec::new();
    for (index, key) in [b"a", b"b", b"c", b"d"].iter().enumerate() {
        command(
            &registry,
            &auth,
            &handle,
            "put",
            *key,
            index as u64 + 1,
            ServiceTtl::Eternal,
        );
        let token = command(
            &registry,
            &auth,
            &handle,
            "get_entry_view",
            *key,
            0,
            ServiceTtl::Preserve,
        )
        .result
        .token
        .unwrap();
        entries.push(VersionBulkEntry {
            key: key.to_vec(),
            token,
            value: None,
            ttl: ServiceTtl::ExpireAfter(1),
        });
    }
    let ttl = helper(
        &registry,
        &auth,
        &handle,
        request(
            "ttl",
            1,
            ClusterHelperCommand::VersionBulk {
                operation: VersionBulkOperation::SetTtl,
                entries,
            },
        ),
        RuntimeCallOptions::default(),
    );
    assert!(ttl.backup_proved);
    registry.advance_clock(1).unwrap();
    let mut expired = 0;
    for sequence in 1..9 {
        let result = helper(
            &registry,
            &auth,
            &handle,
            request(
                "expiry",
                sequence,
                ClusterHelperCommand::ExpireStep { max_work: 1 },
            ),
            RuntimeCallOptions::default(),
        );
        assert!(result.backup_proved);
        let ClusterHelperResult::Expiry(progress) = result.result else {
            panic!("{result:?}")
        };
        assert!(progress.examined <= 1);
        assert!(progress.expired <= 1);
        expired += progress.expired;
        if !progress.pending {
            break;
        }
    }
    assert_eq!(expired, 4);
}

#[test]
fn cluster_entry_views_return_partition_scoped_tokens_and_preflight_the_whole_response() {
    let (registry, auth, handle) = fixture();
    let mut keys = std::collections::BTreeMap::new();
    for sequence in 1..32 {
        let key = vec![sequence as u8];
        let inserted = command(
            &registry,
            &auth,
            &handle,
            "put",
            &key,
            sequence,
            ServiceTtl::ExpireAfter(10),
        );
        keys.entry(inserted.routes.last().unwrap().partition)
            .or_insert(key);
        if keys.len() == 2 {
            break;
        }
    }
    let first = keys.remove(&0).unwrap();
    let second = keys.remove(&1).unwrap();
    let input = vec![
        second.clone(),
        b"missing".to_vec(),
        first.clone(),
        second.clone(),
    ];
    let before = registry.diagnostics(&handle, &auth).unwrap();
    let response = helper(
        &registry,
        &auth,
        &handle,
        request(
            "views",
            1,
            ClusterHelperCommand::EntryViews {
                keys: input.clone(),
                max_bytes: 1_024,
            },
        ),
        RuntimeCallOptions::default(),
    );
    assert!(response.backup_proved);
    let ClusterHelperResult::EntryViews(views) = response.result else {
        panic!("{response:?}")
    };
    assert_eq!(
        views.iter().map(|view| view.index).collect::<Vec<_>>(),
        vec![0, 1, 2, 3]
    );
    assert_eq!(
        views
            .iter()
            .map(|view| view.key.clone())
            .collect::<Vec<_>>(),
        input
    );
    assert_eq!(
        views[0],
        VersionedEntryView {
            index: 0,
            key: second.clone(),
            value: Some(b"old".to_vec()),
            token: views[3].token,
            remaining_ttl: Some(10)
        }
    );
    assert!(
        views[1].value.is_none() && views[1].token.is_none() && views[1].remaining_ttl.is_none()
    );
    let first_token = views[2].token.unwrap();
    let second_token = views[0].token.unwrap();
    assert_eq!(first_token.namespace_generation, 1);
    assert_ne!(first_token.map_id, second_token.map_id);
    assert_eq!(
        registry
            .diagnostics(&handle, &auth)
            .unwrap()
            .owners
            .completed_outcomes,
        before.owners.completed_outcomes
    );
    let updated = helper(
        &registry,
        &auth,
        &handle,
        request(
            "cas",
            1,
            ClusterHelperCommand::VersionBulk {
                operation: VersionBulkOperation::Replace,
                entries: vec![
                    VersionBulkEntry {
                        key: second.clone(),
                        token: second_token,
                        value: Some(b"new".to_vec()),
                        ttl: ServiceTtl::Preserve,
                    },
                    VersionBulkEntry {
                        key: first.clone(),
                        token: second_token,
                        value: Some(b"foreign".to_vec()),
                        ttl: ServiceTtl::Preserve,
                    },
                ],
            },
        ),
        RuntimeCallOptions::default(),
    );
    let ClusterHelperResult::Versions(updated) = updated.result else {
        panic!("{updated:?}")
    };
    assert_eq!(updated.items[0].kind, "replaced");
    assert_eq!(updated.items[1].kind, "mismatch");
    assert_eq!(
        command(
            &registry,
            &auth,
            &handle,
            "get_entry_view",
            &first,
            0,
            ServiceTtl::Preserve
        )
        .result
        .token,
        Some(first_token)
    );
    registry.advance_clock(10).unwrap();
    let before = registry.diagnostics(&handle, &auth).unwrap();
    let usage = registry.usage();
    assert!(registry
        .enqueue_helper(
            &handle,
            &auth,
            SurfaceId::Hc2Rust,
            request(
                "too-small",
                1,
                ClusterHelperCommand::EntryViews {
                    keys: vec![first, second],
                    max_bytes: 1
                }
            ),
            RuntimeCallOptions::default()
        )
        .is_err());
    assert_eq!(registry.diagnostics(&handle, &auth).unwrap(), before);
    assert_eq!(registry.usage(), usage);
}

#[test]
fn authenticated_transfer_controls_use_safety_and_repair_permits_with_scoped_cleanup() {
    let (registry, legacy, handle) = fixture();
    registry.advance_policy_epoch(1).unwrap();
    let auth = legacy
        .with_grant(
            NamespaceGrant::new("tenant", "map", 1, 1, GrantRights::ALL, GrantKeys::All).unwrap(),
        )
        .unwrap();
    let read = auth
        .attenuate(
            GrantRights {
                read: true,
                write: false,
                listen: false,
                admin: false,
            },
            GrantKeys::All,
        )
        .unwrap();
    let dummy = TransferScope {
        transfer: 1,
        partition: 0,
        source_epoch: 1,
    };
    for operation in [
        RuntimeMaintenance::AbortTransfer { scope: dummy },
        RuntimeMaintenance::CleanupTransfer { scope: dummy },
        RuntimeMaintenance::RestartTransfer {
            scope: dummy,
            expected_current_epoch: 1,
            target: "c".into(),
            options: OnlineTransferOptions::default(),
        },
    ] {
        let before = registry.usage();
        assert!(registry
            .enqueue_maintenance(&handle, &read, operation)
            .is_err());
        assert_eq!(registry.usage(), before);
    }
    command(
        &registry,
        &auth,
        &handle,
        "put",
        b"live",
        1,
        ServiceTtl::ExpireAfter(30),
    );
    let epoch = registry.cluster_health(&handle, &auth).unwrap().partitions[0].epoch;
    let maintenance = |operation| {
        let ticket = registry
            .enqueue_maintenance(&handle, &auth, operation)
            .unwrap();
        let record = registry.run_next().unwrap().unwrap();
        assert_eq!(record.ticket, ticket);
        assert_eq!(
            registry.cluster_admission_observations().unwrap().inflight,
            [0; 3]
        );
        assert_eq!(registry.usage().reserved_growth, 0);
        record.result
    };
    let started = maintenance(RuntimeMaintenance::RepairStep {
        partition: 0,
        backup: "c".into(),
        faults: ServiceLinkFaults {
            ack: LoopbackFault::Drop,
            ..ServiceLinkFaults::default()
        },
        maximum_work: 1,
    })
    .unwrap();
    let RegistryResult::Maintenance(MaintenanceResult::Online(started)) = started else {
        panic!("online repair")
    };
    let cut = TransferScope {
        transfer: started.transfer.unwrap(),
        partition: 0,
        source_epoch: epoch,
    };
    let before = registry.diagnostics(&handle, &auth).unwrap();
    assert!(maintenance(RuntimeMaintenance::AbortTransfer {
        scope: TransferScope {
            source_epoch: epoch + 1,
            ..cut
        }
    })
    .is_err());
    assert!(maintenance(RuntimeMaintenance::CleanupTransfer { scope: cut }).is_err());
    assert_eq!(
        registry.diagnostics(&handle, &auth).unwrap().resources,
        before.resources
    );
    let RegistryResult::Maintenance(MaintenanceResult::Transfer(aborted)) =
        maintenance(RuntimeMaintenance::AbortTransfer { scope: cut }).unwrap()
    else {
        panic!("abort diagnostic")
    };
    let aborted = aborted.current.unwrap();
    assert_eq!(aborted.phase, OnlineTransferPhase::Aborted);
    assert_eq!(
        (
            aborted.held_bytes,
            aborted.pending_deltas,
            aborted.unacknowledged,
            aborted.staged_bytes
        ),
        (0, 0, 0, 384)
    );
    assert!(maintenance(RuntimeMaintenance::RestartTransfer {
        scope: cut,
        expected_current_epoch: epoch + 1,
        target: "c".into(),
        options: OnlineTransferOptions::default()
    })
    .is_err());
    let RegistryResult::Maintenance(MaintenanceResult::Transfer(restarted)) =
        maintenance(RuntimeMaintenance::RestartTransfer {
            scope: cut,
            expected_current_epoch: epoch,
            target: "c".into(),
            options: OnlineTransferOptions::default(),
        })
        .unwrap()
    else {
        panic!("restart diagnostic")
    };
    assert!(restarted.removed);
    let next = restarted.current.unwrap().scope;
    assert!(next.transfer > cut.transfer);
    let mut complete = false;
    for _ in 0..64 {
        let RegistryResult::Maintenance(MaintenanceResult::Online(step)) =
            maintenance(RuntimeMaintenance::RepairStep {
                partition: 0,
                backup: "c".into(),
                faults: ServiceLinkFaults::default(),
                maximum_work: 1,
            })
            .unwrap()
        else {
            panic!("online continuation")
        };
        assert_eq!(step.transfer, Some(next.transfer));
        assert!(step.work <= 1);
        if step.complete {
            complete = true;
            break;
        }
    }
    assert!(complete);
    assert!(registry.cluster_health(&handle, &auth).unwrap().partitions[0].backup_proved);
    let RegistryResult::Maintenance(MaintenanceResult::Transfer(cleaned)) =
        maintenance(RuntimeMaintenance::CleanupTransfer { scope: next }).unwrap()
    else {
        panic!("cleanup diagnostic")
    };
    assert!(cleaned.removed && cleaned.current.is_none());
    let RegistryResult::Maintenance(MaintenanceResult::Transfer(cleaned)) =
        maintenance(RuntimeMaintenance::CleanupTransfer { scope: next }).unwrap()
    else {
        panic!("idempotent cleanup")
    };
    assert!(!cleaned.removed && cleaned.current.is_none());
    assert!(maintenance(RuntimeMaintenance::AbortTransfer { scope: cut }).is_err());
    assert_eq!(
        registry
            .diagnostics(&handle, &auth)
            .unwrap()
            .owners
            .resync_leases,
        0
    );
    assert_eq!(
        command(
            &registry,
            &auth,
            &handle,
            "get",
            b"live",
            0,
            ServiceTtl::Preserve
        )
        .result
        .value
        .as_deref(),
        Some(b"old".as_slice())
    );
}

#[test]
fn helper_progress_reserves_request_derived_peak_before_any_registry_query_allocation() {
    let original = request(
        "large-original",
        1,
        ClusterHelperCommand::VersionBulk {
            operation: VersionBulkOperation::Replace,
            entries: vec![VersionBulkEntry {
                key: b"key".to_vec(),
                token: VersionToken {
                    namespace_generation: 1,
                    map_id: 1,
                    version: 1,
                },
                value: Some(vec![17; 4_096]),
                ttl: ServiceTtl::Preserve,
            }],
        },
    );
    let query = || {
        request(
            "progress",
            1,
            ClusterHelperCommand::QueryProgress {
                original: Box::new(original.clone()),
                original_digest: original.request_digest(),
                max_bytes: 4_096,
            },
        )
    };
    let admission = ClusterAdmissionBounds {
        max_safety_bytes: 1_024 * 1_024,
        ..ClusterAdmissionBounds::default()
    };
    let (registry, auth, handle) = fixture_with_limits(
        RegistryBounds {
            partitions: 2,
            max_owned_bytes: 100_000,
            max_tenant_owned_bytes: 100_000,
            ..RegistryBounds::default()
        },
        admission,
    );
    let before = registry.diagnostics(&handle, &auth).unwrap();
    let bytes = registry.usage();
    let scheduling = registry.cluster_admission_observations().unwrap();
    assert!(matches!(
        registry.enqueue_helper(
            &handle,
            &auth,
            SurfaceId::Hc2Rust,
            query(),
            RuntimeCallOptions::default()
        ),
        Err(RegistryError::Quota {
            dimension: "owned_bytes",
            ..
        })
    ));
    assert_eq!(registry.diagnostics(&handle, &auth).unwrap(), before);
    assert_eq!(registry.usage(), bytes);
    let after = registry.cluster_admission_observations().unwrap();
    assert_eq!(after.inflight, scheduling.inflight);
    assert_eq!(after.owned_bytes, scheduling.owned_bytes);
    let (registry, auth, handle) = fixture_with_limits(
        RegistryBounds {
            partitions: 2,
            max_owned_bytes: 1_024 * 1_024,
            max_tenant_owned_bytes: 1_024 * 1_024,
            ..RegistryBounds::default()
        },
        admission,
    );
    let before = registry.diagnostics(&handle, &auth).unwrap();
    let result = helper(
        &registry,
        &auth,
        &handle,
        query(),
        RuntimeCallOptions::default(),
    );
    assert!(
        matches!(result.result, ClusterHelperResult::Progress(progress) if progress.root == OperationOutcomeQuery::Unknown && progress.groups.iter().all(|group| group.outcome == OperationOutcomeQuery::Unknown))
    );
    assert_eq!(registry.diagnostics(&handle, &auth).unwrap(), before);
    assert_eq!(registry.usage().reserved_growth, 0);
}
