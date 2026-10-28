//! The connects the runtime makes on its poller and timer, where a connection has to wait.
//!
//! A connect to a listener with room for it is over before it starts waiting, which is what a
//! connection to a bus nearly always meets; these two aim at a listener with none, and at what the
//! runtime then waits on.

use std::pin::pin;

use ntest::timeout;
use socket2::{SockAddr, SockRef, Socket, Type};
use zbus::runtime::traits;

use crate::runtime::Runtime;

/// A TCP connect the kernel has not finished is waited for on the poller, and then handed over.
#[test]
#[timeout(15000)]
fn a_tcp_connect_the_kernel_takes_over_is_waited_for_on_the_poller() {
    let runtime = Runtime::new().unwrap();
    let handle = runtime.handle();
    let listener = listener(&SockAddr::from(std::net::SocketAddr::from((
        [127, 0, 0, 1],
        0,
    ))));
    let address = listener.local_addr().unwrap().as_socket().unwrap();

    runtime.run(async {
        // The listener's one place is taken from here on, so the kernel leaves the next
        // connection unanswered until the listener accepts this one.
        let _queued = traits::Runtime::connect_tcp(&handle, address)
            .await
            .unwrap();
        let mut pending = pin!(traits::Runtime::connect_tcp(&handle, address));
        assert!(
            futures_lite::future::poll_once(pending.as_mut())
                .await
                .is_none(),
            "the connection was reported before the listener had room for it",
        );

        let _accepted = listener.accept().unwrap();
        let source = pending.await.unwrap();

        assert!(
            SockRef::from(&source).peer_addr().is_ok(),
            "the connection was reported without a peer at the other end",
        );
        // Zbus registers the socket again for its traffic, which the poller would turn away if it
        // still watched the socket for the connect.
        traits::Runtime::register_io_source(&handle, source).unwrap();
    });
}

/// A unix connect a full backlog turns away is tried again after a wait on the runtime's timer.
#[cfg(any(target_os = "android", target_os = "linux"))]
#[test]
#[timeout(15000)]
fn a_unix_connect_turned_away_by_a_full_backlog_is_retried_on_the_timer() {
    let runtime = Runtime::new().unwrap();
    let handle = runtime.handle();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("socket");
    let listener = listener(&SockAddr::unix(&path).unwrap());

    runtime.run(async {
        let _queued = traits::Runtime::connect_unix(&handle, &path).await.unwrap();
        let mut pending = pin!(traits::Runtime::connect_unix(&handle, &path));
        assert!(
            futures_lite::future::poll_once(pending.as_mut())
                .await
                .is_none(),
            "the connection was reported before the listener had room for it",
        );
        assert_eq!(
            runtime.pending_timers(),
            1,
            "the turned away connect is not waiting on the runtime's timer",
        );

        let _accepted = listener.accept().unwrap();
        let source = pending.await.unwrap();

        assert!(
            SockRef::from(&source).peer_addr().is_ok(),
            "the connection was reported without a peer at the other end",
        );
    });

    assert_eq!(runtime.pending_timers(), 0, "the retry left a timer behind");
}

/// A listening socket at `address` with room for a single connection at a time.
fn listener(address: &SockAddr) -> Socket {
    let listener = Socket::new(address.domain(), Type::STREAM, None).unwrap();
    listener.bind(address).unwrap();
    listener.listen(0).unwrap();

    listener
}
