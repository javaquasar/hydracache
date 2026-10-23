//! Parent-local fingerprint and retained entry role; never an inherited FD map.
use super::*;
use sha2::{Digest, Sha256};

#[derive(PartialEq, Eq)]
pub(crate) struct EntryBindings {
    roles: [i32; 15],
    fds: [i32; 27],
    entry: Stamp,
}
impl BootstrapImageBindings {
    pub(crate) fn owner_fingerprint(&self) -> [u8; 32] {
        let mut hash = Sha256::new();
        hash.update(b"hydracache-parent-bootstrap-census-074-v1\0");
        for v in self.roles.iter().chain(&self.fds) {
            hash.update(v.to_le_bytes());
        }
        for s in &self.images {
            hash.update(s.fd.to_le_bytes());
            for v in s.stat {
                hash.update(v.to_le_bytes());
            }
            for v in s.flags {
                hash.update(v.to_le_bytes());
            }
            hash.update([u8::from(s.offset.is_some())]);
            hash.update(s.offset.unwrap_or_default().to_le_bytes());
        }
        hash.finalize().into()
    }
    pub(crate) fn append_entry(&self, file: &File) -> io::Result<EntryBindings> {
        let m = file.metadata()?;
        need(m.len() == 1024)?;
        let entry = image(file, [m.dev(), m.ino(), m.len()])?;
        let mut fds = [-1; 27];
        fds[..26].copy_from_slice(&self.fds);
        fds[26] = entry.fd;
        need(distinct(&fds))?;
        // Reject any inode alias, including legacy/config/worker, not only images.
        for fd in self.fds {
            let mut s: libc::stat = unsafe { std::mem::zeroed() };
            need(
                unsafe { libc::fstat(fd, &mut s) } == 0 && [s.st_dev, s.st_ino] != entry.stat[..2],
            )?;
        }
        let mut roles = [-1; 15];
        roles[..14].copy_from_slice(&self.roles);
        roles[14] = entry.fd;
        need(stamp(file)? == entry)?;
        Ok(EntryBindings { roles, fds, entry })
    }
}

#[path = "diagnostic_bootstrap_installed_census.rs"]
pub(crate) mod installed;

#[cfg(test)]
mod tests {
    use super::super::tests::Fixture;
    use super::*;
    use crate::diagnostic_builder::bootstrap_build::tests::sealed_image;
    #[test]
    fn bootstrap_entry_census_preserves_prefix_rejects_alias_and_tracks_drift() {
        let f = Fixture::new();
        let base = collect(
            &f.prior(),
            f.original(),
            f.images.each_ref(),
            f.identities(),
        )
        .unwrap();
        let entry = sealed_image(
            &[9; 1024],
            libc::F_SEAL_SEAL | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_WRITE,
        );
        let before = base.append_entry(&entry).unwrap();
        assert_eq!(before.fds[..26], base.fds);
        assert_eq!(before.roles[..14], base.roles);
        assert!(base.append_entry(&f.images[0]).is_err());
        let mut collision = base;
        collision.fds[0] = entry.as_raw_fd();
        assert!(collision.append_entry(&entry).is_err());
        collision.fds[0] = f.old[0].as_raw_fd();
        let alias = entry.try_clone().unwrap();
        collision.fds[0] = alias.as_raw_fd();
        assert!(collision.append_entry(&entry).is_err());
        collision.fds[0] = f.old[0].as_raw_fd();
        let hash = collision.owner_fingerprint();
        collision.roles[0] += 1;
        assert_ne!(hash, collision.owner_fingerprint());
        assert_eq!(
            unsafe { libc::lseek(entry.as_raw_fd(), 1, libc::SEEK_SET) },
            1
        );
        assert!(collision.append_entry(&entry).unwrap() != before);
    }
}
