//! Parent-local installed origin extension. Not a child FD map or permit.
use super::*;
#[derive(PartialEq, Eq)]
pub(crate) struct InstalledBindings {
    roles: [i32; 16],
    fds: [i32; 32],
    installed: Stamp,
    ancestry: [Stamp; 4],
}
impl EntryBindings {
    pub(crate) fn append_installed_origin(
        &self,
        directories: [&File; 4],
        file: &File,
    ) -> io::Result<InstalledBindings> {
        let installed = stamp(file)?;
        need(
            installed.stat[4] as u32 & libc::S_IFMT == libc::S_IFREG
                && installed.flags[1] & libc::O_ACCMODE == libc::O_RDONLY
                && installed.offset == Some(0),
        )?;
        let ancestry = [
            stamp(directories[0])?,
            stamp(directories[1])?,
            stamp(directories[2])?,
            stamp(directories[3])?,
        ];
        for d in &ancestry {
            need(d.stat[4] as u32 & libc::S_IFMT == libc::S_IFDIR)?;
        }
        let mut fds = [-1; 32];
        fds[..27].copy_from_slice(&self.fds);
        for (at, d) in ancestry.iter().enumerate() {
            fds[27 + at] = d.fd;
        }
        fds[31] = installed.fd;
        need(distinct(&fds))?;
        for fd in self.fds {
            let mut m: libc::stat = unsafe { std::mem::zeroed() };
            need(
                unsafe { libc::fstat(fd, &mut m) } == 0
                    && [m.st_dev, m.st_ino] != installed.stat[..2],
            )?;
        }
        let mut roles = [-1; 16];
        roles[..15].copy_from_slice(&self.roles);
        roles[15] = installed.fd;
        need(stamp(file)? == installed)?;
        for (d, retained) in directories.into_iter().zip(&ancestry) {
            need(stamp(d)? == *retained)?;
        }
        Ok(InstalledBindings {
            roles,
            fds,
            installed,
            ancestry,
        })
    }
}

#[cfg(test)]
#[path = "diagnostic_bootstrap_installed_census_tests.rs"]
mod tests;

#[path = "diagnostic_bootstrap_installed_entry_census.rs"]
pub(crate) mod entry_material;
