use super::*;
use std::os::unix::fs::PermissionsExt;
#[path = "../tests/support/diagnostic_gnu_images.rs"]
pub(super) mod images;

struct Fixture {
    root: Root,
    path: std::path::PathBuf,
    temp: tempfile::TempDir,
}
fn sources(path: &std::path::Path) -> [Source; 4] {
    std::fs::create_dir(path.join("sources")).unwrap();
    images::images()
        .into_iter()
        .enumerate()
        .map(|(i, bytes)| {
            let path = path.join("sources").join(i.to_string());
            std::fs::write(&path, bytes).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o555)).unwrap();
            Source::capture(File::open(path).unwrap(), unsafe { libc::getuid() }, i == 0).unwrap()
        })
        .collect::<Vec<_>>()
        .try_into()
        .ok()
        .unwrap()
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let sources = sources(temp.path());
        let path = temp.path().join("root");
        let root = Root::create(File::open(temp.path()).unwrap(), c"root".into(), sources).unwrap();
        Self { root, path, temp }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !self.root.cleanup_attempted {
            let _ = self.root.cleanup();
        }
        // Outer unit fixture owns the entire TempDir, including deliberate foreign
        // substitutions. This is NOT the normal Root cleanup algorithm.
        for node in &self.root.dirs {
            let _ = chmod(&node.file, 0o700);
        }
    }
}
#[test]
fn roundtrip_retains_original_sources_and_readonly_copies_then_removes_exact_tree() {
    let mut f = Fixture::new();
    f.root.check().unwrap();
    assert_eq!(f.root.dirs.len(), 4);
    assert_eq!(f.root.files.len(), 4);
    for source in &f.root.sources {
        source.node.check().unwrap();
    }
    f.root.cleanup().unwrap();
    assert!(!f.path.exists());
    assert!(f.temp.path().join("sources/0").exists());
    assert!(f.root.check().is_err());
    assert!(f.root.cleanup().is_err());
}
#[test]
fn collision_is_not_adopted_or_deleted() {
    let f = Fixture::new();
    let other = tempfile::tempdir().unwrap();
    let error = Root::create(
        File::open(f.temp.path()).unwrap(),
        c"root".into(),
        sources(other.path()),
    );
    assert!(error.is_err());
    f.root.checked().unwrap();
}
#[test]
fn original_source_content_and_mode_drift_refuse_stickily() {
    for mutate_content in [false, true] {
        let mut f = Fixture::new();
        let path = f.temp.path().join("sources/2");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        if mutate_content {
            OpenOptions::new()
                .write(true)
                .open(&path)
                .unwrap()
                .write_all_at(b"X", 100)
                .unwrap();
        }
        assert!(f.root.check().is_err());
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o555)).unwrap();
        assert!(f.root.check().is_err());
        f.root.cleanup().unwrap();
    }
}
#[test]
fn copied_content_mode_and_hardlink_drift_refuse() {
    for case in 0..3 {
        let mut f = Fixture::new();
        let path = f.path.join("signer");
        if case == 2 {
            std::fs::hard_link(&path, f.temp.path().join("alias")).unwrap();
        } else {
            chmod(&f.root.files[0].file, 0o755).unwrap();
            if case == 1 {
                OpenOptions::new()
                    .write(true)
                    .open(&path)
                    .unwrap()
                    .write_all_at(b"X", 100)
                    .unwrap();
            }
        }
        assert!(f.root.check().is_err());
        f.root.cleanup().unwrap();
    }
}
#[test]
fn equal_bytes_replacement_refuses_cleanup_without_mutating_foreign_file() {
    let mut f = Fixture::new();
    chmod(&f.root.dirs[0].file, 0o700).unwrap();
    std::fs::rename(f.path.join("signer"), f.temp.path().join("original")).unwrap();
    std::fs::write(f.path.join("signer"), &f.root.sources[0].bytes).unwrap();
    let foreign = Stamp::of(&std::fs::metadata(f.path.join("signer")).unwrap());
    assert!(f.root.check().is_err());
    assert!(f.root.cleanup().is_err());
    assert_eq!(
        foreign,
        Stamp::of(&std::fs::metadata(f.path.join("signer")).unwrap())
    );
    assert!(f.path.join("lib64/ld-linux-x86-64.so.2").exists());
    assert!(f.temp.path().join("original").exists());
}
#[test]
fn foreign_symlink_and_extra_entry_never_removed_by_cleanup() {
    for symlink in [false, true] {
        let mut f = Fixture::new();
        chmod(&f.root.dirs[0].file, 0o700).unwrap();
        let foreign = f.temp.path().join("foreign");
        std::fs::create_dir(&foreign).unwrap();
        std::fs::write(foreign.join("keep"), b"foreign").unwrap();
        if symlink {
            chmod(&f.root.dirs[3].file, 0o700).unwrap();
            std::fs::rename(f.path.join("lib64"), f.temp.path().join("original-lib64")).unwrap();
            std::os::unix::fs::symlink(&foreign, f.path.join("lib64")).unwrap();
        } else {
            std::fs::write(f.path.join("extra"), b"extra").unwrap();
        }
        assert!(f.root.check().is_err());
        assert!(f.root.cleanup().is_err());
        assert_eq!(std::fs::read(foreign.join("keep")).unwrap(), b"foreign");
        assert!(f.path.join("signer").exists());
    }
}
#[test]
fn every_owned_partial_construction_prefix_cleans_only_known_names() {
    for dirs in 1..=4 {
        for files in 0..=if dirs == 4 { 4 } else { 0 } {
            let temp = tempfile::tempdir().unwrap();
            let sources = sources(temp.path());
            let parent = File::open(temp.path()).unwrap();
            let first = Node::new(mkdir(&parent, c"partial").unwrap()).unwrap();
            let mut root = Root {
                parent,
                name: c"partial".into(),
                dirs: vec![first],
                files: vec![],
                sources,
                refused: false,
                cleanup_attempted: false,
                _not_send_sync: PhantomData,
            };
            for (parent, name) in DIRS.iter().take(dirs - 1) {
                root.dirs
                    .push(Node::new(mkdir(&root.dirs[*parent].file, name).unwrap()).unwrap());
            }
            for (parent, name) in FILES.iter().take(files) {
                let fd = unsafe {
                    libc::openat(
                        root.dirs[*parent].file.as_raw_fd(),
                        name.as_ptr(),
                        libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_CLOEXEC,
                        0o600,
                    )
                };
                assert!(fd >= 0);
                drop(unsafe { File::from_raw_fd(fd) });
                root.files.push(
                    Node::new(open_at(&root.dirs[*parent].file, name, false).unwrap()).unwrap(),
                );
            }
            drop(root);
            assert!(!temp.path().join("partial").exists());
            assert!(temp.path().join("sources").exists());
        }
    }
}
#[test]
fn source_size_type_permissions_and_unknown_graph_refuse_before_root_creation() {
    let f = Fixture::new();
    let path = f.temp.path().join("invalid");
    for size in [0, 63, MAX_IMAGE + 1] {
        let file = File::create(&path).unwrap();
        file.set_len(size).unwrap();
        drop(file);
        assert!(
            Source::capture(File::open(&path).unwrap(), unsafe { libc::getuid() }, false).is_err()
        );
    }
    assert!(Source::capture(
        File::open(f.temp.path()).unwrap(),
        unsafe { libc::getuid() },
        false
    )
    .is_err());
    let temp = tempfile::tempdir().unwrap();
    let mut original = sources(temp.path());
    original[0].bytes[0] = 0;
    assert!(Root::create(
        File::open(temp.path()).unwrap(),
        c"invalid-root".into(),
        original
    )
    .is_err());
    assert!(!temp.path().join("invalid-root").exists());
}
#[test]
fn four_independent_builders_keep_distinct_originals() {
    let joins: Vec<_> = (0..4)
        .map(|_| {
            std::thread::spawn(|| {
                let mut f = Fixture::new();
                f.root.check().unwrap();
                f.root.cleanup().unwrap();
            })
        })
        .collect();
    let results: Vec<_> = joins.into_iter().map(|j| j.join()).collect();
    assert!(results.into_iter().all(|r| r.is_ok()));
    println!("independent_fixed_roots=4 synthetic_graphs=true admission=false");
}

#[test]
fn replaced_root_name_preserves_foreign_and_original_trees() {
    let mut f = Fixture::new();
    let original = f.temp.path().join("original-root");
    std::fs::rename(&f.path, &original).unwrap();
    std::fs::create_dir(&f.path).unwrap();
    std::fs::write(f.path.join("foreign"), b"keep").unwrap();
    assert!(f.root.check().is_err());
    assert!(f.root.cleanup().is_err());
    assert_eq!(std::fs::read(f.path.join("foreign")).unwrap(), b"keep");
    assert!(original.join("signer").exists());
}

#[test]
fn lost_cloexec_and_source_path_replacement_never_recapture_originals() {
    for index in 0..4 {
        let mut f = Fixture::new();
        assert_eq!(
            unsafe { libc::fcntl(f.root.files[index].file.as_raw_fd(), libc::F_SETFD, 0) },
            0
        );
        assert!(f.root.check().is_err());
        assert_eq!(
            unsafe {
                libc::fcntl(
                    f.root.files[index].file.as_raw_fd(),
                    libc::F_SETFD,
                    libc::FD_CLOEXEC,
                )
            },
            0
        );
        assert!(f.root.check().is_err());
        f.root.cleanup().unwrap();
    }
    let mut f = Fixture::new();
    let source = f.temp.path().join("sources/0");
    let id = f.root.sources[0].node.stamp.clone();
    std::fs::rename(&source, f.temp.path().join("old-source")).unwrap();
    std::fs::write(&source, b"foreign non-ELF pathname replacement").unwrap();
    assert!(id.same_object(&Stamp::of(&f.root.sources[0].node.file.metadata().unwrap())));
    assert!(!id.same_object(&Stamp::of(&std::fs::metadata(source).unwrap())));
    // Source paths are acquisition inputs, not retained name bindings. A rename
    // may or may not change the old inode's ctime on this filesystem. Preserve
    // exact stamp semantics; never recapture the new pathname in either case.
    let original_unchanged = id == Stamp::of(&f.root.sources[0].node.file.metadata().unwrap());
    assert_eq!(f.root.check().is_ok(), original_unchanged);
    assert_eq!(contents(&f.root.files[0]).unwrap(), f.root.sources[0].bytes);
    f.root.cleanup().unwrap();
}
