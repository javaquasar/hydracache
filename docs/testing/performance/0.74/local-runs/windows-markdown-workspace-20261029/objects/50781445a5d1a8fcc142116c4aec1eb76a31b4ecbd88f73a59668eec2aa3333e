//! Numeric deltas only: never a fresh-check cache, guard clock or execution right.
use super::{kernel_walk as kernel, live_walk as live};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

#[derive(Default)]
struct Totals {
    calls: u64,
    completed: u64,
    wall_ns: u128,
    kernel: kernel::Report,
    live: [live::Stats; 6],
    auth: super::Stats,
}
#[derive(Default)]
struct Frame {
    steps: [Totals; 8],
    open: usize,
}
type Shared = Rc<RefCell<Frame>>;
thread_local! {static FRAME: RefCell<Option<Shared>> = const { RefCell::new(None) };}
pub(crate) struct Capture {
    previous: Option<Shared>,
    frame: Shared,
}
impl Capture {
    pub(crate) fn new() -> Self {
        let frame = Rc::new(RefCell::new(Frame::default()));
        Self {
            previous: FRAME.replace(Some(Rc::clone(&frame))),
            frame,
        }
    }
    pub(crate) fn finish(self) -> Vec<serde_json::Value> {
        let frame = self.frame.borrow();
        assert_eq!(frame.open, 0, "unfinished round observation");
        frame.steps.iter().enumerate().map(|(step,t)| {
            let nodes:Vec<_>=t.kernel.nodes.iter().map(|s|serde_json::json!({"calls":s.calls,"inclusive_ns":s.inclusive_ns,"outside_children_ns":s.outside_children_ns})).collect();
            let edges:Vec<_>=t.kernel.edges.iter().enumerate().flat_map(|(parent,row)|row.iter().enumerate().filter(|(_,s)|s.calls!=0).map(move |(child,s)|serde_json::json!({"parent":parent,"child":child,"calls":s.calls,"inclusive_ns":s.inclusive_ns}))).collect();
            let live:Vec<_>=t.live.iter().map(|s|serde_json::json!({"calls":s.calls,"inclusive_ns":s.inclusive_ns,"outside_children_ns":s.outside_children_ns})).collect();
            serde_json::json!({"step":step,"calls":t.calls,"completed":t.completed,"wall_ns":t.wall_ns,"kernel":nodes,"edges":edges,"roots":t.kernel.roots,"live":live,"authentication_calls":t.auth.authentication_calls,"authentication_ns":t.auth.authentication_ns,"signature_calls":t.auth.signature_calls,"signature_ns":t.auth.signature_ns,"worker_policy_calls":t.auth.worker_policy_calls,"worker_policy_ns":t.auth.worker_policy_ns,"overlapping_spans":true,"guard_clock_added":false})
        }).collect()
    }
}
impl Drop for Capture {
    fn drop(&mut self) {
        FRAME.replace(self.previous.take());
    }
}
struct Before {
    frame: Shared,
    step: usize,
    kernel: kernel::Report,
    live: [live::Stats; 6],
    auth: super::Report,
    started: Instant,
}
pub(crate) struct Span(Option<Before>, bool);
pub(crate) fn span(step: usize) -> Span {
    Span(
        FRAME.with_borrow(|frame| {
            frame.as_ref().map(|frame| {
                assert!(step < 8);
                frame.borrow_mut().open += 1;
                Before {
                    frame: Rc::clone(frame),
                    step,
                    kernel: kernel::numeric_snapshot().unwrap_or_default(),
                    live: live::numeric_snapshot().unwrap_or_default(),
                    auth: super::numeric_snapshot().unwrap_or_default(),
                    started: Instant::now(),
                }
            })
        }),
        false,
    )
}
impl Span {
    pub(crate) fn complete(mut self) {
        self.1 = true;
    }
}
impl Drop for Span {
    fn drop(&mut self) {
        let Some(before) = self.0.take() else { return };
        let elapsed = before.started.elapsed().as_nanos();
        let after_kernel = kernel::numeric_snapshot().unwrap_or_default();
        let after_live = live::numeric_snapshot().unwrap_or_default();
        let after_auth = super::numeric_snapshot().unwrap_or_default();
        let mut frame = before.frame.borrow_mut();
        frame.open -= 1;
        let total = &mut frame.steps[before.step];
        total.calls += 1;
        total.completed += u64::from(self.1);
        total.wall_ns += elapsed;
        for i in 0..kernel::COUNT {
            let a = after_kernel.nodes[i];
            let b = before.kernel.nodes[i];
            let t = &mut total.kernel.nodes[i];
            t.calls += a.calls - b.calls;
            t.inclusive_ns += a.inclusive_ns - b.inclusive_ns;
            t.outside_children_ns += a.outside_children_ns - b.outside_children_ns;
            total.kernel.roots[i] += after_kernel.roots[i] - before.kernel.roots[i];
            for j in 0..kernel::COUNT {
                let a = after_kernel.edges[i][j];
                let b = before.kernel.edges[i][j];
                let t = &mut total.kernel.edges[i][j];
                t.calls += a.calls - b.calls;
                t.inclusive_ns += a.inclusive_ns - b.inclusive_ns;
            }
        }
        for (i, t) in total.live.iter_mut().enumerate() {
            let a = after_live[i];
            let b = before.live[i];
            t.calls += a.calls - b.calls;
            t.inclusive_ns += a.inclusive_ns - b.inclusive_ns;
            t.outside_children_ns += a.outside_children_ns - b.outside_children_ns;
        }
        for (a, b) in after_auth.stages.iter().zip(before.auth.stages.iter()) {
            total.auth.authentication_calls += a.authentication_calls - b.authentication_calls;
            total.auth.authentication_ns += a.authentication_ns - b.authentication_ns;
            total.auth.signature_calls += a.signature_calls - b.signature_calls;
            total.auth.signature_ns += a.signature_ns - b.signature_ns;
            total.auth.worker_policy_calls += a.worker_policy_calls - b.worker_policy_calls;
            total.auth.worker_policy_ns += a.worker_policy_ns - b.worker_policy_ns;
        }
    }
}

#[test]
fn round_cost_inactive_and_nested_captures_do_not_leak() {
    let inactive = span(99);
    let outer = Capture::new();
    drop(inactive);
    {
        let child = Capture::new();
        span(2).complete();
        assert_eq!(child.finish()[2]["completed"], 1);
    }
    std::thread::spawn(|| span(99).complete()).join().unwrap();
    let rows = outer.finish();
    assert!(rows.iter().all(|r| r["calls"] == 0));
    assert!(FRAME.with_borrow(|f| f.is_none()));
}
#[test]
fn round_cost_deltas_preserve_parent_counts_errors_and_panic_identity() {
    let marker = std::sync::Arc::new(74);
    let capture = Capture::new();
    let (((value, auth), live), kernel) = kernel::capture(|| {
        live::capture(|| {
            super::capture(|| {
                {
                    let _scope = span(0);
                    let _kernel = kernel::span(kernel::Kind::Sender);
                    let _live = live::span(live::Kind::KernelWorker);
                    let _auth = super::authentication();
                }
                let error = std::panic::catch_unwind(|| {
                    let _scope = span(1);
                    let _kernel = kernel::span(kernel::Kind::Boot);
                    std::panic::panic_any(std::sync::Arc::clone(&marker));
                })
                .unwrap_err();
                assert!(std::sync::Arc::ptr_eq(
                    &error.downcast::<std::sync::Arc<i32>>().unwrap(),
                    &marker
                ));
                span(2).complete();
                Err::<(), _>(74)
            })
        })
    });
    assert_eq!(value, Err(74));
    assert_eq!(kernel.nodes[1].calls, 1);
    assert_eq!(kernel.nodes[5].calls, 1);
    assert_eq!(live[0].calls, 1);
    assert_eq!(auth.stages[0].authentication_calls, 1);
    let rows = capture.finish();
    assert_eq!(rows[0]["completed"], 0);
    assert_eq!(rows[1]["completed"], 0);
    assert_eq!(rows[2]["completed"], 1);
    assert_eq!(rows[0]["kernel"][1]["calls"], 1);
    assert_eq!(rows[1]["kernel"][5]["calls"], 1);
    assert_eq!(rows[0]["authentication_calls"], 1);
}
#[test]
fn round_cost_escaping_span_cannot_complete_capture() {
    let capture = Capture::new();
    let scope = span(0);
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| capture.finish())).is_err());
    drop(scope);
    assert!(FRAME.with_borrow(|f| f.is_none()));
}
