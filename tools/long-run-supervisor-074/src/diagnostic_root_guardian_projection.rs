//! Snapshot of the SAME armed guardians, never pidfds or an Armed permit.
use super::*;

impl BoundGuardians<'_> {
    pub(crate) fn projection_guardians(&self) -> io::Result<[(i32, [u64; 2]); 2]> {
        identities(
            self.pair.guards.each_ref().map(|g| (g.pid, &g.pin)),
            self.pair.consumed && self.pair.armed && !self.pair.refused,
        )
    }
}
fn identities(guards: [(i32, &File); 2], armed: bool) -> io::Result<[(i32, [u64; 2]); 2]> {
    if !armed {
        return Err(refused());
    }
    let identity = |(pid, file): (i32, &File)| -> io::Result<(i32, [u64; 2])> {
        let metadata = file.metadata()?;
        Ok((pid, [metadata.dev(), metadata.ino()]))
    };
    Ok([identity(guards[0])?, identity(guards[1])?])
}

#[cfg(test)]
#[test]
fn held_unit_projection_guardian_views_require_armed_original_and_keep_order() {
    let a = tempfile::tempfile().unwrap();
    let b = tempfile::tempfile().unwrap();
    let guards = [(74, &a), (75, &b)];
    assert!(identities(guards, false).is_err());
    let got = identities(guards, true).unwrap();
    assert_eq!([got[0].0, got[1].0], [74, 75]);
    assert_ne!(got[0].1, got[1].1);
    assert!(identities(guards, false).is_err());
}
