#![cfg(feature = "test-failpoints")]

use std::collections::BTreeSet;
use std::panic::{catch_unwind, AssertUnwindSafe};

use fail::FailScenario;
use hydracache::{
    CacheResult, ClusterCandidate, ClusterControlPlane, ClusterEpoch, ClusterGeneration,
    ClusterMember, ClusterNodeId, ClusterRole, RaftMetadataCommand,
};
use hydracache_cluster_raft::{
    InMemoryRaftLogStore, RaftLogStore, RaftMetadataCommandEnvelope, RaftMetadataRuntime,
    RaftMetadataRuntimeConfig, RaftMetadataRuntimeExport, RaftWireMessage,
};
use hydracache_cluster_testkit::RuntimeRaftCluster;
use raft::eraftpb::{Entry, Message, MessageType, Snapshot};

fn voter_set(cluster: &RuntimeRaftCluster, node_id: u64) -> BTreeSet<u64> {
    cluster
        .node(node_id)
        .voter_ids()
        .unwrap()
        .into_iter()
        .collect()
}

fn snapshot_wire_message(index: u64, term: u64, voters: Vec<u64>) -> RaftWireMessage {
    let mut snapshot = Snapshot::default();
    snapshot.mut_metadata().index = index;
    snapshot.mut_metadata().term = term;
    snapshot.mut_metadata().mut_conf_state().voters = voters;

    let mut message = Message {
        from: 2,
        to: 1,
        term,
        ..Message::default()
    };
    message.set_msg_type(MessageType::MsgSnapshot);
    message.set_snapshot(snapshot);
    RaftWireMessage::encode(&message).unwrap()
}

fn member(id: &'static str) -> ClusterCandidate {
    ClusterCandidate::member(id).generation(ClusterGeneration::new(1))
}

fn member_ids(mut members: Vec<ClusterMember>) -> Vec<String> {
    members.sort_by(|left, right| left.node_id.cmp(&right.node_id));
    members
        .into_iter()
        .map(|member| member.node_id.as_str().to_owned())
        .collect()
}

fn snapshot_with_alias_canary(
    captured: RaftMetadataRuntimeExport,
    live: &RaftMetadataRuntime,
) -> RaftMetadataRuntimeExport {
    fail::fail_point!("canary_raft_snapshot_aliases_live_state", |_| live
        .export_snapshot());
    captured
}

async fn replay_tail_with_canary(
    runtime: &RaftMetadataRuntime,
    tail: &[RaftMetadataCommandEnvelope],
) {
    fail::fail_point!("canary_raft_snapshot_skips_tail_apply", |_| ());
    for envelope in tail {
        match &envelope.command {
            RaftMetadataCommand::MemberUpsert {
                node_id,
                generation,
                ..
            } => {
                runtime
                    .join_member(ClusterCandidate::member(node_id.clone()).generation(*generation))
                    .await
                    .unwrap();
            }
            RaftMetadataCommand::ClientUpsert {
                node_id,
                generation,
                ..
            } => {
                runtime
                    .join_client(ClusterCandidate::client(node_id.clone()).generation(*generation))
                    .await
                    .unwrap();
            }
            RaftMetadataCommand::NodeLeft { node_id, .. } => {
                let generation = runtime
                    .members()
                    .into_iter()
                    .chain(runtime.clients())
                    .find(|member| member.node_id == *node_id)
                    .map(|member| member.generation)
                    .expect("tail leave target must exist before replay");
                runtime.leave(node_id, generation).await.unwrap();
            }
            RaftMetadataCommand::CommitTopology { .. } => {}
        }
    }
}

fn restore_with_downgrade_canary(
    snapshot: RaftMetadataRuntimeExport,
) -> CacheResult<RaftMetadataRuntime> {
    match RaftMetadataRuntime::from_snapshot(snapshot) {
        Ok(runtime) => Ok(runtime),
        Err(error) => {
            fail::fail_point!("canary_raft_snapshot_downgrades_apply_error", |_| {
                RaftMetadataRuntime::single_node("orders", 1)
            });
            Err(error)
        }
    }
}

#[test]
fn crash_between_confchange_commit_and_save_conf_state_recovers_consistent_voters() {
    let _scenario = FailScenario::setup();
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);

    let outbound = cluster.node(1).propose_add_voter(4).unwrap();
    fail::cfg("raft_before_save_conf_state", "return").unwrap();
    let failure = catch_unwind(AssertUnwindSafe(|| {
        cluster.drain_until_idle(outbound.clone());
    }));
    assert!(failure.is_err(), "conf-state failpoint should fail loudly");
    fail::remove("raft_before_save_conf_state");

    let mut recovered = RuntimeRaftCluster::three_node();
    recovered.campaign(1);
    recovered.propose_add_voter(1, 4).unwrap();

    for node_id in [1, 2, 3] {
        assert_eq!(voter_set(&recovered, node_id), BTreeSet::from([1, 2, 3, 4]));
    }
}

#[test]
fn crash_after_snapshot_persist_before_apply_replays_or_installs_once() {
    for failpoint in [
        "raft_after_save_snapshot_before_entries",
        "raft_after_install_snapshot_before_apply",
    ] {
        let _scenario = FailScenario::setup();
        let runtime = RaftMetadataRuntime::with_config(
            RaftMetadataRuntimeConfig::multi_voter("orders", 1, [1, 2]).ticks(5, 1),
        )
        .unwrap();
        let wire = snapshot_wire_message(7, 2, vec![1]);

        fail::cfg(failpoint, "return").unwrap();
        let failure = runtime.step(wire).unwrap_err();
        assert!(
            failure.to_string().contains("snapshot"),
            "{failpoint} should fail loudly at the snapshot boundary: {failure}"
        );
        fail::remove(failpoint);

        runtime
            .drain_ready()
            .expect("snapshot ready should replay after clearing failpoint");
        let snapshot = runtime.snapshot();
        assert_eq!(
            snapshot.commands_committed, 0,
            "snapshot recovery should not double-apply metadata commands"
        );
        assert!(
            snapshot.commit_index >= 7 || snapshot.applied_index >= 7,
            "snapshot recovery should install or replay the persisted boundary once: {snapshot:?}"
        );
    }
}

#[test]
fn crash_after_hard_state_before_send_does_not_lose_committed_entry() {
    let _scenario = FailScenario::setup();
    let mut cluster = RuntimeRaftCluster::three_node();
    let outbound = cluster.node(1).campaign().unwrap();

    fail::cfg("raft_after_save_hard_state_before_send", "return").unwrap();
    let failure = catch_unwind(AssertUnwindSafe(|| {
        cluster.drain_until_idle(outbound.clone());
    }));
    assert!(failure.is_err(), "hard-state failpoint should fail loudly");
    fail::remove("raft_after_save_hard_state_before_send");

    let mut recovered = RuntimeRaftCluster::three_node();
    recovered.campaign(1);

    assert!(
        recovered.leader_id().is_some(),
        "clearing the failpoint should let election continue"
    );
}

#[test]
fn disk_full_on_append_fails_loud_not_silent() {
    let _scenario = FailScenario::setup();
    let store = InMemoryRaftLogStore::new();
    let entry = Entry {
        index: 1,
        term: 1,
        data: b"member-a".to_vec().into(),
        ..Entry::default()
    };

    fail::cfg("sled_append_disk_full", "return").unwrap();
    let error = store.append(&[entry]).unwrap_err();

    assert!(
        error.to_string().contains("disk full"),
        "disk-full failpoint should surface loudly: {error}"
    );
}

#[test]
fn falsifiability_canaries_turn_their_guard_tests_red() {
    let _scenario = FailScenario::setup();
    fail::cfg("canary_raft_disable_prevote", "return").unwrap();
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    let leader_term = cluster.node(1).snapshot().term;
    cluster.filters().cut(1, 2);

    for _ in 0..20 {
        cluster.tick_node(2);
    }

    assert!(
        cluster.node(2).snapshot().term > leader_term,
        "disabling pre-vote should make the isolated node inflate its term"
    );
    fail::remove("canary_raft_disable_prevote");

    fail::cfg("canary_raft_skip_save_conf_state", "return").unwrap();
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    cluster.propose_remove_voter(1, 3).unwrap();

    assert!(
        voter_set(&cluster, 1).contains(&3),
        "skipping conf-state persistence should keep a removed voter visible to the drain guard"
    );
}

#[tokio::test]
async fn snapshot_falsifiability_canaries_turn_their_guard_tests_red() {
    let _scenario = FailScenario::setup();

    let runtime = RaftMetadataRuntime::single_node("orders", 1).unwrap();
    runtime.join_member(member("member-a")).await.unwrap();
    let captured = runtime.export_snapshot();
    runtime.join_member(member("member-b")).await.unwrap();

    fail::cfg("canary_raft_snapshot_aliases_live_state", "return").unwrap();
    let aliased = snapshot_with_alias_canary(captured, &runtime);
    fail::remove("canary_raft_snapshot_aliases_live_state");

    let recovered = RaftMetadataRuntime::from_snapshot(aliased).unwrap();
    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W4") {
        assert!(
            !member_ids(recovered.members()).contains(&"member-b".to_owned()),
            "HC-CANARY-RED:W4 exported snapshot aliased live membership"
        );
    }
    assert!(
        member_ids(recovered.members()).contains(&"member-b".to_owned()),
        "alias canary should make a point-in-time snapshot guard observe later live state"
    );

    let authoritative = RaftMetadataRuntime::single_node("orders", 1).unwrap();
    authoritative.join_member(member("member-a")).await.unwrap();
    let snapshot = authoritative.export_snapshot();
    authoritative.join_member(member("member-b")).await.unwrap();
    let full = authoritative.export_snapshot();
    let tail = &full.commands[snapshot.commands.len()..];
    let restored = RaftMetadataRuntime::from_snapshot(snapshot).unwrap();

    fail::cfg("canary_raft_snapshot_skips_tail_apply", "return").unwrap();
    replay_tail_with_canary(&restored, tail).await;
    fail::remove("canary_raft_snapshot_skips_tail_apply");

    assert!(
        !member_ids(restored.members()).contains(&"member-b".to_owned()),
        "skip-tail canary should make the restored membership miss committed tail commands"
    );

    let mut malformed = runtime.export_snapshot();
    malformed.commands.push(RaftMetadataCommandEnvelope {
        command_id: "node-left:missing-member:2".to_owned(),
        command: RaftMetadataCommand::NodeLeft {
            node_id: ClusterNodeId::from("missing-member"),
            role: ClusterRole::Member,
            epoch: ClusterEpoch::new(2),
        },
    });

    fail::cfg("canary_raft_snapshot_downgrades_apply_error", "return").unwrap();
    let downgraded = restore_with_downgrade_canary(malformed);
    fail::remove("canary_raft_snapshot_downgrades_apply_error");

    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W3") {
        assert!(
            downgraded.is_err(),
            "HC-CANARY-RED:W3 snapshot apply contradiction was downgraded"
        );
    }
    assert!(
        downgraded.is_ok(),
        "downgrade canary should suppress the fail-loud snapshot apply error"
    );
}
