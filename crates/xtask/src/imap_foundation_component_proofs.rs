use hydracache_cluster_testkit::value_plane_admission_075::{
    NamespaceLifecycle, ReclamationOwners, ReclamationWatermarks,
};
use hydracache_cluster_testkit::value_plane_bulk_075::{
    BulkBounds, BulkInput, BulkOutcome, PartitionedBulkExecution,
};
use hydracache_cluster_testkit::value_plane_listener_075::{
    ClusterListenerModel, ListenerBounds, ListenerEvent, ListenerEventKind,
};
use hydracache_cluster_testkit::value_plane_model_075::{CanonicalMapKey, MutationIdentity};
use hydracache_cluster_testkit::value_plane_transfer_075::{
    digest_chunks, PartitionTransfer, TransferBounds, TransferManifest,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::error::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentProofKind {
    Transfer,
    Listener,
    Bulk,
    Lifecycle,
}

impl ComponentProofKind {
    pub const ALL: [Self; 4] = [Self::Transfer, Self::Listener, Self::Bulk, Self::Lifecycle];

    pub const fn kind(self) -> &'static str {
        match self {
            Self::Transfer => "transfer",
            Self::Listener => "listener",
            Self::Bulk => "bulk",
            Self::Lifecycle => "lifecycle",
        }
    }

    pub const fn schema(self) -> &'static str {
        match self {
            Self::Transfer => "hydracache.imap.transfer-receipt.v1",
            Self::Listener => "hydracache.imap.listener-receipt.v1",
            Self::Bulk => "hydracache.imap.bulk-receipt.v1",
            Self::Lifecycle => "hydracache.imap.lifecycle-receipt.v1",
        }
    }

    pub const fn file_name(self) -> &'static str {
        match self {
            Self::Transfer => "transfer.json",
            Self::Listener => "listener.json",
            Self::Bulk => "bulk.json",
            Self::Lifecycle => "lifecycle.json",
        }
    }

    pub fn from_schema(schema: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.schema() == schema)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentProof {
    pub seed: u64,
    pub steps: usize,
    pub proof_sha256: String,
}

pub fn execute_component_proof(
    kind: ComponentProofKind,
    seed: u64,
) -> Result<ComponentProof, Box<dyn Error>> {
    let trace = match kind {
        ComponentProofKind::Transfer => transfer_trace(seed)?,
        ComponentProofKind::Listener => listener_trace(seed)?,
        ComponentProofKind::Bulk => bulk_trace(seed)?,
        ComponentProofKind::Lifecycle => lifecycle_trace(seed)?,
    };
    Ok(ComponentProof {
        seed,
        steps: trace.len(),
        proof_sha256: trace_digest(seed, &trace),
    })
}

fn transfer_trace(seed: u64) -> Result<Vec<String>, Box<dyn Error>> {
    let first = seed.to_be_bytes();
    let second = seed.rotate_left(17).to_be_bytes();
    let chunks = [first.as_slice(), second.as_slice()];
    let mut transfer = PartitionTransfer::new(
        TransferBounds {
            max_chunks: 2,
            max_chunk_bytes: 8,
            max_delta_entries: 2,
        },
        TransferManifest {
            partition: 0,
            source_epoch: 1,
            source: "node-a".into(),
            target: "node-b".into(),
            chunk_count: 2,
            snapshot_sha256: digest_chunks(&chunks),
        },
    )?;
    transfer.accept_chunk(1, second.to_vec())?;
    let resume = transfer
        .resume_from()
        .ok_or("transfer did not expose resume point")?;
    transfer.accept_chunk(0, first.to_vec())?;
    transfer.finish_snapshot()?;
    transfer.stage_delta(2, seed)?;
    let digest = transfer.state_sha256();
    transfer.mark_ready(2, &digest)?;
    transfer.commit(2)?;
    if transfer.source_can_serve() || !transfer.target_can_serve() {
        return Err("transfer authority did not cut over".into());
    }
    Ok(vec![
        "chunk:1".into(),
        format!("resume:{resume}"),
        "chunk:0".into(),
        "snapshot-verified".into(),
        "delta:2".into(),
        "ready".into(),
        "committed:2".into(),
    ])
}

fn listener_trace(seed: u64) -> Result<Vec<String>, Box<dyn Error>> {
    let mut listener = ClusterListenerModel::new(ListenerBounds {
        max_partitions: 2,
        max_backlog: 2,
    })?;
    listener.register(0, 1, 0)?;
    for watermark in 1..=2 {
        listener.publish(ListenerEvent {
            partition: 0,
            generation: 1,
            watermark,
            kind: ListenerEventKind::Mutation {
                key: seed.to_be_bytes().to_vec(),
                removed: false,
            },
        })?;
    }
    let overflow = listener.publish(ListenerEvent {
        partition: 0,
        generation: 1,
        watermark: 3,
        kind: ListenerEventKind::Mutation {
            key: vec![0],
            removed: true,
        },
    });
    if overflow.is_ok() || !listener.requires_repair(0)? {
        return Err("listener overflow did not create a repair boundary".into());
    }
    listener.repair(0, 1, 3)?;
    listener.migrate(0, 2)?;
    listener.repair(0, 2, 3)?;
    listener.publish(ListenerEvent {
        partition: 0,
        generation: 2,
        watermark: 4,
        kind: ListenerEventKind::Mutation {
            key: vec![1],
            removed: false,
        },
    })?;
    Ok(vec![
        "event:1".into(),
        "event:2".into(),
        "overflow-gap:2".into(),
        "repair:3".into(),
        "migration-gap:g2".into(),
        "repair:g2:3".into(),
        "event:g2:4".into(),
    ])
}

fn bulk_trace(seed: u64) -> Result<Vec<String>, Box<dyn Error>> {
    let inputs = (0..3)
        .map(|index| BulkInput {
            index,
            key: CanonicalMapKey::new(
                "tenant-a",
                "orders",
                1,
                vec![seed as u8, index as u8],
                (index % 2) as u32,
            ),
            identity: MutationIdentity::new("evidence-bulk", seed + index as u64),
            request_bytes: 2,
        })
        .collect();
    let mut execution = PartitionedBulkExecution::new(
        BulkBounds {
            max_items: 3,
            max_request_bytes: 6,
            max_partitions: 2,
        },
        inputs,
        &BTreeMap::from([(0, 1), (1, 1)]),
    )?;
    execution.complete(0, 1, BulkOutcome::Applied { version: 1 })?;
    execution.advance_owner(0, 2)?;
    execution.complete(2, 2, BulkOutcome::Applied { version: 2 })?;
    execution.complete(1, 1, BulkOutcome::Absent)?;
    let receipt = execution.receipt();
    if !receipt.complete || receipt.items.iter().any(|item| item.outcome.is_none()) {
        return Err("bulk receipt is incomplete".into());
    }
    Ok(receipt
        .items
        .iter()
        .map(|item| {
            format!(
                "item:{}:p{}:g{}:{:?}",
                item.input_index, item.partition, item.owner_generation, item.outcome
            )
        })
        .collect())
}

fn lifecycle_trace(seed: u64) -> Result<Vec<String>, Box<dyn Error>> {
    let generation = seed.max(1);
    let watermark = generation.saturating_add(5);
    let mut lifecycle = NamespaceLifecycle::new(generation)?;
    lifecycle.begin_delete()?;
    lifecycle.commit_delete(
        watermark,
        ReclamationOwners {
            entries: 1,
            ttl_tasks: 1,
            tombstones: 1,
            dedup_results: 1,
            subscriptions: 1,
            staging_files: 1,
            quota_owners: 1,
        },
    )?;
    lifecycle.advance_reclamation(ReclamationWatermarks {
        replica_applied: watermark,
        listener_cutover: watermark,
        dedup_expired: watermark,
        transfer_closed: watermark,
        tombstone_gc: watermark,
    });
    let first = lifecycle.reclaim_step(3)?;
    let second = lifecycle.reclaim_step(4)?;
    let next_generation = lifecycle.recreate()?;
    Ok(vec![
        format!("draining:g{generation}"),
        format!("delete-committed:w{watermark}"),
        "watermarks-proved".into(),
        format!("reclaimed:{first}"),
        format!("reclaimed:{second}"),
        format!("recreated:g{next_generation}"),
    ])
}

fn trace_digest(seed: u64, trace: &[String]) -> String {
    let mut digest = Sha256::new();
    digest.update(seed.to_be_bytes());
    for entry in trace {
        digest.update((entry.len() as u64).to_be_bytes());
        digest.update(entry.as_bytes());
    }
    digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
