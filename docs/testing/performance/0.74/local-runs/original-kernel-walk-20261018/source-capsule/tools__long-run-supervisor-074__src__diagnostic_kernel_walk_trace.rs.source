//! Numeric test-only nested wall spans, never an authorization clock or journal.
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

pub(crate) const COUNT: usize = 10;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    PacketContext,
    Sender,
    Alive,
    Projection,
    Origin,
    Boot,
    Namespace,
    Sample,
    WorkerOrigin,
    Document,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Stats {
    pub(crate) calls: u64,
    pub(crate) inclusive_ns: u128,
    pub(crate) outside_children_ns: u128,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Edge {
    pub(crate) calls: u64,
    pub(crate) inclusive_ns: u128,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Report {
    pub(crate) nodes: [Stats; COUNT],
    pub(crate) edges: [[Edge; COUNT]; COUNT],
    pub(crate) roots: [u64; COUNT],
}
struct Open {
    kind: Kind,
    children: u128,
}
#[derive(Default)]
struct Frame {
    report: Report,
    stack: Vec<Open>,
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
pub(crate) fn capture<T>(run: impl FnOnce() -> T) -> (T, Report) {
    let frame = Rc::new(RefCell::new(Frame::default()));
    let _restore = Capture(FRAME.replace(Some(Rc::clone(&frame))));
    let value = run();
    let frame = frame.borrow();
    assert!(frame.stack.is_empty(), "unfinished kernel numeric span");
    (value, frame.report.clone())
}
pub(crate) struct Span(Option<(Shared, Kind, usize, Instant)>);
pub(crate) fn span(kind: Kind) -> Span {
    Span(FRAME.with_borrow(|current| {
        current.as_ref().map(|frame| {
            let mut state = frame.borrow_mut();
            let depth = state.stack.len();
            state.stack.push(Open { kind, children: 0 });
            (Rc::clone(frame), kind, depth, Instant::now())
        })
    }))
}
impl Drop for Span {
    fn drop(&mut self) {
        if let Some((frame, kind, depth, started)) = self.0.take() {
            let elapsed = started.elapsed().as_nanos();
            let mut frame = frame.borrow_mut();
            assert_eq!(frame.stack.len(), depth + 1, "kernel numeric span order");
            let open = frame.stack.pop().unwrap();
            assert_eq!(open.kind, kind);
            let stats = &mut frame.report.nodes[kind as usize];
            stats.calls += 1;
            stats.inclusive_ns += elapsed;
            stats.outside_children_ns += elapsed.checked_sub(open.children).unwrap();
            let parent = frame.stack.last_mut().map(|parent| {
                parent.children += elapsed;
                parent.kind
            });
            if let Some(parent) = parent {
                let edge = &mut frame.report.edges[parent as usize][kind as usize];
                edge.calls += 1;
                edge.inclusive_ns += elapsed;
            } else {
                frame.report.roots[kind as usize] += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn balanced(report: &Report) {
        for i in 0..COUNT {
            assert_eq!(
                report.nodes[i].calls,
                report.roots[i] + report.edges.iter().map(|r| r[i].calls).sum::<u64>()
            );
            assert_eq!(
                report.nodes[i].inclusive_ns - report.nodes[i].outside_children_ns,
                report.edges[i].iter().map(|e| e.inclusive_ns).sum::<u128>()
            );
        }
    }
    #[test]
    fn kernel_numeric_empty_and_inactive_capture() {
        let inactive = span(Kind::PacketContext);
        let (_, report) = capture(|| drop(inactive));
        assert_eq!(report, Report::default());
        assert_eq!(capture(|| {}).1, Report::default());
    }
    #[test]
    fn kernel_numeric_nested_edges_are_counted_once() {
        let (_, report) = capture(|| {
            let _outer = span(Kind::PacketContext);
            for _ in 0..2 {
                let _middle = span(Kind::Sender);
                let _inner = span(Kind::Alive);
            }
        });
        balanced(&report);
        assert_eq!(
            report.nodes.map(|s| s.calls),
            [1, 2, 2, 0, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(report.edges[0][1].calls, 2);
        assert_eq!(report.edges[1][2].calls, 2);
        assert_eq!(report.roots, [1, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    }
    #[test]
    fn kernel_numeric_recursive_kinds_keep_direct_edges() {
        let (_, report) = capture(|| {
            let _outer = span(Kind::Origin);
            let _inner = span(Kind::Origin);
        });
        balanced(&report);
        assert_eq!(report.nodes[4].calls, 2);
        assert_eq!(report.roots[4], 1);
        assert_eq!(report.edges[4][4].calls, 1);
    }
    #[test]
    fn kernel_numeric_nested_capture_and_unwind_restore_original() {
        let (_, report) = capture(|| {
            let _outer = span(Kind::PacketContext);
            let (_, inner) = capture(|| {
                let _inner = span(Kind::Boot);
            });
            balanced(&inner);
            assert_eq!(inner.roots[5], 1);
            assert!(std::panic::catch_unwind(|| {
                capture(|| {
                    let _inner = span(Kind::Namespace);
                    panic!("kernel numeric restoration canary");
                });
            })
            .is_err());
            let _restored = span(Kind::Sample);
        });
        balanced(&report);
        assert_eq!(
            report.nodes.map(|s| s.calls),
            [1, 0, 0, 0, 0, 0, 0, 1, 0, 0]
        );
        assert_eq!(report.edges[0][7].calls, 1);
        assert!(FRAME.with_borrow(|f| f.is_none()));
    }
    #[test]
    fn kernel_numeric_capture_is_thread_local() {
        let (_, report) = capture(|| {
            let _outer = span(Kind::PacketContext);
            let inner = std::thread::spawn(|| {
                capture(|| {
                    let _inner = span(Kind::WorkerOrigin);
                })
                .1
            })
            .join()
            .unwrap();
            balanced(&inner);
            assert_eq!(inner.roots[8], 1);
        });
        balanced(&report);
        assert_eq!(
            report.nodes.map(|s| s.calls),
            [1, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
    }
    #[test]
    fn kernel_numeric_escaping_span_cannot_complete_capture() {
        assert!(std::panic::catch_unwind(|| capture(|| span(Kind::Document))).is_err());
        assert!(FRAME.with_borrow(|f| f.is_none()));
        assert_eq!(capture(|| {}).1, Report::default());
    }
}
