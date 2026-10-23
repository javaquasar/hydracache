//! Value factory and one retained record role; no descriptor or owner export.
use super::*;
use crate::diagnostic_builder::bootstrap_build::{
    entry_format::AssertedEntry, installed_entry_format::AssertedInstalledEntry,
};
use sha2::{Digest, Sha256};

#[derive(PartialEq, Eq)]
pub(crate) struct InstalledEntryBindings {
    roles: [i32; 17],
    fds: [i32; 33],
    record: Stamp,
}
impl InstalledBindings {
    // The caller's original complete frames authenticate these retained values.
    // This value projection cannot attest a fresh source or turn assertions into authority.
    pub(crate) fn entry_assertions(
        &self,
        raw: &[u8; 1024],
        prior: &EntryBindings,
    ) -> io::Result<[u8; 1280]> {
        need(self.roles[..15] == prior.roles && self.fds[..27] == prior.fds)?;
        need(
            distinct(&self.fds)
                && self.roles[15] == self.installed.fd
                && self.fds[31] == self.installed.fd,
        )?;
        for (at, value) in self.ancestry.iter().enumerate() {
            need(self.fds[27 + at] == value.fd)?;
        }
        let original = AssertedEntry::decode(raw).map_err(|_| refused())?;
        let mut hash = Sha256::new();
        hash.update(b"hydracache-parent-installed-bootstrap-census-074-v1\0");
        for value in self.roles.iter().chain(&self.fds) {
            hash.update(value.to_le_bytes());
        }
        for value in std::iter::once(&prior.entry)
            .chain(std::iter::once(&self.installed))
            .chain(&self.ancestry)
        {
            hash.update(value.fd.to_le_bytes());
            for v in value.stat {
                hash.update(v.to_le_bytes());
            }
            for v in value.flags {
                hash.update(v.to_le_bytes());
            }
            hash.update([u8::from(value.offset.is_some())]);
            hash.update(value.offset.unwrap_or_default().to_le_bytes());
        }
        let value = AssertedInstalledEntry {
            installed_identity: [
                self.installed.stat[0],
                self.installed.stat[1],
                self.installed.stat[6],
            ],
            installed_sha256: original.image_hashes[2],
            original,
            parent_inventory_sha256: hash.finalize().into(),
        };
        value.encode().map_err(|_| refused())
    }
    pub(crate) fn append_installed_entry(&self, file: &File) -> io::Result<InstalledEntryBindings> {
        let m = file.metadata()?;
        need(m.len() == 1280)?;
        let record = image(file, [m.dev(), m.ino(), m.len()])?;
        need(record.offset == Some(0))?;
        let mut fds = [-1; 33];
        fds[..32].copy_from_slice(&self.fds);
        fds[32] = record.fd;
        need(distinct(&fds))?;
        for fd in self.fds {
            let mut stat: libc::stat = unsafe { std::mem::zeroed() };
            need(
                unsafe { libc::fstat(fd, &mut stat) } == 0
                    && [stat.st_dev, stat.st_ino] != record.stat[..2],
            )?;
        }
        let mut roles = [-1; 17];
        roles[..16].copy_from_slice(&self.roles);
        roles[16] = record.fd;
        need(stamp(file)? == record)?;
        Ok(InstalledEntryBindings { roles, fds, record })
    }
}

#[cfg(test)]
#[path = "diagnostic_bootstrap_installed_entry_census_tests.rs"]
pub(crate) mod tests;

impl InstalledBindings {
    pub(crate) fn sandbox_image_identity(&self) -> [u64; 3] {
        [
            self.installed.stat[0],
            self.installed.stat[1],
            self.installed.stat[6],
        ]
    }
}
