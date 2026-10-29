//! Stable read-only identity join, never launch, stop or release authority.
use crate::diagnostic_lease::{DiagnosticState, SURFACES};
use crate::diagnostic_loaded::{InvocationGuard, LoadedSnapshot};
use crate::diagnostic_manager::{ManagerClient, WorkerFailure};
use crate::diagnostic_process::{ExpectedGeneration, ProcessError, ProcessRead};
use crate::diagnostic_tree::{CgroupId, DiagnosticTreeScope, NodeSnapshot, TreeError, TreeRead};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IdentityError {
    #[error("original diagnostic identity join or live status refused")]
    Invalid,
    #[error("original diagnostic identity join previously refused")]
    Refused,
    #[error("original manager observation failed: {0:?}")]
    Manager(WorkerFailure),
    #[error(transparent)]
    Process(#[from] ProcessError),
    #[error(transparent)]
    Tree(#[from] TreeError),
}

struct JoinView<'a> {
    scoped: bool,
    kernel: bool,
    root: CgroupId,
    nodes: &'a [NodeSnapshot],
}
fn tree_view<'a>(tree: &'a TreeRead, scope: &DiagnosticTreeScope) -> JoinView<'a> {
    let snapshot = tree.snapshot();
    JoinView {
        scoped: tree.is_kernel_scope(scope),
        kernel: snapshot.kernel_origin(),
        root: snapshot.root_identity(),
        nodes: snapshot.nodes(),
    }
}
struct IdentityGuard {
    invocation: InvocationGuard,
    generation: ExpectedGeneration,
    root: CgroupId,
    refused: bool,
}
impl IdentityGuard {
    fn pin(
        state: &DiagnosticState,
        snapshot: &LoadedSnapshot,
        generation: &ExpectedGeneration,
        tree: JoinView<'_>,
    ) -> Result<Self, IdentityError> {
        let invocation = snapshot
            .pin_for(state)
            .map_err(|_| IdentityError::Invalid)?;
        if generation.boot_id != state.identity.boot_id || generation.start_ticks == 0 {
            return Err(IdentityError::Invalid);
        }
        let guard = Self {
            invocation,
            generation: generation.clone(),
            root: tree.root,
            refused: false,
        };
        guard.check_unit(snapshot)?;
        guard.check_tree(snapshot, tree)?;
        Ok(guard)
    }
    fn check_unit(&self, snapshot: &LoadedSnapshot) -> Result<(), IdentityError> {
        let unit = snapshot.manager().unit().ok_or(IdentityError::Invalid)?;
        if unit.main_pid == 0
            || unit.main_pid != self.generation.pid
            || unit.active_state != "active"
            || unit.sub_state != "running"
            || unit.result != "success"
        {
            return Err(IdentityError::Invalid);
        }
        Ok(())
    }
    fn check_tree(
        &self,
        snapshot: &LoadedSnapshot,
        tree: JoinView<'_>,
    ) -> Result<(), IdentityError> {
        let unit = snapshot.manager().unit().ok_or(IdentityError::Invalid)?;
        if !tree.scoped
            || !tree.kernel
            || tree.root != self.root
            || tree.root.device == 0
            || tree.root.inode == 0
            || tree.nodes.is_empty()
            || tree.nodes.iter().any(|node| node.frozen)
        {
            return Err(IdentityError::Invalid);
        }
        let mut members = tree
            .nodes
            .iter()
            .filter(|node| node.pids.contains(&self.generation.pid));
        let member = members.next().ok_or(IdentityError::Invalid)?;
        let expected_path = if member.relative_path.is_empty() {
            unit.control_group.clone()
        } else {
            format!("{}/{}", unit.control_group, member.relative_path)
        };
        if members.next().is_some()
            || !member.populated
            || self.generation.cgroup_path != expected_path
        {
            return Err(IdentityError::Invalid);
        }
        Ok(())
    }
    fn revalidate_with(
        &mut self,
        mut observe: impl FnMut(&mut InvocationGuard) -> Result<LoadedSnapshot, WorkerFailure>,
        mut sources: impl FnMut() -> Result<(), IdentityError>,
    ) -> Result<(), IdentityError> {
        if self.is_refused() {
            return Err(IdentityError::Refused);
        }
        let result = (|| {
            let before = observe(&mut self.invocation).map_err(IdentityError::Manager)?;
            self.invocation
                .revalidate(&before)
                .map_err(|_| IdentityError::Invalid)?;
            self.check_unit(&before)?;
            sources()?;
            sources()?;
            let after = observe(&mut self.invocation).map_err(IdentityError::Manager)?;
            self.invocation
                .revalidate(&after)
                .map_err(|_| IdentityError::Invalid)?;
            self.check_unit(&after)
        })();
        if result.is_err() {
            self.refused = true;
            self.invocation.refuse();
        }
        result
    }
    fn is_refused(&self) -> bool {
        self.refused || self.invocation.is_refused()
    }
}

/// Borrows original kernel readers; no fixture promotion or descriptor export.
pub struct LiveIdentityRead<'a> {
    guard: IdentityGuard,
    state: DiagnosticState,
    process: &'a ProcessRead,
    tree: &'a TreeRead,
    scope: DiagnosticTreeScope,
}
impl<'a> LiveIdentityRead<'a> {
    pub(crate) fn state(&self) -> &DiagnosticState {
        &self.state
    }
    pub(crate) fn original_process(&self) -> &'a ProcessRead {
        self.process
    }
    pub(crate) fn refuse(&mut self) {
        self.guard.refused = true;
        self.guard.invocation.refuse();
    }
    /// Two bounded manager reads and two retained process/tree checks; no refresh.
    pub fn revalidate(&mut self, manager: &mut ManagerClient) -> Result<(), IdentityError> {
        self.guard.revalidate_with(
            |invocation| manager.inspect_original(invocation),
            || {
                self.process.revalidate()?;
                if !self.tree.is_kernel_scope(&self.scope) {
                    return Err(IdentityError::Invalid);
                }
                self.tree.revalidate()?;
                self.process.revalidate()?;
                Ok(())
            },
        )
    }
    pub fn is_refused(&self) -> bool {
        self.guard.is_refused()
    }
}

/// Caller state and the tree's expected inode are assertions, not original-start
/// authentication. Success is only sequential consistency of retained reads.
pub fn pin_live_identity<'a>(
    state: &DiagnosticState,
    snapshot: &LoadedSnapshot,
    process: &'a ProcessRead,
    tree: &'a TreeRead,
    manager: &mut ManagerClient,
) -> Result<LiveIdentityRead<'a>, IdentityError> {
    let surface = SURFACES
        .get(state.completed_cells)
        .ok_or(IdentityError::Invalid)?;
    let scope = DiagnosticTreeScope::new(&state.identity, surface)?;
    let guard = IdentityGuard::pin(
        state,
        snapshot,
        process.observation().expected(),
        tree_view(tree, &scope),
    )?;
    let mut read = LiveIdentityRead {
        guard,
        state: state.clone(),
        process,
        tree,
        scope,
    };
    read.revalidate(manager)?;
    Ok(read)
}

#[cfg(test)]
pub(crate) fn unjoined_fixture_identity<'a>(
    state: &DiagnosticState,
    process: &'a ProcessRead,
    tree: &'a TreeRead,
) -> LiveIdentityRead<'a> {
    // Invented holder ONLY for early-refusal tests, never a positive identity
    // proof. The public pin still rejects fixture trees and foreign processes.
    let (invented, snapshot) = crate::diagnostic_loaded::synthetic_loaded_fixture(0);
    LiveIdentityRead {
        guard: IdentityGuard {
            invocation: snapshot.pin_for(&invented).unwrap(),
            generation: process.observation().expected().clone(),
            root: CgroupId {
                device: 1,
                inode: 2,
            },
            refused: false,
        },
        state: state.clone(),
        process,
        tree,
        scope: DiagnosticTreeScope::new(&state.identity, SURFACES[state.completed_cells]).unwrap(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical_json;
    use crate::diagnostic_loaded::synthetic_loaded_fixture;
    use crate::diagnostic_manager::WorkerFailureKind;
    use crate::diagnostic_tree::NodeSnapshot;
    use serde_json::json;
    use std::cell::RefCell;

    fn generation(snapshot: &LoadedSnapshot) -> ExpectedGeneration {
        ExpectedGeneration {
            boot_id: "12345678-1234-1234-1234-123456789abc".into(),
            pid: 9,
            start_ticks: 42,
            cgroup_path: snapshot.manager().unit().unwrap().control_group.clone(),
        }
    }
    fn nodes(relative: &str) -> Vec<NodeSnapshot> {
        let mut nodes = vec![NodeSnapshot {
            relative_path: "".into(),
            populated: true,
            frozen: false,
            pids: vec![],
        }];
        if relative.is_empty() {
            nodes[0].pids.push(9);
        } else {
            nodes.push(NodeSnapshot {
                relative_path: relative.into(),
                populated: true,
                frozen: false,
                pids: vec![9],
            });
        }
        nodes
    }
    // Invented join inputs only: never construct a kernel TreeRead or ProcessRead.
    fn synthetic_view(nodes: &[NodeSnapshot]) -> JoinView<'_> {
        JoinView {
            scoped: true,
            kernel: true,
            root: CgroupId {
                device: 1,
                inode: 2,
            },
            nodes,
        }
    }
    fn changed(snapshot: &LoadedSnapshot, field: &str, value: serde_json::Value) -> LoadedSnapshot {
        let mut wire = serde_json::to_value(snapshot).unwrap();
        wire["manager"]["unit"][field] = value;
        LoadedSnapshot::decode(&canonical_json(&wire).unwrap(), snapshot.manager().scope()).unwrap()
    }
    fn worker_failure() -> WorkerFailure {
        WorkerFailure {
            kind: WorkerFailureKind::Deadline,
            stdout: vec![1],
            stderr: vec![2],
            cleanup_confirmed: false,
        }
    }
    #[test]
    fn synthetic_four_cells_bind_exact_root_and_descendant_membership() {
        for index in 0..4 {
            let (state, snapshot) = synthetic_loaded_fixture(index);
            for relative in ["", "child", "child.grand/grand"] {
                let nodes = nodes(relative);
                let mut generation = generation(&snapshot);
                if !relative.is_empty() {
                    generation.cgroup_path.push_str(&format!("/{relative}"));
                }
                let mut guard =
                    IdentityGuard::pin(&state, &snapshot, &generation, synthetic_view(&nodes))
                        .unwrap();
                guard
                    .revalidate_with(
                        |invocation| {
                            invocation.revalidate(&snapshot).unwrap();
                            Ok(snapshot.clone())
                        },
                        || Ok(()),
                    )
                    .unwrap();
                assert!(!guard.is_refused());
            }
        }
    }
    #[test]
    fn synthetic_same_invocation_pid_and_status_drift_is_sticky() {
        let (state, good) = synthetic_loaded_fixture(0);
        for (field, value) in [
            ("main_pid", json!(10)),
            ("main_pid", json!(0)),
            ("active_state", json!("inactive")),
            ("sub_state", json!("exited")),
            ("result", json!("timeout")),
        ] {
            let bad = changed(&good, field, value);
            // Existing invocation-only policy intentionally permits these changes.
            good.pin_for(&state).unwrap().revalidate(&bad).unwrap();
            let mut guard = IdentityGuard::pin(
                &state,
                &good,
                &generation(&good),
                synthetic_view(&nodes("")),
            )
            .unwrap();
            assert!(guard
                .revalidate_with(|_| Ok(bad.clone()), || Ok(()))
                .is_err());
            assert!(guard.is_refused());
            assert!(guard
                .revalidate_with(
                    |_| panic!("sticky guard read manager"),
                    || panic!("sticky guard read sources")
                )
                .is_err());
            assert!(IdentityGuard::pin(
                &state,
                &bad,
                &generation(&good),
                synthetic_view(&nodes(""))
            )
            .is_err());
        }
    }
    #[test]
    fn synthetic_scope_generation_origin_and_root_mismatch_refuse() {
        let (state, good) = synthetic_loaded_fixture(1);
        let expected = generation(&good);
        let nodes = nodes("");
        for case in 0..8 {
            let mut generation = expected.clone();
            let mut view = synthetic_view(&nodes);
            match case {
                0 => generation.boot_id = "ffffffff-ffff-ffff-ffff-ffffffffffff".into(),
                1 => generation.pid = 10,
                2 => generation.start_ticks = 0,
                3 => generation.cgroup_path.push_str("-foreign"),
                4 => view.kernel = false,
                5 => view.scoped = false,
                6 => view.root.device = 0,
                _ => view.root.inode = 0,
            }
            assert!(
                IdentityGuard::pin(&state, &good, &generation, view).is_err(),
                "case {case}"
            );
        }
        let (foreign, _) = synthetic_loaded_fixture(2);
        assert!(IdentityGuard::pin(&foreign, &good, &expected, synthetic_view(&nodes)).is_err());
    }
    #[test]
    fn synthetic_pid_in_wrong_node_missing_duplicate_or_frozen_refuses() {
        let (state, good) = synthetic_loaded_fixture(0);
        for case in 0..5 {
            let mut nodes = nodes("child");
            let mut generation = generation(&good);
            generation.cgroup_path.push_str("/child");
            match case {
                0 => nodes[1].relative_path = "other".into(),
                1 => nodes[1].pids.clear(),
                2 => nodes[0].pids.push(9),
                3 => nodes[0].frozen = true,
                _ => nodes[1].populated = false,
            }
            assert!(
                IdentityGuard::pin(&state, &good, &generation, synthetic_view(&nodes)).is_err()
            );
        }
    }
    #[test]
    fn synthetic_manager_checks_bracket_both_original_source_checks() {
        let (state, good) = synthetic_loaded_fixture(3);
        let mut guard = IdentityGuard::pin(
            &state,
            &good,
            &generation(&good),
            synthetic_view(&nodes("")),
        )
        .unwrap();
        let order = RefCell::new(vec![]);
        guard
            .revalidate_with(
                |invocation| {
                    order.borrow_mut().push("manager");
                    invocation.revalidate(&good).unwrap();
                    Ok(good.clone())
                },
                || {
                    order.borrow_mut().push("sources");
                    Ok(())
                },
            )
            .unwrap();
        assert_eq!(
            *order.borrow(),
            ["manager", "sources", "sources", "manager"]
        );
    }
    #[test]
    fn synthetic_both_manager_failures_retain_details_and_latch() {
        let (state, good) = synthetic_loaded_fixture(0);
        for fail_at in [1, 2] {
            let mut guard = IdentityGuard::pin(
                &state,
                &good,
                &generation(&good),
                synthetic_view(&nodes("")),
            )
            .unwrap();
            let mut calls = 0;
            let error = guard
                .revalidate_with(
                    |_| {
                        calls += 1;
                        if calls == fail_at {
                            Err(worker_failure())
                        } else {
                            Ok(good.clone())
                        }
                    },
                    || Ok(()),
                )
                .unwrap_err();
            let IdentityError::Manager(failure) = error else {
                panic!("lost manager failure")
            };
            assert_eq!(failure.kind, WorkerFailureKind::Deadline);
            assert_eq!(failure.stdout, [1]);
            assert_eq!(failure.stderr, [2]);
            assert!(!failure.cleanup_confirmed);
            assert!(guard.is_refused());
            assert!(guard
                .revalidate_with(|_| Ok(good.clone()), || Ok(()))
                .is_err());
        }
    }
    #[test]
    fn synthetic_process_and_tree_failures_at_both_positions_latch() {
        let (state, good) = synthetic_loaded_fixture(0);
        for fail_at in [1, 2] {
            for tree in [false, true] {
                let mut guard = IdentityGuard::pin(
                    &state,
                    &good,
                    &generation(&good),
                    synthetic_view(&nodes("")),
                )
                .unwrap();
                let mut calls = 0;
                assert!(guard
                    .revalidate_with(
                        |_| Ok(good.clone()),
                        || {
                            calls += 1;
                            if calls != fail_at {
                                Ok(())
                            } else if tree {
                                Err(TreeError::Invalid.into())
                            } else {
                                Err(ProcessError::NotLive.into())
                            }
                        }
                    )
                    .is_err());
                assert!(guard.is_refused());
                assert!(guard
                    .revalidate_with(|_| Ok(good.clone()), || Ok(()))
                    .is_err());
            }
        }
    }
    #[test]
    fn synthetic_second_pid_or_invocation_change_cannot_be_adopted() {
        let (state, good) = synthetic_loaded_fixture(0);
        for (field, value) in [
            ("main_pid", json!(10)),
            ("invocation_id", json!("cd".repeat(16))),
        ] {
            let bad = changed(&good, field, value);
            let mut guard = IdentityGuard::pin(
                &state,
                &good,
                &generation(&good),
                synthetic_view(&nodes("")),
            )
            .unwrap();
            let mut calls = 0;
            assert!(guard
                .revalidate_with(
                    |_| {
                        calls += 1;
                        Ok(if calls == 1 {
                            good.clone()
                        } else {
                            bad.clone()
                        })
                    },
                    || Ok(())
                )
                .is_err());
            assert!(guard.is_refused());
            assert!(guard
                .revalidate_with(|_| Ok(good.clone()), || Ok(()))
                .is_err());
        }
    }
    #[test]
    fn synthetic_independent_guards_do_not_share_refusal_state() {
        std::thread::scope(|scope| {
            for index in 0..4 {
                scope.spawn(move || {
                    let (state, good) = synthetic_loaded_fixture(index);
                    let mut guard = IdentityGuard::pin(
                        &state,
                        &good,
                        &generation(&good),
                        synthetic_view(&nodes("")),
                    )
                    .unwrap();
                    for _ in 0..32 {
                        guard
                            .revalidate_with(|_| Ok(good.clone()), || Ok(()))
                            .unwrap();
                    }
                    assert!(!guard.is_refused());
                });
            }
        });
    }
    #[test]
    fn actual_owned_helper_and_fixture_tree_cannot_be_promoted() {
        use crate::diagnostic_process::{pin_kernel_process, pin_owned_test_helper};
        use crate::diagnostic_tree::read_fixture_tree;
        use std::os::unix::fs::PermissionsExt;
        use std::process::{Child, Command, Stdio};
        struct OwnedHelper(Child);
        impl Drop for OwnedHelper {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
        let child = OwnedHelper(
            Command::new("/bin/cat")
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let process = pin_owned_test_helper(child.0.id());
        process.revalidate().unwrap();
        let (mut state, original) = synthetic_loaded_fixture(0);
        state.identity.boot_id = process.observation().expected().boot_id.clone();
        let mut wire = serde_json::to_value(&original).unwrap();
        wire["manager"]["scope"]["boot_id"] = json!(state.identity.boot_id);
        wire["manager"]["unit"]["main_pid"] = json!(child.0.id());
        let scope = crate::diagnostic_manager::ManagerScope::new(
            &state.identity.lease_id,
            &state.identity.boot_id,
            "embedded",
        )
        .unwrap();
        let snapshot = LoadedSnapshot::decode(&canonical_json(&wire).unwrap(), &scope).unwrap();
        let directory = tempfile::tempdir().unwrap();
        std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        for (name, bytes) in [
            ("cgroup.type", "domain\n".to_owned()),
            ("cgroup.events", "populated 1\nfrozen 0\n".to_owned()),
            ("cgroup.procs", format!("{}\n", child.0.id())),
        ] {
            let path = directory.path().join(name);
            std::fs::write(&path, bytes).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        // SAFETY: these getters take no pointers and observe current credentials.
        let (uid, gid) = unsafe { (libc::geteuid(), libc::getegid()) };
        let tree = read_fixture_tree(directory.path(), uid, gid).unwrap();
        let fixed_scope = DiagnosticTreeScope::new(&state.identity, "embedded").unwrap();
        assert!(!tree.is_kernel_scope(&fixed_scope));
        assert!(!tree.snapshot().kernel_origin());
        assert!(
            pin_kernel_process(&fixed_scope, process.observation().expected().clone()).is_err()
        );
        assert!(matches!(
            pin_live_identity(
                &state,
                &snapshot,
                &process,
                &tree,
                &mut ManagerClient::default()
            ),
            Err(IdentityError::Invalid)
        ));
        process.revalidate().unwrap(); // Refusal did not terminate or replace it.
    }
}
