use super::*;
use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};

struct Fixture {
    temp: tempfile::TempDir,
    expected: ArtifactDigest,
    owner: (u32, u32),
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        for path in [
            temp.path().to_owned(),
            temp.path().join("a"),
            temp.path().join("a/b"),
        ] {
            if !path.exists() {
                fs::create_dir(&path).unwrap();
            }
            fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
        }
        fs::write(temp.path().join("a/b/input"), b"synthetic unsigned input").unwrap();
        fs::set_permissions(
            temp.path().join("a/b/input"),
            fs::Permissions::from_mode(0o555),
        )
        .unwrap();
        let m = fs::metadata(temp.path()).unwrap();
        Self {
            temp,
            expected: ArtifactDigest::of(b"synthetic unsigned input"),
            owner: (m.uid(), m.gid()),
        }
    }
    fn path(&self) -> PathBuf {
        self.temp.path().join("a/b/input")
    }
    fn read(&self) -> io::Result<RootOwnedInputRead> {
        inspect_at(
            Path::new("/a/b/input"),
            self.expected.clone(),
            self.temp.path(),
            self.owner,
        )
    }
}
#[test]
fn root_input_actual_root_owned_file_and_original_namespaces_are_readonly_inventory() {
    let expected = ArtifactDigest::of(&fs::read("/usr/bin/true").unwrap());
    let mut read = inspect_root_owned_input(Path::new("/usr/bin/true"), expected.clone()).unwrap();
    assert_eq!(read.owner, (0, 0));
    assert_eq!(read.anchor, Path::new("/"));
    assert_eq!(read.context.tid, tid());
    assert_ne!(read.context.user.id, read.context.mount.id);
    assert_eq!(read.check_original().unwrap(), &expected);
    println!("actual_root_owned_input=observed original_reader_namespaces=matched initial_host_or_job_enrolled=false input_executed=false");
}
#[test]
fn root_input_fixture_seam_cannot_enter_public_numeric_root_reader() {
    let f = Fixture::new();
    let mut local = f.read().unwrap();
    local.check_original().unwrap();
    assert!(inspect_root_owned_input(&f.path(), f.expected.clone()).is_err());
    // A synthetic root-looking stamp is only metadata, never the public constructor.
    let mut s = local.nodes.last().unwrap().stamp.clone();
    s.uid = 0;
    s.gid = 0;
    s.allowed(false, (0, 0), f.expected.bytes).unwrap();
}
#[test]
fn root_input_all_required_statx_fields_and_invalid_times_refuse() {
    // SAFETY: initialized local synthetic statx buffer, never a kernel proof.
    let mut s: libc::statx = unsafe { std::mem::zeroed() };
    s.stx_mask = MASK;
    s.stx_mnt_id = 1;
    s.stx_ino = 2;
    Stamp::decode(&s).unwrap();
    for bit in 0..32 {
        if MASK & (1 << bit) != 0 {
            s.stx_mask = MASK & !(1 << bit);
            assert!(Stamp::decode(&s).is_err());
        }
    }
    s.stx_mask = MASK;
    s.stx_mnt_id = 0;
    assert!(Stamp::decode(&s).is_err());
    s.stx_mnt_id = 1;
    s.stx_ino = 0;
    assert!(Stamp::decode(&s).is_err());
    s.stx_ino = 2;
    s.stx_mtime.tv_nsec = 1_000_000_000;
    assert!(Stamp::decode(&s).is_err());
    s.stx_mtime.tv_nsec = 0;
    s.stx_ctime.tv_nsec = 1_000_000_000;
    assert!(Stamp::decode(&s).is_err());
}
#[test]
fn root_input_owner_type_mode_link_and_size_policy_is_exact() {
    let f = Fixture::new();
    let read = f.read().unwrap();
    let original = read.nodes.last().unwrap().stamp.clone();
    for mode in [0o444, 0o555, 0o644, 0o755] {
        let mut s = original.clone();
        s.mode = libc::S_IFREG as u16 | mode;
        s.allowed(false, f.owner, f.expected.bytes).unwrap();
    }
    for mode in [0o600, 0o777, 0o664, 0o4755, 0o2755, 0o1755, 0o000] {
        let mut s = original.clone();
        s.mode = libc::S_IFREG as u16 | mode;
        assert!(s.allowed(false, f.owner, f.expected.bytes).is_err());
    }
    for case in 0..6 {
        let mut s = original.clone();
        match case {
            0 => s.uid ^= 1,
            1 => s.gid ^= 1,
            2 => s.links = 2,
            3 => s.links = 0,
            4 => s.bytes += 1,
            _ => s.mode = libc::S_IFIFO as u16 | 0o555,
        };
        assert!(s.allowed(false, f.owner, f.expected.bytes).is_err());
    }
    let dir = &read.nodes[0].stamp;
    for mode in [0o555, 0o755] {
        let mut s = dir.clone();
        s.mode = libc::S_IFDIR as u16 | mode;
        s.allowed(true, f.owner, 0).unwrap();
    }
    for mode in [0o700, 0o775, 0o1777, 0o2755] {
        let mut s = dir.clone();
        s.mode = libc::S_IFDIR as u16 | mode;
        assert!(s.allowed(true, f.owner, 0).is_err());
    }
}
#[test]
fn root_input_lexical_and_byte_budgets_refuse_before_open() {
    let digest = ArtifactDigest::of(b"x");
    for path in [
        "relative", "/", "//input", "/a//b", "/a/./b", "/a/../b", "/a/", "/a\0b",
    ] {
        assert!(names(Path::new(path), &digest).is_err());
    }
    assert!(names(Path::new(&format!("/{}", "a".repeat(256))), &digest).is_err());
    assert!(names(Path::new(&format!("/{}", vec!["x"; 33].join("/"))), &digest).is_err());
    assert!(names(Path::new(&format!("/{}", "x".repeat(4096))), &digest).is_err());
    for bad in [
        ArtifactDigest {
            sha256: "BAD".into(),
            bytes: 1,
        },
        ArtifactDigest {
            sha256: digest.sha256.clone(),
            bytes: 0,
        },
        ArtifactDigest {
            sha256: digest.sha256.clone(),
            bytes: MAX_BINARY_BYTES + 1,
        },
    ] {
        assert!(names(Path::new("/input"), &bad).is_err());
    }
    assert_eq!(
        names(Path::new(&format!("/{}", vec!["x"; 32].join("/"))), &digest)
            .unwrap()
            .len(),
        32
    );
}
#[test]
fn root_input_file_and_ancestor_symlinks_fifo_hardlinks_and_directories_refuse() {
    for case in 0..5 {
        let f = Fixture::new();
        if case == 0 {
            fs::rename(f.path(), f.temp.path().join("saved")).unwrap();
            symlink(f.temp.path().join("saved"), f.path()).unwrap();
        }
        if case == 1 {
            fs::rename(f.temp.path().join("a"), f.temp.path().join("saved")).unwrap();
            symlink(f.temp.path().join("saved"), f.temp.path().join("a")).unwrap();
        }
        if case == 2 {
            fs::hard_link(f.path(), f.temp.path().join("alias")).unwrap();
        }
        if case == 3 {
            fs::remove_file(f.path()).unwrap();
            let path = CString::new(f.path().as_os_str().as_bytes()).unwrap();
            // SAFETY: create only an absent FIFO in the owned temporary fixture.
            assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o444) }, 0);
        }
        if case == 4 {
            fs::remove_file(f.path()).unwrap();
            fs::create_dir(f.path()).unwrap();
        }
        assert!(f.read().is_err());
    }
}
#[test]
fn root_input_equal_byte_file_and_ancestor_replacements_are_sticky() {
    for case in 0..2 {
        let f = Fixture::new();
        let mut read = f.read().unwrap();
        if case == 0 {
            fs::rename(f.path(), f.temp.path().join("saved-input")).unwrap();
            fs::write(f.path(), b"synthetic unsigned input").unwrap();
            fs::set_permissions(f.path(), fs::Permissions::from_mode(0o555)).unwrap();
        } else {
            fs::rename(
                f.temp.path().join("a/b"),
                f.temp.path().join("saved-directory"),
            )
            .unwrap();
            fs::create_dir(f.temp.path().join("a/b")).unwrap();
            fs::set_permissions(f.temp.path().join("a/b"), fs::Permissions::from_mode(0o755))
                .unwrap();
            fs::write(f.path(), b"synthetic unsigned input").unwrap();
            fs::set_permissions(f.path(), fs::Permissions::from_mode(0o555)).unwrap();
        }
        assert!(read.check_original().is_err());
        assert!(read.is_refused());
        if case == 0 {
            fs::remove_file(f.path()).unwrap();
            fs::rename(f.temp.path().join("saved-input"), f.path()).unwrap();
        }
        assert!(read.check_original().is_err());
    }
}
#[test]
fn root_input_hash_catches_same_size_write_even_with_synthetic_metadata_reset() {
    let f = Fixture::new();
    let mut read = f.read().unwrap();
    fs::set_permissions(f.path(), fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(f.path(), b"Synthetic unsigned input").unwrap();
    fs::set_permissions(f.path(), fs::Permissions::from_mode(0o555)).unwrap();
    // Deliberately refresh ONLY private test metadata to isolate hash rejection.
    // The production guard exposes no such refresh or content replacement method.
    let last = read.nodes.last_mut().unwrap();
    last.stamp = stamp(&last.file).unwrap();
    assert!(read.check_original().is_err());
    assert!(read.is_refused());
    let mut larger = f.expected.clone();
    larger.bytes -= 1;
    assert!(hash(&last_file(&f), &larger).is_err());
    larger.bytes += 2;
    assert!(hash(&last_file(&f), &larger).is_err());
    fn last_file(f: &Fixture) -> File {
        File::open(f.path()).unwrap()
    }
}
#[test]
fn root_input_ancestor_mode_drift_and_unrelated_entry_changes_are_distinct() {
    let f = Fixture::new();
    let mut read = f.read().unwrap();
    fs::write(f.temp.path().join("a/unrelated"), b"other entry").unwrap();
    read.check_original().unwrap();
    fs::set_permissions(f.temp.path().join("a"), fs::Permissions::from_mode(0o775)).unwrap();
    assert!(read.check_original().is_err());
    fs::set_permissions(f.temp.path().join("a"), fs::Permissions::from_mode(0o755)).unwrap();
    assert!(read.check_original().is_err());
}
#[test]
fn root_input_namespace_root_and_original_thread_drift_refuse_without_refresh() {
    for case in 0..4 {
        let f = Fixture::new();
        let mut read = f.read().unwrap();
        match case {
            0 => read.context.tid = -1,
            1 => read.context.user.id.1 ^= 1,
            2 => read.context.mount.id.1 ^= 1,
            _ => read.context.root_stamp.mount ^= 1,
        }
        assert!(read.check_original().is_err());
        assert!(read.is_refused());
        // The sticky failure precedes IO, even if the private owner is now incomplete.
        read.nodes.clear();
        assert!(read.check_original().is_err());
    }
    let f = Fixture::new();
    let file = File::open(f.path()).unwrap();
    assert!(namespace_id(&file, libc::CLONE_NEWUSER).is_err());
    let user = namespace(libc::CLONE_NEWUSER).unwrap();
    assert!(namespace_id(&user.file, libc::CLONE_NEWNS).is_err());
    assert!(namespace(0).is_err());
}
#[test]
fn root_input_descriptor_flags_offsets_and_drop_preserve_other_holders() {
    use std::io::{Seek, SeekFrom};
    let f = Fixture::new();
    let read = f.read().unwrap();
    for node in &read.nodes {
        readonly(&node.file).unwrap();
    }
    let last = &read.nodes.last().unwrap().file;
    let mut shared = last;
    shared.seek(SeekFrom::Start(7)).unwrap();
    hash(last, &f.expected).unwrap();
    assert_eq!(shared.stream_position().unwrap(), 7);
    let other = last.try_clone().unwrap();
    drop(read);
    readonly(&other).unwrap();
    hash(&other, &f.expected).unwrap();
    let writable = OpenOptions::new().write(true).open(f.path());
    assert!(writable.is_err());
    let inherited = other.try_clone().unwrap();
    // SAFETY: mutate only this test's isolated duplicate FD flags.
    assert_eq!(
        unsafe { libc::fcntl(inherited.as_raw_fd(), libc::F_SETFD, 0) },
        0
    );
    assert!(readonly(&inherited).is_err());
    let path_only = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_PATH | libc::O_CLOEXEC)
        .open(f.path())
        .unwrap();
    assert!(readonly(&path_only).is_err());
    fs::set_permissions(f.path(), fs::Permissions::from_mode(0o755)).unwrap();
    let writable = OpenOptions::new().write(true).open(f.path()).unwrap();
    assert!(readonly(&writable).is_err());
}
#[test]
fn root_input_seeded_full_metadata_mutations_and_independent_observers() {
    let f = Fixture::new();
    let read = f.read().unwrap();
    let original = &read.nodes.last().unwrap().stamp;
    let mut seed = 0x0740_c075u64;
    for index in 0..64 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut changed = original.clone();
        match if index < 13 { index } else { seed % 13 } {
            0 => changed.device.0 ^= 1,
            1 => changed.device.1 ^= 1,
            2 => changed.inode ^= 1,
            3 => changed.mount ^= 1,
            4 => changed.uid ^= 1,
            5 => changed.gid ^= 1,
            6 => changed.mode ^= 1,
            7 => changed.links += 1,
            8 => changed.bytes += 1,
            9 => changed.modified.0 += 1,
            10 => changed.modified.1 ^= 1,
            11 => changed.changed.0 += 1,
            _ => changed.changed.1 ^= 1,
        }
        assert_ne!(&changed, original);
        let mut mutated = f.read().unwrap();
        mutated.nodes.last_mut().unwrap().stamp = changed;
        assert!(mutated.check_original().is_err());
        assert!(mutated.is_refused());
    }
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|_| {
                scope.spawn(|| {
                    let mut read = f.read().unwrap();
                    for _ in 0..4 {
                        read.check_original().unwrap();
                    }
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
    });
    println!(
        "seed=0x0740c075 metadata_mutations=64 independent_observers=4 production_custody=false"
    );
}

#[test]
fn root_input_internal_snapshot_keeps_offset_and_latches_budget_or_content_failure() {
    use std::io::{Seek, SeekFrom};
    let f = Fixture::new();
    let mut read = f.read().unwrap();
    let mut last = &read.nodes.last().unwrap().file;
    last.seek(SeekFrom::Start(7)).unwrap();
    assert_eq!(
        read.read_original_bounded(64).unwrap(),
        b"synthetic unsigned input"
    );
    let mut last = &read.nodes.last().unwrap().file;
    assert_eq!(last.stream_position().unwrap(), 7);
    assert!(read.read_original_bounded(1).is_err());
    assert!(read.is_refused());
    assert!(read.read_original_bounded(64).is_err());
    let mut read = f.read().unwrap();
    assert!(read.read_original_bounded(MAX_BINARY_BYTES + 1).is_err());
    assert!(read.is_refused());
    let mut read = f.read().unwrap();
    fs::set_permissions(f.path(), fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(f.path(), b"Synthetic unsigned input").unwrap();
    assert!(read.read_original_bounded(64).is_err());
    assert!(read.is_refused());
}
