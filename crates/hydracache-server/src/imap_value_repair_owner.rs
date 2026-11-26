use super::*;
use crate::imap_value_journal::CapturedTransfer;
use crate::imap_value_repair::{
    admits_repair, admits_transfer, bytes_digest, is_prefix_at_most, version, RepairManifest,
    RepairStep, ReplicaRepairOutcome, CHUNK_BYTES, FINAL_CUT_WAIT, FINAL_DELTA_BYTES,
    FINAL_DELTA_ROWS, REPAIR_WAIT,
};
use hydracache::PartitionReplicaRepair;

impl ValueReplica {
    pub(crate) fn receive_repair<S: RaftLogStore>(
        &self,
        raft: &RaftMetadataRuntime<S>,
        source: &str,
        epoch: u64,
        request: RepairRequest,
    ) -> CacheResult<RepairReceipt> {
        let plan = &request.manifest.plan;
        let partition = plan.partition;
        if self.local != plan.target
            || source != plan.owner.node.as_str()
            || epoch != plan.old_epoch.value()
            || request.manifest.scope != self.repair_scope()
        {
            return Err(repair_error(
                "transfer sender, target, epoch or scope does not match",
            ));
        }
        let _staging = self
            .repair_receiver
            .try_lock()
            .map_err(|_| repair_error("recipient busy"))?;
        let lane = self
            .lane(partition)
            .ok_or_else(|| repair_error("unknown partition"))?;
        let touches_partition = matches!(
            request.step,
            RepairStep::Install | RepairStep::Delta { .. } | RepairStep::Seal { .. }
        );
        let receive = |snapshot: RaftPartitionSnapshot| {
            if !admits_transfer(&snapshot, &request.manifest.plan) {
                return Err(repair_error("recipient has no matching applied cut"));
            }
            let latest = snapshot.value_prefixes.get(&partition).copied();
            match &request.step {
                RepairStep::Delta { mutation } => {
                    if !is_prefix_at_most(Some(mutation.certificate()), latest)
                        || snapshot.value_promotions[&partition].backups != mutation.backups
                        || !self.admits_mutation(mutation)
                    {
                        return Err(repair_error(
                            "delta is outside the applied certificate or placement",
                        ));
                    }
                }
                RepairStep::Seal { prefix } if *prefix != latest => {
                    return Err(repair_error(
                        "recipient has not applied the final Raft certificate",
                    ));
                }
                _ => {}
            }
            self.journal
                .receive_image(request, |key| self.scope.admits_storage_key(key, partition))
        };
        let received = if touches_partition {
            let _storage = lane
                .storage
                .try_lock()
                .map_err(|_| repair_error("recipient lane busy"))?;
            raft.with_applied_partition_snapshot(receive)
        } else {
            raft.partition_snapshot().map(receive)
        };
        received.ok_or_else(|| repair_error("recipient Raft view unavailable"))?
    }

    pub(crate) async fn repair_backup<S: RaftLogStore>(
        &self,
        raft: &RaftMetadataRuntime<S>,
        raft_transport: &Arc<dyn RaftMessageSink>,
        transport: &dyn ReplicaTransport,
        partition: PartitionId,
        replaced: PartitionMember,
        target: PartitionMember,
    ) -> CacheResult<ReplicaRepairOutcome> {
        let _transfer = self
            .repair_sender
            .try_acquire()
            .map_err(|_| repair_error("sender busy"))?;
        let lane = self
            .lane(partition)
            .ok_or_else(|| repair_error("unknown partition"))?;
        let deadline = Instant::now() + REPAIR_WAIT;
        let writer = acquire_writer(lane, deadline).await?;
        let proof = request_read_index(raft, raft_transport, deadline)
            .await
            .map_err(|_| repair_error("owner Raft authority unavailable"))?;
        let (plan, previously_applied) = proof
            .with_snapshot(|snapshot, _| {
                if let Some(prior) = snapshot.value_repairs.get(&partition).filter(|prior| {
                    prior.owner == self.local
                        && prior.replaced == replaced
                        && prior.target == target
                        && admitted(&snapshot, &self.local)
                        && admitted(&snapshot, &target)
                }) {
                    return Ok((prior.clone(), true));
                }
                let next_epoch = snapshot
                    .epoch
                    .value()
                    .checked_add(1)
                    .ok_or_else(|| repair_error("epoch exhausted"))?;
                let plan = PartitionReplicaRepair {
                    partition,
                    old_epoch: snapshot.epoch,
                    new_epoch: ClusterEpoch::new(next_epoch),
                    owner: self.local.clone(),
                    replaced,
                    target,
                    required: snapshot.value_prefixes.get(&partition).copied(),
                    snapshot_digest: [0; 32],
                };
                if !admits_repair(&snapshot, &plan) {
                    return Err(repair_error("owner has no matching replacement slot"));
                }
                Ok((plan, false))
            })
            .map_err(|_| repair_error("owner Raft authority changed"))??;
        if previously_applied {
            if let Some(capture) = self
                .journal
                .captured_transfer()?
                .filter(|capture| matches_slot(&capture.manifest.plan, &plan))
            {
                self.clear_capture(&capture.manifest)?;
            }
            return Ok(ReplicaRepairOutcome {
                plan,
                resumed_from: 0,
                chunks: 0,
                deltas: 0,
            });
        }
        let (manifest, image) = self.capture_image(plan)?;
        drop(writer);
        let transfer = LiveRepair {
            raft,
            raft_transport,
            transport,
            lane,
            journal: &self.journal,
            manifest: &manifest,
        };
        timeout_at(deadline, async {
            let resumed_from = send_step(transport, &manifest, RepairStep::Status)
                .await?
                .next;
            for (index, bytes) in image
                .chunks(CHUNK_BYTES)
                .enumerate()
                .skip(resumed_from as usize)
            {
                let index = index as u32;
                let ack = send_step(
                    transport,
                    &manifest,
                    RepairStep::Chunk {
                        index,
                        digest: bytes_digest(bytes),
                        bytes: bytes.to_vec(),
                    },
                )
                .await?;
                if ack.next != index + 1 || ack.installed {
                    return Err(repair_error("recipient acknowledged a different chunk"));
                }
            }
            let installed = send_step(transport, &manifest, RepairStep::Install).await?;
            if !installed.installed {
                return Err(repair_error("recipient did not confirm a durable image"));
            }
            let plan = transfer.finish(installed.applied, deadline).await?;
            // Once the new epoch is applied, old-epoch owner applies are
            // fenced and new writes cannot append to this capture.
            self.journal.finish_capture(&manifest)?;
            Ok(ReplicaRepairOutcome {
                deltas: version(plan.required) - version(manifest.plan.required),
                plan,
                resumed_from,
                chunks: manifest.chunks(),
            })
        })
        .await
        .map_err(|_| repair_error("transfer or activation timed out"))?
    }

    fn capture_image(
        &self,
        mut plan: PartitionReplicaRepair,
    ) -> CacheResult<(RepairManifest, Vec<u8>)> {
        if let Some(CapturedTransfer { manifest, .. }) = self.journal.captured_transfer()? {
            let prior = &manifest.plan;
            if prior.old_epoch == plan.old_epoch {
                if matches_slot(prior, &plan) {
                    if !is_prefix_at_most(prior.required, plan.required) {
                        return Err(repair_error("source snapshot prefix is no longer current"));
                    }
                    let image = self.journal.captured_image(&manifest)?;
                    return Ok((manifest, image));
                }
                if prior.partition != plan.partition {
                    return Err(repair_error(
                        "another partition has an unfinished source capture",
                    ));
                }
            }
            self.clear_capture(&manifest)?;
        }
        let lane = self
            .lane(plan.partition)
            .ok_or_else(|| repair_error("unknown partition"))?;
        let _storage = lane
            .storage
            .try_lock()
            .map_err(|_| repair_error("owner storage lane busy"))?;
        let image = self.journal.export_image(plan.partition, plan.required)?;
        plan.snapshot_digest = bytes_digest(&image);
        let manifest = RepairManifest {
            plan,
            scope: self.repair_scope(),
            image_len: image.len(),
        };
        self.journal.start_capture(&manifest, &image)?;
        Ok((manifest, image))
    }

    fn clear_capture(&self, manifest: &RepairManifest) -> CacheResult<()> {
        let lane = self
            .lane(manifest.plan.partition)
            .ok_or_else(|| repair_error("unknown partition"))?;
        let _storage = lane
            .storage
            .try_lock()
            .map_err(|_| repair_error("source storage lane busy"))?;
        self.journal.finish_capture(manifest)
    }

    fn repair_scope(&self) -> [u8; 32] {
        let (tenant, namespace, generation) = self.scope.identity();
        hash((1u16, tenant, namespace.as_str(), generation.get()))
    }
}

fn matches_slot(left: &PartitionReplicaRepair, right: &PartitionReplicaRepair) -> bool {
    left.partition == right.partition
        && left.owner == right.owner
        && left.replaced == right.replaced
        && left.target == right.target
}

async fn acquire_writer(
    lane: &Lane,
    deadline: Instant,
) -> CacheResult<tokio::sync::SemaphorePermit<'_>> {
    timeout_at(deadline, lane.writer.acquire())
        .await
        .map_err(|_| repair_error("owner writer lane timed out"))?
        .map_err(|_| repair_error("owner writer lane closed"))
}

struct LiveRepair<'a, S: RaftLogStore> {
    raft: &'a RaftMetadataRuntime<S>,
    raft_transport: &'a Arc<dyn RaftMessageSink>,
    transport: &'a dyn ReplicaTransport,
    lane: &'a Lane,
    journal: &'a ValueJournal,
    manifest: &'a RepairManifest,
}

impl<S: RaftLogStore> LiveRepair<'_, S> {
    async fn certified(&self, deadline: Instant) -> CacheResult<Option<PartitionValuePrefix>> {
        let proof = request_read_index(self.raft, self.raft_transport, deadline)
            .await
            .map_err(|_| repair_error("owner Raft authority unavailable during catch-up"))?;
        proof
            .with_snapshot(|snapshot, _| {
                if !admits_transfer(&snapshot, &self.manifest.plan) {
                    return Err(repair_error("replica placement changed during catch-up"));
                }
                Ok(snapshot
                    .value_prefixes
                    .get(&self.manifest.plan.partition)
                    .copied())
            })
            .map_err(|_| repair_error("catch-up ReadIndex was fenced"))?
    }

    async fn finish(
        &self,
        mut cursor: Option<PartitionValuePrefix>,
        deadline: Instant,
    ) -> CacheResult<PartitionReplicaRepair> {
        loop {
            let watermark = self.certified(deadline).await?;
            let (deltas, _) = self
                .journal
                .captured_deltas(self.manifest, cursor, watermark)?;
            self.send_deltas(deltas).await?;
            cursor = watermark;
            let writer = acquire_writer(self.lane, deadline).await?;
            let cut_deadline = deadline.min(Instant::now() + FINAL_CUT_WAIT);
            let final_prefix = self.certified(cut_deadline).await?;
            if version(final_prefix).saturating_sub(version(cursor)) > FINAL_DELTA_ROWS {
                drop(writer);
                let (gap, _) = self
                    .journal
                    .captured_deltas(self.manifest, cursor, final_prefix)?;
                self.send_deltas(gap).await?;
                cursor = final_prefix;
                continue;
            }
            let (tail, bytes) =
                self.journal
                    .captured_deltas(self.manifest, cursor, final_prefix)?;
            if bytes > FINAL_DELTA_BYTES {
                drop(writer);
                self.send_deltas(tail).await?;
                cursor = final_prefix;
                continue;
            }
            let result = timeout_at(cut_deadline, async {
                self.send_deltas(tail).await?;
                let confirmed = self
                    .send_progress(RepairStep::Seal {
                        prefix: final_prefix,
                    })
                    .await?;
                if !confirmed.installed || confirmed.applied != final_prefix {
                    return Err(repair_error("recipient did not seal the final prefix"));
                }
                self.activate(final_prefix).await
            })
            .await
            .map_err(|_| repair_error("final replica cut timed out"))?;
            drop(writer);
            return result;
        }
    }

    async fn send_deltas(&self, deltas: Vec<Mutation>) -> CacheResult<()> {
        for mutation in deltas {
            let expected = mutation.certificate();
            let ack = self
                .send_progress(RepairStep::Delta {
                    mutation: Box::new(mutation),
                })
                .await?;
            if !ack.installed || ack.applied != Some(expected) {
                return Err(repair_error("recipient acknowledged a different delta"));
            }
        }
        Ok(())
    }

    async fn send_progress(&self, step: RepairStep) -> CacheResult<RepairReceipt> {
        // The recipient can be one Raft append behind the source. A retry is
        // safe because both steps acknowledge the exact durable certificate.
        for _ in 0..3 {
            if let Ok(ack) = send_step(self.transport, self.manifest, step.clone()).await {
                return Ok(ack);
            }
            sleep(Duration::from_millis(20)).await;
        }
        send_step(self.transport, self.manifest, step).await
    }

    async fn activate(
        &self,
        final_prefix: Option<PartitionValuePrefix>,
    ) -> CacheResult<PartitionReplicaRepair> {
        let mut plan = self.manifest.plan.clone();
        plan.required = final_prefix;
        if self
            .raft
            .with_applied_partition_snapshot(|snapshot| admits_repair(&snapshot, &plan))
            != Some(true)
        {
            return Err(repair_error(
                "certified placement changed before activation",
            ));
        }
        self.raft.propose_value_repair(plan.clone())?;
        let _ = send_raft_messages_with_diagnostics(
            self.raft_transport,
            self.raft.take_outbound_messages(),
            None,
        )
        .await;
        loop {
            let snapshot = self
                .raft
                .partition_snapshot()
                .ok_or_else(|| repair_error("activation view unavailable"))?;
            if snapshot.value_repairs.get(&plan.partition) == Some(&plan) {
                return Ok(plan);
            }
            if snapshot.epoch != plan.old_epoch {
                return Err(repair_error(
                    "replacement did not commit at the expected cut",
                ));
            }
            sleep(Duration::from_millis(5)).await;
        }
    }
}

async fn send_step(
    transport: &dyn ReplicaTransport,
    manifest: &RepairManifest,
    step: RepairStep,
) -> CacheResult<RepairReceipt> {
    let receipt = transport
        .repair(RepairRequest {
            manifest: manifest.clone(),
            step,
        })
        .await?;
    if receipt.manifest != *manifest
        || receipt.next > manifest.chunks()
        || (receipt.installed && receipt.next != manifest.chunks())
        || (receipt.installed && !is_prefix_at_most(manifest.plan.required, receipt.applied))
        || (!receipt.installed && receipt.applied.is_some())
    {
        return Err(repair_error("recipient acknowledged another transfer"));
    }
    Ok(receipt)
}
