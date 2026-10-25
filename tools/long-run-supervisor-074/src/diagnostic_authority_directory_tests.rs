//! Frozen directory reference/prototype. No installed origin, service or clock change.
use super::*;
use std::hint::black_box;
use std::io;

fn reference_match(_: &'static str, stamp: &Stamp, file: &File) -> io::Result<bool> {
    Ok(stamp.directory_matches(&file.metadata()?))
}
fn projected_match(_: &'static str, stamp: &Stamp, file: &File) -> io::Result<bool> {
    let m = Stamp::fresh(file)?;
    Ok(m.mode & libc::S_IFMT == libc::S_IFDIR
        && stamp.dev == m.dev
        && stamp.ino == m.ino
        && stamp.uid == m.uid
        && stamp.gid == m.gid
        && stamp.mode == m.mode)
}
// Same opens, conditional observations and FD lifetimes as the baseline methods.
fn directory_with(
    dir: &Directory,
    mut matches: impl FnMut(&'static str, &Stamp, &File) -> io::Result<bool>,
) -> Result<(), AuthorityError> {
    if let Some(ancestry) = &dir.fixed_ancestry {
        let named_root = fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&ancestry.root_name)?;
        if !matches("root-held", &ancestry.root_stamp, &dir.root)?
            || !matches("root-named", &ancestry.root_stamp, &named_root)?
        {
            return Err(AuthorityError::Invalid);
        }
        let mut parent = &dir.root;
        for ancestor in &ancestry.ancestors {
            let named = open_at(parent, &ancestor.name, true)?;
            if !matches("ancestor-named", &ancestor.stamp, &named)?
                || !matches("ancestor-held", &ancestor.stamp, &ancestor.file)?
            {
                return Err(AuthorityError::Invalid);
            }
            parent = &ancestor.file;
        }
        // The ancestry's named_root closes before the outer leaf is opened.
        drop(named_root);
        let named = open_at(&dir.temporary, &dir.leaf_name, true)?;
        if !matches("parent-held", &dir.temporary_stamp, &dir.temporary)?
            || !matches("leaf-named", &dir.leaf_stamp, &named)?
            || !matches("leaf-held", &dir.leaf_stamp, &dir.leaf)?
        {
            return Err(AuthorityError::Invalid);
        }
        return Ok(());
    }
    let temporary = open_at(&dir.root, "tmp", true)?;
    if !matches("parent-named", &dir.temporary_stamp, &temporary)?
        || !matches("parent-held", &dir.temporary_stamp, &dir.temporary)?
    {
        return Err(AuthorityError::Invalid);
    }
    let named = open_at(&temporary, &dir.leaf_name, true)?;
    if !matches("leaf-named", &dir.leaf_stamp, &named)?
        || !matches("leaf-held", &dir.leaf_stamp, &dir.leaf)?
    {
        return Err(AuthorityError::Invalid);
    }
    Ok(())
}
fn reference(dir: &Directory) -> Result<(), AuthorityError> {
    directory_with(dir, reference_match)
}
fn proposed(dir: &Directory) -> Result<(), AuthorityError> {
    directory_with(dir, projected_match)
}
struct Tree {
    root: tempfile::TempDir,
    directory: Directory,
}
impl Tree {
    fn new(fixed: bool) -> Self {
        let uid = unsafe { libc::geteuid() };
        let gid = unsafe { libc::getegid() };
        assert!(uid > 0 && gid > 0);
        let root = tempfile::Builder::new()
            .prefix("hydracache-authority-directory-")
            .tempdir_in("/tmp")
            .unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let directory = if fixed {
            let mut path = root.path().to_owned();
            for name in ANCESTORS.into_iter().chain([FIXED_LEAF]) {
                path.push(name);
                fs::create_dir(&path).unwrap();
                fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
            }
            let file = fs::OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
                .open(root.path())
                .unwrap();
            walk(file, uid, gid, root.path()).unwrap()
        } else {
            Directory::open(root.path(), uid, gid).unwrap()
        };
        Self { root, directory }
    }
    fn leaf(&self) -> std::path::PathBuf {
        if self.directory.fixed_ancestry.is_some() {
            self.root.path().join(ANCESTORS.join("/")).join(FIXED_LEAF)
        } else {
            self.root.path().to_owned()
        }
    }
}
fn class(result: Result<(), AuthorityError>) -> (&'static str, Option<i32>) {
    match result {
        Ok(()) => ("ok", None),
        Err(AuthorityError::Invalid) => ("invalid", None),
        Err(AuthorityError::Io(e)) => ("io", e.raw_os_error()),
        Err(e) => panic!("unexpected {e:?}"),
    }
}
#[test]
fn fresh_authority_directory_predicates_and_benign_fields_are_differential() {
    let tree = Tree::new(false);
    let file = &tree.directory.leaf;
    let original = Stamp::of(&file.metadata().unwrap());
    let flags =
        [libc::F_GETFL, libc::F_GETFD].map(|query| unsafe { libc::fcntl(file.as_raw_fd(), query) });
    for field in 0..12 {
        let mut expected = original.clone();
        match field {
            0 => {}
            1 => expected.dev ^= 1,
            2 => expected.ino ^= 1,
            3 => expected.uid ^= 1,
            4 => expected.gid ^= 1,
            5 => expected.mode ^= 1,
            6 => expected.links ^= 1,
            7 => expected.len ^= 1,
            8 => expected.mtime ^= 1,
            9 => expected.mtime_ns ^= 1,
            10 => expected.ctime ^= 1,
            _ => expected.ctime_ns ^= 1,
        }
        let old = reference_match("test", &expected, file).unwrap();
        assert_eq!(old, projected_match("test", &expected, file).unwrap());
        assert_eq!(old, expected.directory_matches_file(file).unwrap());
        assert_eq!(old, field == 0 || field >= 6);
    }
    fs::create_dir(tree.root.path().join("unrelated")).unwrap();
    assert!(projected_match("test", &original, file).unwrap());
    let regular = tempfile::tempfile().unwrap();
    let expected = Stamp::of(&regular.metadata().unwrap());
    assert!(!reference_match("regular", &expected, &regular).unwrap());
    assert!(!projected_match("regular", &expected, &regular).unwrap());
    assert!(!expected.directory_matches_file(&regular).unwrap());
    for (index, query) in [libc::F_GETFL, libc::F_GETFD].into_iter().enumerate() {
        assert_eq!(
            unsafe { libc::fcntl(file.as_raw_fd(), query) },
            flags[index]
        );
    }
}
#[test]
fn fresh_authority_directory_order_faults_and_runtime_match_frozen_reference() {
    for fixed in [false, true] {
        for case in 0..10 {
            let mut tree = Tree::new(fixed);
            let backup = tempfile::tempdir().unwrap();
            match case {
                0 => {}
                1 => tree.directory.temporary_stamp.ino ^= 1,
                2 => tree.directory.leaf_stamp.uid ^= 1,
                3 => tree.directory.temporary = File::open(backup.path()).unwrap(),
                4 => tree.directory.leaf = File::open("/tmp").unwrap(),
                5 | 6 => {
                    let path = tree.leaf();
                    fs::rename(&path, backup.path().join("original")).unwrap();
                    if case == 6 {
                        fs::create_dir(&path).unwrap();
                        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
                    }
                }
                7 => fs::set_permissions(tree.leaf(), fs::Permissions::from_mode(0o755)).unwrap(),
                8 | 9 => {
                    if let Some(a) = &mut tree.directory.fixed_ancestry {
                        a.root_stamp.ino ^= 1;
                        if case == 9 {
                            // Root metadata refusal must precede missing ancestor.
                            fs::rename(tree.root.path().join("var"), backup.path().join("var"))
                                .unwrap();
                        }
                    } else {
                        tree.directory.temporary_stamp.ino ^= 1;
                        if case == 9 {
                            fs::rename(tree.root.path(), backup.path().join("original")).unwrap();
                        }
                    }
                }
                _ => unreachable!(),
            }
            let mut old_sites = Vec::new();
            let mut new_sites = Vec::new();
            let a = class(directory_with(&tree.directory, |site, s, f| {
                old_sites.push(site);
                reference_match(site, s, f)
            }));
            let b = class(directory_with(&tree.directory, |site, s, f| {
                new_sites.push(site);
                projected_match(site, s, f)
            }));
            assert_eq!(a, b, "fixed={fixed} case={case}");
            assert_eq!(old_sites, new_sites);
            assert_eq!(a, class(tree.directory.revalidate()));
            assert_eq!(a.0 == "ok", case == 0, "fixed={fixed} case={case}");
            if case == 0 {
                assert_eq!(
                    old_sites,
                    if fixed {
                        vec![
                            "root-held",
                            "root-named",
                            "ancestor-named",
                            "ancestor-held",
                            "ancestor-named",
                            "ancestor-held",
                            "ancestor-named",
                            "ancestor-held",
                            "parent-held",
                            "leaf-named",
                            "leaf-held",
                        ]
                    } else {
                        vec!["parent-named", "parent-held", "leaf-named", "leaf-held"]
                    }
                );
            }
            if case == 9 {
                assert_eq!(a, ("invalid", None));
                assert_eq!(old_sites.len(), 1);
            }
        }
    }
}
fn cpu() -> u64 {
    let mut t: libc::timespec = unsafe { std::mem::zeroed() };
    assert_eq!(
        unsafe { libc::clock_gettime(libc::CLOCK_PROCESS_CPUTIME_ID, &mut t) },
        0
    );
    assert!(t.tv_sec >= 0 && (0..1_000_000_000).contains(&t.tv_nsec));
    (t.tv_sec as u64)
        .checked_mul(1_000_000_000)
        .unwrap()
        .checked_add(t.tv_nsec as u64)
        .unwrap()
}
fn screen(installed: bool) {
    let cohort: usize = std::env::var("HC074_DIRECTORY_COHORT")
        .unwrap()
        .parse()
        .unwrap();
    assert!(cohort < 8);
    assert_eq!(unsafe { libc::geteuid() }, 1000);
    let phase = if installed { "installed" } else { "prototype" };
    let orders = [
        [0, 1, 2, 3],
        [1, 2, 3, 0],
        [2, 3, 0, 1],
        [3, 0, 1, 2],
        [0, 3, 2, 1],
        [1, 0, 3, 2],
        [2, 1, 0, 3],
        [3, 2, 1, 0],
    ];
    println!(
        "HC074-DIR-ORIGIN {}",
        serde_json::json!({"cohort":cohort,"phase":phase,
        "samples":16,"iterations":1024,"uid":1000,"warmups":32,"product":false,"admission":false})
    );
    for fixed in [false, true] {
        let tree = Tree::new(fixed);
        let layout = if fixed { "fixed" } else { "fixture" };
        let queries = if fixed { 11 } else { 4 };
        type Call = fn(&Directory) -> Result<(), AuthorityError>;
        let calls: [Call; 4] = [reference, reference, Directory::revalidate, proposed];
        let labels = ["reference", "reference-control", "runtime", "prototype"];
        for call in calls {
            for _ in 0..32 {
                call(&tree.directory).unwrap();
            }
        }
        for sample in 0..16 {
            for (slot, variant) in orders[(sample + cohort) % 8].into_iter().enumerate() {
                let call = black_box(calls[variant]);
                let before = cpu();
                let start = std::time::Instant::now();
                for _ in 0..1024 {
                    black_box(call(black_box(&tree.directory))).unwrap();
                }
                let wall_ns = start.elapsed().as_nanos();
                let cpu_ns = cpu().checked_sub(before).unwrap();
                println!(
                    "HC074-DIR-COST {}",
                    serde_json::json!({"cohort":cohort,"phase":phase,
                    "layout":layout,"sample":sample,"slot":slot,"variant":labels[variant],
                    "calls":1024,"metadata_queries_per_call":queries,"cpu_ns":cpu_ns,"wall_ns":wall_ns,
                    "product":false,"admission":false})
                );
            }
        }
        tree.directory.revalidate().unwrap();
    }
}
#[test]
#[ignore = "manual finite baseline directory CPU attribution; no admission or preparation retry"]
fn fresh_authority_directory_prototype_cpu_screen() {
    screen(false);
}
#[test]
#[ignore = "manual finite installed directory CPU attribution; no admission or preparation retry"]
fn fresh_authority_directory_installed_cpu_screen() {
    screen(true);
}
