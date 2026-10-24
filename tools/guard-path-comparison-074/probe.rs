//! Standalone allocation-only image. Never linked into a package or supervisor.
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::hint::black_box;

#[path = "contract.rs"]
mod contract;
const PREFIX: &str = "tools/long-run-supervisor-074/src/";
const FILES: [&str; 4] = [
    "diagnostic_root_enrollment.rs",
    "diagnostic_cgroup_root.rs",
    "diagnostic_root_placement_binding.rs",
    "diagnostic_owned_placement.rs",
];
const BEFORE: &str = include_str!("fixed-slice-before.txt");

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Counts {
    allocations: usize,
    reallocations: usize,
    requested: usize,
    live: isize,
    peak: isize,
}
thread_local! {
    static STATE: Cell<(bool, Counts)> = const { Cell::new((false, Counts {
        allocations: 0, reallocations: 0, requested: 0, live: 0, peak: 0,
    })) };
}
fn account(allocated: usize, released: usize, reallocation: bool) {
    let _ = STATE.try_with(|cell| {
        let (active, mut value) = cell.get();
        if active {
            if allocated != 0 {
                value.allocations = value.allocations.wrapping_add(usize::from(!reallocation));
                value.reallocations = value.reallocations.wrapping_add(usize::from(reallocation));
                value.requested = value.requested.wrapping_add(allocated);
            }
            value.live = value
                .live
                .wrapping_add(allocated as isize)
                .wrapping_sub(released as isize);
            value.peak = value.peak.max(value.live);
            cell.set((active, value));
        }
    });
}
struct Counter;
// SAFETY: every operation forwards the original pointer/layout/size to System.
// Accounting is allocation-free, nonpanicking TLS and tolerates thread teardown.
unsafe impl GlobalAlloc for Counter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: unchanged caller layout.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            account(layout.size(), 0, false);
        }
        pointer
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: unchanged caller layout and zero-initialization contract.
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            account(layout.size(), 0, false);
        }
        pointer
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        // SAFETY: original live pointer/layout and caller's requested new size.
        let result = unsafe { System.realloc(pointer, layout, size) };
        if !result.is_null() {
            account(size, layout.size(), true);
        }
        result
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        account(0, layout.size(), false);
        // SAFETY: original live pointer and matching layout.
        unsafe { System.dealloc(pointer, layout) };
    }
}
#[global_allocator]
static ALLOCATOR: Counter = Counter;
fn capture(f: impl FnOnce()) -> Counts {
    STATE.with(|cell| {
        assert!(!cell.get().0, "nested capture");
        cell.set((true, Counts::default()));
    });
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            STATE.with(|cell| cell.set((false, cell.get().1)));
        }
    }
    let guard = Guard;
    f();
    drop(guard);
    STATE.with(|cell| cell.get().1)
}
fn baseline(path: &str, file: &&str) -> bool {
    path == format!("{PREFIX}{file}")
}
fn candidate(path: &str, file: &&str) -> bool {
    path.strip_prefix(PREFIX) == Some(*file)
}
fn corpus() -> Vec<String> {
    let mut paths: Vec<String> = FILES.iter().map(|file| format!("{PREFIX}{file}")).collect();
    paths.extend(["", PREFIX, "docs/testing/gated-test-registry.toml", "\0", "é/文件",
        "./tools/long-run-supervisor-074/src/diagnostic_owned_placement.rs",
        "tools/long-run-supervisor-074/src/diagnostic_owned_placement.rs.extra",
        "tools/long-run-supervisor-074/src/tools/long-run-supervisor-074/src/diagnostic_owned_placement.rs",
        "tools/long-run-supervisor-074/src/DIAGNOSTIC_owned_placement.rs"].map(String::from));
    paths
}
#[test]
fn source_delta_is_exact_and_keeps_all_assertions() {
    assert!(contract::SOURCE.ends_with("performance_contract_074_fixed_slice.rs"));
    let changed = contract::candidate(BEFORE);
    assert!(contract::accepts(BEFORE, &changed));
    assert_eq!(changed.replace(contract::NEW, contract::OLD), BEFORE);
    for marker in [
        "assert!",
        "assert_eq!",
        "assert_ne!",
        "namespace::normalize",
        "text.matches",
        "text.replacen",
    ] {
        assert_eq!(
            BEFORE.matches(marker).count(),
            changed.matches(marker).count()
        );
    }
    for bad in [
        BEFORE.replace(contract::OLD, ""),
        format!("{BEFORE}{}", contract::OLD),
        changed.clone(),
    ] {
        assert!(std::panic::catch_unwind(|| contract::candidate(&bad)).is_err());
    }
    assert!(!contract::accepts(BEFORE, &(changed + "\n// unrelated\n")));
}
#[test]
fn literal_paths_and_seeded_unicode_mutations_keep_exact_positions() {
    let mut paths = corpus();
    let mut seed = 0x0745_1ce5_u64;
    let alphabet = ['a', '/', '\\', '\0', 'é', '界', '.', '_'];
    for i in 0..4096 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let base = &paths[(seed as usize) % 13];
        let mut path = base.clone();
        let char_index = path
            .char_indices()
            .map(|(at, _)| at)
            .chain([path.len()])
            .collect::<Vec<_>>();
        let at = char_index[(seed as usize) % char_index.len()];
        path.insert(at, alphabet[i % alphabet.len()]);
        paths.push(path);
    }
    for path in paths {
        let expected: Vec<_> = FILES
            .iter()
            .enumerate()
            .filter_map(|(i, file)| baseline(&path, file).then_some(i))
            .collect();
        let actual: Vec<_> = FILES
            .iter()
            .enumerate()
            .filter_map(|(i, file)| candidate(&path, file).then_some(i))
            .collect();
        assert_eq!(actual, expected, "{path:?}");
    }
}
#[test]
fn allocator_restores_after_unwind_and_separates_threads() {
    assert_eq!(capture(|| ()).allocations, 0);
    assert!(std::panic::catch_unwind(|| capture(|| panic!("capture fault"))).is_err());
    assert!(!STATE.with(|cell| cell.get().0));
    let child = std::thread::spawn(|| capture(|| drop(black_box(vec![0_u8; 17]))));
    let child_counts = child.join().unwrap();
    assert_eq!(child_counts.allocations, 1);
    assert_eq!(child_counts.requested, 17);
    assert_eq!(child_counts.live, 0);
    assert_eq!(capture(|| ()).allocations, 0);
    assert!(std::panic::catch_unwind(|| capture(|| {
        capture(|| ());
    }))
    .is_err());
    assert!(!STATE.with(|cell| cell.get().0));
    let counts = capture(|| {
        let mut value = Vec::<u8>::with_capacity(8);
        value.resize(8, 1);
        value.reserve_exact(64);
        black_box(&value);
    });
    assert_eq!(counts.allocations, 1);
    assert_eq!(counts.reallocations, 1);
    assert_eq!(counts.live, 0);
}
#[test]
fn allocation_child() {
    if std::env::var_os("HC074_GUARD_PATH_ALLOCATION").is_none() {
        return;
    }
    let paths = corpus();
    assert_eq!(paths.len(), 13);
    for block in 0..6 {
        let order = if block % 2 == 0 {
            [false, true, true, false]
        } else {
            [true, false, false, true]
        };
        for (slot, selected) in order.into_iter().enumerate() {
            let mut matched = 0;
            let counts = capture(|| {
                for _ in 0..256 {
                    for path in &paths {
                        for file in &FILES {
                            let found = if selected {
                                candidate(black_box(path), black_box(file))
                            } else {
                                baseline(black_box(path), black_box(file))
                            };
                            matched += usize::from(black_box(found));
                        }
                    }
                }
            });
            assert_eq!(matched, 4 * 256);
            assert_eq!(counts.live, 0);
            if selected {
                assert_eq!(counts, Counts::default());
            } else {
                assert_eq!(counts.allocations, 13 * 4 * 256);
            }
            println!("\nHC074_GUARD_PATH {{\"block\":{block},\"slot\":{slot},\"candidate\":{selected},\"loops\":256,\"paths\":13,\"comparisons\":{},\"matched\":{matched},\"allocations\":{},\"reallocations\":{},\"requested\":{},\"live\":{},\"peak\":{},\"timing\":false}}",
                13 * 4 * 256, counts.allocations, counts.reallocations, counts.requested, counts.live, counts.peak);
        }
    }
    println!("HC074_GUARD_PATH_DONE {{\"samples\":24,\"warmup\":false,\"product\":false,\"package_replay\":false}}");
}
