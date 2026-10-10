//! Actual owned NNP helpers and synthetic account authority; never production enrollment.
use super::super::inspect_context_fixture_files;
use super::*;
use crate::diagnostic_process::{
    pin_namespace_checked_credentials, pin_owned_test_helper, AssertedWorkerCredentials,
    NamespaceError,
};
use crate::diagnostic_worker_policy::{
    verify_worker_policy, CheckedWorkerPolicy, NamespaceIdentity, SignedWorkerPolicy, WorkerPolicy,
    SIGNED_WORKER_POLICY_SCHEMA, WORKER_POLICY_DOMAIN, WORKER_POLICY_SCHEMA,
};
use ed25519_dalek::{Signer, SigningKey};
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};

pub(super) fn ids() -> (u32, u32, Vec<u32>) {
    // SAFETY: scalar queries and a bounded group buffer sized by getgroups.
    let (uid, gid, count) = unsafe {
        (
            libc::geteuid(),
            libc::getegid(),
            libc::getgroups(0, std::ptr::null_mut()),
        )
    };
    assert!(uid > 0 && gid > 0 && (0..=31).contains(&count));
    let mut groups = vec![0; count as usize];
    assert_eq!(
        unsafe { libc::getgroups(count, groups.as_mut_ptr()) },
        count
    );
    groups.sort_unstable();
    assert!(!groups.contains(&0));
    (uid, gid, groups)
}
fn hash(b: &[u8]) -> String {
    crate::sha256_hex(b)
}
fn canonical(value: &impl serde::Serialize) -> Vec<u8> {
    let mut b = crate::canonical_json(value).unwrap();
    b.push(b'\n');
    b
}
pub(super) struct Fixture {
    pub(super) dir: tempfile::TempDir,
    pub(super) policy: WorkerPolicy,
    pub(super) group: Vec<u8>,
    pub(super) bytes: Vec<u8>,
    pub(super) trust: WorkerPolicyTrust,
    pub(super) host: AssertedWorkerHost,
}
impl Fixture {
    pub(super) fn new() -> Self {
        let (uid, gid, groups) = ids();
        Self::with_ids(uid, gid, groups)
    }
    pub(super) fn with_ids(uid: u32, gid: u32, groups: Vec<u32>) -> Self {
        let dir = tempfile::tempdir().unwrap();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let passwd = format!(
            "root:x:0:0::/root:/bin/sh\nhydracache-perf:x:{uid}:{gid}::/nonexistent:/bin/false\n"
        )
        .into_bytes();
        let member = if groups.contains(&gid) {
            "hydracache-perf"
        } else {
            ""
        };
        let mut group = format!("root:x:0:\nhydracache-perf:x:{gid}:{member}\n");
        for g in &groups {
            if *g != gid {
                group.push_str(&format!("g{g}:x:{g}:hydracache-perf\n"));
            }
        }
        let group = group.into_bytes();
        for (name, b) in [("passwd", &passwd), ("group", &group)] {
            fs::write(dir.path().join(name), b).unwrap();
            fs::set_permissions(dir.path().join(name), fs::Permissions::from_mode(0o600)).unwrap();
        }
        let ns = |name| {
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
            uid,
            gid,
            supplementary_gids: groups,
            machine_id: host.machine_id.clone(),
            boot_id: host.boot_id.clone(),
            user_namespace: host.user_namespace,
            mount_namespace: host.mount_namespace,
            passwd_sha256: hash(&passwd),
            group_sha256: hash(&group),
        };
        let key = SigningKey::from_bytes(&[74; 32]);
        let body = canonical(&policy);
        let trust = WorkerPolicyTrust::new(key.verifying_key(), &hash(&body), 7).unwrap();
        let mut message = WORKER_POLICY_DOMAIN.to_vec();
        message.push(0);
        message.extend(body);
        let bytes = canonical(&SignedWorkerPolicy {
            schema_version: SIGNED_WORKER_POLICY_SCHEMA.into(),
            policy: policy.clone(),
            signature_hex: key
                .sign(&message)
                .to_bytes()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
        });
        Self {
            dir,
            policy,
            group,
            bytes,
            trust,
            host,
        }
    }
    pub(super) fn checked(&self) -> CheckedWorkerPolicy {
        verify_worker_policy(&self.bytes, &self.trust, &self.host).unwrap()
    }
    pub(super) fn account<'p>(
        &self,
        p: &'p mut CheckedWorkerPolicy,
    ) -> super::ContextFixtureFilesRead<'p> {
        let (uid, gid, _) = ids();
        inspect_context_fixture_files(
            self.dir.path(),
            uid,
            gid,
            p,
            &self.bytes,
            &self.trust,
            &self.host,
        )
        .unwrap()
    }
}
pub(super) struct Owned(pub(super) Child);
impl Owned {
    pub(super) fn new() -> Self {
        let mut command = Command::new("/bin/cat");
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        // SAFETY: async-signal-safe prctl only before exec.
        unsafe {
            command.pre_exec(|| {
                if libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0 {
                    Err(std::io::Error::last_os_error())
                } else {
                    Ok(())
                }
            });
        }
        Self(command.spawn().unwrap())
    }
    pub(super) fn finish(&mut self) {
        self.0.stdin.take();
        self.0.wait().unwrap();
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        self.finish();
    }
}
fn credentials(process: &crate::diagnostic_process::ProcessRead) -> NamespaceCredentialRead<'_> {
    let (uid, gid, groups) = ids();
    pin_namespace_checked_credentials(
        process,
        AssertedWorkerCredentials::new(uid, gid, groups).unwrap(),
    )
    .unwrap()
}
fn assert_refused(
    account: &super::ContextFixtureFilesRead<'_>,
    credentials: &mut NamespaceCredentialRead<'_>,
) {
    assert!(account.is_refused());
    assert!(credentials.is_refused());
    assert!(matches!(
        credentials.revalidate(),
        Err(NamespaceCredentialError::Refused)
    ));
}

#[test]
fn policy_credentials_order_preserves_both_original_brackets() {
    let mut gate = Gate::default();
    let mut order = vec![];
    gate.observe(false, true, |step| {
        order.push(step);
        Ok(())
    })
    .unwrap();
    assert_eq!(order, [Step::Account, Step::Credentials, Step::Account]);
}
#[test]
fn policy_credentials_prior_refusal_and_mismatch_never_observe() {
    for (refused, matches) in [(true, true), (true, false), (false, false)] {
        let mut gate = Gate::default();
        let error = gate
            .observe(refused, matches, |_| panic!("no IO"))
            .unwrap_err();
        if refused {
            assert!(matches!(error, PolicyCredentialError::Refused));
        } else {
            assert!(matches!(error, PolicyCredentialError::Mapping));
        }
        assert!(matches!(
            gate.observe(false, true, |_| panic!("no refresh")),
            Err(PolicyCredentialError::Refused)
        ));
    }
}
#[test]
fn policy_credentials_every_error_stops_with_original_type() {
    for failed in 0..3 {
        let mut gate = Gate::default();
        let mut calls = 0;
        let error = gate
            .observe(false, true, |step| {
                let here = calls;
                calls += 1;
                if here != failed {
                    return Ok(());
                }
                match step {
                    Step::Account => Err(ContextFilesError::Files(
                        crate::diagnostic_worker_policy::local_files::WorkerFilesError::Io(
                            std::io::Error::from_raw_os_error(libc::EACCES),
                        ),
                    )
                    .into()),
                    Step::Credentials => {
                        Err(NamespaceCredentialError::Namespace(NamespaceError::Drift).into())
                    }
                }
            })
            .unwrap_err();
        assert_eq!(calls, failed + 1);
        if failed == 1 {
            assert!(matches!(
                error,
                PolicyCredentialError::Credentials(NamespaceCredentialError::Namespace(
                    NamespaceError::Drift
                ))
            ));
        } else {
            assert!(
                matches!(error,PolicyCredentialError::Account(ContextFilesError::Files(crate::diagnostic_worker_policy::local_files::WorkerFilesError::Io(e))) if e.raw_os_error()==Some(libc::EACCES))
            );
        }
        assert!(matches!(
            gate.observe(false, true, |_| panic!("no refresh")),
            Err(PolicyCredentialError::Refused)
        ));
    }
}
#[test]
fn policy_credentials_exact_mapping_never_forms_primary_union() {
    let child = Owned::new();
    let process = pin_owned_test_helper(child.0.id());
    let mut read = credentials(&process);
    let f = Fixture::new();
    assert!(read.matches_worker_policy(&f.policy));
    for field in 0..3 {
        let mut p = f.policy.clone();
        match field {
            0 => p.uid += 1,
            1 => p.gid += 1,
            _ => {
                if let Ok(i) = p.supplementary_gids.binary_search(&p.gid) {
                    p.supplementary_gids.remove(i);
                } else {
                    p.supplementary_gids.push(p.gid);
                    p.supplementary_gids.sort_unstable();
                }
            }
        }
        assert!(!read.matches_worker_policy(&p));
    }
    read.refuse();
    assert!(!read.matches_worker_policy(&f.policy));
}
#[test]
fn policy_credentials_seeded_mapping_mutations_cannot_refresh() {
    let seed = 0x7562026_u64;
    eprintln!("policy credentials mutation seed={seed:#x}; mutations=256");
    let child = Owned::new();
    let process = pin_owned_test_helper(child.0.id());
    let read = credentials(&process);
    let f = Fixture::new();
    let mut rng = seed;
    for _ in 0..256 {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        let mut p = f.policy.clone();
        if rng & 1 == 0 {
            p.uid += 1 + (rng % 1000) as u32;
        } else {
            p.gid += 1 + (rng % 1000) as u32;
        }
        let mut gate = Gate::default();
        assert!(matches!(
            gate.observe(false, read.matches_worker_policy(&p), |_| panic!(
                "mismatch no IO"
            )),
            Err(PolicyCredentialError::Mapping)
        ));
        assert!(matches!(
            gate.observe(false, read.matches_worker_policy(&f.policy), |_| panic!(
                "restoration no IO"
            )),
            Err(PolicyCredentialError::Refused)
        ));
    }
}
#[test]
fn policy_credentials_binding_types_are_neither_send_nor_sync() {
    trait AmbiguousSend<A> {
        fn marker() {}
    }
    impl<T: ?Sized> AmbiguousSend<()> for T {}
    impl<T: ?Sized + Send> AmbiguousSend<u8> for T {}
    let _ = <FixedPolicyCredentialRead<'static, 'static, 'static> as AmbiguousSend<_>>::marker;
    let _ = <FixturePolicyCredentialRead<'static, 'static, 'static> as AmbiguousSend<_>>::marker;
    trait AmbiguousSync<A> {
        fn marker() {}
    }
    impl<T: ?Sized> AmbiguousSync<()> for T {}
    impl<T: ?Sized + Sync> AmbiguousSync<u8> for T {}
    let _ = <FixedPolicyCredentialRead<'static, 'static, 'static> as AmbiguousSync<_>>::marker;
    let _ = <FixturePolicyCredentialRead<'static, 'static, 'static> as AmbiguousSync<_>>::marker;
}
#[test]
fn policy_credentials_fixed_prior_refusal_never_reads_production_accounts() {
    let child = Owned::new();
    let process = pin_owned_test_helper(child.0.id());
    let mut c = credentials(&process);
    let f = Fixture::new();
    let mut p = f.checked();
    {
        let a = f.account(&mut p);
        a.reader.context.policy.refused = true;
        // Private test-only invalid fixed wrapper: negative path only, no /etc inspection.
        // Public APIs cannot convert these origins.
        let mut fixed = ContextFixedFilesRead { reader: a.reader };
        assert!(matches!(
            bind_context_fixed_credentials(&mut fixed, &mut c, &f.bytes, &f.trust, &f.host),
            Err(PolicyCredentialError::Refused)
        ));
        assert!(fixed.is_refused() && c.is_refused());
    }
    assert!(p.is_refused());
}

#[test]
fn policy_credentials_owned_roundtrip_and_drop_keep_inputs_healthy() {
    let child = Owned::new();
    let process = pin_owned_test_helper(child.0.id());
    let mut c = credentials(&process);
    let f = Fixture::new();
    let mut p = f.checked();
    {
        let mut a = f.account(&mut p);
        {
            let mut b =
                bind_context_fixture_credentials(&mut a, &mut c, &f.bytes, &f.trust, &f.host)
                    .unwrap();
            for _ in 0..4 {
                b.revalidate(&f.bytes, &f.trust, &f.host).unwrap();
            }
            assert!(!b.is_refused());
        }
        a.revalidate(&f.bytes, &f.trust, &f.host).unwrap();
        c.revalidate().unwrap();
    }
    p.revalidate(&f.bytes, &f.trust, &f.host).unwrap();
}
#[test]
fn policy_credentials_public_mismatch_precedes_broken_account_io() {
    let child = Owned::new();
    let process = pin_owned_test_helper(child.0.id());
    let mut c = credentials(&process);
    let f = Fixture::new();
    let mut p = f.checked();
    {
        let mut a = f.account(&mut p);
        // Private corruption of retained assertions, not a public refresh mechanism.
        a.reader.context.policy.original.uid += 1;
        fs::remove_file(f.dir.path().join("group")).unwrap();
        assert!(matches!(
            bind_context_fixture_credentials(&mut a, &mut c, &f.bytes, &f.trust, &f.host),
            Err(PolicyCredentialError::Mapping)
        ));
        assert_refused(&a, &mut c);
    }
    assert!(p.is_refused());
}
#[test]
fn policy_credentials_public_prior_refusal_latches_other_input() {
    let child = Owned::new();
    let process = pin_owned_test_helper(child.0.id());
    for account in [false, true] {
        let mut c = credentials(&process);
        let f = Fixture::new();
        let mut p = f.checked();
        {
            let mut a = f.account(&mut p);
            if account {
                a.reader.context.policy.refused = true;
            } else {
                c.refuse();
            }
            fs::remove_file(f.dir.path().join("group")).unwrap();
            assert!(matches!(
                bind_context_fixture_credentials(&mut a, &mut c, &f.bytes, &f.trust, &f.host),
                Err(PolicyCredentialError::Refused)
            ));
            assert_refused(&a, &mut c);
        }
        assert!(p.is_refused());
    }
}
#[test]
fn policy_credentials_constructor_file_failure_latches_after_drop() {
    let child = Owned::new();
    let process = pin_owned_test_helper(child.0.id());
    let mut c = credentials(&process);
    let f = Fixture::new();
    let mut p = f.checked();
    {
        let mut a = f.account(&mut p);
        fs::write(f.dir.path().join("group"), b"changed\n").unwrap();
        assert!(matches!(
            bind_context_fixture_credentials(&mut a, &mut c, &f.bytes, &f.trust, &f.host),
            Err(PolicyCredentialError::Account(_))
        ));
        fs::write(f.dir.path().join("group"), &f.group).unwrap();
        assert_refused(&a, &mut c);
    }
    assert!(p.is_refused());
}
#[test]
fn policy_credentials_file_restoration_cannot_clear_binding_refusal() {
    let child = Owned::new();
    let process = pin_owned_test_helper(child.0.id());
    let mut c = credentials(&process);
    let f = Fixture::new();
    let mut p = f.checked();
    {
        let mut a = f.account(&mut p);
        {
            let mut b =
                bind_context_fixture_credentials(&mut a, &mut c, &f.bytes, &f.trust, &f.host)
                    .unwrap();
            fs::rename(f.dir.path().join("group"), f.dir.path().join("original")).unwrap();
            fs::write(f.dir.path().join("group"), &f.group).unwrap();
            fs::set_permissions(
                f.dir.path().join("group"),
                fs::Permissions::from_mode(0o600),
            )
            .unwrap();
            assert!(matches!(
                b.revalidate(&f.bytes, &f.trust, &f.host),
                Err(PolicyCredentialError::Account(_))
            ));
            fs::remove_file(f.dir.path().join("group")).unwrap();
            fs::rename(f.dir.path().join("original"), f.dir.path().join("group")).unwrap();
            assert!(matches!(
                b.revalidate(&f.bytes, &f.trust, &f.host),
                Err(PolicyCredentialError::Refused)
            ));
        }
        assert_refused(&a, &mut c);
    }
    assert!(p.is_refused());
}
#[test]
fn policy_credentials_revocation_precedes_account_and_process_errors() {
    let mut child = Owned::new();
    let process = pin_owned_test_helper(child.0.id());
    let mut c = credentials(&process);
    let f = Fixture::new();
    let mut p = f.checked();
    {
        let mut a = f.account(&mut p);
        let revoked = WorkerPolicyTrust::new(
            SigningKey::from_bytes(&[74; 32]).verifying_key(),
            &hash(&canonical(&f.policy)),
            8,
        )
        .unwrap();
        child.finish();
        fs::remove_file(f.dir.path().join("group")).unwrap();
        assert!(matches!(
            bind_context_fixture_credentials(&mut a, &mut c, &f.bytes, &revoked, &f.host),
            Err(PolicyCredentialError::Account(ContextFilesError::Context(
                super::super::WorkerContextError::Policy(
                    crate::diagnostic_worker_policy::WorkerPolicyError::Revoked
                )
            )))
        ));
        assert_refused(&a, &mut c);
    }
    assert!(p.is_refused());
}
#[test]
fn policy_credentials_original_exit_refuses_constructor_and_later_read() {
    for later in [false, true] {
        let mut child = Owned::new();
        let process = pin_owned_test_helper(child.0.id());
        let mut c = credentials(&process);
        let f = Fixture::new();
        let mut p = f.checked();
        {
            let mut a = f.account(&mut p);
            if later {
                let mut b =
                    bind_context_fixture_credentials(&mut a, &mut c, &f.bytes, &f.trust, &f.host)
                        .unwrap();
                child.finish();
                assert!(matches!(
                    b.revalidate(&f.bytes, &f.trust, &f.host),
                    Err(PolicyCredentialError::Credentials(
                        NamespaceCredentialError::Namespace(NamespaceError::Process(_))
                    ))
                ));
            } else {
                child.finish();
                assert!(matches!(
                    bind_context_fixture_credentials(&mut a, &mut c, &f.bytes, &f.trust, &f.host),
                    Err(PolicyCredentialError::Credentials(
                        NamespaceCredentialError::Namespace(NamespaceError::Process(_))
                    ))
                ));
            }
            assert_refused(&a, &mut c);
        }
        assert!(p.is_refused());
    }
}
#[test]
fn policy_credentials_independent_concurrent_bindings_share_no_refusal() {
    std::thread::scope(|scope| {
        for index in 0..4 {
            scope.spawn(move || {
                let child = Owned::new();
                let process = pin_owned_test_helper(child.0.id());
                let mut c = credentials(&process);
                let f = Fixture::new();
                let mut p = f.checked();
                {
                    let mut a = f.account(&mut p);
                    {
                        let mut b = bind_context_fixture_credentials(
                            &mut a, &mut c, &f.bytes, &f.trust, &f.host,
                        )
                        .unwrap();
                        if index == 0 {
                            fs::remove_file(f.dir.path().join("group")).unwrap();
                            assert!(b.revalidate(&f.bytes, &f.trust, &f.host).is_err());
                        } else {
                            for _ in 0..4 {
                                b.revalidate(&f.bytes, &f.trust, &f.host).unwrap();
                            }
                        }
                        assert_eq!(b.is_refused(), index == 0);
                    }
                    assert_eq!(a.is_refused(), index == 0);
                    assert_eq!(c.is_refused(), index == 0);
                }
                assert_eq!(p.is_refused(), index == 0);
            });
        }
    });
}
