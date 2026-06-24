//! Object-store backup and point-in-time restore seams.

mod full;
mod object_store;
mod pitr;
mod restore;

pub use full::{
    read_manifest, write_full_backup, BackupDataset, BackupEntry, BackupManifest,
    BACKUP_MANIFEST_FORMAT_VERSION,
};
pub use object_store::{InMemoryObjectStore, ObjectStore};
pub use pitr::{write_pitr_log, PitrLog, PitrRecord};
pub use restore::restore_backup_to_point;

use std::error::Error;
use std::fmt;

/// Backup and restore errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackupError {
    /// Object key or logical id is empty/invalid.
    InvalidName(&'static str),
    /// Object is missing from the store.
    MissingObject(String),
    /// Object bytes failed checksum/length validation.
    CorruptObject(String),
    /// Manifest format is not supported by this binary.
    UnsupportedManifestFormat(u16),
    /// Manifest text cannot be decoded.
    InvalidManifest(String),
}

impl fmt::Display for BackupError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidName(name) => write!(formatter, "invalid backup name: {name}"),
            Self::MissingObject(key) => write!(formatter, "missing backup object: {key}"),
            Self::CorruptObject(key) => write!(formatter, "corrupt backup object: {key}"),
            Self::UnsupportedManifestFormat(version) => {
                write!(formatter, "unsupported backup manifest format: {version}")
            }
            Self::InvalidManifest(reason) => write!(formatter, "invalid backup manifest: {reason}"),
        }
    }
}

impl Error for BackupError {}

fn checksum(parts: &[&[u8]]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for part in parts {
        for byte in *part {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        hash ^= 0xfe;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn decode_hex(text: &str) -> Result<Vec<u8>, BackupError> {
    if !text.len().is_multiple_of(2) {
        return Err(BackupError::InvalidManifest("odd hex length".to_owned()));
    }
    let mut bytes = Vec::with_capacity(text.len() / 2);
    let raw = text.as_bytes();
    for chunk in raw.chunks_exact(2) {
        let high = decode_nibble(chunk[0])?;
        let low = decode_nibble(chunk[1])?;
        bytes.push((high << 4) | low);
    }
    Ok(bytes)
}

fn decode_nibble(byte: u8) -> Result<u8, BackupError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(BackupError::InvalidManifest("invalid hex digit".to_owned())),
    }
}

fn validate_name(name: &str, field: &'static str) -> Result<(), BackupError> {
    if name.trim().is_empty() || name.contains(['\n', '\r', '\t']) {
        return Err(BackupError::InvalidName(field));
    }
    Ok(())
}
