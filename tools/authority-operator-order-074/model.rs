//! Whole original bodies with synthetic receivers; no storage candidate.
use crate::{open_at, AuthorityError, File, Metadata};
use std::io::Read;

// BEGIN STAMP CORE
#[derive(Clone, PartialEq, Eq)]
struct Stamp {
    dev: u64,
    ino: u64,
    uid: u32,
    gid: u32,
    mode: u32,
    links: u64,
    len: u64,
    mtime: i64,
    mtime_ns: i64,
    ctime: i64,
    ctime_ns: i64,
}
impl Stamp {
    fn of(m: &Metadata) -> Self {
        Self {
            dev: m.dev(),
            ino: m.ino(),
            uid: m.uid(),
            gid: m.gid(),
            mode: m.mode(),
            links: m.nlink(),
            len: m.len(),
            mtime: m.mtime(),
            mtime_ns: m.mtime_nsec(),
            ctime: m.ctime(),
            ctime_ns: m.ctime_nsec(),
        }
    }
}
// END STAMP CORE

// BEGIN LEAF CORE
struct Leaf {
    file: File,
    stamp: Stamp,
    raw: Vec<u8>,
    name: String,
}
impl Leaf {
    fn open(
        dir: &File,
        name: String,
        uid: u32,
        gid: u32,
        budget: usize,
    ) -> Result<Self, AuthorityError> {
        let (file, stamp, raw) = Self::read(dir, &name, uid, gid, budget)?;
        Ok(Self {
            file,
            stamp,
            raw,
            name,
        })
    }
    fn read(
        dir: &File,
        name: &str,
        uid: u32,
        gid: u32,
        budget: usize,
    ) -> Result<(File, Stamp, Vec<u8>), AuthorityError> {
        let mut file = open_at(dir, name, false)?;
        let before = file.metadata()?;
        regular(&before, uid, gid, 0o400)?;
        if before.dev() != dir.metadata()?.dev() || before.len() > budget as u64 {
            return Err(AuthorityError::Budget);
        }
        let stamp = Stamp::of(&before);
        let mut raw = Vec::with_capacity(before.len() as usize);
        (&mut file).take(budget as u64 + 1).read_to_end(&mut raw)?;
        if Stamp::of(&file.metadata()?) != stamp || raw.len() as u64 != stamp.len {
            return Err(AuthorityError::Invalid);
        }
        Ok((file, stamp, raw))
    }
    fn revalidate(
        &self,
        dir: &File,
        uid: u32,
        gid: u32,
        budget: usize,
    ) -> Result<(), AuthorityError> {
        if Stamp::of(&self.file.metadata()?) != self.stamp {
            return Err(AuthorityError::Invalid);
        }
        // Keep the fresh File (tuple field zero) alive through both comparisons.
        let named = Self::read(dir, &self.name, uid, gid, budget)?;
        if named.1 != self.stamp || named.2 != self.raw {
            return Err(AuthorityError::Invalid);
        }
        Ok(())
    }
}
// END LEAF CORE

// BEGIN REGULAR CORE
fn regular(m: &Metadata, uid: u32, gid: u32, mode: u32) -> Result<(), AuthorityError> {
    if !m.is_file()
        || m.uid() != uid
        || m.gid() != gid
        || m.mode() & 0o7777 != mode
        || m.nlink() != 1
    {
        return Err(AuthorityError::Invalid);
    }
    Ok(())
}
// END REGULAR CORE

#[path = "../authority-owned-journal-074/controls.rs"]
mod controls;
#[path = "journal.rs"]
mod journal;

#[path = "../authority-owned-journal-074/owned.rs"]
mod owned;
#[path = "../authority-owned-journal-074/owned_controls.rs"]
mod owned_controls;

#[path = "operator_controls.rs"]
mod operator_controls;
