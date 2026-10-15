use super::*;
use crate::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::with_owned_empty_fence_for_test;
use crate::host_execution::{HostExecutionClaim, HostExecutionError};
use super::super::fenced_preflight::{ControllerFenceError, Step};
use fs2::FileExt;

#[path = "diagnostic_controller_attempt_tests.rs"]
mod attempt_tests;

struct Paths {
    oidc: PathBuf,
    worker: PathBuf,
    authority: PathBuf,
}

fn with_original(
    refused: bool,
    run: impl FnOnce(&mut OidcControllerInput<'_, '_, '_, '_, '_, '_, '_, '_>, &Paths),
) {
    with_original_operation(
        crate::diagnostic_ipc::DiagnosticOperation::Reserve,
        refused,
        run,
    );
}

fn with_original_operation(
    operation: crate::diagnostic_ipc::DiagnosticOperation,
    refused: bool,
    run: impl FnOnce(&mut OidcControllerInput<'_, '_, '_, '_, '_, '_, '_, '_>, &Paths),
) {
    with_original_operation_at(operation, refused, None, run);
}

fn with_original_operation_at(
    operation: crate::diagnostic_ipc::DiagnosticOperation,
    refused: bool,
    authority_path: Option<&Path>,
    run: impl FnOnce(&mut OidcControllerInput<'_, '_, '_, '_, '_, '_, '_, '_>, &Paths),
) {
    let model = Model::new();
    let mut r = request(&model);
    r.operation = operation;
    r.expected_state_revision =
        u64::from(operation != crate::diagnostic_ipc::DiagnosticOperation::Reserve);
    let p = policy(&r);
    let t = token(&header(&p), &serde_json::to_vec(&claims(&r, &p)).unwrap());
    let bytes = model.sign(r.clone(), &model.key);
    let (root, authority_root, mut source, mut authority, worker_bytes) =
        if let Some(path) = authority_path {
            let (root, source, authority, bytes) =
                crate::diagnostic_worker_policy::trusted_input::tests::controller_worker_fixture_at(
                    path,
                );
            (root, None, source, authority, bytes)
        } else {
            let (root, authority_root, source, authority, bytes) = controller_worker_fixture();
            (root, Some(authority_root), source, authority, bytes)
        };
    let mut packet = kernel_packet_bytes(&bytes);
    let mut worker = source
        .verify_kernel_policy(&worker_bytes, &mut packet, &mut authority)
        .unwrap();
    let mut controller = model.input();
    let mut oidc = TrustModel::new(&r);
    let paths = Paths {
        oidc: oidc.path(),
        worker: root
            .path()
            .join("etc/hydracache-perf/worker-trust-074.json"),
        authority: authority_path
            .unwrap_or_else(|| authority_root.as_ref().unwrap().path())
            .into(),
    };
    {
        let mut original = controller.authenticate_controller(&mut worker).unwrap();
        let mut build = original
            .bind_reviewed_build(BUILDER_POLICY, RECEIPT)
            .unwrap();
        let mut guard = build.bind_oidc(&mut oidc.trust, &t).unwrap();
        run(&mut guard, &paths);
        assert_eq!(guard.is_refused(), refused);
    }
    assert_eq!(oidc.trust.is_refused(), refused);
    assert_eq!(controller.is_refused(), refused);
    assert_eq!(worker.is_refused(), refused);
    drop(worker);
    assert_eq!(source.is_refused(), refused);
    assert_eq!(authority.is_failed(), refused);
    assert_eq!(packet.is_refused(), refused);
}

fn busy(root: &Path) {
    assert!(matches!(
        HostExecutionClaim::acquire(root, &"a".repeat(64)),
        Err(HostExecutionError::Busy)
    ));
}

#[test]
fn controller_fence_preflight_constructor_refuses_both_chains_and_keeps_lock() {
    for controller in [false, true] {
        with_original(true, |original, _| {
            with_owned_empty_fence_for_test(|mut fence, root, leaf| {
                if controller {
                    original.trust.refused = true;
                } else {
                    fence.latch_refusal();
                }
                let failure = original.bind_common_empty_fence(fence).unwrap_err();
                assert!(matches!(failure.error(), ControllerFenceError::Refused));
                assert!(failure.is_refused() && original.is_refused());
                busy(root);
                assert_eq!(fs::read_dir(leaf).unwrap().count(), 0);
                drop(failure);
                drop(HostExecutionClaim::acquire(root, &"a".repeat(64)).unwrap());
            });
        });
    }
}

#[test]
fn controller_fence_preflight_each_late_context_refuses_without_repair_or_io_retry() {
    for name in [
        "active-campaign",
        "active-diagnostic.json",
        ".active-diagnostic.pending",
        ".diagnostic-requests.pending",
        "diagnostic-requests-v1.json",
        "../campaign-lifecycle-smoke-v1.json",
        "../controller-loss-smoke-v1.json",
        "leaf",
    ] {
        with_original(true, |original, _| {
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let mut preflight = original.bind_common_empty_fence(fence).unwrap();
                let path = if name == "leaf" {
                    leaf.join("foreign-attempt")
                } else {
                    root.join(name)
                };
                fs::write(&path, b"owned public fault fixture").unwrap();
                assert!(matches!(
                    preflight.revalidate(),
                    Err(ControllerFenceError::Fence(_))
                ));
                assert!(preflight.is_refused());
                busy(root);
                assert_eq!(fs::read(&path).unwrap(), b"owned public fault fixture");
                fs::remove_file(&path).unwrap();
                assert!(matches!(
                    preflight.revalidate_with(|_| panic!("refused repeat performed observations")),
                    Err(ControllerFenceError::Refused)
                ));
                busy(root);
            });
        });
    }
}

#[test]
fn controller_fence_preflight_original_source_and_kernel_faults_cross_refuse() {
    for case in 0..10 {
        with_original(true, |original, paths| {
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                match case {
                    0 => original.token[10] ^= 1,
                    1 => original.token_digest = "0".repeat(64),
                    2 => fs::write(&paths.oidc, b"drift\n").unwrap(),
                    3 => {
                        let bytes = fs::read(&paths.oidc).unwrap();
                        fs::rename(&paths.oidc, paths.oidc.with_extension("old")).unwrap();
                        fs::write(&paths.oidc, bytes).unwrap();
                        fs::set_permissions(&paths.oidc, fs::Permissions::from_mode(0o600))
                            .unwrap();
                    }
                    4 => original.original.original.request.controller.run_id += 1,
                    5 => original
                        .original
                        .original
                        .worker
                        .simulate_owned_fault_for_test(0),
                    6 => fs::set_permissions(&paths.authority, fs::Permissions::from_mode(0o750))
                        .unwrap(),
                    7 => fs::write(&paths.worker, b"drift\n").unwrap(),
                    8 => original
                        .original
                        .original
                        .worker
                        .simulate_owned_fault_for_test(2),
                    _ => original
                        .original
                        .original
                        .worker
                        .simulate_owned_fault_for_test(1),
                }
                let previously_refused = original.is_refused();
                let failure = original.bind_common_empty_fence(fence).unwrap_err();
                if previously_refused {
                    assert!(matches!(failure.error(), ControllerFenceError::Refused));
                } else {
                    assert!(matches!(
                        failure.error(),
                        ControllerFenceError::Controller(_)
                    ));
                }
                assert!(failure.is_refused() && original.is_refused());
                busy(root);
                assert_eq!(fs::read_dir(leaf).unwrap().count(), 0);
                assert!(!format!("{failure:?}").contains("synthetic-test-token"));
                assert!(!format!("{failure}").contains("synthetic-test-token"));
                fs::set_permissions(&paths.authority, fs::Permissions::from_mode(0o700)).unwrap();
            });
        });
    }
}

#[test]
fn controller_fence_preflight_original_lock_and_directory_replacement_never_adopted() {
    use crate::host_execution::HOST_EXECUTION_LOCK_NAME;
    for case in 0..4 {
        with_original(true, |original, _| {
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let lock = root.join(HOST_EXECUTION_LOCK_NAME);
                let witness = OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(&lock)
                    .unwrap();
                let mut preflight = original.bind_common_empty_fence(fence).unwrap();
                match case {
                    0 => {
                        fs::rename(&lock, lock.with_extension("old")).unwrap();
                        fs::write(&lock, []).unwrap();
                        fs::set_permissions(&lock, fs::Permissions::from_mode(0o600)).unwrap();
                    }
                    1 => fs::set_permissions(&lock, fs::Permissions::from_mode(0o400)).unwrap(),
                    2 => {
                        fs::rename(root, root.with_extension("old")).unwrap();
                        fs::create_dir(root).unwrap();
                        fs::set_permissions(root, fs::Permissions::from_mode(0o700)).unwrap();
                    }
                    _ => {
                        fs::rename(leaf, leaf.with_extension("old")).unwrap();
                        fs::create_dir(leaf).unwrap();
                        fs::set_permissions(leaf, fs::Permissions::from_mode(0o700)).unwrap();
                    }
                }
                assert!(matches!(
                    preflight.revalidate(),
                    Err(ControllerFenceError::Fence(_))
                ));
                assert_eq!(
                    FileExt::try_lock_exclusive(&witness).unwrap_err().kind(),
                    std::io::ErrorKind::WouldBlock
                );
                assert!(matches!(
                    preflight.revalidate(),
                    Err(ControllerFenceError::Refused)
                ));
                drop(preflight);
                FileExt::try_lock_exclusive(&witness).unwrap();
                FileExt::unlock(&witness).unwrap();
            });
        });
    }
}

#[test]
fn controller_fence_preflight_seeded_every_bracket_position_preserves_first_error() {
    let mut seed = 0x0740_c093_u64;
    let mut order = [0, 1, 2, 3, 4];
    for i in (1..order.len()).rev() {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        order.swap(i, seed as usize % (i + 1));
    }
    for target in order {
        with_original(true, |original, paths| {
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let mut preflight = original.bind_common_empty_fence(fence).unwrap();
                let original_bytes = fs::read(&paths.oidc).unwrap();
                let marker = leaf.join("owned-fault-context");
                let mut visited = Vec::new();
                let error = preflight
                    .revalidate_with(|step| {
                        let position = visited.len();
                        visited.push(step);
                        if position == target {
                            match step {
                                Step::Fence => fs::write(&marker, b"fault").unwrap(),
                                Step::Controller => fs::write(&paths.oidc, b"fault").unwrap(),
                            }
                        }
                    })
                    .unwrap_err();
                assert_eq!(visited.len(), target + 1);
                assert_eq!(
                    &visited,
                    &[
                        Step::Fence,
                        Step::Controller,
                        Step::Fence,
                        Step::Controller,
                        Step::Fence
                    ][..=target]
                );
                match visited[target] {
                    Step::Fence => assert!(matches!(error, ControllerFenceError::Fence(_))),
                    Step::Controller => {
                        assert!(matches!(error, ControllerFenceError::Controller(_)))
                    }
                }
                fs::write(&paths.oidc, original_bytes).unwrap();
                if marker.exists() {
                    fs::remove_file(&marker).unwrap();
                }
                assert!(matches!(
                    preflight.revalidate_with(|_| panic!("refused repeat observed")),
                    Err(ControllerFenceError::Refused)
                ));
                busy(root);
            });
        });
    }
    println!("controller_fence_seed=0x0740c093 bracket_positions=5 admission=false");
}

#[test]
fn controller_fence_preflight_late_trust_and_authority_drift_retains_refused_fence() {
    for authority in [false, true] {
        with_original(true, |original, paths| {
            with_owned_empty_fence_for_test(|fence, root, _| {
                let mut preflight = original.bind_common_empty_fence(fence).unwrap();
                if authority {
                    fs::set_permissions(&paths.authority, fs::Permissions::from_mode(0o750))
                        .unwrap();
                } else {
                    fs::write(&paths.oidc, b"changed original source").unwrap();
                }
                assert!(matches!(
                    preflight.revalidate(),
                    Err(ControllerFenceError::Controller(_))
                ));
                assert!(preflight.is_refused());
                busy(root);
                assert!(matches!(
                    preflight.revalidate(),
                    Err(ControllerFenceError::Refused)
                ));
                fs::set_permissions(&paths.authority, fs::Permissions::from_mode(0o700)).unwrap();
            });
        });
    }
}

#[test]
fn controller_fence_preflight_independent_concurrent_originals_do_not_share_refusal() {
    let handles: Vec<_> = (0..4)
        .map(|i| {
            std::thread::spawn(move || {
                let fail = i % 2 == 0;
                with_original(fail, |original, _| {
                    with_owned_empty_fence_for_test(|fence, root, leaf| {
                        let mut preflight = original.bind_common_empty_fence(fence).unwrap();
                        if fail {
                            fs::write(leaf.join("own-context"), b"fault").unwrap();
                            assert!(preflight.revalidate().is_err());
                        } else {
                            preflight.revalidate().unwrap();
                        }
                        assert_eq!(preflight.is_refused(), fail);
                        busy(root);
                    });
                });
            })
        })
        .collect();
    let results: Vec<_> = handles.into_iter().map(|h| h.join()).collect();
    for result in results {
        result.unwrap();
    }
    println!("controller_fence_independent_originals=4 actual_issuer_enrolled=false");
}

#[test]
fn controller_fence_preflight_keeps_original_lock_and_healthy_drop() {
    with_original(false, |original, _| {
        with_owned_empty_fence_for_test(|fence, root, leaf| {
            let mut preflight = original.bind_common_empty_fence(fence).unwrap();
            for _ in 0..3 {
                preflight.revalidate().unwrap();
                assert!(matches!(
                    HostExecutionClaim::acquire(root, &"a".repeat(64)),
                    Err(HostExecutionError::Busy)
                ));
                assert_eq!(fs::read_dir(leaf).unwrap().count(), 0);
            }
            drop(preflight);
            original.revalidate().unwrap();
            drop(HostExecutionClaim::acquire(root, &"a".repeat(64)).unwrap());
            assert_eq!(fs::read_dir(leaf).unwrap().count(), 0);
        });
    });
}
