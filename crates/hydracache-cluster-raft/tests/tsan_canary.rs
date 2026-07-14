#![cfg(target_os = "linux")]

use std::cell::UnsafeCell;
use std::sync::{Arc, Barrier};

struct DeliberatelyRacy(UnsafeCell<u64>);

// This type exists only in this ignored TSan canary test target.
unsafe impl Sync for DeliberatelyRacy {}

static RACY_COUNTER: DeliberatelyRacy = DeliberatelyRacy(UnsafeCell::new(0));

#[test]
#[ignore = "must run only under the pinned ThreadSanitizer lane"]
fn canary_tsan_detects_test_fixture_data_race() {
    let barrier = Arc::new(Barrier::new(3));
    let workers = (0..2)
        .map(|worker| {
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                for value in 0..10_000 {
                    unsafe {
                        *RACY_COUNTER.0.get() = value + worker;
                    }
                }
            })
        })
        .collect::<Vec<_>>();
    barrier.wait();
    for worker in workers {
        worker.join().unwrap();
    }
}
