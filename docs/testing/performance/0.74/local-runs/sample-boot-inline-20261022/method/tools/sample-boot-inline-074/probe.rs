//! Pure sample model: synthetic validation/clock receivers, no host capability.
#![cfg(test)]
#[path = "../proc-document-baseline-074/counter.rs"]
mod counter;
use std::cell::Cell;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KernelError {
    Fault(u8),
    Invalid(&'static str),
}
macro_rules! kernel_invalid {
    ($site:literal) => {
        crate::KernelError::Invalid($site)
    };
}
// Symbolic selectors in the copied body; never linked to libc or a clock.
mod libc {
    pub const CLOCK_BOOTTIME: u8 = 7;
    pub const CLOCK_REALTIME: u8 = 0;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Event {
    Empty,
    Span,
    Before,
    BootClock,
    RealClock,
    After,
}
#[derive(Clone, Copy)]
struct Script {
    faults: u8,
    validations: u8,
    events: [Event; 5],
    used: usize,
    boot_ns: u64,
    real_ns: u64,
}
thread_local! { static SCRIPT: Cell<Script> = const { Cell::new(Script {
    faults:0, validations:0, events:[Event::Empty;5], used:0, boot_ns:0, real_ns:0,
}) }; }
fn reset(faults: u8, boot_ns: u64, real_ns: u64) {
    SCRIPT.set(Script {
        faults,
        validations: 0,
        events: [Event::Empty; 5],
        used: 0,
        boot_ns,
        real_ns,
    });
}
fn event(value: Event) {
    let mut script = SCRIPT.get();
    assert!(script.used < script.events.len());
    script.events[script.used] = value;
    script.used += 1;
    SCRIPT.set(script);
}
fn events() -> ([Event; 5], usize) {
    let script = SCRIPT.get();
    (script.events, script.used)
}
mod diagnostic_ipc {
    pub mod validation_trace {
        pub mod kernel_walk {
            pub enum Kind {
                Sample,
            }
            pub struct Span;
            pub fn span(_: Kind) -> Span {
                crate::event(crate::Event::Span);
                Span
            }
        }
    }
}
fn clock(id: u8) -> Result<u64, KernelError> {
    let (mask, value, event) = match id {
        libc::CLOCK_BOOTTIME => (2, SCRIPT.get().boot_ns, Event::BootClock),
        libc::CLOCK_REALTIME => (4, SCRIPT.get().real_ns, Event::RealClock),
        _ => panic!("unknown symbolic clock"),
    };
    self::event(event);
    if SCRIPT.get().faults & mask != 0 {
        Err(KernelError::Fault(mask))
    } else {
        Ok(value)
    }
}
mod model;
impl model::Origin {
    fn revalidate(&self) -> Result<(), KernelError> {
        let mut script = SCRIPT.get();
        script.validations += 1;
        assert!(script.validations <= 2);
        let (mask, value) = if script.validations == 1 {
            (1, Event::Before)
        } else {
            (8, Event::After)
        };
        SCRIPT.set(script);
        event(value);
        if script.faults & mask != 0 {
            Err(KernelError::Fault(mask))
        } else {
            Ok(())
        }
    }
}

mod prototype;
impl prototype::Origin {
    fn revalidate(&self) -> Result<(), KernelError> {
        let mut script = SCRIPT.get();
        script.validations += 1;
        assert!(script.validations <= 2);
        let (mask, value) = if script.validations == 1 {
            (1, Event::Before)
        } else {
            (8, Event::After)
        };
        SCRIPT.set(script);
        event(value);
        if script.faults & mask != 0 {
            Err(KernelError::Fault(mask))
        } else {
            Ok(())
        }
    }
}
