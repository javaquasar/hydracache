//! Private exact receipt-statement comparison; no descriptor or authority export.
use super::*;
impl PinnedRoles {
    pub(crate) fn revalidate_build_join(
        &mut self,
        current: &VerifiedBuild,
        expected: &LaunchArtifacts,
    ) -> Result<(), ArtifactError> {
        let result = (|| {
            if canonical_json(&self.build.statement)? != canonical_json(&current.statement)? {
                return Err(ArtifactError::Contents);
            }
            self.revalidate(expected)
        })();
        if result.is_err() {
            self.refused = true;
        }
        result
    }
    pub(crate) fn refuse_build_join(&mut self) {
        self.refused = true;
    }
}
