use super::super::*;
use super::from_fd;
use std::hint::black_box;
use std::os::unix::fs::FileExt as _;

// Frozen baseline leaf methods: same opening/regular/parent/buffer/read order,
// fresh File kept through both comparisons, and same scratch eviction on error.
fn reference_read(
    dir: &File,
    name: &str,
    uid: u32,
    gid: u32,
    budget: usize,
    slot: &mut Option<Vec<u8>>,
) -> Result<(File, Stamp, Vec<u8>), AuthorityError> {
    let mut file = open_at(dir, name, false)?;
    let before = file.metadata()?;
    regular(&before, uid, gid, 0o400)?;
    if before.dev() != dir.metadata()?.dev() || before.len() > budget as u64 {
        return Err(AuthorityError::Budget);
    }
    let stamp = Stamp::of(&before);
    let mut raw = match slot.take() {
        Some(mut raw) if raw.capacity() == before.len() as usize => {
            raw.clear();
            raw
        }
        _ => Vec::with_capacity(before.len() as usize),
    };
    (&mut file).take(budget as u64 + 1).read_to_end(&mut raw)?;
    if Stamp::of(&file.metadata()?) != stamp || raw.len() as u64 != stamp.len {
        return Err(AuthorityError::Invalid);
    }
    Ok((file, stamp, raw))
}
fn reference(
    leaf: &mut Leaf,
    dir: &File,
    uid: u32,
    gid: u32,
    budget: usize,
) -> Result<(), AuthorityError> {
    let result = (|| {
        if Stamp::of(&leaf.file.metadata()?) != leaf.stamp {
            return Err(AuthorityError::Invalid);
        }
        let named = reference_read(dir, &leaf.name, uid, gid, budget, &mut leaf.scratch)?;
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
fn same(file: &File) {
    assert!(Stamp::fresh(file).unwrap() == Stamp::of(&file.metadata().unwrap()));
}
fn class(result: Result<(), AuthorityError>) -> (&'static str, Option<i32>) {
    match result {
        Ok(()) => ("ok", None),
        Err(AuthorityError::Invalid) => ("invalid", None),
        Err(AuthorityError::Budget) => ("budget", None),
        Err(AuthorityError::Io(e)) => ("io", e.raw_os_error()),
        Err(e) => panic!("unexpected {e:?}"),
    }
}
#[test]
fn fresh_authority_stamp_fields_changes_unlink_and_errors_are_differential() {
    for name in ["/proc", "/dev/null", "/proc/thread-self/ns/mnt", "/sys"] {
        same(&File::open(name).unwrap());
    }
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("held");
    fs::write(&path, b"original").unwrap();
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();
    let original = Stamp::fresh(&file).unwrap();
    for size in [0, 1, 8193, 2] {
        file.set_len(size).unwrap();
        same(&file);
    }
    for mode in [0o400, 0o600, 0o440] {
        file.set_permissions(fs::Permissions::from_mode(mode))
            .unwrap();
        same(&file);
    }
    let mut seed = 0x074a_5129_u64;
    for _ in 0..32 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let times = [
            libc::timespec {
                tv_sec: 1,
                tv_nsec: 0,
            },
            libc::timespec {
                tv_sec: 1_234_567_890,
                tv_nsec: (seed % 1_000_000_000) as libc::c_long,
            },
        ];
        assert_eq!(
            unsafe { libc::futimens(file.as_raw_fd(), times.as_ptr()) },
            0
        );
        same(&file);
    }
    fs::hard_link(&path, temp.path().join("alias")).unwrap();
    same(&file);
    fs::remove_file(&path).unwrap();
    same(&file);
    fs::write(&path, b"replacement").unwrap();
    assert_ne!(
        Stamp::fresh(&File::open(&path).unwrap()).unwrap().ino,
        original.ino
    );
    fs::remove_file(temp.path().join("alias")).unwrap();
    same(&file);
    assert_eq!(Stamp::fresh(&file).unwrap().links, 0);
    for fd in [-1, i32::MAX] {
        assert_eq!(from_fd(fd).err().unwrap().raw_os_error(), Some(libc::EBADF));
    }
}

#[test]
fn fresh_authority_stamp_leaf_faults_preserve_results_scratch_and_short_circuit() {
    let uid = unsafe { libc::geteuid() };
    let gid = unsafe { libc::getegid() };
    for case in 0..19 {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("leaf");
        fs::write(&path, b"original").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
        let dir = File::open(temp.path()).unwrap();
        let mut old = Leaf::open(&dir, "leaf".into(), uid, gid, 100).unwrap();
        let mut new = Leaf::open(&dir, "leaf".into(), uid, gid, 100).unwrap();
        reference(&mut old, &dir, uid, gid, 100).unwrap();
        new.revalidate(&dir, uid, gid, 100).unwrap();
        assert!(old.scratch.is_some() && new.scratch.is_some());
        for leaf in [&mut old, &mut new] {
            match case {
                1 => leaf.stamp.dev ^= 1,
                2 => leaf.stamp.ino ^= 1,
                3 => leaf.stamp.uid ^= 1,
                4 => leaf.stamp.gid ^= 1,
                5 => leaf.stamp.mode ^= 1,
                6 => leaf.stamp.links ^= 1,
                7 => leaf.stamp.len ^= 1,
                8 => leaf.stamp.mtime ^= 1,
                9 => leaf.stamp.mtime_ns ^= 1,
                10 => leaf.stamp.ctime ^= 1,
                11 => leaf.stamp.ctime_ns ^= 1,
                // If original metadata refuses, a missing named leaf cannot
                // replace that first refusal with ENOENT.
                18 => leaf.stamp.ino ^= 1,
                _ => {}
            }
        }
        match case {
            12 => {
                fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
            }
            13 | 18 => {
                fs::remove_file(&path).unwrap();
            }
            14 => {
                fs::rename(&path, temp.path().join("old")).unwrap();
                fs::write(&path, b"original").unwrap();
            }
            15 => {
                let writer = fs::OpenOptions::new().write(true).open(&path);
                // Use permission change to open as an ordinary test user.
                drop(writer);
                fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
                fs::OpenOptions::new()
                    .write(true)
                    .open(&path)
                    .unwrap()
                    .write_at(b"changed!", 0)
                    .unwrap();
                fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
            }
            _ => {}
        }
        let budget = if case == 16 { 1 } else { 100 };
        let expected_uid = if case == 17 { uid ^ 1 } else { uid };
        let a = class(reference(&mut old, &dir, expected_uid, gid, budget));
        let b = class(new.revalidate(&dir, expected_uid, gid, budget));
        assert_eq!(a, b, "case {case}");
        assert_eq!(old.scratch.is_some(), new.scratch.is_some());
        assert_eq!(b.0 == "ok", case == 0);
        if case == 18 {
            assert_eq!(b, ("invalid", None));
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
#[test]
#[ignore = "manual finite authority leaf CPU screen; not admission or a preparation retry"]
fn fresh_authority_stamp_installed_leaf_cpu_screen() {
    let uid = unsafe { libc::geteuid() };
    let gid = unsafe { libc::getegid() };
    assert_ne!(uid, 0);
    let cohort: usize = std::env::var("HC074_STAMP_COHORT")
        .unwrap()
        .parse()
        .unwrap();
    assert!(cohort < 8);
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("leaf");
    fs::write(&path, vec![b'x'; 1024]).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
    let dir = File::open(temp.path()).unwrap();
    type Call = fn(&mut Leaf, &File, u32, u32, usize) -> Result<(), AuthorityError>;
    let functions: [Call; 3] = [reference, reference, Leaf::revalidate];
    let labels = ["reference", "reference-control", "fresh-stamp"];
    let mut leaf = Leaf::open(&dir, "leaf".into(), uid, gid, MAX_RECORD_BYTES).unwrap();
    for call in functions {
        for _ in 0..32 {
            call(&mut leaf, &dir, uid, gid, MAX_RECORD_BYTES).unwrap();
        }
    }
    let orders = [
        [0, 1, 2],
        [1, 2, 0],
        [2, 0, 1],
        [0, 2, 1],
        [2, 1, 0],
        [1, 0, 2],
    ];
    for sample in 0..12 {
        for (slot, variant) in orders[(sample + cohort) % 6].into_iter().enumerate() {
            let call = black_box(functions[variant]);
            let before = cpu();
            let start = std::time::Instant::now();
            for _ in 0..1024 {
                black_box(call(
                    black_box(&mut leaf),
                    black_box(&dir),
                    uid,
                    gid,
                    MAX_RECORD_BYTES,
                ))
                .unwrap();
            }
            let wall_ns = start.elapsed().as_nanos();
            let cpu_ns = cpu().checked_sub(before).unwrap();
            println!("HC074-STAMP-INSTALLED cohort={cohort} sample={sample} slot={slot} variant={} calls=1024 cpu_ns={cpu_ns} wall_ns={wall_ns}", labels[variant]);
        }
    }
}
