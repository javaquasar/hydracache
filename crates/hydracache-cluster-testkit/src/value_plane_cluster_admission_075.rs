//! Runtime-only admission for actual cluster work. Byte credits describe logical owned
//! payload, not RSS. A permit owns its credits until the real operation finishes or drops.
use crate::value_plane_model_075::MutationIdentity;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Mutex};
use std::task::Waker;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClusterLane {
    Safety,
    Data,
    Repair,
}
impl ClusterLane {
    pub const fn index(self) -> usize {
        match self {
            Self::Safety => 0,
            Self::Data => 1,
            Self::Repair => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterWork {
    pub tenant: String,
    pub partition: u32,
    pub key: Vec<u8>,
    pub lane: ClusterLane,
    /// Includes tenant/key bytes and the bounded request retained by the caller.
    pub bytes: usize,
    pub replica_bytes: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct ClusterAdmissionBounds {
    pub max_jobs: usize,
    pub max_repair_jobs: usize,
    pub max_safety_jobs: usize,
    pub max_bytes: usize,
    pub max_tenant_bytes: usize,
    pub max_work_bytes: usize,
    pub max_partition_jobs: usize,
    pub max_hot_key_waiters: usize,
    pub max_replica_bytes: usize,
    pub max_repair_bytes: usize,
    pub max_safety_bytes: usize,
    pub data_permits: usize,
    pub repair_permits: usize,
    pub safety_permits: usize,
    pub max_identifier_bytes: usize,
    pub max_key_bytes: usize,
}
impl Default for ClusterAdmissionBounds {
    fn default() -> Self {
        Self {
            max_jobs: 256,
            max_repair_jobs: 32,
            max_safety_jobs: 16,
            max_bytes: 8 * 1024 * 1024,
            max_tenant_bytes: 4 * 1024 * 1024,
            max_work_bytes: 1024 * 1024,
            max_partition_jobs: 32,
            max_hot_key_waiters: 16,
            max_replica_bytes: 8 * 1024 * 1024,
            max_repair_bytes: 4 * 1024 * 1024,
            max_safety_bytes: 64 * 1024,
            data_permits: 8,
            repair_permits: 1,
            safety_permits: 1,
            max_identifier_bytes: 128,
            max_key_bytes: 1024 * 1024,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClusterAdmissionError {
    InvalidBound,
    InvalidWork,
    Exhausted(&'static str),
    ReceiptConflict,
    InvocationInFlight,
}
impl fmt::Display for ClusterAdmissionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ClusterAdmissionError {}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ClusterAdmissionObservations {
    pub queued: [usize; 3],
    pub inflight: [usize; 3],
    pub owned_bytes: [usize; 3],
    pub replica_bytes: usize,
    pub released: u64,
    pub cancelled: u64,
    pub logical_tick: u64,
    pub maximum_wait: u64,
}
struct Job {
    work: Arc<ClusterWork>,
    partitions: Arc<[u32]>,
    keys: Arc<[[u8; 32]]>,
    admitted: u64,
    running: bool,
}
struct State {
    waiters: BTreeMap<u64, Waker>,
    bounds: ClusterAdmissionBounds,
    jobs: BTreeMap<u64, Job>,
    next: u64,
    tick: u64,
    lane: usize,
    last_tenant: [Option<String>; 3],
    released: u64,
    cancelled: u64,
    maximum_wait: u64,
}
#[derive(Clone)]
pub struct ClusterAdmission {
    state: Arc<Mutex<State>>,
}
pub struct ClusterPermit {
    credits: Arc<PermitCredits>,
    work: Arc<ClusterWork>,
    partitions: Arc<[u32]>,
    keys: Arc<[[u8; 32]]>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionBinding {
    pub namespace: String,
    pub namespace_generation: u64,
    pub request_digest: [u8; 32],
    pub identity: Option<MutationIdentity>,
    pub deadline: u64,
    pub grant_digest: Option<[u8; 32]>,
}
struct PermitCredits {
    state: Arc<Mutex<State>>,
    ticket: u64,
    attempt: Mutex<(Option<AdmissionBinding>, bool)>,
}
/// One physical attempt borrows the same logical receipt. Credits outlive either owner.
pub struct ClusterAttempt {
    credits: Arc<PermitCredits>,
}
impl Drop for ClusterAttempt {
    fn drop(&mut self) {
        self.credits.attempt.lock().unwrap().1 = false;
    }
}
impl Drop for PermitCredits {
    fn drop(&mut self) {
        let mut state = self.state.lock().unwrap();
        if state.jobs.remove(&self.ticket).is_some() {
            state.released = state.released.saturating_add(1);
        }
        state.waiters.remove(&self.ticket);
        let wakes = std::mem::take(&mut state.waiters);
        drop(state);
        for wake in wakes.into_values() {
            wake.wake();
        }
    }
}
impl ClusterPermit {
    pub fn ticket(&self) -> u64 {
        self.credits.ticket
    }
    pub fn work(&self) -> &ClusterWork {
        &self.work
    }
    pub fn partitions(&self) -> &[u32] {
        &self.partitions
    }
    pub fn keys_match(&self, keys: &[Vec<u8>]) -> bool {
        key_fingerprints(keys, self.work.bytes)
            .is_some_and(|actual| actual.as_slice() == self.keys.as_ref())
    }
    pub fn finish(self) {
        drop(self);
    }
    pub fn begin_invocation(
        &self,
        binding: AdmissionBinding,
    ) -> Result<ClusterAttempt, ClusterAdmissionError> {
        if binding.namespace.is_empty()
            || binding.namespace.len() > 128
            || binding.namespace_generation == 0
            || binding
                .identity
                .as_ref()
                .is_some_and(|identity| identity.client.len() > 128)
        {
            return Err(ClusterAdmissionError::InvalidWork);
        }
        let mut state = self.credits.attempt.lock().unwrap();
        if state
            .0
            .as_ref()
            .is_some_and(|previous| previous != &binding)
        {
            return Err(ClusterAdmissionError::ReceiptConflict);
        }
        if state.1 {
            return Err(ClusterAdmissionError::InvocationInFlight);
        }
        state.0 = Some(binding);
        state.1 = true;
        Ok(ClusterAttempt {
            credits: Arc::clone(&self.credits),
        })
    }
    pub fn receipt_digest(&self, binding: &AdmissionBinding) -> [u8; 32] {
        let mut hash = Sha256::new();
        hash.update(self.ticket().to_le_bytes());
        for value in [self.work.tenant.as_bytes(), binding.namespace.as_bytes()] {
            hash.update((value.len() as u64).to_le_bytes());
            hash.update(value);
        }
        hash.update(binding.namespace_generation.to_le_bytes());
        hash.update(binding.request_digest);
        hash.update(binding.deadline.to_le_bytes());
        hash.update([binding.grant_digest.is_some() as u8]);
        hash.update(binding.grant_digest.unwrap_or([0; 32]));
        hash.update([binding.identity.is_some() as u8]);
        if let Some(identity) = &binding.identity {
            hash.update((identity.client.len() as u64).to_le_bytes());
            hash.update(identity.client.as_bytes());
            hash.update(identity.sequence.to_le_bytes());
        }
        hash.update((self.work.bytes as u64).to_le_bytes());
        hash.update((self.work.replica_bytes as u64).to_le_bytes());
        hash.update([self.work.lane.index() as u8]);
        hash.update((self.partitions.len() as u64).to_le_bytes());
        for partition in self.partitions.iter() {
            hash.update(partition.to_le_bytes());
        }
        hash.update((self.keys.len() as u64).to_le_bytes());
        for key in self.keys.iter() {
            hash.update(key);
        }
        hash.finalize().into()
    }
}
impl ClusterAdmission {
    pub fn new(bounds: ClusterAdmissionBounds) -> Result<Self, ClusterAdmissionError> {
        if [
            bounds.max_jobs,
            bounds.max_repair_jobs,
            bounds.max_safety_jobs,
            bounds.max_bytes,
            bounds.max_tenant_bytes,
            bounds.max_work_bytes,
            bounds.max_partition_jobs,
            bounds.max_hot_key_waiters,
            bounds.max_replica_bytes,
            bounds.max_repair_bytes,
            bounds.max_safety_bytes,
            bounds.data_permits,
            bounds.repair_permits,
            bounds.safety_permits,
            bounds.max_identifier_bytes,
            bounds.max_key_bytes,
        ]
        .iter()
        .any(|value| *value == 0 || *value > isize::MAX as usize)
        {
            return Err(ClusterAdmissionError::InvalidBound);
        }
        Ok(Self {
            state: Arc::new(Mutex::new(State {
                waiters: BTreeMap::new(),
                bounds,
                jobs: BTreeMap::new(),
                next: 1,
                tick: 0,
                lane: 0,
                last_tenant: [None, None, None],
                released: 0,
                cancelled: 0,
                maximum_wait: 0,
            })),
        })
    }
    pub fn admit(&self, work: ClusterWork) -> Result<u64, ClusterAdmissionError> {
        let partition = work.partition;
        self.admit_for_partitions(work, &[partition])
    }
    pub fn admit_for_partitions(
        &self,
        work: ClusterWork,
        partitions: &[u32],
    ) -> Result<u64, ClusterAdmissionError> {
        validate_work(&work, self.state.lock().unwrap().bounds)?;
        let key = work.key.clone();
        self.admit_for_resources(work, partitions, &[key])
    }
    pub fn admit_for_resources(
        &self,
        work: ClusterWork,
        partitions: &[u32],
        keys: &[Vec<u8>],
    ) -> Result<u64, ClusterAdmissionError> {
        if partitions.is_empty() || partitions.len() > 256 || !partitions.contains(&work.partition)
        {
            return Err(ClusterAdmissionError::InvalidWork);
        }
        if keys.is_empty() && !work.key.is_empty() {
            return Err(ClusterAdmissionError::InvalidWork);
        }
        let mut partitions = partitions.to_vec();
        partitions.sort_unstable();
        if partitions.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(ClusterAdmissionError::InvalidWork);
        }
        let mut state = self.state.lock().unwrap();
        let b = state.bounds;
        validate_work(&work, b)?;
        if keys.iter().any(|key| key.len() > b.max_key_bytes) {
            return Err(ClusterAdmissionError::InvalidWork);
        }
        let keys = key_fingerprints(keys, work.bytes).ok_or(ClusterAdmissionError::InvalidWork)?;
        let jobs = state
            .jobs
            .values()
            .filter(|job| job.work.lane == work.lane)
            .collect::<Vec<_>>();
        let (count, bytes) = match work.lane {
            ClusterLane::Data => (b.max_jobs, b.max_bytes),
            ClusterLane::Repair => (b.max_repair_jobs, b.max_repair_bytes),
            ClusterLane::Safety => (b.max_safety_jobs, b.max_safety_bytes),
        };
        check(jobs.len().saturating_add(1), count, "lane_jobs")?;
        check(
            jobs.iter()
                .map(|job| job.work.bytes)
                .sum::<usize>()
                .saturating_add(work.bytes),
            bytes,
            "lane_bytes",
        )?;
        if work.lane == ClusterLane::Data {
            check(
                jobs.iter()
                    .filter(|job| job.work.tenant == work.tenant)
                    .map(|job| job.work.bytes)
                    .sum::<usize>()
                    .saturating_add(work.bytes),
                b.max_tenant_bytes,
                "tenant_bytes",
            )?;
            for partition in &partitions {
                check(
                    jobs.iter()
                        .filter(|job| job.partitions.contains(partition))
                        .count()
                        .saturating_add(1),
                    b.max_partition_jobs,
                    "partition_jobs",
                )?;
            }
            for key in &keys {
                check(
                    jobs.iter()
                        .filter(|job| job.work.tenant == work.tenant && job.keys.contains(key))
                        .count()
                        .saturating_add(1),
                    b.max_hot_key_waiters,
                    "hot_key_waiters",
                )?;
            }
            check(work.replica_bytes, b.max_replica_bytes, "replica_bytes")?;
        }
        let next = state
            .next
            .checked_add(1)
            .ok_or(ClusterAdmissionError::Exhausted("ticket"))?;
        let tick = state
            .tick
            .checked_add(1)
            .ok_or(ClusterAdmissionError::Exhausted("tick"))?;
        let ticket = state.next;
        state.next = next;
        state.tick = tick;
        state.jobs.insert(
            ticket,
            Job {
                work: Arc::new(work),
                partitions: partitions.into(),
                keys: keys.into(),
                admitted: tick,
                running: false,
            },
        );
        Ok(ticket)
    }
    pub fn pop(&self) -> Result<Option<ClusterPermit>, ClusterAdmissionError> {
        self.pop_selected(None, &[])
    }
    pub fn pop_for(&self, ticket: u64) -> Result<Option<ClusterPermit>, ClusterAdmissionError> {
        self.pop_selected(Some(ticket), &[])
    }
    pub fn pop_except(
        &self,
        excluded: &[u64],
    ) -> Result<Option<ClusterPermit>, ClusterAdmissionError> {
        self.pop_selected(None, excluded)
    }
    pub fn register_waiter(&self, ticket: u64, waker: &Waker) {
        let mut state = self.state.lock().unwrap();
        if state.jobs.contains_key(&ticket) {
            state.waiters.insert(ticket, waker.clone());
        }
    }
    pub fn wake_waiters(&self) {
        let wakes = std::mem::take(&mut self.state.lock().unwrap().waiters);
        for wake in wakes.into_values() {
            wake.wake();
        }
    }
    fn pop_selected(
        &self,
        expected: Option<u64>,
        excluded: &[u64],
    ) -> Result<Option<ClusterPermit>, ClusterAdmissionError> {
        let mut state = self.state.lock().unwrap();
        state.tick = state
            .tick
            .checked_add(1)
            .ok_or(ClusterAdmissionError::Exhausted("tick"))?;
        let mut selected = None;
        for offset in 0..3 {
            let lane = (state.lane + offset) % 3;
            let limit = [
                state.bounds.safety_permits,
                state.bounds.data_permits,
                state.bounds.repair_permits,
            ][lane];
            if state
                .jobs
                .values()
                .filter(|job| job.running && job.work.lane.index() == lane)
                .count()
                >= limit
            {
                continue;
            }
            let eligible = state
                .jobs
                .iter()
                .filter(|(id, job)| {
                    if job.running || job.work.lane.index() != lane {
                        return false;
                    }
                    if lane != ClusterLane::Data.index() {
                        return true;
                    }
                    let in_flight = state
                        .jobs
                        .values()
                        .filter(|other| other.running && other.work.lane == ClusterLane::Data);
                    if in_flight.clone().any(|other| {
                        other
                            .partitions
                            .iter()
                            .any(|partition| job.partitions.contains(partition))
                    }) {
                        return false;
                    }
                    if in_flight
                        .map(|other| other.work.replica_bytes)
                        .sum::<usize>()
                        .saturating_add(job.work.replica_bytes)
                        > state.bounds.max_replica_bytes
                    {
                        return false;
                    }
                    !state.jobs.iter().any(|(other_id, other)| {
                        other_id < *id
                            && !other.running
                            && other.work.lane == ClusterLane::Data
                            && other
                                .partitions
                                .iter()
                                .any(|partition| job.partitions.contains(partition))
                    })
                })
                .map(|(id, job)| (*id, job.work.tenant.as_str()))
                .collect::<Vec<_>>();
            let tenant = eligible
                .iter()
                .map(|(_, tenant)| *tenant)
                .filter(|tenant| {
                    state.last_tenant[lane]
                        .as_deref()
                        .is_none_or(|last| *tenant > last)
                })
                .min()
                .or_else(|| eligible.iter().map(|(_, tenant)| *tenant).min());
            if let Some(tenant) = tenant {
                selected = eligible
                    .iter()
                    .find(|(_, name)| *name == tenant)
                    .map(|(id, _)| (*id, lane));
            }
            if selected.is_some() {
                break;
            }
        }
        let Some((ticket, lane)) = selected else {
            return Ok(None);
        };
        if expected.is_some_and(|expected| expected != ticket) || excluded.contains(&ticket) {
            return Ok(None);
        }
        let tick = state.tick;
        let job = state.jobs.get_mut(&ticket).unwrap();
        job.running = true;
        let waited = tick - job.admitted;
        let work = job.work.clone();
        let partitions = job.partitions.clone();
        let keys = job.keys.clone();
        state.maximum_wait = state.maximum_wait.max(waited);
        state.last_tenant[lane] = Some(work.tenant.clone());
        state.lane = (lane + 1) % 3;
        Ok(Some(ClusterPermit {
            credits: Arc::new(PermitCredits {
                state: self.state.clone(),
                ticket,
                attempt: Mutex::new((None, false)),
            }),
            work,
            partitions,
            keys,
        }))
    }
    pub fn cancel(&self, ticket: u64) -> Result<bool, ClusterAdmissionError> {
        let mut state = self.state.lock().unwrap();
        if state.jobs.get(&ticket).is_none_or(|job| job.running) {
            return Ok(false);
        }
        state.jobs.remove(&ticket);
        state.cancelled = state.cancelled.saturating_add(1);
        state.released = state.released.saturating_add(1);
        Ok(true)
    }
    pub fn observations(&self) -> ClusterAdmissionObservations {
        let state = self.state.lock().unwrap();
        let mut seen = ClusterAdmissionObservations {
            released: state.released,
            cancelled: state.cancelled,
            logical_tick: state.tick,
            maximum_wait: state.maximum_wait,
            ..Default::default()
        };
        for job in state.jobs.values() {
            let lane = job.work.lane.index();
            seen.owned_bytes[lane] = seen.owned_bytes[lane].saturating_add(job.work.bytes);
            if job.running {
                seen.inflight[lane] += 1;
                if job.work.lane == ClusterLane::Data {
                    seen.replica_bytes = seen.replica_bytes.saturating_add(job.work.replica_bytes);
                }
            } else {
                seen.queued[lane] += 1;
            }
        }
        seen
    }
}
fn check(
    actual: usize,
    maximum: usize,
    dimension: &'static str,
) -> Result<(), ClusterAdmissionError> {
    if actual > maximum {
        Err(ClusterAdmissionError::Exhausted(dimension))
    } else {
        Ok(())
    }
}
fn validate_work(
    work: &ClusterWork,
    b: ClusterAdmissionBounds,
) -> Result<(), ClusterAdmissionError> {
    if work.tenant.is_empty()
        || work.tenant.len() > b.max_identifier_bytes
        || work.key.len() > b.max_key_bytes
        || work.bytes < work.tenant.len().saturating_add(work.key.len())
        || work.bytes == 0
        || work.bytes > b.max_work_bytes
    {
        return Err(ClusterAdmissionError::InvalidWork);
    }
    Ok(())
}
fn key_fingerprints(keys: &[Vec<u8>], maximum: usize) -> Option<Vec<[u8; 32]>> {
    if keys.len() > 256 {
        return None;
    }
    let mut total = 0usize;
    for key in keys {
        total = total.checked_add(key.len())?;
        if total > maximum {
            return None;
        }
    }
    let mut hashes = keys
        .iter()
        .map(|key| <[u8; 32]>::from(Sha256::digest(key)))
        .collect::<Vec<_>>();
    hashes.sort_unstable();
    hashes.dedup();
    Some(hashes)
}

#[cfg(test)]
mod exhaustion_tests {
    use super::*;
    #[test]
    fn exhausted_ticket_or_tick_refuses_before_job_or_credit_ownership_changes() {
        let queue = ClusterAdmission::new(Default::default()).unwrap();
        let work = || ClusterWork {
            tenant: "a".into(),
            partition: 0,
            key: b"k".to_vec(),
            lane: ClusterLane::Data,
            bytes: 16,
            replica_bytes: 1,
        };
        queue.state.lock().unwrap().next = u64::MAX;
        assert_eq!(
            queue.admit(work()),
            Err(ClusterAdmissionError::Exhausted("ticket"))
        );
        assert_eq!(queue.observations().owned_bytes, [0; 3]);
        {
            let mut state = queue.state.lock().unwrap();
            state.next = 1;
            state.tick = u64::MAX;
        }
        assert_eq!(
            queue.admit(work()),
            Err(ClusterAdmissionError::Exhausted("tick"))
        );
        assert!(matches!(
            queue.pop(),
            Err(ClusterAdmissionError::Exhausted("tick"))
        ));
        assert_eq!(queue.observations().owned_bytes, [0; 3]);
        assert_eq!(queue.state.lock().unwrap().next, 1);
    }
}
