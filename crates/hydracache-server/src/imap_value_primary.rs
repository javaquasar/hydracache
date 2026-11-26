//! Primary movements sequenced by the head of a committed Raft wave.

use hydracache::{PrimaryRebalanceCommand, PrimaryRebalanceWave, PrimaryTransferTask};

use super::repair_owner::acquire_writer;
use super::*;
use crate::imap_value_repair::{BackupTransfer, FINAL_CUT_WAIT};

enum StartDecision {
    Existing(PrimaryRebalanceWave),
    Submit {
        id: ClusterEpoch,
        tasks: Vec<PrimaryTransferTask>,
    },
}

pub(super) struct PrimaryWaveRequest<'a> {
    pub targets: &'a [(PartitionId, PartitionMember)],
    pub epoch: Option<ClusterEpoch>,
    pub recovery: bool,
}

impl ValueReplica {
    pub(crate) async fn start_primary_wave<S: RaftLogStore>(
        &self,
        raft: &RaftMetadataRuntime<S>,
        sink: &Arc<dyn RaftMessageSink>,
        targets: &[(PartitionId, PartitionMember)],
    ) -> CacheResult<PrimaryRebalanceWave> {
        self.submit_primary_wave(
            raft,
            sink,
            PrimaryWaveRequest {
                targets,
                epoch: None,
                recovery: false,
            },
        )
        .await
    }

    pub(super) async fn submit_primary_wave<S: RaftLogStore>(
        &self,
        raft: &RaftMetadataRuntime<S>,
        sink: &Arc<dyn RaftMessageSink>,
        request: PrimaryWaveRequest<'_>,
    ) -> CacheResult<PrimaryRebalanceWave> {
        let targets = request.targets;
        if targets.is_empty()
            || targets.len() > PrimaryRebalanceWave::MAX_TASKS
            || !(1..=2).contains(&self.replication.sync_backups)
        {
            return Err(repair_error("primary wave size is outside its bounds"));
        }
        let scope = self.repair_scope();
        let deadline = Instant::now() + WRITE_WAIT;
        let proof = request_read_index(raft, sink, deadline)
            .await
            .map_err(|_| repair_error("no authority to start the primary wave"))?;
        let operation = proof
            .with_snapshot(|snapshot, _| {
                if request.epoch.is_some_and(|epoch| epoch != snapshot.epoch) {
                    return Err(repair_error("primary selection epoch changed"));
                }
                if let Some(wave) = snapshot.primary_rebalance.as_ref() {
                    let same_request = wave.scope == scope
                        && !wave.cancelled
                        && wave.tasks.len() == targets.len()
                        && wave
                            .tasks
                            .iter()
                            .zip(targets)
                            .all(|(task, (partition, target))| {
                                task.partition == *partition
                                    && task.target == *target
                                    && task.recovery == request.recovery
                            });
                    if same_request
                        && (wave.head().is_some()
                            || wave.tasks.iter().all(|task| {
                                self.placement(&snapshot, task.partition)
                                    .is_some_and(|placement| placement.primary == task.target)
                            }))
                    {
                        return Ok(StartDecision::Existing(wave.clone()));
                    }
                    if wave.head().is_some() {
                        return Err(repair_error("another primary wave is active"));
                    }
                }
                let mut seen = std::collections::BTreeSet::new();
                let tasks = targets
                    .iter()
                    .map(|(partition, target)| {
                        let placement = self
                            .primary_placement(&snapshot, *partition, request.recovery)
                            .ok_or_else(|| repair_error("primary wave has no applied placement"))?;
                        if !seen.insert(*partition)
                            || !snapshot.value_prefixes.contains_key(partition)
                            || placement.primary.node == target.node
                            || !admitted(&snapshot, target)
                            || (request.recovery
                                && (self.replication.sync_backups != 1
                                    || !placement.backups.contains(target)))
                        {
                            return Err(repair_error(
                                "primary wave target is invalid or duplicated",
                            ));
                        }
                        Ok(PrimaryTransferTask {
                            partition: *partition,
                            source: placement.primary,
                            target: target.clone(),
                            recovery: request.recovery,
                        })
                    })
                    .collect::<CacheResult<Vec<_>>>()?;
                Ok(StartDecision::Submit {
                    id: next_epoch(snapshot.epoch)?,
                    tasks,
                })
            })
            .map_err(|_| repair_error("primary wave proof was fenced"))??;
        let (id, tasks) = match operation {
            StartDecision::Submit { id, tasks } => (id, tasks),
            StartDecision::Existing(existing) => return Ok(existing),
        };
        raft.propose_primary_rebalance(PrimaryRebalanceCommand::Start {
            id,
            scope,
            sync_backups: self.replication.sync_backups,
            tasks,
        })?;
        wait_for_wave(raft, sink, deadline, |wave| wave.id == id).await
    }

    pub(crate) async fn cancel_primary_wave<S: RaftLogStore>(
        &self,
        raft: &RaftMetadataRuntime<S>,
        sink: &Arc<dyn RaftMessageSink>,
        id: ClusterEpoch,
    ) -> CacheResult<PrimaryRebalanceWave> {
        let deadline = Instant::now() + WRITE_WAIT;
        let proof = request_read_index(raft, sink, deadline)
            .await
            .map_err(|_| repair_error("no authority to cancel the primary wave"))?;
        let (wave, epoch) = proof
            .with_snapshot(|snapshot, _| {
                snapshot
                    .primary_rebalance
                    .filter(|wave| wave.id == id && wave.scope == self.repair_scope())
                    .map(|wave| (wave, snapshot.epoch))
                    .ok_or_else(|| repair_error("primary wave is unknown to this scope"))
            })
            .map_err(|_| repair_error("primary wave proof was fenced"))??;
        if wave.head().is_none() {
            return Ok(wave);
        }
        raft.propose_primary_rebalance(PrimaryRebalanceCommand::Cancel {
            id,
            at_epoch: epoch,
        })?;
        wait_for_wave(raft, sink, deadline, |wave| {
            wave.id == id && wave.head().is_none()
        })
        .await
    }

    pub(crate) async fn drive_primary_wave<S: RaftLogStore>(
        &self,
        raft: &RaftMetadataRuntime<S>,
        sink: &Arc<dyn RaftMessageSink>,
        transport: &dyn ReplicaTransport,
    ) -> CacheResult<()> {
        let Some(snapshot) = raft.partition_snapshot() else {
            return Ok(());
        };
        if snapshot.applied_index != snapshot.commit_index {
            return Ok(());
        }
        let Some(wave) = snapshot.primary_rebalance.as_ref().filter(|wave| {
            wave.scope == self.repair_scope() && wave.sync_backups == self.replication.sync_backups
        }) else {
            return Ok(());
        };
        let Some(task) = wave.head().filter(|task| {
            if task.recovery {
                task.target == self.local
            } else {
                task.source == self.local
            }
        }) else {
            return Ok(());
        };
        let Some(placement) = self.primary_placement(&snapshot, task.partition, task.recovery)
        else {
            return Ok(());
        };
        if placement.primary != task.source || !admitted(&snapshot, &task.target) {
            return Ok(());
        }
        if !placement.backups.contains(&task.target) {
            if task.recovery {
                return Err(repair_error(
                    "recovery requires an existing synchronous backup",
                ));
            }
            let probe = self.candidate_probe(task.target.clone(), snapshot.epoch);
            if timeout_at(
                Instant::now() + Duration::from_millis(600),
                transport.check_candidate(probe.clone()),
            )
            .await
            .ok()
            .and_then(Result::ok)
                != Some(probe)
            {
                return Err(repair_error("primary destination is unavailable"));
            }
            let replaced =
                placement.backups.last().cloned().ok_or_else(|| {
                    repair_error("primary movement requires a synchronous backup")
                })?;
            self.transfer_backup(
                raft,
                sink,
                transport,
                BackupTransfer {
                    partition: task.partition,
                    replaced,
                    target: task.target.clone(),
                    wave: Some(wave.id),
                },
            )
            .await?;
        }
        self.handoff_primary(raft, sink, transport, wave.id, task)
            .await
    }

    pub(super) fn primary_placement(
        &self,
        snapshot: &RaftPartitionSnapshot,
        partition: PartitionId,
        recovery: bool,
    ) -> Option<PartitionPlacement> {
        if recovery && snapshot.applied_index == snapshot.commit_index {
            if let Some((primary, backups)) = snapshot.value_assignment(partition) {
                if backups.len() == self.replication.sync_backups {
                    return Some(PartitionPlacement {
                        partition,
                        epoch: snapshot.epoch,
                        primary,
                        backups,
                    });
                }
            }
        }
        self.placement(snapshot, partition)
    }

    async fn handoff_primary<S: RaftLogStore>(
        &self,
        raft: &RaftMetadataRuntime<S>,
        sink: &Arc<dyn RaftMessageSink>,
        transport: &dyn ReplicaTransport,
        wave: ClusterEpoch,
        task: &PrimaryTransferTask,
    ) -> CacheResult<()> {
        let lane = self
            .lane(task.partition)
            .ok_or_else(|| repair_error("unknown primary partition"))?;
        let _writer = acquire_writer(lane, Instant::now() + WRITE_WAIT).await?;
        let deadline = Instant::now() + FINAL_CUT_WAIT;
        let proof = request_read_index(raft, sink, deadline)
            .await
            .map_err(|_| repair_error("source has no authority for primary handoff"))?;
        let plan = {
            let _storage = lane
                .storage
                .try_lock()
                .map_err(|_| repair_error("primary lane busy"))?;
            proof
                .with_snapshot(|snapshot, _| {
                    if !current_task(&snapshot, wave, task) {
                        return Err(repair_error("primary wave head changed"));
                    }
                    let mut placement = self
                        .primary_placement(&snapshot, task.partition, task.recovery)
                        .filter(|placement| placement.primary == task.source)
                        .ok_or_else(|| repair_error("primary ownership changed"))?;
                    let slot = placement
                        .backups
                        .iter_mut()
                        .find(|member| **member == task.target)
                        .ok_or_else(|| repair_error("primary destination has not been seeded"))?;
                    *slot = task.source.clone();
                    if !placement.backups.iter().all(|backup| {
                        admitted(&snapshot, backup) || (task.recovery && backup == &task.source)
                    }) || !admitted(&snapshot, &task.target)
                    {
                        return Err(repair_error("primary handoff generation changed"));
                    }
                    let required = snapshot.value_prefixes.get(&task.partition).copied();
                    if required.is_none() || !self.journal.load(task.partition)?.covers(required) {
                        return Err(repair_error(
                            "handoff executor does not hold the final certified prefix",
                        ));
                    }
                    Ok(PartitionPromotion {
                        partition: task.partition,
                        old_epoch: snapshot.epoch,
                        new_epoch: next_epoch(snapshot.epoch)?,
                        old_owner: task.source.clone(),
                        new_owner: task.target.clone(),
                        backups: placement.backups,
                        required,
                        wave: Some(wave),
                    })
                })
                .map_err(|_| repair_error("primary handoff proof was fenced"))??
        };
        timeout_at(deadline, async {
            let probe = ReplicaPrefix {
                target: task.target.clone(),
                epoch: plan.old_epoch,
                partition: task.partition,
                required: plan.required,
                wave: Some(wave),
            };
            if !task.recovery && transport.probe(probe.clone()).await? != probe {
                return Err(repair_error(
                    "primary destination did not prove the final prefix",
                ));
            }
            if raft.with_applied_partition_snapshot(|snapshot| {
                snapshot.epoch == plan.old_epoch
                    && current_task(&snapshot, wave, task)
                    && snapshot.value_prefixes.get(&task.partition).copied() == plan.required
            }) != Some(true)
            {
                return Err(repair_error(
                    "primary handoff changed after the prefix proof",
                ));
            }
            raft.propose_value_promotion(plan.clone())?;
            let _ = send_raft_messages_with_diagnostics(sink, raft.take_outbound_messages(), None)
                .await;
            loop {
                let snapshot = raft
                    .partition_snapshot()
                    .ok_or_else(|| repair_error("primary handoff view lost"))?;
                if snapshot.value_promotions.get(&task.partition) == Some(&plan) {
                    return Ok(());
                }
                if snapshot.epoch != plan.old_epoch || !current_task(&snapshot, wave, task) {
                    return Err(repair_error("primary handoff was fenced"));
                }
                sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .map_err(|_| repair_error("final primary handoff timed out"))?
    }
}

fn current_task(
    snapshot: &RaftPartitionSnapshot,
    id: ClusterEpoch,
    expected: &PrimaryTransferTask,
) -> bool {
    snapshot
        .primary_rebalance
        .as_ref()
        .is_some_and(|wave| wave.id == id && wave.head() == Some(expected))
}

fn next_epoch(epoch: ClusterEpoch) -> CacheResult<ClusterEpoch> {
    epoch
        .value()
        .checked_add(1)
        .map(ClusterEpoch::new)
        .ok_or_else(|| repair_error("epoch exhausted"))
}

async fn wait_for_wave<S: RaftLogStore>(
    raft: &RaftMetadataRuntime<S>,
    sink: &Arc<dyn RaftMessageSink>,
    deadline: Instant,
    predicate: impl Fn(&PrimaryRebalanceWave) -> bool,
) -> CacheResult<PrimaryRebalanceWave> {
    timeout_at(deadline, async {
        let _ =
            send_raft_messages_with_diagnostics(sink, raft.take_outbound_messages(), None).await;
        loop {
            if let Some(wave) = raft
                .partition_snapshot()
                .and_then(|view| view.primary_rebalance)
                .filter(&predicate)
            {
                return wave;
            }
            sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .map_err(|_| repair_error("primary wave decision timed out"))
}
