//! Scope-bound primary placement policy for a planned drain and RF2 recovery.

use std::collections::{BTreeMap, BTreeSet};

use hydracache::{ClusterNodeId, PrimaryRebalanceWave};

use super::primary::PrimaryWaveRequest;
use super::rebalance::candidate_ready;
use super::*;

const PRIMARY_PROBES: usize = 16;
const SELECTION_WAIT: Duration = Duration::from_secs(6);

impl ValueReplica {
    pub(crate) async fn plan_primary_drain<S: RaftLogStore>(
        &self,
        raft: &RaftMetadataRuntime<S>,
        sink: &Arc<dyn RaftMessageSink>,
        transport: &dyn ReplicaTransport,
        request: RebalanceRequest<'_>,
    ) -> CacheResult<Option<PrimaryRebalanceWave>> {
        let snapshot = raft
            .partition_snapshot()
            .ok_or_else(|| repair_error("no primary placement view"))?;
        if active_wave(&snapshot) {
            return Err(repair_error("another primary wave is active"));
        }
        let topology =
            PartitionTopology::from_snapshot(snapshot, D2_PARTITION_COUNT, self.replication)
                .map_err(|_| repair_error("primary placement view is incoherent"))?;
        let snapshot = topology.snapshot();
        let placements = certified_placements(&topology);
        let mut counts = primary_counts(placements.values());
        let relevant: BTreeSet<_> = request.partitions.iter().copied().collect();
        let mut checked = BTreeMap::new();
        let mut targets = Vec::new();
        let mut probes = PRIMARY_PROBES;
        let deadline = Instant::now() + SELECTION_WAIT;

        for (partition, placement) in &placements {
            if targets.len() == PrimaryRebalanceWave::MAX_TASKS || Instant::now() >= deadline {
                break;
            }
            if !request.drained.contains(&placement.primary.node)
                || (!relevant.is_empty() && !relevant.contains(partition))
                || request.health.failed.contains(&placement.primary.node)
            {
                continue;
            }
            for candidate in ranked_primaries(snapshot, placement, &counts, &request) {
                let ready = match checked.get(&candidate.node) {
                    // This attachment already proves the local scope and profile.
                    None if candidate == self.local => true,
                    None if candidate.node == self.local.node => false,
                    Some(ready) => *ready,
                    None if probes == 0 => false,
                    None => {
                        let probe = self.candidate_probe(candidate.clone(), snapshot.epoch);
                        let ready = candidate_ready(transport, &probe, &mut probes, deadline).await;
                        checked.insert(candidate.node.clone(), ready);
                        ready
                    }
                };
                if ready {
                    *counts
                        .get_mut(&placement.primary.node)
                        .expect("certified owner counted") -= 1;
                    *counts.entry(candidate.node.clone()).or_default() += 1;
                    targets.push((*partition, candidate));
                    break;
                }
            }
        }
        if targets.is_empty() {
            return Ok(None);
        }
        self.submit_primary_wave(
            raft,
            sink,
            PrimaryWaveRequest {
                targets: &targets,
                epoch: Some(snapshot.epoch),
                recovery: false,
            },
        )
        .await
        .map(Some)
    }

    pub(crate) fn supports_primary_failover(&self) -> bool {
        self.replication.sync_backups == 1
    }

    /// The sole synchronous backup can initiate RF2 recovery only after it
    /// observes owner failure and proves the exact certified local prefix.
    pub(crate) async fn recover_failed_primaries<S: RaftLogStore>(
        &self,
        raft: &RaftMetadataRuntime<S>,
        sink: &Arc<dyn RaftMessageSink>,
        transport: &dyn ReplicaTransport,
        samples: &mut BackupObservations,
        mut health: RebalanceHealth,
    ) -> CacheResult<()> {
        let Some(snapshot) = raft.partition_snapshot() else {
            return Ok(());
        };
        if !self.supports_primary_failover()
            || active_wave(&snapshot)
            || !self.roster_compatible(&snapshot)
            || self
                .operations
                .as_ref()
                .is_some_and(|ops| ops.is_draining())
        {
            *samples = BackupObservations::default();
            return Ok(());
        }
        let Ok(topology) =
            PartitionTopology::from_snapshot(snapshot, D2_PARTITION_COUNT, self.replication)
        else {
            return Ok(());
        };
        let snapshot = topology.snapshot();
        if !admitted(snapshot, &self.local) {
            return Ok(());
        }
        let proposals: Vec<_> = snapshot
            .value_prefixes
            .iter()
            .filter_map(|(partition, required)| {
                let placement = topology
                    .placement(*partition)
                    .ok()
                    .or_else(|| self.primary_placement(snapshot, *partition, true))?;
                (placement.primary.node != self.local.node
                    && placement.backups == [self.local.clone()])
                .then_some((placement, *required))
            })
            .collect();
        let owners = proposals
            .iter()
            .map(|(placement, _)| &placement.primary)
            .filter(|owner| admitted(snapshot, owner))
            .map(|owner| (owner.node.clone(), owner.clone()))
            .collect();
        self.observe_replica_endpoints(snapshot.epoch, transport, samples, &mut health, owners)
            .await;
        let mut targets = Vec::new();
        for (placement, required) in proposals {
            if targets.len() == PrimaryRebalanceWave::MAX_TASKS {
                break;
            }
            if admitted(snapshot, &placement.primary)
                && !health.failed.contains(&placement.primary.node)
            {
                continue;
            }
            let Some(lane) = self.lane(placement.partition) else {
                continue;
            };
            let Ok(_guard) = lane.storage.try_lock() else {
                continue;
            };
            if self
                .journal
                .load(placement.partition)?
                .covers(Some(required))
            {
                targets.push((placement.partition, self.local.clone()));
            }
        }
        if !targets.is_empty() {
            self.submit_primary_wave(
                raft,
                sink,
                PrimaryWaveRequest {
                    targets: &targets,
                    epoch: Some(snapshot.epoch),
                    recovery: true,
                },
            )
            .await?;
        }
        Ok(())
    }
}

fn active_wave(snapshot: &RaftPartitionSnapshot) -> bool {
    snapshot
        .primary_rebalance
        .as_ref()
        .is_some_and(|wave| wave.head().is_some())
}

fn certified_placements(topology: &PartitionTopology) -> BTreeMap<PartitionId, PartitionPlacement> {
    topology
        .snapshot()
        .value_prefixes
        .keys()
        .filter_map(|partition| {
            topology
                .placement(*partition)
                .ok()
                .map(|placement| (*partition, placement))
        })
        .collect()
}

fn primary_counts<'a>(
    placements: impl Iterator<Item = &'a PartitionPlacement>,
) -> BTreeMap<ClusterNodeId, usize> {
    let mut counts = BTreeMap::new();
    for placement in placements {
        *counts.entry(placement.primary.node.clone()).or_default() += 1;
    }
    counts
}

fn ranked_primaries(
    snapshot: &RaftPartitionSnapshot,
    placement: &PartitionPlacement,
    counts: &BTreeMap<ClusterNodeId, usize>,
    request: &RebalanceRequest<'_>,
) -> Vec<PartitionMember> {
    let mut candidates: Vec<_> = snapshot
        .members
        .iter()
        .filter(|member| {
            member.node_id != placement.primary.node
                && !request.drained.contains(&member.node_id)
                && !request.health.failed.contains(&member.node_id)
                && !request.health.suspect.contains(&member.node_id)
        })
        .map(|member| PartitionMember {
            node: member.node_id.clone(),
            generation: member.generation,
        })
        .collect();
    candidates.sort_by(|left, right| {
        let rank = |candidate: &PartitionMember| {
            (
                counts.get(&candidate.node).copied().unwrap_or_default(),
                !placement.backups.contains(candidate),
            )
        };
        rank(left)
            .cmp(&rank(right))
            .then_with(|| left.node.cmp(&right.node))
    });
    candidates
}

#[cfg(test)]
mod tests {
    use hydracache::{ClusterGeneration, ClusterMember, ClusterRole};

    use super::*;

    #[test]
    fn drain_ranking_accounts_for_primary_load_health_and_exact_backup_generation() {
        let epoch = ClusterEpoch::new(9);
        let member = |name, generation| PartitionMember {
            node: ClusterNodeId::new(name),
            generation: ClusterGeneration::new(generation),
        };
        let snapshot = RaftPartitionSnapshot {
            term: 1,
            epoch,
            commit_index: 9,
            applied_index: 9,
            members: [
                ("source", 1),
                ("drained", 1),
                ("failed", 1),
                ("suspect", 1),
                ("live-backup", 1),
                ("a-restarted", 2),
                ("b-spare", 1),
            ]
            .into_iter()
            .map(|(name, generation)| {
                let identity = member(name, generation);
                ClusterMember {
                    node_id: identity.node,
                    generation: identity.generation,
                    role: ClusterRole::Member,
                    epoch,
                    endpoints: Default::default(),
                    metadata: Default::default(),
                }
            })
            .collect(),
            value_prefixes: BTreeMap::new(),
            value_promotions: BTreeMap::new(),
            value_repairs: BTreeMap::new(),
            primary_rebalance: None,
        };
        let current = PartitionPlacement {
            partition: PartitionId::new(0),
            epoch,
            primary: member("source", 1),
            backups: vec![member("live-backup", 1), member("a-restarted", 1)],
        };
        let request = RebalanceRequest {
            partitions: &[],
            drained: &BTreeSet::from([member("drained", 1).node]),
            health: &RebalanceHealth {
                failed: BTreeSet::from([member("failed", 1).node]),
                suspect: BTreeSet::from([member("suspect", 1).node]),
            },
        };
        let mut counts = primary_counts([&current, &current].into_iter());
        assert_eq!(counts[&current.primary.node], 2);
        assert_eq!(
            ranked_primaries(&snapshot, &current, &counts, &request),
            [
                member("live-backup", 1),
                member("a-restarted", 2),
                member("b-spare", 1)
            ],
        );
        counts.insert(member("live-backup", 1).node, 1);
        assert_eq!(
            ranked_primaries(&snapshot, &current, &counts, &request),
            [
                member("a-restarted", 2),
                member("b-spare", 1),
                member("live-backup", 1)
            ],
        );
    }
}
