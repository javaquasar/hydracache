//! What is left of a runtime once the connections it ran are gone.
//!
//! A connection hands its runtime tasks, timers and sources, and each of those holds the
//! runtime's state; releasing the runtime has to give all of it back, or a process that outlives
//! one connection carries the whole of it for good. A peer-to-peer pair covers both ways a
//! connection ends: one side says goodbye, the other simply stops existing.
//!
//! What a task produced is the runtime's to release as well, which is what the second test here
//! is about: such a value is free to reach back into the runtime as it goes. The last one is about
//! blocking work given up on before it is done: the thread doing it runs on, and must not keep the
//! runtime alive while it does.
#![cfg(feature = "p2p")]

use std::{
    os::unix::net::UnixStream,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
};

use ntest::timeout;
use zbus::{Guid, connection::Builder, runtime::traits};

use crate::runtime::{Handle, Runtime};

/// Nothing of two finished connections outlives the runtime that ran them.
#[test]
#[timeout(15000)]
fn a_released_runtime_gives_back_everything_its_connections_took() {
    let runtime = Runtime::new().unwrap();
    let probe = runtime.probe();
    let (server_end, client_end) = UnixStream::pair().unwrap();

    runtime.run(async {
        // Both ends of the handshake run on this one thread, so neither can be awaited first.
        let (server, client) = futures_util::try_join!(
            Builder::unix_stream(server_end)
                .server(Guid::generate())
                .p2p()
                .runtime(runtime.handle())
                .build(),
            Builder::unix_stream(client_end)
                .p2p()
                .runtime(runtime.handle())
                .build(),
        )
        .unwrap();

        // The client leaves without a word, so its tasks are cancelled rather than run out.
        drop(client);
        server.graceful_shutdown().await;
    });

    assert_eq!(
        runtime.pending_timers(),
        0,
        "a connection left a timer behind"
    );

    drop(runtime);
    assert!(
        probe.is_released(),
        "the runtime's state outlived it: something it handed out still holds it",
    );
}

/// A detached task that has ended is released outside the list the runtime keeps it in.
///
/// Releasing it releases what it produced, and such a value's destructor is free to do what any
/// other one may: [`traits::TaskHandle::detach`] on a task spawned through the runtime's own
/// handle is one thing a connection's values do. That reaches for the same list, so a runtime
/// that dropped a finished task while holding it would wait on itself and never come back out
/// of `run`.
#[test]
#[timeout(15000)]
fn a_finished_detached_task_is_released_outside_the_runtimes_list() {
    let runtime = Runtime::new().unwrap();
    let probe = runtime.probe();
    let spawned_from_drop = Arc::new(AtomicBool::new(false));

    runtime.run(async {
        let output = SpawnsOnDrop {
            handle: runtime.handle(),
            spawned: spawned_from_drop.clone(),
        };
        // Detached, so nothing but the runtime's run loop can notice that this task has ended
        // and let the value it produced go.
        traits::TaskHandle::detach(traits::Runtime::spawn(
            &runtime.handle(),
            "a task with an output",
            async move { output },
        ));

        // One turn of the loop ends the task above and releases its output, and the task that
        // output spawned runs on another; both are turns this wait leaves room for.
        while !spawned_from_drop.load(Ordering::Acquire) {
            futures_lite::future::yield_now().await;
        }
    });

    drop(runtime);
    assert!(
        probe.is_released(),
        "the runtime's state outlived it: something it handed out still holds it",
    );
}

/// Blocking work whose future is dropped before it is done holds nothing of the runtime.
///
/// The future leaves the waker it was polled with for the thread doing the work, and here that is
/// the waker of [`Runtime::run`] itself, which holds the runtime's state. A connection attempt
/// cancelled during a blocking lookup, or a wait for a helper process, gives such work up in the
/// same way, and the work may go on for as long as the process it waits for.
#[test]
#[timeout(15000)]
fn blocking_work_given_up_on_holds_nothing_of_the_runtime() {
    let runtime = Runtime::new().unwrap();
    let probe = runtime.probe();
    let (release, released) = mpsc::channel::<()>();
    let (report, reported) = mpsc::channel();

    runtime.run(async {
        let mut work = traits::Runtime::spawn_blocking(&runtime.handle(), move || {
            released.recv().expect("the test lets the work go on");
            report.send(()).expect("the test waits for the work");
        });
        // The work waits on the channel, so one poll leaves the waker with the thread, and the
        // future goes at the end of this block with the work still under way.
        assert!(futures_lite::future::poll_once(&mut work).await.is_none());
    });

    drop(runtime);
    assert!(
        probe.is_released(),
        "the thread doing the blocking work still holds the runtime's state",
    );
    release.send(()).expect("the work waits for the test");
    reported.recv().expect("the work runs to its end");
}

/// A value that spawns a task on the runtime as it goes, standing in for the connection values
/// whose destructors do the same.
struct SpawnsOnDrop {
    handle: Handle,
    // Set by the spawned task, so the test can wait for the whole chain rather than for a turn
    // count of the loop.
    spawned: Arc<AtomicBool>,
}

impl Drop for SpawnsOnDrop {
    fn drop(&mut self) {
        let spawned = self.spawned.clone();

        traits::TaskHandle::detach(traits::Runtime::spawn(
            &self.handle,
            "a task spawned from a drop",
            async move { spawned.store(true, Ordering::Release) },
        ));
    }
}
