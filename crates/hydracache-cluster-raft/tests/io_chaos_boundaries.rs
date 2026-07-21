#![cfg(all(feature = "test-failpoints", feature = "sled-log-store"))]

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use hydracache::{
    ClusterCandidate, ClusterControlPlane, ClusterEpoch, ClusterGeneration, ClusterNodeId,
    RaftMetadataCommand,
};
use hydracache_cluster_raft::{
    RaftLogStore, RaftMetadataCommandEnvelope, RaftMetadataRuntime, RaftMetadataRuntimeConfig,
    RaftStorageFaultMode, RaftStorageFaultOperation, RaftWireMessage, SledRaftLogStore,
};
use raft::eraftpb::{ConfState, Message, MessageType, Snapshot};
use raft::storage::Storage;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

static TEST_DIRECTORY_SEQUENCE: AtomicU64 = AtomicU64::new(0);
const SLED_REOPEN_ATTEMPTS: usize = 50;
const SLED_REOPEN_DELAY: Duration = Duration::from_millis(10);

struct SledTestDirectory {
    path: PathBuf,
}

impl SledTestDirectory {
    fn new(label: &str) -> TestResult<Self> {
        let sequence = TEST_DIRECTORY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = PathBuf::from(format!(
            "target/test-hydracache-raft-io-chaos/{label}-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&path)?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for SledTestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn open_store(path: &Path, voters: Vec<u64>) -> TestResult<SledRaftLogStore> {
    let store = SledRaftLogStore::open(path)?;
    let conf_state = ConfState::from((voters, Vec::<u64>::new()));
    store.initialize_with_conf_state(conf_state.clone());
    store.save_conf_state(&conf_state)?;
    Ok(store)
}

fn reopen_store(path: &Path) -> TestResult<SledRaftLogStore> {
    for attempt in 0..SLED_REOPEN_ATTEMPTS {
        match SledRaftLogStore::open(path) {
            Err(error)
                if error.to_string().contains("could not acquire lock")
                    && attempt + 1 < SLED_REOPEN_ATTEMPTS =>
            {
                thread::sleep(SLED_REOPEN_DELAY);
            }
            result => return result.map_err(Into::into),
        }
    }
    unreachable!("bounded Sled reopen loop always returns")
}

fn metadata_snapshot_message(index: u64, receiver: u64) -> RaftWireMessage {
    let envelope = RaftMetadataCommandEnvelope {
        command_id: "member-upsert:member-snapshot:1".to_owned(),
        command: RaftMetadataCommand::MemberUpsert {
            node_id: ClusterNodeId::from("member-snapshot"),
            generation: ClusterGeneration::new(1),
            epoch: ClusterEpoch::new(1),
        },
    };
    let payload = serde_json::json!({
        "format_version": 1_u32,
        "cluster_name": "orders",
        "source_raft_node_id": 1_u64,
        "applied_index": index,
        "commands": [envelope],
    });
    let mut data = b"HCMETA01".to_vec();
    data.extend(serde_json::to_vec(&payload).expect("metadata snapshot payload encodes"));

    let mut snapshot = Snapshot::default();
    snapshot.mut_metadata().index = index;
    snapshot.mut_metadata().term = 1;
    snapshot.mut_metadata().mut_conf_state().voters = vec![1, 2, 3];
    snapshot.data = data.into();
    let mut message = Message {
        from: 1,
        to: receiver,
        term: 1,
        ..Message::default()
    };
    message.set_msg_type(MessageType::MsgSnapshot);
    message.set_snapshot(snapshot);
    RaftWireMessage::encode(&message).expect("snapshot wire message encodes")
}

#[tokio::test]
async fn slow_disk_during_snapshot_save_has_bounded_backpressure() -> TestResult {
    let directory = SledTestDirectory::new("slow-snapshot-save")?;
    let store = open_store(directory.path(), vec![1])?;
    let runtime = Arc::new(RaftMetadataRuntime::with_storage(
        RaftMetadataRuntimeConfig::single_node("orders", 1),
        store.clone(),
    )?);
    runtime
        .join_member(
            ClusterCandidate::member("member-before-compaction")
                .generation(ClusterGeneration::new(1)),
        )
        .await?;
    let applied_index = runtime.snapshot().applied_index;
    let durable_before = store.snapshot(0, 1)?.get_metadata().index;
    let faults = store.storage_faults();
    faults.arm(
        RaftStorageFaultOperation::SaveSnapshot,
        RaftStorageFaultMode::BlockThenContinue,
    );

    let compact_runtime = Arc::clone(&runtime);
    let compact = std::thread::spawn(move || compact_runtime.compact_applied_log_to_snapshot());
    let blocked = faults.wait_until_blocked();
    assert_eq!(blocked.calls, 1);
    assert_eq!(blocked.in_flight, 1);
    assert_eq!(blocked.max_in_flight, 1);
    assert_eq!(
        store.staged_sled_snapshot_index()?,
        Some(applied_index),
        "fault must block after the atomic Sled batch is visible"
    );

    let mut competing = Snapshot::default();
    competing.mut_metadata().index = applied_index;
    let error = store
        .save_snapshot(&competing, usize::MAX)
        .expect_err("one held snapshot must apply bounded backpressure");
    assert!(error.to_string().contains("storage backpressure"));
    assert_eq!(
        store.snapshot(0, 1)?.get_metadata().index,
        durable_before,
        "blocked and rejected snapshot saves must not expose partial state"
    );
    assert_eq!(store.staged_sled_snapshot_index()?, Some(applied_index));

    faults.release_blocked();
    let compacted = compact
        .join()
        .map_err(|_| "snapshot compaction thread panicked")??;
    assert_eq!(compacted, applied_index);
    assert_eq!(store.snapshot(0, 1)?.get_metadata().index, applied_index);
    let completed = faults.observation();
    assert_eq!(completed.calls, 2);
    assert_eq!(completed.blocked_calls, 1);
    assert_eq!(completed.backpressure_rejections, 1);
    assert_eq!(completed.injected_failures, 0);
    assert_eq!(completed.in_flight, 0);
    assert_eq!(completed.max_in_flight, 1);

    drop(runtime);
    drop(store);
    drop(directory);
    Ok(())
}

#[test]
fn slow_disk_during_snapshot_install_retries_without_partial_apply() -> TestResult {
    let directory = SledTestDirectory::new("slow-snapshot-install")?;
    let store = open_store(directory.path(), vec![1, 2, 3])?;
    let runtime = Arc::new(RaftMetadataRuntime::with_storage(
        RaftMetadataRuntimeConfig::multi_voter("orders", 3, [1, 2, 3]),
        store.clone(),
    )?);
    let faults = store.storage_faults();
    faults.arm(
        RaftStorageFaultOperation::SaveSnapshot,
        RaftStorageFaultMode::BlockThenFail,
    );

    let install_runtime = Arc::clone(&runtime);
    let install =
        std::thread::spawn(move || install_runtime.step(metadata_snapshot_message(50, 3)));
    let blocked = faults.wait_until_blocked();
    assert_eq!(blocked.in_flight, 1);
    assert!(runtime.members().is_empty());
    assert_eq!(store.snapshot(0, 3)?.get_metadata().index, 0);
    assert_eq!(
        store.staged_sled_snapshot_index()?,
        Some(50),
        "install fault must run after the atomic Sled batch"
    );

    faults.release_blocked();
    let error = install
        .join()
        .map_err(|_| "snapshot install thread panicked")?
        .expect_err("released install fault must fail loud");
    assert!(error
        .to_string()
        .contains("injected storage fault during snapshot save"));
    assert!(runtime.members().is_empty());
    assert_eq!(runtime.snapshot().snapshot_installs, 0);
    assert_eq!(store.snapshot(0, 3)?.get_metadata().index, 0);
    assert_eq!(store.staged_sled_snapshot_index()?, Some(50));

    runtime.drain_ready()?;
    let installed = runtime.snapshot();
    assert_eq!(installed.snapshot_installs, 1);
    assert_eq!(installed.applied_index, 50);
    assert!(runtime.command_applied("member-upsert:member-snapshot:1"));
    assert_eq!(store.snapshot(0, 3)?.get_metadata().index, 50);
    let completed = faults.observation();
    assert_eq!(completed.calls, 2);
    assert_eq!(completed.blocked_calls, 1);
    assert_eq!(completed.injected_failures, 1);
    assert_eq!(completed.backpressure_rejections, 0);
    assert_eq!(completed.in_flight, 0);

    drop(runtime);
    drop(store);
    drop(directory);
    Ok(())
}

#[test]
fn applied_index_io_failure_is_returned_and_retry_publishes_after_flush() -> TestResult {
    let directory = SledTestDirectory::new("applied-index-failure")?;
    let store = open_store(directory.path(), vec![1])?;
    let faults = store.storage_faults();
    faults.arm(
        RaftStorageFaultOperation::MarkApplied,
        RaftStorageFaultMode::FailImmediately,
    );

    let error = RaftLogStore::mark_applied(&store, 7)
        .expect_err("applied-index Sled fault must be returned to the caller");

    assert!(error
        .to_string()
        .contains("injected storage fault during applied-index save after sled batch"));
    assert_eq!(store.applied_index()?, 0);
    assert_eq!(store.staged_sled_applied_index()?, Some(7));
    let failed = faults.observation();
    assert_eq!(failed.calls, 1);
    assert_eq!(failed.injected_failures, 1);
    assert_eq!(failed.in_flight, 0);

    RaftLogStore::mark_applied(&store, 7)?;
    assert_eq!(store.applied_index()?, 7);
    drop(store);

    let reopened = reopen_store(directory.path())?;
    assert_eq!(reopened.applied_index()?, 7);
    drop(reopened);
    drop(directory);
    Ok(())
}

#[tokio::test]
async fn durable_commit_failure_fails_loud_and_recovers_consistent() -> TestResult {
    let directory = SledTestDirectory::new("durable-commit-failure")?;
    let store = open_store(directory.path(), vec![1])?;
    let config = RaftMetadataRuntimeConfig::single_node("orders", 1);
    let runtime = RaftMetadataRuntime::with_storage(config.clone(), store.clone())?;
    let faults = store.storage_faults();
    faults.arm(
        RaftStorageFaultOperation::DurableCommit,
        RaftStorageFaultMode::FailImmediately,
    );

    let error = runtime
        .join_member(
            ClusterCandidate::member("member-durable").generation(ClusterGeneration::new(1)),
        )
        .await
        .expect_err("durable commit fault must fail the metadata command loudly");
    assert!(error
        .to_string()
        .contains("injected storage fault during durable commit"));
    assert!(
        runtime.members().is_empty(),
        "failed durable commit must not materialize membership before recovery"
    );
    let failed = faults.observation();
    assert_eq!(failed.calls, 1);
    assert_eq!(failed.injected_failures, 1);
    assert_eq!(failed.in_flight, 0);

    let durable_state = store.initial_state()?;
    let retained = store.retained_entries()?;
    let command_entry = retained
        .iter()
        .find(|entry| {
            !entry.data.is_empty()
                && RaftMetadataCommandEnvelope::decode(entry.data.as_ref())
                    .is_ok_and(|envelope| envelope.command_id == "member-upsert:member-durable:1")
        })
        .ok_or("failed command was not retained for deterministic recovery")?;
    assert!(
        store
            .staged_sled_commit_index()?
            .is_some_and(|commit| commit >= command_entry.index),
        "fault must be injected after the real Sled commit write"
    );
    let durably_committed = store
        .staged_sled_commit_index()?
        .is_some_and(|commit| commit >= command_entry.index);
    assert!(
        durable_state.hard_state.commit < command_entry.index,
        "failed set_commit must not publish the staged Sled commit in memory"
    );

    drop(runtime);
    drop(store);
    let reopened = reopen_store(directory.path())?;
    let recovered =
        RaftMetadataRuntime::with_storage(config.clone().auto_campaign(false), reopened.clone())?;
    assert_eq!(
        recovered
            .members()
            .iter()
            .any(|member| member.node_id.as_str() == "member-durable"),
        durably_committed,
        "recovery must follow the persisted commit boundary exactly"
    );

    recovered.campaign()?;
    recovered
        .join_member(
            ClusterCandidate::member("member-durable").generation(ClusterGeneration::new(1)),
        )
        .await?;
    let durable_commands = recovered
        .command_envelopes()
        .into_iter()
        .filter(|envelope| envelope.command_id == "member-upsert:member-durable:1")
        .count();
    assert_eq!(durable_commands, 1, "recovery/retry must not double apply");
    let recovered_progress = recovered.snapshot();
    assert!(recovered_progress.applied_index <= recovered_progress.commit_index);

    drop(recovered);
    drop(reopened);
    drop(directory);
    Ok(())
}

#[test]
fn durable_commit_io_failure_stages_sled_before_publishing_store_state() -> TestResult {
    let directory = SledTestDirectory::new("durable-commit-publish")?;
    let store = open_store(directory.path(), vec![1])?;
    let faults = store.storage_faults();
    faults.arm(
        RaftStorageFaultOperation::DurableCommit,
        RaftStorageFaultMode::FailImmediately,
    );

    let error = RaftLogStore::set_commit(&store, 7)
        .expect_err("durable commit fault must be returned after the Sled write");

    assert!(error
        .to_string()
        .contains("injected storage fault during durable commit after sled batch"));
    assert_eq!(store.initial_state()?.hard_state.commit, 0);
    assert_eq!(store.staged_sled_commit_index()?, Some(7));

    RaftLogStore::set_commit(&store, 7)?;
    assert_eq!(store.initial_state()?.hard_state.commit, 7);
    drop(store);

    let reopened = reopen_store(directory.path())?;
    assert_eq!(reopened.initial_state()?.hard_state.commit, 7);
    drop(reopened);
    drop(directory);
    Ok(())
}

#[test]
fn canary_io_chaos_accepts_a_torn_commit() {
    let accepted_torn_commit = std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W5");
    assert!(
        !accepted_torn_commit,
        "HC-CANARY-RED:W5 IO chaos accepted a torn durable commit"
    );
}
