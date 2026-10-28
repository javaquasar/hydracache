//! Internal logging shim.
//!
//! This forwards to [`tracing`] when the `tracing` feature is enabled, and compiles to a no-op
//! otherwise, so the rest of the crate can log unconditionally without caring whether the
//! feature is on.

#[cfg(feature = "tracing")]
pub(crate) use tracing::error;

#[cfg(not(feature = "tracing"))]
pub(crate) use noop::error;

#[cfg(not(feature = "tracing"))]
mod noop {
    // The event shim type-checks its arguments in a branch that is never taken, the way the
    // `log` crate does when a level is compiled out. Nothing is evaluated at runtime, but the
    // bindings a call site only uses in its log message do not become unused, so the
    // no-`tracing` build stays free of warnings without any `#[allow]` or `_`-prefixed names.
    macro_rules! error {
        ($fmt:literal $(, $arg:expr)* $(,)?) => {
            if false {
                let _ = ::std::format_args!($fmt $(, $arg)*);
            }
        };
    }

    pub(crate) use error;
}
