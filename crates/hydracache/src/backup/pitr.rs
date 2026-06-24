use super::{checksum, decode_hex, encode_hex, validate_name, BackupError, ObjectStore};

/// One point-in-time restore change record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PitrRecord {
    /// Monotonic sequence number.
    pub sequence: u64,
    /// Logical value key.
    pub key: String,
    /// `Some` for put/upsert, `None` for delete.
    pub value: Option<Vec<u8>>,
}

impl PitrRecord {
    /// Create a put/upsert record.
    pub fn put(sequence: u64, key: impl Into<String>, value: impl Into<Vec<u8>>) -> Self {
        Self {
            sequence,
            key: key.into(),
            value: Some(value.into()),
        }
    }

    /// Create a delete record.
    pub fn delete(sequence: u64, key: impl Into<String>) -> Self {
        Self {
            sequence,
            key: key.into(),
            value: None,
        }
    }
}

/// PITR log shipped after a full backup checkpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PitrLog {
    /// Ordered change records.
    pub records: Vec<PitrRecord>,
}

impl PitrLog {
    /// Create an empty log.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a record.
    pub fn push(mut self, record: PitrRecord) -> Self {
        self.records.push(record);
        self
    }

    /// Encode the log as deterministic text.
    pub fn encode(&self) -> Result<Vec<u8>, BackupError> {
        let mut previous = 0;
        let mut text = String::from("hydracache-pitr\t1\n");
        for record in &self.records {
            if record.sequence <= previous {
                return Err(BackupError::InvalidManifest(
                    "pitr sequence must be strictly increasing".to_owned(),
                ));
            }
            validate_name(&record.key, "pitr key")?;
            previous = record.sequence;
            match &record.value {
                Some(value) => text.push_str(&format!(
                    "put\t{}\t{}\t{}\t{}\n",
                    record.sequence,
                    encode_hex(record.key.as_bytes()),
                    checksum(&[value]),
                    encode_hex(value)
                )),
                None => text.push_str(&format!(
                    "delete\t{}\t{}\t0\t\n",
                    record.sequence,
                    encode_hex(record.key.as_bytes())
                )),
            }
        }
        Ok(text.into_bytes())
    }

    /// Decode a PITR log.
    pub fn decode(bytes: &[u8]) -> Result<Self, BackupError> {
        let text = std::str::from_utf8(bytes)
            .map_err(|_| BackupError::InvalidManifest("pitr log is not utf-8".to_owned()))?;
        let mut lines = text.lines();
        if lines.next() != Some("hydracache-pitr\t1") {
            return Err(BackupError::InvalidManifest(
                "invalid pitr header".to_owned(),
            ));
        }
        let mut records = Vec::new();
        let mut previous = 0;
        for line in lines {
            let parts: Vec<_> = line.split('\t').collect();
            if parts.len() != 5 {
                return Err(BackupError::InvalidManifest(
                    "invalid pitr entry".to_owned(),
                ));
            }
            let sequence = parts[1]
                .parse::<u64>()
                .map_err(|_| BackupError::InvalidManifest("invalid pitr sequence".to_owned()))?;
            if sequence <= previous {
                return Err(BackupError::InvalidManifest(
                    "pitr sequence must be strictly increasing".to_owned(),
                ));
            }
            previous = sequence;
            let key = String::from_utf8(decode_hex(parts[2])?)
                .map_err(|_| BackupError::InvalidManifest("invalid pitr key".to_owned()))?;
            let checksum_value = parts[3]
                .parse::<u64>()
                .map_err(|_| BackupError::InvalidManifest("invalid pitr checksum".to_owned()))?;
            let value = match parts[0] {
                "put" => {
                    let value = decode_hex(parts[4])?;
                    if checksum(&[&value]) != checksum_value {
                        return Err(BackupError::CorruptObject(format!("pitr/{sequence}")));
                    }
                    Some(value)
                }
                "delete" if checksum_value == 0 && parts[4].is_empty() => None,
                _ => return Err(BackupError::InvalidManifest("invalid pitr op".to_owned())),
            };
            records.push(PitrRecord {
                sequence,
                key,
                value,
            });
        }
        Ok(Self { records })
    }
}

/// Write a PITR log object and return its key.
pub fn write_pitr_log<S>(
    store: &mut S,
    backup_id: &str,
    log: &PitrLog,
) -> Result<String, BackupError>
where
    S: ObjectStore,
{
    validate_name(backup_id, "backup id")?;
    let key = format!("backups/{backup_id}/pitr-log");
    store.put(&key, log.encode()?)?;
    Ok(key)
}
