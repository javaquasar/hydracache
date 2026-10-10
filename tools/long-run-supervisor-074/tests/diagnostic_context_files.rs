#![cfg(target_os = "linux")]
//! Actual local context with owned files and synthetic issuer pins, not production enrollment.
use ed25519_dalek::{Signer, SigningKey};
use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::{
    inspect_context_fixed_files, inspect_context_fixture_files, ContextFilesError,
};
use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::WorkerContextError;
use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_files::WorkerFilesError;
use hydracache_long_run_supervisor_074::diagnostic_worker_policy::*;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::os::unix::fs::{symlink, MetadataExt, PermissionsExt};
use std::path::Path;

const PASSWD: &[u8] =
    b"root:x:0:0::/root:/bin/sh\nhydracache-perf:x:986:986::/nonexistent:/bin/false\n";
const GROUP: &[u8] = b"root:x:0:\nhydracache-perf:x:986:\nextra:x:987:hydracache-perf\n";
fn canonical(value: &impl Serialize) -> Vec<u8> {
    let mut b = serde_json::to_vec(&serde_json::to_value(value).unwrap()).unwrap();
    b.push(b'\n');
    b
}
fn hash(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn owner() -> (u32, u32) {
    let ids = unsafe { (libc::geteuid(), libc::getegid()) };
    assert!(
        ids.0 > 0 && ids.1 > 0,
        "owned fixtures require an unprivileged test user"
    );
    ids
}
struct Fixture {
    directory: tempfile::TempDir,
    policy: WorkerPolicy,
    bytes: Vec<u8>,
    trust: WorkerPolicyTrust,
    host: AssertedWorkerHost,
}
impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700)).unwrap();
        for (name, bytes) in [("passwd", PASSWD), ("group", GROUP)] {
            fs::write(directory.path().join(name), bytes).unwrap();
            fs::set_permissions(
                directory.path().join(name),
                fs::Permissions::from_mode(0o600),
            )
            .unwrap();
        }
        let ns = |name: &str| {
            let m = fs::metadata(format!("/proc/thread-self/ns/{name}")).unwrap();
            NamespaceIdentity {
                device: m.dev(),
                inode: m.ino(),
            }
        };
        let host = AssertedWorkerHost {
            machine_id: fs::read_to_string("/etc/machine-id")
                .unwrap()
                .trim_end_matches('\n')
                .into(),
            boot_id: fs::read_to_string("/proc/sys/kernel/random/boot_id")
                .unwrap()
                .trim_end_matches('\n')
                .into(),
            user_namespace: ns("user"),
            mount_namespace: ns("mnt"),
        };
        let policy = WorkerPolicy {
            schema_version: WORKER_POLICY_SCHEMA.into(),
            repository_id: 1_217_101_761,
            purpose: "diagnostic-worker-only".into(),
            account_source: "local-files".into(),
            account: "hydracache-perf".into(),
            group: "hydracache-perf".into(),
            policy_epoch: 7,
            uid: 986,
            gid: 986,
            supplementary_gids: vec![987],
            machine_id: host.machine_id.clone(),
            boot_id: host.boot_id.clone(),
            user_namespace: host.user_namespace,
            mount_namespace: host.mount_namespace,
            passwd_sha256: hash(PASSWD),
            group_sha256: hash(GROUP),
        };
        let mut f = Self {
            directory,
            policy,
            bytes: vec![],
            trust: WorkerPolicyTrust::new(
                SigningKey::from_bytes(&[74; 32]).verifying_key(),
                &"a".repeat(64),
                7,
            )
            .unwrap(),
            host,
        };
        f.sign();
        f
    }
    fn sign(&mut self) {
        let key = SigningKey::from_bytes(&[74; 32]);
        self.host = AssertedWorkerHost {
            machine_id: self.policy.machine_id.clone(),
            boot_id: self.policy.boot_id.clone(),
            user_namespace: self.policy.user_namespace,
            mount_namespace: self.policy.mount_namespace,
        };
        let body = canonical(&self.policy);
        self.trust =
            WorkerPolicyTrust::new(key.verifying_key(), &hash(&body), self.policy.policy_epoch)
                .unwrap();
        let mut message = WORKER_POLICY_DOMAIN.to_vec();
        message.push(0);
        message.extend(body);
        self.bytes = canonical(&SignedWorkerPolicy {
            schema_version: SIGNED_WORKER_POLICY_SCHEMA.into(),
            policy: self.policy.clone(),
            signature_hex: key
                .sign(&message)
                .to_bytes()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
        });
    }
    fn checked(&self) -> CheckedWorkerPolicy {
        verify_worker_policy(&self.bytes, &self.trust, &self.host).unwrap()
    }
    fn path(&self) -> &Path {
        self.directory.path()
    }
}

#[test]
fn context_files_owned_roundtrip_and_successful_drop_preserve_policy() {
    let f = Fixture::new();
    let mut p = f.checked();
    {
        let mut read = inspect_context_fixture_files(
            f.path(),
            owner().0,
            owner().1,
            &mut p,
            &f.bytes,
            &f.trust,
            &f.host,
        )
        .unwrap();
        for _ in 0..4 {
            read.revalidate(&f.bytes, &f.trust, &f.host).unwrap();
        }
        assert!(!read.is_refused());
    }
    p.revalidate(&f.bytes, &f.trust, &f.host).unwrap();
}
#[test]
fn context_files_refusal_and_revocation_precede_any_account_path_access() {
    let f = Fixture::new();
    let mut p = f.checked();
    p.revalidate(b"bad", &f.trust, &f.host).unwrap_err();
    assert!(matches!(
        inspect_context_fixed_files(&mut p, &f.bytes, &f.trust, &f.host),
        Err(ContextFilesError::Refused)
    ));
    let mut p = f.checked();
    let revoked = WorkerPolicyTrust::new(
        SigningKey::from_bytes(&[74; 32]).verifying_key(),
        &hash(&canonical(&f.policy)),
        8,
    )
    .unwrap();
    assert!(matches!(
        inspect_context_fixture_files(
            Path::new("/missing/context-files"),
            owner().0,
            owner().1,
            &mut p,
            &f.bytes,
            &revoked,
            &f.host
        ),
        Err(ContextFilesError::Context(WorkerContextError::Policy(
            WorkerPolicyError::Revoked
        )))
    ));
    assert!(p.is_refused());
}
#[test]
fn context_files_actual_context_failure_precedes_missing_file_path() {
    let mut f = Fixture::new();
    f.policy.mount_namespace.inode += 1;
    f.sign();
    let mut p = f.checked();
    assert!(matches!(
        inspect_context_fixture_files(
            Path::new("/missing/context-files"),
            owner().0,
            owner().1,
            &mut p,
            &f.bytes,
            &f.trust,
            &f.host
        ),
        Err(ContextFilesError::Context(WorkerContextError::Drift))
    ));
    assert!(p.is_refused());
}
#[test]
fn context_files_constructor_open_mapping_and_document_failures_latch_original() {
    for kind in 0..4 {
        let mut f = Fixture::new();
        match kind {
            0 => {
                fs::remove_file(f.path().join("group")).unwrap();
            }
            1 => {
                fs::write(f.path().join("passwd"), b"malformed\n").unwrap();
                f.policy.passwd_sha256 = hash(b"malformed\n");
                f.sign();
            }
            2 => {
                f.policy.uid += 1;
                f.sign();
            }
            _ => {
                fs::set_permissions(f.path().join("group"), fs::Permissions::from_mode(0o666))
                    .unwrap();
            }
        }
        let mut p = f.checked();
        let error = inspect_context_fixture_files(
            f.path(),
            owner().0,
            owner().1,
            &mut p,
            &f.bytes,
            &f.trust,
            &f.host,
        )
        .err()
        .unwrap();
        assert!(matches!(error, ContextFilesError::Files(_)));
        assert!(p.is_refused());
    }
}
#[test]
fn context_files_changed_names_contents_and_restoration_cannot_refresh() {
    for replace in [false, true] {
        let f = Fixture::new();
        let mut p = f.checked();
        {
            let mut read = inspect_context_fixture_files(
                f.path(),
                owner().0,
                owner().1,
                &mut p,
                &f.bytes,
                &f.trust,
                &f.host,
            )
            .unwrap();
            if replace {
                fs::rename(f.path().join("group"), f.path().join("original")).unwrap();
                fs::write(f.path().join("group"), GROUP).unwrap();
                fs::set_permissions(f.path().join("group"), fs::Permissions::from_mode(0o600))
                    .unwrap();
            } else {
                fs::write(f.path().join("group"), b"changed\n").unwrap();
            }
            assert!(matches!(
                read.revalidate(&f.bytes, &f.trust, &f.host),
                Err(ContextFilesError::Files(WorkerFilesError::Drift))
            ));
            if replace {
                fs::remove_file(f.path().join("group")).unwrap();
                fs::rename(f.path().join("original"), f.path().join("group")).unwrap();
            } else {
                fs::write(f.path().join("group"), GROUP).unwrap();
            }
            assert!(matches!(
                read.revalidate(&f.bytes, &f.trust, &f.host),
                Err(ContextFilesError::Refused)
            ));
        }
        assert!(p.is_refused());
    }
}
#[test]
fn context_files_policy_failure_after_open_latches_through_drop() {
    let f = Fixture::new();
    let mut p = f.checked();
    {
        let mut read = inspect_context_fixture_files(
            f.path(),
            owner().0,
            owner().1,
            &mut p,
            &f.bytes,
            &f.trust,
            &f.host,
        )
        .unwrap();
        let mut changed = f.host.clone();
        changed.user_namespace.inode += 1;
        assert!(matches!(
            read.revalidate(&f.bytes, &f.trust, &changed),
            Err(ContextFilesError::Context(WorkerContextError::Policy(
                WorkerPolicyError::Context
            )))
        ));
        assert!(matches!(
            read.revalidate(&f.bytes, &f.trust, &f.host),
            Err(ContextFilesError::Refused)
        ));
    }
    assert!(p.is_refused());
}
#[test]
fn context_files_fixture_cannot_claim_fixed_root_owner_or_symlink_origin() {
    let f = Fixture::new();
    for (path, uid, gid) in [
        (Path::new("/etc"), owner().0, owner().1),
        (f.path(), 0, owner().1),
        (f.path(), owner().0, u32::MAX),
    ] {
        let mut p = f.checked();
        assert!(matches!(
            inspect_context_fixture_files(path, uid, gid, &mut p, &f.bytes, &f.trust, &f.host),
            Err(ContextFilesError::Files(WorkerFilesError::Security))
        ));
        assert!(p.is_refused());
    }
    let temp = tempfile::tempdir().unwrap();
    symlink(f.path(), temp.path().join("link")).unwrap();
    let mut p = f.checked();
    assert!(matches!(
        inspect_context_fixture_files(
            &temp.path().join("link"),
            owner().0,
            owner().1,
            &mut p,
            &f.bytes,
            &f.trust,
            &f.host
        ),
        Err(ContextFilesError::Files(_))
    ));
    assert!(p.is_refused());
}
#[test]
fn context_files_seeded_resigned_foreign_context_cannot_enroll_owned_files() {
    let seed = 0x7552026_u64;
    eprintln!("context files mutation seed={seed:#x}; mutations=64");
    let mut rng = seed;
    for _ in 0..64 {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        let mut f = Fixture::new();
        if rng & 1 == 0 {
            f.policy.user_namespace.inode += 1 + (rng % 1000);
        } else {
            f.policy.mount_namespace.inode += 1 + (rng % 1000);
        }
        f.sign();
        let mut p = f.checked();
        assert!(matches!(
            inspect_context_fixture_files(
                f.path(),
                owner().0,
                owner().1,
                &mut p,
                &f.bytes,
                &f.trust,
                &f.host
            ),
            Err(ContextFilesError::Context(WorkerContextError::Drift))
        ));
        assert!(p.is_refused());
    }
}
#[test]
fn context_files_independent_concurrent_guards_do_not_share_refusal() {
    std::thread::scope(|scope| {
        for index in 0..4 {
            scope.spawn(move || {
                let f = Fixture::new();
                let mut p = f.checked();
                let mut read = inspect_context_fixture_files(
                    f.path(),
                    owner().0,
                    owner().1,
                    &mut p,
                    &f.bytes,
                    &f.trust,
                    &f.host,
                )
                .unwrap();
                if index == 0 {
                    fs::remove_file(f.path().join("group")).unwrap();
                    assert!(read.revalidate(&f.bytes, &f.trust, &f.host).is_err());
                } else {
                    for _ in 0..4 {
                        read.revalidate(&f.bytes, &f.trust, &f.host).unwrap();
                    }
                }
                assert_eq!(read.is_refused(), index == 0);
            });
        }
    });
}
