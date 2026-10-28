//! The wait for a child process, on the runtime's own timer.
//!
//! A runtime with a pidfd, a kqueue or a handler for `SIGCHLD` is told when a child exits. This
//! one has nothing of the kind, and starts no thread to block in `wait`, so it looks at the child
//! instead and waits between the looks on the timer it has for everything else.
//!
//! A future that is dropped before it resolves leaves the process to be collected when this
//! process exits. A connection awaits the future of a `unixexec:` program on a task of this
//! runtime, and drops one only where a call that waits for an `ibus:` or `launchd:` program is
//! given up on.

use std::{
    future::Future,
    io,
    pin::Pin,
    process::{Child, Command, ExitStatus, Stdio},
    task::{Context, Poll, ready},
    time::Duration,
};

use zbus::runtime::traits;

use super::{Handle, Sleep};

/// Spawns `command` with the three streams, and hands back the future of its exit status.
pub(super) fn spawn(
    handle: &Handle,
    mut command: Command,
    stdin: Stdio,
    stdout: Stdio,
    stderr: Stdio,
) -> io::Result<Pin<Box<dyn Future<Output = io::Result<ExitStatus>> + Send + 'static>>> {
    let child = command.stdin(stdin).stdout(stdout).stderr(stderr).spawn();
    // The command holds the streams it was given for as long as it lives, which is longer than
    // the process needs them: a copy of the write end of a pipe that stays open here is a pipe
    // that never ends for whoever reads it.
    drop(command);

    Ok(Box::pin(Exit {
        handle: handle.clone(),
        child: Some(child?),
        sleep: None,
    }))
}

/// How long the runtime waits between two looks at a child that is still running.
///
/// This is the cost of a runtime with nothing to tell it that a child exited: the exit is found
/// up to this long after it happened, and the runtime wakes this often for as long as it waits for
/// one.
const INTERVAL: Duration = Duration::from_millis(10);

/// The exit status of a child, found by looking at it on the runtime's timer.
///
/// A poll looks at the child and, while it runs, waits for [`INTERVAL`] before it looks again. A
/// future that is dropped before it resolves leaves the child as it is: it stays in the process
/// table, as a zombie once it has exited, until this process exits and the init process takes it
/// over.
struct Exit {
    handle: Handle,
    child: Option<Child>,
    sleep: Option<Sleep>,
}

impl Future for Exit {
    type Output = io::Result<ExitStatus>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut *self;

        loop {
            if let Some(sleep) = this.sleep.as_mut() {
                ready!(Pin::new(sleep).poll(cx));
                this.sleep = None;
            }

            let child = this
                .child
                .as_mut()
                .expect("the exit status of a child process was polled after it resolved");
            match child.try_wait() {
                Ok(Some(status)) => {
                    this.child = None;

                    return Poll::Ready(Ok(status));
                }
                Err(e) => {
                    this.child = None;

                    return Poll::Ready(Err(e));
                }
                // The timer is polled on the next turn of the loop, which is what stores the
                // waker that wakes this task once the interval has passed.
                Ok(None) => this.sleep = Some(traits::Runtime::sleep(&this.handle, INTERVAL)),
            }
        }
    }
}
