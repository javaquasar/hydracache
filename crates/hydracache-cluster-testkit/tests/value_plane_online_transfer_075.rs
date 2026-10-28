use hydracache_cluster_testkit::value_plane_collection_075::CollectionBounds;
use hydracache_cluster_testkit::value_plane_model_075::MutationIdentity;
use hydracache_cluster_testkit::value_plane_service_075::*;
use hydracache_cluster_testkit::value_plane_service_cluster_075::*;

fn cluster() -> ServiceClusterHarness {
    ServiceClusterHarness::new(
        &["a", "b", "c"],
        ServiceClusterBounds {
            partitions: 1,
            ..ServiceClusterBounds::default()
        },
        CollectionBounds::default(),
        ServiceLimits::default(),
        1,
    )
    .unwrap()
}
fn command(key: &[u8], n: u64) -> ServiceCommand {
    let mut c = ServiceCommand::point("put", key);
    c.identity = Some(MutationIdentity::new("transfer", n));
    c.value = Some(vec![n as u8]);
    c
}
fn run(cluster: &mut ServiceClusterHarness, c: ServiceCommand) -> ServiceClusterExecution {
    cluster
        .execute("c", c, ServiceClusterCall::default())
        .unwrap()
}
fn snapshot(cluster: &mut ServiceClusterHarness, id: u64) {
    let manifest = cluster.online_transfer_manifest(id).unwrap();
    for part in manifest.snapshots {
        for index in 0..part.chunk_count {
            let chunk = cluster
                .online_transfer_chunk(id, part.component, index)
                .unwrap();
            cluster.accept_online_transfer_chunk(id, chunk).unwrap();
        }
    }
    cluster.finish_online_snapshot(id).unwrap();
}
fn deltas(cluster: &mut ServiceClusterHarness, id: u64) {
    while let Some(delta) = cluster.next_online_delta(id).unwrap() {
        cluster
            .apply_online_delta(id, delta.sequence, delta.checksum)
            .unwrap();
    }
}

#[test]
fn sparse_online_deltas_exclude_unchanged_payload_and_preserve_decisions() {
    let mut cluster = cluster();
    let mut large = command(b"unchanged", 1);
    large.value = Some(vec![173; 32 * 1_024]);
    run(&mut cluster, large);
    let mut expiring = command(b"removed", 2);
    expiring.ttl = ServiceTtl::ExpireAfter(20);
    run(&mut cluster, expiring);
    let id = cluster
        .begin_online_transfer(0, "c", OnlineTransferOptions::default())
        .unwrap();
    let original_bytes = cluster
        .online_transfer_manifest(id)
        .unwrap()
        .snapshots
        .iter()
        .map(|part| part.total_bytes)
        .sum::<usize>();
    snapshot(&mut cluster, id);
    let receipt = run(&mut cluster, command(b"small", 3)).result;
    let first = cluster.next_online_delta(id).unwrap().unwrap();
    let measured = cluster.online_delta_measure(id, first.sequence).unwrap();
    assert_eq!(measured.entry_upserts, 1);
    assert_eq!(measured.entry_deletions, 0);
    assert_eq!(measured.outcome_upserts, 1);
    assert!(first.payload_bytes * 10 < original_bytes);
    deltas(&mut cluster, id);
    let mut remove = ServiceCommand::point("delete", b"removed");
    remove.identity = Some(MutationIdentity::new("transfer", 4));
    let removed = run(&mut cluster, remove.clone()).result;
    let deletion = cluster.next_online_delta(id).unwrap().unwrap();
    assert_eq!(
        cluster
            .online_delta_measure(id, deletion.sequence)
            .unwrap()
            .entry_deletions,
        1
    );
    deltas(&mut cluster, id);
    cluster.retire_outcomes("transfer", 2).unwrap();
    let fence = cluster.next_online_delta(id).unwrap().unwrap();
    let measured = cluster.online_delta_measure(id, fence.sequence).unwrap();
    assert_eq!(measured.entry_upserts + measured.entry_deletions, 0);
    assert_eq!(measured.fence_upserts, 1);
    assert_eq!(measured.outcome_deletions, 2);
    deltas(&mut cluster, id);
    cluster.ready_online_transfer(id).unwrap();
    cluster.commit_online_transfer(id).unwrap();
    let state = cluster.diagnostic_node_snapshot("c", 0, 0).unwrap();
    assert_eq!(state.len(), 2);
    assert_eq!(
        state
            .iter()
            .find(|entry| entry.key == b"unchanged")
            .unwrap()
            .value,
        vec![173; 32 * 1_024]
    );
    assert_eq!(
        cluster
            .query_operation_outcome(&MutationIdentity::new("transfer", 3))
            .unwrap(),
        OperationOutcomeQuery::Completed(Box::new(receipt))
    );
    assert_eq!(run(&mut cluster, remove).result, removed);
    assert!(matches!(
        cluster.execute("c", command(b"unchanged", 1), ServiceClusterCall::default()),
        Err(ServiceClusterError::Model(MapServiceError::RetiredIdentity))
    ));
    cluster.cleanup_online_transfer(id).unwrap();
    assert_eq!(cluster.aggregate_namespace_owners().resync_leases, 0);
}

#[test]
fn online_snapshot_resumes_chunks_and_applies_actual_live_decision_deltas() {
    let mut cluster = cluster();
    let mut first = command(b"a", 1);
    first.ttl = ServiceTtl::ExpireAfter(10);
    run(&mut cluster, first.clone());
    let id = cluster
        .begin_online_transfer(
            0,
            "c",
            OnlineTransferOptions {
                chunk_bytes: 128,
                ..OnlineTransferOptions::default()
            },
        )
        .unwrap();
    let manifest = cluster.online_transfer_manifest(id).unwrap();
    let first_part = &manifest.snapshots[0];
    let chunk = cluster
        .online_transfer_chunk(id, first_part.component, 0)
        .unwrap();
    assert_eq!(
        cluster
            .accept_online_transfer_chunk(id, chunk.clone())
            .unwrap(),
        TransferAcceptance::Stored
    );
    assert_eq!(
        cluster
            .accept_online_transfer_chunk(id, chunk.clone())
            .unwrap(),
        TransferAcceptance::Replayed
    );
    let mut bad = chunk;
    bad.bytes[0] ^= 1;
    assert_eq!(
        cluster.accept_online_transfer_chunk(id, bad).unwrap_err(),
        ServiceClusterError::ConflictingChunk
    );
    assert_eq!(
        cluster.finish_online_snapshot(id).unwrap_err(),
        ServiceClusterError::MissingChunks
    );
    let second = run(&mut cluster, command(b"b", 2)).result;
    cluster.retire_outcomes("transfer", 1).unwrap();
    snapshot(&mut cluster, id);
    assert!(cluster.next_online_delta(id).unwrap().is_some());
    let first_delta = cluster.next_online_delta(id).unwrap().unwrap();
    assert_eq!(
        cluster
            .apply_online_delta(id, first_delta.sequence + 1, first_delta.checksum)
            .unwrap_err(),
        ServiceClusterError::DeltaOutOfOrder
    );
    assert_eq!(
        cluster
            .apply_online_delta(id, first_delta.sequence, [0; 32])
            .unwrap_err(),
        ServiceClusterError::ConflictingDelta
    );
    cluster
        .apply_online_delta(id, first_delta.sequence, first_delta.checksum)
        .unwrap();
    assert_eq!(
        cluster
            .apply_online_delta(id, first_delta.sequence, first_delta.checksum)
            .unwrap(),
        TransferAcceptance::Replayed
    );
    deltas(&mut cluster, id);
    assert!(cluster.ready_online_transfer(id).unwrap().ready);
    let changed = run(&mut cluster, command(b"later", 3));
    assert_eq!(changed.result.kind, "inserted");
    assert_eq!(
        cluster.commit_online_transfer(id).unwrap_err(),
        ServiceClusterError::TransferState
    );
    deltas(&mut cluster, id);
    cluster.ready_online_transfer(id).unwrap();
    let moved = cluster.commit_online_transfer(id).unwrap();
    assert_eq!(moved.owner, "c");
    assert_eq!(
        cluster
            .query_operation_outcome(&MutationIdentity::new("transfer", 2))
            .unwrap(),
        OperationOutcomeQuery::Completed(Box::new(second))
    );
    assert!(matches!(
        cluster.execute("c", first, ServiceClusterCall::default()),
        Err(ServiceClusterError::Model(MapServiceError::RetiredIdentity))
    ));
    let values = cluster.diagnostic_node_snapshot("c", 0, 0).unwrap();
    assert_eq!(values.len(), 3);
    assert_eq!(
        values
            .iter()
            .find(|entry| entry.key == b"a")
            .unwrap()
            .remaining_ttl,
        Some(10)
    );
    assert!(cluster.cleanup_online_transfer(id).unwrap());
    assert_eq!(cluster.aggregate_namespace_owners().resync_leases, 0);
}

#[test]
fn online_transfer_backpressure_never_drops_a_source_write_or_early_cutover() {
    let mut cluster = cluster();
    let id = cluster
        .begin_online_transfer(
            0,
            "c",
            OnlineTransferOptions {
                max_delta_images: 2,
                ..OnlineTransferOptions::default()
            },
        )
        .unwrap();
    run(&mut cluster, command(b"a", 1));
    assert_eq!(
        cluster
            .execute("c", command(b"b", 2), ServiceClusterCall::default())
            .unwrap_err(),
        ServiceClusterError::TransferBackpressure
    );
    assert_eq!(
        cluster
            .query_operation_outcome(&MutationIdentity::new("transfer", 2))
            .unwrap(),
        OperationOutcomeQuery::Unknown
    );
    assert!(cluster.commit_online_transfer(id).is_err());
    snapshot(&mut cluster, id);
    deltas(&mut cluster, id);
    run(&mut cluster, command(b"b", 2));
    deltas(&mut cluster, id);
    cluster.ready_online_transfer(id).unwrap();
    cluster.commit_online_transfer(id).unwrap();
    assert_eq!(
        cluster.diagnostic_node_snapshot("c", 0, 0).unwrap().len(),
        2
    );
    cluster.cleanup_online_transfer(id).unwrap();
}

#[test]
fn byte_overflow_requires_explicit_restart_and_keeps_confirmed_source_state() {
    let mut cluster = cluster();
    let id = cluster
        .begin_online_transfer(
            0,
            "c",
            OnlineTransferOptions {
                max_delta_bytes: 1,
                ..OnlineTransferOptions::default()
            },
        )
        .unwrap();
    assert_eq!(run(&mut cluster, command(b"a", 1)).result.kind, "inserted");
    let readiness = cluster
        .transfer_readiness()
        .into_iter()
        .find(|state| state.id == id)
        .unwrap();
    assert_eq!(readiness.phase, OnlineTransferPhase::RestartRequired);
    assert!(readiness.restart_reason.is_some());
    assert!(cluster.commit_online_transfer(id).is_err());
    assert_eq!(
        cluster.diagnostic_node_snapshot("a", 0, 0).unwrap().len(),
        1
    );
    cluster.cleanup_online_transfer(id).unwrap();
    let next = cluster
        .begin_online_transfer(0, "c", OnlineTransferOptions::default())
        .unwrap();
    snapshot(&mut cluster, next);
    cluster.ready_online_transfer(next).unwrap();
    cluster.commit_online_transfer(next).unwrap();
    assert_eq!(
        cluster.diagnostic_node_snapshot("c", 0, 0).unwrap().len(),
        1
    );
    cluster.cleanup_online_transfer(next).unwrap();
}

#[test]
fn pending_roots_and_topology_changes_fence_online_readiness() {
    let mut cluster = cluster();
    let id = cluster
        .begin_online_transfer(0, "c", OnlineTransferOptions::default())
        .unwrap();
    snapshot(&mut cluster, id);
    let mut call = ServiceClusterCall::default();
    call.faults.data.ack =
        hydracache_cluster_testkit::value_plane_loopback_075::LoopbackFault::Drop;
    let pending = command(b"a", 1);
    assert_eq!(
        cluster
            .execute("c", pending.clone(), call)
            .unwrap()
            .result
            .kind,
        "outcome_unknown"
    );
    deltas(&mut cluster, id);
    assert!(cluster.ready_online_transfer(id).is_err());
    run(&mut cluster, pending);
    deltas(&mut cluster, id);
    cluster.ready_online_transfer(id).unwrap();
    cluster.set_node_live("c", false).unwrap();
    assert!(cluster.commit_online_transfer(id).is_err());
    cluster.set_node_live("c", true).unwrap();
    cluster.abort_online_transfer(id).unwrap();
    assert!(cluster.cleanup_online_transfer(id).unwrap());
    let id = cluster
        .begin_online_transfer(0, "c", OnlineTransferOptions::default())
        .unwrap();
    cluster.set_node_live("a", false).unwrap();
    cluster.promote_backup(0).unwrap();
    assert!(cluster.commit_online_transfer(id).is_err());
    cluster.abort_online_transfer(id).unwrap();
    cluster.cleanup_online_transfer(id).unwrap();
}

#[test]
fn online_transfer_bounds_abort_and_namespace_cleanup_release_actual_leases() {
    let mut cluster = cluster();
    assert!(cluster
        .begin_online_transfer(0, "a", OnlineTransferOptions::default())
        .is_err());
    assert!(cluster
        .begin_online_transfer(
            0,
            "c",
            OnlineTransferOptions {
                chunk_bytes: 0,
                ..OnlineTransferOptions::default()
            }
        )
        .is_err());
    let id = cluster
        .begin_online_transfer(0, "c", OnlineTransferOptions::default())
        .unwrap();
    assert!(cluster
        .begin_online_transfer(0, "b", OnlineTransferOptions::default())
        .is_err());
    assert!(cluster
        .online_transfer_chunk(id, TransferComponent::Data, usize::MAX)
        .is_err());
    assert!(cluster.cleanup_online_transfer(id).is_err());
    cluster.abort_online_transfer(id).unwrap();
    assert!(cluster.cleanup_online_transfer(id).unwrap());
    assert!(!cluster.cleanup_online_transfer(id).unwrap());
    assert_eq!(cluster.aggregate_namespace_owners().resync_leases, 0);
    let id = cluster
        .begin_online_transfer(0, "c", OnlineTransferOptions::default())
        .unwrap();
    let wakes = cluster.fence_namespace_deferred();
    for wake in wakes {
        wake.wake();
    }
    assert!(cluster.finish_online_snapshot(id).is_err());
    while cluster.aggregate_namespace_owners().total() != 0 {
        cluster.reclaim_namespace_step(256).unwrap();
    }
    assert_eq!(
        cluster.aggregate_resource_usage(),
        ServiceResourceUsage::default()
    );
    assert!(cluster.empty_successor(2).is_ok());
}
