//! A test of the child processes that fills the pool of threads that all blocking work shares, and
//! so is a binary of its own.
//!
//! The pool is the one `zruntime::unblock` hands work to, which `Unblock` and the pipes of a child
//! on Windows use as well. The test holds every thread of it for as long as it needs, and any other
//! test in the same process that had blocking work to run would wait for it to let them go.
//! The unit tests of the crate are one binary, whose tests run side by side, so none of them can do
//! that. A file under `tests/` is a binary of its own, and Cargo runs the binaries one after
//! another, never side by side. This one holds the single test: a second would run beside it, and
//! wait behind the pool that it fills.

#![cfg(all(feature = "process", not(target_os = "wasi")))]

use std::{
    pin::pin,
    sync::{Arc, Barrier, mpsc},
    time::Duration,
};
#[cfg(unix)]
use std::{thread, time::Instant};

use futures::future::poll_immediate;
#[cfg(unix)]
use rustix::{
    io::Errno,
    process::{Pid, test_kill_process},
};
use zruntime::{
    LocalRuntime,
    process::{Command, Stdio},
};

/// Children are waited for, although every thread of the pool of blocking work is held: the status
/// of a child that is still running as the wait for it starts resolves once the child has exited,
/// and, on unix, a child that is dropped while its process runs is reaped.
///
/// The waits for children have a pool of their own, so none of them waits for a thread of
/// `unblock()`'s, which the test holds until it has seen both done. Where the runtime watches for
/// the exit of a child, as it does on Linux through a pidfd, the status needs no thread, and
/// resolves either way. Where it cannot, as on Windows, the wait for the exit is blocking work, and
/// that it resolves is what shows the wait ran on the pool of its own.
#[test]
#[ntest::timeout(30000)]
fn children_are_waited_for_while_blocking_work_fills_the_pool() {
    // The test's own thread counts in both barriers: in the first, so as to know that every thread
    // of the pool holds a job, and in the second, to let the jobs go once it is done.
    let started = Arc::new(Barrier::new(THREADS + 1));
    let release = Arc::new(Barrier::new(THREADS + 1));
    for _ in 0..THREADS {
        let started = started.clone();
        let release = release.clone();
        drop(zruntime::unblock(move || {
            started.wait();
            release.wait();
        }));
    }
    started.wait();

    // The pool is full, as the test needs it to be: one job more has no thread to run on.
    let (report, reported) = mpsc::channel();
    drop(zruntime::unblock(move || {
        let _ = report.send(());
    }));
    assert!(
        reported.recv_timeout(Duration::from_millis(200)).is_err(),
        "the pool has a thread to spare",
    );

    let runtime = LocalRuntime::new().unwrap();

    // The child reads its input to its end before it exits, and the test holds the pipe to that
    // input until the wait for the exit is under way: the first look at the status finds the child
    // running, then, and starts the wait that needs a thread where the runtime does not watch for
    // the exit. A child that had exited before that look would be collected by it, with no wait at
    // all. Closing the pipe takes no thread of the full pool: the runtime watches it on unix, and
    // on Windows a pipe with no operation in flight closes as it is dropped. The other streams are
    // inherited, so that no other pipe is in the way. A wait that never ends is given up on, so
    // that the test still gets to let the pool go and judge.
    let mut child = filter().stdin(Stdio::piped()).spawn(&runtime).unwrap();
    let input = child.stdin.take();
    let (waited, status) = runtime.block_on(async {
        let mut status = pin!(child.status());
        if let Some(status) = poll_immediate(&mut status).await {
            return (false, Ok(status));
        }
        drop(input);

        (true, runtime.timeout(Duration::from_secs(5), status).await)
    });

    #[cfg(unix)]
    let reaped = {
        let child = Command::new("sh")
            .args(["-c", "sleep 0.3"])
            .spawn(&runtime)
            .unwrap();
        let id = child.id();

        drop(child);
        disappears(id)
    };

    // The pool is let go of before the test judges, so that a failure does not leave it held.
    release.wait();
    assert!(waited, "the child exited before its input ended");
    let status = status.expect("the wait for the child never ended").unwrap();
    assert!(status.success());
    #[cfg(unix)]
    assert!(reaped, "the child was never collected");
}

/// A command that reads its input to its end and then exits, writing out what it read: `cat`, and
/// `sort` on Windows, which has no `cat`.
fn filter() -> Command {
    Command::new(if cfg!(windows) { "sort" } else { "cat" })
}

/// Waits, for a few seconds at most, for the process with the ID `id` to be gone from the process
/// table, and tells whether it went.
///
/// A process that has exited stays there, as a zombie, until its status is collected, so this is
/// also how the test sees that somebody collected it.
#[cfg(unix)]
fn disappears(id: u32) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);

    while exists(id) {
        if Instant::now() > deadline {
            return false;
        }
        thread::sleep(Duration::from_millis(10));
    }

    true
}

/// Whether a process with the ID `id` is there, a zombie included.
#[cfg(unix)]
fn exists(id: u32) -> bool {
    let pid = Pid::from_raw(id as i32).expect("a child's process ID is not zero");

    test_kill_process(pid) != Err(Errno::SRCH)
}

/// The most threads that the pool of `zruntime::unblock` has at once, as the documentation of that
/// function gives it.
const THREADS: usize = 500;
