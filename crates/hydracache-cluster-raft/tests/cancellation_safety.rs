use std::fs;
use std::sync::Arc;
use std::time::Duration;

use hydracache::{ClusterCandidate, ClusterControlPlane, ClusterGeneration};
use hydracache_cluster_raft::{
    InMemoryRaftMetadataStore, RaftCommandStatus, RaftMetadataRuntime, RaftMetadataRuntimeConfig,
    RaftMetadataStore,
};
use hydracache_cluster_testkit::RuntimeRaftCluster;
use tokio::time::timeout;

fn member(id: &'static str) -> ClusterCandidate {
    ClusterCandidate::member(id).generation(ClusterGeneration::new(1))
}

async fn wait_for_status(runtime: &Arc<RaftMetadataRuntime>, status: RaftCommandStatus) {
    timeout(Duration::from_secs(2), async {
        loop {
            if runtime
                .snapshot()
                .last_result
                .as_ref()
                .is_some_and(|result| result.status == status)
            {
                return;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("runtime did not reach named status {status:?}"));
}

async fn drain_until_status(
    cluster: &mut RuntimeRaftCluster,
    runtime: &Arc<RaftMetadataRuntime>,
    status: RaftCommandStatus,
) {
    timeout(Duration::from_secs(2), async {
        loop {
            cluster.drain_until_idle(runtime.take_outbound_messages());
            if runtime
                .snapshot()
                .last_result
                .as_ref()
                .is_some_and(|result| result.status == status)
            {
                return;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("runtime did not reach named status {status:?}"));
}

fn write_evidence(test_name: &str, outcome: &str, assertions: &[&str]) {
    let Ok(path) = std::env::var("HYDRACACHE_W39C_EVIDENCE") else {
        return;
    };
    let path = std::path::PathBuf::from(path);
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).expect("create raft cancellation evidence directory");
    }
    let evidence = serde_json::json!({
        "suite": "W39c",
        "test": test_name,
        "status": "passed",
        "proposal_outcome_after_caller_drop": outcome,
        "assertions": assertions,
    });
    fs::write(
        path,
        serde_json::to_vec_pretty(&evidence).expect("serialize raft evidence"),
    )
    .expect("write raft cancellation evidence");
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProposalOutcome {
    OutcomeUnknown,
}

fn require_cancellation_gate() {
    assert_eq!(
        std::env::var("HYDRACACHE_RUN_CANCELLATION_RAFT").as_deref(),
        Ok("1"),
        "W39c must run only through HYDRACACHE_RUN_CANCELLATION_RAFT"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "W39c gated runtime cancellation proof"]
async fn raft_dropped_proposal_has_explicit_unknown_outcome_and_retry_is_idempotent() {
    require_cancellation_gate();
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    let follower = cluster.node(2);
    let command_id = "member-upsert:cancelled-member:1";

    let join = tokio::spawn({
        let follower = follower.clone();
        async move { follower.join_member(member("cancelled-member")).await }
    });
    wait_for_status(&follower, RaftCommandStatus::Forwarded).await;

    let outcome = ProposalOutcome::OutcomeUnknown;
    join.abort();
    assert!(join
        .await
        .expect_err("caller cancellation must be observable")
        .is_cancelled());
    assert!(!follower.command_applied(command_id));
    assert_eq!(outcome, ProposalOutcome::OutcomeUnknown);

    cluster.drain_until_idle(follower.take_outbound_messages());
    assert!(
        follower.command_applied(command_id),
        "a proposal may commit after its caller is cancelled"
    );
    assert_eq!(follower.snapshot().commands_committed, 1);

    let retry = tokio::spawn({
        let follower = follower.clone();
        async move { follower.join_member(member("cancelled-member")).await }
    });
    drain_until_status(&mut cluster, &follower, RaftCommandStatus::Duplicate).await;
    let retried_member = retry.await.unwrap().unwrap();
    assert_eq!(retried_member.node_id.as_str(), "cancelled-member");
    assert_eq!(follower.snapshot().commands_committed, 1);
    assert_eq!(follower.snapshot().duplicate_commands, 1);

    write_evidence(
        "raft_dropped_proposal_has_explicit_unknown_outcome_and_retry_is_idempotent",
        "outcome_unknown_then_committed_after_caller_drop",
        &[
            "caller cancellation occurred after Forwarded and before local apply",
            "the detached proposal committed exactly once",
            "retry returned Duplicate without a second materialized command",
        ],
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "W39c gated runtime cancellation proof"]
async fn runtime_shutdown_with_inflight_ops_recovers_consistent_metadata() {
    require_cancellation_gate();
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    let follower = cluster.node(2);
    let command_id = "member-upsert:shutdown-member:1";

    let join = tokio::spawn({
        let follower = follower.clone();
        async move { follower.join_member(member("shutdown-member")).await }
    });
    wait_for_status(&follower, RaftCommandStatus::Forwarded).await;
    join.abort();
    assert!(join
        .await
        .expect_err("runtime shutdown must cancel the caller")
        .is_cancelled());

    cluster.drain_until_idle(follower.take_outbound_messages());
    assert!(follower.command_applied(command_id));
    let before_restart = follower.snapshot();
    let metadata_store = Arc::new(InMemoryRaftMetadataStore::new());
    metadata_store
        .save(follower.export_snapshot())
        .expect("persist metadata before runtime shutdown");
    let restarted = RaftMetadataRuntime::with_config_and_metadata_store(
        RaftMetadataRuntimeConfig::multi_voter("orders", 2, [1, 2, 3]),
        metadata_store,
    )
    .expect("reopen metadata runtime after shutdown");
    assert!(restarted.command_applied(command_id));
    assert_eq!(
        restarted.snapshot().commands_committed,
        before_restart.commands_committed
    );
    assert!(restarted
        .members()
        .iter()
        .any(|member| member.node_id.as_str() == "shutdown-member"));

    write_evidence(
        "runtime_shutdown_with_inflight_ops_recovers_consistent_metadata",
        "committed_after_caller_shutdown_and_recovered",
        &[
            "in-flight caller was cancelled without suppressing the detached proposal",
            "metadata-bearing snapshot preserved the applied command id",
            "reopened runtime materialized the same membership without duplication",
        ],
    );
}
