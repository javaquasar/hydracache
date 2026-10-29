//! Selected expression CPU windows in the source-checked owners fixture only.
use super::*;
use crate::diagnostic_worker_policy::authority_registry::fixture::owners_controls::{
    Fixture, Observer, Phase, Site, Uncounted,
};
const ITERATIONS: usize = 1024;
const SAMPLES: usize = 20;
const WARMUPS: usize = 32;
const CASES: [&str; 10] = [
    "actual",
    "traced",
    "held-metadata",
    "leaf-open",
    "leaf-before",
    "parent-metadata",
    "buffer",
    "read",
    "after",
    "empty",
];
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

trait Clock {
    fn now(&mut self) -> u64;
}
struct Real;
impl Clock for Real {
    fn now(&mut self) -> u64 {
        cpu()
    }
}
struct Timed<C> {
    selected: usize,
    ns: u64,
    calls: u64,
    clock: C,
}
impl<C: Clock> Timed<C> {
    fn new(selected: usize, clock: C) -> Self {
        assert!((6..13).contains(&selected) || selected == usize::MAX);
        Self {
            selected,
            ns: 0,
            calls: 0,
            clock,
        }
    }
}
impl<C: Clock> Observer for Timed<C> {
    fn around<T>(&mut self, site: Site, call: impl FnOnce() -> T) -> T {
        if site as usize != self.selected {
            return call();
        }
        let before = self.clock.now();
        let value = call();
        let elapsed = self
            .clock
            .now()
            .checked_sub(before)
            .expect("backwards selected CPU; no repair");
        self.ns = self.ns.checked_add(elapsed).expect("selected CPU overflow");
        self.calls = self.calls.checked_add(1).expect("selected count overflow");
        value
    }
}
fn position(sample: usize, slot: usize) -> usize {
    assert!(sample < SAMPLES && slot < CASES.len());
    if sample < 10 {
        (sample + slot) % 10
    } else {
        (sample - 10 + 10 - slot) % 10
    }
}
fn phase(kind: usize, traced: bool) -> Phase {
    match (kind, traced) {
        (0, false) => Phase::Records,
        (1, false) => Phase::Markers,
        (0, true) => Phase::TracedRecords,
        (1, true) => Phase::TracedMarkers,
        _ => panic!("unknown leaf kind"),
    }
}
fn selected(index: usize) -> usize {
    match index {
        0 | 1 => usize::MAX,
        2..=8 => index + 4,
        9 => Site::Buffer as usize,
        _ => panic!("unknown case"),
    }
}
fn run(fixture: &mut Fixture, depth: usize, kind: usize, index: usize, observer: &mut Timed<Real>) {
    match index {
        0 | 1 => fixture
            .run(phase(kind, index == 1), &mut Uncounted)
            .unwrap(),
        2..=8 => fixture.run(phase(kind, true), observer).unwrap(),
        9 => {
            for _ in 0..depth {
                observer.around(Site::Buffer, || std::hint::black_box(()));
            }
        }
        _ => panic!("unknown case"),
    }
}
#[test]
fn authority_leaf_cpu_selected_observer_preserves_values_errors_and_owner_lifetime() {
    struct Script {
        values: std::collections::VecDeque<u64>,
        reads: usize,
    }
    impl Clock for Script {
        fn now(&mut self) -> u64 {
            self.reads += 1;
            self.values.pop_front().unwrap()
        }
    }
    struct Owned(std::rc::Rc<std::cell::Cell<usize>>);
    impl Drop for Owned {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }
    let script = Script {
        values: [5, 15, 17, 18].into(),
        reads: 0,
    };
    let mut observer = Timed::new(Site::Buffer as usize, script);
    assert_eq!(observer.around(Site::Path, || 42), 42);
    assert_eq!(observer.clock.reads, 0);
    assert_eq!(
        observer.around(Site::Buffer, || Err::<(), _>("original")),
        Err("original")
    );
    let drops = std::rc::Rc::new(std::cell::Cell::new(0));
    let returned = observer.around(Site::Buffer, || Owned(drops.clone()));
    assert_eq!(
        (observer.calls, observer.ns, observer.clock.reads),
        (2, 11, 4)
    );
    assert_eq!(drops.get(), 0);
    drop(returned);
    assert_eq!(drops.get(), 1);
    for (values, ns, calls) in [([9, 8], 0, 0), ([0, 1], u64::MAX, 0), ([0, 1], 0, u64::MAX)] {
        let mut observer = Timed::new(
            Site::Buffer as usize,
            Script {
                values: values.into(),
                reads: 0,
            },
        );
        observer.ns = ns;
        observer.calls = calls;
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || observer.around(Site::Buffer, || ())
        ))
        .is_err());
    }
    assert!(std::panic::catch_unwind(|| Timed::new(13, Real)).is_err());
}
#[test]
fn authority_leaf_cpu_clock_and_case_schedule_refuse_invalid_inputs() {
    assert_eq!(reading(0, 2, 7), Some(2_000_000_007));
    for (status, sec, ns) in [
        (-1, 0, 0),
        (1, 0, 0),
        (0, -1, 0),
        (0, 0, -1),
        (0, 0, 1_000_000_000),
        (0, i64::MAX, 0),
    ] {
        assert_eq!(reading(status, sec, ns), None);
    }
    for start in [0, 10] {
        let mut counts = [[0; 10]; 10];
        for sample in start..start + 10 {
            let mut seen = [false; 10];
            for (slot, row) in counts.iter_mut().enumerate() {
                let index = position(sample, slot);
                assert!(!seen[index]);
                seen[index] = true;
                row[index] += 1;
            }
            assert!(seen.into_iter().all(|value| value));
        }
        assert_eq!(counts, [[1; 10]; 10]);
    }
    for (sample, slot) in [(20, 0), (0, 10)] {
        assert!(std::panic::catch_unwind(|| position(sample, slot)).is_err());
    }
    assert!(std::panic::catch_unwind(|| selected(10)).is_err());
    assert!(std::panic::catch_unwind(|| phase(2, false)).is_err());
    for (index, expected) in (0..10).zip([usize::MAX, usize::MAX, 6, 7, 8, 9, 10, 11, 12, 10]) {
        assert_eq!(selected(index), expected);
    }
}
#[test]
#[ignore = "manual local selected fresh leaf CPU; no runtime or admission"]
fn authority_leaf_cpu_selected_attribution() {
    assert_ne!(unsafe { libc::geteuid() }, 0);
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
            cpu().checked_sub(before).expect("backwards clock pair")
        })
        .collect::<Vec<_>>();
    println!(
        "HC074-LEAF-CPU-ORIGIN {}",
        serde_json::json!({
            "pid":std::process::id(),"boot_id":origin.boot,"uid":unsafe {libc::geteuid()},
            "clock":"CLOCK_THREAD_CPUTIME_ID","resolution_ns":resolution,"clock_pair_ns":pairs,
            "iterations":ITERATIONS,"samples":SAMPLES,"warmups":WARMUPS,
            "runtime_changed":false,"server":false,"product":false,"native_gate_open":false,
        })
    );
    for depth in [1, 7] {
        let mut fixture = Fixture::new(depth);
        println!("HC074-LEAF-CPU-FIXTURE {}", fixture.identity());
        for (kind, label) in ["record", "marker"].into_iter().enumerate() {
            for index in 0..10 {
                let mut observer = Timed::new(selected(index), Real);
                for _ in 0..WARMUPS {
                    run(&mut fixture, depth, kind, index, &mut observer);
                }
            }
            for sample in 0..SAMPLES {
                for slot in 0..10 {
                    let index = position(sample, slot);
                    let mut observer = Timed::new(selected(index), Real);
                    assert!(!COUNTS.with(|state| state.get().active));
                    let before = cpu();
                    for _ in 0..ITERATIONS {
                        run(
                            &mut fixture,
                            depth,
                            kind,
                            std::hint::black_box(index),
                            &mut observer,
                        );
                    }
                    let outer = cpu()
                        .checked_sub(before)
                        .expect("backwards batch CPU; no repair");
                    assert!(!COUNTS.with(|state| state.get().active));
                    assert!(outer > 0 && observer.ns <= outer);
                    let calls = if index < 2 {
                        0
                    } else {
                        (depth * ITERATIONS) as u64
                    };
                    assert_eq!(observer.calls, calls);
                    println!(
                        "HC074-LEAF-CPU {}",
                        serde_json::json!({
                            "generations":depth,"kind":label,"case":CASES[index],"sample":sample,"position":slot,
                            "iterations":ITERATIONS,"outer_ns":outer,"selected_ns":observer.ns,"calls":observer.calls,
                            "selected_clock_reads":2*observer.calls,"allocator_active":false,
                            "runtime_changed":false,"server":false,"product":false,"overhead_subtracted":false,
                        })
                    );
                }
            }
        }
        fixture.run(Phase::Full, &mut Uncounted).unwrap();
    }
    origin.revalidate().unwrap();
}
