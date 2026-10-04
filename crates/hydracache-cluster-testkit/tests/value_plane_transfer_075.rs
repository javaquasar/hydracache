use hydracache_cluster_testkit::value_plane_transfer_075::{
    digest_chunks, ChunkAcceptance, PartitionTransfer, TransferBounds, TransferError,
    TransferManifest, TransferPhase,
};

fn transfer() -> PartitionTransfer {
    let chunks = [b"snapshot-a".as_slice(), b"snapshot-b".as_slice()];
    PartitionTransfer::new(
        TransferBounds {
            max_chunks: 4,
            max_chunk_bytes: 16,
            max_delta_entries: 2,
        },
        TransferManifest {
            partition: 7,
            source_epoch: 3,
            source: "node-a".into(),
            target: "node-b".into(),
            chunk_count: chunks.len(),
            snapshot_sha256: digest_chunks(&chunks),
        },
    )
    .unwrap()
}

#[test]
fn transfer_resumes_from_first_missing_verified_chunk() {
    let mut transfer = transfer();
    transfer.accept_chunk(1, b"snapshot-b".to_vec()).unwrap();
    assert_eq!(transfer.resume_from(), Some(0));
    assert_eq!(
        transfer.accept_chunk(1, b"snapshot-b".to_vec()).unwrap(),
        ChunkAcceptance::Replayed
    );
    transfer.accept_chunk(0, b"snapshot-a".to_vec()).unwrap();
    assert_eq!(transfer.resume_from(), None);
    transfer.finish_snapshot().unwrap();
    assert_eq!(transfer.phase(), TransferPhase::Delta);
}

#[test]
fn target_never_serves_before_cutover_and_source_is_fenced_after() {
    let mut transfer = transfer();
    transfer.accept_chunk(0, b"snapshot-a".to_vec()).unwrap();
    transfer.accept_chunk(1, b"snapshot-b".to_vec()).unwrap();
    transfer.finish_snapshot().unwrap();
    transfer.stage_delta(8, 80).unwrap();
    let digest = transfer.state_sha256();
    transfer.mark_ready(8, &digest).unwrap();
    assert!(transfer.source_can_serve());
    assert!(!transfer.target_can_serve());
    transfer.commit(4).unwrap();
    assert!(!transfer.source_can_serve());
    assert!(transfer.target_can_serve());
    assert_eq!(transfer.committed_epoch(), Some(4));
    assert_eq!(transfer.cutover_version(), Some(8));
}

#[test]
fn checksum_conflicts_bounds_and_early_commit_fail_loud() {
    let mut transfer = transfer();
    transfer.accept_chunk(0, b"snapshot-a".to_vec()).unwrap();
    assert_eq!(
        transfer.accept_chunk(0, b"different".to_vec()),
        Err(TransferError::ConflictingChunk)
    );
    assert_eq!(
        transfer.finish_snapshot(),
        Err(TransferError::MissingChunks)
    );
    assert!(matches!(
        transfer.commit(4),
        Err(TransferError::WrongPhase { .. })
    ));
    assert_eq!(
        transfer.accept_chunk(2, Vec::new()),
        Err(TransferError::ChunkOutOfRange)
    );
}

#[test]
fn rollback_clears_staging_but_cannot_undo_committed_cutover() {
    let mut rolled_back = transfer();
    rolled_back.accept_chunk(0, b"snapshot-a".to_vec()).unwrap();
    rolled_back.rollback().unwrap();
    assert_eq!(rolled_back.phase(), TransferPhase::RolledBack);
    assert!(!rolled_back.target_can_serve());

    let mut committed = transfer();
    committed.accept_chunk(0, b"snapshot-a".to_vec()).unwrap();
    committed.accept_chunk(1, b"snapshot-b".to_vec()).unwrap();
    committed.finish_snapshot().unwrap();
    let digest = committed.state_sha256();
    committed.mark_ready(0, &digest).unwrap();
    committed.commit(4).unwrap();
    assert_eq!(committed.rollback(), Err(TransferError::AlreadyCommitted));
}
