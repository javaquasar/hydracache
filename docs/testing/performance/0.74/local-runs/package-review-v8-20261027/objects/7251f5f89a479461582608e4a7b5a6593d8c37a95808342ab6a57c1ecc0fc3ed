//! Standalone/cache-target controls; imported gates contain no extra test cases.
#[path = "library.rs"]
mod implementation;
pub use implementation::Cache;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn same_immutable_key_loads_once_and_returned_strings_are_independent() {
        let cache = Cache::default();
        let calls = AtomicUsize::new(0);
        let mut first = cache.get("commit-a", "path-a", || {
            calls.fetch_add(1, Ordering::SeqCst);
            "exact historical bytes\r\n".into()
        });
        first.clear();
        for _ in 0..64 {
            assert_eq!(
                cache.get("commit-a", "path-a", || panic!("duplicate Git lookup")),
                "exact historical bytes\r\n"
            );
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn revisions_paths_and_cache_instances_do_not_share_entries() {
        let cache = Cache::default();
        for (revision, path, value) in [("a", "p", "one"), ("b", "p", "two"), ("a", "q", "three")] {
            assert_eq!(cache.get(revision, path, || value.into()), value);
        }
        let other_root = Cache::default();
        assert_eq!(
            other_root.get("a", "p", || "other root".into()),
            "other root"
        );
        assert_eq!(cache.get("a", "p", || unreachable!()), "one");
    }

    #[test]
    fn concurrent_same_key_runs_one_complete_loader() {
        let cache = Arc::new(Cache::default());
        let calls = Arc::new(AtomicUsize::new(0));
        let barrier = Arc::new(std::sync::Barrier::new(16));
        let threads: Vec<_> = (0..16)
            .map(|_| {
                let (cache, calls, barrier) =
                    (Arc::clone(&cache), Arc::clone(&calls), Arc::clone(&barrier));
                std::thread::spawn(move || {
                    barrier.wait();
                    cache.get("fixed", "whole-file", || {
                        calls.fetch_add(1, Ordering::SeqCst);
                        "complete blob".repeat(1024)
                    })
                })
            })
            .collect();
        for thread in threads {
            assert_eq!(thread.join().unwrap(), "complete blob".repeat(1024));
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn loader_can_read_another_key_without_holding_the_index_lock() {
        let cache = Cache::default();
        assert_eq!(
            cache.get("a", "outer", || cache.get("a", "inner", || "nested".into())),
            "nested"
        );
    }

    #[test]
    fn failed_loader_never_publishes_partial_data_or_poisons_other_keys() {
        let cache = Cache::default();
        assert!(
            std::panic::catch_unwind(|| cache.get("a", "failed", || panic!("first refusal")))
                .is_err()
        );
        assert_eq!(cache.get("a", "other", || "whole".into()), "whole");
        assert_eq!(
            cache.get("a", "failed", || "separate controlled load".into()),
            "separate controlled load"
        );
    }
}
