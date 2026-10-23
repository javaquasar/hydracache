//! Private expectation factory. No caller assertions become live authority.
use super::*;
use bootstrap_build::entry_format::AssertedEntry;
use kernel::bootstrap_entry::LegacyContribution;
use sha2::{Digest, Sha256};
fn hash(raw: &str) -> Result<[u8; 32], BootstrapError> {
    if !crate::is_hash(raw) {
        return Err(BootstrapError::Binding);
    }
    let mut out = [0; 32];
    for (i, v) in out.iter_mut().enumerate() {
        *v = u8::from_str_radix(&raw[i * 2..i * 2 + 2], 16).map_err(|_| BootstrapError::Binding)?;
    }
    Ok(out)
}
pub(super) fn expected(
    c: &Context,
    docs: &[Vec<u8>; 2],
    bootstrap: [u64; 3],
    legacy: LegacyContribution,
    census: &BootstrapImageBindings,
) -> Result<[u8; 1024], BootstrapError> {
    // These exact receipt bytes were just verified against the protected selector.
    // Parsing their assertion again exports no key, permit or verified capability.
    let receipt: bootstrap_build::SignedReceipt =
        serde_json::from_slice(&docs[1]).map_err(ArtifactError::from)?;
    let mut documents = [[0; 32]; 7];
    for (i, d) in c.documents_sha256.iter().enumerate() {
        documents[i] = hash(d)?;
    }
    for (i, d) in docs.iter().enumerate() {
        documents[5 + i] = Sha256::digest(d).into();
    }
    let mut owner = [0; 6];
    for (i, v) in c.owner.iter().enumerate() {
        owner[i] = u64::try_from(*v).map_err(|_| BootstrapError::Binding)?;
    }
    let value = AssertedEntry {
        owner,
        origins: c.origins,
        bounds: c.bounds,
        attempt: c.root.attempt_sha256,
        original_context: c.root.original_context_sha256,
        root: [
            c.root.root_device,
            c.root.root_inode,
            c.root.deadline_boottime_ns,
        ],
        serial: c.serial.get(),
        names: [
            Sha256::digest(c.names.0.as_bytes()).into(),
            Sha256::digest(c.names.1.as_bytes()).into(),
        ],
        documents,
        identities: [
            c.host.identity,
            c.launcher.identity,
            bootstrap,
            c.worker.identity,
            legacy.config,
            legacy.legacy,
        ],
        streams: [legacy.stdin, c.streams[0], c.streams[1], legacy.channel],
        guardians: c.guardians.map(|g| [g.0 as u64, g.1[0], g.1[1]]),
        image_hashes: [
            hash(&c.host.sha256)?,
            hash(&c.launcher.sha256)?,
            hash(&receipt.statement.bootstrap.sha256)?,
            hash(&c.worker.sha256)?,
            hash(&c.config.sha256)?,
        ],
        legacy_hash: legacy.hash,
        census_hash: census.owner_fingerprint(),
        state_hash: Sha256::digest(crate::canonical_json(&c.state).map_err(ArtifactError::from)?)
            .into(),
    };
    value.encode().map_err(|_| BootstrapError::Binding)
}

#[cfg(test)]
#[path = "diagnostic_bootstrap_entry_binding_tests.rs"]
mod tests;
