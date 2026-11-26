//! Durable, version-ordered changes behind an in-progress repair snapshot.

use super::*;
use crate::imap_value_repair::{
    bytes_digest, is_prefix_at_most, repair_error, version, RepairManifest, MAX_DELTA_BYTES,
    MAX_DELTA_ROWS,
};

const SOURCE_STATE: &[u8] = b"repair-source";
const SOURCE_IMAGE: &[u8] = b"repair-source-image";
const DELTA_KEY: u8 = b'z';

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct CapturedTransfer {
    pub manifest: RepairManifest,
    last: Option<PartitionValuePrefix>,
    bytes: usize,
}

impl ValueJournal {
    pub(crate) fn captured_transfer(&self) -> CacheResult<Option<CapturedTransfer>> {
        let capture = self
            .0
            .get(SOURCE_STATE)
            .map_err(storage_error)?
            .map(|bytes| serde_json::from_slice::<CapturedTransfer>(&bytes).map_err(storage_error))
            .transpose()?;
        if capture.as_ref().is_some_and(|state| {
            let base = state.manifest.plan.required;
            !state.manifest.is_bounded()
                || !is_prefix_at_most(base, state.last)
                || version(state.last).saturating_sub(version(base)) > MAX_DELTA_ROWS
                || state.bytes > MAX_DELTA_BYTES
        }) {
            return Err(repair_error("invalid persisted source transfer"));
        }
        Ok(capture)
    }

    /// Called with the captured partition's writer and storage locks.
    pub(crate) fn start_capture(&self, manifest: &RepairManifest, image: &[u8]) -> CacheResult<()> {
        if !manifest.is_bounded()
            || image.len() != manifest.image_len
            || bytes_digest(image) != manifest.plan.snapshot_digest
            || self.captured_transfer()?.is_some()
        {
            return Err(repair_error("invalid or overlapping source transfer"));
        }
        let state = CapturedTransfer {
            manifest: manifest.clone(),
            last: manifest.plan.required,
            bytes: 0,
        };
        let mut batch = sled::Batch::default();
        batch.insert(
            SOURCE_STATE,
            serde_json::to_vec(&state).map_err(storage_error)?,
        );
        batch.insert(SOURCE_IMAGE, image);
        self.0.apply_batch(batch).map_err(storage_error)?;
        self.0.flush().map_err(storage_error)?;
        Ok(())
    }

    pub(crate) fn captured_image(&self, manifest: &RepairManifest) -> CacheResult<Vec<u8>> {
        let image = self
            .0
            .get(SOURCE_IMAGE)
            .map_err(storage_error)?
            .ok_or_else(|| repair_error("source snapshot is missing"))?;
        if image.len() != manifest.image_len
            || bytes_digest(&image) != manifest.plan.snapshot_digest
        {
            return Err(repair_error("source snapshot digest changed"));
        }
        // A retry must also finish any interrupted flush before relying on the capture.
        self.0.flush().map_err(storage_error)?;
        Ok(image.to_vec())
    }

    /// Clear after its epoch is fenced or under the original storage lock.
    pub(crate) fn finish_capture(&self, manifest: &RepairManifest) -> CacheResult<()> {
        let Some(current) = self.captured_transfer()? else {
            return Ok(());
        };
        if current.manifest != *manifest {
            return Err(repair_error("source transfer changed before cleanup"));
        }
        let start = version(manifest.plan.required);
        let end = version(current.last);
        let mut batch = sled::Batch::default();
        for offset in 1..=end - start {
            batch.remove(&delta_key(start + offset));
        }
        batch.remove(SOURCE_IMAGE);
        batch.remove(SOURCE_STATE);
        self.0.apply_batch(batch).map_err(storage_error)?;
        self.0.flush().map_err(storage_error)?;
        Ok(())
    }

    /// Append in the same batch as the owner apply, before acknowledging it.
    pub(super) fn capture_delta(
        &self,
        batch: &mut sled::Batch,
        prior: Option<PartitionValuePrefix>,
        mutation: &Mutation,
    ) -> CacheResult<bool> {
        let Some(mut current) = self.captured_transfer()? else {
            return Ok(true);
        };
        let plan = &current.manifest.plan;
        if plan.partition != mutation.record.partition || plan.old_epoch != mutation.record.epoch {
            return Ok(true);
        }
        let certificate = mutation.certificate();
        if current.last == Some(certificate) {
            // An ambiguous write can retry the same pending mutation.
            return Ok(true);
        }
        if current.last != prior || mutation.owner != plan.owner || !consecutive(prior, certificate)
        {
            return Err(repair_error("source delta is not the next mutation"));
        }
        if certificate.version - version(plan.required) > MAX_DELTA_ROWS {
            return Ok(false);
        }
        let encoded = serde_json::to_vec(mutation).map_err(storage_error)?;
        if encoded.len() > MAX_DELTA_BYTES - current.bytes {
            return Ok(false);
        }
        current.bytes += encoded.len();
        current.last = Some(certificate);
        batch.insert(&delta_key(certificate.version), encoded);
        batch.insert(
            SOURCE_STATE,
            serde_json::to_vec(&current).map_err(storage_error)?,
        );
        Ok(true)
    }

    /// Return a chain ending at a certificate observed in Raft by the caller.
    pub(crate) fn captured_deltas(
        &self,
        manifest: &RepairManifest,
        from: Option<PartitionValuePrefix>,
        through: Option<PartitionValuePrefix>,
    ) -> CacheResult<(Vec<Mutation>, usize)> {
        let capture = self
            .captured_transfer()?
            .filter(|capture| capture.manifest == *manifest)
            .ok_or_else(|| repair_error("source capture is missing"))?;
        let base = manifest.plan.required;
        if !is_prefix_at_most(base, from)
            || !is_prefix_at_most(from, through)
            || !is_prefix_at_most(through, capture.last)
        {
            return Err(repair_error("delta cursor is outside the captured chain"));
        }
        if from != base
            && self.read_delta(manifest, version(from))?.0.certificate() != from.unwrap()
        {
            return Err(repair_error("recipient cursor has the wrong digest"));
        }
        let start = version(from);
        let mut previous = from;
        let mut deltas = Vec::new();
        let mut bytes = 0;
        for offset in 1..=version(through) - start {
            let (mutation, length) = self.read_delta(manifest, start + offset)?;
            let certificate = mutation.certificate();
            if !consecutive(previous, certificate) || length > MAX_DELTA_BYTES - bytes {
                return Err(repair_error("captured delta chain is invalid"));
            }
            bytes += length;
            previous = Some(certificate);
            deltas.push(mutation);
        }
        if previous != through {
            return Err(repair_error(
                "delta chain differs from the Raft certificate",
            ));
        }
        Ok((deltas, bytes))
    }

    fn read_delta(
        &self,
        manifest: &RepairManifest,
        version: u64,
    ) -> CacheResult<(Mutation, usize)> {
        let encoded = self
            .0
            .get(delta_key(version))
            .map_err(storage_error)?
            .ok_or_else(|| repair_error("captured mutation is missing"))?;
        let mutation: Mutation = serde_json::from_slice(&encoded).map_err(storage_error)?;
        let plan = &manifest.plan;
        if mutation.record.version != version
            || mutation.record.epoch != plan.old_epoch
            || mutation.record.partition != plan.partition
            || mutation.owner != plan.owner
        {
            return Err(repair_error("captured mutation is outside the transfer"));
        }
        Ok((mutation, encoded.len()))
    }
}

pub(super) fn consecutive(prior: Option<PartitionValuePrefix>, next: PartitionValuePrefix) -> bool {
    version(prior).checked_add(1) == Some(next.version)
        && prior.map_or([0; 32], |prefix| prefix.digest) == next.previous
        && prior
            .is_none_or(|prefix| prefix.epoch <= next.epoch && prefix.partition == next.partition)
}

fn delta_key(version: u64) -> [u8; 9] {
    let [a, b, c, d, e, f, g, h] = version.to_be_bytes();
    [DELTA_KEY, a, b, c, d, e, f, g, h]
}
