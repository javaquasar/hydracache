//! Sequential namespace checks before status opening and around credential reads.

use super::{
    AssertedWorkerCredentials, CredentialError, Document, ProcessCredentialRead, ProcessRead,
};
use crate::diagnostic_process::{pin_same_user_namespace, NamespaceError, SameUserNamespaceRead};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NamespaceCredentialError {
    #[error(transparent)]
    Namespace(#[from] NamespaceError),
    #[error(transparent)]
    Credentials(#[from] CredentialError),
    #[error("namespace checked credential reader permanently refused")]
    Refused,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Namespace,
    Open,
    Credentials,
}
#[derive(Default)]
struct Gate {
    initialized: bool,
    refused: bool,
}
impl Gate {
    fn observe(
        &mut self,
        mut read: impl FnMut(Step) -> Result<(), NamespaceCredentialError>,
    ) -> Result<(), NamespaceCredentialError> {
        if self.refused {
            return Err(NamespaceCredentialError::Refused);
        }
        let steps: &[Step] = if self.initialized {
            &[Step::Namespace, Step::Credentials, Step::Namespace]
        } else {
            &[
                Step::Namespace,
                Step::Open,
                Step::Namespace,
                Step::Credentials,
                Step::Namespace,
            ]
        };
        for step in steps {
            if let Err(error) = read(*step) {
                self.refused = true;
                return Err(error);
            }
        }
        self.initialized = true;
        Ok(())
    }
}

/// Owns both original guards; sequential context checks, not host enrollment.
pub struct NamespaceCredentialRead<'a> {
    namespace: SameUserNamespaceRead<'a>,
    credentials: ProcessCredentialRead<'a>,
    gate: Gate,
}

/// Opens status only inside this call's original namespace bracket.
/// Already opened credential readers cannot be retroactively enrolled.
pub fn pin_namespace_checked_credentials(
    process: &ProcessRead,
    policy: AssertedWorkerCredentials,
) -> Result<NamespaceCredentialRead<'_>, NamespaceCredentialError> {
    let mut namespace = pin_same_user_namespace(process)?;
    let mut gate = Gate::default();
    let mut credentials: Option<ProcessCredentialRead<'_>> = None;
    let mut policy = Some(policy);
    let result = gate.observe(|step| match step {
        Step::Namespace => namespace.revalidate().map_err(Into::into),
        Step::Open => {
            process.revalidate().map_err(CredentialError::from)?;
            let document = Document::open(&process.probe.files.directory, "status")
                .map_err(CredentialError::from)?;
            credentials = Some(ProcessCredentialRead {
                process,
                document,
                gate: super::Gate::new(
                    process.observation.expected.pid,
                    policy.take().ok_or(CredentialError::Invalid)?,
                ),
            });
            Ok(())
        }
        Step::Credentials => credentials
            .as_mut()
            .ok_or(CredentialError::Invalid)?
            .revalidate()
            .map_err(Into::into),
    });
    if let Err(error) = result {
        namespace.refuse();
        if let Some(credentials) = &mut credentials {
            credentials.refuse();
        }
        return Err(error);
    }
    Ok(NamespaceCredentialRead {
        namespace,
        credentials: credentials.ok_or(CredentialError::Invalid)?,
        gate,
    })
}
impl NamespaceCredentialRead<'_> {
    pub(crate) fn matches_worker_policy(
        &self,
        policy: &crate::diagnostic_worker_policy::WorkerPolicy,
    ) -> bool {
        !self.is_refused()
            && !self.namespace.is_refused()
            && self.credentials.matches_worker_policy(policy)
    }
    pub(crate) fn refuse(&mut self) {
        self.gate.refused = true;
        self.namespace.refuse();
        self.credentials.refuse();
    }
    pub(crate) fn matches_account(
        &self,
        account: &crate::diagnostic_manager::WorkerAccountRead,
    ) -> bool {
        !self.is_refused()
            && !self.namespace.is_refused()
            && self.credentials.matches_account(account)
    }
    pub fn is_refused(&self) -> bool {
        self.gate.refused
    }
    /// No IO follows refusal; neither original guard is exported or refreshed.
    pub fn revalidate(&mut self) -> Result<(), NamespaceCredentialError> {
        let result = self.gate.observe(|step| match step {
            Step::Namespace => self.namespace.revalidate().map_err(Into::into),
            Step::Credentials => self.credentials.revalidate().map_err(Into::into),
            // A completed constructor never schedules an opener again.
            Step::Open => Err(CredentialError::Invalid.into()),
        });
        if result.is_err() {
            self.refuse();
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic_process::pin_owned_test_helper;
    use std::fs::File;
    use std::os::unix::process::CommandExt;
    use std::process::{Child, Command, Stdio};

    #[test]
    fn namespace_policy_matching_requires_original_projection_and_both_guards() {
        let child = Owned::new(true);
        let process = pin_owned_test_helper(child.0.id());
        let mut read = pin_namespace_checked_credentials(&process, policy()).unwrap();
        let original = &read.credentials.gate.policy;
        let signed = crate::diagnostic_worker_policy::WorkerPolicy {
            schema_version: crate::diagnostic_worker_policy::WORKER_POLICY_SCHEMA.into(),
            repository_id: 1_217_101_761,
            purpose: "diagnostic-worker-only".into(),
            account_source: "local-files".into(),
            account: "hydracache-perf".into(),
            group: "hydracache-perf".into(),
            policy_epoch: 7,
            uid: original.uid,
            gid: original.gid,
            supplementary_gids: original.groups.clone(),
            machine_id: "a".repeat(32),
            boot_id: "00000000-0000-0000-0000-000000000001".into(),
            user_namespace: crate::diagnostic_worker_policy::NamespaceIdentity {
                device: 1,
                inode: 1,
            },
            mount_namespace: crate::diagnostic_worker_policy::NamespaceIdentity {
                device: 1,
                inode: 2,
            },
            passwd_sha256: "b".repeat(64),
            group_sha256: "c".repeat(64),
        };
        // This private boolean test is numeric consistency, not byte verification/enrollment.
        assert!(read.matches_worker_policy(&signed));
        let projection = read.credentials.gate.original.take();
        assert!(!read.matches_worker_policy(&signed));
        read.credentials.gate.original = projection;
        assert!(read.matches_worker_policy(&signed));
        read.namespace.refuse();
        assert!(!read.matches_worker_policy(&signed));
        read.refuse();
        assert!(read.namespace.is_refused() && read.credentials.is_refused());
        refused(&mut read);
        let mut read = pin_namespace_checked_credentials(&process, policy()).unwrap();
        read.credentials.refuse();
        assert!(!read.matches_worker_policy(&signed));
    }

    #[test]
    fn namespace_binding_reader_refusal_reaches_all_owned_guards() {
        let child = Owned::new(true);
        let process = pin_owned_test_helper(child.0.id());
        let mut read = pin_namespace_checked_credentials(&process, policy()).unwrap();
        read.refuse();
        assert!(read.namespace.is_refused() && read.credentials.is_refused());
        refused(&mut read);
    }

    #[test]
    fn namespace_credentials_open_and_read_orders_are_distinct() {
        let mut gate = Gate::default();
        let mut order = Vec::new();
        gate.observe(|step| {
            order.push(step);
            Ok(())
        })
        .unwrap();
        assert_eq!(
            order,
            [
                Step::Namespace,
                Step::Open,
                Step::Namespace,
                Step::Credentials,
                Step::Namespace
            ]
        );
        order.clear();
        gate.observe(|step| {
            order.push(step);
            Ok(())
        })
        .unwrap();
        assert_eq!(order, [Step::Namespace, Step::Credentials, Step::Namespace]);
    }

    #[test]
    fn namespace_credentials_every_open_failure_stops_and_never_reopens() {
        for failure in 0..5 {
            let mut gate = Gate::default();
            let mut calls = 0;
            assert!(matches!(
                gate.observe(|_| {
                    let current = calls;
                    calls += 1;
                    if current == failure {
                        Err(NamespaceError::Drift.into())
                    } else {
                        Ok(())
                    }
                }),
                Err(NamespaceCredentialError::Namespace(NamespaceError::Drift))
            ));
            assert_eq!(calls, failure + 1);
            assert!(gate.refused);
            assert!(!gate.initialized);
            assert!(matches!(
                gate.observe(|_| panic!("no IO after refusal")),
                Err(NamespaceCredentialError::Refused)
            ));
        }
    }

    #[test]
    fn namespace_credentials_every_read_failure_preserves_credential_error() {
        for failure in 0..3 {
            let mut gate = Gate::default();
            gate.observe(|_| Ok(())).unwrap();
            let mut calls = 0;
            assert!(matches!(
                gate.observe(|_| {
                    let current = calls;
                    calls += 1;
                    if current == failure {
                        Err(CredentialError::Budget.into())
                    } else {
                        Ok(())
                    }
                }),
                Err(NamespaceCredentialError::Credentials(
                    CredentialError::Budget
                ))
            ));
            assert_eq!(calls, failure + 1);
            assert!(matches!(
                gate.observe(|_| panic!("no IO after refusal")),
                Err(NamespaceCredentialError::Refused)
            ));
        }
    }

    #[test]
    fn namespace_credentials_seeded_first_error_and_stage_are_sticky() {
        let seed = 0x074f_2026_u64;
        eprintln!("namespace credential mutation seed={seed:#x}");
        let mut rng = seed;
        for _ in 0..256 {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            let mut gate = Gate::default();
            let read = rng & 1 == 1;
            if read {
                gate.observe(|_| Ok(())).unwrap();
            }
            let failure = rng as usize % if read { 3 } else { 5 };
            let mut calls = 0;
            let result = gate.observe(|_| {
                let current = calls;
                calls += 1;
                if current != failure {
                    return Ok(());
                }
                if rng & 2 == 0 {
                    Err(NamespaceError::Invalid.into())
                } else {
                    Err(CredentialError::Drift.into())
                }
            });
            if rng & 2 == 0 {
                assert!(matches!(
                    result,
                    Err(NamespaceCredentialError::Namespace(NamespaceError::Invalid))
                ));
            } else {
                assert!(matches!(
                    result,
                    Err(NamespaceCredentialError::Credentials(
                        CredentialError::Drift
                    ))
                ));
            }
            assert_eq!(calls, failure + 1);
            assert!(matches!(
                gate.observe(|_| panic!("cannot adopt restored input")),
                Err(NamespaceCredentialError::Refused)
            ));
        }
    }

    struct Owned(Child);
    impl Owned {
        fn new(nnp: bool) -> Self {
            let mut command = Command::new("/bin/cat");
            command
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            if nnp {
                // SAFETY: the child hook calls only prctl, with no allocation.
                unsafe {
                    command.pre_exec(|| {
                        if libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0 {
                            return Err(std::io::Error::last_os_error());
                        }
                        Ok(())
                    });
                }
            }
            Self(command.spawn().unwrap())
        }
        fn finish(&mut self) {
            self.0.stdin.take();
            self.0.wait().unwrap();
        }
    }
    impl Drop for Owned {
        fn drop(&mut self) {
            self.0.stdin.take();
            let _ = self.0.wait();
        }
    }
    fn policy() -> AssertedWorkerCredentials {
        // SAFETY: scalar queries and a correctly sized bounded group buffer.
        let (uid, gid, count) = unsafe {
            (
                libc::geteuid(),
                libc::getegid(),
                libc::getgroups(0, std::ptr::null_mut()),
            )
        };
        assert!((0..=32).contains(&count));
        let mut groups = vec![0; count as usize];
        assert_eq!(
            unsafe { libc::getgroups(count, groups.as_mut_ptr()) },
            count
        );
        groups.sort_unstable();
        AssertedWorkerCredentials::new(uid, gid, groups).unwrap()
    }
    fn refused(read: &mut NamespaceCredentialRead<'_>) {
        assert!(read.is_refused());
        assert!(read.namespace.is_refused());
        assert!(read.credentials.is_refused());
        assert!(matches!(
            read.revalidate(),
            Err(NamespaceCredentialError::Refused)
        ));
    }

    #[test]
    fn namespace_credentials_owned_original_reads_and_numeric_api_stay_separate() {
        let child = Owned::new(true);
        let process = pin_owned_test_helper(child.0.id());
        let mut read = pin_namespace_checked_credentials(&process, policy()).unwrap();
        for _ in 0..8 {
            read.revalidate().unwrap();
        }
        assert!(!read.is_refused());
        read.credentials.document.file = File::open("/dev/null").unwrap();
        assert!(matches!(
            read.revalidate(),
            Err(NamespaceCredentialError::Credentials(
                CredentialError::Process(super::super::super::ProcessError::Drift)
            ))
        ));
        refused(&mut read);
        let mut numeric =
            super::super::pin_asserted_worker_credentials(&process, policy()).unwrap();
        numeric.revalidate().unwrap();
        assert!(!numeric.is_refused());
    }

    #[test]
    fn namespace_credentials_owned_wrong_policy_and_unhardened_helper_refuse() {
        let child = Owned::new(true);
        let process = pin_owned_test_helper(child.0.id());
        let mut wrong = policy();
        wrong.uid += 1;
        assert!(matches!(
            pin_namespace_checked_credentials(&process, wrong),
            Err(NamespaceCredentialError::Credentials(
                CredentialError::Drift
            ))
        ));
        assert_eq!(
            unsafe { libc::prctl(libc::PR_GET_NO_NEW_PRIVS, 0, 0, 0, 0) },
            0
        );
        let child = Owned::new(false);
        let process = pin_owned_test_helper(child.0.id());
        assert!(matches!(
            pin_namespace_checked_credentials(&process, policy()),
            Err(NamespaceCredentialError::Credentials(
                CredentialError::Drift
            ))
        ));
    }

    #[test]
    fn namespace_credentials_owned_exit_latches_both_and_constructor_refuses() {
        let mut child = Owned::new(true);
        let process = pin_owned_test_helper(child.0.id());
        let mut read = pin_namespace_checked_credentials(&process, policy()).unwrap();
        child.finish();
        assert!(matches!(
            read.revalidate(),
            Err(NamespaceCredentialError::Namespace(
                NamespaceError::Process(_)
            ))
        ));
        refused(&mut read);
        assert!(matches!(
            pin_namespace_checked_credentials(&process, policy()),
            Err(NamespaceCredentialError::Namespace(
                NamespaceError::Process(_)
            ))
        ));
    }

    #[test]
    fn namespace_credentials_status_substitution_restoration_cannot_refresh() {
        let child = Owned::new(true);
        let process = pin_owned_test_helper(child.0.id());
        for named in [false, true] {
            let mut read = pin_namespace_checked_credentials(&process, policy()).unwrap();
            if named {
                let original = read.credentials.document.name.clone();
                read.credentials.document.name = "stat".into();
                assert!(matches!(
                    read.revalidate(),
                    Err(NamespaceCredentialError::Credentials(
                        CredentialError::Process(super::super::super::ProcessError::Drift)
                    ))
                ));
                read.credentials.document.name = original;
            } else {
                let original = std::mem::replace(
                    &mut read.credentials.document.file,
                    File::open("/dev/null").unwrap(),
                );
                assert!(read.revalidate().is_err());
                read.credentials.document.file = original;
            }
            refused(&mut read);
        }
    }

    #[test]
    fn namespace_credentials_namespace_substitution_latches_credentials_too() {
        let child = Owned::new(true);
        let process = pin_owned_test_helper(child.0.id());
        let mut read = pin_namespace_checked_credentials(&process, policy()).unwrap();
        let original = read
            .namespace
            .replace_worker_for_test(File::open("/dev/null").unwrap());
        assert!(matches!(
            read.revalidate(),
            Err(NamespaceCredentialError::Namespace(NamespaceError::Invalid))
        ));
        read.namespace.replace_worker_for_test(original);
        refused(&mut read);
    }

    #[test]
    fn namespace_credentials_pre_refused_component_latches_the_other() {
        let child = Owned::new(true);
        let process = pin_owned_test_helper(child.0.id());
        for namespace in [false, true] {
            let mut read = pin_namespace_checked_credentials(&process, policy()).unwrap();
            if namespace {
                read.namespace.refuse();
                assert!(matches!(
                    read.revalidate(),
                    Err(NamespaceCredentialError::Namespace(NamespaceError::Refused))
                ));
            } else {
                read.credentials.refuse();
                assert!(matches!(
                    read.revalidate(),
                    Err(NamespaceCredentialError::Credentials(
                        CredentialError::Refused
                    ))
                ));
            }
            refused(&mut read);
        }
    }

    #[test]
    fn namespace_credentials_invalid_private_open_state_refuses_without_reopening() {
        let child = Owned::new(true);
        let process = pin_owned_test_helper(child.0.id());
        let mut read = pin_namespace_checked_credentials(&process, policy()).unwrap();
        let original = read.credentials.document.id;
        // Corrupt private scheduling state only; no public reset exists.
        read.gate.initialized = false;
        assert!(matches!(
            read.revalidate(),
            Err(NamespaceCredentialError::Credentials(
                CredentialError::Invalid
            ))
        ));
        assert_eq!(read.credentials.document.id, original);
        refused(&mut read);
    }

    #[test]
    fn namespace_credentials_concurrent_owned_readers_have_independent_refusal() {
        let child = Owned::new(true);
        let process = pin_owned_test_helper(child.0.id());
        std::thread::scope(|scope| {
            for index in 0..4 {
                let process = &process;
                scope.spawn(move || {
                    let mut read = pin_namespace_checked_credentials(process, policy()).unwrap();
                    if index == 0 {
                        read.credentials.document.file = File::open("/dev/null").unwrap();
                        assert!(read.revalidate().is_err());
                        refused(&mut read);
                    } else {
                        for _ in 0..8 {
                            read.revalidate().unwrap();
                        }
                        assert!(!read.is_refused());
                    }
                });
            }
        });
    }
}
