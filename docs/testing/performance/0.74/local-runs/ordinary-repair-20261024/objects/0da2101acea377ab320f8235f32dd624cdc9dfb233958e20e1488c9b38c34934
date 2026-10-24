//! Frozen checked reads and bounded name ownership controls. Never production authority.
use super::*;
#[derive(Clone, Copy)]
pub(crate) enum Reading {
    Reference,
    Proposed,
    Actual,
}
fn reference_open(
    dir: &File,
    name: String,
    uid: u32,
    gid: u32,
    budget: usize,
) -> Result<Leaf, AuthorityError> {
    let mut file = open_at(dir, &name, false)?;
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
    Ok(Leaf {
        file,
        stamp,
        raw,
        name,
        scratch: None,
    })
}

fn proposed_read(
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

fn reference(
    leaf: &Leaf,
    dir: &File,
    uid: u32,
    gid: u32,
    budget: usize,
) -> Result<(), AuthorityError> {
    if Stamp::of(&leaf.file.metadata()?) != leaf.stamp {
        return Err(AuthorityError::Invalid);
    }
    let named = reference_open(dir, leaf.name.clone(), uid, gid, budget)?;
    if named.stamp != leaf.stamp || named.raw != leaf.raw {
        return Err(AuthorityError::Invalid);
    }
    Ok(())
}
fn proposed(
    leaf: &Leaf,
    dir: &File,
    uid: u32,
    gid: u32,
    budget: usize,
) -> Result<(), AuthorityError> {
    if Stamp::of(&leaf.file.metadata()?) != leaf.stamp {
        return Err(AuthorityError::Invalid);
    }
    let named = proposed_read(dir, &leaf.name, uid, gid, budget)?;
    if named.1 != leaf.stamp || named.2 != leaf.raw {
        return Err(AuthorityError::Invalid);
    }
    Ok(())
}
pub(crate) struct LeafFixture {
    directory: File,
    leaf: Leaf,
    uid: u32,
    gid: u32,
    budget: usize,
    root: tempfile::TempDir,
}
impl LeafFixture {
    fn build(suffix: &str, revision: usize, length: usize, seed: u8) -> Self {
        assert_ne!(unsafe { libc::geteuid() }, 0);
        let root = tempfile::Builder::new()
            .prefix("hydracache-authority-leaf-name-")
            .tempdir_in("/tmp")
            .unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let name = name(revision, suffix);
        let path = root.path().join(&name);
        let raw: Vec<_> = (0..length).map(|i| (i as u8).wrapping_add(seed)).collect();
        fs::write(&path, raw).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
        let directory = File::open(root.path()).unwrap();
        let uid = unsafe { libc::geteuid() };
        let gid = unsafe { libc::getegid() };
        let budget = if suffix == "json" {
            MAX_RECORD_BYTES
        } else {
            COMMIT_MARKER_BYTES
        };
        let leaf = Leaf::open(&directory, name, uid, gid, budget).unwrap();
        Self {
            directory,
            leaf,
            uid,
            gid,
            budget,
            root,
        }
    }
    pub(crate) fn new(case: &str) -> Self {
        match case {
            "record" => Self::build("json", 1, 512, 74),
            "commit" => Self::build("commit", 1, 65, 74),
            "metadata-refusal" => {
                let value = Self::build("json", 1, 512, 74);
                fs::set_permissions(
                    value.root.path().join(&value.leaf.name),
                    fs::Permissions::from_mode(0o600),
                )
                .unwrap();
                value
            }
            // Synthetic retained-name mismatch: changing a pathname on disk can
            // first change original inode metadata, so it is a separate fault.
            "missing-name" => {
                let mut value = Self::build("json", 1, 512, 74);
                value.leaf.name = name(2, "json");
                value
            }
            _ => panic!("unknown bounded leaf name case"),
        }
    }
    pub(crate) fn check(&mut self, mode: Reading) -> Result<(), AuthorityError> {
        match mode {
            Reading::Reference => {
                reference(&self.leaf, &self.directory, self.uid, self.gid, self.budget)
            }
            Reading::Proposed => {
                proposed(&self.leaf, &self.directory, self.uid, self.gid, self.budget)
            }
            Reading::Actual => {
                self.leaf
                    .revalidate(&self.directory, self.uid, self.gid, self.budget)
            }
        }
    }
}
fn class(result: Result<(), AuthorityError>) -> (&'static str, Option<i32>) {
    match result {
        Ok(()) => ("ok", None),
        Err(AuthorityError::Invalid) => ("invalid", None),
        Err(AuthorityError::Budget) => ("budget", None),
        Err(AuthorityError::Io(error)) => ("io", error.raw_os_error()),
        Err(_) => panic!("unexpected error class"),
    }
}
#[test]
fn authority_leaf_name_seeded_names_payloads_and_open_results_match_frozen_reference() {
    let mut seed = 0x0741_eaf5_u64;
    for suffix in ["json", "commit"] {
        let budget = if suffix == "json" {
            MAX_RECORD_BYTES
        } else {
            COMMIT_MARKER_BYTES
        };
        for index in 0..32 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let length = match index {
                0 => 0,
                1 => budget,
                _ => seed as usize % (budget + 1),
            };
            let mut value = LeafFixture::build(suffix, index + 1, length, seed as u8);
            for mode in [Reading::Reference, Reading::Proposed, Reading::Actual] {
                assert_eq!(class(value.check(mode)), ("ok", None));
            }
            let before = reference_open(
                &value.directory,
                value.leaf.name.clone(),
                value.uid,
                value.gid,
                budget,
            )
            .unwrap();
            let proposed = proposed_read(
                &value.directory,
                &value.leaf.name,
                value.uid,
                value.gid,
                budget,
            )
            .unwrap();
            let actual = Leaf::open(
                &value.directory,
                value.leaf.name.clone(),
                value.uid,
                value.gid,
                budget,
            )
            .unwrap();
            assert!(before.stamp == proposed.1 && proposed.1 == actual.stamp);
            assert_eq!(before.raw, proposed.2);
            assert_eq!(proposed.2, actual.raw);
            assert_eq!(before.name, actual.name);
            assert_eq!(before.raw.len(), length);
        }
    }
    eprintln!("authority leaf seed=0x0741eaf5 names=64 includes-empty-and-full-budget");
}
#[test]
fn authority_leaf_name_fault_classes_and_original_metadata_short_circuit_match() {
    for fault in 0..12 {
        let mut value = LeafFixture::new("record");
        let path = value.root.path().join(&value.leaf.name);
        match fault {
            0 => (),
            1 => fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap(),
            2 => {
                fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
                fs::write(&path, b"truncated").unwrap();
            }
            3 => {
                fs::rename(&path, value.root.path().join("old")).unwrap();
                fs::write(&path, &value.leaf.raw).unwrap();
                fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
            }
            4 => value.uid = u32::MAX,
            5 => value.gid = u32::MAX,
            6 => value.budget = value.leaf.raw.len() - 1,
            7 => {
                fs::rename(&path, value.root.path().join("old")).unwrap();
                std::os::unix::fs::symlink("old", &path).unwrap();
            }
            8 => fs::hard_link(&path, value.root.path().join("alias")).unwrap(),
            9 => value.leaf.name = name(2, "json"),
            10 => {
                let other = value.root.path().join(name(2, "json"));
                fs::write(&other, &value.leaf.raw).unwrap();
                fs::set_permissions(&other, fs::Permissions::from_mode(0o400)).unwrap();
                value.leaf.name = name(2, "json");
            }
            _ => fs::remove_file(&path).unwrap(),
        }
        let expected = class(value.check(Reading::Reference));
        assert_eq!(expected.0 == "ok", fault == 0);
        for mode in [Reading::Proposed, Reading::Actual] {
            assert_eq!(class(value.check(mode)), expected);
        }
        // Wrong UID and over-budget together preserve ownership-before-budget.
        if fault == 4 {
            value.budget = 0;
            for mode in [Reading::Reference, Reading::Proposed, Reading::Actual] {
                assert_eq!(class(value.check(mode)), ("invalid", None));
            }
        }
    }
    for case in ["record", "commit", "metadata-refusal", "missing-name"] {
        let mut value = LeafFixture::new(case);
        let expected = class(value.check(Reading::Reference));
        for mode in [Reading::Proposed, Reading::Actual] {
            assert_eq!(class(value.check(mode)), expected);
        }
    }
    assert!(std::panic::catch_unwind(|| LeafFixture::new("unknown")).is_err());
}
