//! Retained content identities only. The caller still owes fresh full frames.
use super::*;

#[path = "diagnostic_bundle_material.rs"]
mod material;

#[path = "diagnostic_bundle_role_census.rs"]
mod role_census;

impl BundleSnapshot {
    pub(crate) fn projection_contents(
        &self,
        surface: &str,
    ) -> Result<[([u64; 3], ArtifactDigest); 2], ArtifactError> {
        contents(&self.files, surface)
    }
}
fn contents(
    files: &BTreeMap<String, PinnedFile>,
    surface: &str,
) -> Result<[([u64; 3], ArtifactDigest); 2], ArtifactError> {
    if !SURFACES.contains(&surface) {
        return Err(ArtifactError::Contents);
    }
    let get = |name: &str| -> Result<([u64; 3], ArtifactDigest), ArtifactError> {
        let file = files.get(name).ok_or(ArtifactError::Contents)?;
        Ok((
            [file.stamp.dev, file.stamp.ino, file.stamp.len],
            file.digest.clone(),
        ))
    };
    Ok([
        get("timing-controls-074")?,
        get(&format!("{surface}.json"))?,
    ])
}

#[cfg(test)]
#[test]
fn held_unit_projection_bundle_views_are_fixed_surface_and_retained_content_only() {
    let mut files = BTreeMap::new();
    assert!(contents(&files, "embedded").is_err());
    for (name, raw) in [
        ("timing-controls-074", b"worker".as_slice()),
        ("embedded.json", b"config"),
    ] {
        let file = tempfile::tempfile().unwrap();
        let stamp = Stamp::from(file.metadata().unwrap());
        files.insert(
            name.into(),
            PinnedFile {
                file,
                stamp,
                digest: ArtifactDigest::of(raw),
            },
        );
    }
    let got = contents(&files, "embedded").unwrap();
    assert_eq!(got[0].1, ArtifactDigest::of(b"worker"));
    assert_eq!(got[1].1, ArtifactDigest::of(b"config"));
    assert_ne!(got[0].0, got[1].0);
    for surface in ["", "../embedded", "embedded.json", "direct"] {
        assert!(contents(&files, surface).is_err());
    }
    assert_eq!(files.len(), 2);
}
