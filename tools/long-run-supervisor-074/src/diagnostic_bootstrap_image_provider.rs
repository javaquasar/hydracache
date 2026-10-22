//! Original role contribution only. No descriptor getter, copy or callback.
use super::*;
use crate::diagnostic_artifacts::linux::held_role_census::{
    bootstrap_images::{self, BootstrapImageBindings, OriginalFdSet},
    CompleteBindings,
};

impl PinnedRoles {
    pub(crate) fn bootstrap_image_census(
        &mut self,
        prior: &CompleteBindings,
        original: OriginalFdSet,
        bootstrap: &File,
        expected: [[u64; 3]; 3],
    ) -> Result<BootstrapImageBindings, ArtifactError> {
        let result = (|| {
            if self.refused
                || Stamp::from(self.host.file.metadata()?) != self.host.stamp
                || Stamp::from(self.launcher.file.metadata()?) != self.launcher.stamp
            {
                return Err(ArtifactError::Contents);
            }
            let census = bootstrap_images::collect(
                prior,
                original,
                [&self.host.file, &self.launcher.file, bootstrap],
                expected,
            )?;
            if Stamp::from(self.host.file.metadata()?) != self.host.stamp
                || Stamp::from(self.launcher.file.metadata()?) != self.launcher.stamp
            {
                return Err(ArtifactError::Contents);
            }
            Ok(census)
        })();
        if result.is_err() {
            self.refused = true;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic_artifacts::linux::held_role_census::bootstrap_images::tests::Fixture;
    fn roles(host: File, launcher: File) -> PinnedRoles {
        // This is deliberately a modeled previous receipt, never verified build
        // or original-owner evidence. Only the provider's new source checks run.
        let body = serde_json::json!({"schema_version":"model","purpose":"model","policy_sha256":"","authority_epoch":0,
            "observation":{"source_commit_before":"","source_commit_after":"","source_tree_before":"","source_tree_after":"",
                "source_clean_before":false,"source_clean_after":false,"isolated_build":false,"rustc_version":"","cargo_version":"","build_command":[]},
            "roles":{"launcher":"model","watchdog":"model","collector":"model"},
            "images":{"original_host":{"sha256":"","bytes":0},"held_launcher":{"sha256":"","bytes":0}},
            "root_lock":{"sha256":"","bytes":0},"build_log":{"sha256":"","bytes":0}});
        PinnedRoles {
            build: VerifiedBuild {
                statement: serde_json::from_value(body).unwrap(),
            },
            host: Role {
                stamp: Stamp::from(host.metadata().unwrap()),
                file: host,
            },
            launcher: Role {
                stamp: Stamp::from(launcher.metadata().unwrap()),
                file: launcher,
            },
            refused: false,
        }
    }
    #[test]
    fn bootstrap_image_provider_model_retains_original_roles_without_descriptor_export() {
        let f = Fixture::new();
        let expected = f.identities();
        let prior = f.prior();
        let original = f.original();
        let Fixture {
            old,
            images: [host, launcher, bootstrap],
        } = f;
        let mut roles = roles(host, launcher);
        let first = roles
            .bootstrap_image_census(&prior, original, &bootstrap, expected)
            .unwrap();
        let original =
            OriginalFdSet::from_material(std::array::from_fn(|i| old[i].as_raw_fd())).unwrap();
        assert!(
            roles
                .bootstrap_image_census(&prior, original, &bootstrap, expected)
                .unwrap()
                == first
        );
    }
    #[test]
    fn bootstrap_image_provider_model_wrong_identity_and_original_mode_are_sticky() {
        use std::os::unix::fs::PermissionsExt;
        for mode in [false, true] {
            let f = Fixture::new();
            let expected = f.identities();
            let prior = f.prior();
            let original = f.original();
            let Fixture {
                old,
                images: [host, launcher, bootstrap],
            } = f;
            let mut roles = roles(host, launcher);
            let mut bad = expected;
            if mode {
                roles
                    .host
                    .file
                    .set_permissions(std::fs::Permissions::from_mode(0o400))
                    .unwrap();
            } else {
                bad[0][1] += 1;
            }
            assert!(roles
                .bootstrap_image_census(&prior, original, &bootstrap, bad)
                .is_err());
            assert!(roles.refused);
            let original =
                OriginalFdSet::from_material(std::array::from_fn(|i| old[i].as_raw_fd())).unwrap();
            assert!(roles
                .bootstrap_image_census(&prior, original, &bootstrap, expected)
                .is_err());
        }
    }
}
