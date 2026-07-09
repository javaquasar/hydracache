use std::collections::BTreeMap;

use hydracache_cluster_testkit::{RaftPacketFilter, RuntimeRaftCluster};

const PREVOTE_NIGHTLY_SOAK_ENV: &str = "HYDRACACHE_RUN_PREVOTE_NIGHTLY_SOAK";

#[test]
fn prevote_nightly_mixed_restart_topology_soak() {
    if !prevote_nightly_soak_enabled() {
        eprintln!(
            "skipping prevote_nightly_mixed_restart_topology_soak: set {PREVOTE_NIGHTLY_SOAK_ENV}=1 to run"
        );
        return;
    }

    let seeds = env_u64("HYDRACACHE_PREVOTE_SOAK_SEEDS").unwrap_or(16);
    let steps = env_u64("HYDRACACHE_PREVOTE_SOAK_STEPS").unwrap_or(64);
    for seed in 0..seeds {
        run_prevote_seed(0x6200_0000_u64 ^ seed, steps);
    }
}

fn run_prevote_seed(seed: u64, steps: u64) {
    let mut rng = XorShift64::new(seed);
    let mut cluster =
        RuntimeRaftCluster::with_prevote_overrides([1, 2, 3], BTreeMap::from([(2, false)]));
    cluster.campaign(1);
    let mut expected_leader = cluster.leader_id();

    for _ in 0..steps {
        match rng.next() % 6 {
            0 => {
                let node = 1 + (rng.next() % 3);
                cluster.filters().isolate(node, [1, 2, 3]);
            }
            1 => cluster.filters().recover(),
            2 => {
                let from = 1 + (rng.next() % 3);
                let to = 1 + (rng.next() % 3);
                if from != to {
                    cluster
                        .filters()
                        .add_filter(RaftPacketFilter::drop_between(from, to));
                }
            }
            3 => cluster.tick_all(3),
            4 => {
                if let Some(leader) = cluster.leader_id() {
                    let voter = 4 + (rng.next() % 3);
                    let _ = cluster.propose_add_voter(leader, voter);
                }
            }
            _ => {
                let candidate = 1 + (rng.next() % 3);
                cluster.campaign(candidate);
            }
        }
        cluster.tick_all(1);
        let mut leaders_by_term = BTreeMap::<u64, Vec<u64>>::new();
        for node_id in [1, 2, 3] {
            let snapshot = cluster.node(node_id).snapshot();
            if snapshot.role == hydracache_cluster_raft::RaftRuntimeRole::Leader {
                leaders_by_term
                    .entry(snapshot.term)
                    .or_default()
                    .push(node_id);
            }
        }
        for (term, leaders) in leaders_by_term {
            assert!(
                leaders.len() <= 1,
                "seed {seed} observed multiple leaders in term {term} among original voters: {leaders:?}"
            );
        }
        if let Some(leader) = cluster.leader_id() {
            expected_leader = Some(leader);
        }
    }

    cluster.filters().recover();
    cluster.tick_all(10);
    assert!(
        cluster.leader_id().or(expected_leader).is_some(),
        "seed {seed} never observed a stable leader"
    );
}

fn prevote_nightly_soak_enabled() -> bool {
    std::env::var(PREVOTE_NIGHTLY_SOAK_ENV)
        .map(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

fn env_u64(name: &str) -> Option<u64> {
    std::env::var(name).ok()?.parse().ok()
}

struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    fn next(&mut self) -> u64 {
        let mut value = self.state;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.state = value;
        value
    }
}
