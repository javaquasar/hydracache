//! What a connection's method timeout does to the timers of the runtime that serves it.
//!
//! Between them the two tests here cover both ways a timer leaves the runtime's map: the deadline
//! arrives, or the call returns first and the timer is dropped unfired.
#![cfg(all(feature = "proxy", feature = "service"))]

use std::{io::ErrorKind, time::Duration};

use ntest::timeout;
use zbus::{Connection, connection::Builder, proxy::CacheProperties};

use crate::runtime::Runtime;

/// A call the peer never answers fails with the connection's timeout, and leaves no timer behind.
#[test]
#[timeout(15000)]
fn a_timed_out_call_on_the_runtime_is_cancelled() {
    let runtime = Runtime::new().unwrap();
    let handle = runtime.handle();
    let probe = runtime.probe();

    runtime.run(async {
        let conn = Builder::session()
            .runtime(handle)
            .method_timeout(Duration::from_millis(200))
            .serve_at(PATH, Answers)
            .build()
            .await
            .unwrap();

        let proxy = proxy(&conn).await;
        match proxy.never_replies().await {
            Err(zbus::Error::InputOutput(e)) => assert_eq!(e.kind(), ErrorKind::TimedOut),
            other => panic!("a call nobody answers should time out, got {other:?}"),
        }
    });

    assert_eq!(
        runtime.pending_timers(),
        0,
        "the timed-out call left a timer"
    );

    drop(runtime);
    assert!(
        probe.is_released(),
        "the runtime's state outlived it: something it handed out still holds it",
    );
}

/// A call that returns takes its timer with it, long before that timer would have fired.
#[test]
#[timeout(15000)]
fn a_call_that_returns_leaves_no_timer_on_the_runtime() {
    let runtime = Runtime::new().unwrap();
    let handle = runtime.handle();
    let probe = runtime.probe();

    runtime.run(async {
        let conn = Builder::session()
            .runtime(handle)
            // Far enough away that only dropping the timer can take it out of the map.
            .method_timeout(Duration::from_secs(5))
            .serve_at(PATH, Answers)
            .build()
            .await
            .unwrap();

        let proxy = proxy(&conn).await;
        assert_eq!(proxy.greet("world").await.unwrap(), "Hello world!");

        assert_eq!(
            runtime.pending_timers(),
            0,
            "the call that returned left its timer behind",
        );
    });

    drop(runtime);
    assert!(
        probe.is_released(),
        "the runtime's state outlived it: something it handed out still holds it",
    );
}

/// The path the interface is served at, and the one the proxy defaults to.
const PATH: &str = "/org/zbus/PollingRuntime";

/// A proxy on `conn` aimed at the interface `conn` serves itself.
async fn proxy(conn: &Connection) -> AnswersProxy<'_> {
    let name = conn
        .unique_name()
        .expect("a bus connection has a unique name")
        .to_string();

    AnswersProxy::builder(conn)
        .destination(name)
        .cache_properties(CacheProperties::No)
        .build()
        .await
        .unwrap()
}

/// An interface with one method that answers and one that never does.
struct Answers;

#[zbus::interface(
    interface = "org.zbus.PollingRuntime.Answers",
    proxy(default_path = "/org/zbus/PollingRuntime")
)]
impl Answers {
    async fn greet(&self, name: &str) -> String {
        format!("Hello {name}!")
    }

    async fn never_replies(&self) {
        std::future::pending::<()>().await
    }
}
