//! Test-only numeric attribution. Never an authorization clock or execution journal.
use super::DiagnosticOperation;
use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;
use std::time::Instant;

#[cfg(target_os = "linux")]
#[path = "diagnostic_live_walk_trace.rs"]
pub(crate) mod live_walk;

#[cfg(target_os = "linux")]
#[path = "diagnostic_kernel_walk_trace.rs"]
pub(crate) mod kernel_walk;

#[cfg(target_os = "linux")]
#[path = "../tests/support/custody_boundaries.rs"]
pub(crate) mod custody_boundaries;

#[cfg(target_os = "linux")]
#[path = "../tests/support/round_cost.rs"]
pub(crate) mod round_cost;

#[cfg(target_os = "linux")]
pub(crate) fn numeric_snapshot() -> Option<Report> {
    FRAME.with_borrow(|frame| frame.as_ref().map(|frame| frame.borrow().report.clone()))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stage {
    Unassigned,
    Setup,
    Fence,
    Retain,
    Observe,
    Publish,
    Bind,
    Checks,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Stats {
    pub(crate) wall_ns: u128,
    pub(crate) authentication_calls: u64,
    pub(crate) authentication_ns: u128,
    pub(crate) signature_calls: u64,
    pub(crate) signature_ns: u128,
    pub(crate) worker_policy_calls: u64,
    pub(crate) worker_policy_ns: u128,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TimeRefusal {
    pub(crate) stage: Stage,
    pub(crate) operation: DiagnosticOperation,
    pub(crate) issued: u64,
    pub(crate) expires: u64,
    pub(crate) now: u64,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Report {
    pub(crate) stages: [Stats; 8],
    pub(crate) first_time: Option<TimeRefusal>,
    pub(crate) time_refusals: u64,
}
struct Frame {
    report: Report,
    stage: Stage,
    last: Instant,
}
impl Frame {
    fn account_wall(&mut self) {
        let now = Instant::now();
        self.report.stages[self.stage as usize].wall_ns += now.duration_since(self.last).as_nanos();
        self.last = now;
    }
}
type Shared = Rc<RefCell<Frame>>;
thread_local! {
    static FRAME: RefCell<Option<Shared>> = const { RefCell::new(None) };
}
pub(crate) struct Scope {
    original: Option<Shared>,
    frame: Shared,
}
impl Scope {
    fn new() -> Self {
        let frame = Rc::new(RefCell::new(Frame {
            report: Report::default(),
            stage: Stage::Unassigned,
            last: Instant::now(),
        }));
        let original = FRAME.replace(Some(Rc::clone(&frame)));
        Self { original, frame }
    }
    fn report(&self) -> Report {
        let mut frame = self.frame.borrow_mut();
        frame.account_wall();
        frame.report.clone()
    }
}
impl Drop for Scope {
    fn drop(&mut self) {
        FRAME.replace(self.original.take());
        if std::thread::panicking() {
            // Best effort: do not replace the original panic if diagnostics are unavailable.
            if let Ok(mut frame) = self.frame.try_borrow_mut() {
                frame.account_wall();
                let _ = writeln!(
                    std::io::stderr().lock(),
                    "HC074-VALIDATION-COST {:?}",
                    frame.report
                );
            }
        }
    }
}
pub(crate) fn ambient() -> Option<Scope> {
    FRAME.with_borrow(|frame| frame.is_none()).then(Scope::new)
}
pub(crate) fn capture<T>(run: impl FnOnce() -> T) -> (T, Report) {
    let scope = Scope::new();
    let value = run();
    let report = scope.report();
    drop(scope);
    (value, report)
}
pub(crate) fn stage(stage: Stage) {
    FRAME.with_borrow(|current| {
        if let Some(current) = current {
            let mut frame = current.borrow_mut();
            frame.account_wall();
            frame.stage = stage;
        }
    });
}
#[derive(Clone, Copy)]
enum Kind {
    Authentication,
    Signature,
    WorkerPolicy,
}
pub(crate) struct Span(Option<(Shared, Stage, Kind, Instant)>);
impl Span {
    fn new(kind: Kind) -> Self {
        Self(FRAME.with_borrow(|frame| {
            frame.as_ref().map(|frame| {
                let stage = frame.borrow().stage;
                (Rc::clone(frame), stage, kind, Instant::now())
            })
        }))
    }
}
impl Drop for Span {
    fn drop(&mut self) {
        if let Some((frame, stage, kind, started)) = self.0.take() {
            if let Ok(mut frame) = frame.try_borrow_mut() {
                let stats = &mut frame.report.stages[stage as usize];
                match kind {
                    Kind::Authentication => {
                        stats.authentication_calls = stats.authentication_calls.saturating_add(1);
                        stats.authentication_ns += started.elapsed().as_nanos();
                    }
                    Kind::Signature => {
                        stats.signature_calls = stats.signature_calls.saturating_add(1);
                        stats.signature_ns += started.elapsed().as_nanos();
                    }
                    Kind::WorkerPolicy => {
                        stats.worker_policy_calls = stats.worker_policy_calls.saturating_add(1);
                        stats.worker_policy_ns += started.elapsed().as_nanos();
                    }
                }
            }
        }
    }
}
pub(crate) fn authentication() -> Span {
    Span::new(Kind::Authentication)
}
pub(crate) fn signature() -> Span {
    Span::new(Kind::Signature)
}
pub(crate) fn worker_policy() -> Span {
    Span::new(Kind::WorkerPolicy)
}
pub(crate) fn time_refusal(operation: DiagnosticOperation, issued: u64, expires: u64, now: u64) {
    FRAME.with_borrow(|current| {
        if let Some(current) = current {
            let mut frame = current.borrow_mut();
            let stage = frame.stage;
            frame.report.time_refusals = frame.report.time_refusals.saturating_add(1);
            frame.report.first_time.get_or_insert(TimeRefusal {
                stage,
                operation,
                issued,
                expires,
                now,
            });
        }
    });
}
