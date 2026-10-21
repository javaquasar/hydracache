//! Source-local reference/actual pairs; Rust requests and diagnostic thread CPU.
use super::*;
use crate::diagnostic_worker_policy::authority_registry::fixture::open_name_controls::Probe;

fn nanoseconds(seconds: i64, nanos: i64) -> Option<u64> {
    if seconds < 0 || !(0..1_000_000_000).contains(&nanos) {
        return None;
    }
    (seconds as u64)
        .checked_mul(1_000_000_000)?
        .checked_add(nanos as u64)
}
fn reading(status: i32, seconds: i64, nanos: i64) -> Option<u64> {
    if status != 0 {
        return None;
    }
    nanoseconds(seconds, nanos)
}
fn cpu() -> u64 {
    let mut value = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    let status = unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut value) };
    reading(status, value.tv_sec, value.tv_nsec).expect("valid thread CPU reading; no fallback")
}
fn order(sample: usize) -> [usize; 2] {
    if sample.is_multiple_of(2) {
        [0, 1]
    } else {
        [1, 0]
    }
}
#[test]
fn authority_open_name_clock_and_pair_order_are_checked() {
    assert_eq!(nanoseconds(1, 2), Some(1_000_000_002));
    for (seconds, nanos) in [(-1, 0), (0, -1), (0, 1_000_000_000), (i64::MAX, 0)] {
        assert_eq!(nanoseconds(seconds, nanos), None);
    }
    assert_eq!(reading(-1, 0, 0), None);
    assert_eq!(reading(0, 0, 0), Some(0));
    for sample in 0..20 {
        assert_eq!(
            order(sample),
            if sample.is_multiple_of(2) {
                [0, 1]
            } else {
                [1, 0]
            }
        );
    }
}

#[test]
#[ignore = "manual local short open-name pairs and cold/warm requests; no admission"]
fn authority_open_name_allocation_cpu_attribution() {
    let origin = Origin::open().unwrap();
    let mut resolution = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    let status = unsafe { libc::clock_getres(libc::CLOCK_THREAD_CPUTIME_ID, &mut resolution) };
    let resolution = reading(status, resolution.tv_sec, resolution.tv_nsec).unwrap();
    assert!(resolution > 0);
    let pairs = (0..64)
        .map(|_| {
            let before = cpu();
            cpu()
                .checked_sub(before)
                .expect("backwards clock pair; no repair")
        })
        .collect::<Vec<_>>();
    println!(
        "HC074-OPEN-NAME-ORIGIN {}",
        serde_json::json!({
            "pid":std::process::id(),"boot_id":origin.boot,"uid":unsafe{libc::geteuid()},
            "clock":"CLOCK_THREAD_CPUTIME_ID","resolution_ns":resolution,"clock_pair_ns":pairs,
            "iterations":1024,"samples":20,"allocation_samples":3,"warmups":32,
            "server":false,"product":false,"native_gate_open":false,"overhead_subtracted":false,
        })
    );
    for case in 0..6 {
        let fixture = Probe::new(case);
        println!(
            "HC074-OPEN-NAME-FIXTURE {}",
            serde_json::json!({"case":case,"identity":fixture.identity()})
        );
        for arm in [0, 1] {
            for _ in 0..32 {
                fixture.run(arm == 0);
            }
        }
        for sample in 0..3 {
            let mut counts = [Counts::default(); 2];
            for arm in order(sample) {
                counts[arm] = capture(|| {
                    for _ in 0..1024 {
                        fixture.run(arm == 0);
                    }
                })
                .1;
            }
            println!(
                "HC074-OPEN-NAME-ALLOC {}",
                serde_json::json!({
                    "case":case,"sample":sample,"order":order(sample),"iterations":1024,
                    "reference":counts[0],"actual":counts[1],"server":false,"product":false,
                })
            );
        }
        for sample in 0..20 {
            let mut ns = [0_u64; 2];
            assert!(!COUNTS.with(|state| state.get().active));
            for arm in order(sample) {
                let before = cpu();
                for _ in 0..1024 {
                    fixture.run(arm == 0);
                }
                ns[arm] = cpu()
                    .checked_sub(before)
                    .expect("backwards batch CPU; no repair");
                assert!(ns[arm] > 0);
            }
            println!(
                "HC074-OPEN-NAME-CPU {}",
                serde_json::json!({
                    "case":case,"sample":sample,"order":order(sample),"iterations":1024,
                    "reference_ns":ns[0],"actual_ns":ns[1],"difference_ns":i128::from(ns[1])-i128::from(ns[0]),
                    "allocator_active":false,"overhead_subtracted":false,"server":false,"product":false,
                })
            );
        }
        if case < 2 {
            for cold in [true, false] {
                for _ in 0..32 {
                    fixture.leaf(cold);
                }
                for sample in 0..3 {
                    let counts = capture(|| {
                        for _ in 0..1024 {
                            fixture.leaf(cold);
                        }
                    })
                    .1;
                    println!(
                        "HC074-OPEN-NAME-LEAF {}",
                        serde_json::json!({
                            "case":case,"cold":cold,"sample":sample,"iterations":1024,"warmups":32,
                            "counts":counts,"whole_registry_initialization":false,"server":false,"product":false,
                        })
                    );
                }
            }
        }
    }
    origin.revalidate().unwrap();
}
