//! Same original sealed resource only. No caller bytes, FD getter or transport.
use super::*;
use crate::diagnostic_artifacts::linux::held_role_census::bootstrap_images::entry::installed::{
    entry_material::InstalledEntryBindings, InstalledBindings,
};

fn expected(
    original: &EntryResource,
    base: &BootstrapImageBindings,
    prior: &EntryBindings,
    installed: &InstalledBindings,
) -> io::Result<[u8; 1280]> {
    need(original.retained.position == Some(0))?;
    need(stamp(original.file.as_raw_fd())? == original.retained)?;
    readonly(&original.file)?;
    need(base.append_entry(&original.file)? == *prior)?;
    let raw: [u8; 1024] = read(&original.file, 1024)?
        .try_into()
        .map_err(|_| refused())?;
    let result = installed.entry_assertions(&raw, prior)?;
    need(base.append_entry(&original.file)? == *prior)?;
    readonly(&original.file)?;
    need(stamp(original.file.as_raw_fd())? == original.retained)?;
    Ok(result)
}

pub(in super::super::super) struct InstalledEntryResource {
    file: File,
    retained: Stamp,
    refused: bool,
}
impl InstalledEntryResource {
    pub(in super::super::super) fn create(
        original: &EntryResource,
        base: &BootstrapImageBindings,
        prior: &EntryBindings,
        installed: &InstalledBindings,
    ) -> io::Result<Self> {
        let raw = expected(original, base, prior, installed)?;
        let file = sealed(&raw)?;
        let retained = stamp(file.as_raw_fd())?;
        need(retained.position == Some(0))?;
        Ok(Self {
            file,
            retained,
            refused: false,
        })
    }
    pub(in super::super::super) fn inspect(
        &mut self,
        original: &EntryResource,
        base: &BootstrapImageBindings,
        prior: &EntryBindings,
        installed: &InstalledBindings,
    ) -> io::Result<InstalledEntryBindings> {
        let result = (|| {
            need(!self.refused)?;
            let raw = expected(original, base, prior, installed)?;
            need(stamp(self.file.as_raw_fd())? == self.retained)?;
            readonly(&self.file)?;
            need(read(&self.file, 1280)? == raw)?;
            let result = installed.append_installed_entry(&self.file)?;
            need(expected(original, base, prior, installed)? == raw)?;
            readonly(&self.file)?;
            need(stamp(self.file.as_raw_fd())? == self.retained)?;
            Ok(result)
        })();
        if result.is_err() {
            self.refused = true;
        }
        result
    }
}

#[cfg(test)]
#[path = "diagnostic_bootstrap_installed_entry_resource_tests.rs"]
mod tests;
