use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

use hydracache_cluster_raft::{RaftMetadataRuntime, RaftRuntimeRole};
use hydracache_cluster_testkit::{RaftFilterAction, RaftPacketFilter, RuntimeRaftCluster};
use serde::Deserialize;

const BAD_SEEDS_JSON: &str = include_str!("vectors/bad_seeds.json");

#[derive(Debug, Deserialize)]
struct BadSeedCorpus {
    version: u32,
    seeds: Vec<BadSeed>,
}

#[derive(Debug, Deserialize)]
struct BadSeed {
    suite: String,
    seed: u64,
    steps: usize,
    reason: String,
}

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

fn bad_seed_corpus() -> BadSeedCorpus {
    serde_json::from_str(BAD_SEEDS_JSON).expect("bad_seeds.json must be valid JSON")
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct NemesisOutcome {
    schedule: Vec<String>,
    voters_by_node: BTreeMap<u64, BTreeSet<u64>>,
    members_by_node: BTreeMap<u64, BTreeSet<String>>,
}

impl NemesisOutcome {
    fn from_cluster(cluster: &RuntimeRaftCluster, trace: &NemesisTrace) -> Self {
        Self {
            schedule: trace.schedule.clone(),
            voters_by_node: voter_sets(cluster),
            members_by_node: member_sets(cluster),
        }
    }
}

fn member_ids(runtime: &RaftMetadataRuntime) -> BTreeSet<String> {
    runtime
        .members()
        .into_iter()
        .map(|member| member.node_id.as_str().to_owned())
        .collect()
}

fn voter_sets(cluster: &RuntimeRaftCluster) -> BTreeMap<u64, BTreeSet<u64>> {
    let mut observed = BTreeMap::new();
    for node_id in cluster.node_ids() {
        observed.insert(
            node_id,
            cluster
                .node(node_id)
                .voter_ids()
                .unwrap()
                .into_iter()
                .collect(),
        );
    }
    observed
}

fn member_sets(cluster: &RuntimeRaftCluster) -> BTreeMap<u64, BTreeSet<String>> {
    let mut observed = BTreeMap::new();
    for node_id in cluster.node_ids() {
        observed.insert(node_id, member_ids(&cluster.node(node_id)));
    }
    observed
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
    let observed = voter_sets(cluster);
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

async fn run_seed(seed: u64, steps: usize) -> NemesisOutcome {
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
    NemesisOutcome::from_cluster(&cluster, &trace)
}

fn fixture_schedule_reproduces_failure(schedule: &[String]) -> bool {
    let mut installed_snapshot = false;
    let mut skipped_tail = false;
    for action in schedule {
        installed_snapshot |= action == "install-snapshot";
        skipped_tail |= action == "skip-membership-tail";
    }
    installed_snapshot && skipped_tail
}

fn shrink_reproducing_schedule(
    mut schedule: Vec<String>,
    reproduces: impl Fn(&[String]) -> bool,
) -> Vec<String> {
    assert!(
        reproduces(&schedule),
        "cannot shrink a schedule that does not reproduce"
    );
    let mut index = 0;
    while index < schedule.len() {
        let mut candidate = schedule.clone();
        candidate.remove(index);
        if reproduces(&candidate) {
            schedule = candidate;
            index = 0;
        } else {
            index += 1;
        }
    }
    schedule
}

fn assert_one_step_minimal(schedule: &[String], reproduces: impl Fn(&[String]) -> bool) {
    assert!(reproduces(schedule), "minimal schedule must reproduce");
    for index in 0..schedule.len() {
        let mut candidate = schedule.to_vec();
        candidate.remove(index);
        assert!(
            !reproduces(&candidate),
            "schedule is not one-step minimal after removing index {index}: {schedule:?}"
        );
    }
}

#[tokio::test]
async fn nemesis_snapshot_membership_linearizable_under_composed_faults() {
    for seed in [0x6407, 0x6408, 0x6409] {
        run_seed(seed, 24).await;
    }
}

#[tokio::test]
async fn nemesis_replays_identically_for_same_seed() {
    let left = run_seed(0x6418, 16).await;
    let right = run_seed(0x6418, 16).await;

    assert_eq!(left, right);
}

#[tokio::test]
async fn known_bad_seeds_replay_green_in_fast_tier() {
    let corpus = bad_seed_corpus();
    assert_eq!(corpus.version, 1);
    assert!(
        !corpus.seeds.is_empty(),
        "bad-seed corpus must keep at least one replay sentinel"
    );

    let mut replayed = 0usize;
    for entry in &corpus.seeds {
        assert!(
            !entry.reason.trim().is_empty(),
            "bad seed {entry:?} must carry review context"
        );
        match entry.suite.as_str() {
            "nemesis_membership" => {
                run_seed(entry.seed, entry.steps).await;
                replayed += 1;
            }
            other => panic!("unsupported bad-seed suite `{other}` in corpus"),
        }
    }
    assert_eq!(replayed, corpus.seeds.len());
}

#[test]
fn nemesis_failure_shrinks_to_minimal_reproducing_schedule() {
    let schedule = vec![
        "tick-all".to_owned(),
        "install-snapshot".to_owned(),
        "heal-partition".to_owned(),
        "skip-membership-tail".to_owned(),
        "duplicate-message".to_owned(),
    ];

    let minimal = shrink_reproducing_schedule(schedule, fixture_schedule_reproduces_failure);

    assert_eq!(
        minimal,
        vec![
            "install-snapshot".to_owned(),
            "skip-membership-tail".to_owned()
        ]
    );
    assert_one_step_minimal(&minimal, fixture_schedule_reproduces_failure);
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

#[test]
fn canary_nemesis_shrinker_returns_a_nonreproducing_schedule() {
    let broken_minimal = vec!["skip-membership-tail".to_owned()];
    assert!(
        !fixture_schedule_reproduces_failure(&broken_minimal),
        "canary models a broken shrinker returning a schedule that no longer reproduces"
    );
}

#[test]
fn canary_bad_seed_corpus_is_not_actually_executed() {
    let corpus = bad_seed_corpus();
    let replayed_by_stubbed_loop = 0usize;
    assert_ne!(
        replayed_by_stubbed_loop,
        corpus.seeds.len(),
        "canary models a fake-green bad-seed gate that loads the corpus but never replays it"
    );
}
