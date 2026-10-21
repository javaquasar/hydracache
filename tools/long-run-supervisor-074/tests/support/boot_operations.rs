//! Selected operation windows; not a guard schedule, admission or cached boot.
use super::*;
use crate::diagnostic_ipc::validation_trace::kernel_walk::{self, Kind};
use crate::diagnostic_kernel_admission::origin::boot_comparison::matches as actual_matches;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Site {
    Open,
    Filesystem,
    Read,
    Decode,
    Close,
}
trait Observer {
    fn around<T>(&mut self, site: Site, call: impl FnOnce() -> T) -> T;
}
struct Uncounted;
impl Observer for Uncounted {
    fn around<T>(&mut self, _: Site, call: impl FnOnce() -> T) -> T {
        call()
    }
}
fn child<O: Observer>(
    parent: &File,
    name: &std::ffi::CStr,
    directory: bool,
    observer: &mut O,
) -> Result<File, KernelError> {
    let flags = libc::O_RDONLY
        | libc::O_NOFOLLOW
        | libc::O_CLOEXEC
        | libc::O_NONBLOCK
        | if directory { libc::O_DIRECTORY } else { 0 };
    let fd = observer.around(Site::Open, || unsafe {
        libc::openat(parent.as_raw_fd(), name.as_ptr(), flags)
    });
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let file = unsafe { File::from_raw_fd(fd) };
    observer.around(Site::Filesystem, || filesystem(&file, 0x9fa0))?;
    Ok(file)
}
fn observed<O: Observer>(
    root: &File,
    expected: &str,
    observer: &mut O,
) -> Result<bool, KernelError> {
    let _kernel_walk = kernel_walk::span(Kind::Boot);
    let sys = child(root, c"sys", true, observer)?;
    let kernel = child(&sys, c"kernel", true, observer)?;
    let random = child(&kernel, c"random", true, observer)?;
    let file = child(&random, c"boot_id", false, observer)?;
    let raw = observer.around(Site::Read, || boot_read(|bytes| file.read_at(bytes, 0)))?;
    observer.around(Site::Close, || drop(file));
    let value = observer.around(Site::Decode, || {
        boot_view(&raw).map(|value| value == expected)
    });
    // Same reverse declaration order and original close-before-decode boundary.
    observer.around(Site::Close, || drop(random));
    observer.around(Site::Close, || drop(kernel));
    observer.around(Site::Close, || drop(sys));
    value
}
fn reading(status: i32, seconds: i64, nanos: i64) -> Option<u64> {
    if status != 0 || seconds < 0 || !(0..1_000_000_000).contains(&nanos) {
        return None;
    }
    (seconds as u64)
        .checked_mul(1_000_000_000)?
        .checked_add(nanos as u64)
}
fn cpu() -> u64 {
    let mut value = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    let status = unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut value) };
    reading(status, value.tv_sec, value.tv_nsec).expect("valid thread CPU; no repair")
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
    selected: Site,
    clock: C,
    ns: u64,
    calls: u64,
}
impl<C: Clock> Observer for Timed<C> {
    fn around<T>(&mut self, site: Site, call: impl FnOnce() -> T) -> T {
        if site != self.selected {
            return call();
        }
        // Diagnostic clocks may not replace the observed syscall's errno.
        let errno_before = unsafe { *libc::__errno_location() };
        let before = self.clock.now();
        unsafe { *libc::__errno_location() = errno_before };
        let result = call();
        let errno_after = unsafe { *libc::__errno_location() };
        let after = self.clock.now();
        unsafe { *libc::__errno_location() = errno_after };
        let elapsed = after.checked_sub(before).expect("backwards CPU");
        self.ns = self.ns.checked_add(elapsed).expect("CPU overflow");
        self.calls = self.calls.checked_add(1).expect("count overflow");
        result
    }
}
const CASES: [&str; 8] = [
    "actual",
    "traced",
    "open",
    "filesystem",
    "read",
    "decode",
    "close",
    "empty",
];
fn position(sample: usize, slot: usize) -> usize {
    assert!(sample < 16 && slot < 8);
    if sample < 8 {
        (sample + slot) % 8
    } else {
        (sample - 8 + 8 - slot) % 8
    }
}
fn run(origin: &Origin, case: usize, observer: &mut Timed<Real>) {
    match case {
        0 => assert!(actual_matches(&origin.root, &origin.boot).unwrap()),
        1 => assert!(observed(&origin.root, &origin.boot, &mut Uncounted).unwrap()),
        2..=6 => assert!(observed(&origin.root, &origin.boot, observer).unwrap()),
        7 => {
            observer.around(Site::Read, || std::hint::black_box(()));
        }
        _ => panic!("unknown operation"),
    }
}
fn selected(case: usize) -> Site {
    match case {
        0 | 1 | 4 | 7 => Site::Read,
        2 => Site::Open,
        3 => Site::Filesystem,
        5 => Site::Decode,
        6 => Site::Close,
        _ => panic!("unknown operation"),
    }
}
#[test]
fn boot_operations_values_fresh_errors_flags_and_trace_counts_match_actual() {
    struct Record(Vec<Site>);
    impl Observer for Record {
        fn around<T>(&mut self, site: Site, call: impl FnOnce() -> T) -> T {
            self.0.push(site);
            call()
        }
    }
    let origin = Origin::open().unwrap();
    let mut record = Record(Vec::new());
    let (result, report) =
        kernel_walk::capture(|| observed(&origin.root, &origin.boot, &mut record));
    assert!(result.unwrap());
    assert_eq!(report.nodes[Kind::Boot as usize].calls, 1);
    assert_eq!(report.nodes[Kind::Document as usize].calls, 1);
    assert_eq!(
        record.0,
        [
            Site::Open,
            Site::Filesystem,
            Site::Open,
            Site::Filesystem,
            Site::Open,
            Site::Filesystem,
            Site::Open,
            Site::Filesystem,
            Site::Read,
            Site::Close,
            Site::Decode,
            Site::Close,
            Site::Close,
            Site::Close
        ]
    );
    assert!(!observed(&origin.root, "different", &mut Uncounted).unwrap());
    let actual = fixed_child(&origin.root, c"sys", true).unwrap();
    let traced = child(&origin.root, c"sys", true, &mut Uncounted).unwrap();
    assert_eq!(
        actual.metadata().unwrap().ino(),
        traced.metadata().unwrap().ino()
    );
    for query in [libc::F_GETFL, libc::F_GETFD] {
        assert_eq!(unsafe { libc::fcntl(actual.as_raw_fd(), query) }, unsafe {
            libc::fcntl(traced.as_raw_fd(), query)
        });
    }
    let class = |result: Result<bool, KernelError>| match result {
        Ok(v) => (Some(v), None),
        Err(KernelError::Io(e)) => (None, e.raw_os_error()),
        Err(KernelError::Invalid) => (None, Some(0)),
        Err(e) => panic!("unexpected {e:?}"),
    };
    let directory = tempfile::tempdir().unwrap();
    for file in [
        File::open(directory.path()).unwrap(),
        tempfile::tempfile().unwrap(),
    ] {
        assert_eq!(
            class(actual_matches(&file, &origin.boot)),
            class(observed(&file, &origin.boot, &mut Uncounted))
        );
    }
    origin.revalidate().unwrap();
}
#[test]
fn boot_operations_clock_schedule_and_returned_owner_lifetime_are_checked() {
    assert_eq!(reading(0, 1, 2), Some(1_000_000_002));
    for args in [
        (-1, 0, 0),
        (0, -1, 0),
        (0, 0, -1),
        (0, 0, 1_000_000_000),
        (0, i64::MAX, 0),
    ] {
        assert_eq!(reading(args.0, args.1, args.2), None);
    }
    for start in [0, 8] {
        let mut counts = [[0; 8]; 8];
        for sample in start..start + 8 {
            for (slot, row) in counts.iter_mut().enumerate() {
                row[position(sample, slot)] += 1;
            }
        }
        assert_eq!(counts, [[1; 8]; 8]);
    }
    for args in [(16, 0), (0, 8)] {
        assert!(std::panic::catch_unwind(|| position(args.0, args.1)).is_err());
    }
    assert!(std::panic::catch_unwind(|| selected(8)).is_err());
    struct Script(std::collections::VecDeque<u64>);
    impl Clock for Script {
        fn now(&mut self) -> u64 {
            self.0.pop_front().unwrap()
        }
    }
    let mut timed = Timed {
        selected: Site::Read,
        clock: Script([1, 5].into()),
        ns: 0,
        calls: 0,
    };
    assert_eq!(timed.around(Site::Open, || 42), 42);
    let value = timed.around(Site::Read, || Err::<(), _>("original"));
    assert_eq!(value, Err("original"));
    assert_eq!((timed.calls, timed.ns), (1, 4));
    struct Owner(std::rc::Rc<std::cell::Cell<usize>>);
    impl Drop for Owner {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }
    struct ErrnoClock(u64);
    impl Clock for ErrnoClock {
        fn now(&mut self) -> u64 {
            unsafe { *libc::__errno_location() = libc::EIO };
            self.0 += 1;
            self.0
        }
    }
    let drops = std::rc::Rc::new(std::cell::Cell::new(0));
    let mut timed = Timed {
        selected: Site::Open,
        clock: ErrnoClock(0),
        ns: 0,
        calls: 0,
    };
    unsafe { *libc::__errno_location() = libc::EINTR };
    let owner = timed.around(Site::Open, || {
        assert_eq!(unsafe { *libc::__errno_location() }, libc::EINTR);
        unsafe { *libc::__errno_location() = libc::ENOENT };
        Owner(drops.clone())
    });
    assert_eq!(unsafe { *libc::__errno_location() }, libc::ENOENT);
    assert_eq!(drops.get(), 0);
    drop(owner);
    assert_eq!(drops.get(), 1);
    for (clock, ns, calls) in [([9, 8], 0, 0), ([0, 1], u64::MAX, 0), ([0, 1], 0, u64::MAX)] {
        let mut timed = Timed {
            selected: Site::Read,
            clock: Script(clock.into()),
            ns,
            calls,
        };
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || timed.around(Site::Read, || ())
        ))
        .is_err());
    }
}
#[test]
#[ignore = "manual local Boot operation CPU attribution; no authority or admission"]
fn boot_operations_selected_cpu_attribution() {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    let origin = Origin::open().unwrap();
    let pairs: Vec<_> = (0..64)
        .map(|_| {
            let before = cpu();
            cpu().checked_sub(before).unwrap()
        })
        .collect();
    println!(
        "HC074-BOOT-OPS-ORIGIN {}",
        serde_json::json!({"boot":origin.boot,"uid":unsafe{libc::geteuid()},"pairs_ns":pairs,"iterations":1024,"samples":16,"warmups":32,"server":false,"product":false,"native_gate_open":false})
    );
    for case in 0..8 {
        let mut observer = Timed {
            selected: selected(case),
            clock: Real,
            ns: 0,
            calls: 0,
        };
        for _ in 0..32 {
            run(&origin, case, &mut observer);
        }
    }
    for sample in 0..16 {
        for slot in 0..8 {
            let case = position(sample, slot);
            let mut observer = Timed {
                selected: selected(case),
                clock: Real,
                ns: 0,
                calls: 0,
            };
            let before = cpu();
            for _ in 0..1024 {
                run(&origin, std::hint::black_box(case), &mut observer);
            }
            let outer = cpu().checked_sub(before).expect("backwards batch CPU");
            let expected = match case {
                0 | 1 => 0,
                2 | 3 | 6 => 4096,
                _ => 1024,
            };
            assert_eq!(observer.calls, expected);
            assert!(observer.ns <= outer);
            println!(
                "HC074-BOOT-OPS {}",
                serde_json::json!({"sample":sample,"slot":slot,"case":CASES[case],"iterations":1024,"outer_cpu_ns":outer,"selected_cpu_ns":observer.ns,"selected_calls":observer.calls,"overhead_subtracted":false,"server":false,"product":false,"native_gate_open":false})
            );
        }
    }
    origin.revalidate().unwrap();
}
