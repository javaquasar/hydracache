//! Test-only selected-operation CPU observer. Never a guard or authority input.
use std::cell::RefCell;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Open,
    Filesystem,
    Identity,
    Document,
    Decode,
    Path,
    Empty,
}
#[cfg(target_arch = "x86_64")]
pub(super) const KINDS: [Kind; 7] = [
    Kind::Open,
    Kind::Filesystem,
    Kind::Identity,
    Kind::Document,
    Kind::Decode,
    Kind::Path,
    Kind::Empty,
];
#[cfg(target_arch = "x86_64")]
pub(super) const LABELS: [&str; 7] = [
    "open",
    "filesystem",
    "identity",
    "document",
    "decode",
    "path",
    "empty",
];
#[derive(Default, Debug, PartialEq, Eq)]
pub(super) struct Report {
    pub calls: u64,
    pub cpu_ns: u64,
    pub clock_failed: bool,
}
struct Frame {
    kind: Kind,
    serial: u64,
    report: Report,
}
#[derive(Default)]
struct State {
    serial: u64,
    frame: Option<Frame>,
}
thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}
pub(super) fn cpu() -> Option<u64> {
    let mut t: libc::timespec = unsafe { std::mem::zeroed() };
    // The observer must not replace errno from the operation it surrounds.
    let errno = unsafe { libc::__errno_location() };
    let saved = unsafe { *errno };
    // SAFETY: process CPU clock writes exactly the initialized timespec object.
    let status = unsafe { libc::clock_gettime(libc::CLOCK_PROCESS_CPUTIME_ID, &mut t) };
    unsafe { *errno = saved };
    if status != 0 || t.tv_sec < 0 || !(0..1_000_000_000).contains(&t.tv_nsec) {
        return None;
    }
    (t.tv_sec as u64)
        .checked_mul(1_000_000_000)?
        .checked_add(t.tv_nsec as u64)
}
pub(super) struct Capture {
    serial: u64,
    local: std::marker::PhantomData<std::rc::Rc<()>>,
}
impl Capture {
    pub fn new(kind: Kind) -> Result<Self, ()> {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            if state.frame.is_some() {
                return Err(());
            }
            let serial = state.serial.checked_add(1).ok_or(())?;
            state.serial = serial;
            state.frame = Some(Frame {
                kind,
                serial,
                report: Report::default(),
            });
            Ok(Self {
                serial,
                local: std::marker::PhantomData,
            })
        })
    }
    pub fn finish(self) -> Report {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            assert_eq!(state.frame.as_ref().map(|f| f.serial), Some(self.serial));
            state.frame.take().unwrap().report
        })
    }
}
impl Drop for Capture {
    fn drop(&mut self) {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            if state.frame.as_ref().map(|f| f.serial) == Some(self.serial) {
                state.frame = None;
            }
        });
    }
}
pub(super) struct Span {
    observation: Option<(u64, Option<u64>)>,
    local: std::marker::PhantomData<std::rc::Rc<()>>,
}
pub(super) fn span(kind: Kind) -> Span {
    let serial = STATE.with(|state| {
        state
            .borrow()
            .frame
            .as_ref()
            .filter(|frame| frame.kind == kind)
            .map(|frame| frame.serial)
    });
    Span {
        observation: serial.map(|serial| (serial, cpu())),
        local: std::marker::PhantomData,
    }
}
impl Drop for Span {
    fn drop(&mut self) {
        if let Some((serial, before)) = self.observation {
            finish_span(serial, before, cpu());
        }
    }
}
fn finish_span(serial: u64, before: Option<u64>, after: Option<u64>) {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        if let Some(frame) = &mut state.frame {
            if frame.serial != serial {
                return;
            }
            let calls = frame.report.calls.checked_add(1);
            let elapsed = before.zip(after).and_then(|(a, b)| b.checked_sub(a));
            let sum = elapsed.and_then(|n| frame.report.cpu_ns.checked_add(n));
            match (calls, sum) {
                (Some(calls), Some(sum)) => {
                    frame.report.calls = calls;
                    frame.report.cpu_ns = sum;
                }
                _ => frame.report.clock_failed = true,
            }
        }
    });
}

#[test]
fn operation_observer_selects_once_and_rejects_nested_capture_without_replacement() {
    let scope = Capture::new(Kind::Open).unwrap();
    assert!(Capture::new(Kind::Filesystem).is_err());
    drop(span(Kind::Filesystem));
    finish_span(scope.serial, Some(100), Some(175));
    assert_eq!(
        scope.finish(),
        Report {
            calls: 1,
            cpu_ns: 75,
            clock_failed: false
        }
    );
    let scope = Capture::new(Kind::Open).unwrap();
    assert_eq!(scope.finish(), Report::default());
}
#[test]
fn operation_observer_clock_fault_and_stale_spans_cannot_replace_results() {
    let scope = Capture::new(Kind::Open).unwrap();
    for pair in [(None, Some(1)), (Some(2), Some(1)), (Some(1), None)] {
        finish_span(scope.serial, pair.0, pair.1);
    }
    assert!(scope.finish().clock_failed);
    let first = Capture::new(Kind::Open).unwrap();
    let serial = first.serial;
    drop(first);
    let second = Capture::new(Kind::Open).unwrap();
    finish_span(serial, Some(1), Some(2));
    assert_eq!(second.finish(), Report::default());
}
#[test]
fn operation_observer_unwind_clears_only_its_scope_and_preserves_panic() {
    let token = std::sync::Arc::new(74u32);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _scope = Capture::new(Kind::Open).unwrap();
        let _span = span(Kind::Open);
        std::panic::panic_any(std::sync::Arc::clone(&token));
    }))
    .unwrap_err();
    assert!(std::sync::Arc::ptr_eq(
        &token,
        &result.downcast::<std::sync::Arc<u32>>().unwrap()
    ));
    assert_eq!(
        Capture::new(Kind::Open).unwrap().finish(),
        Report::default()
    );
}

#[test]
fn operation_observer_preserves_errno_and_is_thread_local() {
    let errno = unsafe { libc::__errno_location() };
    let saved = unsafe { *errno };
    unsafe { *errno = libc::EDOM };
    let scope = Capture::new(Kind::Open).unwrap();
    drop(span(Kind::Open));
    assert_eq!(unsafe { *errno }, libc::EDOM);
    assert!(!scope.finish().clock_failed);
    unsafe { *errno = saved };
    std::thread::scope(|threads| {
        for _ in 0..4 {
            threads.spawn(|| {
                let scope = Capture::new(Kind::Open).unwrap();
                finish_span(scope.serial, Some(1), Some(3));
                assert_eq!(
                    scope.finish(),
                    Report {
                        calls: 1,
                        cpu_ns: 2,
                        clock_failed: false
                    }
                );
            });
        }
    });
}
