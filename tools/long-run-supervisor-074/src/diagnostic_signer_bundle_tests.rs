//! Owned nonroot fixtures only; signed context is opaque, never actual custody.
use super::*;
use crate::diagnostic_builder::REPOSITORY_ID;
use crate::diagnostic_signer_build::*;
use ed25519_dalek::{Signer, SigningKey};
use std::os::unix::fs::{symlink, PermissionsExt};

struct Fixture {
    temp: tempfile::TempDir,
    root: PathBuf,
    statement: SignerBuildStatement,
    key: SigningKey,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        for name in ["bundle", "saved"] {
            let path = self.temp.path().join(name);
            if path.is_dir() {
                fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
            }
        }
    }
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("bundle");
        fs::create_dir(&root).unwrap();
        let parts: [&[u8]; 5] = [
            b"synthetic non-ELF executable",
            b"synthetic lock",
            b"invented build log",
            b"opaque runtime",
            b"opaque context",
        ];
        let key = SigningKey::from_bytes(&[93; 32]);
        let statement = SignerBuildStatement {
            schema_version: SCHEMA.into(),
            repository_id: REPOSITORY_ID,
            role: ROLE.into(),
            source_commit: "d".repeat(40),
            source_tree: "e".repeat(40),
            source_clean_before: true,
            source_clean_after: true,
            rustc_version: RUSTC_VERSION.into(),
            cargo_version: CARGO_VERSION.into(),
            target: "x86_64-unknown-linux-gnu".into(),
            profile: "release".into(),
            features: vec![],
            allocator: "System".into(),
            counting_allocator: false,
            build_command: BUILD_COMMAND.iter().map(|s| s.to_string()).collect(),
            binary: ArtifactDigest::of(parts[0]),
            root_lock: ArtifactDigest::of(parts[1]),
            build_log: ArtifactDigest::of(parts[2]),
            runtime_manifest: ArtifactDigest::of(parts[3]),
            bootstrap_context: ArtifactDigest::of(parts[4]),
        };
        for (name, bytes) in [
            (FILES[0], parts[0]),
            (FILES[2], parts[1]),
            (FILES[3], parts[2]),
            (FILES[4], parts[3]),
            (FILES[5], parts[4]),
        ] {
            fs::write(root.join(name), bytes).unwrap();
            fs::set_permissions(root.join(name), fs::Permissions::from_mode(limits(name).1))
                .unwrap();
        }
        let fixture = Self {
            temp,
            root,
            statement,
            key,
        };
        fixture.write_receipt();
        fs::set_permissions(&fixture.root, fs::Permissions::from_mode(0o555)).unwrap();
        fixture
    }
    fn write_receipt(&self) {
        let mut message = SIGNATURE_DOMAIN.to_vec();
        message.push(0);
        message.extend(crate::canonical_json(&self.statement).unwrap());
        let mut raw = crate::canonical_json(&SignedSignerBuild {
            statement: self.statement.clone(),
            signature_hex: crate::hex(&self.key.sign(&message).to_bytes()),
        })
        .unwrap();
        raw.push(b'\n');
        fs::write(self.root.join(FILES[1]), raw).unwrap();
        fs::set_permissions(self.root.join(FILES[1]), fs::Permissions::from_mode(0o444)).unwrap();
    }
    fn selection(&self) -> AssertedSignerBuildSelection {
        AssertedSignerBuildSelection::new(self.key.verifying_key(), self.statement.clone()).unwrap()
    }
    fn owners(&self) -> (u32, u32) {
        let m = fs::metadata(&self.root).unwrap();
        (m.uid(), m.gid())
    }
    fn inspect(&self) -> Result<AssertedSignerBundle, ArtifactError> {
        let (uid, gid) = self.owners();
        retain_asserted_bundle(&self.root, uid, gid, self.selection())
    }
    fn writable_root(&self) {
        fs::set_permissions(&self.root, fs::Permissions::from_mode(0o755)).unwrap();
    }
    fn readonly_root(&self) {
        fs::set_permissions(&self.root, fs::Permissions::from_mode(0o555)).unwrap();
    }
    fn replace(&self, name: &str, bytes: &[u8]) {
        self.writable_root();
        fs::remove_file(self.root.join(name)).unwrap();
        fs::write(self.root.join(name), bytes).unwrap();
        fs::set_permissions(
            self.root.join(name),
            fs::Permissions::from_mode(limits(name).1),
        )
        .unwrap();
        self.readonly_root();
    }
    fn edit(&self, name: &str, bytes: &[u8]) {
        fs::set_permissions(self.root.join(name), fs::Permissions::from_mode(0o644)).unwrap();
        fs::write(self.root.join(name), bytes).unwrap();
        fs::set_permissions(
            self.root.join(name),
            fs::Permissions::from_mode(limits(name).1),
        )
        .unwrap();
    }
}

#[test]
fn signer_bundle_original_positive_cloexec_and_drop_release_owned_descriptors() {
    let f = Fixture::new();
    let mut b = f.inspect().unwrap();
    let receipt = ArtifactDigest::of(&fs::read(f.root.join(FILES[1])).unwrap()).sha256;
    assert_eq!(b.check_original().unwrap(), receipt);
    assert!(!b.is_refused());
    let mut identities = vec![(b.directory.as_raw_fd(), b.stamp.clone())];
    identities.extend(
        b.files
            .iter()
            .map(|p| (p.file.as_raw_fd(), p.stamp.clone())),
    );
    assert_eq!(identities.len(), 7);
    for (fd, _) in &identities {
        // SAFETY: observed, still-owned fixture fd, F_GETFD has no side effects.
        let flags = unsafe { libc::fcntl(*fd, libc::F_GETFD) };
        assert!(flags >= 0);
        assert_ne!(flags & libc::FD_CLOEXEC, 0);
    }
    drop(b);
    for (fd, original) in identities {
        let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
        // SAFETY: fstat writes the complete struct on success. Descriptor numbers
        // may be reused by parallel tests, so compare inode/dev rather than EBADF.
        if unsafe { libc::fstat(fd, stat.as_mut_ptr()) } == 0 {
            // SAFETY: successful fstat initialized the struct above.
            let m = unsafe { stat.assume_init() };
            assert_ne!((m.st_dev, m.st_ino), (original.dev, original.ino));
        }
    }
}

#[test]
fn signer_bundle_bad_signature_selection_and_each_initial_content_refuse() {
    for name in FILES {
        let f = Fixture::new();
        let mut bytes = fs::read(f.root.join(name)).unwrap();
        bytes[0] ^= 1;
        f.edit(name, &bytes);
        assert!(f.inspect().is_err());
    }
    let f = Fixture::new();
    let (uid, gid) = f.owners();
    let mut expected = f.statement.clone();
    expected.source_commit = "a".repeat(40);
    let selection = AssertedSignerBuildSelection::new(f.key.verifying_key(), expected).unwrap();
    assert!(retain_asserted_bundle(&f.root, uid, gid, selection).is_err());
    let selection = AssertedSignerBuildSelection::new(
        SigningKey::from_bytes(&[94; 32]).verifying_key(),
        f.statement.clone(),
    )
    .unwrap();
    assert!(retain_asserted_bundle(&f.root, uid, gid, selection).is_err());
}

#[test]
fn signer_bundle_initial_empty_oversized_missing_and_extra_entries_refuse() {
    for name in FILES {
        for length in [0, limits(name).0 + 1] {
            let f = Fixture::new();
            let path = f.root.join(name);
            fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
            OpenOptions::new()
                .write(true)
                .open(&path)
                .unwrap()
                .set_len(length)
                .unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(limits(name).1)).unwrap();
            assert!(f.inspect().is_err(), "{name}:{length}");
        }
        let f = Fixture::new();
        f.writable_root();
        fs::remove_file(f.root.join(name)).unwrap();
        f.readonly_root();
        assert!(f.inspect().is_err());
    }
    let f = Fixture::new();
    f.writable_root();
    fs::write(f.root.join("extra"), b"x").unwrap();
    f.readonly_root();
    assert!(f.inspect().is_err());
}

#[test]
fn signer_bundle_initial_permissions_and_asserted_owner_mismatch_refuse() {
    for name in FILES {
        for mode in [0o644, 0o400, 0o4755, 0o777] {
            let f = Fixture::new();
            fs::set_permissions(f.root.join(name), fs::Permissions::from_mode(mode)).unwrap();
            assert!(f.inspect().is_err());
        }
    }
    let f = Fixture::new();
    f.writable_root();
    assert!(f.inspect().is_err());
    f.readonly_root();
    let (uid, gid) = f.owners();
    assert!(retain_asserted_bundle(&f.root, uid.wrapping_add(1), gid, f.selection()).is_err());
    assert!(retain_asserted_bundle(&f.root, uid, gid.wrapping_add(1), f.selection()).is_err());
}

#[test]
fn signer_bundle_symlink_hardlink_directory_and_fifo_children_refuse_without_blocking() {
    for kind in 0..4 {
        let f = Fixture::new();
        let name = FILES[0];
        let path = f.root.join(name);
        f.writable_root();
        let outside = f.temp.path().join("outside");
        fs::write(&outside, b"public fixture").unwrap();
        match kind {
            0 => {
                fs::remove_file(&path).unwrap();
                symlink(&outside, &path).unwrap();
            }
            1 => {
                fs::hard_link(&path, outside.with_extension("link")).unwrap();
            }
            2 => {
                fs::remove_file(&path).unwrap();
                fs::create_dir(&path).unwrap();
            }
            _ => {
                fs::remove_file(&path).unwrap();
                let name = CString::new(path.as_os_str().as_bytes()).unwrap();
                // SAFETY: owned fixture pathname, no real install or secret.
                assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o444) }, 0);
            }
        }
        f.readonly_root();
        assert!(f.inspect().is_err());
    }
}

#[test]
fn signer_bundle_absolute_normal_path_and_symlink_ancestry_refuse() {
    let f = Fixture::new();
    let (uid, gid) = f.owners();
    let alias = f.temp.path().join("alias");
    symlink(&f.root, &alias).unwrap();
    let paths = [
        PathBuf::from("bundle"),
        PathBuf::from("/"),
        f.root.join(".."),
        f.root.join("."),
        alias,
        PathBuf::from(format!("{}/../bundle", f.root.display())),
        PathBuf::from("/tmp/\0refused"),
        PathBuf::from(crate::diagnostic_artifacts::INSTALL_ROOT),
        PathBuf::from(format!("/{}", "a".repeat(MAX_PATH_BYTES))),
    ];
    for path in paths {
        assert!(
            retain_asserted_bundle(&path, uid, gid, f.selection()).is_err(),
            "{path:?}"
        );
    }
    let parent_alias = f.temp.path().join("ancestor");
    symlink(f.temp.path(), &parent_alias).unwrap();
    assert!(retain_asserted_bundle(&parent_alias.join("bundle"), uid, gid, f.selection()).is_err());
}

#[test]
fn signer_bundle_streaming_hash_chunk_boundary_empty_and_read_ceiling() {
    let f = Fixture::new();
    let bytes = vec![0x74; 2 * 65536 + 7];
    f.edit(FILES[0], &bytes);
    let mut original = File::open(f.root.join(FILES[0])).unwrap();
    let (digest, retained) = hash_original(&mut original, bytes.len() as u64, false).unwrap();
    assert_eq!(digest, ArtifactDigest::of(&bytes));
    assert!(retained.is_empty());
    assert!(hash_original(&mut original, bytes.len() as u64 - 1, false).is_err());
    let mut receipt = File::open(f.root.join(FILES[1])).unwrap();
    let raw = fs::read(f.root.join(FILES[1])).unwrap();
    let (digest, retained) = hash_original(&mut receipt, raw.len() as u64, true).unwrap();
    assert_eq!(digest, ArtifactDigest::of(&raw));
    assert_eq!(retained, raw);
    f.edit(FILES[0], b"");
    assert!(hash_original(&mut original, model::MAX_BINARY_BYTES, false).is_err());
}

#[test]
fn signer_bundle_equal_byte_named_replacement_root_replacement_and_sticky_refusal() {
    for name in FILES {
        let f = Fixture::new();
        let mut b = f.inspect().unwrap();
        let bytes = fs::read(f.root.join(name)).unwrap();
        f.replace(name, &bytes);
        // Isolate named-file identity even though the overall root stamp also
        // changed. This private check is not a production repair/override seam.
        let pinned = b.files.iter_mut().find(|p| p.name == name).unwrap();
        assert!(pinned.check(&b.directory).is_err());
        assert!(b.check_original().is_err());
        assert!(b.is_refused());
        assert!(b.check_original().is_err());
    }
    let f = Fixture::new();
    let mut b = f.inspect().unwrap();
    let saved = f.temp.path().join("saved");
    fs::rename(&f.root, &saved).unwrap();
    fs::create_dir(&f.root).unwrap();
    fs::set_permissions(&f.root, fs::Permissions::from_mode(0o555)).unwrap();
    assert!(b.check_original().is_err());
    fs::remove_dir(&f.root).unwrap();
    fs::rename(saved, &f.root).unwrap();
    assert!(b.check_original().is_err());
}

#[test]
fn signer_bundle_in_place_write_metadata_or_namespace_drift_cannot_be_repaired() {
    for name in FILES {
        let f = Fixture::new();
        let mut b = f.inspect().unwrap();
        let original = fs::read(f.root.join(name)).unwrap();
        let mut changed = original.clone();
        changed[0] ^= 1;
        f.edit(name, &changed);
        assert!(b.check_original().is_err());
        f.edit(name, &original);
        assert!(b.check_original().is_err());
        assert!(b.is_refused());
    }
    let f = Fixture::new();
    let mut b = f.inspect().unwrap();
    f.writable_root();
    fs::write(f.root.join("transient"), b"x").unwrap();
    f.readonly_root();
    assert!(b.check_original().is_err());
    f.writable_root();
    fs::remove_file(f.root.join("transient")).unwrap();
    f.readonly_root();
    assert!(b.check_original().is_err());
}

#[test]
fn signer_bundle_seeded_substitutions_and_independent_concurrent_originals() {
    let mut seed = 0x0740_3b23u64;
    eprintln!(
        "seed=0x07403b23 fresh_substitutions=32 independent_originals=4 actual_bootstrap=false"
    );
    for _ in 0..32 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let f = Fixture::new();
        let mut b = f.inspect().unwrap();
        let name = FILES[seed as usize % FILES.len()];
        let mut bytes = fs::read(f.root.join(name)).unwrap();
        let position = (seed >> 8) as usize % bytes.len();
        bytes[position] ^= 1;
        f.edit(name, &bytes);
        assert!(b.check_original().is_err());
        assert!(b.is_refused());
    }
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|_| {
                scope.spawn(|| {
                    let f = Fixture::new();
                    let mut b = f.inspect().unwrap();
                    b.check_original().unwrap();
                    f.edit(FILES[4], b"changed runtime");
                    assert!(b.check_original().is_err());
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
    });
}
