use hydracache::{ClusterCandidate, ClusterControlPlane, ClusterGeneration};
use hydracache_cluster_raft::{
    InMemoryRaftLogStore, RaftLogStore, RaftMetadataRuntime, RaftMetadataRuntimeConfig,
};

#[test]
fn compaction_seam_rejects_an_index_past_applied_progress() {
    let store = InMemoryRaftLogStore::new();
    RaftLogStore::mark_applied(&store, 3).unwrap();

    let error = store.compact_to(4).unwrap_err();

    assert!(
        error.to_string().contains("past applied index 3"),
        "past-applied compaction must fail loud: {error}"
    );
}

#[test]
fn compaction_seam_rejects_before_any_entry_is_applied() {
    let config = RaftMetadataRuntimeConfig::try_joining("compaction-empty", 2, [1]).unwrap();
    let store = InMemoryRaftLogStore::new_with_conf_state((vec![1, 2], vec![]));
    let runtime = RaftMetadataRuntime::with_storage(config, store).unwrap();

    let error = runtime.compact_applied_log_to_snapshot().unwrap_err();

    assert!(
        error.to_string().contains("before any entry is applied"),
        "empty-log compaction must fail loud: {error}"
    );
    let observation = runtime.log_compaction_observation().unwrap();
    assert_eq!(observation.applied_index, 0);
    assert_eq!(observation.snapshot_index, 0);
}

#[tokio::test]
async fn compaction_seam_snapshots_exactly_current_applied_progress() {
    let runtime = RaftMetadataRuntime::single_node("compaction-boundary", 1).unwrap();
    runtime
        .join_member(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)))
        .await
        .unwrap();

    let before = runtime.log_compaction_observation().unwrap();
    assert!(before.applied_index > 0);
    assert_eq!(before.snapshot_index, 0);

    let compacted_index = runtime.compact_applied_log_to_snapshot().unwrap();
    let after = runtime.log_compaction_observation().unwrap();

    assert_eq!(compacted_index, before.applied_index);
    assert_eq!(after.applied_index, before.applied_index);
    assert_eq!(after.snapshot_index, after.applied_index);
    assert_eq!(after.first_log_index, after.snapshot_index + 1);
    assert!(after.snapshot_index <= after.applied_index);
    assert!(runtime
        .members()
        .iter()
        .any(|member| member.node_id.as_str() == "member-a"));
}

#[tokio::test]
async fn oversized_candidate_is_rejected_before_snapshot_or_log_prefix_changes() {
    let config =
        RaftMetadataRuntimeConfig::single_node("compaction-size-guard", 1).max_size_per_msg(1_024);
    let store = InMemoryRaftLogStore::new_with_conf_state((vec![1], vec![]));
    let runtime = RaftMetadataRuntime::with_storage(config, store).unwrap();
    runtime
        .join_member(
            ClusterCandidate::member("snapshot-prefix").generation(ClusterGeneration::new(1)),
        )
        .await
        .unwrap();
    let initial_size = runtime.snapshot_size_observation().unwrap();
    assert!(initial_size.transportable, "{initial_size:?}");
    let previous_snapshot_index = runtime.compact_applied_log_to_snapshot().unwrap();

    let mut oversized = None;
    for index in 0..64 {
        runtime
            .join_member(
                ClusterCandidate::member(format!("retained-tail-member-{index:02}"))
                    .generation(ClusterGeneration::new(1)),
            )
            .await
            .unwrap();
        let observation = runtime.snapshot_size_observation().unwrap();
        if !observation.transportable {
            oversized = Some(observation);
            break;
        }
    }
    let oversized = oversized.expect("test history must exceed the reduced wire budget");
    assert!(oversized.encoded_wire_bytes > oversized.max_wire_bytes);
    let before = runtime.log_compaction_observation().unwrap();
    assert_eq!(before.snapshot_index, previous_snapshot_index);
    assert!(before.last_log_index >= before.first_log_index);

    let error = runtime.compact_applied_log_to_snapshot().unwrap_err();

    assert!(error.to_string().contains("snapshot compaction rejected"));
    assert!(error
        .to_string()
        .contains("previous snapshot and retained log are unchanged"));
    assert_eq!(runtime.log_compaction_observation().unwrap(), before);
}

#[cfg(feature = "sled-log-store")]
#[tokio::test]
async fn compaction_seam_sled_restart_restores_snapshot_before_retained_tail() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "hydracache-compaction-seam-{}-{unique}",
        std::process::id()
    ));
    let config = RaftMetadataRuntimeConfig::single_node("compaction-sled", 1);
    let runtime = RaftMetadataRuntime::sled_with_config(config.clone(), &path).unwrap();
    runtime
        .join_member(
            ClusterCandidate::member("member-before-restart").generation(ClusterGeneration::new(1)),
        )
        .await
        .unwrap();
    let compacted_index = runtime.compact_applied_log_to_snapshot().unwrap();
    drop(runtime);

    let reopened = RaftMetadataRuntime::sled_with_config(config, &path).unwrap();
    let observation = reopened.log_compaction_observation().unwrap();
    assert_eq!(observation.snapshot_index, compacted_index);
    assert!(observation.applied_index >= compacted_index);
    assert!(reopened
        .members()
        .iter()
        .any(|member| member.node_id.as_str() == "member-before-restart"));

    drop(reopened);
    let _ = std::fs::remove_dir_all(path);
}

#[cfg(feature = "sled-log-store")]
#[tokio::test]
async fn sled_restart_replays_last_snapshot_plus_tail_after_oversized_rejection() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "hydracache-compaction-size-guard-{}-{unique}",
        std::process::id()
    ));
    let config = RaftMetadataRuntimeConfig::single_node("compaction-size-recovery", 1)
        .max_size_per_msg(1_024);
    let runtime = RaftMetadataRuntime::sled_with_config(config.clone(), &path).unwrap();
    runtime
        .join_member(
            ClusterCandidate::member("snapshot-prefix").generation(ClusterGeneration::new(1)),
        )
        .await
        .unwrap();
    let previous_snapshot_index = runtime.compact_applied_log_to_snapshot().unwrap();

    let mut tail_members = Vec::new();
    for index in 0..64 {
        let member = format!("retained-tail-member-{index:02}");
        runtime
            .join_member(
                ClusterCandidate::member(member.clone()).generation(ClusterGeneration::new(1)),
            )
            .await
            .unwrap();
        tail_members.push(member);
        if !runtime.snapshot_size_observation().unwrap().transportable {
            break;
        }
    }
    assert!(!runtime.snapshot_size_observation().unwrap().transportable);
    let before = runtime.log_compaction_observation().unwrap();
    let error = runtime.compact_applied_log_to_snapshot().unwrap_err();
    assert!(error.to_string().contains("snapshot compaction rejected"));
    assert_eq!(runtime.log_compaction_observation().unwrap(), before);
    drop(runtime);

    let reopened = RaftMetadataRuntime::sled_with_config(config, &path).unwrap();
    let recovered = reopened
        .members()
        .into_iter()
        .map(|member| member.node_id.to_string())
        .collect::<std::collections::BTreeSet<_>>();
    assert!(recovered.contains("snapshot-prefix"));
    for member in &tail_members {
        assert!(
            recovered.contains(member),
            "restart lost retained tail member {member}"
        );
    }
    let after_restart = reopened.log_compaction_observation().unwrap();
    assert_eq!(after_restart.snapshot_index, previous_snapshot_index);
    assert!(after_restart.applied_index > previous_snapshot_index);

    drop(reopened);
    let _ = std::fs::remove_dir_all(path);
}

#[cfg(feature = "sled-log-store")]
#[tokio::test]
async fn compaction_seam_sled_restart_applies_newer_retained_tail_after_snapshot_prefix() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "hydracache-compaction-tail-recovery-{}-{unique}",
        std::process::id()
    ));
    let config = RaftMetadataRuntimeConfig::single_node("compaction-tail-recovery", 1);
    let runtime = RaftMetadataRuntime::sled_with_config(config.clone(), &path).unwrap();
    runtime
        .join_member(
            ClusterCandidate::member("member-with-tail").generation(ClusterGeneration::new(1)),
        )
        .await
        .unwrap();
    let snapshot_index = runtime.compact_applied_log_to_snapshot().unwrap();
    let upgraded = runtime
        .join_member(
            ClusterCandidate::member("member-with-tail").generation(ClusterGeneration::new(2)),
        )
        .await
        .unwrap();
    assert_eq!(upgraded.generation, ClusterGeneration::new(2));
    assert!(runtime.snapshot().applied_index > snapshot_index);
    drop(runtime);

    let reopened = RaftMetadataRuntime::sled_with_config(config, &path).unwrap();
    let recovered = reopened
        .members()
        .into_iter()
        .find(|member| member.node_id.as_str() == "member-with-tail")
        .expect("snapshot prefix member and retained tail should recover");
    assert_eq!(recovered.generation, ClusterGeneration::new(2));
    assert_eq!(reopened.commands().len(), 2);
    assert!(reopened.snapshot().applied_index > snapshot_index);

    drop(reopened);
    let _ = std::fs::remove_dir_all(path);
}

#[cfg(feature = "sled-log-store")]
#[tokio::test]
async fn compaction_seam_recovery_applies_committed_confchange_past_persisted_applied() {
    use hydracache_cluster_raft::SledRaftLogStore;
    use protobuf::Message as ProtobufMessage;
    use raft::eraftpb::{ConfChange, ConfChangeType, Entry, EntryType};
    use raft::Storage;

    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "hydracache-compaction-confchange-recovery-{}-{unique}",
        std::process::id()
    ));
    let config = RaftMetadataRuntimeConfig::single_node("compaction-confchange-recovery", 1);
    let runtime = RaftMetadataRuntime::sled_with_config(config.clone(), &path).unwrap();
    runtime
        .join_member(
            ClusterCandidate::member("snapshot-prefix").generation(ClusterGeneration::new(1)),
        )
        .await
        .unwrap();
    let snapshot_index = runtime.compact_applied_log_to_snapshot().unwrap();
    drop(runtime);

    let store = retry_sled_reopen(|| SledRaftLogStore::open(&path)).unwrap();
    assert_eq!(RaftLogStore::applied_index(&store).unwrap(), snapshot_index);
    assert_eq!(store.initial_state().unwrap().conf_state.voters, vec![1]);
    let tail_index = snapshot_index + 1;
    let mut change = ConfChange {
        node_id: 2,
        ..ConfChange::default()
    };
    change.set_change_type(ConfChangeType::AddNode);
    let mut entry = Entry {
        index: tail_index,
        term: store.term(snapshot_index).unwrap(),
        data: change.write_to_bytes().unwrap().into(),
        ..Entry::default()
    };
    entry.set_entry_type(EntryType::EntryConfChange);
    store.append(&[entry]).unwrap();
    let mut hard_state = store.initial_state().unwrap().hard_state;
    hard_state.commit = tail_index;
    store.save_hard_state(&hard_state).unwrap();
    // Model a crash after commit persistence but before ConfChange apply and
    // before the updated ConfState/applied boundary are persisted.
    assert_eq!(RaftLogStore::applied_index(&store).unwrap(), snapshot_index);
    assert_eq!(store.initial_state().unwrap().conf_state.voters, vec![1]);
    drop(store);

    let recovered =
        retry_sled_reopen(|| RaftMetadataRuntime::sled_with_config(config.clone(), &path)).unwrap();
    assert_eq!(recovered.voter_ids().unwrap(), vec![1, 2]);
    assert!(recovered.snapshot().applied_index >= tail_index);
    drop(recovered);

    let persisted = retry_sled_reopen(|| SledRaftLogStore::open(&path)).unwrap();
    assert_eq!(
        persisted.initial_state().unwrap().conf_state.voters,
        vec![1, 2]
    );
    assert!(RaftLogStore::applied_index(&persisted).unwrap() >= tail_index);
    drop(persisted);
    let _ = std::fs::remove_dir_all(path);
}

#[cfg(feature = "sled-log-store")]
fn retry_sled_reopen<T, E>(mut open: impl FnMut() -> Result<T, E>) -> Result<T, E>
where
    E: std::fmt::Display,
{
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    loop {
        match open() {
            Ok(value) => return Ok(value),
            Err(error)
                if error.to_string().contains("could not acquire lock")
                    && std::time::Instant::now() < deadline =>
            {
                // Sled releases its filesystem lock after the last Db handle is
                // dropped, but the background flusher may finish asynchronously.
                // A crash/reopen proof must wait for that close boundary rather
                // than racing it or accepting any other open error.
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(error) => return Err(error),
        }
    }
}

#[test]
fn canary_compaction_seam_leaks_into_default_release_path() {
    let control_leaked_into_default =
        std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W0");
    assert!(
        !control_leaked_into_default,
        "HC-CANARY-RED:W0 raft compaction control leaked into the default release path"
    );
}
