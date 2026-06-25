use std::collections::BTreeSet;

use hydracache::{
    ClusterEpoch, ConditionalError, ConsistencyLevel, FenceToken, LockOwner, LogicalDuration,
    LogicalTime, SingleKeyConditionalStore,
};

use crate::{InvariantReport, SimRng};

const LOCK_KEY: &str = "sim:lock:account-42";

/// Deterministic lock-safety scenario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockSafetyScenario {
    /// Replay seed.
    pub seed: u64,
    /// Number of deterministic scheduler steps.
    pub steps: u64,
    /// Number of contending clients.
    pub clients: u64,
}

impl LockSafetyScenario {
    /// Create the complete W7 scenario.
    pub fn all(seed: u64) -> Self {
        Self {
            seed,
            steps: 64,
            clients: 4,
        }
    }
}

/// Lock-safety report returned by the W7 simulator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockSafetyReport {
    /// Replay seed.
    pub seed: u64,
    /// Steps executed.
    pub steps: u64,
    /// Stable digest over the simulated trace.
    pub trace_digest: u64,
    /// Number of successful lock acquisitions.
    pub acquisitions: u64,
    /// Number of leader changes exercised.
    pub leader_changes: u64,
    /// Number of partitioned/blocked client attempts.
    pub partition_blocks: u64,
    /// Number of session-loss expirations.
    pub session_losses: u64,
    /// Number of zombie-owner attempts rejected.
    pub zombie_rejections: u64,
    /// Number of weak consistency attempts rejected.
    pub weak_rejections: u64,
    /// Number of weak consistency attempts that incorrectly acquired a lock.
    pub weak_acquisitions: u64,
    /// Number of reentrant-limit attempts rejected.
    pub reentrancy_rejections: u64,
    /// Highest observed number of live owners for the key.
    pub max_live_owners: u64,
    /// Fences observed on successful ownership changes.
    pub acquired_fences: Vec<u64>,
    /// Invariant verdict.
    pub invariants: InvariantReport,
}

/// Run the W7 lock-safety scenario.
pub fn run_lock_safety(scenario: LockSafetyScenario) -> LockSafetyReport {
    let clients = scenario.clients.max(2);
    let mut sim = LockSafetySim::new(scenario.seed, clients);
    for step in 0..scenario.steps {
        sim.step(step);
    }
    sim.finish(scenario.steps)
}

struct LockSafetySim {
    seed: u64,
    rng: SimRng,
    store: SingleKeyConditionalStore,
    now: LogicalTime,
    clients: u64,
    leader_epoch: u64,
    partitioned_clients: BTreeSet<u64>,
    trace: Vec<String>,
    acquisitions: u64,
    leader_changes: u64,
    partition_blocks: u64,
    session_losses: u64,
    zombie_rejections: u64,
    weak_rejections: u64,
    weak_acquisitions: u64,
    reentrancy_rejections: u64,
    max_live_owners: u64,
    acquired_fences: Vec<u64>,
    first_owner_fence: Option<FenceToken>,
}

impl LockSafetySim {
    fn new(seed: u64, clients: u64) -> Self {
        Self {
            seed,
            rng: SimRng::from_seed(seed),
            store: SingleKeyConditionalStore::new(ClusterEpoch::new(1), 32)
                .with_lock_acquire_limit(Some(1)),
            now: LogicalTime::from_millis(0),
            clients,
            leader_epoch: 1,
            partitioned_clients: BTreeSet::new(),
            trace: Vec::new(),
            acquisitions: 0,
            leader_changes: 0,
            partition_blocks: 0,
            session_losses: 0,
            zombie_rejections: 0,
            weak_rejections: 0,
            weak_acquisitions: 0,
            reentrancy_rejections: 0,
            max_live_owners: 0,
            acquired_fences: Vec::new(),
            first_owner_fence: None,
        }
    }

    fn step(&mut self, step: u64) {
        self.advance(1);
        match step {
            0 => self.acquire(0, ConsistencyLevel::Quorum),
            1 => self.acquire(1, ConsistencyLevel::One),
            2 => self.acquire(0, ConsistencyLevel::Quorum),
            3 => self.partition_client(2),
            4 => self.acquire(2, ConsistencyLevel::Quorum),
            5 => self.change_leader(),
            6 => self.expire_current_owner_by_session_loss(),
            7 => self.acquire(1, ConsistencyLevel::Quorum),
            8 => self.zombie_unlock(0),
            _ => self.seeded_contention(),
        }
        self.observe_live_owners();
    }

    fn seeded_contention(&mut self) {
        if self.rng.chance(1, 11) {
            self.change_leader();
        }
        if self.rng.chance(1, 13) {
            let client = self.next_client();
            if self.partitioned_clients.contains(&client) {
                self.heal_client(client);
            } else {
                self.partition_client(client);
            }
        }
        if self.rng.chance(1, 17) {
            self.expire_current_owner_by_session_loss();
        }
        let client = self.next_client();
        self.acquire(client, ConsistencyLevel::Quorum);
    }

    fn acquire(&mut self, client: u64, level: ConsistencyLevel) {
        if self.partitioned_clients.contains(&client) {
            self.partition_blocks = self.partition_blocks.saturating_add(1);
            self.record(format!(
                "blocked:client-{client}:leader-{}",
                self.leader_epoch
            ));
            return;
        }

        let owner = owner(client);
        let result = self.store.try_acquire_lock(
            LOCK_KEY,
            level,
            owner,
            LogicalDuration::from_millis(1_000),
            self.now,
        );
        match result {
            Ok(Some(fence)) if level == ConsistencyLevel::One => {
                self.weak_acquisitions = self.weak_acquisitions.saturating_add(1);
                self.record(format!("weak-acquired:client-{client}:{}", fence.value()));
            }
            Ok(Some(fence)) => {
                self.acquisitions = self.acquisitions.saturating_add(1);
                if self.first_owner_fence.is_none() {
                    self.first_owner_fence = Some(fence);
                }
                if self.acquired_fences.last().copied() != Some(fence.value()) {
                    self.acquired_fences.push(fence.value());
                }
                self.record(format!("acquired:client-{client}:{}", fence.value()));
            }
            Ok(None) => {
                self.record(format!("busy:client-{client}"));
            }
            Err(ConditionalError::WeakConsistency { .. }) => {
                self.weak_rejections = self.weak_rejections.saturating_add(1);
                self.record(format!("weak-rejected:client-{client}"));
            }
            Err(ConditionalError::ReentrancyLimit { .. }) => {
                self.reentrancy_rejections = self.reentrancy_rejections.saturating_add(1);
                self.record(format!("reentrant-rejected:client-{client}"));
            }
            Err(error) => {
                self.record(format!("acquire-error:client-{client}:{error}"));
            }
        }
    }

    fn zombie_unlock(&mut self, client: u64) {
        let Some(fence) = self.first_owner_fence else {
            return;
        };
        let result = self.store.release_lock(LOCK_KEY, &owner(client), fence);
        if matches!(
            result,
            Err(ConditionalError::NotOwner { .. })
                | Err(ConditionalError::StaleFenceToken { .. })
                | Err(ConditionalError::LeaseExpired { .. })
        ) {
            self.zombie_rejections = self.zombie_rejections.saturating_add(1);
            self.record(format!("zombie-rejected:client-{client}:{}", fence.value()));
        }
    }

    fn expire_current_owner_by_session_loss(&mut self) {
        let Some(hold) = self.store.lock_hold(LOCK_KEY).cloned() else {
            return;
        };
        self.advance(2);
        let expired = self
            .store
            .expire_lost_sessions(self.now, LogicalDuration::from_millis(1));
        self.session_losses = self.session_losses.saturating_add(expired as u64);
        self.record(format!(
            "session-loss:{:?}:{}",
            hold.owner.session,
            hold.fence.value()
        ));
    }

    fn change_leader(&mut self) {
        self.leader_epoch = self.leader_epoch.saturating_add(1);
        self.leader_changes = self.leader_changes.saturating_add(1);
        self.record(format!("leader-change:{}", self.leader_epoch));
    }

    fn partition_client(&mut self, client: u64) {
        self.partitioned_clients.insert(client);
        self.record(format!("partition:client-{client}"));
    }

    fn heal_client(&mut self, client: u64) {
        self.partitioned_clients.remove(&client);
        self.record(format!("heal:client-{client}"));
    }

    fn next_client(&mut self) -> u64 {
        self.rng.next_index(self.clients as usize) as u64
    }

    fn advance(&mut self, millis: u64) {
        self.now = self
            .now
            .saturating_add(LogicalDuration::from_millis(millis));
        self.store.expire_due(self.now);
    }

    fn observe_live_owners(&mut self) {
        let live = u64::from(self.store.lock_hold(LOCK_KEY).is_some());
        self.max_live_owners = self.max_live_owners.max(live);
    }

    fn finish(self, steps: u64) -> LockSafetyReport {
        let mut report = InvariantReport::default();
        record_mutual_exclusion(self.max_live_owners, &mut report);
        record_fence_monotonicity(&self.acquired_fences, &mut report);
        record_required_faults(&self, &mut report);

        LockSafetyReport {
            seed: self.seed,
            steps,
            trace_digest: trace_digest(&self.trace),
            acquisitions: self.acquisitions,
            leader_changes: self.leader_changes,
            partition_blocks: self.partition_blocks,
            session_losses: self.session_losses,
            zombie_rejections: self.zombie_rejections,
            weak_rejections: self.weak_rejections,
            weak_acquisitions: self.weak_acquisitions,
            reentrancy_rejections: self.reentrancy_rejections,
            max_live_owners: self.max_live_owners,
            acquired_fences: self.acquired_fences,
            invariants: report,
        }
    }

    fn record(&mut self, event: String) {
        self.trace.push(format!("{}:{event}", self.now.as_millis()));
    }
}

fn record_mutual_exclusion(max_live_owners: u64, report: &mut InvariantReport) {
    report.record_check();
    if max_live_owners > 1 {
        report.record_violation(
            "lock-mutual-exclusion",
            format!("observed {max_live_owners} live owners for one key"),
        );
    }
}

fn record_fence_monotonicity(fences: &[u64], report: &mut InvariantReport) {
    report.record_check();
    for pair in fences.windows(2) {
        if pair[1] <= pair[0] {
            report.record_violation(
                "lock-fence-monotonicity",
                format!("fence {} was not greater than {}", pair[1], pair[0]),
            );
        }
    }
}

fn record_required_faults(sim: &LockSafetySim, report: &mut InvariantReport) {
    for (name, ok) in [
        ("lock-leader-change-exercised", sim.leader_changes > 0),
        ("lock-partition-exercised", sim.partition_blocks > 0),
        ("lock-session-loss-exercised", sim.session_losses > 0),
        ("lock-zombie-rejected", sim.zombie_rejections > 0),
        ("lock-weak-consistency-rejected", sim.weak_rejections > 0),
        (
            "lock-reentrancy-limit-rejected",
            sim.reentrancy_rejections > 0,
        ),
        ("lock-no-weak-acquisition", sim.weak_acquisitions == 0),
    ] {
        report.record_check();
        if !ok {
            report.record_violation(name, "required lock-safety condition was not observed");
        }
    }
}

fn owner(client: u64) -> LockOwner {
    LockOwner::new(format!("client-{client}"), 0)
}

fn trace_digest(trace: &[String]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for line in trace {
        for byte in line.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
}
