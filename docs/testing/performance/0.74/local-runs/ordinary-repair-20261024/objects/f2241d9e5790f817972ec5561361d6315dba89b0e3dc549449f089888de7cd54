use super::*;
use crate::diagnostic_signer_build::{self as build, SignedSignerBuild, SignerBuildStatement};
use ed25519_dalek::{Signer, SigningKey};
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::PathBuf;

#[path = "../tests/support/diagnostic_gnu_images.rs"]
mod images;

fn lf(value: &impl serde::Serialize) -> Vec<u8> {
    let mut bytes = crate::canonical_json(value).unwrap();
    bytes.push(b'\n');
    bytes
}
struct Fixture {
    temp: tempfile::TempDir,
    images: [Vec<u8>; 4],
    statement: SignerBuildStatement,
    key: SigningKey,
    companion_bytes: [Vec<u8>; 3],
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o755)).unwrap();
        let images = images::images();
        for (index, bytes) in images.iter().enumerate() {
            let path = temp.path().join(format!("image-{index}"));
            fs::write(&path, bytes).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o555)).unwrap();
        }
        let catalog = inspect_gnu_declarations(images.each_ref().map(Vec::as_slice)).unwrap();
        let companion_bytes = [
            b"synthetic lock".to_vec(),
            b"synthetic log".to_vec(),
            b"opaque context".to_vec(),
        ];
        let statement = SignerBuildStatement {
            schema_version: build::SCHEMA.into(),
            repository_id: crate::diagnostic_builder::REPOSITORY_ID,
            role: build::ROLE.into(),
            source_commit: "a".repeat(40),
            source_tree: "b".repeat(40),
            source_clean_before: true,
            source_clean_after: true,
            rustc_version: build::RUSTC_VERSION.into(),
            cargo_version: build::CARGO_VERSION.into(),
            target: "x86_64-unknown-linux-gnu".into(),
            profile: "release".into(),
            features: vec![],
            allocator: "System".into(),
            counting_allocator: false,
            build_command: build::BUILD_COMMAND.iter().map(|s| s.to_string()).collect(),
            binary: ArtifactDigest::of(&images[0]),
            root_lock: ArtifactDigest::of(&companion_bytes[0]),
            build_log: ArtifactDigest::of(&companion_bytes[1]),
            runtime_manifest: ArtifactDigest::of(&lf(&catalog)),
            bootstrap_context: ArtifactDigest::of(&companion_bytes[2]),
        };
        Self {
            temp,
            images,
            statement,
            key: SigningKey::from_bytes(&[93; 32]),
            companion_bytes,
        }
    }
    fn path(&self, role: usize) -> PathBuf {
        self.temp.path().join(format!("image-{role}"))
    }
    fn selection(&self) -> AssertedSignerBuildSelection {
        AssertedSignerBuildSelection::new(self.key.verifying_key(), self.statement.clone()).unwrap()
    }
    fn receipt(&self) -> Vec<u8> {
        self.sign(build::SIGNATURE_DOMAIN, &self.key)
    }
    fn sign(&self, domain: &[u8], key: &SigningKey) -> Vec<u8> {
        let mut message = domain.to_vec();
        message.push(0);
        message.extend(crate::canonical_json(&self.statement).unwrap());
        lf(&SignedSignerBuild {
            statement: self.statement.clone(),
            signature_hex: crate::hex(&key.sign(&message).to_bytes()),
        })
    }
    fn companions(&self) -> SignerRuntimeCompanions<'_> {
        SignerRuntimeCompanions {
            root_lock: &self.companion_bytes[0],
            build_log: &self.companion_bytes[1],
            bootstrap_context: &self.companion_bytes[2],
        }
    }
    fn originals(&self) -> Vec<RootOwnedInputRead> {
        let m = fs::metadata(self.temp.path()).unwrap();
        (0..4)
            .map(|i| {
                crate::diagnostic_root_owned_input::inspect_runtime_fixture(
                    Path::new(&format!("/image-{i}")),
                    ArtifactDigest::of(&self.images[i]),
                    self.temp.path(),
                    (m.uid(), m.gid()),
                )
                .unwrap()
            })
            .collect()
    }
    fn read(&self) -> Result<AssertedGnuRuntimeRead, ArtifactError> {
        join(
            self.originals(),
            self.selection(),
            &self.receipt(),
            &self.companions(),
        )
    }
    fn public_read(
        &self,
        receipt: &[u8],
        lengths: [u64; 4],
    ) -> Result<AssertedGnuRuntimeRead, ArtifactError> {
        let paths = [self.path(0), self.path(1), self.path(2), self.path(3)];
        let inputs = std::array::from_fn(|i| {
            let mut digest = ArtifactDigest::of(&self.images[i]);
            digest.bytes = lengths[i];
            (paths[i].as_path(), digest)
        });
        retain_asserted_gnu_runtime(inputs, self.selection(), receipt, &self.companions())
    }
}

#[test]
fn synthetic_originals_join_signature_and_generated_catalog_without_authority() {
    let f = Fixture::new();
    let mut read = f.read().unwrap();
    for _ in 0..3 {
        let (receipt, catalog) = read.check_original().unwrap();
        assert_eq!(receipt, ArtifactDigest::of(&f.receipt()).sha256);
        assert_eq!(catalog, f.statement.runtime_manifest.sha256);
    }
    assert!(!read.is_refused());
}

#[test]
fn public_constructor_rejects_fixture_and_checks_all_budgets_and_signature_before_io() {
    let f = Fixture::new();
    // The same numeric nonroot temporary files accepted by the private test seam
    // are not accepted by the public root-owned acquisition entry.
    assert!(f.public_read(&f.receipt(), [4096; 4]).is_err());
    for role in 0..4 {
        for bad in [0, 63, MAX_IMAGE + 1, u64::MAX] {
            let mut lengths = [4096; 4];
            lengths[role] = bad;
            assert!(matches!(
                f.public_read(b"malformed", lengths),
                Err(ArtifactError::Contents)
            ));
        }
    }
    // A malformed receipt fails as JSON, rather than reaching the fixture's IO/owner refusal.
    assert!(matches!(
        f.public_read(b"malformed", [4096; 4]),
        Err(ArtifactError::Json(_))
    ));
}

#[test]
fn valid_resigned_opaque_noncanonical_or_other_catalog_manifest_cannot_join() {
    let mut f = Fixture::new();
    let mut other = f.images.clone();
    other[3][4095] ^= 1;
    let c = inspect_gnu_declarations(other.each_ref().map(Vec::as_slice)).unwrap();
    for manifest in [
        b"opaque runtime assertion".to_vec(),
        lf(&c),
        serde_json::to_vec(
            &inspect_gnu_declarations(f.images.each_ref().map(Vec::as_slice)).unwrap(),
        )
        .unwrap(),
    ] {
        f.statement.runtime_manifest = ArtifactDigest::of(&manifest);
        // This receipt is correctly signed under the selected synthetic key.
        assert!(f.read().is_err());
    }
}

#[test]
fn wrong_domain_key_and_signature_refuse_with_matching_originals() {
    let f = Fixture::new();
    for receipt in [
        f.sign(b"other-purpose", &f.key),
        f.sign(build::SIGNATURE_DOMAIN, &SigningKey::from_bytes(&[94; 32])),
    ] {
        assert!(join(f.originals(), f.selection(), &receipt, &f.companions()).is_err());
    }
    let mut raw: SignedSignerBuild = serde_json::from_slice(&f.receipt()).unwrap();
    raw.signature_hex.replace_range(..2, "00");
    assert!(join(f.originals(), f.selection(), &lf(&raw), &f.companions()).is_err());
}

#[test]
fn every_companion_and_original_role_is_bound_without_alternate_binary_bytes() {
    let mut f = Fixture::new();
    for index in 0..3 {
        f.companion_bytes[index][0] ^= 1;
        assert!(f.read().is_err());
        f.companion_bytes[index][0] ^= 1;
    }
    let mut originals = f.originals();
    originals.swap(1, 2);
    assert!(join(originals, f.selection(), &f.receipt(), &f.companions()).is_err());
    f.statement.binary = ArtifactDigest::of(b"unrelated signer");
    assert!(f.read().is_err());
}

#[test]
fn equal_byte_replacement_of_each_role_refuses_all_originals_and_cannot_restore() {
    for role in 0..4 {
        let f = Fixture::new();
        let mut read = f.read().unwrap();
        let saved = f.temp.path().join("saved");
        fs::rename(f.path(role), &saved).unwrap();
        fs::write(f.path(role), &f.images[role]).unwrap();
        fs::set_permissions(f.path(role), fs::Permissions::from_mode(0o555)).unwrap();
        assert!(read.check_original().is_err());
        assert!(read.originals.iter().all(RootOwnedInputRead::is_refused));
        fs::remove_file(f.path(role)).unwrap();
        fs::rename(saved, f.path(role)).unwrap();
        assert!(read.check_original().is_err());
        read.originals.clear(); // sticky refusal must precede any further original IO.
        assert!(read.check_original().is_err());
    }
}

#[test]
fn content_and_ancestor_mode_drift_latch_the_composed_owner() {
    for role in 0..4 {
        let f = Fixture::new();
        let mut read = f.read().unwrap();
        fs::set_permissions(f.path(role), fs::Permissions::from_mode(0o755)).unwrap();
        let mut changed = f.images[role].clone();
        changed[4095] ^= 1;
        fs::write(f.path(role), changed).unwrap();
        assert!(read.check_original().is_err());
        assert!(read.originals.iter().all(RootOwnedInputRead::is_refused));
    }
    let f = Fixture::new();
    let mut read = f.read().unwrap();
    fs::set_permissions(f.temp.path(), fs::Permissions::from_mode(0o775)).unwrap();
    assert!(read.check_original().is_err());
    assert!(read.originals.iter().all(RootOwnedInputRead::is_refused));
}

#[test]
fn seeded_signed_manifest_substitutions_and_independent_thread_local_owners() {
    let mut seed = 0x0740_c077u64;
    let mut f = Fixture::new();
    let original = f.statement.runtime_manifest.clone();
    for _ in 0..64 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut changed = f.images.clone();
        changed[seed as usize % 4][4095] ^= 1;
        let catalog = inspect_gnu_declarations(changed.each_ref().map(Vec::as_slice)).unwrap();
        f.statement.runtime_manifest = ArtifactDigest::of(&lf(&catalog));
        assert!(f.read().is_err());
    }
    f.statement.runtime_manifest = original;
    f.read().unwrap().check_original().unwrap();
    let handles: Vec<_> = (0..4)
        .map(|_| {
            std::thread::spawn(|| {
                let f = Fixture::new();
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
    println!("seed=0x0740c077 signed_catalog_mutations=64 independent_thread_local_owners=4 production_custody=false");
}

#[test]
fn bootstrap_runtime_stronger_path_refuses_opaque_and_public_nonroot_inputs() {
    let mut f = Fixture::new();
    let paths = [f.path(0), f.path(1), f.path(2), f.path(3)];
    let inputs = || std::array::from_fn(|i| (paths[i].as_path(), ArtifactDigest::of(&f.images[i])));
    assert!(matches!(
        retain_asserted_bootstrap_runtime(inputs(), f.selection(), &f.receipt(), &f.companions()),
        Err(ArtifactError::Json(_))
    ));
    f.read().unwrap(); // legacy original runtime join keeps opaque context compatibility.
    let c = serde_json::json!({
        "schema_version": crate::diagnostic_signer_bootstrap_context::SCHEMA,
        "repository_id": f.statement.repository_id,
        "source_commit": f.statement.source_commit, "source_tree": f.statement.source_tree,
        "binary": f.statement.binary, "runtime_manifest": f.statement.runtime_manifest,
        "launcher": ArtifactDigest::of(b"synthetic separate launcher"),
        "workflow_ref": crate::diagnostic_signer_bootstrap_context::WORKFLOW_REF,
        "workflow_sha": "c".repeat(40), "environment": crate::diagnostic_signer_bootstrap_context::ENVIRONMENT,
        "subject": crate::diagnostic_signer_bootstrap_context::SUBJECT,
        "event_name": "workflow_dispatch", "runner_environment": "github-hosted",
        "build_has_signing_authority": false, "build_has_oidc_authority": false,
        "signer_compilation_allowed": false, "signer_dependency_install_allowed": false,
        "signer_checkout_allowed": false, "inherits_unreviewed_environment": false
    });
    f.companion_bytes[2] = lf(&c);
    f.statement.bootstrap_context = ArtifactDigest::of(&f.companion_bytes[2]);
    let selection = f.selection();
    selection.check_receipt(&f.receipt()).unwrap();
    selection
        .check_bootstrap_context(&f.companion_bytes[2])
        .unwrap();
    let mut strong = AssertedBootstrapRuntimeRead {
        original: join(f.originals(), selection, &f.receipt(), &f.companions()).unwrap(),
    };
    strong.check_original().unwrap();
    assert!(!strong.is_refused());
    assert!(retain_asserted_bootstrap_runtime(
        inputs(),
        f.selection(),
        &f.receipt(),
        &f.companions()
    )
    .is_err());
    fs::set_permissions(f.path(0), fs::Permissions::from_mode(0o755)).unwrap();
    let mut changed = f.images[0].clone();
    changed[4095] ^= 1;
    fs::write(f.path(0), changed).unwrap();
    assert!(strong.check_original().is_err());
    assert!(strong.is_refused());
}
