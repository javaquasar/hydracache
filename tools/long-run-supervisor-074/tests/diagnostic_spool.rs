//! Temporary filesystem + SYNTHETIC signer/exit claims; no measured process.
#[cfg(target_os = "linux")]
mod linux {
    use ed25519_dalek::{Signer, SigningKey};
    use hydracache_long_run_supervisor_074::diagnostic_artifacts::*;
    use hydracache_long_run_supervisor_074::diagnostic_lease::{
        CellIntent, DiagnosticIdentity, BINARY_PATH, SOURCE_COMMIT,
    };
    use hydracache_long_run_supervisor_074::diagnostic_receipts::{
        Decision, TerminalSummary, STREAM_BYTES,
    };
    use hydracache_long_run_supervisor_074::diagnostic_spool::*;
    use std::collections::BTreeMap;
    use std::fs;
    use std::os::unix::fs::{symlink, PermissionsExt};
    use std::path::{Path, PathBuf};

    fn fixture() -> (VerifiedBuild, CellIntent, TerminalSummary) {
        let configs: BTreeMap<String, &[u8]> = BTreeMap::from([
            (
                "embedded".into(),
                include_bytes!(
                    "../../../docs/testing/performance/0.74/rental-pilot-draft/embedded.json"
                )
                .as_slice(),
            ),
            (
                "direct".into(),
                include_bytes!(
                    "../../../docs/testing/performance/0.74/rental-pilot-draft/direct.json"
                )
                .as_slice(),
            ),
            (
                "resp2".into(),
                include_bytes!(
                    "../../../docs/testing/performance/0.74/rental-pilot-draft/resp2.json"
                )
                .as_slice(),
            ),
            (
                "resp3".into(),
                include_bytes!(
                    "../../../docs/testing/performance/0.74/rental-pilot-draft/resp3.json"
                )
                .as_slice(),
            ),
        ]);
        let statement = BuildStatement {
            schema_version: 1,
            repository_id: 74,
            builder_id: "synthetic-test-builder".into(),
            source_commit: SOURCE_COMMIT.into(),
            source_tree: SOURCE_TREE.into(),
            source_clean_before: true,
            source_clean_after: true,
            rustc_version: "rustc 1.94.0 (4a4ef493e 2026-03-02)".into(),
            cargo_version: "cargo 1.94.0 (85eff7c80 2026-01-15)".into(),
            target: "x86_64-unknown-linux-gnu".into(),
            profile: "release".into(),
            features: vec![],
            allocator: "System".into(),
            counting_allocator: false,
            build_command: BUILD_COMMAND.iter().map(|s| s.to_string()).collect(),
            binary: ArtifactDigest::of(b"SYNTHETIC-not-executable"),
            root_lock: ArtifactDigest::of(include_bytes!("../../../Cargo.lock")),
            observer_lock: ArtifactDigest::of(include_bytes!(
                "../../get-owner-scheduled-controls-074/Cargo.lock"
            )),
            build_log: ArtifactDigest::of(b"SYNTHETIC-not-a-build-log"),
            configs: configs
                .into_iter()
                .map(|(k, v)| (k, ArtifactDigest::of(v)))
                .collect(),
        };
        let key = SigningKey::from_bytes(&[74; 32]);
        let signed = SignedBuild {
            signature_hex: key
                .sign(&signing_message(&statement).unwrap())
                .to_bytes()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
            statement,
        };
        let raw = receipt_bytes(&signed).unwrap();
        let id = DiagnosticIdentity {
            lease_id: "a".repeat(64),
            boot_id: "12345678-1234-1234-1234-123456789abc".into(),
            binary_sha256: signed.statement.binary.sha256.clone(),
            build_provenance_sha256: ArtifactDigest::of(&raw).sha256,
        };
        let build = verify_receipt(
            &raw,
            &id,
            &BuildTrust {
                key: key.verifying_key(),
                repository_id: 74,
                builder_id: "synthetic-test-builder".into(),
            },
        )
        .unwrap();
        let unit = format!("hydracache-diagnostic-074-{}-1.service", id.lease_id);
        let intent = CellIntent {
            lease_id: id.lease_id,
            boot_id: id.boot_id.clone(),
            surface: "embedded".into(),
            unit_name: unit.clone(),
            cgroup_path: format!("/system.slice/{unit}"),
            binary_path: BINARY_PATH.into(),
            binary_sha256: id.binary_sha256,
            config_path: format!("{INSTALL_ROOT}/embedded.json"),
            config_sha256: signed.statement.configs["embedded"].sha256.clone(),
            source_commit: SOURCE_COMMIT.into(),
            maximum_runtime_seconds: 60,
        };
        let terminal = TerminalSummary {
            unit_name: unit,
            boot_id: id.boot_id,
            cgroup_path: intent.cgroup_path.clone(),
            cgroup_inode: 74,
            populated: false,
            exit_code: Some(1),
            term_signal: None,
        };
        (build, intent, terminal)
    }
    fn uid() -> u32 {
        unsafe { libc::geteuid() }
    }
    fn gid() -> u32 {
        unsafe { libc::getegid() }
    }
    fn mode(path: &Path, mode: u32) {
        fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
    }
    struct Dirs {
        _temp: tempfile::TempDir,
        source: PathBuf,
        output: PathBuf,
    }
    impl Dirs {
        fn new(stdout: &[u8], stderr: &[u8]) -> Self {
            let temp = tempfile::tempdir().unwrap();
            let source = temp.path().join("source");
            let output = temp.path().join("output");
            fs::create_dir(&source).unwrap();
            fs::create_dir(&output).unwrap();
            mode(&source, 0o700);
            mode(&output, 0o700);
            fs::write(source.join("stdout.json"), stdout).unwrap();
            fs::write(source.join("stderr.log"), stderr).unwrap();
            mode(&source.join("stdout.json"), 0o600);
            mode(&source.join("stderr.log"), 0o600);
            Self {
                _temp: temp,
                source,
                output,
            }
        }
    }
    impl Drop for Dirs {
        fn drop(&mut self) {
            // Restore permissions only in our exact temporary fixture for TempDir cleanup.
            if let Ok(entries) = fs::read_dir(&self.output) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if fs::symlink_metadata(&path)
                        .is_ok_and(|m| m.is_dir() && !m.file_type().is_symlink())
                    {
                        mode(&path, 0o700);
                    }
                }
            }
        }
    }

    #[test]
    fn fixture_publication_is_byte_exact_and_replay_never_overwrites() {
        let d = Dirs::new(b"", b"synthetic startup failure\0\xff");
        let (b, i, t) = fixture();
        let mut spool = inspect_fixture_spool(&d.source, uid(), gid()).unwrap();
        let first = spool.publish_fixture(&d.output, &b, &i, &t, None).unwrap();
        assert!(!first.was_replay());
        assert_eq!(
            first.manifest().packet.as_ref().unwrap().decision,
            Decision::FailedProcess
        );
        assert!(first.manifest().fixture_only && !first.manifest().promotable);
        assert_eq!(
            fs::read(first.path().join("stderr.prefix")).unwrap(),
            b"synthetic startup failure\0\xff"
        );
        let before = fs::read(first.path().join("manifest.json")).unwrap();
        let second = spool.publish_fixture(&d.output, &b, &i, &t, None).unwrap();
        assert!(second.was_replay());
        assert_eq!(first.manifest_digest(), second.manifest_digest());
        assert_eq!(
            before,
            fs::read(first.path().join("manifest.json")).unwrap()
        );
        verify_fixture_packet(
            first.path(),
            uid(),
            gid(),
            &b,
            &i,
            &t,
            first.manifest_digest(),
        )
        .unwrap();
        let other = Dirs::new(b"different", b"");
        let mut different = inspect_fixture_spool(&other.source, uid(), gid()).unwrap();
        assert!(different
            .publish_fixture(&d.output, &b, &i, &t, None)
            .is_err());
        assert_eq!(
            before,
            fs::read(first.path().join("manifest.json")).unwrap()
        );
    }

    #[test]
    fn overflow_retains_only_bounded_prefix_and_original_size_without_full_hash_claim() {
        let d = Dirs::new(b"", b"");
        let (b, i, t) = fixture();
        fs::OpenOptions::new()
            .write(true)
            .open(d.source.join("stdout.json"))
            .unwrap()
            .set_len(STREAM_BYTES as u64 + 123)
            .unwrap();
        let mut spool = inspect_fixture_spool(&d.source, uid(), gid()).unwrap();
        let p = spool.publish_fixture(&d.output, &b, &i, &t, None).unwrap();
        assert_eq!(p.manifest().decision, SpoolDecision::OverflowRetained);
        assert!(p.manifest().packet.is_none());
        assert!(!p.manifest().stdout.complete);
        assert_eq!(
            p.manifest().stdout.observed_bytes,
            STREAM_BYTES as u64 + 123
        );
        assert_eq!(p.manifest().stdout.prefix.bytes, STREAM_BYTES as u64);
        assert_eq!(
            fs::metadata(p.path().join("stdout.prefix")).unwrap().len(),
            STREAM_BYTES as u64
        );
        verify_fixture_packet(p.path(), uid(), gid(), &b, &i, &t, p.manifest_digest()).unwrap();
    }

    #[test]
    fn spool_refuses_links_special_files_wrong_modes_owners_and_extra_entries() {
        for case in 0..8 {
            let d = Dirs::new(b"x", b"");
            let file = d.source.join("stdout.json");
            match case {
                0 => {
                    fs::remove_file(&file).unwrap();
                    symlink("stderr.log", &file).unwrap();
                }
                1 => {
                    fs::hard_link(&file, d._temp.path().join("hardlink")).unwrap();
                }
                2 => mode(&file, 0o644),
                3 => mode(&d.source, 0o755),
                4 => {
                    fs::write(d.source.join("extra"), b"x").unwrap();
                }
                5 => {
                    fs::remove_file(&file).unwrap();
                    let c = std::ffi::CString::new(file.as_os_str().as_encoded_bytes()).unwrap();
                    assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o600) }, 0);
                }
                6 => {
                    fs::remove_file(&file).unwrap();
                    fs::create_dir(&file).unwrap();
                }
                _ => {
                    fs::remove_file(&file).unwrap();
                }
            }
            assert!(
                inspect_fixture_spool(&d.source, uid(), gid()).is_err(),
                "case {case}"
            );
        }
        let d = Dirs::new(b"x", b"");
        assert!(inspect_fixture_spool(&d.source, uid() + 1, gid()).is_err());
        assert!(inspect_fixture_spool(&d.source, uid(), gid() + 1).is_err());
        let alias = d._temp.path().join("alias");
        symlink(&d.source, &alias).unwrap();
        assert!(inspect_fixture_spool(&alias, uid(), gid()).is_err());
        assert!(inspect_fixture_spool(Path::new("relative"), uid(), gid()).is_err());
        assert!(
            inspect_fixture_spool(Path::new("/var/lib/hydracache-performance"), uid(), gid())
                .is_err()
        );
    }

    #[test]
    fn pinned_spool_detects_replacement_and_in_place_drift() {
        for change in 0..4 {
            let d = Dirs::new(b"x", b"");
            let mut s = inspect_fixture_spool(&d.source, uid(), gid()).unwrap();
            match change {
                0 => fs::write(d.source.join("stdout.json"), b"y").unwrap(),
                1 => {
                    fs::rename(d.source.join("stdout.json"), d._temp.path().join("old")).unwrap();
                    fs::write(d.source.join("stdout.json"), b"x").unwrap();
                    mode(&d.source.join("stdout.json"), 0o600);
                }
                2 => mode(&d.source.join("stderr.log"), 0o400),
                _ => {
                    fs::rename(&d.source, d._temp.path().join("old-source")).unwrap();
                    fs::create_dir(&d.source).unwrap();
                    mode(&d.source, 0o700);
                }
            }
            assert!(s.revalidate().is_err(), "change {change}");
            let (b, i, t) = fixture();
            assert!(s.publish_fixture(&d.output, &b, &i, &t, None).is_err());
            assert_eq!(fs::read_dir(&d.output).unwrap().count(), 0);
        }
    }

    #[test]
    fn every_crash_window_retains_pending_or_reverifies_same_published_packet() {
        for fault in [
            FaultPoint::AfterPending,
            FaultPoint::AfterStdout,
            FaultPoint::AfterStderr,
            FaultPoint::AfterManifest,
            FaultPoint::AfterReadonly,
            FaultPoint::AfterRename,
        ] {
            let d = Dirs::new(b"torn-json", b"retained");
            let (b, i, t) = fixture();
            let mut spool = inspect_fixture_spool(&d.source, uid(), gid()).unwrap();
            assert!(matches!(
                spool.publish_fixture(&d.output, &b, &i, &t, Some(fault)),
                Err(SpoolError::Injected(_))
            ));
            assert_eq!(fs::read_dir(&d.output).unwrap().count(), 1);
            if fault == FaultPoint::AfterRename {
                let p = spool.publish_fixture(&d.output, &b, &i, &t, None).unwrap();
                assert!(p.was_replay());
                assert_eq!(
                    p.manifest().packet.as_ref().unwrap().decision,
                    Decision::InvalidContent
                );
            } else {
                assert!(spool.publish_fixture(&d.output, &b, &i, &t, None).is_err());
            }
        }
    }

    #[test]
    fn offline_verification_requires_external_digest_and_reconciles_content() {
        for change in 0..10 {
            let d = Dirs::new(b"torn", b"opaque");
            let (b, i, t) = fixture();
            let mut s = inspect_fixture_spool(&d.source, uid(), gid()).unwrap();
            let p = s.publish_fixture(&d.output, &b, &i, &t, None).unwrap();
            match change {
                0 => {
                    let mut digest = p.manifest_digest().clone();
                    digest.sha256 = "b".repeat(64);
                    assert!(
                        verify_fixture_packet(p.path(), uid(), gid(), &b, &i, &t, &digest).is_err()
                    );
                    continue;
                }
                1 => {
                    let file = p.path().join("stdout.prefix");
                    mode(&file, 0o600);
                    fs::write(&file, b"changed").unwrap();
                    mode(&file, 0o400);
                }
                2 => {
                    let file = p.path().join("manifest.json");
                    mode(&file, 0o600);
                    fs::write(&file, b"{}\n").unwrap();
                    mode(&file, 0o400);
                }
                3 => {
                    mode(p.path(), 0o700);
                    fs::write(p.path().join("extra"), b"x").unwrap();
                    mode(p.path(), 0o500);
                }
                4 => {
                    mode(&p.path().join("stderr.prefix"), 0o600);
                }
                5 => {
                    let mut other = t.clone();
                    other.cgroup_inode += 1;
                    assert!(verify_fixture_packet(
                        p.path(),
                        uid(),
                        gid(),
                        &b,
                        &i,
                        &other,
                        p.manifest_digest()
                    )
                    .is_err());
                    continue;
                }
                _ => {
                    // A matching external digest cannot legitimize forged semantics
                    // or bypass strict canonical parsing.
                    let file = p.path().join("manifest.json");
                    let original = fs::read(&file).unwrap();
                    let mut value: serde_json::Value = serde_json::from_slice(&original).unwrap();
                    match change {
                        6 => value["promotable"] = true.into(),
                        7 => value["schema_version"] = 2.into(),
                        8 => value["stdout"]["observed_bytes"] = 99.into(),
                        _ => value["unknown_field"] = true.into(),
                    }
                    let mut forged = serde_json::to_vec(&value).unwrap();
                    forged.push(b'\n');
                    mode(&file, 0o600);
                    fs::write(&file, &forged).unwrap();
                    mode(&file, 0o400);
                    assert!(
                        verify_fixture_packet(
                            p.path(),
                            uid(),
                            gid(),
                            &b,
                            &i,
                            &t,
                            &ArtifactDigest::of(&forged)
                        )
                        .is_err(),
                        "forgery {change}"
                    );
                    continue;
                }
            }
            assert!(
                verify_fixture_packet(p.path(), uid(), gid(), &b, &i, &t, p.manifest_digest())
                    .is_err(),
                "change {change}"
            );
        }
    }

    #[test]
    fn concurrent_publication_has_one_generation_and_no_replacement() {
        let d = Dirs::new(b"torn", b"opaque");
        let (b, i, t) = fixture();
        let gate = std::sync::Barrier::new(4);
        std::thread::scope(|scope| {
            let handles: Vec<_> = (0..4)
                .map(|_| {
                    scope.spawn(|| {
                        let mut s = inspect_fixture_spool(&d.source, uid(), gid()).unwrap();
                        gate.wait();
                        s.publish_fixture(&d.output, &b, &i, &t, None)
                            .ok()
                            .map(|p| p.manifest_digest().clone())
                    })
                })
                .collect();
            let digests: Vec<_> = handles
                .into_iter()
                .filter_map(|h| h.join().unwrap())
                .collect();
            assert!(!digests.is_empty());
            assert!(digests.iter().all(|v| *v == digests[0]));
        });
        assert_eq!(fs::read_dir(&d.output).unwrap().count(), 1);
    }
}
