//! Read-only cgroup tree snapshots. No unit operation, watchdog or cleanup authority.
use crate::diagnostic_lease::{validate_identity, DiagnosticIdentity, SURFACES};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::CString;
use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Component, Path, PathBuf};
use thiserror::Error;

const MAX_NODES: usize = 32;
const MAX_DEPTH: usize = 8;
const MAX_PIDS: usize = 256;
const MAX_ENTRIES: usize = 256;
const DOC_BYTES: usize = 65_536;
const TOTAL_BYTES: usize = 4_194_304;
const DOCS: [&str; 3] = ["cgroup.type", "cgroup.events", "cgroup.procs"];
const CGROUP2_MAGIC: libc::c_long = 0x63677270;

#[derive(Debug, Error)]
pub enum TreeError {
    #[error("tree identity, ownership, document or observation drift refused")]
    Invalid,
    #[error("bounded recursive tree observation exceeded its budget")]
    Budget,
    #[error("read-only tree inspection failed: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CgroupId {
    pub device: u64,
    pub inode: u64,
}
#[derive(Debug, Clone)]
pub struct DiagnosticTreeScope {
    path: PathBuf,
    boot_id: String,
}
impl DiagnosticTreeScope {
    pub fn new(identity: &DiagnosticIdentity, surface: &str) -> Result<Self, TreeError> {
        validate_identity(identity).map_err(|_| TreeError::Invalid)?;
        let n = SURFACES
            .iter()
            .position(|v| *v == surface)
            .ok_or(TreeError::Invalid)?
            + 1;
        Ok(Self {
            path: Path::new("/sys/fs/cgroup/system.slice").join(format!(
                "hydracache-diagnostic-074-{}-{n}.service",
                identity.lease_id
            )),
            boot_id: identity.boot_id.clone(),
        })
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub(crate) fn boot_id(&self) -> &str {
        &self.boot_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeSnapshot {
    pub relative_path: String,
    pub populated: bool,
    pub frozen: bool,
    pub pids: Vec<u32>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeSnapshot {
    id: CgroupId,
    kernel: bool,
    nodes: Vec<NodeSnapshot>,
}
impl TreeSnapshot {
    pub fn root_identity(&self) -> CgroupId {
        self.id
    }
    pub fn kernel_origin(&self) -> bool {
        self.kernel
    }
    pub fn nodes(&self) -> &[NodeSnapshot] {
        &self.nodes
    }
    pub fn process_count(&self) -> usize {
        self.nodes.iter().map(|v| v.pids.len()).sum()
    }
    /// A bounded observation, NOT proof of revoked writers or release authority.
    pub fn empty_at_read(&self) -> bool {
        self.nodes.iter().all(|v| !v.populated && v.pids.is_empty())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Stamp {
    id: CgroupId,
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
impl From<Metadata> for Stamp {
    fn from(m: Metadata) -> Self {
        Self {
            id: CgroupId {
                device: m.dev(),
                inode: m.ino(),
            },
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
struct Document {
    file: File,
    stamp: Stamp,
    bytes: Vec<u8>,
}
struct Node {
    directory: File,
    stamp: Stamp,
    entries: BTreeMap<String, bool>,
    documents: BTreeMap<&'static str, Document>,
    snapshot: NodeSnapshot,
}
struct Pass {
    nodes: BTreeMap<String, Node>,
}
impl Pass {
    fn agrees(&self, other: &Self) -> bool {
        self.nodes.len() == other.nodes.len()
            && self.nodes.iter().all(|(name, a)| {
                other.nodes.get(name).is_some_and(|b| {
                    a.stamp == b.stamp
                        && a.entries == b.entries
                        && a.snapshot == b.snapshot
                        && a.documents.iter().all(|(n, d)| {
                            b.documents
                                .get(n)
                                .is_some_and(|e| d.stamp == e.stamp && d.bytes == e.bytes)
                        })
                })
            })
    }
    fn revalidate_descriptors(&self) -> Result<(), TreeError> {
        for n in self.nodes.values() {
            if Stamp::from(n.directory.metadata()?) != n.stamp
                || inventory(&n.directory)? != n.entries
            {
                return Err(TreeError::Invalid);
            }
            for (name, d) in &n.documents {
                let mut retained = d.file.try_clone()?;
                let named = child(&n.directory, name, false)?;
                if Stamp::from(retained.metadata()?) != d.stamp
                    || Stamp::from(named.metadata()?) != d.stamp
                    || read_doc(&mut retained)? != d.bytes
                    || Stamp::from(retained.metadata()?) != d.stamp
                {
                    return Err(TreeError::Invalid);
                }
            }
        }
        Ok(())
    }
}
pub struct TreeRead {
    path: PathBuf,
    uid: u32,
    gid: u32,
    kernel: bool,
    boot: Option<String>,
    pass: Pass,
    snapshot: TreeSnapshot,
}
impl TreeRead {
    pub(crate) fn is_kernel_scope(&self, scope: &DiagnosticTreeScope) -> bool {
        self.kernel && self.path == scope.path && self.boot.as_deref() == Some(scope.boot_id())
    }
    pub fn snapshot(&self) -> &TreeSnapshot {
        &self.snapshot
    }
    /// No refresh/retry: callers cannot repair this retained snapshot after drift.
    pub fn revalidate(&self) -> Result<(), TreeError> {
        if let Some(boot) = &self.boot {
            check_boot(boot)?;
        }
        self.pass.revalidate_descriptors()?;
        let root = open_directory(&self.path, self.uid, self.gid, self.kernel)?;
        if self.kernel {
            require_cgroupfs(&root)?;
        }
        let current = scan(root, self.uid, self.gid, self.kernel)?;
        if !self.pass.agrees(&current) {
            return Err(TreeError::Invalid);
        }
        self.pass.revalidate_descriptors()?;
        Ok(())
    }
}

/// The expected root device/inode must come from separate authenticated unit
/// observation. This read-only function does NOT authenticate that caller.
pub fn read_kernel_tree(
    scope: &DiagnosticTreeScope,
    expected: CgroupId,
) -> Result<TreeRead, TreeError> {
    if expected.device == 0 || expected.inode == 0 {
        return Err(TreeError::Invalid);
    }
    check_boot(&scope.boot_id)?;
    let root = open_directory(&scope.path, 0, 0, true)?;
    require_cgroupfs(&root)?;
    if Stamp::from(root.metadata()?).id != expected {
        return Err(TreeError::Invalid);
    }
    read_tree(
        scope.path.clone(),
        root,
        0,
        0,
        true,
        Some(scope.boot_id.clone()),
    )
}
/// Explicit synthetic filesystem seam. Never upgrades its result to kernel origin.
pub fn read_fixture_tree(path: &Path, uid: u32, gid: u32) -> Result<TreeRead, TreeError> {
    for prohibited in [
        "/sys",
        "/proc",
        "/var/lib/hydracache-performance",
        "/opt/hydracache-performance",
    ] {
        if path.starts_with(prohibited) {
            return Err(TreeError::Invalid);
        }
    }
    let root = open_directory(path, uid, gid, false)?;
    read_tree(path.into(), root, uid, gid, false, None)
}
fn read_tree(
    path: PathBuf,
    root: File,
    uid: u32,
    gid: u32,
    kernel: bool,
    boot: Option<String>,
) -> Result<TreeRead, TreeError> {
    let pass = scan(root, uid, gid, kernel)?;
    let id = pass.nodes[""].stamp.id;
    let snapshot = TreeSnapshot {
        id,
        kernel,
        nodes: pass.nodes.values().map(|v| v.snapshot.clone()).collect(),
    };
    let result = TreeRead {
        path,
        uid,
        gid,
        kernel,
        boot,
        pass,
        snapshot,
    };
    result.revalidate()?;
    Ok(result)
}

#[derive(Default)]
struct Budget {
    nodes: usize,
    bytes: usize,
    pids: BTreeSet<u32>,
    identities: BTreeSet<(u64, u64)>,
}
impl Budget {
    fn bytes(&mut self, n: usize) -> Result<(), TreeError> {
        self.bytes = self.bytes.checked_add(n).ok_or(TreeError::Budget)?;
        if self.bytes > TOTAL_BYTES {
            Err(TreeError::Budget)
        } else {
            Ok(())
        }
    }
}
fn scan(root: File, uid: u32, gid: u32, kernel: bool) -> Result<Pass, TreeError> {
    let device = root.metadata()?.dev();
    let mut pass = Pass {
        nodes: BTreeMap::new(),
    };
    walk(
        root,
        "",
        0,
        (uid, gid),
        device,
        kernel,
        &mut Budget::default(),
        &mut pass,
    )?;
    // populated is recursive. Contradictions are refusal, not repaired emptiness.
    for (path, n) in &pass.nodes {
        if !n.snapshot.populated
            && pass.nodes.iter().any(|(name, child)| {
                (path.is_empty() || name == path || name.starts_with(&format!("{path}/")))
                    && (child.snapshot.populated || !child.snapshot.pids.is_empty())
            })
        {
            return Err(TreeError::Invalid);
        }
    }
    pass.revalidate_descriptors()?;
    Ok(pass)
}
#[allow(clippy::too_many_arguments)]
fn walk(
    directory: File,
    relative: &str,
    depth: usize,
    owner: (u32, u32),
    device: u64,
    kernel: bool,
    budget: &mut Budget,
    pass: &mut Pass,
) -> Result<(), TreeError> {
    budget.nodes += 1;
    if budget.nodes > MAX_NODES || depth > MAX_DEPTH {
        return Err(TreeError::Budget);
    }
    let meta = directory.metadata()?;
    safe_meta(&meta, owner.0, owner.1, true)?;
    if meta.dev() != device {
        return Err(TreeError::Invalid);
    }
    if kernel {
        require_cgroupfs(&directory)?;
    }
    let stamp = Stamp::from(meta);
    if !budget.identities.insert((stamp.id.device, stamp.id.inode)) {
        return Err(TreeError::Invalid);
    }
    let entries = inventory(&directory)?;
    let mut documents = BTreeMap::new();
    for name in DOCS {
        let mut file = child(&directory, name, false)?;
        safe_meta(&file.metadata()?, owner.0, owner.1, false)?;
        let stamp = Stamp::from(file.metadata()?);
        let bytes = read_doc(&mut file)?;
        budget.bytes(bytes.len())?;
        if Stamp::from(file.metadata()?) != stamp {
            return Err(TreeError::Invalid);
        }
        documents.insert(name, Document { file, stamp, bytes });
    }
    if documents["cgroup.type"].bytes != b"domain\n" {
        return Err(TreeError::Invalid);
    }
    let (populated, frozen) = events(&documents["cgroup.events"].bytes)?;
    let pids = pids(&documents["cgroup.procs"].bytes)?;
    for pid in &pids {
        if !budget.pids.insert(*pid) {
            return Err(TreeError::Invalid);
        }
        if budget.pids.len() > MAX_PIDS {
            return Err(TreeError::Budget);
        }
    }
    for (name, is_dir) in &entries {
        if *is_dir {
            let path = if relative.is_empty() {
                name.clone()
            } else {
                format!("{relative}/{name}")
            };
            walk(
                child(&directory, name, true)?,
                &path,
                depth + 1,
                owner,
                device,
                kernel,
                budget,
                pass,
            )?;
        }
    }
    if Stamp::from(directory.metadata()?) != stamp || inventory(&directory)? != entries {
        return Err(TreeError::Invalid);
    }
    pass.nodes.insert(
        relative.into(),
        Node {
            directory,
            stamp,
            entries,
            documents,
            snapshot: NodeSnapshot {
                relative_path: relative.into(),
                populated,
                frozen,
                pids,
            },
        },
    );
    Ok(())
}
fn events(raw: &[u8]) -> Result<(bool, bool), TreeError> {
    let text = std::str::from_utf8(raw).map_err(|_| TreeError::Invalid)?;
    if !text.ends_with('\n') {
        return Err(TreeError::Invalid);
    }
    let mut values = BTreeMap::new();
    for line in text.lines() {
        let (name, value) = line.split_once(' ').ok_or(TreeError::Invalid)?;
        if !matches!(name, "populated" | "frozen")
            || !matches!(value, "0" | "1")
            || values.insert(name, value == "1").is_some()
        {
            return Err(TreeError::Invalid);
        }
    }
    if values.len() != 2 {
        return Err(TreeError::Invalid);
    }
    Ok((values["populated"], values["frozen"]))
}
fn pids(raw: &[u8]) -> Result<Vec<u32>, TreeError> {
    if raw.is_empty() {
        return Ok(Vec::new());
    }
    let text = std::str::from_utf8(raw).map_err(|_| TreeError::Invalid)?;
    if !text.ends_with('\n') {
        return Err(TreeError::Invalid);
    }
    let mut pids = BTreeSet::new();
    for line in text.lines() {
        if line.is_empty() || line.len() > 10 || !line.bytes().all(|b| b.is_ascii_digit()) {
            return Err(TreeError::Invalid);
        }
        let pid: u32 = line.parse().map_err(|_| TreeError::Invalid)?;
        if pid == 0 || pid.to_string() != line || !pids.insert(pid) {
            return Err(TreeError::Invalid);
        }
        if pids.len() > MAX_PIDS {
            return Err(TreeError::Budget);
        }
    }
    Ok(pids.into_iter().collect())
}
fn safe_meta(m: &Metadata, uid: u32, gid: u32, dir: bool) -> Result<(), TreeError> {
    if m.uid() != uid
        || m.gid() != gid
        || m.mode() & 0o7022 != 0
        || if dir {
            !m.is_dir()
        } else {
            !m.is_file() || m.nlink() != 1
        }
    {
        return Err(TreeError::Invalid);
    }
    Ok(())
}
fn read_doc(file: &mut File) -> Result<Vec<u8>, TreeError> {
    if file.metadata()?.len() > DOC_BYTES as u64 {
        return Err(TreeError::Budget);
    }
    file.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::new();
    file.take((DOC_BYTES + 1) as u64).read_to_end(&mut bytes)?;
    if bytes.len() > DOC_BYTES {
        return Err(TreeError::Budget);
    }
    Ok(bytes)
}
fn inventory(directory: &File) -> Result<BTreeMap<String, bool>, TreeError> {
    let mut entries = BTreeMap::new();
    let owner = directory.metadata()?;
    for entry in fs::read_dir(format!("/proc/self/fd/{}", directory.as_raw_fd()))? {
        let entry = entry?;
        if entries.len() >= MAX_ENTRIES {
            return Err(TreeError::Budget);
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| TreeError::Invalid)?;
        if name.len() > 128
            || name.is_empty()
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        {
            return Err(TreeError::Invalid);
        }
        let kind = entry.file_type()?;
        if !kind.is_file() && !kind.is_dir() {
            return Err(TreeError::Invalid);
        }
        let pinned = child(directory, &name, kind.is_dir())?;
        let meta = pinned.metadata()?;
        safe_meta(&meta, owner.uid(), owner.gid(), kind.is_dir())?;
        if meta.dev() != owner.dev() || entries.insert(name, kind.is_dir()).is_some() {
            return Err(TreeError::Invalid);
        }
    }
    Ok(entries)
}
fn child(parent: &File, name: &str, dir: bool) -> Result<File, TreeError> {
    let name = CString::new(name).map_err(|_| TreeError::Invalid)?;
    // SAFETY: owned parent descriptor and live NUL-terminated name.
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY
                | libc::O_NOFOLLOW
                | libc::O_CLOEXEC
                | libc::O_NONBLOCK
                | if dir { libc::O_DIRECTORY } else { 0 },
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    // SAFETY: successful openat returned a fresh owned descriptor.
    Ok(unsafe { File::from_raw_fd(fd) })
}
fn open_directory(path: &Path, uid: u32, gid: u32, kernel: bool) -> Result<File, TreeError> {
    if !path.is_absolute()
        || path
            .as_os_str()
            .as_bytes()
            .split(|b| *b == b'/')
            .any(|s| s == b"." || s == b"..")
    {
        return Err(TreeError::Invalid);
    }
    let mut directory = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open("/")?;
    if kernel {
        safe_meta(&directory.metadata()?, 0, 0, true)?;
    }
    for component in path.components() {
        match component {
            Component::RootDir => {}
            Component::Normal(name) => {
                let name = name.to_str().ok_or(TreeError::Invalid)?;
                directory = child(&directory, name, true)?;
                if kernel {
                    safe_meta(&directory.metadata()?, 0, 0, true)?;
                }
            }
            _ => return Err(TreeError::Invalid),
        }
    }
    safe_meta(&directory.metadata()?, uid, gid, true)?;
    Ok(directory)
}
fn require_cgroupfs(file: &File) -> Result<(), TreeError> {
    // SAFETY: zero-initialized writable statfs and a live owned descriptor.
    let mut stat: libc::statfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::fstatfs(file.as_raw_fd(), &mut stat) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    if stat.f_type != CGROUP2_MAGIC {
        return Err(TreeError::Invalid);
    }
    Ok(())
}
fn check_boot(expected: &str) -> Result<(), TreeError> {
    let directory = open_directory(Path::new("/proc/sys/kernel/random"), 0, 0, true)?;
    let mut file = child(&directory, "boot_id", false)?;
    safe_meta(&file.metadata()?, 0, 0, false)?;
    let raw = read_doc(&mut file)?;
    if raw != format!("{expected}\n").as_bytes() {
        return Err(TreeError::Invalid);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn seeded_unordered_pid_sets_reconcile_without_deduplicating_ambiguity() {
        let mut seed = 740074u64;
        for count in 1..=MAX_PIDS {
            let mut values: Vec<u32> = (1..=count as u32).collect();
            for index in (1..count).rev() {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                values.swap(index, seed as usize % (index + 1));
            }
            let raw = values
                .iter()
                .map(|pid| format!("{pid}\n"))
                .collect::<String>();
            assert_eq!(
                pids(raw.as_bytes()).unwrap(),
                (1..=count as u32).collect::<Vec<_>>(),
                "seed=740074 count={count}"
            );
            let duplicate = format!("{raw}{}\n", values[0]);
            assert!(
                pids(duplicate.as_bytes()).is_err(),
                "seed=740074 count={count}"
            );
        }
    }
    #[test]
    fn aggregate_document_budget_and_parser_boundaries_are_exact() {
        let mut b = Budget::default();
        b.bytes(TOTAL_BYTES).unwrap();
        assert!(matches!(b.bytes(1), Err(TreeError::Budget)));
        assert_eq!(pids(b"4294967295\n").unwrap(), [u32::MAX]);
        assert!(pids(b"01\n").is_err());
        assert_eq!(events(b"frozen 0\npopulated 1\n").unwrap(), (true, false));
    }
    #[test]
    fn temporary_filesystem_cannot_pass_kernel_gate() {
        let temp = tempfile::tempdir().unwrap();
        assert!(require_cgroupfs(&File::open(temp.path()).unwrap()).is_err());
    }
    #[test]
    fn scope_is_fixed_and_rejects_untrusted_namespace_inputs() {
        let mut identity = DiagnosticIdentity {
            lease_id: "a".repeat(64),
            boot_id: "12345678-1234-1234-1234-123456789abc".into(),
            binary_sha256: "b".repeat(64),
            build_provenance_sha256: "c".repeat(64),
        };
        for (n, surface) in SURFACES.iter().enumerate() {
            let scope = DiagnosticTreeScope::new(&identity, surface).unwrap();
            assert!(scope.path().ends_with(format!(
                "hydracache-diagnostic-074-{}-{}.service",
                identity.lease_id,
                n + 1
            )));
            assert!(read_kernel_tree(
                &scope,
                CgroupId {
                    device: 0,
                    inode: 0
                }
            )
            .is_err());
        }
        assert!(DiagnosticTreeScope::new(&identity, "../escape").is_err());
        identity.lease_id = "../escape".into();
        assert!(DiagnosticTreeScope::new(&identity, "embedded").is_err());
    }
}
