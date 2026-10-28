//! A whole connection lifecycle on the runtime, with the process's thread count around it.
//!
//! A connection built on the runtime does everything it does on the thread that calls
//! `Runtime::run`, and the thread count before and after a whole lifecycle is what checks that it
//! left no thread of its own behind.
#![cfg(all(feature = "proxy", feature = "service"))]

use std::{process::Command, time::Duration};

use futures_util::StreamExt;
use ntest::timeout;
use zbus::{connection::Builder, object_server::SignalEmitter, proxy::CacheProperties};

use crate::runtime::Runtime;

/// A connection built, served on, called through and shut down leaves the thread count as it was.
///
/// What happens in between is a round trip over everything a connection has to keep in flight at
/// once: a method call that answers out of a property and emits a signal on the way, a property
/// set and the `PropertiesChanged` it emits by default, and the two streams that see the signal
/// and the change. The proxy caches lazily, so the cache's own task is in flight as well. All of
/// it runs on the thread that calls `Runtime::run`.
///
/// Two readings of the thread count cannot see a thread that started and exited between them;
/// what they establish is that nothing the lifecycle started outlives it.
///
/// The count is of the whole process, and the harness starts and finishes a thread for every
/// other test in this binary, so the readings are taken in a process where this test is the only
/// one running: the test runs the binary again on its own name, alone, and [`ALONE`] in the
/// environment tells that run to do the work. A failure there is reported with everything that
/// run printed.
#[test]
#[timeout(15000)]
fn a_single_threaded_runtime_runs_a_connection_without_zbus_threads() {
    if std::env::var_os(ALONE).is_some() {
        return a_connection_lifecycle_leaves_no_thread();
    }

    // The harness names a test by its path from the crate root.
    let name = module_path!()
        .split_once("::")
        .map(|(_crate, module)| module)
        .expect("the test lives in a module of the binary");
    let output = Command::new(std::env::current_exe().expect("the test binary knows its path"))
        .args(["--exact", "--test-threads=1"])
        .arg(format!(
            "{name}::a_single_threaded_runtime_runs_a_connection_without_zbus_threads"
        ))
        .env(ALONE, "1")
        .output()
        .expect("the test binary runs again");

    let report = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "the run with the process to itself failed:\n{report}{}",
        String::from_utf8_lossy(&output.stderr),
    );
    // A name the harness matches nothing with is a run of no tests, and that passes too.
    assert!(
        report.contains("test result: ok. 1 passed"),
        "the run with the process to itself ran no test:\n{report}",
    );
}

/// Set in the environment of the run that has the process to itself.
const ALONE: &str = "ZBUS_POLLING_RUNTIME_ALONE";

/// The lifecycle itself, between the two readings of the thread count.
fn a_connection_lifecycle_leaves_no_thread() {
    let runtime = Runtime::new().unwrap();
    let handle = runtime.handle();
    let probe = runtime.probe();
    let before = threads();

    runtime.run(async {
        let conn = Builder::session()
            .runtime(handle)
            .method_timeout(Duration::from_secs(1))
            .serve_at("/org/zbus/PollingRuntime", Greeter::new("Hello"))
            .build()
            .await
            .unwrap();

        let name = conn
            .unique_name()
            .expect("a bus connection has a unique name")
            .to_string();
        let proxy = GreeterProxy::builder(&conn)
            .destination(name)
            .cache_properties(CacheProperties::Lazily)
            .build()
            .await
            .unwrap();

        // Both streams are in place before anything they are to see happens.
        let mut greeted = proxy.receive_greeted().await.unwrap();
        let mut greeting_changed = proxy.receive_greeting_changed().await;

        assert_eq!(proxy.greet("world").await.unwrap(), "Hello world!");
        let signal = greeted
            .next()
            .await
            .expect("the call named whom it greeted");
        assert_eq!(signal.args().unwrap().name, "world");

        // The stream's first value is the one the cache filled itself with. Once it is out, only
        // an announced change can put another value on the stream. Each event holds a clone of
        // the proxy, so only the values are kept.
        let initial = greeting_changed
            .next()
            .await
            .expect("the cache took the property's value")
            .get()
            .await
            .unwrap();
        assert_eq!(initial, "Hello");

        proxy.set_greeting("Howdy").await.unwrap();
        let changed = greeting_changed
            .next()
            .await
            .expect("the set was announced as a property change")
            .get()
            .await
            .unwrap();
        assert_eq!(changed, "Howdy");
        assert_eq!(proxy.greeting().await.unwrap(), "Howdy");
        assert_eq!(proxy.greet("world").await.unwrap(), "Howdy world!");

        // A shutdown waits for every clone of the connection: the streams' and the proxy's, and
        // the one the property cache's task holds, which goes with the proxy.
        drop(greeting_changed);
        drop(greeted);
        drop(proxy);
        conn.graceful_shutdown().await;
    });

    assert_eq!(
        threads(),
        before,
        "the connection's lifecycle left a thread behind",
    );
    assert_eq!(
        runtime.pending_timers(),
        0,
        "the lifecycle left a timer behind"
    );

    drop(runtime);
    assert!(
        probe.is_released(),
        "the runtime's state outlived it: something it handed out still holds it",
    );
}

/// How many threads the process has.
///
/// Only Linux is asked for it. Elsewhere the lifecycle still runs, with nothing to compare.
#[cfg(target_os = "linux")]
fn threads() -> usize {
    std::fs::read_dir("/proc/self/task")
        .expect("the thread directory of a running process")
        .count()
}

#[cfg(not(target_os = "linux"))]
fn threads() -> usize {
    0
}

/// An interface with a greeting to read, to write and to be told about.
struct Greeter {
    greeting: String,
}

impl Greeter {
    fn new(greeting: &str) -> Self {
        Self {
            greeting: greeting.to_string(),
        }
    }
}

#[zbus::interface(
    interface = "org.zbus.PollingRuntime.Greeter",
    proxy(default_path = "/org/zbus/PollingRuntime")
)]
impl Greeter {
    /// Greets `name` with the current greeting, and says whom it greeted.
    async fn greet(
        &self,
        name: &str,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> String {
        emitter
            .greeted(name)
            .await
            .expect("the greeting is announced");

        format!("{} {name}!", self.greeting)
    }

    /// What a greeting puts in front of the name it is for.
    #[zbus(property)]
    fn greeting(&self) -> String {
        self.greeting.clone()
    }

    #[zbus(property)]
    fn set_greeting(&mut self, greeting: &str) {
        self.greeting = greeting.to_string();
    }

    /// Emitted for each name that was greeted.
    #[zbus(signal)]
    async fn greeted(emitter: &SignalEmitter<'_>, name: &str) -> zbus::Result<()>;
}
