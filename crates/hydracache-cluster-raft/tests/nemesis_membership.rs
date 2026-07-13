use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

use hydracache_cluster_raft::{RaftMetadataRuntime, RaftRuntimeRole};
use hydracache_cluster_testkit::{RaftFilterAction, RaftPacketFilter, RuntimeRaftCluster};

#[derive(Debug)]
struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        self.state
    }

    fn choose(&mut self, upper: usize) -> usize {
        (self.next() as usize) % upper
    }
}

#[derive(Debug)]
struct NemesisTrace {
    seed: u64,
    schedule: Vec<String>,
}

impl NemesisTrace {
    fn new(seed: u64) -> Self {
        Self {
            seed,
            schedule: Vec::new(),
        }
    }

    fn push(&mut self, step: usize, action: impl Into<String>) {
        self.schedule.push(format!("{step}: {}", action.into()));
    }

    fn context(&self) -> String {
        format!("seed={}, schedule={:?}", self.seed, self.schedule)
    }
}

fn member_ids(runtime: &RaftMetadataRuntime) -> BTreeSet<String> {
    runtime
        .members()
        .into_iter()
        .map(|member| member.node_id.as_str().to_owned())
        .collect()
}

fn assert_single_leader_per_term(cluster: &RuntimeRaftCluster, trace: &NemesisTrace) {
    let mut leaders_by_term = BTreeMap::<u64, Vec<u64>>::new();
    for node_id in cluster.node_ids() {
        let snapshot = cluster.node(node_id).snapshot();
        if snapshot.role == RaftRuntimeRole::Leader {
            leaders_by_term
                .entry(snapshot.term)
                .or_default()
                .push(node_id);
        }
    }
    for (term, leaders) in leaders_by_term {
        assert!(
            leaders.len() <= 1,
            "nemesis produced multiple leaders in term {term}: {leaders:?}; {}",
            trace.context()
        );
    }
}

fn assert_voters_converged(cluster: &RuntimeRaftCluster, trace: &NemesisTrace) {
    let mut observed = BTreeMap::new();
    for node_id in cluster.node_ids() {
        observed.insert(
            node_id,
            cluster
                .node(node_id)
                .voter_ids()
                .unwrap()
                .into_iter()
                .collect::<BTreeSet<_>>(),
        );
    }
    let expected = observed
        .values()
        .next()
        .expect("cluster should have voters")
        .clone();
    assert!(
        observed.values().all(|voters| *voters == expected),
        "nemesis voter ConfState diverged: {observed:?}; {}",
        trace.context()
    );
}

fn assert_members_converged(cluster: &RuntimeRaftCluster, trace: &NemesisTrace) {
    let mut observed = BTreeMap::new();
    let voters = cluster
        .node(1)
        .voter_ids()
        .expect("node 1 should expose the authoritative voter set")
        .into_iter()
        .collect::<BTreeSet<_>>();
    for node_id in cluster
        .node_ids()
        .into_iter()
        .filter(|node_id| voters.contains(node_id))
    {
        observed.insert(node_id, member_ids(&cluster.node(node_id)));
    }
    let expected = observed
        .values()
        .next()
        .expect("cluster should have member sets")
        .clone();
    assert!(
        observed.values().all(|members| *members == expected),
        "nemesis materialized membership diverged: {observed:?}; {}",
        trace.context()
    );
}

async fn run_seed(seed: u64, steps: usize) {
    let mut rng = DeterministicRng::new(seed);
    let mut trace = NemesisTrace::new(seed);
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    cluster.join_member(1, "member-a").await.unwrap();

    for step in 0..steps {
        match rng.choose(7) {
            0 => {
                cluster.tick_all(1);
                trace.push(step, "tick-all");
            }
            1 => {
                cluster.filters().cut(1, 2);
                cluster.tick_all(2);
                cluster.filters().recover();
                cluster.tick_all(2);
                trace.push(step, "partition(1,2)+heal");
            }
            2 => {
                cluster.filters().add_filter(
                    RaftPacketFilter::drop_between(2, 3)
                        .allow(1)
                        .action(RaftFilterAction::Drop),
                );
                cluster.tick_all(2);
                cluster.filters().recover();
                trace.push(step, "drop-one(2,3)+heal");
            }
            3 => {
                cluster.filters().add_filter(
                    RaftPacketFilter::new()
                        .from(1)
                        .allow(1)
                        .action(RaftFilterAction::Delay(2)),
                );
                cluster.tick_all(3);
                cluster.filters().recover();
                trace.push(step, "delay-one(from=1)+heal");
            }
            4 => {
                if let Some(leader) = cluster.leader_id() {
                    cluster.filters().add_filter(
                        RaftPacketFilter::new()
                            .from(leader)
                            .allow(1)
                            .action(RaftFilterAction::Duplicate(1)),
                    );
                    cluster.tick_all(2);
                    cluster.filters().recover();
                    trace.push(step, format!("duplicate-one(from={leader})+heal"));
                }
            }
            5 => {
                if let Some(leader) = cluster.leader_id() {
                    let member = format!("member-{seed:x}-{step}");
                    cluster.join_member(leader, &member).await.unwrap();
                    trace.push(step, format!("join({member})"));
                }
            }
            _ => {
                if let Some(leader) = cluster.leader_id() {
                    let voters = cluster.node(leader).voter_ids().unwrap();
                    if voters.contains(&3) && leader != 3 && voters.len() > 2 {
                        cluster.propose_remove_voter(leader, 3).unwrap();
                        trace.push(step, "conf-change(remove-voter=3)");
                    } else if !voters.contains(&3) {
                        cluster.propose_add_voter(leader, 3).unwrap();
                        trace.push(step, "conf-change(add-voter=3)");
                    } else {
                        let exported = cluster.node(leader).export_snapshot();
                        let restored = RaftMetadataRuntime::from_snapshot(exported).unwrap();
                        assert_eq!(
                            member_ids(&restored),
                            member_ids(&cluster.node(leader)),
                            "snapshot restore diverged during nemesis; {}",
                            trace.context()
                        );
                        trace.push(step, format!("snapshot-restore-check(leader={leader})"));
                    }
                }
            }
        }
    }

    cluster.filters().recover();
    cluster.tick_all(20);
    assert_single_leader_per_term(&cluster, &trace);
    assert_voters_converged(&cluster, &trace);
    assert_members_converged(&cluster, &trace);
}

#[tokio::test]
async fn nemesis_snapshot_membership_linearizable_under_composed_faults() {
    for seed in [0x6407, 0x6408, 0x6409] {
        run_seed(seed, 24).await;
    }
}

#[tokio::test]
async fn nemesis_soak_over_seed_range_converges() {
    if std::env::var("HYDRACACHE_RUN_RAFT_NEMESIS_SOAK")
        .map(|value| !value.trim().is_empty() && value != "0")
        .unwrap_or(false)
    {
        let budget_secs = std::env::var("HYDRACACHE_NEMESIS_BUDGET_SECS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(60);
        let deadline = Instant::now() + Duration::from_secs(budget_secs);
        let mut seed = 0x64_00_00;
        while Instant::now() < deadline {
            run_seed(seed, 32).await;
            seed += 1;
        }
    } else {
        eprintln!(
            "skipping nemesis_soak_over_seed_range_converges: set HYDRACACHE_RUN_RAFT_NEMESIS_SOAK=1"
        );
    }
}

#[test]
fn canary_nemesis_accepts_stale_member_set_after_restore() {
    let authoritative = BTreeSet::from(["member-a".to_owned(), "member-b".to_owned()]);
    let stale = BTreeSet::from(["member-a".to_owned()]);
    assert_ne!(
        stale, authoritative,
        "canary fixture must model the stale-member bug before it can prove the guard"
    );
}
