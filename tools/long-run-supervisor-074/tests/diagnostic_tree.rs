//! Synthetic cgroup documents on temporary Linux storage, never kernel cleanup proof.
#[cfg(target_os = "linux")]
mod linux {
    use hydracache_long_run_supervisor_074::diagnostic_tree::*;
    use std::fs;
    use std::os::unix::fs::{symlink, PermissionsExt};
    use std::path::{Path, PathBuf};

    fn uid() -> u32 {
        unsafe { libc::geteuid() }
    }
    fn gid() -> u32 {
        unsafe { libc::getegid() }
    }
    fn mode(path: &Path, value: u32) {
        fs::set_permissions(path, fs::Permissions::from_mode(value)).unwrap();
    }
    fn node(path: &Path, populated: bool, pids: &str) {
        fs::create_dir(path).unwrap();
        mode(path, 0o700);
        fs::write(path.join("cgroup.type"), b"domain\n").unwrap();
        fs::write(
            path.join("cgroup.events"),
            format!("populated {}\nfrozen 0\n", u8::from(populated)),
        )
        .unwrap();
        fs::write(path.join("cgroup.procs"), pids).unwrap();
        for name in ["cgroup.type", "cgroup.events", "cgroup.procs"] {
            mode(&path.join(name), 0o600);
        }
    }
    struct Fixture {
        _temp: tempfile::TempDir,
        root: PathBuf,
    }
    impl Fixture {
        fn new(populated: bool, pids: &str) -> Self {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path().join("tree");
            node(&root, populated, pids);
            Self { _temp: temp, root }
        }
        fn read(&self) -> Result<TreeRead, TreeError> {
            read_fixture_tree(&self.root, uid(), gid())
        }
    }
    #[test]
    fn recursive_child_membership_survives_empty_leader_list() {
        let f = Fixture::new(true, "");
        node(&f.root.join("child"), true, "");
        node(&f.root.join("child/grandchild"), true, "19\n7\n");
        let r = f.read().unwrap();
        assert!(!r.snapshot().kernel_origin());
        assert!(!r.snapshot().empty_at_read());
        assert_eq!(r.snapshot().process_count(), 2);
        assert_eq!(r.snapshot().nodes().len(), 3);
        assert_eq!(r.snapshot().nodes()[2].relative_path, "child/grandchild");
        assert_eq!(r.snapshot().nodes()[2].pids, [7, 19]);
        r.revalidate().unwrap();
    }
    #[test]
    fn empty_and_populated_without_pids_are_distinct_observations() {
        let empty = Fixture::new(false, "");
        node(&empty.root.join("child"), false, "");
        assert!(empty.read().unwrap().snapshot().empty_at_read());
        let busy = Fixture::new(true, "");
        assert!(!busy.read().unwrap().snapshot().empty_at_read());
        assert_eq!(busy.read().unwrap().snapshot().process_count(), 0);
    }
    #[test]
    fn contradictory_recursive_events_and_duplicate_pids_fail_closed() {
        for case in 0..4 {
            let f = Fixture::new(case != 0, if case == 1 { "1\n" } else { "" });
            node(
                &f.root.join("child"),
                true,
                if case == 2 { "" } else { "1\n" },
            );
            if case == 2 {
                fs::write(f.root.join("cgroup.events"), "populated 0\nfrozen 0\n").unwrap();
            }
            if case == 3 {
                fs::write(
                    f.root.join("child/cgroup.events"),
                    "populated 0\nfrozen 0\n",
                )
                .unwrap();
            }
            assert!(f.read().is_err(), "case {case}");
        }
    }
    #[test]
    fn strict_documents_refuse_missing_unknown_duplicate_and_numeric_drift() {
        for (name, value) in [
            ("cgroup.type", "threaded\n"),
            ("cgroup.type", "domain threaded\n"),
            ("cgroup.type", "domain (invalid)\n"),
            ("cgroup.events", "populated 0\n"),
            ("cgroup.events", "populated 0\nfrozen 2\n"),
            ("cgroup.events", "populated 0\nfrozen 0\nfuture 1\n"),
            ("cgroup.events", "populated 0\nfrozen 0\npopulated 0\n"),
            ("cgroup.procs", "0\n"),
            ("cgroup.procs", "-1\n"),
            ("cgroup.procs", "4294967296\n"),
            ("cgroup.procs", "1\n1\n"),
            ("cgroup.procs", " 1\n"),
            ("cgroup.procs", "1"),
        ] {
            let f = Fixture::new(false, "");
            fs::write(f.root.join(name), value).unwrap();
            assert!(f.read().is_err(), "{name}: {value:?}");
        }
        let f = Fixture::new(false, "");
        fs::write(f.root.join("cgroup.events"), "frozen 1\npopulated 0\n").unwrap();
        assert!(f.read().unwrap().snapshot().nodes()[0].frozen);
    }
    #[test]
    fn unsafe_paths_links_special_files_ownership_and_modes_are_refused() {
        for case in 0..7 {
            let f = Fixture::new(false, "");
            let procs = f.root.join("cgroup.procs");
            match case {
                0 => {
                    fs::remove_file(&procs).unwrap();
                    symlink("cgroup.events", &procs).unwrap();
                }
                1 => {
                    fs::hard_link(&procs, f._temp.path().join("linked")).unwrap();
                }
                2 => {
                    mode(&procs, 0o666);
                }
                3 => {
                    mode(&f.root, 0o777);
                }
                4 => {
                    symlink(&f.root, f.root.join("child")).unwrap();
                }
                5 => {
                    fs::remove_file(&procs).unwrap();
                    let name =
                        std::ffi::CString::new(procs.as_os_str().as_encoded_bytes()).unwrap();
                    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
                }
                _ => {
                    fs::remove_file(&procs).unwrap();
                }
            }
            assert!(f.read().is_err(), "case {case}");
        }
        let f = Fixture::new(false, "");
        assert!(read_fixture_tree(&f.root, uid() + 1, gid()).is_err());
        assert!(read_fixture_tree(&f.root, uid(), gid() + 1).is_err());
        for path in [
            "relative",
            "/sys/fs/cgroup",
            "/proc/self",
            "/var/lib/hydracache-performance",
            "/opt/hydracache-performance",
        ] {
            assert!(read_fixture_tree(Path::new(path), uid(), gid()).is_err());
        }
        let alias = f._temp.path().join("alias");
        symlink(&f.root, &alias).unwrap();
        assert!(read_fixture_tree(&alias, uid(), gid()).is_err());
    }
    #[test]
    fn retained_root_child_and_document_descriptors_detect_drift() {
        for case in 0..5 {
            let f = Fixture::new(false, "");
            node(&f.root.join("child"), false, "");
            let r = f.read().unwrap();
            match case {
                0 => {
                    fs::rename(&f.root, f._temp.path().join("old")).unwrap();
                    node(&f.root, false, "");
                }
                1 => {
                    fs::rename(f.root.join("child"), f._temp.path().join("old-child")).unwrap();
                    node(&f.root.join("child"), false, "");
                }
                2 => {
                    fs::write(
                        f.root.join("child/cgroup.events"),
                        "populated 1\nfrozen 0\n",
                    )
                    .unwrap();
                }
                3 => {
                    let p = f.root.join("cgroup.procs");
                    fs::rename(&p, f._temp.path().join("old-procs")).unwrap();
                    fs::write(&p, "").unwrap();
                    mode(&p, 0o600);
                }
                _ => {
                    node(&f.root.join("new-child"), false, "");
                }
            }
            assert!(r.revalidate().is_err(), "case {case}");
        }
    }
    #[test]
    fn node_depth_pid_entry_and_document_bounds_are_enforced() {
        for case in 0..6 {
            let f = Fixture::new(true, "");
            match case {
                0 => {
                    for n in 0..32 {
                        node(&f.root.join(format!("child-{n}")), false, "");
                    }
                }
                1 => {
                    let mut p = f.root.clone();
                    for _ in 0..9 {
                        p = p.join("child");
                        node(&p, false, "");
                    }
                }
                2 => {
                    fs::write(
                        f.root.join("cgroup.procs"),
                        (1..=257).map(|n| format!("{n}\n")).collect::<String>(),
                    )
                    .unwrap();
                }
                3 => {
                    for n in 0..254 {
                        fs::write(f.root.join(format!("control-{n}")), "").unwrap();
                    }
                }
                4 => {
                    fs::write(f.root.join("cgroup.events"), vec![b'x'; 65537]).unwrap();
                }
                _ => {
                    fs::write(
                        f.root.join("cgroup.procs"),
                        (1..=256).map(|n| format!("{n}\n")).collect::<String>(),
                    )
                    .unwrap();
                    node(&f.root.join("child"), true, "257\n");
                }
            }
            assert!(f.read().is_err(), "case {case}");
        }
        let f = Fixture::new(true, "");
        fs::write(
            f.root.join("cgroup.procs"),
            (1..=256).map(|n| format!("{n}\n")).collect::<String>(),
        )
        .unwrap();
        for n in 0..31 {
            node(&f.root.join(format!("child-{n}")), false, "");
        }
        let read = f.read().unwrap();
        assert_eq!(read.snapshot().nodes().len(), 32);
        assert_eq!(read.snapshot().process_count(), 256);
    }
    #[test]
    fn independent_parallel_readers_keep_identical_fixture_snapshots() {
        let f = Fixture::new(true, "1\n");
        std::thread::scope(|s| {
            let handles: Vec<_> = (0..4)
                .map(|_| {
                    s.spawn(|| {
                        let read = f.read().unwrap();
                        read.revalidate().unwrap();
                        read.snapshot().clone()
                    })
                })
                .collect();
            let snapshots: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
            assert!(snapshots.iter().all(|v| *v == snapshots[0]));
        });
    }
}
