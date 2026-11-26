use super::*;
use crate::imap_value_repair::{
    admits_repair, bytes_digest, RepairManifest, RepairStep, ReplicaRepairOutcome, CHUNK_BYTES,
    REPAIR_WAIT,
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
        let _storage = lane
            .storage
            .try_lock()
            .map_err(|_| repair_error("recipient lane busy"))?;
        raft.with_applied_partition_snapshot(|snapshot| {
            if !admits_repair(&snapshot, &request.manifest.plan) {
                return Err(repair_error("recipient has no matching applied cut"));
            }
            self.journal
                .receive_image(request, |key| self.scope.admits_storage_key(key, partition))
        })
        .ok_or_else(|| repair_error("recipient Raft view unavailable"))?
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
        let _writer = lane
            .writer
            .try_acquire()
            .map_err(|_| repair_error("owner lane busy"))?;
        let deadline = Instant::now() + REPAIR_WAIT;
        let proof = request_read_index(raft, raft_transport, deadline)
            .await
            .map_err(|_| repair_error("owner Raft authority unavailable"))?;
        let (plan, image) = {
            let _storage = lane
                .storage
                .try_lock()
                .map_err(|_| repair_error("owner lane busy"))?;
            proof
                .with_snapshot(|snapshot, _| {
                    if let Some(prior) = snapshot.value_repairs.get(&partition).filter(|prior| {
                        prior.owner == self.local
                            && prior.replaced == replaced
                            && prior.target == target
                            && admitted(&snapshot, &self.local)
                            && admitted(&snapshot, &target)
                    }) {
                        return Ok((prior.clone(), None));
                    }
                    let next_epoch = snapshot
                        .epoch
                        .value()
                        .checked_add(1)
                        .ok_or_else(|| repair_error("epoch exhausted"))?;
                    let mut plan = PartitionReplicaRepair {
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
                    let image = self.journal.export_image(partition, plan.required)?;
                    plan.snapshot_digest = bytes_digest(&image);
                    Ok((plan, Some(image)))
                })
                .map_err(|_| repair_error("owner Raft authority changed"))??
        };
        let Some(image) = image else {
            return Ok(ReplicaRepairOutcome {
                plan,
                resumed_from: 0,
                chunks: 0,
            });
        };
        let manifest = RepairManifest {
            plan: plan.clone(),
            scope: self.repair_scope(),
            image_len: image.len(),
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
            if !send_step(transport, &manifest, RepairStep::Install)
                .await?
                .installed
            {
                return Err(repair_error("recipient did not confirm a durable image"));
            }
            let proof = request_read_index(raft, raft_transport, deadline)
                .await
                .map_err(|_| repair_error("owner lost authority before activation"))?;
            let current = proof
                .with_snapshot(|snapshot, _| admits_repair(&snapshot, &plan))
                .map_err(|_| repair_error("activation proof was fenced"))?;
            if !current {
                return Err(repair_error("certified placement changed during transfer"));
            }
            raft.propose_value_repair(plan.clone())?;
            let _ = send_raft_messages_with_diagnostics(
                raft_transport,
                raft.take_outbound_messages(),
                None,
            )
            .await;
            loop {
                let snapshot = raft
                    .partition_snapshot()
                    .ok_or_else(|| repair_error("activation view unavailable"))?;
                if snapshot.value_repairs.get(&partition) == Some(&plan) {
                    return Ok(ReplicaRepairOutcome {
                        plan,
                        resumed_from,
                        chunks: manifest.chunks(),
                    });
                }
                if snapshot.epoch != plan.old_epoch {
                    return Err(repair_error(
                        "replacement did not commit at the expected cut",
                    ));
                }
                sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .map_err(|_| repair_error("transfer or activation timed out"))?
    }

    fn repair_scope(&self) -> [u8; 32] {
        let (tenant, namespace, generation) = self.scope.identity();
        hash((1u16, tenant, namespace.as_str(), generation.get()))
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
    {
        return Err(repair_error("recipient acknowledged another transfer"));
    }
    Ok(receipt)
}
