//! Only the same consumed, Armed pair contributes; no pidfd leaves this owner.
use super::*;
use crate::diagnostic_artifacts::linux::held_role_census::Census;

impl BoundGuardians<'_> {
    pub(crate) fn contribute_held_guardians(&self, census: &mut Census) -> io::Result<()> {
        contribute(
            [self.pair.consumed, self.pair.armed, self.pair.refused],
            self.pair.guards.each_ref().map(|g| (g.pid, &g.pin)),
            census,
        )
    }
}
fn contribute(flags: [bool; 3], files: [(i32, &File); 2], census: &mut Census) -> io::Result<()> {
    let result = (|| {
        if flags != [true, true, false] {
            return Err(refused());
        }
        census.guardians(files)
    })();
    if result.is_err() {
        census.reject();
    }
    result
}

#[cfg(test)]
#[test]
fn held_complete_census_guardian_provider_requires_consumed_armed_unrefused_pair() {
    use crate::diagnostic_artifacts::linux::held_role_census::tests::Fixture;
    let f = Fixture::new();
    for bits in 0..8 {
        let flags = [bits & 1 != 0, bits & 2 != 0, bits & 4 != 0];
        let mut census = f.census();
        census
            .worker(&f.worker, &crate::sha256_hex(b"worker"))
            .unwrap();
        if flags == [true, true, false] {
            contribute(flags, f.files(), &mut census).unwrap();
        } else {
            assert!(contribute(flags, f.files(), &mut census).is_err());
            assert!(contribute([true, true, false], f.files(), &mut census).is_err());
        }
    }
}
