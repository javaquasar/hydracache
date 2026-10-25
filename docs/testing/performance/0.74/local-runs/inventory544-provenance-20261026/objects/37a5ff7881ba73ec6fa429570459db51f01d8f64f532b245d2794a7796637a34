//! Finite nonproduct attribution. Selected spans include observer cost.
use super::*;
use operation_cost::{Capture, Kind, KINDS, LABELS};
use std::hint::black_box;

#[path = "diagnostic_kernel_operation_reference.rs"]
mod reference;

const CASES: [&str; 5] = ["boot", "user", "mnt", "pid", "time"];
const EXPECTED: [[u64; 7]; 2] = [[4, 4, 0, 1, 1, 0, 1], [1, 1, 1, 0, 0, 1, 1]];
const ARMS: [&str; 4] = ["reference", "reference-control", "runtime", "observed"];
const ORDERS: [[usize; 4]; 8] = [
    [0, 1, 2, 3],
    [1, 2, 3, 0],
    [2, 3, 0, 1],
    [3, 0, 1, 2],
    [0, 3, 2, 1],
    [1, 0, 3, 2],
    [2, 1, 0, 3],
    [3, 2, 1, 0],
];
fn invoke(case: usize, reference: bool, root: &File, expected: &str) -> Result<(), KernelError> {
    if case == 0 {
        let matched = if reference {
            reference::boot_matches(root, expected)?
        } else {
            boot_comparison::matches(root, expected)?
        };
        assert!(matched);
    } else {
        let named = if reference {
            reference::reference_namespace(CASES[case])?
        } else {
            namespace(CASES[case])?
        };
        black_box(named.1);
        // The returned namespace descriptor closes at the same caller boundary.
        drop(named);
    }
    Ok(())
}
fn result<T>(value: Result<T, KernelError>) -> (&'static str, Option<i32>) {
    match value {
        Ok(_) => ("ok", None),
        Err(KernelError::Io(e)) => ("io", e.raw_os_error()),
        Err(KernelError::Invalid) => ("invalid", None),
        Err(e) => panic!("unexpected {e:?}"),
    }
}
#[test]
fn operation_paths_match_original_values_refusals_and_ordered_counts() {
    let origin = Origin::open().unwrap();
    for (case, _) in CASES.iter().enumerate() {
        for (index, kind) in KINDS.into_iter().enumerate() {
            let scope = Capture::new(kind).unwrap();
            invoke(case, false, &origin.root, &origin.boot).unwrap();
            let report = scope.finish();
            assert_eq!(report.calls, EXPECTED[usize::from(case != 0)][index]);
            assert!(!report.clock_failed);
            invoke(case, true, &origin.root, &origin.boot).unwrap();
        }
    }
    for name in ["absent-hc074", "bad\0name"] {
        for kind in KINDS {
            let scope = Capture::new(kind).unwrap();
            let observed = result(namespace(name));
            let report = scope.finish();
            assert_eq!(observed, result(reference::reference_namespace(name)));
            assert!(observed.0 != "ok");
            assert_eq!(
                report.calls,
                u64::from(matches!(kind, Kind::Open | Kind::Path | Kind::Empty))
            );
            assert!(!report.clock_failed);
        }
    }
    let wrong = File::open("/dev/null").unwrap();
    for kind in KINDS {
        let scope = Capture::new(kind).unwrap();
        let observed = result(boot_comparison::matches(&wrong, &origin.boot));
        let report = scope.finish();
        assert_eq!(
            observed,
            result(reference::boot_matches(&wrong, &origin.boot))
        );
        assert_eq!(observed, ("io", Some(libc::ENOTDIR)));
        assert_eq!(
            report.calls,
            u64::from(matches!(kind, Kind::Open | Kind::Empty))
        );
        assert!(!report.clock_failed);
    }
    assert_eq!(
        result(filesystem(&wrong, 0x9fa0)),
        result(reference::reference_filesystem(&wrong, 0x9fa0))
    );
    assert_eq!(
        result(boot_comparison::matches(&origin.root, "wrong")),
        result(reference::boot_matches(&origin.root, "wrong"))
    );
}

#[test]
#[ignore = "manual finite Boot Namespace operation attribution; no preparation retry or admission"]
fn fresh_kernel_operation_cpu_screen() {
    let cohort: usize = std::env::var("HC074_KERNEL_OPERATION_COHORT")
        .unwrap()
        .parse()
        .unwrap();
    assert!(cohort < 8);
    assert_eq!(unsafe { libc::geteuid() }, 1000);
    let origin = Origin::open().unwrap();
    println!(
        "HC074-OPERATION-ORIGIN {}",
        serde_json::json!({"cohort":cohort,"uid":1000,"samples":16,
        "iterations":256,"warmups":16,"product":false,"admission":false})
    );
    for (case, name) in CASES.into_iter().enumerate() {
        for reference in [true, false] {
            for _ in 0..16 {
                invoke(case, reference, &origin.root, &origin.boot).unwrap();
            }
        }
        for (stage, kind) in KINDS.into_iter().enumerate() {
            for sample in 0..16 {
                for (slot, variant) in ORDERS[(sample + cohort) % 8].into_iter().enumerate() {
                    let scope = (variant == 3).then(|| Capture::new(kind).unwrap());
                    let before = operation_cost::cpu().unwrap();
                    let start = std::time::Instant::now();
                    for _ in 0..256 {
                        black_box(invoke(
                            black_box(case),
                            black_box(variant < 2),
                            black_box(&origin.root),
                            black_box(&origin.boot),
                        ))
                        .unwrap();
                    }
                    let wall_ns = start.elapsed().as_nanos();
                    let cpu_ns = operation_cost::cpu().unwrap().checked_sub(before).unwrap();
                    let report = scope.map(Capture::finish).unwrap_or_default();
                    let calls = if variant == 3 {
                        EXPECTED[usize::from(case != 0)][stage] * 256
                    } else {
                        0
                    };
                    assert_eq!(report.calls, calls);
                    assert!(!report.clock_failed);
                    println!(
                        "HC074-OPERATION-COST {}",
                        serde_json::json!({"cohort":cohort,"case":name,
                        "stage":LABELS[stage],"sample":sample,"slot":slot,"variant":ARMS[variant],"calls":256,
                        "operation_calls":report.calls,"operation_cpu_ns":report.cpu_ns,"clock_failed":report.clock_failed,
                        "cpu_ns":cpu_ns,"wall_ns":wall_ns,"product":false,"admission":false})
                    );
                }
            }
        }
    }
}
