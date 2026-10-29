//! Read-only identities of retained roles. No descriptor or execution handle.
use super::*;

impl PinnedRoles {
    pub(crate) fn projection_identities(&self) -> Result<[[u64; 3]; 2], ArtifactError> {
        identities([&self.host, &self.launcher], self.refused)
    }
}
fn identities(roles: [&Role; 2], refused: bool) -> Result<[[u64; 3]; 2], ArtifactError> {
    if refused {
        return Err(ArtifactError::Contents);
    }
    Ok(roles.map(|role| {
        [
            role.stamp.identity[0],
            role.stamp.identity[1],
            role.stamp.length,
        ]
    }))
}

#[cfg(test)]
#[test]
fn held_unit_projection_role_views_keep_order_and_refuse_poisoned_owner() {
    let role = || {
        let file = tempfile::tempfile().unwrap();
        Role {
            stamp: Stamp::from(file.metadata().unwrap()),
            file,
        }
    };
    let host = role();
    let launcher = role();
    let expected = [host.stamp.identity, launcher.stamp.identity];
    let got = identities([&host, &launcher], false).unwrap();
    assert_eq!(
        [got[0][..2].to_vec(), got[1][..2].to_vec()],
        expected.map(|x| x.to_vec())
    );
    assert!(identities([&host, &launcher], true).is_err());
}
