//! Bounded evidence and admission for the distributed value plane.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::RwLock;
use std::time::{Duration, Instant};

use hydracache::{ClusterEpoch, ClusterGeneration, ClusterNodeId};
use hydracache_cluster_raft::RaftPartitionSnapshot;
use serde::Serialize;
use tokio::sync::{watch, Semaphore, SemaphorePermit, TryAcquireError};

use crate::config::IMapOperationalConfig;
use crate::imap_topology::PartitionMember;

pub(crate) const VALUE_WIRE_GENERATION: u16 = 2;
const HEALTH_TTL: Duration = Duration::from_secs(3);
const ACK_BUCKETS_US: [u64; 7] = [100, 500, 1_000, 5_000, 20_000, 100_000, 500_000];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IMapReadinessReason {
    Ready,
    Draining,
    TopologyUnavailable,
    EpochStale,
    PartitionCatchingUp,
    RequiredBackupUnavailable,
    StoreRecoveryIncomplete,
    TransferIncomplete,
    ClockUncertaintyExceeded,
    WireGenerationIncompatible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct IMapReadiness {
    pub ready: bool,
    pub epoch: u64,
    pub reason: IMapReadinessReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PartitionState {
    Empty,
    Healthy,
    CatchingUp,
    UnderReplicated,
    Moving,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct PartitionHealth {
    pub id: u32,
    pub state: PartitionState,
    pub local_primary: bool,
    pub local_backup: bool,
    pub certified_version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HealthObservation {
    pub epoch: u64,
    pub reason: IMapReadinessReason,
    pub rows: Vec<PartitionHealth>,
    pub compatible_members: usize,
    pub reachable_members: usize,
    pub retained_decisions: usize,
    pub repairs: usize,
    pub promotions: usize,
}

impl HealthObservation {
    pub fn unavailable(reason: IMapReadinessReason, epoch: u64) -> Self {
        Self {
            epoch,
            reason,
            rows: Vec::new(),
            compatible_members: 0,
            reachable_members: 0,
            retained_decisions: 0,
            repairs: 0,
            promotions: 0,
        }
    }
}

#[derive(Debug, Clone)]
struct Sample {
    observed_at: Instant,
    sequence: u64,
    observation: HealthObservation,
}

#[derive(Debug, Clone)]
pub(crate) struct IMapEvidence {
    pub readiness: IMapReadiness,
    pub observation: HealthObservation,
    pub sequence: u64,
    pub age_ms: u64,
    pub complete: bool,
    pub policy: IMapOperationalConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Denial {
    Draining,
    Saturated,
}

#[derive(Debug, Default)]
pub(crate) struct IMapCounters {
    pub direct: AtomicU64,
    pub proxy: AtomicU64,
    pub stale: AtomicU64,
    pub backpressure: AtomicU64,
    pub drain_rejected: AtomicU64,
    pub incompatible: AtomicU64,
    pub topology_rejected: AtomicU64,
    pub listener_gaps: AtomicU64,
    pub repairs: AtomicU64,
    pub handoffs: AtomicU64,
    pub dedup_replays: AtomicU64,
    ack_count: AtomicU64,
    ack_failed: AtomicU64,
    ack_sum_us: AtomicU64,
    ack_buckets: [AtomicU64; ACK_BUCKETS_US.len()],
}

#[derive(Debug)]
pub(crate) struct IMapOps {
    pub config: IMapOperationalConfig,
    ingress: Semaphore,
    owners: Semaphore,
    draining: watch::Sender<bool>,
    pub ownership_evacuated: AtomicBool,
    pub membership_left: AtomicBool,
    sample: RwLock<Option<Sample>>,
    compatible: RwLock<BTreeMap<ClusterNodeId, ClusterGeneration>>,
    pub counters: IMapCounters,
}

impl IMapOps {
    pub fn new(config: IMapOperationalConfig) -> Self {
        Self {
            config,
            ingress: Semaphore::new(config.max_inflight),
            owners: Semaphore::new(config.max_inflight),
            draining: watch::channel(false).0,
            ownership_evacuated: AtomicBool::new(false),
            membership_left: AtomicBool::new(false),
            sample: RwLock::new(None),
            compatible: RwLock::new(BTreeMap::new()),
            counters: IMapCounters::default(),
        }
    }

    pub fn enter(&self, owner: bool) -> Result<SemaphorePermit<'_>, Denial> {
        let semaphore = if owner { &self.owners } else { &self.ingress };
        semaphore.try_acquire().map_err(|error| match error {
            TryAcquireError::Closed => {
                self.counters.drain_rejected.fetch_add(1, Ordering::Relaxed);
                Denial::Draining
            }
            TryAcquireError::NoPermits => {
                self.backpressure();
                Denial::Saturated
            }
        })
    }

    pub fn begin_drain(&self) {
        self.ingress.close();
        self.owners.close();
        self.draining.send_replace(true);
    }

    pub fn is_draining(&self) -> bool {
        self.owners.is_closed()
    }

    pub fn proves_member(&self, member: &PartitionMember) -> bool {
        self.compatible
            .read()
            .expect("IMap capability lock")
            .get(&member.node)
            == Some(&member.generation)
    }

    pub fn record_compatible(&self, snapshot: &RaftPartitionSnapshot, peers: Vec<PartitionMember>) {
        let mut compatible = self.compatible.write().expect("IMap capability lock");
        compatible.retain(|node, generation| {
            snapshot
                .members
                .iter()
                .any(|member| member.node_id == *node && member.generation == *generation)
        });
        for peer in peers {
            if snapshot
                .members
                .iter()
                .any(|member| member.node_id == peer.node && member.generation == peer.generation)
            {
                compatible.insert(peer.node, peer.generation);
            }
        }
    }

    pub fn compatible_members(
        &self,
        snapshot: &RaftPartitionSnapshot,
        local: &PartitionMember,
    ) -> BTreeSet<ClusterNodeId> {
        let compatible = self.compatible.read().expect("IMap capability lock");
        snapshot
            .members
            .iter()
            .filter(|member| {
                let is_local =
                    member.node_id == local.node && member.generation == local.generation;
                is_local || compatible.get(&member.node_id) == Some(&member.generation)
            })
            .map(|member| member.node_id.clone())
            .collect()
    }

    pub fn drain_signal(&self) -> watch::Receiver<bool> {
        self.draining.subscribe()
    }

    pub fn inflight(&self) -> usize {
        2 * self.config.max_inflight
            - self.ingress.available_permits()
            - self.owners.available_permits()
    }

    pub fn backpressure(&self) {
        self.counters.backpressure.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_ack(&self, elapsed: Duration, success: bool) {
        if !success {
            self.counters.ack_failed.fetch_add(1, Ordering::Relaxed);
            return;
        }
        let elapsed = elapsed.as_micros().min(u128::from(u64::MAX)) as u64;
        self.counters.ack_count.fetch_add(1, Ordering::Relaxed);
        self.counters
            .ack_sum_us
            .fetch_add(elapsed, Ordering::Relaxed);
        for (bound, counter) in ACK_BUCKETS_US.iter().zip(&self.counters.ack_buckets) {
            if elapsed <= *bound {
                counter.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    pub fn replica_ack(&self) -> AckObservation<'_> {
        AckObservation {
            ops: self,
            start: Instant::now(),
            success: false,
        }
    }

    pub fn publish(&self, observation: HealthObservation) {
        let mut sample = self.sample.write().expect("IMap health lock");
        let sequence = sample.as_ref().map_or(1, |old| {
            old.sequence
                .saturating_add(u64::from(old.observation != observation))
        });
        *sample = Some(Sample {
            observed_at: Instant::now(),
            sequence,
            observation,
        });
    }

    pub fn evidence(&self, current: Option<(ClusterEpoch, bool)>) -> IMapEvidence {
        let current_epoch = current.map(|(epoch, _)| epoch);
        let mut view = self.sample.read().expect("IMap health lock").clone();
        let epoch = current_epoch.map_or(0, |epoch| epoch.value());
        let fresh = view.as_ref().is_some_and(|sample| {
            sample.observed_at.elapsed() <= HEALTH_TTL
                && Some(sample.observation.epoch) == current_epoch.map(|epoch| epoch.value())
        });
        let reason = if self.is_draining() {
            IMapReadinessReason::Draining
        } else if current_epoch.is_none() {
            IMapReadinessReason::TopologyUnavailable
        } else if !fresh || current.is_some_and(|(_, applied)| !applied) {
            IMapReadinessReason::EpochStale
        } else {
            view.as_ref().expect("fresh sample").observation.reason
        };
        let (observation, sequence, age_ms) = view.take().map_or_else(
            || (HealthObservation::unavailable(reason, epoch), 0, 0),
            |sample| {
                (
                    sample.observation,
                    sample.sequence,
                    sample.observed_at.elapsed().as_millis() as u64,
                )
            },
        );
        IMapEvidence {
            readiness: IMapReadiness {
                ready: reason == IMapReadinessReason::Ready,
                epoch,
                reason,
            },
            complete: fresh
                && current.is_some_and(|(_, applied)| applied)
                && !observation.rows.is_empty(),
            observation,
            sequence,
            age_ms,
            policy: self.config,
        }
    }

    pub fn prometheus(&self, evidence: &IMapEvidence) -> String {
        let mut out = String::from("hydracache_imap_routing_mode{mode=\"distributed\"} 1\n");
        let ready = evidence.readiness;
        let _ = writeln!(out, "hydracache_imap_ready {}", u8::from(ready.ready));
        let _ = writeln!(out, "hydracache_imap_epoch {}", ready.epoch);
        let _ = writeln!(out, "hydracache_imap_inflight {}", self.inflight());
        let _ = writeln!(
            out,
            "hydracache_imap_drain_evacuated {}",
            u8::from(self.ownership_evacuated.load(Ordering::Acquire))
        );
        let _ = writeln!(
            out,
            "hydracache_imap_drain_membership_left {}",
            u8::from(self.membership_left.load(Ordering::Acquire))
        );
        let _ = writeln!(
            out,
            "hydracache_imap_observation_complete {}",
            u8::from(evidence.complete)
        );
        let _ = writeln!(
            out,
            "hydracache_imap_retained_decisions {}",
            evidence.observation.retained_decisions
        );
        let _ = writeln!(
            out,
            "hydracache_imap_repairs_committed {}",
            evidence.observation.repairs
        );
        let _ = writeln!(
            out,
            "hydracache_imap_promotions_committed {}",
            evidence.observation.promotions
        );
        let counters = &self.counters;
        for (metric, value) in [
            ("direct_total", &counters.direct),
            ("proxy_total", &counters.proxy),
            ("stale_owner_total", &counters.stale),
            ("backpressure_total", &counters.backpressure),
            ("drain_rejected_total", &counters.drain_rejected),
            ("incompatible_total", &counters.incompatible),
            ("topology_rejected_total", &counters.topology_rejected),
            ("listener_gaps_total", &counters.listener_gaps),
            ("repair_total", &counters.repairs),
            ("handoff_total", &counters.handoffs),
            ("dedup_replays_total", &counters.dedup_replays),
            ("replica_ack_failed_total", &counters.ack_failed),
        ] {
            let _ = writeln!(out, "# TYPE hydracache_imap_{metric} counter");
            let _ = writeln!(
                out,
                "hydracache_imap_{metric} {}",
                value.load(Ordering::Relaxed)
            );
        }
        for (state, label) in [
            (PartitionState::Empty, "empty"),
            (PartitionState::Healthy, "healthy"),
            (PartitionState::CatchingUp, "catching_up"),
            (PartitionState::UnderReplicated, "under_replicated"),
            (PartitionState::Moving, "moving"),
        ] {
            let count = evidence
                .observation
                .rows
                .iter()
                .filter(|row| row.state == state)
                .count();
            let _ = writeln!(
                out,
                "hydracache_imap_partitions{{state=\"{label}\"}} {count}"
            );
        }
        let count = counters.ack_count.load(Ordering::Relaxed);
        let sum = counters.ack_sum_us.load(Ordering::Relaxed);
        out.push_str("# TYPE hydracache_imap_replica_ack_seconds histogram\n");
        let mut previous_bucket = 0;
        for (bound, bucket) in ACK_BUCKETS_US.iter().zip(&counters.ack_buckets) {
            let count = bucket
                .load(Ordering::Relaxed)
                .max(previous_bucket)
                .min(count);
            previous_bucket = count;
            let _ = writeln!(
                out,
                "hydracache_imap_replica_ack_seconds_bucket{{le=\"{:.6}\"}} {count}",
                *bound as f64 / 1e6
            );
        }
        let _ = writeln!(
            out,
            "hydracache_imap_replica_ack_seconds_bucket{{le=\"+Inf\"}} {count}"
        );
        let _ = writeln!(out, "hydracache_imap_replica_ack_seconds_count {count}");
        let _ = writeln!(
            out,
            "hydracache_imap_replica_ack_seconds_sum {:.6}",
            sum as f64 / 1e6
        );
        out
    }
}

pub(crate) struct AckObservation<'a> {
    ops: &'a IMapOps,
    start: Instant,
    success: bool,
}

impl AckObservation<'_> {
    pub fn confirmed(mut self, success: bool) {
        self.success = success;
    }
}

impl Drop for AckObservation<'_> {
    fn drop(&mut self) {
        self.ops.record_ack(self.start.elapsed(), self.success);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admission_seals_both_paths_without_discarding_inflight_work() {
        let ops = IMapOps::new(IMapOperationalConfig {
            max_inflight: 1,
            ..Default::default()
        });
        let ingress = ops.enter(false).unwrap();
        let owner = ops.enter(true).unwrap();
        assert_eq!(ops.inflight(), 2);
        assert_eq!(ops.enter(false).unwrap_err(), Denial::Saturated);
        assert_eq!(ops.enter(true).unwrap_err(), Denial::Saturated);
        let signal = ops.drain_signal();
        ops.begin_drain();
        assert!(*signal.borrow());
        assert_eq!(ops.enter(false).unwrap_err(), Denial::Draining);
        assert_eq!(ops.enter(true).unwrap_err(), Denial::Draining);
        assert_eq!(ops.inflight(), 2);
        drop((ingress, owner));
        assert_eq!(ops.inflight(), 0);
        assert_eq!(ops.counters.backpressure.load(Ordering::Relaxed), 2);
        assert_eq!(ops.counters.drain_rejected.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn readiness_fences_old_evidence_and_histogram_has_fixed_buckets() {
        let ops = IMapOps::new(IMapOperationalConfig::default());
        let epoch = ClusterEpoch::new(4);
        assert_eq!(
            ops.evidence(None).readiness.reason,
            IMapReadinessReason::TopologyUnavailable
        );
        assert_eq!(
            ops.evidence(Some((epoch, true))).readiness.reason,
            IMapReadinessReason::EpochStale
        );
        let mut observation = HealthObservation::unavailable(IMapReadinessReason::Ready, 4);
        observation.rows.push(PartitionHealth {
            id: 0,
            state: PartitionState::Healthy,
            local_primary: true,
            local_backup: false,
            certified_version: 1,
        });
        ops.publish(observation.clone());
        ops.publish(observation);
        let evidence = ops.evidence(Some((epoch, true)));
        assert!(evidence.readiness.ready && evidence.complete);
        assert_eq!(evidence.sequence, 1);
        assert_eq!(
            ops.evidence(Some((epoch, false))).readiness.reason,
            IMapReadinessReason::EpochStale
        );
        assert_eq!(
            ops.evidence(Some((ClusterEpoch::new(5), true)))
                .readiness
                .reason,
            IMapReadinessReason::EpochStale
        );
        ops.record_ack(Duration::from_micros(50), true);
        ops.record_ack(Duration::from_micros(190), true);
        ops.record_ack(Duration::from_millis(2), false);
        drop(ops.replica_ack());
        let metrics = ops.prometheus(&evidence);
        assert!(metrics.contains("hydracache_imap_partitions{state=\"healthy\"} 1\n"));
        assert!(metrics.contains("hydracache_imap_replica_ack_seconds_bucket{le=\"0.000100\"} 1\n"));
        assert!(metrics.contains("hydracache_imap_replica_ack_seconds_bucket{le=\"0.000500\"} 2\n"));
        assert!(metrics.contains("hydracache_imap_replica_ack_seconds_count 2\n"));
        assert!(metrics.contains("hydracache_imap_replica_ack_failed_total 2\n"));
        assert!(!metrics.contains("partition_id="));
        ops.begin_drain();
        assert_eq!(
            ops.evidence(Some((epoch, true))).readiness.reason,
            IMapReadinessReason::Draining
        );
    }
}
