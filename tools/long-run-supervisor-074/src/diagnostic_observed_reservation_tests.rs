use super::*;
use std::os::unix::fs::MetadataExt;

#[path = "diagnostic_collector_reserved_tests.rs"]
mod collector_reserved_tests;

#[test]
fn observed_reservation_api_shape() {
    use super::super::super::super::super::fenced_preflight::request_attempt::ControllerRequestAttempt;
    fn probe(attempt: ControllerRequestAttempt<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) {
        let mut observed = attempt.into_observed_reservation();
        let _ = observed.state_to_sign();
        let _ = observed.bind_collector_requirements(b"");
    }
    let _ = probe;
}

fn boottime() -> u64 {
    let mut value: libc::timespec = unsafe { std::mem::zeroed() };
    assert_eq!(
        unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut value) },
        0
    );
    (value.tv_sec as u64) * 1_000_000_000 + value.tv_nsec as u64
}

#[test]
fn observed_reservation_freezes_actual_kernel_state_and_retains_originals() {
    with_original(false, |original, _| {
        let request = original.original.original.request.clone();
        with_owned_empty_fence_for_test(|fence, root, leaf| {
            let attempt = original
                .bind_common_empty_fence(fence)
                .unwrap()
                .retain_request()
                .unwrap();
            let retained = fs::read(leaf.join(NAME)).unwrap();
            let metadata = fs::metadata(leaf.join(NAME)).unwrap();
            let mut observed = attempt.into_observed_reservation();
            let before = boottime();
            let frozen = observed.state_to_sign().unwrap();
            let after = boottime();
            assert!((before..=after).contains(&frozen.reserved_monotonic_ns));
            assert!(frozen.reserved_monotonic_ns > 1);
            assert_eq!(frozen.identity, request.identity);
            assert_eq!(
                frozen.identity.boot_id,
                fs::read_to_string("/proc/sys/kernel/random/boot_id")
                    .unwrap()
                    .trim()
            );
            let mut expected = state(&request);
            expected.reserved_monotonic_ns = frozen.reserved_monotonic_ns;
            expected.last_observed_monotonic_ns = frozen.reserved_monotonic_ns;
            expected.controller_monotonic_ns = frozen.reserved_monotonic_ns;
            assert_eq!(frozen, expected);
            let mut copy = observed.state_to_sign().unwrap();
            copy.reserved_monotonic_ns = 1;
            copy.identity.lease_id = "e".repeat(64);
            assert_ne!(copy, frozen);
            assert_eq!(observed.state_to_sign().unwrap(), frozen);
            let bytes = signed(&body(&request, &frozen, &retained));
            let mut guard = observed.bind_collector_requirements(&bytes).unwrap();
            guard.revalidate().unwrap();
            assert!(!guard.is_refused());
            assert!(!format!("{guard:?}").contains("request_id"));
            busy(root);
            drop(guard);
            assert_eq!(observed.state_to_sign().unwrap(), frozen);
            assert_eq!(fs::read_dir(leaf).unwrap().count(), 1);
            assert_eq!(fs::read(leaf.join(NAME)).unwrap(), retained);
            let now = fs::metadata(leaf.join(NAME)).unwrap();
            assert_eq!(
                (
                    metadata.dev(),
                    metadata.ino(),
                    metadata.mode(),
                    metadata.len(),
                    metadata.mtime(),
                    metadata.ctime()
                ),
                (
                    now.dev(),
                    now.ino(),
                    now.mode(),
                    now.len(),
                    now.mtime(),
                    now.ctime()
                )
            );
            busy(root);
            drop(observed);
            drop(crate::host_execution::lock_host_root(root).unwrap());
            retained_context_refuses_for_test(root);
        });
    });
}

#[test]
fn observed_reservation_seeded_signed_clock_and_state_substitutions_refuse() {
    let mut cases: [u8; 6] = std::array::from_fn(|i| i as u8);
    let mut seed = 0x0740c099_u64;
    for i in (1..cases.len()).rev() {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        cases.swap(i, seed as usize % (i + 1));
    }
    for case in cases {
        with_original(true, |original, _| {
            let request = original.original.original.request.clone();
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let attempt = original
                    .bind_common_empty_fence(fence)
                    .unwrap()
                    .retain_request()
                    .unwrap();
                let retained = fs::read(leaf.join(NAME)).unwrap();
                let mut observed = attempt.into_observed_reservation();
                let frozen = observed.state_to_sign().unwrap();
                let mut changed = frozen.clone();
                match case {
                    0 => changed.reserved_monotonic_ns = 1,
                    1 => changed.last_observed_monotonic_ns = 1,
                    2 => changed.controller_monotonic_ns = 1,
                    3 => changed.identity.lease_id = "e".repeat(64),
                    4 => changed.revision = 2,
                    _ => changed.completed_cells = 1,
                }
                let mut b = body(&request, &frozen, &retained);
                b.state_sha256 = crate::sha256_hex(&line(&changed));
                assert!(matches!(
                    observed.bind_collector_requirements(&signed(&b)),
                    Err(OriginalCollectorError::Input(TrustInputError::Collector(
                        IntentProjectionError::Binding
                    )))
                ));
                assert!(observed.is_refused());
                assert!(matches!(
                    observed.state_to_sign(),
                    Err(OriginalCollectorError::Refused)
                ));
                assert!(matches!(
                    observed.bind_collector_requirements(&vec![0; 8193]),
                    Err(OriginalCollectorError::Refused)
                ));
                assert_eq!(fs::read(leaf.join(NAME)).unwrap(), retained);
                busy(root);
                drop(observed);
                retained_context_refuses_for_test(root);
            });
        });
    }
    println!(
        "observed_reservation seed=0x0740c099 signed_mutations=6 caller_state=false durable_ack=false start=false"
    );
}

#[test]
fn observed_reservation_pre_refused_owner_does_not_read_missing_attempt() {
    with_original(true, |original, _| {
        with_owned_empty_fence_for_test(|fence, root, leaf| {
            let mut attempt = original
                .bind_common_empty_fence(fence)
                .unwrap()
                .retain_request()
                .unwrap();
            attempt.simulate_binding_drift_for_test();
            assert!(attempt.revalidate().is_err());
            fs::remove_file(leaf.join(NAME)).unwrap();
            let mut observed = attempt.into_observed_reservation();
            assert!(matches!(
                observed.state_to_sign(),
                Err(OriginalCollectorError::Refused)
            ));
            assert!(matches!(
                observed.bind_collector_requirements(b""),
                Err(OriginalCollectorError::Refused)
            ));
            busy(root);
            assert_eq!(fs::read_dir(leaf).unwrap().count(), 0);
        });
    });
}

#[test]
fn observed_reservation_late_source_and_storage_drift_cannot_heal() {
    for case in 0..3 {
        with_original(true, |original, paths| {
            let request = original.original.original.request.clone();
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let attempt = original
                    .bind_common_empty_fence(fence)
                    .unwrap()
                    .retain_request()
                    .unwrap();
                let retained = fs::read(leaf.join(NAME)).unwrap();
                let mut observed = attempt.into_observed_reservation();
                let state = observed.state_to_sign().unwrap();
                let bytes = signed(&body(&request, &state, &retained));
                let mut guard = observed.bind_collector_requirements(&bytes).unwrap();
                let path = match case {
                    0 => paths.worker.clone(),
                    1 => paths.oidc.clone(),
                    _ => leaf.join("foreign"),
                };
                let saved = if case < 2 {
                    Some(fs::read(&path).unwrap())
                } else {
                    None
                };
                fs::write(&path, b"public test drift").unwrap();
                assert!(guard.revalidate().is_err());
                if let Some(raw) = saved {
                    fs::write(&path, raw).unwrap();
                } else {
                    fs::remove_file(&path).unwrap();
                }
                fs::remove_file(leaf.join(NAME)).unwrap();
                assert!(matches!(
                    guard.revalidate(),
                    Err(OriginalCollectorError::Refused)
                ));
                assert!(guard.is_refused());
                busy(root);
                drop(guard);
                assert!(matches!(
                    observed.state_to_sign(),
                    Err(OriginalCollectorError::Refused)
                ));
                assert!(matches!(
                    observed.bind_collector_requirements(&vec![0; 8193]),
                    Err(OriginalCollectorError::Refused)
                ));
                fs::write(leaf.join(NAME), retained).unwrap();
            });
        });
    }
}

#[test]
fn observed_reservation_independent_owners_keep_refusal_local() {
    let tasks: Vec<_> = (0..2)
        .map(|index| {
            std::thread::spawn(move || {
                with_original(index == 0, |original, _| {
                    let request = original.original.original.request.clone();
                    with_owned_empty_fence_for_test(|fence, root, leaf| {
                        let attempt = original
                            .bind_common_empty_fence(fence)
                            .unwrap()
                            .retain_request()
                            .unwrap();
                        let retained = fs::read(leaf.join(NAME)).unwrap();
                        let mut observed = attempt.into_observed_reservation();
                        let state = observed.state_to_sign().unwrap();
                        if index == 0 {
                            assert!(observed.bind_collector_requirements(b"invalid").is_err());
                        } else {
                            observed
                                .bind_collector_requirements(&signed(&body(
                                    &request, &state, &retained,
                                )))
                                .unwrap()
                                .revalidate()
                                .unwrap();
                        }
                        assert_eq!(observed.is_refused(), index == 0);
                        assert_eq!(fs::read(leaf.join(NAME)).unwrap(), retained);
                        busy(root);
                    });
                });
            })
        })
        .collect();
    for task in tasks {
        task.join().unwrap();
    }
}
