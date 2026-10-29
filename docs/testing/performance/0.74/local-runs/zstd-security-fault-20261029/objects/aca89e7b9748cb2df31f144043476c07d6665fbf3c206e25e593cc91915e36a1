//! Parent-device-only reference/prototype. Before-read checks stay original.
use super::*;
use std::hint::black_box;

type ReadCall = fn(
    &File,
    &str,
    u32,
    u32,
    usize,
    &mut Option<Vec<u8>>,
) -> Result<(File, Stamp, Vec<u8>), AuthorityError>;
fn reference_read(
    dir: &File,
    name: &str,
    uid: u32,
    gid: u32,
    budget: usize,
    slot: &mut Option<Vec<u8>>,
) -> Result<(File, Stamp, Vec<u8>), AuthorityError> {
    let mut file = open_at(dir, name, false)?;
    let stamp = Stamp::fresh_regular(&file, uid, gid, 0o400)?;
    if stamp.dev != dir.metadata()?.dev() || stamp.len > budget as u64 {
        return Err(AuthorityError::Budget);
    }
    let mut raw = match slot.take() {
        Some(mut raw) if raw.capacity() == stamp.len as usize => {
            raw.clear();
            raw
        }
        _ => Vec::with_capacity(stamp.len as usize),
    };
    (&mut file).take(budget as u64 + 1).read_to_end(&mut raw)?;
    if Stamp::fresh(&file)? != stamp || raw.len() as u64 != stamp.len {
        return Err(AuthorityError::Invalid);
    }
    Ok((file, stamp, raw))
}
fn proposed_read(
    dir: &File,
    name: &str,
    uid: u32,
    gid: u32,
    budget: usize,
    slot: &mut Option<Vec<u8>>,
) -> Result<(File, Stamp, Vec<u8>), AuthorityError> {
    let mut file = open_at(dir, name, false)?;
    let stamp = Stamp::fresh_regular(&file, uid, gid, 0o400)?;
    if stamp.dev != Stamp::fresh(dir)?.dev || stamp.len > budget as u64 {
        return Err(AuthorityError::Budget);
    }
    let mut raw = match slot.take() {
        Some(mut raw) if raw.capacity() == stamp.len as usize => {
            raw.clear();
            raw
        }
        _ => Vec::with_capacity(stamp.len as usize),
    };
    (&mut file).take(budget as u64 + 1).read_to_end(&mut raw)?;
    if Stamp::fresh(&file)? != stamp || raw.len() as u64 != stamp.len {
        return Err(AuthorityError::Invalid);
    }
    Ok((file, stamp, raw))
}
fn validate_with(
    leaf: &mut Leaf,
    dir: &File,
    uid: u32,
    gid: u32,
    budget: usize,
    read: ReadCall,
) -> Result<(), AuthorityError> {
    let result = (|| {
        if Stamp::fresh(&leaf.file)? != leaf.stamp {
            return Err(AuthorityError::Invalid);
        }
        let named = read(dir, &leaf.name, uid, gid, budget, &mut leaf.scratch)?;
        if named.1 != leaf.stamp || named.2 != leaf.raw {
            return Err(AuthorityError::Invalid);
        }
        if named.2.capacity() == named.1.len as usize && named.2.capacity() <= budget {
            leaf.scratch = Some(named.2);
        }
        Ok(())
    })();
    if result.is_err() {
        leaf.evict_scratch();
    }
    result
}
fn reference(
    leaf: &mut Leaf,
    dir: &File,
    uid: u32,
    gid: u32,
    budget: usize,
) -> Result<(), AuthorityError> {
    validate_with(leaf, dir, uid, gid, budget, reference_read)
}
fn proposed(
    leaf: &mut Leaf,
    dir: &File,
    uid: u32,
    gid: u32,
    budget: usize,
) -> Result<(), AuthorityError> {
    validate_with(leaf, dir, uid, gid, budget, proposed_read)
}
fn class<T>(result: Result<T, AuthorityError>) -> (&'static str, Option<i32>) {
    match result {
        Ok(_) => ("ok", None),
        Err(AuthorityError::Invalid) => ("invalid", None),
        Err(AuthorityError::Budget) => ("budget", None),
        Err(AuthorityError::Io(e)) => ("io", e.raw_os_error()),
        Err(e) => panic!("unexpected {e:?}"),
    }
}
#[test]
fn fresh_authority_parent_device_and_benign_fields_match_metadata() {
    let named = tempfile::NamedTempFile::new().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let parent = File::open(dir.path()).unwrap();
    let special = File::open("/dev/null").unwrap();
    for file in [named.as_file(), &parent, &special] {
        let metadata = file.metadata().unwrap();
        assert_eq!(metadata.dev(), Stamp::fresh(file).unwrap().dev);
        assert_eq!(metadata.dev(), Stamp::fresh_device(file).unwrap());
    }
    let original = parent.metadata().unwrap().dev();
    fs::write(dir.path().join("unrelated"), b"x").unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o750)).unwrap();
    assert_eq!(original, parent.metadata().unwrap().dev());
    assert_eq!(original, Stamp::fresh(&parent).unwrap().dev);
    assert_eq!(original, Stamp::fresh_device(&parent).unwrap());
    assert_eq!(
        original,
        Stamp::fresh(&parent.try_clone().unwrap()).unwrap().dev
    );
    // A different device still produces the same mismatch as the original
    // projection, not an ownership/type predicate added by the optimization.
    let other = special.metadata().unwrap().dev();
    for observed in [original, other] {
        assert_eq!(
            observed != original,
            observed != Stamp::fresh(&parent).unwrap().dev
        );
    }
    assert_ne!(original, other);
}
#[test]
fn fresh_authority_parent_full_leaf_faults_preserve_precedence_and_scratch() {
    let uid = unsafe { libc::geteuid() };
    let gid = unsafe { libc::getegid() };
    assert!(uid > 0 && gid > 0);
    for case in 0..14 {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("leaf");
        fs::write(&path, vec![b'x'; 65]).unwrap();
        let writer = fs::OpenOptions::new().write(true).open(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
        let dir = File::open(temp.path()).unwrap();
        let mut leaves: Vec<_> = (0..3)
            .map(|_| Leaf::open(&dir, "leaf".into(), uid, gid, 128).unwrap())
            .collect();
        for leaf in &mut leaves {
            leaf.revalidate(&dir, uid, gid, 128).unwrap();
        }
        match case {
            4 => fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap(),
            5 => fs::hard_link(&path, temp.path().join("alias")).unwrap(),
            6 => writer.set_len(129).unwrap(),
            7..=9 => {
                fs::rename(&path, temp.path().join("original")).unwrap();
                if case == 8 {
                    fs::write(&path, vec![b'x'; 65]).unwrap();
                    fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
                } else if case == 9 {
                    std::os::unix::fs::symlink(temp.path().join("original"), &path).unwrap();
                }
            }
            10 => fs::set_permissions(&path, fs::Permissions::from_mode(0o4400)).unwrap(),
            11 => writer.set_len(0).unwrap(),
            13 => fs::rename(&path, temp.path().join("original")).unwrap(),
            _ => {}
        }
        for leaf in &mut leaves {
            // Test-only restamping exposes the before-read guard instead of
            // letting the existing held check mask every metadata fault.
            leaf.stamp = Stamp::fresh(&leaf.file).unwrap();
            if case == 11 {
                leaf.raw.clear();
            }
            if case == 12 {
                leaf.name = "bad\0name".into();
            }
            if case == 13 {
                leaf.stamp.ino ^= 1;
            }
        }
        let expected_uid = if case == 1 { uid ^ 1 } else { uid };
        let expected_gid = if case == 2 { gid ^ 1 } else { gid };
        let budget = if matches!(case, 1..=6) { 1 } else { 128 };
        let a = class(reference(
            &mut leaves[0],
            &dir,
            expected_uid,
            expected_gid,
            budget,
        ));
        let b = class(proposed(
            &mut leaves[1],
            &dir,
            expected_uid,
            expected_gid,
            budget,
        ));
        let c = class(leaves[2].revalidate(&dir, expected_uid, expected_gid, budget));
        assert_eq!(a, b, "case {case}");
        assert_eq!(a, c, "case {case}");
        assert_eq!(a.0 == "ok", matches!(case, 0 | 11), "case {case}");
        assert_eq!(leaves[0].scratch.is_some(), leaves[1].scratch.is_some());
        assert_eq!(leaves[0].scratch.is_some(), leaves[2].scratch.is_some());
        if matches!(case, 1 | 2 | 4 | 5 | 10 | 13) {
            assert_eq!(a, ("invalid", None));
        }
        if matches!(case, 3 | 6) {
            assert_eq!(a, ("budget", None));
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
    let cohort: usize = std::env::var("HC074_PARENT_COHORT")
        .unwrap()
        .parse()
        .unwrap();
    assert!(cohort < 8);
    assert_eq!(unsafe { libc::geteuid() }, 1000);
    let phase = if installed { "installed" } else { "prototype" };
    println!(
        "HC074-PARENT-ORIGIN {}",
        serde_json::json!({"cohort":cohort,"phase":phase,
        "samples":16,"iterations":1024,"uid":1000,"warmups":32,"product":false,"admission":false})
    );
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
    type Call = fn(&mut Leaf, &File, u32, u32, usize) -> Result<(), AuthorityError>;
    let calls: [Call; 4] = [reference, reference, Leaf::revalidate, proposed];
    let labels = ["reference", "reference-control", "runtime", "prototype"];
    for (payload, length, budget) in [
        ("marker", 65, COMMIT_MARKER_BYTES),
        ("record", 1024, MAX_RECORD_BYTES),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("leaf");
        fs::write(&path, vec![b'x'; length]).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
        let dir = File::open(temp.path()).unwrap();
        let mut leaf = Leaf::open(&dir, "leaf".into(), 1000, 1000, budget).unwrap();
        for call in calls {
            for _ in 0..32 {
                call(&mut leaf, &dir, 1000, 1000, budget).unwrap();
            }
        }
        for sample in 0..16 {
            for (slot, variant) in orders[(sample + cohort) % 8].into_iter().enumerate() {
                let call = black_box(calls[variant]);
                let before = cpu();
                let start = std::time::Instant::now();
                for _ in 0..1024 {
                    black_box(call(
                        black_box(&mut leaf),
                        black_box(&dir),
                        1000,
                        1000,
                        budget,
                    ))
                    .unwrap();
                }
                let wall_ns = start.elapsed().as_nanos();
                let cpu_ns = cpu().checked_sub(before).unwrap();
                println!(
                    "HC074-PARENT-COST {}",
                    serde_json::json!({"cohort":cohort,"phase":phase,
                    "payload":payload,"sample":sample,"slot":slot,"variant":labels[variant],"calls":1024,
                    "metadata_observations":4,"payload_bytes":length,"budget":budget,"cpu_ns":cpu_ns,"wall_ns":wall_ns,
                    "product":false,"admission":false})
                );
            }
        }
        leaf.revalidate(&dir, 1000, 1000, budget).unwrap();
    }
}
#[test]
#[ignore = "manual finite baseline parent-device CPU attribution; no admission or preparation retry"]
fn fresh_authority_parent_prototype_cpu_screen() {
    screen(false);
}
#[test]
#[ignore = "manual finite installed parent-device CPU attribution; no admission or preparation retry"]
fn fresh_authority_parent_installed_cpu_screen() {
    screen(true);
}
