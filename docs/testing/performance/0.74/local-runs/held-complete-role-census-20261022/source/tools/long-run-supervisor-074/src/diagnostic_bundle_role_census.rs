//! Fixed original worker contribution, never a descriptor getter.
use super::*;
use crate::diagnostic_artifacts::linux::held_role_census::Census;

impl BundleSnapshot {
    pub(crate) fn contribute_held_worker(&self, census: &mut Census) -> Result<(), ArtifactError> {
        contribute(&self.files, census)
    }
}
fn contribute(
    files: &BTreeMap<String, PinnedFile>,
    census: &mut Census,
) -> Result<(), ArtifactError> {
    let result = (|| {
        let file = files
            .get("timing-controls-074")
            .ok_or(ArtifactError::Contents)?;
        if Stamp::from(file.file.metadata()?) != file.stamp {
            return Err(ArtifactError::Contents);
        }
        census.worker(&file.file, &file.digest.sha256)?;
        Ok(())
    })();
    if result.is_err() {
        census.reject();
    }
    result
}

#[cfg(test)]
#[test]
fn held_complete_census_bundle_provider_uses_only_original_worker_and_poisoned_failures_stay_refused(
) {
    use crate::diagnostic_artifacts::linux::held_role_census::tests::Fixture;
    let f = Fixture::new();
    let file = f.worker.try_clone().unwrap();
    let mut files = BTreeMap::from([(
        "timing-controls-074".into(),
        PinnedFile {
            stamp: Stamp::from(file.metadata().unwrap()),
            file,
            digest: ArtifactDigest::of(b"worker"),
        },
    )]);
    let mut census = f.census();
    contribute(&files, &mut census).unwrap();
    assert!(contribute(&files, &mut census).is_err());
    let mut census = f.census();
    assert!(contribute(&BTreeMap::new(), &mut census).is_err());
    assert!(contribute(&files, &mut census).is_err());
    let mut census = f.census();
    files.get_mut("timing-controls-074").unwrap().stamp.len += 1;
    assert!(contribute(&files, &mut census).is_err());
    files.get_mut("timing-controls-074").unwrap().stamp.len -= 1;
    assert!(contribute(&files, &mut census).is_err());
}
