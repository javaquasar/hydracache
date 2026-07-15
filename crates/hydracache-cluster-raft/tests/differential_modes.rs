use std::collections::BTreeSet;

use hydracache_cluster_raft::RaftMetadataRuntime;
use hydracache_cluster_testkit::{
    invariants::{assert_cluster_invariants, ClusterInvariantView},
    RaftFilterAction, RaftPacketFilter, RuntimeRaftCluster,
};

#[tokio::test]
async fn same_op_stream_agrees_across_consistency_levels_where_contract_requires() {
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);

    for member in ["member-a", "member-b", "member-c"] {
        cluster.join_member(1, member).await.unwrap();
    }
    cluster.tick_all(10);

    let views = [
        committed_view(&cluster, ReadMode::Quorum),
        committed_view(&cluster, ReadMode::All),
        committed_view(&cluster, ReadMode::SnapshotRecovered),
    ];
    assert_views_agree(&views);
}

#[tokio::test]
async fn hazelcast_mined_split_brain_scenarios_never_lose_a_committed_write() {
    for scenario in SplitBrainScenario::hazelcast_mined() {
        let mut cluster = RuntimeRaftCluster::three_node();
        cluster.campaign(1);
        cluster.join_member(1, "baseline").await.unwrap();

        scenario.apply(&mut cluster);
        let member = format!("{}-committed", scenario.name);
        cluster.join_member(1, &member).await.unwrap();
        cluster.filters().recover();
        cluster.tick_all(30);

        assert_committed_write_visible(
            &cluster,
            &format!("member-upsert:{member}:1"),
            scenario.name,
        );
        assert_views_agree(&[
            committed_view(&cluster, ReadMode::Quorum),
            committed_view(&cluster, ReadMode::All),
        ]);
        assert_cluster_invariants(&ClusterInvariantView::from_runtime_raft_cluster(&cluster));
    }
}

#[test]
fn canary_differential_passes_when_two_modes_disagree() {
    let authoritative = CommittedView {
        mode: ReadMode::Quorum,
        members: BTreeSet::from(["member-a".to_owned()]),
        commands_committed: 1,
    };
    let stale = CommittedView {
        mode: ReadMode::All,
        members: BTreeSet::new(),
        commands_committed: 0,
    };

    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W28") {
        assert!(
            find_disagreement(&[authoritative.clone(), stale.clone()]).is_none(),
            "HC-CANARY-RED:W28 consistency modes disagreed"
        );
    }

    assert!(
        find_disagreement(&[authoritative, stale]).is_some(),
        "canary fixture must make a differential checker observe stale-mode disagreement"
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReadMode {
    Quorum,
    All,
    SnapshotRecovered,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CommittedView {
    mode: ReadMode,
    members: BTreeSet<String>,
    commands_committed: usize,
}

fn committed_view(cluster: &RuntimeRaftCluster, mode: ReadMode) -> CommittedView {
    let leader = cluster.leader_id().expect("cluster should have a leader");
    match mode {
        ReadMode::Quorum => {
            let mut quorum = cluster
                .node(leader)
                .voter_ids()
                .unwrap()
                .into_iter()
                .take(2)
                .collect::<Vec<_>>();
            quorum.sort_unstable();
            let reference = view_for_node(cluster, quorum[0], mode);
            for node_id in quorum.into_iter().skip(1) {
                assert_same_contract_view(&reference, &view_for_node(cluster, node_id, mode));
            }
            reference
        }
        ReadMode::All => {
            let voters = cluster.node(leader).voter_ids().unwrap();
            let reference = view_for_node(cluster, voters[0], mode);
            for node_id in voters.into_iter().skip(1) {
                assert_same_contract_view(&reference, &view_for_node(cluster, node_id, mode));
            }
            reference
        }
        ReadMode::SnapshotRecovered => {
            let recovered =
                RaftMetadataRuntime::from_snapshot(cluster.node(leader).export_snapshot()).unwrap();
            CommittedView {
                mode,
                members: member_ids(recovered.members()),
                commands_committed: recovered.snapshot().commands_committed,
            }
        }
    }
}

fn view_for_node(cluster: &RuntimeRaftCluster, node_id: u64, mode: ReadMode) -> CommittedView {
    let node = cluster.node(node_id);
    CommittedView {
        mode,
        members: member_ids(node.members()),
        commands_committed: node.snapshot().commands_committed,
    }
}

fn member_ids(members: Vec<hydracache::ClusterMember>) -> BTreeSet<String> {
    members
        .into_iter()
        .map(|member| member.node_id.as_str().to_owned())
        .collect()
}

fn assert_same_contract_view(left: &CommittedView, right: &CommittedView) {
    assert_eq!(
        comparable_view(left),
        comparable_view(right),
        "committed read modes disagree: left={left:?}, right={right:?}"
    );
}

fn assert_views_agree(views: &[CommittedView]) {
    if let Some((left, right)) = find_disagreement(views) {
        panic!("differential modes disagree: left={left:?}, right={right:?}");
    }
}

fn find_disagreement(views: &[CommittedView]) -> Option<(CommittedView, CommittedView)> {
    let first = views.first()?;
    views.iter().skip(1).find_map(|view| {
        (comparable_view(first) != comparable_view(view)).then(|| (first.clone(), view.clone()))
    })
}

fn comparable_view(view: &CommittedView) -> (BTreeSet<String>, usize) {
    (view.members.clone(), view.commands_committed)
}

#[derive(Debug, Clone, Copy)]
struct SplitBrainScenario {
    name: &'static str,
    apply: fn(&mut RuntimeRaftCluster),
}

impl SplitBrainScenario {
    fn hazelcast_mined() -> [Self; 3] {
        [
            Self {
                name: "minority-follower-isolated-then-merged",
                apply: |cluster| cluster.filters().isolate(3, [1, 2]),
            },
            Self {
                name: "delayed-majority-link-then-merged",
                apply: |cluster| {
                    cluster.filters().add_filter(
                        RaftPacketFilter::new()
                            .from(1)
                            .to(2)
                            .allow(1)
                            .action(RaftFilterAction::Delay(2)),
                    );
                },
            },
            Self {
                name: "duplicate-heal-messages-after-merge",
                apply: |cluster| {
                    cluster.filters().add_filter(
                        RaftPacketFilter::new()
                            .from(1)
                            .allow(2)
                            .action(RaftFilterAction::Duplicate(1)),
                    );
                },
            },
        ]
    }

    fn apply(&self, cluster: &mut RuntimeRaftCluster) {
        (self.apply)(cluster);
    }
}

fn assert_committed_write_visible(cluster: &RuntimeRaftCluster, command_id: &str, scenario: &str) {
    let leader = cluster.leader_id().expect("cluster should have a leader");
    for node_id in cluster.node(leader).voter_ids().unwrap() {
        assert!(
            cluster.node(node_id).command_applied(command_id),
            "scenario {scenario} lost committed command {command_id} on node {node_id}"
        );
    }
}
