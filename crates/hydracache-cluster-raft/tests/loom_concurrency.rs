#![cfg(hydracache_loom)]

use hydracache::{CasResult, ClusterEpoch, ConsistencyLevel, SingleKeyConditionalStore};
use loom::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use loom::sync::{Arc, Mutex};
use loom::thread;

#[test]
fn loom_single_key_conditional_store_is_mutually_exclusive_under_all_interleavings() {
    loom::model(|| {
        let store = Arc::new(Mutex::new(SingleKeyConditionalStore::new(
            ClusterEpoch::new(64),
            16,
        )));
        let winners = Arc::new(AtomicUsize::new(0));

        let left = spawn_put_if_absent(Arc::clone(&store), Arc::clone(&winners), b"left".to_vec());
        let right =
            spawn_put_if_absent(Arc::clone(&store), Arc::clone(&winners), b"right".to_vec());

        left.join().unwrap();
        right.join().unwrap();

        assert_eq!(
            winners.load(Ordering::SeqCst),
            1,
            "put-if-absent must have exactly one winner under all interleavings"
        );
        let value = store.lock().unwrap().current_value("lock").unwrap();
        assert!(
            value == b"left" || value == b"right",
            "winner value must be one of the racing writers"
        );
    });
}

#[test]
fn loom_invalidation_ring_never_loses_or_duplicates_a_fence() {
    loom::model(|| {
        let ring = Arc::new(LoomInvalidationRing::new(2));
        let first = {
            let ring = Arc::clone(&ring);
            thread::spawn(move || ring.publish(1))
        };
        let second = {
            let ring = Arc::clone(&ring);
            thread::spawn(move || ring.publish(2))
        };

        first.join().unwrap();
        second.join().unwrap();

        let mut fences = ring.drain();
        fences.sort_unstable();
        assert_eq!(
            fences,
            vec![1, 2],
            "ring must retain every published fence exactly once"
        );
    });
}

#[test]
fn canary_loom_conditional_store_with_a_relaxed_ordering_races() {
    let caught = std::panic::catch_unwind(|| {
        loom::model(|| {
            let acquired = Arc::new(AtomicBool::new(false));
            let winners = Arc::new(AtomicUsize::new(0));
            let left = spawn_relaxed_acquire(Arc::clone(&acquired), Arc::clone(&winners));
            let right = spawn_relaxed_acquire(Arc::clone(&acquired), Arc::clone(&winners));

            left.join().unwrap();
            right.join().unwrap();

            assert!(
                winners.load(Ordering::Relaxed) <= 1,
                "broken relaxed load/store acquire allowed multiple winners"
            );
        });
    })
    .is_err();

    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W26") {
        assert!(
            !caught,
            "HC-CANARY-RED:W26 relaxed lock ordering admitted multiple winners"
        );
    }

    assert!(
        caught,
        "loom canary must find the relaxed load/store mutual-exclusion violation"
    );
}

fn spawn_put_if_absent(
    store: Arc<Mutex<SingleKeyConditionalStore>>,
    winners: Arc<AtomicUsize>,
    value: Vec<u8>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let result = store
            .lock()
            .unwrap()
            .put_if_absent("lock", value, ConsistencyLevel::Quorum)
            .unwrap();
        if matches!(result, CasResult::Applied { .. }) {
            winners.fetch_add(1, Ordering::SeqCst);
        }
    })
}

fn spawn_relaxed_acquire(
    acquired: Arc<AtomicBool>,
    winners: Arc<AtomicUsize>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        if !acquired.load(Ordering::Relaxed) {
            thread::yield_now();
            acquired.store(true, Ordering::Relaxed);
            winners.fetch_add(1, Ordering::Relaxed);
        }
    })
}

struct LoomInvalidationRing {
    next_sequence: AtomicUsize,
    slots: Vec<Mutex<Option<usize>>>,
}

impl LoomInvalidationRing {
    fn new(capacity: usize) -> Self {
        assert!(capacity > 0);
        Self {
            next_sequence: AtomicUsize::new(0),
            slots: (0..capacity).map(|_| Mutex::new(None)).collect(),
        }
    }

    fn publish(&self, fence: usize) {
        let sequence = self.next_sequence.fetch_add(1, Ordering::AcqRel);
        let slot_index = sequence % self.slots.len();
        let mut slot = self.slots[slot_index].lock().unwrap();
        assert!(
            slot.is_none(),
            "test ring capacity must not overwrite an unread fence"
        );
        *slot = Some(fence);
    }

    fn drain(&self) -> Vec<usize> {
        self.slots
            .iter()
            .filter_map(|slot| *slot.lock().unwrap())
            .collect()
    }
}
