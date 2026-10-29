//! Test-only first refusal, not a runtime journal, clock source or admission API.
use std::cell::RefCell;
use std::io::Write;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Event {
    pub(crate) phase: &'static str,
    pub(crate) site: &'static str,
    pub(crate) file: &'static str,
    pub(crate) line: u32,
    // Clock sites only: before/after boottime, before/after realtime, same boot, hold limit.
    pub(crate) clock: [u64; 6],
}
struct Frame {
    phase: &'static str,
    first: Option<Event>,
}
thread_local! {
    static FRAME: RefCell<Option<Frame>> = const { RefCell::new(None) };
}
pub(crate) fn record(site: &'static str, file: &'static str, line: u32, clock: [u64; 6]) {
    FRAME.with_borrow_mut(|frame| {
        if let Some(frame) = frame {
            if frame.first.is_none() {
                frame.first = Some(Event {
                    phase: frame.phase,
                    site,
                    file,
                    line,
                    clock,
                });
            }
        }
    });
}
pub(crate) fn phase(name: &'static str) {
    FRAME.with_borrow_mut(|frame| {
        if let Some(frame) = frame {
            frame.phase = name;
        }
    });
}
struct Restore(Option<Frame>);
impl Drop for Restore {
    fn drop(&mut self) {
        let current = FRAME.replace(self.0.take());
        if std::thread::panicking() {
            if let Some(event) = current.and_then(|f| f.first) {
                // Best effort only: a closed stderr must not replace the original panic.
                let _ = writeln!(
                    std::io::stderr().lock(),
                    "HC074-KERNEL-FIRST-REFUSAL {event:?}"
                );
            }
        }
    }
}
pub(crate) fn capture<T>(run: impl FnOnce() -> T) -> (T, Option<Event>) {
    let _validation = crate::diagnostic_ipc::validation_trace::ambient();
    let restore = Restore(FRAME.replace(Some(Frame {
        phase: "unassigned",
        first: None,
    })));
    let result = run();
    let event = FRAME.with_borrow(|frame| frame.as_ref().unwrap().first);
    drop(restore);
    (result, event)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn kernel_refusal_capture_first_event_and_phase_are_not_overwritten() {
        let (_, event) = capture(|| {
            phase("build");
            record("first", file!(), line!(), [1; 6]);
            let (_, inner) = capture(|| record("nested", file!(), line!(), [3; 6]));
            assert_eq!(inner.unwrap().clock, [3; 6]);
            phase("later");
            record("second", file!(), line!(), [2; 6]);
        });
        let event = event.unwrap();
        assert_eq!(
            (event.phase, event.site, event.clock),
            ("build", "first", [1; 6])
        );
        record("outside", file!(), line!(), [0; 6]);
        assert!(capture(|| ()).1.is_none());
    }
    #[test]
    fn kernel_refusal_capture_nested_and_unwind_restore_original_scope() {
        let (_, outer) = capture(|| {
            phase("outer");
            let (_, inner) = capture(|| {
                phase("inner");
                record("nested", file!(), line!(), [0; 6]);
            });
            assert_eq!(inner.unwrap().phase, "inner");
            let panic = std::panic::catch_unwind(|| {
                capture(|| {
                    phase("unwind");
                    record("unwinding", file!(), line!(), [0; 6]);
                    panic!("owned trace unwind canary");
                })
            });
            let payload = panic.unwrap_err();
            assert_eq!(
                payload.downcast_ref::<&str>().copied(),
                Some("owned trace unwind canary")
            );
            record("restored", file!(), line!(), [0; 6]);
        });
        assert_eq!(
            (outer.unwrap().phase, outer.unwrap().site),
            ("outer", "restored")
        );
        assert!(capture(|| ()).1.is_none());
    }
    #[test]
    fn kernel_refusal_capture_threads_are_independent() {
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
        let tasks: Vec<_> = (0..8)
            .map(|i| {
                let barrier = std::sync::Arc::clone(&barrier);
                std::thread::spawn(move || {
                    capture(|| {
                        record("own-thread", file!(), line!(), [i; 6]);
                        barrier.wait();
                    })
                    .1
                    .unwrap()
                })
            })
            .collect();
        for (i, task) in tasks.into_iter().enumerate() {
            assert_eq!(task.join().unwrap().clock, [i as u64; 6]);
        }
        assert!(capture(|| ()).1.is_none());
    }
}
