//! Numeric test-only wall attribution; never an admission clock or cached guard.
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

#[derive(Clone, Copy, Debug)]
pub(crate) enum Kind {
    KernelWorker,
    WorkerPolicy,
    WorkerSource,
    ActiveAuthority,
    KernelContext,
    LocalContext,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Stats {
    pub(crate) calls: u64,
    pub(crate) inclusive_ns: u128,
    // Wall time outside instrumented direct children, NOT exclusive CPU/IO time.
    pub(crate) outside_children_ns: u128,
}
#[derive(Default)]
struct Frame {
    stats: [Stats; 6],
    stack: Vec<u128>,
}
type Shared = Rc<RefCell<Frame>>;
thread_local! {
    static FRAME: RefCell<Option<Shared>> = const { RefCell::new(None) };
}
struct Capture(Option<Shared>);
impl Drop for Capture {
    fn drop(&mut self) {
        FRAME.replace(self.0.take());
    }
}
pub(crate) fn capture<T>(run: impl FnOnce() -> T) -> (T, [Stats; 6]) {
    let frame = Rc::new(RefCell::new(Frame::default()));
    let _restore = Capture(FRAME.replace(Some(Rc::clone(&frame))));
    let value = run();
    let frame = frame.borrow();
    assert!(frame.stack.is_empty(), "unfinished numeric live-walk span");
    (value, frame.stats)
}
pub(crate) fn numeric_snapshot() -> Option<[Stats; 6]> {
    FRAME.with_borrow(|frame| frame.as_ref().map(|frame| frame.borrow().stats))
}
pub(crate) struct Span(Option<(Shared, Kind, usize, Instant)>);
pub(crate) fn span(kind: Kind) -> Span {
    Span(FRAME.with_borrow(|current| {
        current.as_ref().map(|frame| {
            let mut state = frame.borrow_mut();
            let depth = state.stack.len();
            state.stack.push(0);
            (Rc::clone(frame), kind, depth, Instant::now())
        })
    }))
}
impl Drop for Span {
    fn drop(&mut self) {
        if let Some((frame, kind, depth, started)) = self.0.take() {
            let elapsed = started.elapsed().as_nanos();
            let mut frame = frame.borrow_mut();
            assert_eq!(frame.stack.len(), depth + 1, "numeric span order");
            let children = frame.stack.pop().unwrap();
            let stats = &mut frame.stats[kind as usize];
            stats.calls += 1;
            stats.inclusive_ns += elapsed;
            stats.outside_children_ns += elapsed.checked_sub(children).unwrap();
            if let Some(parent) = frame.stack.last_mut() {
                *parent += elapsed;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_and_inactive_spans_cannot_leak_into_capture() {
        let inactive = span(Kind::KernelWorker);
        let (_, stats) = capture(|| drop(inactive));
        assert_eq!(stats, [Stats::default(); 6]);
        let (_, stats) = capture(|| {});
        assert_eq!(stats, [Stats::default(); 6]);
    }
    #[test]
    fn nested_spans_count_direct_children_once() {
        let (_, stats) = capture(|| {
            let _outer = span(Kind::KernelWorker);
            for _ in 0..2 {
                let _middle = span(Kind::WorkerPolicy);
                let _inner = span(Kind::ActiveAuthority);
            }
        });
        assert_eq!(stats.map(|s| s.calls), [1, 2, 0, 2, 0, 0]);
        assert_eq!(
            stats[0].inclusive_ns - stats[0].outside_children_ns,
            stats[1].inclusive_ns
        );
        assert_eq!(
            stats[1].inclusive_ns - stats[1].outside_children_ns,
            stats[3].inclusive_ns
        );
        assert_eq!(stats[3].inclusive_ns, stats[3].outside_children_ns);
    }
    #[test]
    fn nested_capture_and_panic_restore_original_frame() {
        let (_, stats) = capture(|| {
            let _outer = span(Kind::KernelWorker);
            let (_, nested) = capture(|| {
                let _span = span(Kind::LocalContext);
            });
            assert_eq!(nested.map(|s| s.calls), [0, 0, 0, 0, 0, 1]);
            assert!(std::panic::catch_unwind(|| {
                capture(|| {
                    let _span = span(Kind::KernelContext);
                    panic!("numeric restoration canary");
                });
            })
            .is_err());
            let _restored = span(Kind::WorkerSource);
        });
        assert_eq!(stats.map(|s| s.calls), [1, 0, 1, 0, 0, 0]);
        assert!(FRAME.with_borrow(|f| f.is_none()));
    }
    #[test]
    fn thread_local_capture_does_not_observe_another_thread() {
        let (_, stats) = capture(|| {
            let _outer = span(Kind::KernelWorker);
            let nested = std::thread::spawn(|| {
                let (_, stats) = capture(|| {
                    let _span = span(Kind::WorkerPolicy);
                });
                stats
            })
            .join()
            .unwrap();
            assert_eq!(nested.map(|s| s.calls), [0, 1, 0, 0, 0, 0]);
        });
        assert_eq!(stats.map(|s| s.calls), [1, 0, 0, 0, 0, 0]);
    }
}
