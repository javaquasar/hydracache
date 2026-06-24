use std::collections::BTreeMap;

use super::{checksum, decode_hex, encode_hex, validate_name, BackupError, ObjectStore};

/// Current full-backup manifest format.
pub const BACKUP_MANIFEST_FORMAT_VERSION: u16 = 1;

/// Source/restore dataset used by the backup seam.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BackupDataset {
    /// Serialized control-plane snapshot bytes.
    pub control_plane: Vec<u8>,
    /// Durable value records keyed by logical cache key.
    pub values: BTreeMap<String, Vec<u8>>,
}

impl BackupDataset {
    /// Create a dataset with control-plane bytes.
    pub fn new(control_plane: impl Into<Vec<u8>>) -> Self {
        Self {
            control_plane: control_plane.into(),
            values: BTreeMap::new(),
        }
    }

    /// Add one durable value record.
    pub fn with_value(mut self, key: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Self {
        self.values.insert(key.into(), bytes.into());
        self
    }
}

/// One manifest entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupEntry {
    /// Logical record key.
    pub logical_key: String,
    /// Object-store key that contains the bytes.
    pub object_key: String,
    /// Byte length expected during restore.
    pub len: usize,
    /// Checksum expected during restore.
    pub checksum: u64,
}

/// Full backup manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupManifest {
    /// Manifest format version.
    pub format_version: u16,
    /// Backup id.
    pub backup_id: String,
    /// Monotonic checkpoint covered by the snapshot.
    pub checkpoint: u64,
    /// Object key for control-plane bytes.
    pub control_plane: BackupEntry,
    /// Durable value entries.
    pub values: Vec<BackupEntry>,
    /// Object key where this manifest was written.
    pub manifest_key: String,
}

impl BackupManifest {
    /// Encode manifest as a deterministic text artifact.
    pub fn encode(&self) -> Vec<u8> {
        let mut text = format!(
            "hydracache-backup\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            self.format_version,
            encode_hex(self.backup_id.as_bytes()),
            self.checkpoint,
            encode_hex(self.control_plane.logical_key.as_bytes()),
            encode_hex(self.control_plane.object_key.as_bytes()),
            self.control_plane.len,
            self.control_plane.checksum
        );
        for entry in &self.values {
            text.push_str(&format!(
                "value\t{}\t{}\t{}\t{}\n",
                encode_hex(entry.logical_key.as_bytes()),
                encode_hex(entry.object_key.as_bytes()),
                entry.len,
                entry.checksum
            ));
        }
        text.into_bytes()
    }

    /// Decode a manifest read from object storage.
    pub fn decode(manifest_key: impl Into<String>, bytes: &[u8]) -> Result<Self, BackupError> {
        let manifest_key = manifest_key.into();
        let text = std::str::from_utf8(bytes)
            .map_err(|_| BackupError::InvalidManifest("manifest is not utf-8".to_owned()))?;
        let mut lines = text.lines();
        let header = lines
            .next()
            .ok_or_else(|| BackupError::InvalidManifest("missing header".to_owned()))?;
        let parts: Vec<_> = header.split('\t').collect();
        if parts.len() != 8 || parts[0] != "hydracache-backup" {
            return Err(BackupError::InvalidManifest("invalid header".to_owned()));
        }
        let format_version = parse_u16(parts[1])?;
        if format_version != BACKUP_MANIFEST_FORMAT_VERSION {
            return Err(BackupError::UnsupportedManifestFormat(format_version));
        }
        let backup_id = decode_string(parts[2])?;
        let checkpoint = parse_u64(parts[3])?;
        let control_plane = BackupEntry {
            logical_key: decode_string(parts[4])?,
            object_key: decode_string(parts[5])?,
            len: parse_usize(parts[6])?,
            checksum: parse_u64(parts[7])?,
        };
        let mut values = Vec::new();
        for line in lines {
            let parts: Vec<_> = line.split('\t').collect();
            if parts.len() != 5 || parts[0] != "value" {
                return Err(BackupError::InvalidManifest(
                    "invalid value entry".to_owned(),
                ));
            }
            values.push(BackupEntry {
                logical_key: decode_string(parts[1])?,
                object_key: decode_string(parts[2])?,
                len: parse_usize(parts[3])?,
                checksum: parse_u64(parts[4])?,
            });
        }
        Ok(Self {
            format_version,
            backup_id,
            checkpoint,
            control_plane,
            values,
            manifest_key,
        })
    }
}

/// Write a full backup to an object store and return its manifest.
pub fn write_full_backup<S>(
    store: &mut S,
    backup_id: &str,
    checkpoint: u64,
    dataset: &BackupDataset,
) -> Result<BackupManifest, BackupError>
where
    S: ObjectStore,
{
    validate_name(backup_id, "backup id")?;
    let prefix = format!("backups/{backup_id}");
    let control_key = format!("{prefix}/control-plane");
    store.put(&control_key, dataset.control_plane.clone())?;
    let control_plane = entry(
        "control-plane".to_owned(),
        control_key,
        &dataset.control_plane,
    );

    let mut values = Vec::with_capacity(dataset.values.len());
    for (index, (logical_key, bytes)) in dataset.values.iter().enumerate() {
        validate_name(logical_key, "logical key")?;
        let object_key = format!("{prefix}/values/{index:016}");
        store.put(&object_key, bytes.clone())?;
        values.push(entry(logical_key.clone(), object_key, bytes));
    }

    let manifest_key = format!("{prefix}/manifest");
    let manifest = BackupManifest {
        format_version: BACKUP_MANIFEST_FORMAT_VERSION,
        backup_id: backup_id.to_owned(),
        checkpoint,
        control_plane,
        values,
        manifest_key,
    };
    store.put(&manifest.manifest_key, manifest.encode())?;
    Ok(manifest)
}

/// Read and decode a manifest from object storage.
pub fn read_manifest<S>(store: &S, manifest_key: &str) -> Result<BackupManifest, BackupError>
where
    S: ObjectStore,
{
    let bytes = store.get(manifest_key)?;
    BackupManifest::decode(manifest_key, &bytes)
}

pub(super) fn restore_dataset_from_manifest<S>(
    store: &S,
    manifest: &BackupManifest,
) -> Result<BackupDataset, BackupError>
where
    S: ObjectStore,
{
    let control_plane = read_checked(store, &manifest.control_plane)?;
    let mut values = BTreeMap::new();
    for entry in &manifest.values {
        values.insert(entry.logical_key.clone(), read_checked(store, entry)?);
    }
    Ok(BackupDataset {
        control_plane,
        values,
    })
}

fn entry(logical_key: String, object_key: String, bytes: &[u8]) -> BackupEntry {
    BackupEntry {
        logical_key,
        object_key,
        len: bytes.len(),
        checksum: checksum(&[bytes]),
    }
}

fn read_checked<S>(store: &S, entry: &BackupEntry) -> Result<Vec<u8>, BackupError>
where
    S: ObjectStore,
{
    let bytes = store.get(&entry.object_key)?;
    if bytes.len() != entry.len || checksum(&[&bytes]) != entry.checksum {
        return Err(BackupError::CorruptObject(entry.object_key.clone()));
    }
    Ok(bytes)
}

fn decode_string(text: &str) -> Result<String, BackupError> {
    String::from_utf8(decode_hex(text)?)
        .map_err(|_| BackupError::InvalidManifest("hex string is not utf-8".to_owned()))
}

fn parse_u16(text: &str) -> Result<u16, BackupError> {
    text.parse()
        .map_err(|_| BackupError::InvalidManifest("invalid u16".to_owned()))
}

fn parse_u64(text: &str) -> Result<u64, BackupError> {
    text.parse()
        .map_err(|_| BackupError::InvalidManifest("invalid u64".to_owned()))
}

fn parse_usize(text: &str) -> Result<usize, BackupError> {
    text.parse()
        .map_err(|_| BackupError::InvalidManifest("invalid usize".to_owned()))
}
