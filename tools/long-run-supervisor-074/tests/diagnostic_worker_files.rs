//! Owned temporary files and synthetic trust/context; not production enrollment.
#![cfg(target_os = "linux")]
use ed25519_dalek::{Signer, SigningKey};
use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_files::{
    inspect_fixed_worker_files, inspect_fixture_worker_files, WorkerFilesError,
};
use hydracache_long_run_supervisor_074::diagnostic_worker_policy::*;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::Path;

const PASSWD: &[u8] = b"root:x:0:0:root:/root:/bin/sh\nhydracache-perf:x:986:986::/nonexistent:/usr/sbin/nologin\nother:x:989:989::/:/bin/false\n";
const GROUP: &[u8] =
    b"root:x:0:\nhydracache-perf:x:986:\nextra:x:987:hydracache-perf\nother:x:989:other\n";
fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn canonical(value: &impl Serialize) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(&serde_json::to_value(value).unwrap()).unwrap();
    bytes.push(b'\n');
    bytes
}
struct Fixture {
    directory: tempfile::TempDir,
    bytes: Vec<u8>,
    trust: WorkerPolicyTrust,
    host: AssertedWorkerHost,
}
impl Fixture {
    fn new(passwd: &[u8], group: &[u8], groups: Vec<u32>) -> Self {
        let directory = tempfile::tempdir().unwrap();
        fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700)).unwrap();
        for (name, bytes) in [("passwd", passwd), ("group", group)] {
            fs::write(directory.path().join(name), bytes).unwrap();
            fs::set_permissions(
                directory.path().join(name),
                fs::Permissions::from_mode(0o600),
            )
            .unwrap();
        }
        let host = AssertedWorkerHost {
            machine_id: "a".repeat(32),
            boot_id: "11111111-2222-3333-4444-555555555555".into(),
            user_namespace: NamespaceIdentity {
                device: 4,
                inode: 111,
            },
            mount_namespace: NamespaceIdentity {
                device: 4,
                inode: 222,
            },
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
            supplementary_gids: groups,
            machine_id: host.machine_id.clone(),
            boot_id: host.boot_id.clone(),
            user_namespace: host.user_namespace,
            mount_namespace: host.mount_namespace,
            passwd_sha256: hash(passwd),
            group_sha256: hash(group),
        };
        let key = SigningKey::from_bytes(&[74; 32]);
        let trust =
            WorkerPolicyTrust::new(key.verifying_key(), &hash(&canonical(&policy)), 7).unwrap();
        let mut message = WORKER_POLICY_DOMAIN.to_vec();
        message.push(0);
        message.extend(canonical(&policy));
        let bytes = canonical(&SignedWorkerPolicy {
            schema_version: SIGNED_WORKER_POLICY_SCHEMA.into(),
            policy,
            signature_hex: key
                .sign(&message)
                .to_bytes()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
        });
        Self {
            directory,
            bytes,
            trust,
            host,
        }
    }
    fn standard() -> Self {
        Self::new(PASSWD, GROUP, vec![987])
    }
    fn checked(&self) -> CheckedWorkerPolicy {
        verify_worker_policy(&self.bytes, &self.trust, &self.host).unwrap()
    }
    fn path(&self) -> &Path {
        self.directory.path()
    }
    fn rejected(&self) -> WorkerFilesError {
        let mut policy = self.checked();
        let error = inspect_fixture_worker_files(
            self.path(),
            owner().0,
            owner().1,
            &mut policy,
            &self.bytes,
            &self.trust,
            &self.host,
        )
        .err()
        .unwrap();
        assert!(policy.is_refused());
        error
    }
}
fn owner() -> (u32, u32) {
    // SAFETY: these calls take no arguments and return the reading process IDs.
    let ids = unsafe { (libc::geteuid(), libc::getegid()) };
    assert!(
        ids.0 > 0 && ids.1 > 0,
        "fixtures require an unprivileged test user"
    );
    ids
}

#[test]
fn worker_files_fixture_roundtrip_keeps_policy_private_and_healthy() {
    let f = Fixture::standard();
    let mut policy = f.checked();
    let original = policy.policy_sha256().to_owned();
    let mut reader = inspect_fixture_worker_files(
        f.path(),
        owner().0,
        owner().1,
        &mut policy,
        &f.bytes,
        &f.trust,
        &f.host,
    )
    .unwrap();
    for _ in 0..3 {
        reader.revalidate(&f.bytes, &f.trust, &f.host).unwrap();
    }
    assert!(!reader.is_refused());
    drop(reader);
    assert!(!policy.is_refused());
    assert_eq!(policy.policy_sha256(), original);
}

#[test]
fn worker_files_refused_or_revoked_policy_precedes_any_path_access() {
    let f = Fixture::standard();
    let mut policy = f.checked();
    let revoked = WorkerPolicyTrust::new(
        SigningKey::from_bytes(&[75; 32]).verifying_key(),
        policy.policy_sha256(),
        7,
    )
    .unwrap();
    let error = inspect_fixture_worker_files(
        Path::new("/does-not-exist-worker-files"),
        owner().0,
        owner().1,
        &mut policy,
        &f.bytes,
        &revoked,
        &f.host,
    )
    .err()
    .unwrap();
    assert!(matches!(
        error,
        WorkerFilesError::Policy(WorkerPolicyError::Revoked)
    ));
    assert!(matches!(
        inspect_fixed_worker_files(&mut policy, &f.bytes, &f.trust, &f.host)
            .err()
            .unwrap(),
        WorkerFilesError::Refused
    ));
    assert!(policy.is_refused());
}

#[test]
fn worker_files_fixture_rejects_production_paths_and_root_assertions() {
    let f = Fixture::standard();
    for (path, uid, gid) in [
        (Path::new("/etc"), owner().0, owner().1),
        (Path::new("/etc/child"), owner().0, owner().1),
        (Path::new("relative"), owner().0, owner().1),
        (f.path(), 0, owner().1),
        (f.path(), owner().0, 0),
    ] {
        let mut policy = f.checked();
        assert!(inspect_fixture_worker_files(
            path,
            uid,
            gid,
            &mut policy,
            &f.bytes,
            &f.trust,
            &f.host
        )
        .is_err());
        assert!(policy.is_refused());
    }
}

#[test]
fn worker_files_resigned_strict_grammar_and_ambiguity_refuse() {
    let passwd = String::from_utf8(PASSWD.to_vec()).unwrap();
    let group = String::from_utf8(GROUP.to_vec()).unwrap();
    let bad_passwd = vec![
        passwd.trim_end().into(),
        passwd.replace('\n', "\r\n"),
        format!("{passwd}\n"),
        format!("#comment\n{passwd}"),
        passwd.replace("986:986", "0986:986"),
        passwd.replace("986:986", "+986:986"),
        passwd.replace("986:986", "4294967296:986"),
        passwd.replace("986:986", "4294967295:986"),
        passwd.replace("986:986", "986:-986"),
        passwd.replace("986:986", "986:"),
        passwd.replace("root:x", "root:\0"),
        passwd.replace("root:x", "root:é"),
        passwd.replace("986:986", "986"),
        format!("{passwd}hydracache-perf:x:990:990::/:/bin/false\n"),
        format!("{passwd}alias:x:986:990::/:/bin/false\n"),
        format!("{passwd}+compat:x:991:991::/:/bin/false\n"),
        format!("{passwd}{}:x:991:991::/:/bin/false\n", "a".repeat(257)),
    ];
    for bytes in bad_passwd {
        assert!(matches!(
            Fixture::new(bytes.as_bytes(), GROUP, vec![987]).rejected(),
            WorkerFilesError::Document
        ));
    }
    for bytes in [
        group.replace("987", "0987"),
        group.replace("extra:x:987:", "extra:x:987::"),
        group.replace("987:hydracache-perf", "987:hydracache-perf,"),
        group.replace("987:hydracache-perf", "987:hydracache-perf,hydracache-perf"),
        group.replace("989:other", "989:unknown"),
        format!("{group}alias:x:987:\n"),
        format!("{group}extra:x:990:\n"),
    ] {
        assert!(matches!(
            Fixture::new(PASSWD, bytes.as_bytes(), vec![987]).rejected(),
            WorkerFilesError::Document
        ));
    }
}

#[test]
fn worker_files_mapping_and_exact_supplementary_groups_refuse_inference() {
    for (passwd, group, groups) in [
        (
            String::from_utf8(PASSWD.to_vec())
                .unwrap()
                .replace("986:986", "985:986"),
            String::from_utf8(GROUP.to_vec()).unwrap(),
            vec![987],
        ),
        (
            String::from_utf8(PASSWD.to_vec()).unwrap(),
            String::from_utf8(GROUP.to_vec())
                .unwrap()
                .replace("hydracache-perf:x:986:", "hydracache-perf:x:985:"),
            vec![987],
        ),
        (
            String::from_utf8(PASSWD.to_vec()).unwrap(),
            String::from_utf8(GROUP.to_vec()).unwrap(),
            vec![],
        ),
        (
            String::from_utf8(PASSWD.to_vec()).unwrap(),
            String::from_utf8(GROUP.to_vec()).unwrap(),
            vec![986, 987],
        ),
    ] {
        assert!(matches!(
            Fixture::new(passwd.as_bytes(), group.as_bytes(), groups).rejected(),
            WorkerFilesError::Mapping
        ));
    }
    for (group, groups) in [
        (
            String::from_utf8(GROUP.to_vec()).unwrap().replace(
                "hydracache-perf:x:986:",
                "hydracache-perf:x:986:hydracache-perf",
            ),
            vec![986, 987],
        ),
        (
            String::from_utf8(GROUP.to_vec())
                .unwrap()
                .replace("extra:x:987:hydracache-perf", "extra:x:987:"),
            vec![],
        ),
    ] {
        let f = Fixture::new(PASSWD, group.as_bytes(), groups);
        let mut policy = f.checked();
        assert!(inspect_fixture_worker_files(
            f.path(),
            owner().0,
            owner().1,
            &mut policy,
            &f.bytes,
            &f.trust,
            &f.host
        )
        .is_ok());
        assert!(!policy.is_refused());
    }
}

#[test]
fn worker_files_whole_documents_and_budgets_are_not_worker_only() {
    let f = Fixture::standard();
    fs::write(
        f.path().join("group"),
        String::from_utf8(GROUP.to_vec())
            .unwrap()
            .replace("other:x", "other:!"),
    )
    .unwrap();
    assert!(matches!(f.rejected(), WorkerFilesError::Drift));
    for bytes in [
        vec![],
        vec![b'x'; 1_048_577],
        [b"x".repeat(4097), b"\n".to_vec()].concat(),
    ] {
        assert!(matches!(
            Fixture::new(&bytes, GROUP, vec![987]).rejected(),
            WorkerFilesError::Document | WorkerFilesError::Security
        ));
    }
    let mut bytes = PASSWD.to_vec();
    for i in 0..16_384 {
        bytes.extend(format!("u{i}:x:{}:989::/:/bin/false\n", 10_000 + i).as_bytes());
    }
    assert!(matches!(
        Fixture::new(&bytes, GROUP, vec![987]).rejected(),
        WorkerFilesError::Document
    ));
}

#[test]
fn worker_files_symlink_hardlink_fifo_directory_and_socket_refuse() {
    for kind in 0..5 {
        let f = Fixture::standard();
        let path = f.path().join("group");
        fs::remove_file(&path).unwrap();
        let mut socket = None;
        match kind {
            0 => symlink("passwd", &path).unwrap(),
            1 => fs::hard_link(f.path().join("passwd"), &path).unwrap(),
            2 => {
                use std::os::unix::ffi::OsStrExt;
                let path = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
                // SAFETY: owned temporary path, live NUL-terminated bytes, no data pointers.
                assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
            }
            3 => fs::create_dir(&path).unwrap(),
            _ => socket = Some(std::os::unix::net::UnixListener::bind(&path).unwrap()),
        }
        assert!(f.rejected().to_string().len() < 256);
        drop(socket);
    }
}

#[test]
fn worker_files_modes_ownership_and_symlink_ancestors_refuse() {
    for (name, mode) in [
        ("group", 0o620),
        ("passwd", 0o602),
        ("group", 0o700),
        ("group", 0o4600),
        ("", 0o722),
    ] {
        let f = Fixture::standard();
        fs::set_permissions(f.path().join(name), fs::Permissions::from_mode(mode)).unwrap();
        assert!(matches!(f.rejected(), WorkerFilesError::Security));
    }
    let f = Fixture::standard();
    let mut policy = f.checked();
    assert!(inspect_fixture_worker_files(
        f.path(),
        owner().0 + 1,
        owner().1,
        &mut policy,
        &f.bytes,
        &f.trust,
        &f.host
    )
    .is_err());
    assert!(policy.is_refused());
    let parent = tempfile::tempdir().unwrap();
    symlink(f.path(), parent.path().join("alias")).unwrap();
    let mut policy = f.checked();
    assert!(inspect_fixture_worker_files(
        &parent.path().join("alias"),
        owner().0,
        owner().1,
        &mut policy,
        &f.bytes,
        &f.trust,
        &f.host
    )
    .is_err());
}

#[test]
fn worker_files_named_leaf_and_directory_replacement_refuse_after_restore() {
    for directory in [false, true] {
        let f = Fixture::standard();
        let mut policy = f.checked();
        let mut read = inspect_fixture_worker_files(
            f.path(),
            owner().0,
            owner().1,
            &mut policy,
            &f.bytes,
            &f.trust,
            &f.host,
        )
        .unwrap();
        let parent = tempfile::tempdir().unwrap();
        let old = parent.path().join("old");
        if directory {
            fs::rename(f.path(), &old).unwrap();
            fs::create_dir(f.path()).unwrap();
            fs::set_permissions(f.path(), fs::Permissions::from_mode(0o700)).unwrap();
        } else {
            fs::rename(f.path().join("group"), &old).unwrap();
            fs::write(f.path().join("group"), GROUP).unwrap();
            fs::set_permissions(f.path().join("group"), fs::Permissions::from_mode(0o600)).unwrap();
        }
        assert!(read.revalidate(&f.bytes, &f.trust, &f.host).is_err());
        if directory {
            fs::remove_dir(f.path()).unwrap();
            fs::rename(&old, f.path()).unwrap();
        } else {
            fs::remove_file(f.path().join("group")).unwrap();
            fs::rename(&old, f.path().join("group")).unwrap();
        }
        assert!(matches!(
            read.revalidate(&f.bytes, &f.trust, &f.host),
            Err(WorkerFilesError::Refused)
        ));
        drop(read);
        assert!(policy.is_refused());
    }
}

#[test]
fn worker_files_inplace_rewrite_growth_and_trust_failures_latch_original() {
    for kind in 0..3 {
        let f = Fixture::standard();
        let mut policy = f.checked();
        let mut read = inspect_fixture_worker_files(
            f.path(),
            owner().0,
            owner().1,
            &mut policy,
            &f.bytes,
            &f.trust,
            &f.host,
        )
        .unwrap();
        match kind {
            0 => {
                fs::write(
                    f.path().join("group"),
                    String::from_utf8(GROUP.to_vec())
                        .unwrap()
                        .replace("extra:x", "extra:!"),
                )
                .unwrap();
            }
            1 => {
                fs::write(f.path().join("group"), vec![b'x'; 1_048_577]).unwrap();
            }
            _ => {}
        }
        let mut host = f.host.clone();
        if kind == 2 {
            host.mount_namespace.inode += 1;
        }
        assert!(read.revalidate(&f.bytes, &f.trust, &host).is_err());
        fs::write(f.path().join("group"), GROUP).unwrap();
        assert!(matches!(
            read.revalidate(&f.bytes, &f.trust, &f.host),
            Err(WorkerFilesError::Refused)
        ));
        drop(read);
        assert!(policy.is_refused());
    }
}

#[test]
fn worker_files_seeded_valid_unrelated_changes_keep_original_digest_pin() {
    let mut seed = 0x7522026u64;
    eprintln!("worker-files seed=0x7522026; mutations=64");
    for _ in 0..64 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let passwd = String::from_utf8(PASSWD.to_vec())
            .unwrap()
            .replace("root:/root", &format!("r{seed}:/root"));
        let f = Fixture::new(passwd.as_bytes(), GROUP, vec![987]);
        let mut own = f.checked();
        assert!(inspect_fixture_worker_files(
            f.path(),
            owner().0,
            owner().1,
            &mut own,
            &f.bytes,
            &f.trust,
            &f.host
        )
        .is_ok());
        let original = Fixture::standard();
        let mut old = original.checked();
        assert!(matches!(
            inspect_fixture_worker_files(
                f.path(),
                owner().0,
                owner().1,
                &mut old,
                &original.bytes,
                &original.trust,
                &original.host
            )
            .err()
            .unwrap(),
            WorkerFilesError::Drift
        ));
        assert!(old.is_refused());
    }
}

#[test]
fn worker_files_independent_concurrent_readers_share_no_refusal() {
    let threads: Vec<_> = (0..4)
        .map(|i| {
            std::thread::spawn(move || {
                let f = Fixture::standard();
                let mut policy = f.checked();
                let mut read = inspect_fixture_worker_files(
                    f.path(),
                    owner().0,
                    owner().1,
                    &mut policy,
                    &f.bytes,
                    &f.trust,
                    &f.host,
                )
                .unwrap();
                if i == 0 {
                    fs::remove_file(f.path().join("passwd")).unwrap();
                }
                assert_eq!(
                    read.revalidate(&f.bytes, &f.trust, &f.host).is_err(),
                    i == 0
                );
                drop(read);
                assert_eq!(policy.is_refused(), i == 0);
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
}

#[test]
fn worker_files_exact_byte_line_record_and_group_bounds_accept() {
    let mut passwd = PASSWD.to_vec();
    let mut i = 0;
    while passwd.len() < 1_048_576 {
        let remaining = 1_048_576 - passwd.len();
        let size = if remaining > 4097 && remaining - 4097 < 64 {
            4033
        } else {
            remaining.min(4097)
        };
        let prefix = format!("large{i}:x:{}:989:", 20_000 + i);
        let suffix = ":/:/bin/false\n";
        let padding = size.checked_sub(prefix.len() + suffix.len()).unwrap();
        passwd.extend(prefix.as_bytes());
        passwd.extend(vec![b'g'; padding]);
        passwd.extend(suffix.as_bytes());
        i += 1;
    }
    assert_eq!(passwd.len(), 1_048_576);
    let mut records = PASSWD.to_vec();
    for i in 0..16_381 {
        records.extend(format!("u{i}:x:{}:989::/:/bin/false\n", 10_000 + i).as_bytes());
    }
    let mut group = String::from_utf8(GROUP.to_vec()).unwrap().replace(
        "hydracache-perf:x:986:",
        "hydracache-perf:x:986:hydracache-perf",
    );
    let mut groups = vec![986, 987];
    for i in 0..30 {
        group.push_str(&format!("g{i}:x:{}:hydracache-perf\n", 990 + i));
        groups.push(990 + i);
    }
    for passwd in [&passwd, &records] {
        let f = Fixture::new(passwd, group.as_bytes(), groups.clone());
        let mut policy = f.checked();
        let mut read = inspect_fixture_worker_files(
            f.path(),
            owner().0,
            owner().1,
            &mut policy,
            &f.bytes,
            &f.trust,
            &f.host,
        )
        .unwrap();
        read.revalidate(&f.bytes, &f.trust, &f.host).unwrap();
        assert!(!read.is_refused());
    }
}
