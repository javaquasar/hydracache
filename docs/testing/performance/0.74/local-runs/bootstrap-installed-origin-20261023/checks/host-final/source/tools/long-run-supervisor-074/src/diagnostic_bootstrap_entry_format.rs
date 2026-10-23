//! Untrusted assertions only. A canonical record is NOT a permit or authentication.
pub const BYTES: usize = 1024;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssertedEntry {
    pub owner: [u64; 6],
    pub origins: [u64; 4],
    pub bounds: [u64; 4],
    pub attempt: [u8; 32],
    pub original_context: [u8; 32],
    pub root: [u64; 3],
    pub serial: u32,
    pub names: [[u8; 32]; 2],
    pub documents: [[u8; 32]; 7],
    // host, launcher, bootstrap, worker, SEALED config, legacy record.
    pub identities: [[u64; 3]; 6],
    // stdin, stdout, stderr, child-facing release channel.
    pub streams: [[u64; 2]; 4],
    pub guardians: [[u64; 3]; 2],
    // host, launcher, bootstrap, worker, config.
    pub image_hashes: [[u8; 32]; 5],
    pub legacy_hash: [u8; 32],
    pub census_hash: [u8; 32],
    pub state_hash: [u8; 32],
}
#[derive(Debug, thiserror::Error)]
#[error("invalid bootstrap entry assertion shape")]
pub struct InvalidEntry;
fn need(value: bool) -> Result<(), InvalidEntry> {
    if value {
        Ok(())
    } else {
        Err(InvalidEntry)
    }
}
impl AssertedEntry {
    fn shape(&self) -> Result<(), InvalidEntry> {
        need(
            self.owner.iter().all(|v| *v <= u32::MAX as u64)
                && self.owner[0] > 1
                && self.owner[1] > 1
                && self.owner[..2].iter().all(|v| *v <= i32::MAX as u64)
                && self.serial != 0,
        )?;
        need(
            self.bounds[..3]
                .iter()
                .all(|v| *v > 0 && *v <= i64::MAX as u64)
                && self.bounds[3] == *self.bounds[..3].iter().min().ok_or(InvalidEntry)?
                && self.root[2] == self.bounds[3]
                && self.root[..2].iter().all(|v| *v > 0),
        )?;
        need(
            self.identities.iter().all(|id| id.iter().all(|v| *v > 0))
                && self.identities[5][2] == 256
                && self.streams.iter().all(|id| id.iter().all(|v| *v > 0)),
        )?;
        need(
            self.guardians.iter().all(|g| {
                g[0] > 1 && g[0] <= i32::MAX as u64 && g[0] != self.owner[0] && g[1] > 0 && g[2] > 0
            }) && self.guardians[0][0] != self.guardians[1][0],
        )
    }
    pub fn encode(&self) -> Result<[u8; BYTES], InvalidEntry> {
        self.shape()?;
        let mut raw = [0; BYTES];
        raw[..8].copy_from_slice(b"HCBT0741");
        raw[8..12].copy_from_slice(&(BYTES as u32).to_le_bytes());
        fn words(raw: &mut [u8], at: usize, words: &[u64]) {
            for (i, v) in words.iter().enumerate() {
                raw[at + i * 8..at + i * 8 + 8].copy_from_slice(&v.to_le_bytes());
            }
        }
        words(&mut raw, 16, &self.owner);
        words(&mut raw, 64, &self.origins);
        words(&mut raw, 96, &self.bounds);
        raw[128..160].copy_from_slice(&self.attempt);
        raw[160..192].copy_from_slice(&self.original_context);
        words(&mut raw, 192, &self.root);
        raw[216..220].copy_from_slice(&self.serial.to_le_bytes());
        for (i, hash) in self.names.iter().chain(&self.documents).enumerate() {
            raw[224 + i * 32..256 + i * 32].copy_from_slice(hash);
        }
        for (i, id) in self.identities.iter().enumerate() {
            words(&mut raw, 512 + i * 24, id);
        }
        for (i, id) in self.streams.iter().enumerate() {
            words(&mut raw, 656 + i * 16, id);
        }
        for (i, id) in self.guardians.iter().enumerate() {
            words(&mut raw, 720 + i * 24, id);
        }
        for (i, hash) in self
            .image_hashes
            .iter()
            .chain([&self.legacy_hash, &self.census_hash, &self.state_hash])
            .enumerate()
        {
            raw[768 + i * 32..800 + i * 32].copy_from_slice(hash);
        }
        Ok(raw)
    }
    pub fn decode(raw: &[u8]) -> Result<Self, InvalidEntry> {
        need(raw.len() == BYTES)?;
        need(
            &raw[..8] == b"HCBT0741"
                && raw[8..12] == (BYTES as u32).to_le_bytes()
                && raw[12..16] == [0; 4]
                && raw[220..224] == [0; 4],
        )?;
        fn words<const N: usize>(raw: &[u8], at: usize) -> [u64; N] {
            std::array::from_fn(|i| {
                u64::from_le_bytes(
                    raw[at + i * 8..at + i * 8 + 8]
                        .try_into()
                        .expect("fixed shape"),
                )
            })
        }
        fn hash(raw: &[u8], at: usize) -> [u8; 32] {
            raw[at..at + 32].try_into().expect("fixed shape")
        }
        let value = Self {
            owner: words(raw, 16),
            origins: words(raw, 64),
            bounds: words(raw, 96),
            attempt: hash(raw, 128),
            original_context: hash(raw, 160),
            root: words(raw, 192),
            serial: u32::from_le_bytes(raw[216..220].try_into().expect("fixed shape")),
            names: std::array::from_fn(|i| hash(raw, 224 + i * 32)),
            documents: std::array::from_fn(|i| hash(raw, 288 + i * 32)),
            identities: std::array::from_fn(|i| words(raw, 512 + i * 24)),
            streams: std::array::from_fn(|i| words(raw, 656 + i * 16)),
            guardians: std::array::from_fn(|i| words(raw, 720 + i * 24)),
            image_hashes: std::array::from_fn(|i| hash(raw, 768 + i * 32)),
            legacy_hash: hash(raw, 928),
            census_hash: hash(raw, 960),
            state_hash: hash(raw, 992),
        };
        value.shape()?;
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    pub(super) fn fixture() -> AssertedEntry {
        AssertedEntry {
            owner: [74, 75, 0, 0, 0, 0],
            origins: [1, 2, 3, 4],
            bounds: [300, 60, 10, 10],
            attempt: [1; 32],
            original_context: [2; 32],
            root: [1, 2, 10],
            serial: 1,
            names: [[3; 32]; 2],
            documents: [[4; 32]; 7],
            identities: std::array::from_fn(|i| [1, i as u64 + 2, if i == 5 { 256 } else { 128 }]),
            streams: [[1, 2]; 4],
            guardians: [[76, 1, 3], [77, 1, 4]],
            image_hashes: [[5; 32]; 5],
            legacy_hash: [6; 32],
            census_hash: [7; 32],
            state_hash: [8; 32],
        }
    }
    #[test]
    fn bootstrap_entry_format_roundtrip_and_every_byte_is_canonical_not_authenticated() {
        let entry = fixture();
        let raw = entry.encode().unwrap();
        assert_eq!(AssertedEntry::decode(&raw).unwrap(), entry);
        for at in 0..BYTES {
            let mut changed = raw;
            changed[at] ^= 1;
            if let Ok(parsed) = AssertedEntry::decode(&changed) {
                assert_ne!(parsed, entry);
                assert_eq!(parsed.encode().unwrap(), changed);
            }
        }
    }
    #[test]
    fn bootstrap_entry_format_refuses_length_padding_version_and_deadline_reset() {
        let raw = fixture().encode().unwrap();
        for n in [0, 1, 256, 1023, 1025, 2048] {
            let mut bad = raw.to_vec();
            bad.resize(n, 0);
            assert!(AssertedEntry::decode(&bad).is_err());
        }
        for at in [0, 7, 8, 11, 12, 15, 220, 223] {
            let mut bad = raw;
            bad[at] ^= 1;
            assert!(AssertedEntry::decode(&bad).is_err());
        }
        let mut e = fixture();
        e.bounds[3] += 1;
        assert!(e.encode().is_err());
        let mut e = fixture();
        e.root[2] += 1;
        assert!(e.encode().is_err());
        let mut e = fixture();
        e.serial = 0;
        assert!(e.encode().is_err());
        let mut e = fixture();
        e.identities[5][2] = 1024;
        assert!(e.encode().is_err());
    }
}
