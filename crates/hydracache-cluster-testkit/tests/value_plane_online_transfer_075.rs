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

fn scope(cluster: &ServiceClusterHarness, transfer: u64) -> TransferScope {
    let manifest = cluster.online_transfer_manifest(transfer).unwrap();
    TransferScope {
        transfer,
        partition: manifest.partition,
        source_epoch: manifest.source_epoch,
    }
}

#[test]
fn scoped_transfer_abort_cleanup_and_restart_release_actual_staging_without_losing_writes() {
    let mut cluster = cluster();
    let original = run(&mut cluster, command(b"original", 1)).result;
    let baseline = cluster.aggregate_resource_usage();
    let id = cluster
        .begin_online_transfer(0, "c", OnlineTransferOptions::default())
        .unwrap();
    let cut = scope(&cluster, id);
    let chunk = cluster
        .online_transfer_chunk(id, TransferComponent::Data, 0)
        .unwrap();
    cluster
        .accept_online_transfer_chunk(id, chunk.clone())
        .unwrap();
    run(&mut cluster, command(b"during", 2));
    let active = cluster.transfer_diagnostic(cut).unwrap();
    assert_eq!(active.scope, cut);
    assert!(active.staged_bytes > 384 && active.pending_deltas > 0);
    let before = cluster.aggregate_resource_usage();
    assert_eq!(
        cluster.cleanup_transfer_scoped(cut).unwrap_err(),
        ServiceClusterError::TransferState
    );
    assert_eq!(
        cluster
            .restart_transfer_scoped(cut, cut.source_epoch, "c", OnlineTransferOptions::default())
            .unwrap_err(),
        ServiceClusterError::TransferState
    );
    let wrong = TransferScope {
        source_epoch: cut.source_epoch + 1,
        ..cut
    };
    assert_eq!(
        cluster.abort_transfer_scoped(wrong).unwrap_err(),
        ServiceClusterError::StaleEpoch
    );
    assert_eq!(
        cluster.cleanup_transfer_scoped(wrong).unwrap_err(),
        ServiceClusterError::StaleEpoch
    );
    assert_eq!(cluster.aggregate_resource_usage(), before);
    let aborted = cluster.abort_transfer_scoped(cut).unwrap().current.unwrap();
    assert_eq!(aborted.phase, OnlineTransferPhase::Aborted);
    assert_eq!(
        (
            aborted.pending_deltas,
            aborted.held_bytes,
            aborted.unacknowledged,
            aborted.remaining_chunks,
            aborted.staged_bytes
        ),
        (0, 0, 0, 0, 384)
    );
    assert_eq!(cluster.aggregate_namespace_owners().resync_leases, 1);
    assert_eq!(
        cluster.aggregate_resource_usage().resync,
        baseline.resync + 384
    );
    assert!(cluster.accept_online_transfer_chunk(id, chunk).is_err());
    assert!(cluster.transfer_diagnostics(0).is_err());
    assert_eq!(
        cluster.transfer_diagnostics(1).unwrap(),
        vec![aborted.clone()]
    );
    assert_eq!(
        cluster
            .restart_transfer_scoped(
                cut,
                cut.source_epoch + 1,
                "c",
                OnlineTransferOptions::default()
            )
            .unwrap_err(),
        ServiceClusterError::StaleEpoch
    );
    assert_eq!(
        cluster
            .restart_transfer_scoped(
                cut,
                cut.source_epoch,
                "c",
                OnlineTransferOptions {
                    chunk_bytes: 0,
                    ..OnlineTransferOptions::default()
                }
            )
            .unwrap_err(),
        ServiceClusterError::InvalidBound
    );
    assert_eq!(cluster.transfer_diagnostic(cut).unwrap(), aborted);
    let restarted = cluster
        .restart_transfer_scoped(cut, cut.source_epoch, "c", OnlineTransferOptions::default())
        .unwrap();
    assert!(restarted.removed);
    assert_eq!(restarted.previous, cut);
    let next = restarted.current.unwrap().scope;
    assert!(next.transfer > id);
    assert_eq!(next.source_epoch, cut.source_epoch);
    assert_eq!(
        cluster.transfer_diagnostic(cut).unwrap_err(),
        ServiceClusterError::UnknownTransfer
    );
    assert!(!cluster.cleanup_transfer_scoped(cut).unwrap().removed);
    snapshot(&mut cluster, next.transfer);
    run(&mut cluster, command(b"after", 3));
    deltas(&mut cluster, next.transfer);
    cluster.ready_online_transfer(next.transfer).unwrap();
    cluster.commit_online_transfer(next.transfer).unwrap();
    assert_eq!(
        cluster.diagnostic_node_snapshot("c", 0, 0).unwrap().len(),
        3
    );
    assert_eq!(
        cluster
            .query_operation_outcome(&MutationIdentity::new("transfer", 1))
            .unwrap(),
        OperationOutcomeQuery::Completed(Box::new(original))
    );
    assert!(cluster.cleanup_transfer_scoped(next).unwrap().removed);
    assert_eq!(cluster.aggregate_namespace_owners().resync_leases, 0);
    assert_eq!(cluster.aggregate_resource_usage().resync, baseline.resync);
}

#[test]
fn scoped_restart_keeps_terminal_attempt_when_pending_or_fencing_blocks_a_fresh_snapshot() {
    let mut cluster = cluster();
    let id = cluster
        .begin_online_transfer(0, "c", OnlineTransferOptions::default())
        .unwrap();
    let cut = scope(&cluster, id);
    cluster.abort_transfer_scoped(cut).unwrap();
    let pending = command(b"pending", 1);
    cluster
        .reserve_operation(pending.identity.clone().unwrap(), &pending)
        .unwrap();
    let before = cluster.transfer_diagnostic(cut).unwrap();
    let resources = cluster.aggregate_resource_usage();
    assert_eq!(before.pending_owners, 1);
    assert!(matches!(
        cluster.restart_transfer_scoped(
            cut,
            cut.source_epoch,
            "c",
            OnlineTransferOptions::default()
        ),
        Err(ServiceClusterError::Model(MapServiceError::ActiveOwners))
    ));
    assert_eq!(cluster.transfer_diagnostic(cut).unwrap(), before);
    assert_eq!(cluster.aggregate_resource_usage(), resources);
    assert!(cluster
        .cancel_pending_identity(pending.identity.as_ref().unwrap())
        .unwrap());
    cluster.promote_backup(0).unwrap();
    let current_epoch = cluster.assignment(0).unwrap().epoch;
    assert!(current_epoch > cut.source_epoch);
    assert_eq!(
        cluster
            .restart_transfer_scoped(cut, cut.source_epoch, "c", OnlineTransferOptions::default())
            .unwrap_err(),
        ServiceClusterError::StaleEpoch
    );
    let next = cluster
        .restart_transfer_scoped(cut, current_epoch, "c", OnlineTransferOptions::default())
        .unwrap()
        .current
        .unwrap()
        .scope;
    assert!(next.transfer > id);
    assert_eq!(next.source_epoch, current_epoch);
    assert!(cluster
        .abort_transfer_scoped(TransferScope {
            transfer: 0,
            ..next
        })
        .is_err());
    assert!(cluster
        .cleanup_transfer_scoped(TransferScope {
            partition: 1,
            ..next
        })
        .is_err());
    let _ = cluster.fence_namespace_deferred();
    cluster.abort_transfer_scoped(next).unwrap();
    assert_eq!(
        cluster
            .restart_transfer_scoped(
                next,
                next.source_epoch,
                "c",
                OnlineTransferOptions::default()
            )
            .unwrap_err(),
        ServiceClusterError::Model(MapServiceError::NamespaceDeleted)
    );
    assert!(cluster.cleanup_transfer_scoped(next).unwrap().removed);
    assert!(cluster.transfer_diagnostics(1).unwrap().is_empty());
    assert_eq!(
        cluster.transfer_diagnostic(next).unwrap_err(),
        ServiceClusterError::UnknownTransfer
    );
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
fn byte_budget_exhaustion_is_resumable_before_any_source_write() {
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
    assert_eq!(
        cluster
            .execute("c", command(b"a", 1), ServiceClusterCall::default())
            .unwrap_err(),
        ServiceClusterError::TransferBackpressure
    );
    let readiness = cluster
        .transfer_readiness()
        .into_iter()
        .find(|state| state.id == id)
        .unwrap();
    assert_eq!(readiness.phase, OnlineTransferPhase::Snapshot);
    assert!(readiness.restart_reason.is_none());
    assert!(cluster.commit_online_transfer(id).is_err());
    assert_eq!(
        cluster.diagnostic_node_snapshot("a", 0, 0).unwrap().len(),
        0
    );
    cluster.abort_online_transfer(id).unwrap();
    cluster.cleanup_online_transfer(id).unwrap();
    let next = cluster
        .begin_online_transfer(0, "c", OnlineTransferOptions::default())
        .unwrap();
    snapshot(&mut cluster, next);
    cluster.ready_online_transfer(next).unwrap();
    cluster.commit_online_transfer(next).unwrap();
    assert_eq!(
        cluster.diagnostic_node_snapshot("c", 0, 0).unwrap().len(),
        0
    );
    cluster.cleanup_online_transfer(next).unwrap();
}

#[test]
fn delta_credits_reserve_before_journal_and_resume_after_apply() {
    let mut cluster = cluster();
    let transfer = cluster
        .begin_online_transfer(
            0,
            "c",
            OnlineTransferOptions {
                max_delta_bytes: 16 * 1_024,
                max_delta_images: 64,
                ..OnlineTransferOptions::default()
            },
        )
        .unwrap();
    snapshot(&mut cluster, transfer);
    let mut refused = None;
    for sequence in 1..32 {
        let request = command(&[sequence as u8], sequence);
        let before = cluster.diagnostic_node_snapshot("a", 0, 0).unwrap();
        let usage = cluster.aggregate_resource_usage();
        match cluster.execute("c", request.clone(), ServiceClusterCall::default()) {
            Ok(result) => assert!(result.backup_proved),
            Err(ServiceClusterError::TransferBackpressure) => {
                assert_eq!(cluster.diagnostic_node_snapshot("a", 0, 0).unwrap(), before);
                assert_eq!(cluster.aggregate_resource_usage(), usage);
                assert_eq!(
                    cluster
                        .query_operation_outcome(request.identity.as_ref().unwrap())
                        .unwrap(),
                    OperationOutcomeQuery::Unknown
                );
                refused = Some(request);
                break;
            }
            other => panic!("unexpected result {other:?}"),
        }
    }
    let refused = refused.expect("byte budget must backpressure before 64 delta records");
    let readiness = cluster
        .transfer_readiness()
        .into_iter()
        .find(|state| state.id == transfer)
        .unwrap();
    assert_eq!(readiness.phase, OnlineTransferPhase::Delta);
    assert!(readiness.restart_reason.is_none());
    assert!(readiness.pending_deltas < 64);
    deltas(&mut cluster, transfer);
    assert!(run(&mut cluster, refused).backup_proved);
    deltas(&mut cluster, transfer);
    cluster.ready_online_transfer(transfer).unwrap();
    cluster.commit_online_transfer(transfer).unwrap();
    cluster.cleanup_online_transfer(transfer).unwrap();
    assert_eq!(cluster.aggregate_namespace_owners().resync_leases, 0);
}

#[test]
fn pending_root_delta_credit_is_idempotent_and_released_by_resumed_decisions() {
    let mut cluster = cluster();
    let transfer = cluster
        .begin_online_transfer(
            0,
            "c",
            OnlineTransferOptions {
                max_delta_bytes: 16 * 1_024,
                ..OnlineTransferOptions::default()
            },
        )
        .unwrap();
    snapshot(&mut cluster, transfer);
    let original = command(b"reserved", 1);
    let before = cluster
        .transfer_readiness()
        .into_iter()
        .find(|state| state.id == transfer)
        .unwrap()
        .staged_bytes;
    cluster
        .reserve_operation(original.identity.clone().unwrap(), &original)
        .unwrap();
    let held = cluster
        .transfer_readiness()
        .into_iter()
        .find(|state| state.id == transfer)
        .unwrap()
        .staged_bytes;
    assert!(held > before);
    assert_eq!(cluster.aggregate_namespace_owners().pending, 2);
    assert!(cluster
        .diagnostic_node_snapshot("a", 0, 0)
        .unwrap()
        .is_empty());
    cluster
        .reserve_operation(original.identity.clone().unwrap(), &original)
        .unwrap();
    assert_eq!(
        cluster
            .transfer_readiness()
            .into_iter()
            .find(|state| state.id == transfer)
            .unwrap()
            .staged_bytes,
        held
    );
    assert!(cluster.ready_online_transfer(transfer).is_err());
    assert!(run(&mut cluster, original).backup_proved);
    assert_eq!(cluster.aggregate_namespace_owners().pending, 0);
    deltas(&mut cluster, transfer);
    cluster.ready_online_transfer(transfer).unwrap();
    cluster.commit_online_transfer(transfer).unwrap();
    cluster.cleanup_online_transfer(transfer).unwrap();
    assert_eq!(cluster.aggregate_namespace_owners().resync_leases, 0);
}

#[test]
fn retirement_delta_credit_rejects_before_receipts_and_fences_change() {
    let mut cluster = cluster();
    let original = command(b"key", 1);
    let applied = run(&mut cluster, original.clone()).result;
    let transfer = cluster
        .begin_online_transfer(
            0,
            "c",
            OnlineTransferOptions {
                max_delta_bytes: 1,
                ..OnlineTransferOptions::default()
            },
        )
        .unwrap();
    let before = cluster.aggregate_resource_usage();
    assert_eq!(
        cluster.retire_outcomes("transfer", 1).unwrap_err(),
        ServiceClusterError::TransferBackpressure
    );
    assert_eq!(cluster.aggregate_resource_usage(), before);
    assert_eq!(
        cluster
            .query_operation_outcome(original.identity.as_ref().unwrap())
            .unwrap(),
        OperationOutcomeQuery::Completed(Box::new(applied))
    );
    cluster.abort_online_transfer(transfer).unwrap();
    cluster.cleanup_online_transfer(transfer).unwrap();
    cluster.retire_outcomes("transfer", 1).unwrap();
    let next = cluster
        .begin_online_transfer(
            0,
            "c",
            OnlineTransferOptions {
                max_delta_bytes: 1,
                ..OnlineTransferOptions::default()
            },
        )
        .unwrap();
    cluster.retire_outcomes("transfer", 1).unwrap();
    cluster.abort_online_transfer(next).unwrap();
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
