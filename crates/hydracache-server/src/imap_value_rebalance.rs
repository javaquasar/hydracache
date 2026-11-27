//! Owner-local bounded reconciliation of synchronous replica assignments.

use std::collections::{BTreeMap, BTreeSet};

use hydracache::{ClusterGeneration, ClusterNodeId};
use serde::{Deserialize, Serialize};

use super::*;
use crate::imap_value_repair::ReplicaRepairOutcome;

const MAX_TRANSFERS: usize = 4;
const MAX_PROBES: usize = 16;
const PASS_WAIT: Duration = Duration::from_secs(45);
const PROBE_WAIT: Duration = Duration::from_millis(600);
const WATCH_PROBES: usize = 8;

#[derive(Default)]
pub(crate) struct RebalanceHealth {
    pub failed: BTreeSet<ClusterNodeId>,
    pub suspect: BTreeSet<ClusterNodeId>,
}

#[derive(Default)]
pub(crate) struct BackupObservations {
    misses: BTreeMap<ClusterNodeId, (ClusterGeneration, u8)>,
    next: usize,
}

impl BackupObservations {
    fn record(&mut self, replica: &PartitionMember, ready: bool) {
        if ready {
            self.misses.remove(&replica.node);
            return;
        }
        let entry = self
            .misses
            .entry(replica.node.clone())
            .or_insert((replica.generation, 0));
        if entry.0 != replica.generation {
            *entry = (replica.generation, 0);
        }
        entry.1 = (entry.1 + 1).min(2);
    }
}

/// Progress from one bounded pass over the local owner's certified partitions.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ReplicaRebalanceReport {
    pub committed: Vec<ReplicaRepairOutcome>,
    pub deferred: Vec<PartitionId>,
}

pub(crate) struct RebalanceRequest<'a> {
    pub partitions: &'a [PartitionId],
    pub drained: &'a BTreeSet<ClusterNodeId>,
    pub health: &'a RebalanceHealth,
}

impl RebalanceRequest<'_> {
    fn blocked(&self, node: &ClusterNodeId) -> bool {
        self.drained.contains(node) || self.health.failed.contains(node)
    }
}

impl ValueReplica {
    pub(crate) async fn observe_backups<S: RaftLogStore>(
        &self,
        raft: &RaftMetadataRuntime<S>,
        transport: &dyn ReplicaTransport,
        samples: &mut BackupObservations,
        health: &mut RebalanceHealth,
    ) {
        let Some(snapshot) = raft.partition_snapshot() else {
            return;
        };
        if snapshot
            .primary_rebalance
            .as_ref()
            .is_some_and(|wave| wave.head().is_some())
        {
            *samples = BackupObservations::default();
            return;
        }
        let Ok(topology) =
            PartitionTopology::from_snapshot(snapshot, D2_PARTITION_COUNT, self.replication)
        else {
            return;
        };
        let snapshot = topology.snapshot();
        let peers: BTreeMap<_, _> = snapshot
            .value_prefixes
            .keys()
            .filter_map(|partition| topology.placement(*partition).ok())
            .filter(|placement| placement.primary == self.local)
            .flat_map(|placement| placement.backups)
            .filter(|backup| admitted(snapshot, backup))
            .map(|backup| (backup.node.clone(), backup))
            .collect();
        self.observe_replica_endpoints(snapshot.epoch, transport, samples, health, peers)
            .await;
    }

    pub(super) async fn observe_replica_endpoints(
        &self,
        epoch: ClusterEpoch,
        transport: &dyn ReplicaTransport,
        samples: &mut BackupObservations,
        health: &mut RebalanceHealth,
        peers: BTreeMap<ClusterNodeId, PartitionMember>,
    ) {
        samples.misses.retain(|id, (generation, _)| {
            peers
                .get(id)
                .is_some_and(|peer| &peer.generation == generation)
        });
        let peers: Vec<_> = peers.into_values().collect();
        let count = peers.len();
        if count == 0 {
            return;
        }
        let start = samples.next % count;
        for offset in 0..count.min(WATCH_PROBES) {
            let peer = &peers[(start + offset) % count];
            if !health.failed.contains(&peer.node) {
                let probe = self.candidate_probe(peer.clone(), epoch);
                let ready = timeout_at(
                    Instant::now() + PROBE_WAIT,
                    transport.check_candidate(probe.clone()),
                )
                .await
                .is_ok_and(|reply| reply.is_ok_and(|reply| reply == probe));
                samples.record(peer, ready);
            }
        }
        samples.next = (start + count.min(WATCH_PROBES)) % count;
        health.failed.extend(
            samples
                .misses
                .iter()
                .filter(|(_, (_, misses))| *misses >= 2)
                .map(|(node, _)| node.clone()),
        );
    }

    pub(super) fn candidate_probe(
        &self,
        candidate: PartitionMember,
        epoch: ClusterEpoch,
    ) -> ReplicaReady {
        ReplicaReady {
            candidate,
            owner: self.local.clone(),
            epoch,
            scope: self.repair_scope(),
            sync_backups: self.replication.sync_backups,
            wire_generation: None,
        }
    }

    pub(crate) fn check_candidate(
        &self,
        snapshot: &RaftPartitionSnapshot,
        sender: &str,
        epoch: u64,
        request: ReplicaReady,
    ) -> CacheResult<ReplicaReady> {
        let eligible = match request.wire_generation {
            Some(version) => {
                self.operations.is_some() && version == crate::imap_ops::VALUE_WIRE_GENERATION
            }
            None => self
                .operations
                .as_ref()
                .is_none_or(|ops| !ops.is_draining()),
        };
        if !eligible
            || snapshot.applied_index != snapshot.commit_index
            || request.epoch != snapshot.epoch
            || epoch != request.epoch.value()
            || request.candidate != self.local
            || request.owner.node == self.local.node
            || request.owner.node.as_str() != sender
            || request.scope != self.repair_scope()
            || request.sync_backups != self.replication.sync_backups
            || !admitted(snapshot, &request.owner)
            || !admitted(snapshot, &self.local)
        {
            return Err(repair_error("candidate scope, owner or membership changed"));
        }
        Ok(request)
    }

    pub(crate) async fn rebalance_backups<S: RaftLogStore>(
        &self,
        raft: &RaftMetadataRuntime<S>,
        raft_transport: &Arc<dyn RaftMessageSink>,
        transport: &dyn ReplicaTransport,
        request: RebalanceRequest<'_>,
    ) -> CacheResult<ReplicaRebalanceReport> {
        let _pass = self
            .rebalance_pass
            .try_acquire()
            .map_err(|_| repair_error("rebalance already running"))?;
        let deadline = Instant::now() + PASS_WAIT;
        let snapshot = raft
            .partition_snapshot()
            .ok_or_else(|| repair_error("no applied placement"))?;
        if snapshot
            .primary_rebalance
            .as_ref()
            .is_some_and(|wave| wave.head().is_some())
        {
            return Err(repair_error("primary rebalance owns the transfer slot"));
        }
        let initial =
            PartitionTopology::from_snapshot(snapshot, D2_PARTITION_COUNT, self.replication)
                .map_err(|_| repair_error("no coherent placement for rebalance"))?;
        let todo: BTreeSet<_> = if request.partitions.is_empty() {
            initial.snapshot().value_prefixes.keys().copied().collect()
        } else {
            request.partitions.iter().copied().collect()
        };
        let mut pending: Vec<_> = todo
            .into_iter()
            .filter(|partition| {
                let snapshot = initial.snapshot();
                snapshot.value_prefixes.contains_key(partition)
                    && initial.placement(*partition).is_ok_and(|placement| {
                        placement.primary == self.local
                            && victim(snapshot, &placement, &request).is_some()
                    })
            })
            .collect();
        if let Some(capture) = self.journal.captured_transfer()? {
            if let Some(index) = pending
                .iter()
                .position(|partition| *partition == capture.manifest.plan.partition)
            {
                pending[..=index].rotate_right(1);
            }
        }

        let mut report = ReplicaRebalanceReport::default();
        let mut probes = MAX_PROBES;
        let mut attempts = 0;
        let mut pending = pending.into_iter();
        while let Some(partition) = pending.next() {
            if probes == 0 || attempts == MAX_TRANSFERS || Instant::now() >= deadline {
                report.deferred.push(partition);
                report.deferred.extend(pending);
                break;
            }
            let Some(snapshot) = raft.partition_snapshot() else {
                report.deferred.push(partition);
                continue;
            };
            let Ok(topology) =
                PartitionTopology::from_snapshot(snapshot, D2_PARTITION_COUNT, self.replication)
            else {
                report.deferred.push(partition);
                continue;
            };
            let Ok(placement) = topology.placement(partition) else {
                report.deferred.push(partition);
                continue;
            };
            if placement.primary != self.local {
                report.deferred.push(partition);
                continue;
            }
            let Some(replaced) = victim(topology.snapshot(), &placement, &request) else {
                continue;
            };
            let mut target = None;
            for candidate in candidates(&topology, &placement, &replaced, &request) {
                if probes == 0 || Instant::now() >= deadline {
                    break;
                }
                let probe = self.candidate_probe(candidate, placement.epoch);
                if candidate_ready(transport, &probe, &mut probes, deadline).await {
                    target = Some(probe.candidate);
                    break;
                }
            }
            let Some(target) = target else {
                report.deferred.push(partition);
                continue;
            };
            attempts += 1;
            let outcome = timeout_at(
                deadline,
                self.repair_backup(raft, raft_transport, transport, partition, replaced, target),
            )
            .await;
            match outcome {
                Ok(Ok(repair)) => report.committed.push(repair),
                _ => {
                    report.deferred.push(partition);
                    // Resume a started capture before opening another one on
                    // this owner. A later pass refreshes reachability hints.
                    if self
                        .journal
                        .captured_transfer()?
                        .is_some_and(|capture| capture.manifest.plan.partition == partition)
                    {
                        report.deferred.extend(pending);
                        break;
                    }
                }
            }
        }
        Ok(report)
    }
}

pub(super) async fn candidate_ready(
    transport: &dyn ReplicaTransport,
    probe: &ReplicaReady,
    probes: &mut usize,
    deadline: Instant,
) -> bool {
    for attempt in 0..2 {
        if *probes == 0 || Instant::now() >= deadline {
            break;
        }
        if attempt > 0 {
            sleep(Duration::from_millis(30)).await;
        }
        *probes -= 1;
        let limit = deadline.min(Instant::now() + PROBE_WAIT);
        if timeout_at(limit, transport.check_candidate(probe.clone()))
            .await
            .is_ok_and(|reply| reply.is_ok_and(|reply| reply == *probe))
        {
            return true;
        }
    }
    false
}

fn victim(
    snapshot: &RaftPartitionSnapshot,
    placement: &PartitionPlacement,
    request: &RebalanceRequest<'_>,
) -> Option<PartitionMember> {
    placement
        .backups
        .iter()
        .find(|backup| request.blocked(&backup.node) || !admitted(snapshot, backup))
        .cloned()
}

fn candidates(
    topology: &PartitionTopology,
    placement: &PartitionPlacement,
    replaced: &PartitionMember,
    request: &RebalanceRequest<'_>,
) -> Vec<PartitionMember> {
    let snapshot = topology.snapshot();
    let mut load = BTreeMap::new();
    for partition in snapshot.value_prefixes.keys() {
        if let Ok(effective) = topology.placement(*partition) {
            for backup in effective.backups {
                *load.entry(backup.node).or_insert(0usize) += 1;
            }
        }
    }
    let mut ranked: Vec<_> = snapshot
        .members
        .iter()
        .filter_map(|member| {
            let candidate = PartitionMember {
                node: member.node_id.clone(),
                generation: member.generation,
            };
            (candidate != *replaced
                && candidate.node != placement.primary.node
                && !request.blocked(&candidate.node)
                && !request.health.suspect.contains(&candidate.node)
                && !placement
                    .backups
                    .iter()
                    .any(|backup| backup != replaced && backup.node == candidate.node))
            .then_some(candidate)
        })
        .collect();
    ranked.sort_by(|left, right| {
        load.get(&left.node)
            .copied()
            .unwrap_or_default()
            .cmp(&load.get(&right.node).copied().unwrap_or_default())
            .then_with(|| left.node.cmp(&right.node))
    });
    ranked
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydracache::{ClusterMember, ClusterRole};
    use std::num::NonZeroU64;

    fn member(name: &str, generation: u64) -> PartitionMember {
        PartitionMember {
            node: ClusterNodeId::new(name),
            generation: ClusterGeneration::new(generation),
        }
    }

    fn fixture() -> (RaftPartitionSnapshot, ReplicationConfig) {
        let epoch = ClusterEpoch::new(9);
        let owner = member("owner", 1);
        let slot = PartitionId::new(3);
        let other = PartitionId::new(4);
        let promotion = |partition, backups| PartitionPromotion {
            partition,
            old_epoch: ClusterEpoch::new(8),
            new_epoch: epoch,
            old_owner: member("retired", 1),
            new_owner: owner.clone(),
            backups,
            required: None,
            wave: None,
        };
        let members = [
            ("owner", 1),
            ("retired", 2),
            ("assigned", 1),
            ("failed", 1),
            ("suspect", 1),
            ("loaded", 1),
            ("spare", 1),
        ]
        .into_iter()
        .map(|(name, generation)| ClusterMember {
            node_id: ClusterNodeId::new(name),
            generation: ClusterGeneration::new(generation),
            role: ClusterRole::Member,
            epoch,
            endpoints: Default::default(),
            metadata: Default::default(),
        })
        .collect();
        let snapshot = RaftPartitionSnapshot {
            term: 1,
            epoch,
            commit_index: 9,
            applied_index: 9,
            members,
            value_prefixes: [slot, other]
                .into_iter()
                .map(|partition| {
                    (
                        partition,
                        PartitionValuePrefix {
                            partition,
                            epoch,
                            version: 1,
                            previous: [0; 32],
                            digest: [1; 32],
                        },
                    )
                })
                .collect(),
            value_promotions: BTreeMap::from([
                (
                    slot,
                    promotion(slot, vec![member("retired", 1), member("assigned", 1)]),
                ),
                (
                    other,
                    promotion(other, vec![member("loaded", 1), member("assigned", 1)]),
                ),
            ]),
            value_repairs: BTreeMap::new(),
            primary_rebalance: None,
        };
        let profile = ReplicationConfig {
            replication_factor: 3,
            sync_backups: 2,
            ..ReplicationConfig::local_first()
        };
        (snapshot, profile)
    }

    #[test]
    fn ranking_excludes_invalid_candidates_and_prefers_fewer_certified_backups() {
        let (snapshot, profile) = fixture();
        let topology =
            PartitionTopology::from_snapshot(snapshot, D2_PARTITION_COUNT, profile).unwrap();
        let placement = topology.placement(PartitionId::new(3)).unwrap();
        let health = RebalanceHealth {
            failed: BTreeSet::from([ClusterNodeId::new("failed")]),
            suspect: BTreeSet::from([ClusterNodeId::new("suspect")]),
        };
        let request = RebalanceRequest {
            partitions: &[],
            drained: &BTreeSet::new(),
            health: &health,
        };
        let replaced = victim(topology.snapshot(), &placement, &request).unwrap();
        assert_eq!(replaced, member("retired", 1));
        assert_eq!(
            candidates(&topology, &placement, &replaced, &request),
            vec![
                member("spare", 1),
                member("loaded", 1),
                member("retired", 2)
            ]
        );
        let healthy = topology.placement(PartitionId::new(4)).unwrap();
        assert!(victim(topology.snapshot(), &healthy, &request).is_none());
        let request = RebalanceRequest {
            drained: &BTreeSet::from([ClusterNodeId::new("retired"), ClusterNodeId::new("loaded")]),
            ..request
        };
        assert_eq!(
            victim(topology.snapshot(), &healthy, &request),
            Some(member("loaded", 1))
        );
        assert_eq!(
            candidates(&topology, &placement, &replaced, &request),
            vec![member("spare", 1)]
        );
    }

    #[test]
    fn candidate_reply_requires_exact_scope_generation_epoch_and_profile() {
        let (mut snapshot, profile) = fixture();
        let replica = ValueReplica::with_journal(
            ValueJournal::temporary(),
            member("spare", 1),
            ReplicaReadScope::new(
                "test".into(),
                Namespace::new("test").unwrap(),
                NonZeroU64::MIN,
                |_| None,
            ),
            profile,
        )
        .unwrap();
        let probe = ReplicaReady {
            candidate: member("spare", 1),
            owner: member("owner", 1),
            epoch: snapshot.epoch,
            scope: replica.repair_scope(),
            sync_backups: profile.sync_backups,
            wire_generation: None,
        };
        let epoch = snapshot.epoch.value();
        assert_eq!(
            replica
                .check_candidate(&snapshot, "owner", epoch, probe.clone())
                .unwrap(),
            probe
        );
        for invalid in [
            ReplicaReady {
                candidate: member("spare", 2),
                ..probe.clone()
            },
            ReplicaReady {
                owner: member("owner", 2),
                ..probe.clone()
            },
            ReplicaReady {
                epoch: ClusterEpoch::new(epoch - 1),
                ..probe.clone()
            },
            ReplicaReady {
                scope: [0; 32],
                ..probe.clone()
            },
            ReplicaReady {
                sync_backups: 1,
                ..probe.clone()
            },
        ] {
            assert!(replica
                .check_candidate(&snapshot, "owner", epoch, invalid)
                .is_err());
        }
        assert!(replica
            .check_candidate(&snapshot, "another", epoch, probe.clone())
            .is_err());
        snapshot.applied_index -= 1;
        assert!(replica
            .check_candidate(&snapshot, "owner", epoch, probe)
            .is_err());
    }

    #[test]
    fn missed_health_checks_accumulate_only_for_the_same_incarnation() {
        let mut checks = BackupObservations::default();
        let old = member("backup", 1);
        let restarted = member("backup", 2);
        checks.record(&old, false);
        assert_eq!(checks.misses[&old.node], (old.generation, 1));
        checks.record(&old, false);
        checks.record(&old, false);
        assert_eq!(checks.misses[&old.node], (old.generation, 2));
        checks.record(&restarted, false);
        assert_eq!(checks.misses[&old.node], (restarted.generation, 1));
        checks.record(&restarted, true);
        assert!(checks.misses.is_empty());
    }
}
