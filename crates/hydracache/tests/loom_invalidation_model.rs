use loom::sync::atomic::{AtomicUsize, Ordering};
use loom::sync::{Arc, Mutex};
use loom::thread;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Generation {
    global: u64,
    tag: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InFlight {
    id: u64,
    generation: Generation,
}

#[derive(Debug, Default)]
struct State {
    global_generation: u64,
    tag_generation: u64,
    value: Option<u64>,
    in_flight: Option<InFlight>,
    stale_discards: u64,
}

#[derive(Debug, Default)]
struct CacheModel {
    state: Mutex<State>,
}

impl CacheModel {
    fn snapshot(&self) -> Generation {
        let state = self.state.lock().unwrap();
        Generation {
            global: state.global_generation,
            tag: state.tag_generation,
        }
    }

    fn invalidate_tag(&self) {
        let mut state = self.state.lock().unwrap();
        state.tag_generation = state.tag_generation.wrapping_add(1);
        state.value = None;
    }

    fn flush(&self) {
        let mut state = self.state.lock().unwrap();
        state.global_generation = state.global_generation.wrapping_add(1);
        state.value = None;
    }

    fn store_if_fresh(&self, generation: Generation, value: u64) -> bool {
        let mut state = self.state.lock().unwrap();
        if state.global_generation == generation.global && state.tag_generation == generation.tag {
            state.value = Some(value);
            true
        } else {
            state.stale_discards += 1;
            false
        }
    }

    fn insert_or_get_current(&self, id: u64, generation: Generation) -> u64 {
        let mut state = self.state.lock().unwrap();
        if let Some(existing) = state.in_flight {
            if existing.generation == generation {
                return existing.id;
            }
        }

        state.in_flight = Some(InFlight { id, generation });
        id
    }

    fn remove_if_generation_matches(&self, generation: Generation) {
        let mut state = self.state.lock().unwrap();
        if state
            .in_flight
            .map(|existing| existing.generation == generation)
            .unwrap_or(false)
        {
            state.in_flight = None;
        }
    }

    fn value(&self) -> Option<u64> {
        self.state.lock().unwrap().value
    }

    fn in_flight(&self) -> Option<InFlight> {
        self.state.lock().unwrap().in_flight
    }

    fn stale_discards(&self) -> u64 {
        self.state.lock().unwrap().stale_discards
    }
}

#[test]
fn invalidation_and_store_never_leave_stale_value_cached() {
    loom::model(|| {
        let model = Arc::new(CacheModel::default());

        let load_model = model.clone();
        let load = thread::spawn(move || {
            let generation = load_model.snapshot();
            thread::yield_now();
            load_model.store_if_fresh(generation, 1);
        });

        let invalidate_model = model.clone();
        let invalidate = thread::spawn(move || {
            thread::yield_now();
            invalidate_model.invalidate_tag();
        });

        load.join().unwrap();
        invalidate.join().unwrap();

        assert_eq!(model.value(), None);
    });
}

#[test]
fn stale_load_cannot_overwrite_fresh_value_after_invalidation() {
    loom::model(|| {
        let model = Arc::new(CacheModel::default());
        let stage = Arc::new(AtomicUsize::new(0));

        let stale_model = model.clone();
        let stale_stage = stage.clone();
        let stale = thread::spawn(move || {
            let stale_generation = stale_model.snapshot();
            stale_stage.store(1, Ordering::Release);

            while stale_stage.load(Ordering::Acquire) < 2 {
                thread::yield_now();
            }

            assert!(!stale_model.store_if_fresh(stale_generation, 1));
        });

        let fresh_model = model.clone();
        let fresh_stage = stage.clone();
        let fresh = thread::spawn(move || {
            while fresh_stage.load(Ordering::Acquire) < 1 {
                thread::yield_now();
            }

            fresh_model.invalidate_tag();
            let fresh_generation = fresh_model.snapshot();
            assert!(fresh_model.store_if_fresh(fresh_generation, 2));
            fresh_stage.store(2, Ordering::Release);
        });

        stale.join().unwrap();
        fresh.join().unwrap();

        assert_eq!(model.value(), Some(2));
        assert_eq!(model.stale_discards(), 1);
    });
}

#[test]
fn post_invalidation_caller_replaces_stale_in_flight_entry() {
    loom::model(|| {
        let model = Arc::new(CacheModel::default());
        let stage = Arc::new(AtomicUsize::new(0));

        let stale_model = model.clone();
        let stale_stage = stage.clone();
        let stale = thread::spawn(move || {
            let stale_generation = stale_model.snapshot();
            assert_eq!(stale_model.insert_or_get_current(1, stale_generation), 1);
            stale_stage.store(1, Ordering::Release);

            while stale_stage.load(Ordering::Acquire) < 2 {
                thread::yield_now();
            }

            stale_model.remove_if_generation_matches(stale_generation);
        });

        let fresh_model = model.clone();
        let fresh_stage = stage.clone();
        let fresh = thread::spawn(move || {
            while fresh_stage.load(Ordering::Acquire) < 1 {
                thread::yield_now();
            }

            fresh_model.invalidate_tag();
            let fresh_generation = fresh_model.snapshot();
            assert_eq!(fresh_model.insert_or_get_current(2, fresh_generation), 2);
            fresh_stage.store(2, Ordering::Release);
        });

        stale.join().unwrap();
        fresh.join().unwrap();

        assert_eq!(model.in_flight().map(|entry| entry.id), Some(2));
    });
}

#[test]
fn flush_makes_active_load_generation_stale() {
    loom::model(|| {
        let model = Arc::new(CacheModel::default());

        let load_model = model.clone();
        let load = thread::spawn(move || {
            let generation = load_model.snapshot();
            thread::yield_now();
            load_model.store_if_fresh(generation, 1);
        });

        let flush_model = model.clone();
        let flush = thread::spawn(move || {
            thread::yield_now();
            flush_model.flush();
        });

        load.join().unwrap();
        flush.join().unwrap();

        assert_eq!(model.value(), None);
    });
}
