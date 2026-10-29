//! The child processes the runtime waits for on its own timer.
//!
//! The runtime has no way to be told that a child exited and starts no thread to wait for one, so
//! it looks at the child at an interval, on the timer it has for everything else.

use std::{
    io,
    process::{Command, Stdio},
};

use futures_lite::future::poll_once;
use ntest::timeout;
use zbus::runtime::traits;

use crate::runtime::Runtime;

/// The exit status of a process comes back.
#[test]
#[timeout(15000)]
fn the_exit_status_of_a_process_comes_back() {
    let runtime = Runtime::new().unwrap();
    let handle = runtime.handle();
    let mut command = Command::new("sh");
    command.args(["-c", "exit 3"]);

    let status = runtime.run(async {
        traits::Runtime::spawn_process(
            &handle,
            command,
            Stdio::null(),
            Stdio::null(),
            Stdio::null(),
        )
        .unwrap()
        .await
        .unwrap()
    });

    assert_eq!(status.code(), Some(3));
    assert_eq!(runtime.pending_timers(), 0, "the wait left a timer behind");
}

/// A process that is still running is waited for on the runtime's timer.
///
/// The process reads its input before it exits, and nothing ends that input until the wait has
/// been polled once, so the first poll finds it running and arms a timer for the next look.
#[test]
#[timeout(15000)]
fn a_process_that_is_running_is_waited_for_on_the_runtimes_timer() {
    let runtime = Runtime::new().unwrap();
    let handle = runtime.handle();
    let mut command = Command::new("sh");
    command.args(["-c", "read ignored; exit 5"]);
    let (process_input, input) = io::pipe().unwrap();

    let status = runtime.run(async {
        let mut exit = traits::Runtime::spawn_process(
            &handle,
            command,
            Stdio::from(process_input),
            Stdio::null(),
            Stdio::null(),
        )
        .unwrap();
        assert!(
            poll_once(&mut exit).await.is_none(),
            "the process had exited before its input ended",
        );
        assert_eq!(
            runtime.pending_timers(),
            1,
            "the wait for a running process is not on the runtime's timer",
        );

        drop(input);
        exit.await.unwrap()
    });

    assert_eq!(status.code(), Some(5));
    assert_eq!(runtime.pending_timers(), 0, "the wait left a timer behind");
}
