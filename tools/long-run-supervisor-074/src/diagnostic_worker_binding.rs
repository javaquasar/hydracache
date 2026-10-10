//! Asserted read-only mapping consistency, never trusted account enrollment.

use super::WorkerAccountRead;
use crate::diagnostic_manager::{ManagerClient, WorkerFailure};
use crate::diagnostic_process::{CredentialError, ProcessCredentialRead};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WorkerBindingError {
    #[error("asserted worker account and original credential policy do not agree")]
    Invalid,
    #[error("original worker binding previously refused")]
    Refused,
    #[error("fixed worker account observation failed")]
    Account(WorkerFailure),
    #[error(transparent)]
    Credentials(#[from] CredentialError),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    Account,
    Credentials,
}
#[derive(Default)]
struct BindingGuard {
    refused: bool,
}
impl BindingGuard {
    fn observe(
        &mut self,
        input_refused: bool,
        mapping_matches: bool,
        mut read: impl FnMut(Step) -> Result<(), WorkerBindingError>,
    ) -> Result<(), WorkerBindingError> {
        if self.refused || input_refused {
            self.refused = true;
            return Err(WorkerBindingError::Refused);
        }
        if !mapping_matches {
            self.refused = true;
            return Err(WorkerBindingError::Invalid);
        }
        for step in [Step::Account, Step::Credentials, Step::Account] {
            if let Err(error) = read(step) {
                self.refused = true;
                return Err(error);
            }
        }
        Ok(())
    }
}

/// Borrows original guards, never enrolls their assertions as trusted host policy.
pub struct AssertedWorkerBindingRead<'guard, 'process> {
    account: &'guard mut WorkerAccountRead,
    credentials: &'guard mut ProcessCredentialRead<'process>,
    guard: BindingGuard,
}
/// No process/PID or numeric policy inputs: use the already retained originals.
pub fn bind_asserted_worker<'guard, 'process>(
    account: &'guard mut WorkerAccountRead,
    credentials: &'guard mut ProcessCredentialRead<'process>,
    manager: &mut ManagerClient,
) -> Result<AssertedWorkerBindingRead<'guard, 'process>, WorkerBindingError> {
    let mut read = AssertedWorkerBindingRead {
        account,
        credentials,
        guard: BindingGuard::default(),
    };
    read.revalidate(manager)?;
    Ok(read)
}
impl AssertedWorkerBindingRead<'_, '_> {
    pub fn is_refused(&self) -> bool {
        self.guard.refused || self.account.is_refused() || self.credentials.is_refused()
    }
    /// Sequential bracket only; not an atomic snapshot or whole-operation deadline.
    pub fn revalidate(&mut self, manager: &mut ManagerClient) -> Result<(), WorkerBindingError> {
        self.revalidate_with(|account| account.revalidate(manager))
    }
    fn revalidate_with(
        &mut self,
        mut account_read: impl FnMut(&mut WorkerAccountRead) -> Result<(), WorkerFailure>,
    ) -> Result<(), WorkerBindingError> {
        let input_refused = self.is_refused();
        let mapping_matches = self.credentials.matches_account(self.account);
        let result = self
            .guard
            .observe(input_refused, mapping_matches, |step| match step {
                Step::Account => account_read(self.account).map_err(WorkerBindingError::Account),
                Step::Credentials => self.credentials.revalidate().map_err(Into::into),
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
    use super::super::{WorkerAccountRead, WorkerAccountSnapshot, ACCOUNT};
    use super::*;
    use crate::diagnostic_manager::{ManagerClient, WorkerFailure, WorkerFailureKind};
    use crate::diagnostic_process::{
        pin_asserted_worker_credentials, pin_owned_test_helper, AssertedWorkerCredentials,
    };
    use std::cell::RefCell;
    use std::os::unix::process::CommandExt;
    use std::process::{Child, Command, Stdio};

    fn snapshot(uid: u32, gid: u32, mut groups: Vec<u32>) -> WorkerAccountSnapshot {
        if let Err(position) = groups.binary_search(&gid) {
            groups.insert(position, gid);
        }
        let result = WorkerAccountSnapshot {
            schema_version: 1,
            account: ACCOUNT.into(),
            group: ACCOUNT.into(),
            uid,
            gid,
            membership_gids: groups,
        };
        let bytes = crate::canonical_json(&result).unwrap();
        WorkerAccountSnapshot::decode(&bytes).unwrap()
    }
    #[test]
    fn mapping_union_keeps_explicit_primary_inclusion_or_omission() {
        let account = WorkerAccountRead::new(snapshot(1000, 1001, vec![4]));
        assert!(account.matches_assertions(1000, 1001, &[4]));
        assert!(account.matches_assertions(1000, 1001, &[4, 1001]));
        let primary_only = WorkerAccountRead::new(snapshot(1000, 1001, vec![]));
        assert!(primary_only.matches_assertions(1000, 1001, &[]));
        assert!(primary_only.matches_assertions(1000, 1001, &[1001]));
    }
    #[test]
    fn mapping_union_refuses_wrong_ids_missing_foreign_root_and_invalid_groups() {
        let mut account = WorkerAccountRead::new(snapshot(1000, 1001, vec![4]));
        for (uid, gid, groups) in [
            (1002, 1001, vec![4]),
            (1000, 1002, vec![4]),
            (0, 1001, vec![4]),
            (1000, 0, vec![4]),
            (1000, 1001, vec![]),
            (1000, 1001, vec![4, 5]),
            (1000, 1001, vec![0, 4]),
            (1000, 1001, vec![4, 4]),
            (1000, 1001, vec![1001, 4]),
            (1000, 1001, vec![4, u32::MAX]),
            (1000, 1001, (1..=33).collect()),
        ] {
            assert!(!account.matches_assertions(uid, gid, &groups));
        }
        account.refuse();
        assert!(!account.matches_assertions(1000, 1001, &[4]));
    }
    #[test]
    fn mapping_union_full_bound_does_not_grow_or_drop_membership() {
        let groups = (1..=32).collect::<Vec<_>>();
        let account = WorkerAccountRead::new(snapshot(1000, 32, groups.clone()));
        assert!(account.matches_assertions(1000, 32, &groups));
        assert!(account.matches_assertions(1000, 32, &groups[..31]));
        assert!(!account.matches_assertions(1000, 32, &groups[..30]));
        let root_member = WorkerAccountRead::new(snapshot(1000, 32, vec![0, 32]));
        assert!(!root_member.matches_assertions(1000, 32, &[0, 32]));
    }
    #[test]
    fn binding_bracket_has_exact_account_credential_account_order() {
        let mut gate = BindingGuard::default();
        let steps = RefCell::new(vec![]);
        gate.observe(false, true, |step| {
            steps.borrow_mut().push(step);
            Ok(())
        })
        .unwrap();
        assert_eq!(
            *steps.borrow(),
            [Step::Account, Step::Credentials, Step::Account]
        );
        assert!(!gate.refused);
    }
    #[test]
    fn binding_prior_refusal_and_mapping_mismatch_perform_no_observation() {
        for (refused, matches) in [(true, true), (false, false)] {
            let mut gate = BindingGuard::default();
            let error = gate
                .observe(refused, matches, |_| panic!("must not read"))
                .unwrap_err();
            assert!(if refused {
                matches!(error, WorkerBindingError::Refused)
            } else {
                matches!(error, WorkerBindingError::Invalid)
            });
            assert!(gate.refused);
            assert!(matches!(
                gate.observe(false, true, |_| panic!("must not refresh")),
                Err(WorkerBindingError::Refused)
            ));
        }
    }
    #[test]
    fn binding_every_failure_preserves_first_error_and_stops_the_bracket() {
        for failed in 0..3 {
            let mut gate = BindingGuard::default();
            let mut calls = 0;
            let error = gate
                .observe(false, true, |step| {
                    let index = calls;
                    calls += 1;
                    if index != failed {
                        return Ok(());
                    }
                    match step {
                        Step::Account => Err(WorkerBindingError::Account(WorkerFailure {
                            kind: WorkerFailureKind::Deadline,
                            stdout: b"prefix".to_vec(),
                            stderr: b"partial".to_vec(),
                            cleanup_confirmed: false,
                        })),
                        Step::Credentials => Err(WorkerBindingError::Credentials(
                            crate::diagnostic_process::CredentialError::Drift,
                        )),
                    }
                })
                .unwrap_err();
            assert_eq!(calls, failed + 1);
            match error {
                WorkerBindingError::Account(error) => {
                    assert_eq!(error.kind, WorkerFailureKind::Deadline);
                    assert_eq!(error.stdout, b"prefix");
                    assert_eq!(error.stderr, b"partial");
                    assert!(!error.cleanup_confirmed);
                }
                WorkerBindingError::Credentials(
                    crate::diagnostic_process::CredentialError::Drift,
                ) => {}
                other => panic!("unexpected {other:?}"),
            }
            assert!(matches!(
                gate.observe(false, true, |_| panic!("no reads after failure")),
                Err(WorkerBindingError::Refused)
            ));
        }
    }
    #[test]
    fn seeded_valid_mapping_mutations_cannot_be_adopted() {
        let seed = 0x074b_2026_u64;
        eprintln!("binding mutation seed={seed:#x}");
        let mut rng = seed;
        for _ in 0..256 {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            let original = WorkerAccountRead::new(snapshot(1000, 1001, vec![4]));
            let mut changed = original.original.clone();
            match rng % 3 {
                0 => changed.uid += 1,
                1 => {
                    changed.gid += 1;
                    changed.membership_gids = vec![4, changed.gid];
                }
                _ => changed.membership_gids.insert(1, 5),
            }
            let bytes = crate::canonical_json(&changed).unwrap();
            WorkerAccountSnapshot::decode(&bytes).unwrap();
            let account = WorkerAccountRead::new(changed);
            let mut gate = BindingGuard::default();
            assert!(gate
                .observe(
                    false,
                    account.matches_assertions(1000, 1001, &[4]),
                    |_| panic!("mismatch must not read")
                )
                .is_err());
            assert!(gate.refused);
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
            // SAFETY: only a self-only async-signal-safe prctl before exec.
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
        // SAFETY: scalar queries and an array with exactly the requested capacity.
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
    #[test]
    fn owned_original_credentials_and_synthetic_account_bind_without_refresh() {
        let child = Owned::new();
        let process = pin_owned_test_helper(child.0.id());
        let (uid, gid, groups) = assertions();
        let mut credentials = pin_asserted_worker_credentials(
            &process,
            AssertedWorkerCredentials::new(uid, gid, groups.clone()).unwrap(),
        )
        .unwrap();
        let original = snapshot(uid, gid, groups);
        let mut account = WorkerAccountRead::new(original.clone());
        {
            let mut read = AssertedWorkerBindingRead {
                account: &mut account,
                credentials: &mut credentials,
                guard: BindingGuard::default(),
            };
            for _ in 0..5 {
                read.revalidate_with(|account| account.observe(|| Ok(original.clone())))
                    .unwrap();
            }
            assert!(!read.is_refused());
            let mut changed = original.clone();
            changed.uid += 1;
            assert!(matches!(
                read.revalidate_with(|account| account.observe(|| Ok(changed.clone()))),
                Err(WorkerBindingError::Account(_))
            ));
            assert!(matches!(
                read.revalidate_with(|_| panic!("no restoration read")),
                Err(WorkerBindingError::Refused)
            ));
        }
        assert!(account.is_refused() && credentials.is_refused());
        assert!(credentials.revalidate().is_err());
    }
    #[test]
    fn owned_original_exit_refuses_account_and_credentials_after_wrapper_drop() {
        let mut child = Owned::new();
        let process = pin_owned_test_helper(child.0.id());
        let (uid, gid, groups) = assertions();
        let mut credentials = pin_asserted_worker_credentials(
            &process,
            AssertedWorkerCredentials::new(uid, gid, groups.clone()).unwrap(),
        )
        .unwrap();
        let original = snapshot(uid, gid, groups);
        let mut account = WorkerAccountRead::new(original.clone());
        {
            let mut read = AssertedWorkerBindingRead {
                account: &mut account,
                credentials: &mut credentials,
                guard: BindingGuard::default(),
            };
            read.revalidate_with(|account| account.observe(|| Ok(original.clone())))
                .unwrap();
            child.finish();
            assert!(matches!(
                read.revalidate_with(|account| account.observe(|| Ok(original.clone()))),
                Err(WorkerBindingError::Credentials(_))
            ));
        }
        assert!(account.is_refused() && credentials.is_refused());
    }
    #[test]
    fn public_binding_mapping_mismatch_latches_both_original_inputs() {
        let child = Owned::new();
        let process = pin_owned_test_helper(child.0.id());
        let (uid, gid, groups) = assertions();
        let mut credentials = pin_asserted_worker_credentials(
            &process,
            AssertedWorkerCredentials::new(uid, gid, groups.clone()).unwrap(),
        )
        .unwrap();
        let mut account = WorkerAccountRead::new(snapshot(uid + 1, gid, groups));
        let mut manager = ManagerClient::default();
        assert!(matches!(
            bind_asserted_worker(&mut account, &mut credentials, &mut manager),
            Err(WorkerBindingError::Invalid)
        ));
        assert!(account.is_refused() && credentials.is_refused());
        assert!(manager.pending.is_none());
    }
    #[test]
    fn public_binding_pre_refused_input_latches_the_other_without_helper_reads() {
        let child = Owned::new();
        let process = pin_owned_test_helper(child.0.id());
        let (uid, gid, groups) = assertions();
        for refused_account in [true, false] {
            let mut credentials = pin_asserted_worker_credentials(
                &process,
                AssertedWorkerCredentials::new(uid, gid, groups.clone()).unwrap(),
            )
            .unwrap();
            let mut account = WorkerAccountRead::new(snapshot(uid, gid, groups.clone()));
            if refused_account {
                account.refuse();
            } else {
                credentials.refuse();
            }
            let mut manager = ManagerClient::default();
            assert!(matches!(
                bind_asserted_worker(&mut account, &mut credentials, &mut manager),
                Err(WorkerBindingError::Refused)
            ));
            assert!(account.is_refused() && credentials.is_refused());
            assert!(manager.pending.is_none());
        }
    }
    #[test]
    fn public_binding_pending_helper_cleanup_is_preserved_and_latches_inputs() {
        let child = Owned::new();
        let process = pin_owned_test_helper(child.0.id());
        let (uid, gid, groups) = assertions();
        let mut credentials = pin_asserted_worker_credentials(
            &process,
            AssertedWorkerCredentials::new(uid, gid, groups.clone()).unwrap(),
        )
        .unwrap();
        let mut account = WorkerAccountRead::new(snapshot(uid, gid, groups));
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
        let result = bind_asserted_worker(&mut account, &mut credentials, &mut manager);
        let error = match result {
            Err(error) => error,
            Ok(_) => panic!("pending helper must refuse"),
        };
        let mut held = manager.pending.take().unwrap();
        let same = held.id() == pid && held.try_wait().unwrap().is_none();
        held.kill().unwrap();
        held.wait().unwrap();
        match error {
            WorkerBindingError::Account(error) => {
                assert_eq!(error.kind, WorkerFailureKind::PendingCleanup);
                assert!(!error.cleanup_confirmed);
            }
            other => panic!("unexpected {other:?}"),
        }
        assert!(same);
        assert!(account.is_refused() && credentials.is_refused());
    }
    #[test]
    fn owned_concurrent_bindings_keep_independent_refusal_state() {
        let child = Owned::new();
        let process = pin_owned_test_helper(child.0.id());
        let (uid, gid, groups) = assertions();
        std::thread::scope(|scope| {
            for index in 0..4 {
                let groups = groups.clone();
                let process = &process;
                scope.spawn(move || {
                    let mut credentials = pin_asserted_worker_credentials(
                        process,
                        AssertedWorkerCredentials::new(uid, gid, groups.clone()).unwrap(),
                    )
                    .unwrap();
                    let original = snapshot(uid, gid, groups);
                    let mut account = WorkerAccountRead::new(original.clone());
                    let mut read = AssertedWorkerBindingRead {
                        account: &mut account,
                        credentials: &mut credentials,
                        guard: BindingGuard::default(),
                    };
                    read.revalidate_with(|account| account.observe(|| Ok(original.clone())))
                        .unwrap();
                    if index == 0 {
                        assert!(read
                            .revalidate_with(|_| Err(super::super::failure(WorkerFailureKind::Io)))
                            .is_err());
                    } else {
                        for _ in 0..5 {
                            read.revalidate_with(|account| {
                                account.observe(|| Ok(original.clone()))
                            })
                            .unwrap();
                        }
                    }
                    assert_eq!(read.is_refused(), index == 0);
                });
            }
        });
    }
}
