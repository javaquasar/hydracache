//! Private test-only observer at existing authority append checkpoints.
use super::PublicationStage;
use std::cell::RefCell;

type Observer = Box<dyn FnMut(PublicationStage)>;
thread_local! {
    static OBSERVER: RefCell<Option<Observer>> = const { RefCell::new(None) };
}

struct Scope;
impl Drop for Scope {
    fn drop(&mut self) {
        OBSERVER.with(|slot| {
            slot.borrow_mut().take();
        });
    }
}
pub(crate) fn with<T>(
    observer: impl FnMut(PublicationStage) + 'static,
    operation: impl FnOnce() -> T,
) -> T {
    OBSERVER.with(|slot| {
        let mut slot = slot.borrow_mut();
        assert!(slot.is_none(), "nested authority publication observer");
        *slot = Some(Box::new(observer));
    });
    let _scope = Scope;
    operation()
}
pub(crate) fn observe(stage: PublicationStage) {
    OBSERVER.with(|slot| {
        if let Some(observer) = slot.borrow_mut().as_mut() {
            observer(stage);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Barrier,
    };

    #[test]
    fn authority_checkpoint_panic_restores_empty_scope() {
        assert!(std::panic::catch_unwind(|| with(
            |_| panic!("expected observer failure"),
            || observe(PublicationStage::RecordCreated),
        ))
        .is_err());
        with(|_| {}, || observe(PublicationStage::DurableCommit));
    }

    #[test]
    fn authority_checkpoint_nested_scope_refuses_without_replacing_original() {
        let calls = Arc::new(AtomicUsize::new(0));
        let seen = calls.clone();
        with(
            move |_| {
                seen.fetch_add(1, Ordering::SeqCst);
            },
            || {
                assert!(std::panic::catch_unwind(|| with(
                    |_| panic!("nested observer ran"),
                    || {}
                ))
                .is_err());
                observe(PublicationStage::RecordSynced);
            },
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn authority_checkpoint_concurrent_threads_have_independent_scopes() {
        let barrier = Arc::new(Barrier::new(2));
        let counts: Vec<_> = (0..2).map(|_| Arc::new(AtomicUsize::new(0))).collect();
        std::thread::scope(|scope| {
            let threads: Vec<_> = counts
                .iter()
                .map(|calls| {
                    let seen = calls.clone();
                    let barrier = barrier.clone();
                    scope.spawn(move || {
                        with(
                            move |_| {
                                seen.fetch_add(1, Ordering::SeqCst);
                            },
                            || {
                                barrier.wait();
                                observe(PublicationStage::MarkerCreated);
                            },
                        )
                    })
                })
                .collect();
            for thread in threads {
                thread.join().unwrap();
            }
        });
        for calls in counts {
            assert_eq!(calls.load(Ordering::SeqCst), 1);
        }
        observe(PublicationStage::DurableCommit);
    }
}
