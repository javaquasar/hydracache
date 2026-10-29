//! Test-only elapsed observations, never a guard clock or retained authority.
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;
#[derive(Default)]
struct Frame {
    started: Option<Instant>,
    cpu: Option<u128>,
    events: Vec<serde_json::Value>,
}
type Shared = Rc<RefCell<Frame>>;
thread_local! {static FRAME: RefCell<Option<Shared>> = const {RefCell::new(None)};}
struct Restore(Option<Shared>);
impl Drop for Restore {
    fn drop(&mut self) {
        FRAME.replace(self.0.take());
    }
}
fn cpu() -> Option<u128> {
    let mut t: libc::timespec = unsafe { std::mem::zeroed() };
    if unsafe { libc::clock_gettime(libc::CLOCK_PROCESS_CPUTIME_ID, &mut t) } != 0
        || t.tv_sec < 0
        || !(0..1_000_000_000).contains(&t.tv_nsec)
    {
        return None;
    }
    Some(t.tv_sec as u128 * 1_000_000_000 + t.tv_nsec as u128)
}
pub(crate) fn capture<T>(run: impl FnOnce() -> T) -> (T, Vec<serde_json::Value>) {
    let frame = Rc::new(RefCell::new(Frame {
        started: Some(Instant::now()),
        cpu: cpu(),
        events: Vec::new(),
    }));
    let _restore = Restore(FRAME.replace(Some(Rc::clone(&frame))));
    let result = run();
    let events = std::mem::take(&mut frame.borrow_mut().events);
    (result, events)
}
pub(crate) fn mark(name: &'static str) {
    FRAME.with_borrow(|current| {
        if let Some(frame) = current {
            let mut frame = frame.borrow_mut();
            let wall_ns = frame.started.unwrap().elapsed().as_nanos();
            let process_cpu_ns = frame.cpu.zip(cpu()).and_then(|(a, b)| b.checked_sub(a));
            frame.events.push(
                serde_json::json!({"name":name,"wall_ns":wall_ns,"process_cpu_ns":process_cpu_ns}),
            );
        }
    });
}
#[test]
fn custody_boundary_capture_is_inactive_nested_thread_local_and_unwind_safe() {
    mark("inactive");
    let marker = std::sync::Arc::new(74);
    let ((value, inner), outer) = capture(|| {
        mark("before");
        let inner = capture(|| mark("inside")).1;
        let panic = std::panic::catch_unwind(|| {
            capture(|| {
                mark("lost");
                std::panic::panic_any(std::sync::Arc::clone(&marker));
            });
        });
        assert!(std::sync::Arc::ptr_eq(
            &panic
                .unwrap_err()
                .downcast::<std::sync::Arc<i32>>()
                .unwrap(),
            &marker
        ));
        std::thread::spawn(|| mark("other-thread")).join().unwrap();
        mark("after");
        (Err::<(), _>(74), inner)
    });
    assert_eq!(value, Err(74));
    assert_eq!(inner.len(), 1);
    assert_eq!(inner[0]["name"], "inside");
    assert_eq!(outer.len(), 2);
    assert_eq!(outer[0]["name"], "before");
    assert_eq!(outer[1]["name"], "after");
    assert!(FRAME.with_borrow(|f| f.is_none()));
}
