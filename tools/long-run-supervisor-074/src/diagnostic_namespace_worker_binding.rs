//! Original fixed-account mapping and namespace-checked credentials, not enrollment.

use super::WorkerAccountRead;
use crate::diagnostic_manager::{ManagerClient, WorkerFailure};
use crate::diagnostic_process::{NamespaceCredentialError, NamespaceCredentialRead};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NamespaceWorkerBindingError {
    #[error("original account mapping and namespace checked credential assertions differ")]
    Invalid,
    #[error("original namespace checked worker binding previously refused")]
    Refused,
    #[error("original fixed account observation failed")]
    Account(WorkerFailure),
    #[error(transparent)]
    Reader(#[from] NamespaceCredentialError),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    Account,
    Reader,
}
#[derive(Default)]
struct Gate {
    refused: bool,
}
impl Gate {
    fn observe(
        &mut self,
        input_refused: bool,
        mapping_matches: bool,
        mut read: impl FnMut(Step) -> Result<(), NamespaceWorkerBindingError>,
    ) -> Result<(), NamespaceWorkerBindingError> {
        if self.refused || input_refused {
            self.refused = true;
            return Err(NamespaceWorkerBindingError::Refused);
        }
        if !mapping_matches {
            self.refused = true;
            return Err(NamespaceWorkerBindingError::Invalid);
        }
        for step in [Step::Account, Step::Reader, Step::Account] {
            if let Err(error) = read(step) {
                self.refused = true;
                return Err(error);
            }
        }
        Ok(())
    }
}

/// Borrows original observations; consistency only, not trusted worker enrollment.
pub struct NamespaceWorkerBindingRead<'guard, 'process> {
    account: &'guard mut WorkerAccountRead,
    credentials: &'guard mut NamespaceCredentialRead<'process>,
    gate: Gate,
}
/// No replacement policy/process or already-open numeric credential reader is accepted.
pub fn bind_namespace_checked_worker<'guard, 'process>(
    account: &'guard mut WorkerAccountRead,
    credentials: &'guard mut NamespaceCredentialRead<'process>,
    manager: &mut ManagerClient,
) -> Result<NamespaceWorkerBindingRead<'guard, 'process>, NamespaceWorkerBindingError> {
    let mut read = NamespaceWorkerBindingRead {
        account,
        credentials,
        gate: Gate::default(),
    };
    read.revalidate(manager)?;
    Ok(read)
}
impl NamespaceWorkerBindingRead<'_, '_> {
    pub fn is_refused(&self) -> bool {
        self.gate.refused || self.account.is_refused() || self.credentials.is_refused()
    }
    /// Sequential checks only; no whole-composition deadline or live start capability.
    pub fn revalidate(
        &mut self,
        manager: &mut ManagerClient,
    ) -> Result<(), NamespaceWorkerBindingError> {
        self.revalidate_with(|account| account.revalidate(manager))
    }
    fn revalidate_with(
        &mut self,
        mut account_read: impl FnMut(&mut WorkerAccountRead) -> Result<(), WorkerFailure>,
    ) -> Result<(), NamespaceWorkerBindingError> {
        let input_refused = self.is_refused();
        let mapping_matches = self.credentials.matches_account(self.account);
        let result = self
            .gate
            .observe(input_refused, mapping_matches, |step| match step {
                Step::Account => {
                    account_read(self.account).map_err(NamespaceWorkerBindingError::Account)
                }
                Step::Reader => self.credentials.revalidate().map_err(Into::into),
            });
        if result.is_err() {
            self.account.refuse();
            self.credentials.refuse();
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::super::{WorkerAccountSnapshot, ACCOUNT};
    use super::*;
    use crate::diagnostic_manager::WorkerFailureKind;
    use crate::diagnostic_process::{
        pin_namespace_checked_credentials, pin_owned_test_helper, AssertedWorkerCredentials,
        CredentialError, NamespaceError,
    };
    use std::os::unix::process::CommandExt;
    use std::process::{Child, Command, Stdio};

    fn snapshot(uid: u32, gid: u32, mut groups: Vec<u32>) -> WorkerAccountSnapshot {
        if let Err(position) = groups.binary_search(&gid) {
            groups.insert(position, gid);
        }
        let value = WorkerAccountSnapshot {
            schema_version: 1,
            account: ACCOUNT.into(),
            group: ACCOUNT.into(),
            uid,
            gid,
            membership_gids: groups,
        };
        WorkerAccountSnapshot::decode(&crate::canonical_json(&value).unwrap()).unwrap()
    }

    #[test]
    fn namespace_binding_order_keeps_original_reader_between_accounts() {
        let mut gate = Gate::default();
        let mut steps = Vec::new();
        gate.observe(false, true, |step| {
            steps.push(step);
            Ok(())
        })
        .unwrap();
        assert_eq!(steps, [Step::Account, Step::Reader, Step::Account]);
        assert!(!gate.refused);
    }

    #[test]
    fn namespace_binding_prior_refusal_and_mismatch_make_no_observations() {
        for (refused, mapping) in [(true, true), (true, false), (false, false)] {
            let mut gate = Gate::default();
            let result = gate.observe(refused, mapping, |_| panic!("no observation"));
            if refused {
                assert!(matches!(result, Err(NamespaceWorkerBindingError::Refused)));
            } else {
                assert!(matches!(result, Err(NamespaceWorkerBindingError::Invalid)));
            }
            assert!(gate.refused);
            assert!(matches!(
                gate.observe(false, true, |_| panic!("no refresh")),
                Err(NamespaceWorkerBindingError::Refused)
            ));
        }
    }

    #[test]
    fn namespace_binding_every_failure_keeps_typed_first_error_and_details() {
        for failed in 0..3 {
            for namespace in [false, true] {
                let mut gate = Gate::default();
                let mut calls = 0;
                let error = gate
                    .observe(false, true, |step| {
                        let current = calls;
                        calls += 1;
                        if current != failed {
                            return Ok(());
                        }
                        match step {
                            Step::Account => {
                                Err(NamespaceWorkerBindingError::Account(WorkerFailure {
                                    kind: WorkerFailureKind::Deadline,
                                    stdout: b"prefix".to_vec(),
                                    stderr: b"partial".to_vec(),
                                    cleanup_confirmed: false,
                                }))
                            }
                            Step::Reader if namespace => {
                                Err(NamespaceCredentialError::Namespace(NamespaceError::Drift)
                                    .into())
                            }
                            Step::Reader => Err(NamespaceCredentialError::Credentials(
                                CredentialError::Budget,
                            )
                            .into()),
                        }
                    })
                    .unwrap_err();
                assert_eq!(calls, failed + 1);
                match error {
                    NamespaceWorkerBindingError::Account(error) => {
                        assert_eq!(error.kind, WorkerFailureKind::Deadline);
                        assert_eq!(error.stdout, b"prefix");
                        assert_eq!(error.stderr, b"partial");
                        assert!(!error.cleanup_confirmed);
                    }
                    NamespaceWorkerBindingError::Reader(NamespaceCredentialError::Namespace(
                        NamespaceError::Drift,
                    )) => assert!(namespace),
                    NamespaceWorkerBindingError::Reader(NamespaceCredentialError::Credentials(
                        CredentialError::Budget,
                    )) => assert!(!namespace),
                    other => panic!("unexpected {other:?}"),
                }
                assert!(matches!(
                    gate.observe(false, true, |_| panic!("no IO after first error")),
                    Err(NamespaceWorkerBindingError::Refused)
                ));
            }
        }
    }

    #[test]
    fn namespace_binding_seeded_valid_mapping_drift_cannot_be_adopted() {
        let seed = 0x0750_2026_u64;
        eprintln!("namespace binding mutation seed={seed:#x}");
        let mut rng = seed;
        for _ in 0..256 {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            let mut changed = snapshot(1000, 1001, vec![4]);
            match rng % 3 {
                0 => changed.uid += 1,
                1 => {
                    changed.gid += 1;
                    changed.membership_gids = vec![4, changed.gid];
                }
                _ => changed.membership_gids.insert(1, 5),
            }
            let changed =
                WorkerAccountSnapshot::decode(&crate::canonical_json(&changed).unwrap()).unwrap();
            let account = WorkerAccountRead::new(changed);
            let mut gate = Gate::default();
            assert!(matches!(
                gate.observe(
                    false,
                    account.matches_assertions(1000, 1001, &[4]),
                    |_| panic!("mapping drift cannot read")
                ),
                Err(NamespaceWorkerBindingError::Invalid)
            ));
            assert!(matches!(
                gate.observe(false, true, |_| panic!("restoration cannot refresh")),
                Err(NamespaceWorkerBindingError::Refused)
            ));
        }
    }

    struct Owned(Child);
    impl Owned {
        fn new() -> Self {
            let mut command = Command::new("/bin/cat");
            command
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            // SAFETY: only async-signal-safe prctl before exec, no allocations.
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
        fn finish(&mut self) {
            self.0.stdin.take();
            self.0.wait().unwrap();
        }
    }
    impl Drop for Owned {
        fn drop(&mut self) {
            self.finish();
        }
    }
    fn assertions() -> (u32, u32, Vec<u32>) {
        // SAFETY: scalar queries and a group buffer of exactly the requested size.
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
        assert!(!groups.contains(&0));
        (uid, gid, groups)
    }
    fn reader(
        process: &crate::diagnostic_process::ProcessRead,
    ) -> (NamespaceCredentialRead<'_>, WorkerAccountSnapshot) {
        let (uid, gid, groups) = assertions();
        (
            pin_namespace_checked_credentials(
                process,
                AssertedWorkerCredentials::new(uid, gid, groups.clone()).unwrap(),
            )
            .unwrap(),
            snapshot(uid, gid, groups),
        )
    }

    #[test]
    fn namespace_binding_owned_success_drop_preserves_healthy_original_inputs() {
        let child = Owned::new();
        let process = pin_owned_test_helper(child.0.id());
        let (mut credentials, original) = reader(&process);
        let mut account = WorkerAccountRead::new(original.clone());
        {
            let mut binding = NamespaceWorkerBindingRead {
                account: &mut account,
                credentials: &mut credentials,
                gate: Gate::default(),
            };
            for _ in 0..8 {
                binding
                    .revalidate_with(|account| account.observe(|| Ok(original.clone())))
                    .unwrap();
            }
            assert!(!binding.is_refused());
        }
        assert!(!account.is_refused() && !credentials.is_refused());
        credentials.revalidate().unwrap();
        account.observe(|| Ok(original)).unwrap();
    }

    #[test]
    fn namespace_binding_owned_first_and_second_account_drift_latch_after_drop() {
        let child = Owned::new();
        let process = pin_owned_test_helper(child.0.id());
        for failed in [0, 1] {
            let (mut credentials, original) = reader(&process);
            let mut account = WorkerAccountRead::new(original.clone());
            {
                let mut binding = NamespaceWorkerBindingRead {
                    account: &mut account,
                    credentials: &mut credentials,
                    gate: Gate::default(),
                };
                let mut calls = 0;
                assert!(matches!(
                    binding.revalidate_with(|account| {
                        let mut observed = original.clone();
                        if calls == failed {
                            observed.uid += 1;
                        }
                        calls += 1;
                        account.observe(|| Ok(observed))
                    }),
                    Err(NamespaceWorkerBindingError::Account(_))
                ));
                assert_eq!(calls, failed + 1);
                assert!(matches!(
                    binding.revalidate_with(|_| panic!("no restoration read")),
                    Err(NamespaceWorkerBindingError::Refused)
                ));
            }
            assert!(account.is_refused() && credentials.is_refused());
            assert!(matches!(
                credentials.revalidate(),
                Err(NamespaceCredentialError::Refused)
            ));
        }
    }

    #[test]
    fn namespace_binding_owned_exit_preserves_namespace_error_and_refuses_account() {
        let mut child = Owned::new();
        let process = pin_owned_test_helper(child.0.id());
        let (mut credentials, original) = reader(&process);
        let mut account = WorkerAccountRead::new(original.clone());
        {
            let mut binding = NamespaceWorkerBindingRead {
                account: &mut account,
                credentials: &mut credentials,
                gate: Gate::default(),
            };
            binding
                .revalidate_with(|account| account.observe(|| Ok(original.clone())))
                .unwrap();
            child.finish();
            assert!(matches!(
                binding.revalidate_with(|account| account.observe(|| Ok(original.clone()))),
                Err(NamespaceWorkerBindingError::Reader(
                    NamespaceCredentialError::Namespace(NamespaceError::Process(_))
                ))
            ));
        }
        assert!(account.is_refused() && credentials.is_refused());
        assert!(matches!(
            credentials.revalidate(),
            Err(NamespaceCredentialError::Refused)
        ));
    }

    #[test]
    fn namespace_binding_public_mapping_mismatch_refuses_without_helper() {
        let child = Owned::new();
        let process = pin_owned_test_helper(child.0.id());
        let (mut credentials, mut original) = reader(&process);
        original.uid += 1;
        let mut account = WorkerAccountRead::new(original);
        let mut manager = ManagerClient::default();
        assert!(matches!(
            bind_namespace_checked_worker(&mut account, &mut credentials, &mut manager),
            Err(NamespaceWorkerBindingError::Invalid)
        ));
        assert!(account.is_refused() && credentials.is_refused());
        assert!(manager.pending.is_none());
        assert!(matches!(
            credentials.revalidate(),
            Err(NamespaceCredentialError::Refused)
        ));
    }

    #[test]
    fn namespace_binding_public_prior_refusal_latches_other_without_helper() {
        let child = Owned::new();
        let process = pin_owned_test_helper(child.0.id());
        for refused_account in [false, true] {
            let (mut credentials, original) = reader(&process);
            let mut account = WorkerAccountRead::new(original);
            if refused_account {
                account.refuse();
            } else {
                credentials.refuse();
            }
            let mut manager = ManagerClient::default();
            assert!(matches!(
                bind_namespace_checked_worker(&mut account, &mut credentials, &mut manager),
                Err(NamespaceWorkerBindingError::Refused)
            ));
            assert!(account.is_refused() && credentials.is_refused());
            assert!(manager.pending.is_none());
        }
    }

    #[test]
    fn namespace_binding_public_pending_cleanup_preserves_exact_owned_helper() {
        let child = Owned::new();
        let process = pin_owned_test_helper(child.0.id());
        let (mut credentials, original) = reader(&process);
        let mut account = WorkerAccountRead::new(original);
        let held = Command::new("/bin/cat")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let pid = held.id();
        let mut manager = ManagerClient {
            pending: Some(held),
        };
        let error =
            match bind_namespace_checked_worker(&mut account, &mut credentials, &mut manager) {
                Err(error) => error,
                Ok(_) => panic!("pending helper must refuse"),
            };
        let mut held = manager.pending.take().unwrap();
        let unchanged = held.id() == pid && held.try_wait().unwrap().is_none();
        held.kill().unwrap();
        held.wait().unwrap();
        assert!(unchanged);
        match error {
            NamespaceWorkerBindingError::Account(error) => {
                assert_eq!(error.kind, WorkerFailureKind::PendingCleanup);
                assert!(!error.cleanup_confirmed);
            }
            other => panic!("unexpected {other:?}"),
        }
        assert!(account.is_refused() && credentials.is_refused());
    }

    #[test]
    fn namespace_binding_owned_concurrent_bindings_do_not_share_refusal() {
        let child = Owned::new();
        let process = pin_owned_test_helper(child.0.id());
        std::thread::scope(|scope| {
            for index in 0..4 {
                let process = &process;
                scope.spawn(move || {
                    let (mut credentials, original) = reader(process);
                    let mut account = WorkerAccountRead::new(original.clone());
                    let mut binding = NamespaceWorkerBindingRead {
                        account: &mut account,
                        credentials: &mut credentials,
                        gate: Gate::default(),
                    };
                    binding
                        .revalidate_with(|account| account.observe(|| Ok(original.clone())))
                        .unwrap();
                    if index == 0 {
                        assert!(binding
                            .revalidate_with(|_| Err(super::super::failure(WorkerFailureKind::Io)))
                            .is_err());
                    } else {
                        for _ in 0..8 {
                            binding
                                .revalidate_with(|account| account.observe(|| Ok(original.clone())))
                                .unwrap();
                        }
                    }
                    assert_eq!(binding.is_refused(), index == 0);
                });
            }
        });
    }
}
