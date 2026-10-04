//! Bounded resumable partition-transfer state machine for provisional 0.75 proofs.

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransferBounds {
    pub max_chunks: usize,
    pub max_chunk_bytes: usize,
    pub max_delta_entries: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferManifest {
    pub partition: u32,
    pub source_epoch: u64,
    pub source: String,
    pub target: String,
    pub chunk_count: usize,
    pub snapshot_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferPhase {
    Snapshot,
    Delta,
    Ready,
    Committed,
    RolledBack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkAcceptance {
    Stored,
    Replayed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferError {
    InvalidBound(&'static str),
    InvalidManifest,
    WrongPhase {
        expected: TransferPhase,
        actual: TransferPhase,
    },
    ChunkOutOfRange,
    ChunkTooLarge,
    ConflictingChunk,
    MissingChunks,
    SnapshotChecksumMismatch,
    DeltaLimit,
    DeltaVersionRegression,
    ConflictingDelta,
    FinalChecksumMismatch,
    EpochMustAdvance,
    AlreadyCommitted,
}

#[derive(Debug, Clone)]
pub struct PartitionTransfer {
    bounds: TransferBounds,
    manifest: TransferManifest,
    phase: TransferPhase,
    chunks: BTreeMap<usize, Vec<u8>>,
    deltas: BTreeMap<u64, u64>,
    cutover_version: Option<u64>,
    committed_epoch: Option<u64>,
}

impl PartitionTransfer {
    pub fn new(bounds: TransferBounds, manifest: TransferManifest) -> Result<Self, TransferError> {
        for (name, value) in [
            ("chunks", bounds.max_chunks),
            ("chunk_bytes", bounds.max_chunk_bytes),
            ("delta_entries", bounds.max_delta_entries),
        ] {
            if value == 0 {
                return Err(TransferError::InvalidBound(name));
            }
        }
        if manifest.source_epoch == 0
            || manifest.source.is_empty()
            || manifest.target.is_empty()
            || manifest.source == manifest.target
            || manifest.chunk_count == 0
            || manifest.chunk_count > bounds.max_chunks
            || manifest.snapshot_sha256.len() != 64
            || !manifest
                .snapshot_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(TransferError::InvalidManifest);
        }
        Ok(Self {
            bounds,
            manifest,
            phase: TransferPhase::Snapshot,
            chunks: BTreeMap::new(),
            deltas: BTreeMap::new(),
            cutover_version: None,
            committed_epoch: None,
        })
    }

    pub const fn phase(&self) -> TransferPhase {
        self.phase
    }

    pub fn accept_chunk(
        &mut self,
        index: usize,
        bytes: Vec<u8>,
    ) -> Result<ChunkAcceptance, TransferError> {
        self.require_phase(TransferPhase::Snapshot)?;
        if index >= self.manifest.chunk_count {
            return Err(TransferError::ChunkOutOfRange);
        }
        if bytes.len() > self.bounds.max_chunk_bytes {
            return Err(TransferError::ChunkTooLarge);
        }
        if let Some(existing) = self.chunks.get(&index) {
            return if existing == &bytes {
                Ok(ChunkAcceptance::Replayed)
            } else {
                Err(TransferError::ConflictingChunk)
            };
        }
        self.chunks.insert(index, bytes);
        Ok(ChunkAcceptance::Stored)
    }

    pub fn resume_from(&self) -> Option<usize> {
        (0..self.manifest.chunk_count).find(|index| !self.chunks.contains_key(index))
    }

    pub fn finish_snapshot(&mut self) -> Result<(), TransferError> {
        self.require_phase(TransferPhase::Snapshot)?;
        if self.chunks.len() != self.manifest.chunk_count {
            return Err(TransferError::MissingChunks);
        }
        let ordered = (0..self.manifest.chunk_count)
            .map(|index| {
                self.chunks
                    .get(&index)
                    .expect("all chunks checked")
                    .as_slice()
            })
            .collect::<Vec<_>>();
        if digest_chunks(&ordered) != self.manifest.snapshot_sha256 {
            return Err(TransferError::SnapshotChecksumMismatch);
        }
        self.phase = TransferPhase::Delta;
        Ok(())
    }

    pub fn stage_delta(&mut self, version: u64, checksum: u64) -> Result<(), TransferError> {
        self.require_phase(TransferPhase::Delta)?;
        if let Some(existing) = self.deltas.get(&version) {
            return if *existing == checksum {
                Ok(())
            } else {
                Err(TransferError::ConflictingDelta)
            };
        }
        if self.deltas.len() >= self.bounds.max_delta_entries {
            return Err(TransferError::DeltaLimit);
        }
        if self
            .deltas
            .last_key_value()
            .is_some_and(|(last, _)| version <= *last)
        {
            return Err(TransferError::DeltaVersionRegression);
        }
        self.deltas.insert(version, checksum);
        Ok(())
    }

    pub fn state_sha256(&self) -> String {
        let mut digest = Sha256::new();
        for index in 0..self.manifest.chunk_count {
            if let Some(chunk) = self.chunks.get(&index) {
                digest.update((index as u64).to_be_bytes());
                digest.update((chunk.len() as u64).to_be_bytes());
                digest.update(chunk);
            }
        }
        for (version, checksum) in &self.deltas {
            digest.update(version.to_be_bytes());
            digest.update(checksum.to_be_bytes());
        }
        hex_digest(digest.finalize().as_slice())
    }

    pub fn mark_ready(
        &mut self,
        cutover_version: u64,
        expected_state_sha256: &str,
    ) -> Result<(), TransferError> {
        self.require_phase(TransferPhase::Delta)?;
        if self.state_sha256() != expected_state_sha256 {
            return Err(TransferError::FinalChecksumMismatch);
        }
        if self
            .deltas
            .last_key_value()
            .is_some_and(|(last, _)| cutover_version < *last)
        {
            return Err(TransferError::DeltaVersionRegression);
        }
        self.cutover_version = Some(cutover_version);
        self.phase = TransferPhase::Ready;
        Ok(())
    }

    pub fn commit(&mut self, epoch: u64) -> Result<(), TransferError> {
        self.require_phase(TransferPhase::Ready)?;
        if epoch <= self.manifest.source_epoch {
            return Err(TransferError::EpochMustAdvance);
        }
        self.committed_epoch = Some(epoch);
        self.phase = TransferPhase::Committed;
        Ok(())
    }

    pub fn rollback(&mut self) -> Result<(), TransferError> {
        if self.phase == TransferPhase::Committed {
            return Err(TransferError::AlreadyCommitted);
        }
        self.phase = TransferPhase::RolledBack;
        self.chunks.clear();
        self.deltas.clear();
        Ok(())
    }

    pub fn source_can_serve(&self) -> bool {
        self.phase != TransferPhase::Committed
    }

    pub fn target_can_serve(&self) -> bool {
        self.phase == TransferPhase::Committed
    }

    pub const fn committed_epoch(&self) -> Option<u64> {
        self.committed_epoch
    }

    pub const fn cutover_version(&self) -> Option<u64> {
        self.cutover_version
    }

    fn require_phase(&self, expected: TransferPhase) -> Result<(), TransferError> {
        if self.phase == expected {
            Ok(())
        } else {
            Err(TransferError::WrongPhase {
                expected,
                actual: self.phase,
            })
        }
    }
}

pub fn digest_chunks(chunks: &[&[u8]]) -> String {
    let mut digest = Sha256::new();
    for (index, chunk) in chunks.iter().enumerate() {
        digest.update((index as u64).to_be_bytes());
        digest.update((chunk.len() as u64).to_be_bytes());
        digest.update(chunk);
    }
    hex_digest(digest.finalize().as_slice())
}

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
