use super::*;
use crate::diagnostic_lease::{DiagnosticClock, DiagnosticCoordinator, DiagnosticStopReason};
use crate::{canonical_json, sha256_hex};
use std::fs::{self, OpenOptions};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{symlink, OpenOptionsExt, PermissionsExt};
use std::path::PathBuf;

pub(crate) struct Fixture {
    pub(crate) temp: tempfile::TempDir,
    pub(crate) root: PathBuf,
    pub(crate) state: crate::diagnostic_lease::DiagnosticState,
}
impl Fixture {
    pub(crate) fn new(boot: &str) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("campaigns");
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(root.join(".host-execution.lock"))
            .unwrap();
        let identity = DiagnosticIdentity {
            lease_id: "a".repeat(64),
            boot_id: boot.into(),
            binary_sha256: "b".repeat(64),
            build_provenance_sha256: "c".repeat(64),
        };
        let clock = DiagnosticClock {
            boot_id: boot.into(),
            monotonic_ns: 1_000_000_000,
        };
        let fence = FixtureFence::acquire(&root).unwrap();
        DiagnosticCoordinator::reserve_fenced(&root, identity, &clock).unwrap();
        let state = crate::diagnostic_lease::read_active(&root)
            .unwrap()
            .unwrap();
        fence.release().unwrap();
        Self { temp, root, state }
    }
    pub(crate) fn persist(&self) {
        let value = serde_json::json!({"schema_version": 1, "state": self.state,
            "state_sha256": sha256_hex(&canonical_json(&self.state).unwrap())});
        self.write("active-diagnostic.json", &value);
    }
    fn write(&self, name: &str, value: &serde_json::Value) {
        let mut bytes = canonical_json(value).unwrap();
        bytes.push(b'\n');
        fs::write(self.root.join(name), bytes).unwrap();
        fs::set_permissions(self.root.join(name), fs::Permissions::from_mode(0o600)).unwrap();
    }
    fn inspect(&self) -> Result<FixtureRecoveryObservation, FixtureRecoveryError> {
        inspect_fixture_recovery(&self.root, &self.state.identity)
    }
    fn inventory(&self) -> Vec<(String, Vec<u8>)> {
        let mut entries: Vec<_> = fs::read_dir(&self.root)
            .unwrap()
            .map(|e| {
                let p = e.unwrap().path();
                (
                    p.file_name().unwrap().to_str().unwrap().to_owned(),
                    fs::read(p).unwrap(),
                )
            })
            .collect();
        entries.sort();
        entries
    }
    pub(crate) fn ledger(&self, pending: bool) {
        use crate::diagnostic_ipc::*;
        let r = DiagnosticRequest {
            schema_version: 1,
            request_id: "d".repeat(64),
            nonce_sha256: "e".repeat(64),
            operation: DiagnosticOperation::Reserve,
            expected_state_revision: 0,
            identity: self.state.identity.clone(),
            source_commit: crate::diagnostic_lease::SOURCE_COMMIT.into(),
            preset: PRESET.into(),
            controller: DiagnosticController {
                repository_id: 1,
                run_id: 2,
                run_attempt: 1,
                actor_id: 3,
            },
            issued_at_unix_seconds: 100,
            expires_at_unix_seconds: 160,
        };
        let response = (!pending).then(|| DiagnosticResponse {
            schema_version: 1,
            request_id: r.request_id.clone(),
            lease_id: r.identity.lease_id.clone(),
            ok: true,
            state: Some(self.state.clone()),
            error_code: None,
        });
        let ledger = serde_json::json!({"entries": [{"request_sha256": sha256_hex(&canonical_json(&r).unwrap()),
            "request": r, "response": response}]});
        self.write(
            "diagnostic-requests-v1.json",
            &serde_json::json!({"schema_version": 1,
            "ledger_sha256": sha256_hex(&canonical_json(&ledger).unwrap()), "ledger": ledger}),
        );
    }
}
const BOOT: &str = "12345678-1234-1234-1234-123456789abc";

#[test]
fn fixture_recovery_quiet_observations_keep_every_byte_and_fence() {
    for terminal in [false, true] {
        let mut f = Fixture::new(BOOT);
        if terminal {
            f.state.stage = DiagnosticStage::Terminal;
            f.state.reason = Some(DiagnosticStopReason::OperatorCancelled);
            f.persist();
        }
        f.ledger(false);
        let before = f.inventory();
        let observed = f.inspect().unwrap();
        assert_eq!(observed.stage, f.state.stage);
        assert_eq!(observed.revision, f.state.revision);
        assert_eq!(f.inventory(), before);
        FixtureFence::acquire(&f.root).unwrap().release().unwrap();
    }
}

#[test]
fn fixture_recovery_each_interrupted_stage_and_unacknowledged_ipc_refuses() {
    for stage in [
        DiagnosticStage::Starting,
        DiagnosticStage::Running,
        DiagnosticStage::Stopping,
    ] {
        let mut f = Fixture::new(BOOT);
        f.state.stage = stage;
        f.state.cleanup_confirmed = false;
        f.state.cell_started_monotonic_ns = Some(1_000_000_000);
        if stage == DiagnosticStage::Stopping {
            f.state.reason = Some(DiagnosticStopReason::OperatorCancelled);
        }
        f.persist();
        let before = f.inventory();
        assert!(matches!(
            f.inspect(),
            Err(FixtureRecoveryError::Interrupted)
        ));
        assert_eq!(f.inventory(), before);
    }
    let f = Fixture::new(BOOT);
    f.ledger(true);
    let before = f.inventory();
    assert!(matches!(f.inspect(), Err(FixtureRecoveryError::Uncertain)));
    assert_eq!(f.inventory(), before);
}

#[test]
fn fixture_recovery_pending_names_and_conflicts_never_follow_or_remove() {
    for name in [
        ".active-diagnostic.pending",
        ".diagnostic-requests.pending",
        "active-campaign",
        "campaign-lifecycle-smoke-v1.json",
        "controller-loss-smoke-v1.json",
    ] {
        for link in [false, true] {
            let f = Fixture::new(BOOT);
            let parent = if name.ends_with("smoke-v1.json") {
                f.root.parent().unwrap()
            } else {
                &f.root
            };
            let p = parent.join(name);
            if link {
                symlink(f.temp.path().join("absent"), &p).unwrap();
            } else {
                fs::write(&p, b"retained-conflict").unwrap();
            }
            assert!(f.inspect().is_err(), "{name}");
            assert!(fs::symlink_metadata(&p).is_ok());
        }
    }
}

#[test]
fn fixture_recovery_exact_identity_missing_state_and_production_paths_refuse() {
    let f = Fixture::new(BOOT);
    for field in 0..4 {
        let mut expected = f.state.identity.clone();
        match field {
            0 => expected.lease_id = "d".repeat(64),
            1 => expected.boot_id = "22345678-1234-1234-1234-123456789abc".into(),
            2 => expected.binary_sha256 = "d".repeat(64),
            _ => expected.build_provenance_sha256 = "d".repeat(64),
        }
        assert!(matches!(
            inspect_fixture_recovery(&f.root, &expected),
            Err(FixtureRecoveryError::Identity)
        ));
    }
    for path in [
        "/var/lib/hydracache-performance/campaigns",
        "/opt/hydracache-performance/campaigns",
        "relative",
        "/tmp/../tmp",
    ] {
        assert!(inspect_fixture_recovery(Path::new(path), &f.state.identity).is_err());
    }
    fs::remove_file(f.root.join("active-diagnostic.json")).unwrap();
    assert!(matches!(f.inspect(), Err(FixtureRecoveryError::Missing)));
}

#[test]
fn fixture_recovery_canonical_hash_schema_and_invalid_ipc_shape_refuse() {
    for name in ["active-diagnostic.json", "diagnostic-requests-v1.json"] {
        for case in 0..6 {
            let f = Fixture::new(BOOT);
            f.ledger(false);
            let p = f.root.join(name);
            let original = fs::read(&p).unwrap();
            let mut v: serde_json::Value = serde_json::from_slice(&original).unwrap();
            match case {
                0 => v["schema_version"] = 2.into(),
                1 => {
                    v[if name.starts_with("active") {
                        "state_sha256"
                    } else {
                        "ledger_sha256"
                    }] = "f".repeat(64).into()
                }
                2 => v["unknown"] = true.into(),
                3 => {
                    fs::write(&p, [original.as_slice(), b" "].concat()).unwrap();
                }
                4 => {
                    if name.starts_with("active") {
                        v["state"]["admission_allowed"] = true.into();
                        v["state_sha256"] =
                            sha256_hex(&canonical_json(&v["state"]).unwrap()).into();
                    } else {
                        v["ledger"]["entries"][0]["response"]["ok"] = false.into();
                        v["ledger_sha256"] =
                            sha256_hex(&canonical_json(&v["ledger"]).unwrap()).into();
                    }
                }
                _ => {
                    fs::write(&p, b"{").unwrap();
                }
            }
            if case != 3 && case != 5 {
                f.write(name, &v);
            }
            assert!(f.inspect().is_err(), "{name} case={case}");
        }
    }
}

#[test]
fn fixture_recovery_unsafe_documents_and_competing_fence_refuse() {
    for name in ["active-diagnostic.json", "diagnostic-requests-v1.json"] {
        for case in 0..5 {
            let f = Fixture::new(BOOT);
            f.ledger(false);
            let p = f.root.join(name);
            match case {
                0 => fs::set_permissions(&p, fs::Permissions::from_mode(0o640)).unwrap(),
                1 => fs::hard_link(&p, f.temp.path().join("alias")).unwrap(),
                2 => fs::write(&p, vec![b'x'; 262_145]).unwrap(),
                3 => {
                    fs::remove_file(&p).unwrap();
                    symlink(f.temp.path().join("absent"), &p).unwrap();
                }
                _ => {
                    fs::remove_file(&p).unwrap();
                    let c = CString::new(p.as_os_str().as_bytes()).unwrap();
                    assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o600) }, 0);
                }
            }
            assert!(f.inspect().is_err(), "{name} case={case}");
        }
    }
    let f = Fixture::new(BOOT);
    let fence = FixtureFence::acquire(&f.root).unwrap();
    assert!(matches!(
        f.inspect(),
        Err(FixtureRecoveryError::Fence(PreparationError::Busy))
    ));
    fence.release().unwrap();
    f.inspect().unwrap();
}

#[test]
fn fixture_recovery_replacements_and_absent_ledger_creation_refuse() {
    for name in [
        "root",
        ".host-execution.lock",
        "active-diagnostic.json",
        "diagnostic-requests-v1.json",
        "in-place",
    ] {
        let f = Fixture::new(BOOT);
        let result = inspect_inner(
            &f.root,
            &f.state.identity,
            |_| {
                if name == "root" {
                    fs::rename(&f.root, f.temp.path().join("displaced")).unwrap();
                    fs::create_dir(&f.root).unwrap();
                    fs::set_permissions(&f.root, fs::Permissions::from_mode(0o700)).unwrap();
                } else if name == "in-place" {
                    let p = f.root.join("active-diagnostic.json");
                    let bytes = fs::read(&p).unwrap();
                    let mut bytes2 = bytes.clone();
                    bytes2[0] = b' ';
                    fs::write(p, bytes2).unwrap();
                } else if name == "diagnostic-requests-v1.json" {
                    f.ledger(false);
                } else {
                    let p = f.root.join(name);
                    let bytes = fs::read(&p).unwrap();
                    fs::rename(&p, f.temp.path().join("displaced")).unwrap();
                    fs::write(p, bytes).unwrap();
                }
            },
            |result| result,
        );
        assert!(result.is_err(), "{name}");
    }
}

#[test]
fn fixture_recovery_callback_holds_host_fence_and_errors_release_it() {
    let f = Fixture::new(BOOT);
    with_fixture_recovery(&f.root, &f.state.identity, |result| {
        result.unwrap();
        assert!(FixtureFence::acquire(&f.root).is_err());
    });
    FixtureFence::acquire(&f.root).unwrap().release().unwrap();
    fs::write(f.root.join(".active-diagnostic.pending"), b"retained").unwrap();
    assert!(f.inspect().is_err());
    FixtureFence::acquire(&f.root).unwrap().release().unwrap();
}
