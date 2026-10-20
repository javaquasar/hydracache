//! Shared append-only engine and separate /tmp fixture entry points.
use super::{
    history, AuthorityError, AuthorityView, PublicationStage, Record, RefusalReason,
    COMMIT_MARKER_BYTES, MAX_GENERATIONS, MAX_RECORD_BYTES,
};
use crate::diagnostic_worker_policy::{AssertedWorkerHost, CheckedWorkerPolicy, WorkerPolicyTrust};
use crate::sha256_hex;
use fs2::FileExt;
use std::collections::BTreeSet;
use std::ffi::CString;
use std::fs::{self, File, Metadata};
use std::io::{Read, Write};
use std::marker::PhantomData;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Component, Path};
use std::rc::Rc;

#[path = "diagnostic_authority_fixed.rs"]
pub mod fixed;

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
    fn directory_matches(&self, m: &Metadata) -> bool {
        m.is_dir()
            && self.dev == m.dev()
            && self.ino == m.ino()
            && self.uid == m.uid()
            && self.gid == m.gid()
            && self.mode == m.mode()
    }
}
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
struct Entry {
    record: Leaf,
    marker: Leaf,
}
impl Entry {
    fn open(dir: &File, revision: usize, uid: u32, gid: u32) -> Result<Self, AuthorityError> {
        let record = Leaf::open(dir, name(revision, "json"), uid, gid, MAX_RECORD_BYTES)?;
        let marker = Leaf::open(dir, name(revision, "commit"), uid, gid, COMMIT_MARKER_BYTES)?;
        if marker.raw != marker_bytes(&record.raw) {
            return Err(AuthorityError::Invalid);
        }
        Ok(Self { record, marker })
    }
}
struct Directory {
    root: File,
    temporary: File,
    temporary_stamp: Stamp,
    leaf: File,
    leaf_stamp: Stamp,
    leaf_name: String,
    uid: u32,
    gid: u32,
    fixed_ancestry: Option<fixed::Ancestry>,
}
impl Drop for Directory {
    fn drop(&mut self) {
        // Closing alone can retain flock through a pre-exec inherited descriptor.
        // End this owner's cooperative lock explicitly; no retry or handover receipt.
        let _ = FileExt::unlock(&self.leaf);
    }
}
impl Directory {
    fn open(path: &Path, uid: u32, gid: u32) -> Result<Self, AuthorityError> {
        let mut parts = path.components();
        if parts.next() != Some(Component::RootDir)
            || parts.next() != Some(Component::Normal(std::ffi::OsStr::new("tmp")))
        {
            return Err(AuthorityError::Invalid);
        }
        let Some(Component::Normal(leaf_name)) = parts.next() else {
            return Err(AuthorityError::Invalid);
        };
        if parts.next().is_some() {
            return Err(AuthorityError::Invalid);
        }
        let leaf_name = leaf_name.to_str().ok_or(AuthorityError::Invalid)?;
        if !leaf_name.starts_with("hydracache-authority-")
            || leaf_name.len() <= 21
            || leaf_name.len() > 128
            || !leaf_name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return Err(AuthorityError::Invalid);
        }
        // SAFETY: scalar credential queries; fixture ownership is never root enrollment.
        if uid == 0
            || gid == 0
            || unsafe { libc::geteuid() } != uid
            || unsafe { libc::getegid() } != gid
        {
            return Err(AuthorityError::Invalid);
        }
        let root = fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open("/")?;
        let temporary = open_at(&root, "tmp", true)?;
        let temporary_stamp = Stamp::of(&temporary.metadata()?);
        let leaf = open_at(&temporary, leaf_name, true)?;
        let metadata = leaf.metadata()?;
        if metadata.uid() != uid || metadata.gid() != gid || metadata.mode() & 0o7777 != 0o700 {
            return Err(AuthorityError::Invalid);
        }
        FileExt::try_lock_exclusive(&leaf)?;
        Ok(Self {
            root,
            temporary,
            temporary_stamp,
            leaf,
            leaf_stamp: Stamp::of(&metadata),
            leaf_name: leaf_name.into(),
            uid,
            gid,
            fixed_ancestry: None,
        })
    }
    fn revalidate(&self) -> Result<(), AuthorityError> {
        if let Some(ancestry) = &self.fixed_ancestry {
            ancestry.revalidate(&self.root)?;
            let named = open_at(&self.temporary, &self.leaf_name, true)?;
            if !self
                .temporary_stamp
                .directory_matches(&self.temporary.metadata()?)
                || !self.leaf_stamp.directory_matches(&named.metadata()?)
                || !self.leaf_stamp.directory_matches(&self.leaf.metadata()?)
            {
                return Err(AuthorityError::Invalid);
            }
            return Ok(());
        }
        let temporary = open_at(&self.root, "tmp", true)?;
        if !self
            .temporary_stamp
            .directory_matches(&temporary.metadata()?)
            || !self
                .temporary_stamp
                .directory_matches(&self.temporary.metadata()?)
        {
            return Err(AuthorityError::Invalid);
        }
        let named = open_at(&temporary, &self.leaf_name, true)?;
        if !self.leaf_stamp.directory_matches(&named.metadata()?)
            || !self.leaf_stamp.directory_matches(&self.leaf.metadata()?)
        {
            return Err(AuthorityError::Invalid);
        }
        Ok(())
    }
    fn inventory(&self) -> Result<usize, AuthorityError> {
        let mut seen = BTreeSet::new();
        for entry in fs::read_dir(format!("/proc/self/fd/{}", self.leaf.as_raw_fd()))? {
            let entry = entry?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| AuthorityError::Invalid)?;
            if !seen.insert(name) || seen.len() > MAX_GENERATIONS * 2 {
                return Err(AuthorityError::Budget);
            }
        }
        if seen.len() % 2 != 0 {
            return Err(AuthorityError::Invalid);
        }
        let generations = seen.len() / 2;
        if !seen.iter().map(String::as_str).eq((1..=generations)
            .flat_map(|revision| [name(revision, "commit"), name(revision, "json")]))
        {
            return Err(AuthorityError::Invalid);
        }
        Ok(generations)
    }
}
use std::os::unix::fs::OpenOptionsExt;

/// A local fixture writer, never fixed/root production authority or a start guard.
pub struct FixtureAuthorityRegistry {
    directory: Directory,
    entries: Vec<Entry>,
    records: Vec<Record>,
    failed: bool,
    _original_thread: PhantomData<Rc<()>>,
}
pub fn initialize_fixture_registry(
    path: &Path,
    uid: u32,
    gid: u32,
    policy: &CheckedWorkerPolicy,
) -> Result<FixtureAuthorityRegistry, AuthorityError> {
    initialize_directory(Directory::open(path, uid, gid)?, policy)
}
fn initialize_directory(
    directory: Directory,
    policy: &CheckedWorkerPolicy,
) -> Result<FixtureAuthorityRegistry, AuthorityError> {
    let mut read = FixtureAuthorityRegistry {
        directory,
        entries: vec![],
        records: vec![],
        failed: false,
        _original_thread: PhantomData,
    };
    if read.directory.inventory()? != 0 {
        return Err(AuthorityError::Invalid);
    }
    // The leaf was provisioned externally; persist its name before publication.
    read.directory.temporary.sync_all()?;
    let record = Record::from_checked(policy)?;
    read.publish(record, None)?;
    Ok(read)
}
pub fn open_fixture_registry(
    path: &Path,
    uid: u32,
    gid: u32,
    trust: &WorkerPolicyTrust,
    host: &AssertedWorkerHost,
) -> Result<FixtureAuthorityRegistry, AuthorityError> {
    open_directory(Directory::open(path, uid, gid)?, trust, host)
}
fn open_directory(
    directory: Directory,
    trust: &WorkerPolicyTrust,
    host: &AssertedWorkerHost,
) -> Result<FixtureAuthorityRegistry, AuthorityError> {
    directory.revalidate()?;
    let count = directory.inventory()?;
    if count == 0 {
        return Err(AuthorityError::Invalid);
    }
    let entries = (1..=count)
        .map(|revision| Entry::open(&directory.leaf, revision, directory.uid, directory.gid))
        .collect::<Result<Vec<_>, _>>()?;
    let records = history(
        &entries
            .iter()
            .map(|entry| entry.record.raw.clone())
            .collect::<Vec<_>>(),
    )?;
    records
        .last()
        .ok_or(AuthorityError::Invalid)?
        .bind(trust, host)?;
    let mut read = FixtureAuthorityRegistry {
        directory,
        entries,
        records,
        failed: false,
        _original_thread: PhantomData,
    };
    read.revalidate()?;
    Ok(read)
}
impl FixtureAuthorityRegistry {
    pub(crate) fn latch_refusal(&mut self) {
        self.failed = true;
    }
    pub fn is_failed(&self) -> bool {
        self.failed
    }
    pub fn view(&self) -> Result<AuthorityView, AuthorityError> {
        if self.failed {
            return Err(AuthorityError::Refused);
        }
        Ok(self.records.last().ok_or(AuthorityError::Invalid)?.view())
    }
    fn revalidate(&mut self) -> Result<(), AuthorityError> {
        if self.failed {
            return Err(AuthorityError::Refused);
        }
        let result = (|| {
            self.directory.revalidate()?;
            if self.directory.inventory()? != self.entries.len() {
                return Err(AuthorityError::Invalid);
            }
            for entry in &self.entries {
                entry.record.revalidate(
                    &self.directory.leaf,
                    self.directory.uid,
                    self.directory.gid,
                    MAX_RECORD_BYTES,
                )?;
                entry.marker.revalidate(
                    &self.directory.leaf,
                    self.directory.uid,
                    self.directory.gid,
                    COMMIT_MARKER_BYTES,
                )?;
            }
            self.directory.revalidate()?;
            if self.directory.inventory()? != self.entries.len() {
                return Err(AuthorityError::Invalid);
            }
            Ok(())
        })();
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    /// Reinspect original objects under independently supplied pins. Refused is data, not permission.
    pub fn inspect(
        &mut self,
        trust: &WorkerPolicyTrust,
        host: &AssertedWorkerHost,
    ) -> Result<AuthorityView, AuthorityError> {
        self.revalidate()?;
        let result = self
            .records
            .last()
            .ok_or(AuthorityError::Invalid)
            .and_then(|record| {
                record.bind(trust, host)?;
                Ok(record.view())
            });
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    pub fn refuse(&mut self, reason: RefusalReason) -> Result<(), AuthorityError> {
        self.change(|record| record.refuse(reason), None)
    }
    /// Explicit fixture operator transition only, not recovery of a signed worker guard.
    pub fn advance(&mut self, policy: &CheckedWorkerPolicy) -> Result<(), AuthorityError> {
        self.change(|record| record.advance(policy), None)
    }
    fn change(
        &mut self,
        transition: impl FnOnce(&Record) -> Result<Record, AuthorityError>,
        fault: Option<PublicationStage>,
    ) -> Result<(), AuthorityError> {
        self.revalidate()?;
        let next = self
            .records
            .last()
            .ok_or(AuthorityError::Invalid)
            .and_then(transition);
        match next {
            Ok(next) => self.publish(next, fault),
            Err(error) => {
                self.failed = true;
                Err(error)
            }
        }
    }
    fn publish(
        &mut self,
        record: Record,
        fault: Option<PublicationStage>,
    ) -> Result<(), AuthorityError> {
        if self.failed {
            return Err(AuthorityError::Refused);
        }
        let result = (|| {
            self.revalidate()?;
            let bytes = record.bytes()?;
            let mut proposed = self
                .entries
                .iter()
                .map(|entry| entry.record.raw.clone())
                .collect::<Vec<_>>();
            proposed.push(bytes.clone());
            history(&proposed)?;
            let revision = record.revision as usize;
            let mut file = create_new(&self.directory.leaf, &name(revision, "json"))?;
            regular(
                &file.metadata()?,
                self.directory.uid,
                self.directory.gid,
                0o600,
            )?;
            self.directory.leaf.sync_all()?;
            fail(fault, PublicationStage::RecordCreated)?;
            write_seal(&mut file, &bytes)?;
            fail(fault, PublicationStage::RecordSynced)?;
            let mut marker = create_new(&self.directory.leaf, &name(revision, "commit"))?;
            regular(
                &marker.metadata()?,
                self.directory.uid,
                self.directory.gid,
                0o600,
            )?;
            self.directory.leaf.sync_all()?;
            fail(fault, PublicationStage::MarkerCreated)?;
            write_seal(&mut marker, &marker_bytes(&bytes))?;
            self.directory.leaf.sync_all()?;
            fail(fault, PublicationStage::DurableCommit)?;
            let entry = Entry::open(
                &self.directory.leaf,
                revision,
                self.directory.uid,
                self.directory.gid,
            )?;
            if entry.record.raw != bytes {
                return Err(AuthorityError::Invalid);
            }
            self.entries.push(entry);
            self.records.push(record);
            self.revalidate()?;
            Ok(())
        })();
        if result.is_err() {
            self.failed = true;
        }
        result
    }
}
fn name(revision: usize, suffix: &str) -> String {
    format!("{revision:016x}.{suffix}")
}
fn marker_bytes(record: &[u8]) -> Vec<u8> {
    format!("{}\n", sha256_hex(record)).into_bytes()
}
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
fn open_at(parent: &File, name: &str, directory: bool) -> Result<File, AuthorityError> {
    let name = CString::new(name).map_err(|_| AuthorityError::Invalid)?;
    let flags = libc::O_RDONLY
        | libc::O_NOFOLLOW
        | libc::O_CLOEXEC
        | libc::O_NONBLOCK
        | if directory { libc::O_DIRECTORY } else { 0 };
    // SAFETY: live parent descriptor and NUL-terminated bounded/fixed name, read-only.
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    // SAFETY: successful openat returns a new owned descriptor.
    Ok(unsafe { File::from_raw_fd(fd) })
}
fn create_new(parent: &File, name: &str) -> Result<File, AuthorityError> {
    let name = CString::new(name).map_err(|_| AuthorityError::Invalid)?;
    // SAFETY: original directory and revision-derived name; O_EXCL forbids replacement.
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_WRONLY
                | libc::O_CREAT
                | libc::O_EXCL
                | libc::O_NOFOLLOW
                | libc::O_CLOEXEC
                | libc::O_NONBLOCK,
            0o600,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    // SAFETY: new owned descriptor transferred exactly once.
    Ok(unsafe { File::from_raw_fd(fd) })
}
fn write_seal(file: &mut File, bytes: &[u8]) -> Result<(), AuthorityError> {
    write_bytes(file, bytes)?;
    file.set_permissions(fs::Permissions::from_mode(0o400))?;
    file.sync_all()?;
    Ok(())
}
fn write_bytes(writer: &mut impl Write, bytes: &[u8]) -> Result<(), AuthorityError> {
    writer.write_all(bytes)?;
    Ok(())
}
fn fail(
    requested: Option<PublicationStage>,
    stage: PublicationStage,
) -> Result<(), AuthorityError> {
    #[cfg(test)]
    publication_checkpoint::observe(stage);
    if requested == Some(stage) {
        Err(AuthorityError::Injected(stage))
    } else {
        Ok(())
    }
}

#[cfg(test)]
#[path = "diagnostic_authority_checkpoint.rs"]
pub(crate) mod publication_checkpoint;

#[cfg(test)]
impl FixtureAuthorityRegistry {
    pub(crate) fn refuse_at_for_test(
        &mut self,
        reason: RefusalReason,
        stage: PublicationStage,
    ) -> Result<(), AuthorityError> {
        self.change(|record| record.refuse(reason), Some(stage))
    }
}
#[cfg(test)]
#[path = "diagnostic_authority_fixture_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/support/authority_leaf_name_controls.rs"]
pub(crate) mod leaf_name_controls;

#[cfg(test)]
#[path = "../tests/support/authority_inventory_controls.rs"]
pub(crate) mod inventory_controls;
