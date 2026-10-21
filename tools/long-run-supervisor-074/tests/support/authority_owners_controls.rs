//! Fresh test-only expression observations, never authority or a validation schedule.
use super::super::tests::checked;
use super::*;

#[derive(Clone, Copy, Debug)]
#[repr(usize)]
pub(crate) enum Site {
    Path,
    DirOpen,
    DirNext,
    DirName,
    DirInsert,
    DirExpected,
    HeldMetadata,
    LeafOpen,
    LeafBefore,
    ParentMetadata,
    Buffer,
    Read,
    After,
}
#[cfg(feature = "fixed-leaf-alloc-probe")]
pub(crate) const SITES: [&str; 13] = [
    "path",
    "dir-open",
    "dir-next",
    "dir-name",
    "dir-insert",
    "dir-expected",
    "held-metadata",
    "leaf-open",
    "leaf-before",
    "parent-metadata",
    "buffer",
    "read",
    "after",
];
pub(crate) trait Observer {
    fn around<T>(&mut self, site: Site, call: impl FnOnce() -> T) -> T;
}
pub(crate) struct Uncounted;
impl Observer for Uncounted {
    fn around<T>(&mut self, _: Site, call: impl FnOnce() -> T) -> T {
        call()
    }
}

fn inventory<O: Observer>(
    observer: &mut O,
    directory: &Directory,
) -> Result<usize, AuthorityError> {
    let mut seen = BTreeSet::new();
    let mut path = std::io::Cursor::new([0_u8; 25]);
    observer
        .around(Site::Path, || {
            write!(&mut path, "/proc/self/fd/{}", directory.leaf.as_raw_fd())
        })
        .expect("i32 fd path fits 25 bytes");
    let length = path.position() as usize;
    let path = std::str::from_utf8(&path.get_ref()[..length]).expect("fd path is ASCII");
    let mut entries = observer.around(Site::DirOpen, || fs::read_dir(path))?;
    while let Some(entry) = observer.around(Site::DirNext, || entries.next()) {
        let entry = entry?;
        let name = observer
            .around(Site::DirName, || entry.file_name())
            .into_string()
            .map_err(|_| AuthorityError::Invalid)?;
        if !observer.around(Site::DirInsert, || seen.insert(name))
            || seen.len() > MAX_GENERATIONS * 2
        {
            return Err(AuthorityError::Budget);
        }
    }
    drop(entries);
    if seen.len() % 2 != 0 {
        return Err(AuthorityError::Invalid);
    }
    let generations = seen.len() / 2;
    if !observer.around(Site::DirExpected, || {
        Directory::expected_inventory(&seen, generations)
    }) {
        return Err(AuthorityError::Invalid);
    }
    Ok(generations)
}

fn leaf_read<O: Observer>(
    observer: &mut O,
    dir: &File,
    name: &str,
    uid: u32,
    gid: u32,
    budget: usize,
) -> Result<(File, Stamp, Vec<u8>), AuthorityError> {
    let mut file = observer.around(Site::LeafOpen, || open_at(dir, name, false))?;
    let before = observer.around(Site::LeafBefore, || file.metadata())?;
    regular(&before, uid, gid, 0o400)?;
    if before.dev()
        != observer
            .around(Site::ParentMetadata, || dir.metadata())?
            .dev()
        || before.len() > budget as u64
    {
        return Err(AuthorityError::Budget);
    }
    let stamp = Stamp::of(&before);
    let mut raw = observer.around(Site::Buffer, || Vec::with_capacity(before.len() as usize));
    observer.around(Site::Read, || {
        (&mut file).take(budget as u64 + 1).read_to_end(&mut raw)
    })?;
    if Stamp::of(&observer.around(Site::After, || file.metadata())?) != stamp
        || raw.len() as u64 != stamp.len
    {
        return Err(AuthorityError::Invalid);
    }
    Ok((file, stamp, raw))
}

fn leaf_revalidate<O: Observer>(
    observer: &mut O,
    leaf: &Leaf,
    dir: &File,
    uid: u32,
    gid: u32,
    budget: usize,
) -> Result<(), AuthorityError> {
    if Stamp::of(&observer.around(Site::HeldMetadata, || leaf.file.metadata())?) != leaf.stamp {
        return Err(AuthorityError::Invalid);
    }
    // Keep the fresh File (tuple field zero) alive through both comparisons.
    let named = leaf_read(observer, dir, &leaf.name, uid, gid, budget)?;
    if named.1 != leaf.stamp || named.2 != leaf.raw {
        return Err(AuthorityError::Invalid);
    }
    Ok(())
}

#[derive(Clone, Copy)]
pub(crate) enum Phase {
    Full,
    Directory,
    Inventory,
    Records,
    Markers,
    Bind,
    TracedInventory,
    TracedRecords,
    TracedMarkers,
}
pub(crate) const PHASES: [(&str, Phase); 9] = [
    ("full", Phase::Full),
    ("directory", Phase::Directory),
    ("inventory", Phase::Inventory),
    ("records", Phase::Records),
    ("markers", Phase::Markers),
    ("bind", Phase::Bind),
    ("traced-inventory", Phase::TracedInventory),
    ("traced-records", Phase::TracedRecords),
    ("traced-markers", Phase::TracedMarkers),
];
pub(crate) struct Fixture {
    registry: FixtureAuthorityRegistry,
    policy: CheckedWorkerPolicy,
    depth: usize,
    root: tempfile::TempDir,
}
impl Fixture {
    pub(crate) fn new(depth: usize) -> Self {
        assert!(matches!(depth, 1 | 7));
        let uid = unsafe { libc::geteuid() };
        let gid = unsafe { libc::getegid() };
        assert!(uid > 0 && gid > 0);
        let root = tempfile::Builder::new()
            .prefix("hydracache-authority-")
            .tempdir_in("/tmp")
            .unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let mut policy = checked(7);
        let mut registry = initialize_fixture_registry(root.path(), uid, gid, &policy).unwrap();
        for epoch in 8..8 + (depth / 2) as u64 {
            registry.refuse(RefusalReason::Operator).unwrap();
            policy = checked(epoch);
            registry.advance(&policy).unwrap();
        }
        assert_eq!(registry.view().unwrap().revision, depth as u64);
        Self {
            registry,
            policy,
            depth,
            root,
        }
    }
    pub(crate) fn run<O: Observer>(
        &mut self,
        phase: Phase,
        observer: &mut O,
    ) -> Result<(), AuthorityError> {
        match phase {
            Phase::Full => {
                let view = self
                    .registry
                    .inspect(&self.policy.trust, &self.policy.context)?;
                assert_eq!(view.revision, self.depth as u64);
                assert!(!view.refused);
                std::hint::black_box(view);
            }
            Phase::Directory => self.registry.directory.revalidate()?,
            Phase::Inventory => assert_eq!(self.registry.directory.inventory()?, self.depth),
            Phase::Records | Phase::Markers | Phase::TracedRecords | Phase::TracedMarkers => {
                for entry in &self.registry.entries {
                    let (leaf, budget) = if matches!(phase, Phase::Records | Phase::TracedRecords) {
                        (&entry.record, MAX_RECORD_BYTES)
                    } else {
                        (&entry.marker, COMMIT_MARKER_BYTES)
                    };
                    let dir = &self.registry.directory;
                    if matches!(phase, Phase::TracedRecords | Phase::TracedMarkers) {
                        leaf_revalidate(observer, leaf, &dir.leaf, dir.uid, dir.gid, budget)?;
                    } else {
                        leaf.revalidate(&dir.leaf, dir.uid, dir.gid, budget)?;
                    }
                }
            }
            Phase::Bind => {
                let record = self
                    .registry
                    .records
                    .last()
                    .ok_or(AuthorityError::Invalid)?;
                record.bind(&self.policy.trust, &self.policy.context)?;
                std::hint::black_box(record.view());
            }
            Phase::TracedInventory => {
                assert_eq!(inventory(observer, &self.registry.directory)?, self.depth)
            }
        }
        Ok(())
    }
    #[cfg(feature = "fixed-leaf-alloc-probe")]
    pub(crate) fn identity(&self) -> serde_json::Value {
        serde_json::json!({
            "generations":self.depth,"revision":self.registry.view().unwrap().revision,
            "epoch":self.policy.trust.epoch,"root_name_bytes":self.root.path().file_name().unwrap().len(),
            "leaf_fd":self.registry.directory.leaf.as_raw_fd(),
            "entries":self.registry.entries.iter().enumerate().map(|(index, entry)| serde_json::json!({
                "revision":index+1,"record_name_bytes":entry.record.name.len(),"marker_name_bytes":entry.marker.name.len(),
                "record_bytes":entry.record.raw.len(),"marker_bytes":entry.marker.raw.len(),
                "record_sha256":sha256_hex(&entry.record.raw),"marker_sha256":sha256_hex(&entry.marker.raw),
            })).collect::<Vec<_>>(),
        })
    }
}

fn class<T>(result: Result<T, AuthorityError>) -> (&'static str, Option<i32>) {
    match result {
        Ok(_) => ("ok", None),
        Err(AuthorityError::Invalid) => ("invalid", None),
        Err(AuthorityError::Budget) => ("budget", None),
        Err(AuthorityError::Io(error)) => ("io", error.raw_os_error()),
        Err(_) => panic!("unexpected authority classification"),
    }
}
#[test]
fn authority_owners_inventory_matches_actual_scan_and_fresh_refusals() {
    use std::os::unix::ffi::OsStringExt;
    struct RetiredIterator {
        device: u64,
        inode: u64,
        original_count: usize,
        checked: bool,
    }
    impl RetiredIterator {
        fn count(device: u64, inode: u64) -> usize {
            fs::read_dir("/proc/self/fd")
                .unwrap()
                .filter_map(Result::ok)
                .filter_map(|entry| fs::metadata(entry.path()).ok())
                .filter(|metadata| metadata.dev() == device && metadata.ino() == inode)
                .count()
        }
        fn new(file: &File) -> Self {
            let metadata = file.metadata().unwrap();
            let (device, inode) = (metadata.dev(), metadata.ino());
            Self {
                device,
                inode,
                original_count: Self::count(device, inode),
                checked: false,
            }
        }
    }
    impl Observer for RetiredIterator {
        fn around<T>(&mut self, site: Site, call: impl FnOnce() -> T) -> T {
            let result = call();
            if matches!(site, Site::DirExpected) {
                assert_eq!(
                    Self::count(self.device, self.inode),
                    self.original_count,
                    "ReadDir retired before expected comparison"
                );
                self.checked = true;
            }
            result
        }
    }
    assert_ne!(unsafe { libc::geteuid() }, 0);
    for case in 0..9 {
        let root = tempfile::Builder::new()
            .prefix("hydracache-authority-")
            .tempdir_in("/tmp")
            .unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let mut directory = Directory::open(root.path(), unsafe { libc::geteuid() }, unsafe {
            libc::getegid()
        })
        .unwrap();
        let depth = match case {
            0 => 0,
            1 => 1,
            2 => 7,
            3 => MAX_GENERATIONS,
            4 => MAX_GENERATIONS + 1,
            _ => 1,
        };
        for name in inventory_controls::observed(depth).iter().rev() {
            fs::write(root.path().join(name), []).unwrap();
        }
        match case {
            5 => fs::remove_file(root.path().join(name(1, "json"))).unwrap(),
            6 => fs::rename(
                root.path().join(name(1, "json")),
                root.path().join("1.json"),
            )
            .unwrap(),
            7 => fs::rename(
                root.path().join(name(1, "json")),
                root.path().join(std::ffi::OsString::from_vec(vec![0xff])),
            )
            .unwrap(),
            8 => directory.leaf = File::open(root.path().join(name(1, "json"))).unwrap(),
            _ => (),
        }
        let actual = class(directory.inventory());
        let mut observer = RetiredIterator::new(&directory.leaf);
        assert_eq!(class(inventory(&mut observer, &directory)), actual);
        if case <= 3 {
            assert!(observer.checked);
        }
        assert_eq!(
            actual.0,
            match case {
                0..=3 => "ok",
                4 => "budget",
                8 => "io",
                _ => "invalid",
            }
        );
    }
    for depth in [1, 7] {
        let mut fixture = Fixture::new(depth);
        for (_, phase) in PHASES {
            fixture.run(phase, &mut Uncounted).unwrap();
        }
        fs::remove_file(fixture.root.path().join(name(depth, "commit"))).unwrap();
        assert!(fixture.run(Phase::TracedInventory, &mut Uncounted).is_err());
        assert!(fixture.run(Phase::Inventory, &mut Uncounted).is_err());
        assert!(fixture.run(Phase::Full, &mut Uncounted).is_err());
        assert!(fixture.registry.is_failed());
        assert!(matches!(
            fixture.run(Phase::Full, &mut Uncounted),
            Err(AuthorityError::Refused)
        ));
        assert!(!fixture.root.path().join(name(depth, "commit")).exists());
    }
}
#[test]
fn authority_owners_leaf_read_and_revalidate_match_bytes_stamps_and_refusals() {
    use std::os::unix::fs::symlink;
    let uid = unsafe { libc::geteuid() };
    let gid = unsafe { libc::getegid() };
    assert_ne!(uid, 0);
    for case in 0..9 {
        let root = tempfile::tempdir_in("/tmp").unwrap();
        let path = root.path().join("record");
        fs::write(&path, b"original").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
        let dir = File::open(root.path()).unwrap();
        let leaf = Leaf::open(&dir, "record".into(), uid, gid, MAX_RECORD_BYTES).unwrap();
        match case {
            1 => {
                fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
                fs::write(&path, b"changed!").unwrap();
                fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
            }
            2 => fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap(),
            3 => fs::remove_file(&path).unwrap(),
            4 => {
                fs::rename(&path, root.path().join("old")).unwrap();
                fs::write(&path, b"original").unwrap();
                fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
            }
            5 => {
                fs::remove_file(&path).unwrap();
                symlink("missing", &path).unwrap();
            }
            6 => {
                fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
                fs::write(&path, vec![0; MAX_RECORD_BYTES + 1]).unwrap();
                fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
            }
            7 => fs::hard_link(&path, root.path().join("alias")).unwrap(),
            8 => {
                fs::remove_file(&path).unwrap();
                fs::create_dir(&path).unwrap();
            }
            _ => (),
        }
        // A changed file is intentionally not repaired or reopened as a new owner.
        let actual_revalidation = class(leaf.revalidate(&dir, uid, gid, MAX_RECORD_BYTES));
        assert_eq!(
            actual_revalidation,
            class(leaf_revalidate(
                &mut Uncounted,
                &leaf,
                &dir,
                uid,
                gid,
                MAX_RECORD_BYTES
            ))
        );
        assert_eq!(actual_revalidation.0 == "ok", case == 0);
        let actual = Leaf::read(&dir, "record", uid, gid, MAX_RECORD_BYTES);
        let traced = leaf_read(&mut Uncounted, &dir, "record", uid, gid, MAX_RECORD_BYTES);
        if case == 0 {
            let actual = actual.unwrap();
            let traced = traced.unwrap();
            assert!(actual.1 == traced.1);
            assert_eq!(actual.2, traced.2);
            assert_eq!(
                actual.0.metadata().unwrap().ino(),
                traced.0.metadata().unwrap().ino()
            );
        } else {
            assert_eq!(class(actual), class(traced));
        }
        assert_eq!(
            class(Leaf::read(&dir, "\0", uid, gid, MAX_RECORD_BYTES)),
            class(leaf_read(
                &mut Uncounted,
                &dir,
                "\0",
                uid,
                gid,
                MAX_RECORD_BYTES
            ))
        );
    }
}
