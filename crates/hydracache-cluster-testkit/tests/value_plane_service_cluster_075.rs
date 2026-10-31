use futures_util::Stream;
use hydracache_cluster_testkit::value_plane_collection_075::{
    CollectionBounds, ReplacementEntry, ScanProjection,
};
use hydracache_cluster_testkit::value_plane_loopback_075::{LoopbackError, LoopbackFault};
use hydracache_cluster_testkit::value_plane_model_075::MutationIdentity;
use hydracache_cluster_testkit::value_plane_service_075::*;
use hydracache_cluster_testkit::value_plane_service_cluster_075::*;
use hydracache_cluster_testkit::value_plane_stream_075::{DemandStream, DemandStreamError};
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

fn cluster(partitions: u32) -> ServiceClusterHarness {
    ServiceClusterHarness::new(
        &["a", "b", "c"],
        ServiceClusterBounds {
            partitions,
            ..ServiceClusterBounds::default()
        },
        CollectionBounds::default(),
        ServiceLimits::default(),
        1,
    )
    .unwrap()
}
fn id(sequence: u64) -> MutationIdentity {
    MutationIdentity::new("client", sequence)
}
fn command(operation: &str, key: &[u8], sequence: u64) -> ServiceCommand {
    let mut c = ServiceCommand::point(operation, key);
    c.identity = Some(id(sequence));
    c.value = Some(b"v".to_vec());
    c
}
fn key_in(cluster: &ServiceClusterHarness, partition: u32, suffix: usize) -> Vec<u8> {
    (suffix..10_000)
        .map(|n| format!("key-{n}").into_bytes())
        .find(|key| cluster.partition_for(key) == partition)
        .unwrap()
}
fn identity_in(cluster: &ServiceClusterHarness, partition: u32, start: u64) -> MutationIdentity {
    (start..start + 10_000)
        .map(id)
        .find(|id| cluster.coordinator_for(id) == partition)
        .unwrap()
}
fn run(cluster: &mut ServiceClusterHarness, command: ServiceCommand) -> ServiceClusterExecution {
    cluster
        .execute("c", command, ServiceClusterCall::default())
        .unwrap()
}

#[test]
fn all_40_service_operations_have_explicit_cluster_scope() {
    let contract: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/testing/imap/0.75/operation-contract.json"
    ))
    .unwrap();
    let operations = contract["operations"].as_array().unwrap();
    assert_eq!(operations.len(), 40);
    for operation in operations {
        let op = operation["id"].as_str().unwrap();
        let scope = service_operation_scope(op).expect(op);
        let mut single = cluster(1);
        run(&mut single, command("put", b"a", 1));
        run(&mut single, command("put", b"b", 2));
        let mut request = command(op, b"a", 3);
        request.expected = Some(b"v".to_vec());
        request.limit = 8;
        if matches!(op, "get_all" | "remove_all_detailed") {
            request.keys = vec![b"b".to_vec(), b"a".to_vec()];
        }
        if matches!(op, "put_all_detailed" | "replace_all_detailed") {
            request.entries = vec![
                ReplacementEntry {
                    key: b"b".to_vec(),
                    value: b"new".to_vec(),
                },
                ReplacementEntry {
                    key: b"a".to_vec(),
                    value: b"new".to_vec(),
                },
            ];
        }
        if matches!(
            op,
            "replace_if_version" | "remove_if_version" | "set_ttl_if_version"
        ) {
            request.token = run(&mut single, ServiceCommand::point("get_entry_view", b"a"))
                .result
                .token;
        }
        if op == "query_operation_outcome" {
            request.identity = Some(id(1));
        }
        if op == "remove_entry_listener" {
            let mut listen = ServiceCommand::point("entry_listener", b"");
            listen.listener_id = 77;
            listen.limit = 8;
            run(&mut single, listen);
            request.listener_id = 77;
        }
        let result = run(&mut single, request);
        assert!(!result.result.kind.is_empty(), "{op}");
        if scope == ServiceOperationScope::SinglePartitionMapWide {
            let mut multi = cluster(3);
            assert!(
                matches!(
                    multi.execute("a", command(op, b"a", 1), ServiceClusterCall::default()),
                    Err(ServiceClusterError::UnsupportedScope)
                ),
                "{op}"
            );
            assert_eq!(
                multi.query_operation_outcome(&id(1)).unwrap(),
                OperationOutcomeQuery::Unknown
            );
        }
    }
    assert_eq!(service_operation_scope("execute_remote_processor"), None);
    let mut multi = cluster(3);
    let inventory = multi.operation_inventory();
    assert_eq!(inventory.len(), 40);
    assert_eq!(inventory.iter().filter(|op| op.supported).count(), 29);
    assert_eq!(
        inventory
            .iter()
            .map(|op| op.operation)
            .collect::<std::collections::BTreeSet<_>>(),
        operations
            .iter()
            .map(|op| op["id"].as_str().unwrap())
            .collect::<std::collections::BTreeSet<_>>()
    );
    assert!(!multi.supports_operation("clear_detailed"));
    assert!(multi.supports_operation("put_all_detailed"));
    assert!(!multi.supports_operation("future_operation"));
    let cap = run(
        &mut multi,
        ServiceCommand::point("capability_descriptor", b""),
    );
    assert_eq!(cap.result.number, 29);
    assert!(!cap.result.flag);
    let mut single = cluster(1);
    assert_eq!(
        run(
            &mut single,
            ServiceCommand::point("capability_descriptor", b"")
        )
        .result
        .number,
        40
    );
}

#[test]
fn actual_service_routes_proxy_and_replicates_owner_decisions_once() {
    let mut cluster = cluster(3);
    let key = key_in(&cluster, 0, 0);
    let first = run(&mut cluster, command("put", &key, 1));
    assert_eq!(first.result.kind, "inserted");
    let version = first.result.version.unwrap();
    let duplicate = ServiceLinkFaults {
        request: LoopbackFault::Duplicate,
        replica: LoopbackFault::Duplicate,
        ack: LoopbackFault::Duplicate,
    };
    let mut request = command("conditional_replace", &key, 2);
    request.expected = Some(b"v".to_vec());
    request.value = Some(b"changed".to_vec());
    let result = cluster
        .execute(
            "c",
            request.clone(),
            ServiceClusterCall {
                faults: ServiceClusterFaults {
                    data: duplicate,
                    coordinator: duplicate,
                    response: LoopbackFault::Duplicate,
                    ..ServiceClusterFaults::default()
                },
                ..ServiceClusterCall::default()
            },
        )
        .unwrap();
    assert_eq!(result.result.kind, "replaced");
    assert_eq!(result.result.value.as_deref(), Some(b"v".as_slice()));
    assert_eq!(result.result.version, Some(version + 1));
    assert!(result.backup_proved);
    let assignment = cluster.assignment(0).unwrap();
    assert_ne!(assignment.owner, "c");
    let owner = cluster
        .diagnostic_node_snapshot(&assignment.owner, 0, 0)
        .unwrap();
    let backup = cluster
        .diagnostic_node_snapshot(&assignment.backup, 0, 0)
        .unwrap();
    assert_eq!(owner, backup);
    assert_eq!(owner[0].version, version + 1);
    assert!(cluster
        .trace()
        .iter()
        .any(|event| event.stage == "owner_request" && event.delivered == 2));
    let replay = run(&mut cluster, request.clone());
    assert_eq!(replay.result, result.result);
    assert!(replay.replayed);
    request.value = Some(b"different".to_vec());
    assert!(matches!(
        cluster.execute("c", request, ServiceClusterCall::default()),
        Err(ServiceClusterError::Model(MapServiceError::Model(_)))
    ));
    assert_eq!(
        cluster
            .diagnostic_node_snapshot(&assignment.owner, 0, 0)
            .unwrap(),
        owner
    );
}

#[test]
fn bounded_bulk_fanout_keeps_root_identity_and_order_after_partial_ack() {
    let mut cluster = cluster(3);
    let keys = [
        key_in(&cluster, 2, 0),
        key_in(&cluster, 0, 0),
        key_in(&cluster, 1, 0),
    ];
    let mut request = command("put_all_detailed", b"", 1);
    request.entries = keys
        .iter()
        .enumerate()
        .map(|(n, key)| ReplacementEntry {
            key: key.clone(),
            value: vec![b'0' + n as u8],
        })
        .collect();
    let mut faults = ServiceClusterFaults::default();
    faults.partitions.insert(
        1,
        ServiceLinkFaults {
            ack: LoopbackFault::Drop,
            ..ServiceLinkFaults::default()
        },
    );
    let lost = cluster
        .execute(
            "c",
            request.clone(),
            ServiceClusterCall {
                faults,
                ..ServiceClusterCall::default()
            },
        )
        .unwrap();
    assert_eq!(lost.result.kind, "outcome_unknown");
    assert!(!lost.backup_proved);
    assert_eq!(
        cluster.query_operation_outcome(&id(1)).unwrap(),
        OperationOutcomeQuery::Pending
    );
    let mut changed = request.clone();
    changed.entries[0].value = b"bad".to_vec();
    assert!(matches!(
        cluster.execute("c", changed, ServiceClusterCall::default()),
        Err(ServiceClusterError::Model(MapServiceError::Model(_)))
    ));
    let mut expired_retry = request.clone();
    expired_retry.now = 9;
    expired_retry.deadline = 1;
    assert_eq!(
        run(&mut cluster, expired_retry).result.kind,
        "outcome_unknown"
    );
    let replay = run(&mut cluster, request.clone());
    assert_eq!(replay.result.kind, "bulk");
    assert!(replay.backup_proved);
    assert_eq!(
        replay
            .result
            .items
            .iter()
            .map(|item| item.key.clone())
            .collect::<Vec<_>>(),
        keys
    );
    assert_eq!(
        replay
            .result
            .items
            .iter()
            .map(|item| item.index)
            .collect::<Vec<_>>(),
        [0, 1, 2]
    );
    for key in &keys {
        let p = cluster.partition_for(key);
        let route = cluster.assignment(p).unwrap();
        let owner = cluster
            .diagnostic_node_snapshot(&route.owner, p, 0)
            .unwrap();
        let backup = cluster
            .diagnostic_node_snapshot(&route.backup, p, 0)
            .unwrap();
        assert_eq!(owner, backup);
        assert_eq!(owner[0].version, 2);
    }
    let mut read = ServiceCommand::point("get_all", b"");
    read.keys = keys.to_vec();
    let read = run(&mut cluster, read);
    assert_eq!(
        read.result
            .items
            .iter()
            .map(|item| item.value.clone().unwrap())
            .collect::<Vec<_>>(),
        [b"0".to_vec(), b"1".to_vec(), b"2".to_vec()]
    );
    let mut replace = command("replace_all_detailed", b"", 2);
    replace.entries = keys
        .iter()
        .map(|key| ReplacementEntry {
            key: key.clone(),
            value: b"next".to_vec(),
        })
        .collect();
    let replaced = run(&mut cluster, replace);
    assert!(replaced
        .result
        .items
        .iter()
        .all(|item| item.kind == "replaced"));
    let mut remove = command("remove_all_detailed", b"", 3);
    remove.keys = keys.to_vec();
    let removed = run(&mut cluster, remove);
    assert!(removed
        .result
        .items
        .iter()
        .all(|item| item.kind == "removed" && item.value.as_deref() == Some(b"next".as_slice())));
    assert_eq!(
        cluster.query_operation_outcome(&id(1)).unwrap(),
        OperationOutcomeQuery::Completed(Box::new(replay.result))
    );
}

#[test]
fn promotion_and_rebalance_preserve_actual_values_receipts_ttl_and_fences() {
    let mut cluster = cluster(3);
    let p = 0;
    let key = key_in(&cluster, p, 0);
    let first_id = identity_in(&cluster, p, 1);
    let second_id = identity_in(&cluster, p, first_id.sequence + 1);
    let third_id = identity_in(&cluster, p, second_id.sequence + 1);
    let mut first = command("put", &key, first_id.sequence);
    first.ttl = ServiceTtl::ExpireAfter(10);
    run(&mut cluster, first.clone());
    cluster
        .retire_outcomes("client", first_id.sequence)
        .unwrap();
    let mut second = command("get_and_put", &key, second_id.sequence);
    second.value = Some(b"second".to_vec());
    second.ttl = ServiceTtl::Preserve;
    let receipt = run(&mut cluster, second.clone()).result;
    let token = receipt.token.unwrap();
    let old = cluster.assignment(p).unwrap();
    cluster.set_node_live(&old.owner, false).unwrap();
    let promoted = cluster.promote_backup(p).unwrap();
    assert_eq!(promoted.owner, old.backup);
    assert!(promoted.epoch > old.epoch);
    assert_eq!(
        cluster.query_operation_outcome(&second_id).unwrap(),
        OperationOutcomeQuery::Completed(Box::new(receipt.clone()))
    );
    cluster
        .repair_backup(p, &promoted.backup, ServiceLinkFaults::default())
        .unwrap();
    let mut cas = command("replace_if_version", &key, third_id.sequence);
    cas.token = Some(token);
    cas.now = 3;
    cas.ttl = ServiceTtl::Preserve;
    cas.value = Some(b"third".to_vec());
    let changed = run(&mut cluster, cas).result;
    assert_eq!(changed.kind, "replaced");
    assert_eq!(changed.remaining_ttl, Some(7));
    assert_eq!(changed.token.unwrap().map_id, token.map_id);
    cluster.set_node_live(&old.owner, true).unwrap();
    assert_eq!(
        cluster
            .execute_at_owner(&old.owner, p, old.epoch, first.clone())
            .unwrap_err(),
        ServiceClusterError::StaleOwner
    );
    assert_eq!(
        cluster
            .execute(
                &old.owner,
                first.clone(),
                ServiceClusterCall {
                    expected_epoch: Some(old.epoch),
                    ..ServiceClusterCall::default()
                }
            )
            .unwrap_err(),
        ServiceClusterError::StaleEpoch
    );
    let before = cluster.assignment(p).unwrap();
    assert!(matches!(
        cluster.rebalance(
            p,
            &old.owner,
            ServiceLinkFaults {
                replica: LoopbackFault::Corrupt,
                ..ServiceLinkFaults::default()
            }
        ),
        Err(ServiceClusterError::Transport(LoopbackError::CorruptFrame))
    ));
    assert_eq!(cluster.assignment(p).unwrap(), before);
    assert!(matches!(
        cluster.rebalance(
            p,
            &old.owner,
            ServiceLinkFaults {
                ack: LoopbackFault::Drop,
                ..ServiceLinkFaults::default()
            }
        ),
        Err(ServiceClusterError::TransportTimeout)
    ));
    assert_eq!(cluster.assignment(p).unwrap(), before);
    let moved = cluster
        .rebalance(p, &old.owner, ServiceLinkFaults::default())
        .unwrap();
    assert_eq!(moved.owner, old.owner);
    assert!(moved.epoch > before.epoch);
    assert_eq!(
        cluster.query_operation_outcome(&second_id).unwrap(),
        OperationOutcomeQuery::Completed(Box::new(receipt.clone()))
    );
    assert!(matches!(
        cluster.execute("c", first, ServiceClusterCall::default()),
        Err(ServiceClusterError::Model(MapServiceError::RetiredIdentity))
    ));
    let snapshot = cluster
        .diagnostic_node_snapshot(&moved.owner, p, 3)
        .unwrap();
    assert_eq!(snapshot[0].value, b"third");
    assert_eq!(snapshot[0].remaining_ttl, Some(7));
    let mut get = ServiceCommand::point("get", &key);
    get.now = 10;
    assert_eq!(run(&mut cluster, get).result.kind, "absent");
    assert!(cluster
        .diagnostic_node_snapshot(&moved.backup, p, 10)
        .unwrap()
        .is_empty());
}

#[test]
fn loopback_faults_cannot_create_unproved_acknowledgements() {
    for fault in [
        LoopbackFault::Drop,
        LoopbackFault::Corrupt,
        LoopbackFault::Partial,
        LoopbackFault::Delay(100),
    ] {
        let mut cluster = cluster(3);
        let key = key_in(&cluster, 0, 0);
        let request = command("put", &key, 1);
        let result = cluster.execute(
            "c",
            request,
            ServiceClusterCall {
                faults: ServiceClusterFaults {
                    coordinator: ServiceLinkFaults {
                        request: fault,
                        ..ServiceLinkFaults::default()
                    },
                    ..ServiceClusterFaults::default()
                },
                ..ServiceClusterCall::default()
            },
        );
        if matches!(fault, LoopbackFault::Drop | LoopbackFault::Delay(_)) {
            assert_eq!(result.unwrap().result.kind, "not_applied");
        } else {
            assert!(matches!(
                result,
                Err(ServiceClusterError::Transport(LoopbackError::CorruptFrame))
            ));
        }
        assert_eq!(
            cluster.query_operation_outcome(&id(1)).unwrap(),
            OperationOutcomeQuery::Unknown
        );
        let owner = cluster.assignment(0).unwrap().owner;
        assert!(cluster
            .diagnostic_node_snapshot(&owner, 0, 0)
            .unwrap()
            .is_empty());
    }
    for fault in [
        LoopbackFault::Drop,
        LoopbackFault::Corrupt,
        LoopbackFault::Partial,
    ] {
        let mut cluster = cluster(3);
        let key = key_in(&cluster, 0, 0);
        let request = command("put", &key, 1);
        let result = cluster
            .execute(
                "c",
                request.clone(),
                ServiceClusterCall {
                    faults: ServiceClusterFaults {
                        data: ServiceLinkFaults {
                            replica: fault,
                            ..ServiceLinkFaults::default()
                        },
                        ..ServiceClusterFaults::default()
                    },
                    ..ServiceClusterCall::default()
                },
            )
            .unwrap();
        assert_eq!(result.result.kind, "outcome_unknown");
        assert!(!result.backup_proved);
        assert_eq!(
            cluster.promote_backup(0).unwrap_err(),
            ServiceClusterError::ReplicaNotProved
        );
        assert_eq!(run(&mut cluster, request).result.kind, "inserted");
    }
    for target in ["ack", "journal", "response"] {
        let mut cluster = cluster(3);
        let key = key_in(&cluster, 0, 0);
        let request = command("put", &key, 1);
        let mut faults = ServiceClusterFaults::default();
        match target {
            "ack" => faults.data.ack = LoopbackFault::Drop,
            "journal" => faults.coordinator.replica = LoopbackFault::Drop,
            _ => faults.response = LoopbackFault::Drop,
        }
        let result = cluster
            .execute(
                "c",
                request.clone(),
                ServiceClusterCall {
                    faults,
                    ..ServiceClusterCall::default()
                },
            )
            .unwrap();
        assert_eq!(result.result.kind, "outcome_unknown");
        if target == "response" {
            assert!(result.backup_proved);
        } else {
            assert!(!result.backup_proved);
        }
        assert_eq!(run(&mut cluster, request).result.kind, "inserted");
        let owner = cluster.assignment(0).unwrap().owner;
        assert_eq!(
            cluster.diagnostic_node_snapshot(&owner, 0, 0).unwrap()[0].version,
            2
        );
    }
    let mut delayed = cluster(3);
    let key = key_in(&delayed, 0, 0);
    let result = delayed
        .execute(
            "c",
            command("put", &key, 1),
            ServiceClusterCall {
                faults: ServiceClusterFaults {
                    coordinator: ServiceLinkFaults {
                        request: LoopbackFault::Delay(1),
                        ..ServiceLinkFaults::default()
                    },
                    ..ServiceClusterFaults::default()
                },
                ..ServiceClusterCall::default()
            },
        )
        .unwrap();
    assert_eq!(result.result.kind, "inserted");
    assert!(ServiceClusterHarness::new(
        &["a", "a"],
        ServiceClusterBounds::default(),
        CollectionBounds::default(),
        ServiceLimits::default(),
        1
    )
    .is_err());
}

#[test]
fn replica_images_preserve_incarnation_and_reject_pending_and_stale_state() {
    let owner = UnifiedIMapService::new(CollectionBounds::default(), 1).unwrap();
    let backup = UnifiedIMapService::new(CollectionBounds::default(), 1).unwrap();
    let first = owner.execute(command("put", b"a", 1)).unwrap();
    let old_image = owner.export_replica_image().unwrap();
    assert_eq!(owner.namespace_owners().resync_leases, 1);
    backup
        .install_replica_image(owner.export_replica_image().unwrap())
        .unwrap();
    assert_eq!(
        backup
            .execute(ServiceCommand::point("get_entry_view", b"a"))
            .unwrap()
            .token,
        first.token
    );
    let mut change = command("put", b"a", 2);
    change.value = Some(b"new".to_vec());
    owner.execute(change).unwrap();
    backup
        .install_replica_image(owner.export_replica_image().unwrap())
        .unwrap();
    assert_eq!(
        backup.install_replica_image(old_image).unwrap_err(),
        MapServiceError::InvalidArgument
    );
    assert_eq!(owner.namespace_owners().resync_leases, 0);
    let pending = command("put", b"b", 3);
    owner.reserve_operation(id(3), &pending).unwrap();
    assert!(matches!(
        owner.export_replica_image(),
        Err(MapServiceError::ActiveOwners)
    ));
    assert_eq!(
        owner.replica_stamp().unwrap_err(),
        MapServiceError::ActiveOwners
    );
    owner.cancel_reserved(&id(3)).unwrap();
    let registration = backup
        .register_listener(ServiceListenerOptions::default())
        .unwrap();
    assert_eq!(
        backup
            .install_replica_image(owner.export_replica_image().unwrap())
            .unwrap_err(),
        MapServiceError::ActiveOwners
    );
    drop(registration);
    assert_eq!(owner.namespace_owners().resync_leases, 0);
    let mismatch = UnifiedIMapService::new(CollectionBounds::default(), 2).unwrap();
    assert_eq!(
        mismatch
            .install_replica_image(owner.export_replica_image().unwrap())
            .unwrap_err(),
        MapServiceError::InvalidArgument
    );
    let image = owner.export_replica_image().unwrap();
    assert!(matches!(
        image.transport_payload(1),
        Err(MapServiceError::ResourceBudget("transport_payload"))
    ));
    drop(image);
    assert_eq!(owner.namespace_owners().resync_leases, 0);
    let before_fence = owner.export_replica_image().unwrap();
    let old_stamp = before_fence.stamp();
    owner.retire_outcomes("client", 1, 1).unwrap();
    let new_stamp = owner.replica_stamp().unwrap();
    assert_eq!(old_stamp.revision, new_stamp.revision);
    assert!(old_stamp.sequence < new_stamp.sequence);
    backup
        .install_replica_image(owner.export_replica_image().unwrap())
        .unwrap();
    assert_eq!(
        backup.install_replica_image(before_fence).unwrap_err(),
        MapServiceError::InvalidArgument
    );
    assert_eq!(
        backup.execute(command("put", b"a", 1)).unwrap_err(),
        MapServiceError::RetiredIdentity
    );
    let journal = UnifiedIMapService::new(CollectionBounds::default(), 1).unwrap();
    let digest = [7; 32];
    assert_eq!(
        journal.admit_external_operation(id(1), digest, 64).unwrap(),
        ExternalAdmission::Reserved
    );
    assert_eq!(
        journal.admit_external_operation(id(1), digest, 64).unwrap(),
        ExternalAdmission::Pending
    );
    let mut oversized = ServiceResult::new("inserted");
    oversized.value = Some(vec![0; 65]);
    assert!(journal
        .complete_external_operation(&id(1), digest, &oversized)
        .is_err());
    assert_eq!(
        journal.query_operation_outcome(&id(1)).unwrap(),
        OperationOutcomeQuery::Pending
    );
    journal
        .complete_external_operation(&id(1), digest, &ServiceResult::new("inserted"))
        .unwrap();
    assert!(matches!(
        journal.admit_external_operation(id(1), digest, 0).unwrap(),
        ExternalAdmission::Completed(_)
    ));
    assert!(journal.recover_external_operation(&id(1), [8; 32]).is_err());
    let old_receipts = journal.export_replica_image().unwrap();
    let old_stamp = old_receipts.stamp();
    journal
        .admit_external_operation(id(2), [8; 32], 64)
        .unwrap();
    journal
        .complete_external_operation(&id(2), [8; 32], &ServiceResult::new("removed"))
        .unwrap();
    let new_stamp = journal.replica_stamp().unwrap();
    assert_eq!(old_stamp.revision, new_stamp.revision);
    assert!(old_stamp.sequence < new_stamp.sequence);
    let receipt_backup = UnifiedIMapService::new(CollectionBounds::default(), 1).unwrap();
    receipt_backup
        .install_replica_image(journal.export_replica_image().unwrap())
        .unwrap();
    assert_eq!(
        receipt_backup
            .install_replica_image(old_receipts)
            .unwrap_err(),
        MapServiceError::InvalidArgument
    );
    assert!(
        matches!(receipt_backup.query_operation_outcome(&id(2)).unwrap(), OperationOutcomeQuery::Completed(result) if result.kind == "removed")
    );
}

#[test]
fn namespace_fence_reclamation_respects_real_pending_stream_and_snapshot_owners() {
    let service = UnifiedIMapService::new(CollectionBounds::default(), 1).unwrap();
    let old = service.clone();
    service.execute(command("put", b"a", 1)).unwrap();
    let pending = command("put", b"pending", 2);
    service.reserve_operation(id(2), &pending).unwrap();
    let source =
        ServiceListenerSource::new(service.clone(), ServiceListenerOptions::default()).unwrap();
    let listener = source.registration_id().unwrap();
    let mut stream = DemandStream::new(source, 2).unwrap();
    service.execute(command("put", b"b", 3)).unwrap();
    service.execute(command("put", b"c", 4)).unwrap();
    let mut cx = Context::from_waker(Waker::noop());
    assert!(matches!(
        Pin::new(&mut stream).poll_next(&mut cx),
        Poll::Ready(Some(Ok(_)))
    ));
    assert_eq!(stream.retained_items(), 1);
    let snapshot = service.resync_listener(listener, 0, 8, 1_024).unwrap();
    let mut pages = DemandStream::new(
        ServicePageSource::new(service.clone(), ScanProjection::Keys, 0).unwrap(),
        2,
    )
    .unwrap();
    assert!(matches!(
        Pin::new(&mut pages).poll_next(&mut cx),
        Poll::Ready(Some(Ok(_)))
    ));
    service.fence_namespace().unwrap();
    service.fence_namespace().unwrap();
    assert!(old.is_namespace_fenced());
    assert_eq!(
        old.execute(ServiceCommand::point("get", b"a")).unwrap_err(),
        MapServiceError::NamespaceDeleted
    );
    assert_eq!(
        old.reserve_operation(id(5), &command("put", b"x", 5))
            .unwrap_err(),
        MapServiceError::NamespaceDeleted
    );
    assert_eq!(
        old.recover_command(&pending, &id(2)).unwrap_err(),
        MapServiceError::NamespaceDeleted
    );
    assert_eq!(
        old.query_operation_outcome(&id(1)).unwrap_err(),
        MapServiceError::NamespaceDeleted
    );
    assert_eq!(
        old.diagnostic_snapshot(0).unwrap_err(),
        MapServiceError::NamespaceDeleted
    );
    assert!(matches!(
        Pin::new(&mut stream).poll_next(&mut cx),
        Poll::Ready(Some(Err(DemandStreamError::Source(
            MapServiceError::NamespaceDeleted
        ))))
    ));
    assert_eq!(stream.retained_items(), 0);
    assert!(matches!(
        Pin::new(&mut pages).poll_next(&mut cx),
        Poll::Ready(Some(Err(DemandStreamError::Source(
            MapServiceError::NamespaceDeleted
        ))))
    ));
    let mut progress = service.reclaim_namespace_step(1).unwrap();
    assert!(progress.reclaimed <= 1);
    assert!(progress.blocked_owners >= 2);
    while progress.remaining != 0 {
        progress = service.reclaim_namespace_step(1).unwrap();
        assert!(progress.reclaimed <= 1);
    }
    assert_eq!(service.namespace_owners().pending, 1);
    assert_eq!(service.pending_identities(), [id(2)]);
    service.cancel_reserved(&id(2)).unwrap();
    assert!(service.reclaim_namespace_step(1).unwrap().blocked_owners > 0);
    drop(snapshot);
    drop(stream);
    drop(pages);
    let progress = service.reclaim_namespace_step(1).unwrap();
    assert_eq!(progress.remaining, 0);
    assert_eq!(progress.blocked_owners, 0);
    assert_eq!(service.namespace_owners().total(), 0);
    assert_eq!(service.resource_usage(), ServiceResourceUsage::default());
}

#[test]
fn cluster_pending_cancellation_never_reopens_an_applied_identity() {
    let mut cluster = cluster(3);
    let key = key_in(&cluster, 0, 0);
    let request = command("put", &key, 1);
    let lost = cluster
        .execute(
            "c",
            request.clone(),
            ServiceClusterCall {
                faults: ServiceClusterFaults {
                    data: ServiceLinkFaults {
                        ack: LoopbackFault::Drop,
                        ..ServiceLinkFaults::default()
                    },
                    ..ServiceClusterFaults::default()
                },
                ..ServiceClusterCall::default()
            },
        )
        .unwrap();
    assert_eq!(lost.result.kind, "outcome_unknown");
    assert!(cluster.cancel_pending(&request).unwrap());
    assert!(!cluster.cancel_pending(&request).unwrap());
    assert!(
        matches!(cluster.query_operation_outcome(&id(1)).unwrap(), OperationOutcomeQuery::Completed(result) if result.kind == "outcome_unknown")
    );
    assert_eq!(
        run(&mut cluster, request.clone()).result.kind,
        "outcome_unknown"
    );
    let mut changed = request;
    changed.key = key_in(&cluster, 1, 0);
    assert!(matches!(
        cluster.execute("c", changed, ServiceClusterCall::default()),
        Err(ServiceClusterError::Model(MapServiceError::Model(_)))
    ));
    let route = cluster.assignment(0).unwrap();
    assert_eq!(
        cluster
            .diagnostic_node_snapshot(&route.owner, 0, 0)
            .unwrap()[0]
            .version,
        2
    );
}

#[test]
fn global_request_and_response_bounds_reject_before_any_cluster_owner_changes() {
    let collection = CollectionBounds {
        max_response_bytes: 50,
        ..CollectionBounds::default()
    };
    let mut cluster = ServiceClusterHarness::new(
        &["a", "b", "c"],
        ServiceClusterBounds::default(),
        collection,
        ServiceLimits::default(),
        1,
    )
    .unwrap();
    let mut keys = Vec::new();
    for p in 0..3 {
        let key = key_in(&cluster, p, 0);
        let mut put = command("put", &key, p as u64 + 1);
        put.value = b"12345678".to_vec().into();
        run(&mut cluster, put);
        keys.push(key);
    }
    let snapshots = (0..3)
        .map(|p| {
            let owner = cluster.assignment(p).unwrap().owner;
            cluster.diagnostic_node_snapshot(&owner, p, 0).unwrap()
        })
        .collect::<Vec<_>>();
    for op in ["get_all", "remove_all_detailed"] {
        let mut request = command(op, b"", 9);
        request.keys = keys.clone();
        assert!(matches!(
            cluster.execute("c", request, ServiceClusterCall::default()),
            Err(ServiceClusterError::Model(MapServiceError::Model(_)))
        ));
    }
    let mut too_many = command("put_all_detailed", b"", 10);
    too_many.entries = (0..257)
        .map(|n| ReplacementEntry {
            key: format!("{n}").into_bytes(),
            value: b"v".to_vec(),
        })
        .collect();
    assert!(cluster
        .execute("c", too_many, ServiceClusterCall::default())
        .is_err());
    for p in 0..3 {
        let owner = cluster.assignment(p).unwrap().owner;
        assert_eq!(
            cluster.diagnostic_node_snapshot(&owner, p, 0).unwrap(),
            snapshots[p as usize]
        );
    }
    assert_eq!(
        cluster.query_operation_outcome(&id(9)).unwrap(),
        OperationOutcomeQuery::Unknown
    );
    assert_eq!(
        cluster.query_operation_outcome(&id(10)).unwrap(),
        OperationOutcomeQuery::Unknown
    );
    let service = UnifiedIMapService::new(CollectionBounds::default(), 1).unwrap();
    let command = command("put", b"a", 1);
    let before = service.statistics();
    assert!(service.preflight_registry_growth(&command).unwrap() > 0);
    assert_eq!(service.statistics(), before);
    service.execute(command.clone()).unwrap();
    assert_eq!(service.preflight_registry_growth(&command).unwrap(), 0);
}
