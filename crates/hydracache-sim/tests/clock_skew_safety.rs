use std::collections::{BTreeMap, BTreeSet};

use hydracache::{
    ClusterEpoch, ConditionalError, ConsistencyLevel, LockOwner, LogicalDuration, LogicalTime,
    SingleKeyConditionalStore,
};
use hydracache_cluster_raft::RaftRuntimeRole;
use hydracache_cluster_testkit::RuntimeRaftCluster;
use hydracache_sim::{run_lock_safety, LockSafetyScenario, SimClock};

const LOCK_KEY: &str = "clock-skew:lock";

fn owner(session: &'static str) -> LockOwner {
    LockOwner::new(session, 0)
}

fn record_leaders(
    cluster: &RuntimeRaftCluster,
    leaders_by_term: &mut BTreeMap<u64, BTreeSet<u64>>,
) {
    for node_id in cluster.node_ids() {
        let snapshot = cluster.node(node_id).snapshot();
        if snapshot.role == RaftRuntimeRole::Leader {
            leaders_by_term
                .entry(snapshot.term)
                .or_default()
                .insert(node_id);
        }
    }
}

#[test]
fn clock_skew_does_not_produce_two_leaders() {
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    let mut leaders_by_term = BTreeMap::new();
    record_leaders(&cluster, &mut leaders_by_term);

    let mut clocks = BTreeMap::from([
        (1, SimClock::new(LogicalTime::from_millis(1_000))),
        (2, SimClock::new(LogicalTime::from_millis(1_125))),
        (3, SimClock::new(LogicalTime::from_millis(940))),
    ]);
    cluster.filters().isolate(1, [1, 2, 3]);

    for step in 0..36 {
        for node_id in [1, 2, 3] {
            let increment = match node_id {
                1 => 10,
                2 => {
                    if step < 18 {
                        30
                    } else {
                        5
                    }
                }
                3 => {
                    if step % 2 == 0 {
                        20
                    } else {
                        5
                    }
                }
                _ => unreachable!(),
            };
            clocks
                .get_mut(&node_id)
                .expect("clock exists")
                .advance(LogicalDuration::from_millis(increment));
            let local_ticks = (increment / 10).clamp(1, 3);
            for _ in 0..local_ticks {
                cluster.tick_node(node_id);
                record_leaders(&cluster, &mut leaders_by_term);
            }
        }
        if step == 20 {
            cluster.filters().recover();
        }
    }

    for (term, leaders) in leaders_by_term {
        assert!(
            leaders.len() <= 1,
            "term {term} had multiple leaders under skewed tick rates: {leaders:?}"
        );
    }
}

#[test]
fn backward_clock_jump_preserves_fence_monotonicity_and_no_zombie_holder() {
    let mut clock = SimClock::new(LogicalTime::from_millis(1_000));
    let mut store =
        SingleKeyConditionalStore::new(ClusterEpoch::new(1), 8).with_lock_acquire_limit(Some(1));

    let first = store
        .try_acquire_lock(
            LOCK_KEY,
            ConsistencyLevel::Quorum,
            owner("session-a"),
            LogicalDuration::from_millis(100),
            clock.now(),
        )
        .unwrap()
        .expect("first owner acquires");

    clock.set(LogicalTime::from_millis(950));
    assert!(
        store
            .try_acquire_lock(
                LOCK_KEY,
                ConsistencyLevel::Quorum,
                owner("session-b"),
                LogicalDuration::from_millis(100),
                clock.now(),
            )
            .unwrap()
            .is_none(),
        "backward clock jump must not expire the active owner early"
    );

    clock.set(LogicalTime::from_millis(1_101));
    store.expire_due(clock.now());
    let second = store
        .try_acquire_lock(
            LOCK_KEY,
            ConsistencyLevel::Quorum,
            owner("session-b"),
            LogicalDuration::from_millis(100),
            clock.now(),
        )
        .unwrap()
        .expect("second owner acquires after true expiry");
    assert!(second.value() > first.value());

    let zombie = store.release_lock(LOCK_KEY, &owner("session-a"), first);
    assert!(
        matches!(
            zombie,
            Err(ConditionalError::NotOwner { .. })
                | Err(ConditionalError::StaleFenceToken { .. })
                | Err(ConditionalError::LeaseExpired { .. })
        ),
        "old owner must not release or revive the lock after backward-jump recovery: {zombie:?}"
    );

    let report = run_lock_safety(LockSafetyScenario {
        seed: 0x64_14_02,
        steps: 96,
        clients: 4,
    });
    assert!(
        report.invariants.is_ok(),
        "lock-safety scenario violated invariants after W14 bridge: {:?}",
        report.invariants.violations
    );
    assert!(report.zombie_rejections > 0);
    assert!(
        report
            .acquired_fences
            .windows(2)
            .all(|pair| pair[1] > pair[0]),
        "fences were not monotonic: {:?}",
        report.acquired_fences
    );
}

#[test]
fn canary_clock_skew_allows_two_leaders() {
    let mut leaders = BTreeMap::new();
    leaders.insert(7, BTreeSet::from([1, 2]));
    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W14") {
        assert!(
            leaders.values().all(|term_leaders| term_leaders.len() <= 1),
            "HC-CANARY-RED:W14 two leaders observed in one term"
        );
    }
    assert!(
        leaders.values().any(|term_leaders| term_leaders.len() > 1),
        "canary fixture must model the forbidden same-term two-leader outcome"
    );
}
