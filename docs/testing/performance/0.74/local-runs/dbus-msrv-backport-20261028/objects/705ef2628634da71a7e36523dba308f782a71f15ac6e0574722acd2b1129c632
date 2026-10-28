//! The wait for a child to exit, which leaves the child unreaped.
//!
//! Each platform has an `Exit` of its own, made as the child is spawned, whose `wait` resolves once
//! the process has exited and leaves it for std's `Child` to collect, which only `Child::status`
//! and `Child::try_status` do while the `Child` is held, and the reaping of a `Child` that is
//! dropped while its process runs does after that. Until then the process stays a zombie that
//! holds on to its process ID, so that a `kill` of the child cannot reach another process that has
//! been given the ID since.
//!
//! Where the runtime can watch for the exit, it does, and the wait holds no thread: on Linux
//! through a pidfd of the process, and on Apple's platforms and the BSDs through a kqueue of the
//! child's own with a filter on the process, each a descriptor that turns readable once the process
//! has exited. Where it cannot, which is on Android, on Windows, on the other unix systems, and on
//! a Linux that has no pidfd to give, the wait runs as blocking work on a thread of the pool kept
//! for the waits for children, apart from that of [`unblock()`](crate::unblock()).

// A kqueue of the child's own on Apple's platforms and the BSDs, which have no wait on the pool to
// fall back on, and the pool elsewhere, with a pidfd in front of it on Linux.
#[cfg(any(
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
mod kqueue;
#[cfg(any(
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
pub(super) use kqueue::Exit;

#[cfg(not(any(
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
)))]
mod pool;
#[cfg(not(any(
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
)))]
pub(super) use pool::Exit;
