use std::{
    future::poll_fn,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::Poll,
    time::Duration,
};

#[test]
fn background_progress_between_entries() {
    let (release, receive) = tokio::sync::oneshot::channel();
    let (done, observe) = std::sync::mpsc::sync_channel(1);
    let runtime = zbus::block_on(async { tokio::runtime::Handle::current() });
    let task = runtime.spawn(async move {
        receive.await.unwrap();
        done.send(74u32).unwrap();
        74u32
    });
    release.send(()).unwrap();
    // Outside every block_on: the old runtime must continue driving its tasks.
    let progress = observe.recv_timeout(Duration::from_millis(250));
    // Retire the task even for the known failing current-thread control.
    let joined = zbus::block_on(task).unwrap();
    println!(
        "HC074_RUNTIME_PROGRESS:{{\"outside_entry\":{},\"joined\":{joined},\"timeout_ms\":250}}",
        progress == Ok(74)
    );
    assert_eq!(joined, 74);
    assert_eq!(
        progress,
        Ok(74),
        "runtime stopped progressing outside entry"
    );
}

#[test]
fn nested_entry_does_not_poll_future() {
    let polled = Arc::new(AtomicBool::new(false));
    let check = polled.clone();
    let nested = std::panic::catch_unwind(|| {
        zbus::block_on(async move {
            zbus::block_on(poll_fn(move |_| {
                check.store(true, Ordering::SeqCst);
                Poll::Ready(74)
            }))
        })
    });
    assert!(nested.is_err());
    assert!(!polled.load(Ordering::SeqCst));
}

#[test]
fn invalid_address_refuses_before_build() {
    assert!(crate::bridge::ConnectionBuilder::address("invalid-transport").is_err());
}
