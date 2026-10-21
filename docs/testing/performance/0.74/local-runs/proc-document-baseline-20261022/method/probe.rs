//! Synthetic body model only: no File adapter, kernel, clock or live sender.
#![cfg(test)]

mod counter;
const DOC_BYTES: usize = 65_536;

#[derive(Debug)]
enum KernelError {
    Invalid(&'static str),
    Io(std::io::Error),
}
impl From<std::io::Error> for KernelError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}
macro_rules! kernel_invalid {
    ($site:literal) => {
        crate::KernelError::Invalid($site)
    };
}

// Only an allocation-free synthetic event receiver, not the runtime observer.
mod diagnostic_ipc {
    pub mod validation_trace {
        pub mod kernel_walk {
            use std::cell::Cell;
            thread_local! { static CALLS: Cell<usize> = const { Cell::new(0) }; }
            pub enum Kind {
                Document,
            }
            pub struct Span;
            pub fn span(_: Kind) -> Span {
                CALLS.set(CALLS.get() + 1);
                Span
            }
            pub fn calls() -> usize {
                CALLS.get()
            }
        }
    }
}
mod model;
