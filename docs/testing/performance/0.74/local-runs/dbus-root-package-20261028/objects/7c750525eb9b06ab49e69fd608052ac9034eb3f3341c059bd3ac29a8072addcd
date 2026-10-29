//! Bounded positional copy from the retained config, never a file/FD extractor.
use super::*;
use std::os::unix::fs::FileExt;

impl BundleSnapshot {
    pub(crate) fn copy_material_config(
        &self,
        surface: &str,
        identity: [u64; 3],
        digest: &str,
    ) -> Result<Vec<u8>, ArtifactError> {
        if !SURFACES.contains(&surface) {
            return Err(ArtifactError::Contents);
        }
        let file = self
            .files
            .get(&format!("{surface}.json"))
            .ok_or(ArtifactError::Contents)?;
        copy(file, identity, digest)
    }
}
fn copy(file: &PinnedFile, identity: [u64; 3], digest: &str) -> Result<Vec<u8>, ArtifactError> {
    let m = file.file.metadata()?;
    if Stamp::from(m.clone()) != file.stamp
        || identity != [m.dev(), m.ino(), m.len()]
        || file.digest.sha256 != digest
        || m.len() == 0
        || m.len() > MAX_CONFIG_BYTES
    {
        return Err(ArtifactError::Contents);
    }
    let mut raw = vec![0; m.len() as usize];
    file.file.read_exact_at(&mut raw, 0)?;
    if ArtifactDigest::of(&raw) != file.digest || Stamp::from(file.file.metadata()?) != file.stamp {
        return Err(ArtifactError::Contents);
    }
    Ok(raw)
}

#[cfg(test)]
#[test]
fn held_material_config_copy_is_positional_exact_and_rejects_substitution() {
    use std::io::{Seek, SeekFrom, Write};
    let mut f = tempfile::tempfile().unwrap();
    f.write_all(b"config\n").unwrap();
    f.seek(SeekFrom::Start(3)).unwrap();
    let m = f.metadata().unwrap();
    let identity = [m.dev(), m.ino(), m.len()];
    let mut pinned = PinnedFile {
        file: f,
        stamp: Stamp::from(m),
        digest: ArtifactDigest::of(b"config\n"),
    };
    let digest = pinned.digest.sha256.clone();
    assert_eq!(copy(&pinned, identity, &digest).unwrap(), b"config\n");
    assert_eq!(pinned.file.stream_position().unwrap(), 3);
    for at in 0..3 {
        let mut bad = identity;
        bad[at] += 1;
        assert!(copy(&pinned, bad, &digest).is_err());
    }
    assert!(copy(&pinned, identity, &"0".repeat(64)).is_err());
    pinned.file.write_all(b"x").unwrap();
    assert!(copy(&pinned, identity, &digest).is_err());
}
