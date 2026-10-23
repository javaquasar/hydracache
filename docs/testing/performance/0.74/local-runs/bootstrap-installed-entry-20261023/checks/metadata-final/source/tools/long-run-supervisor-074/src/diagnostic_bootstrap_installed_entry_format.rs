//! Canonical assertions, never authenticated custody or a child execution permit.
use super::entry_format::AssertedEntry;
use sha2::{Digest, Sha256};

pub const BYTES: usize = 1280;
pub const PARENT_DESCRIPTORS: u32 = 32;
pub const PARENT_ROLES: u32 = 16;
pub const INSTALLED_PATH: &str = "/opt/hydracache-perf/bin/diagnostic-held-bootstrap-074";
const ORIGINAL_END: usize = 16 + super::entry_format::BYTES;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssertedInstalledEntry {
    pub original: AssertedEntry,
    pub installed_identity: [u64; 3],
    pub installed_sha256: [u8; 32],
    // Opaque parent-local value, not an inherited child FD map or authentication.
    pub parent_inventory_sha256: [u8; 32],
}

#[derive(Debug, thiserror::Error)]
#[error("invalid installed bootstrap entry assertion shape")]
pub struct InvalidInstalledEntry;

fn need(value: bool) -> Result<(), InvalidInstalledEntry> {
    if value {
        Ok(())
    } else {
        Err(InvalidInstalledEntry)
    }
}

impl AssertedInstalledEntry {
    fn original_bytes(&self) -> Result<[u8; 1024], InvalidInstalledEntry> {
        let original = self.original.encode().map_err(|_| InvalidInstalledEntry)?;
        need(
            self.installed_identity.iter().all(|v| *v > 0)
                && self.installed_identity[2] <= 128 * 1024 * 1024
                && self.installed_identity[2] == self.original.identities[2][2]
                && self.installed_sha256 == self.original.image_hashes[2]
                && self
                    .original
                    .identities
                    .iter()
                    .all(|id| id[..2] != self.installed_identity[..2]),
        )?;
        Ok(original)
    }

    pub fn encode(&self) -> Result<[u8; BYTES], InvalidInstalledEntry> {
        let original = self.original_bytes()?;
        let mut raw = [0; BYTES];
        raw[..8].copy_from_slice(b"HCBI0741");
        raw[8..12].copy_from_slice(&(BYTES as u32).to_le_bytes());
        raw[16..ORIGINAL_END].copy_from_slice(&original);
        for (i, value) in self.installed_identity.iter().enumerate() {
            raw[1040 + i * 8..1048 + i * 8].copy_from_slice(&value.to_le_bytes());
        }
        raw[1064..1096].copy_from_slice(&self.installed_sha256);
        raw[1096..1128].copy_from_slice(&Sha256::digest(INSTALLED_PATH.as_bytes()));
        raw[1128..1160].copy_from_slice(&self.parent_inventory_sha256);
        raw[1160..1164].copy_from_slice(&PARENT_DESCRIPTORS.to_le_bytes());
        raw[1164..1168].copy_from_slice(&PARENT_ROLES.to_le_bytes());
        raw[1168..1200].copy_from_slice(&Sha256::digest(original));
        Ok(raw)
    }

    pub fn decode(raw: &[u8]) -> Result<Self, InvalidInstalledEntry> {
        need(raw.len() == BYTES)?;
        need(
            &raw[..8] == b"HCBI0741"
                && raw[8..12] == (BYTES as u32).to_le_bytes()
                && raw[12..16] == [0; 4]
                && raw[1200..] == [0; 80]
                && raw[1096..1128] == Sha256::digest(INSTALLED_PATH.as_bytes())[..]
                && raw[1160..1164] == PARENT_DESCRIPTORS.to_le_bytes()
                && raw[1164..1168] == PARENT_ROLES.to_le_bytes()
                && raw[1168..1200] == Sha256::digest(&raw[16..ORIGINAL_END])[..],
        )?;
        let value = Self {
            original: AssertedEntry::decode(&raw[16..ORIGINAL_END])
                .map_err(|_| InvalidInstalledEntry)?,
            installed_identity: std::array::from_fn(|i| {
                u64::from_le_bytes(
                    raw[1040 + i * 8..1048 + i * 8]
                        .try_into()
                        .expect("fixed shape"),
                )
            }),
            installed_sha256: raw[1064..1096].try_into().expect("fixed shape"),
            parent_inventory_sha256: raw[1128..1160].try_into().expect("fixed shape"),
        };
        value.original_bytes()?;
        Ok(value)
    }
}

#[cfg(test)]
#[path = "diagnostic_bootstrap_installed_entry_format_tests.rs"]
mod tests;
