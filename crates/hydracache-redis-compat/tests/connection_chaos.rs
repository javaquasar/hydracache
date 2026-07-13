use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use hydracache_client_transport_axum::{ClientSurfaceLimits, ClientSurfaceState};
use hydracache_redis_compat::{RedisListenerConfig, RedisRespServer};
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};
use tokio::task::JoinHandle;

#[tokio::test]
async fn half_open_and_reset_connections_free_resources_without_leaking_inflight_work() {
    let server = listener(RedisListenerConfig::default());
    let tracker = ConnectionTracker::new(4);
    let (mut client, server_io) = tokio::io::duplex(4096);
    let serve = spawn_tracked(Arc::clone(&server), tracker.clone(), server_io);

    client
        .write_all(b"*3\r\n$3\r\nSET\r\n$1\r\nk\r\n")
        .await
        .unwrap();
    wait_for_active(&tracker, 1).await;
    client.shutdown().await.unwrap();
    let mut output = Vec::new();
    client.read_to_end(&mut output).await.unwrap();
    assert!(output.is_empty());
    join_connection(serve).await;

    assert_eq!(tracker.active(), 0);
    assert_eq!(server.state().state_mutations(), 0);
    assert_eq!(server.metrics().commands, 0);

    let retry = exchange_tracked(
        &server,
        &tracker,
        b"*3\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n\
          *2\r\n$3\r\nGET\r\n$1\r\nk\r\n\
          *1\r\n$4\r\nQUIT\r\n",
    )
    .await;

    assert_eq!(retry, b"+OK\r\n$1\r\nv\r\n+OK\r\n");
    assert_eq!(tracker.active(), 0);
    assert_eq!(server.metrics().accepted_connections, 2);
}

#[tokio::test]
async fn connection_limit_exhaustion_is_bounded_and_recovers_not_ooms() {
    let server = listener(RedisListenerConfig {
        idle_timeout: Duration::from_secs(30),
        ..RedisListenerConfig::default()
    });
    let tracker = ConnectionTracker::new(2);
    let (client_a, server_a) = tokio::io::duplex(4096);
    let (client_b, server_b) = tokio::io::duplex(4096);
    let task_a = spawn_tracked(Arc::clone(&server), tracker.clone(), server_a);
    let task_b = spawn_tracked(Arc::clone(&server), tracker.clone(), server_b);
    wait_for_active(&tracker, 2).await;

    assert!(
        tracker.try_enter().is_none(),
        "third connection must be bounded while the pool is exhausted"
    );
    assert_eq!(tracker.high_water(), 2);

    drop(client_a);
    join_connection(task_a).await;
    wait_for_active(&tracker, 1).await;
    let recovered = tracker
        .try_enter()
        .expect("pool should accept a replacement after one reset");
    assert_eq!(tracker.active(), 2);
    drop(recovered);

    drop(client_b);
    join_connection(task_b).await;
    wait_for_active(&tracker, 0).await;
    assert_eq!(tracker.active(), 0);
    assert_eq!(tracker.high_water(), 2);
}

#[tokio::test]
async fn connection_churn_returns_counters_to_baseline_no_leak() {
    let server = listener(RedisListenerConfig::default());
    let tracker = ConnectionTracker::new(3);
    let baseline = tracker.active();

    for _ in 0..24 {
        let output = exchange_tracked(
            &server,
            &tracker,
            b"*1\r\n$4\r\nPING\r\n\
              *1\r\n$4\r\nQUIT\r\n",
        )
        .await;
        assert_eq!(output, b"+PONG\r\n+OK\r\n");
        assert_eq!(tracker.active(), baseline);
    }

    assert_eq!(server.metrics().accepted_connections, 24);
    assert_eq!(server.metrics().commands, 48);
    assert_eq!(tracker.high_water(), 1);
}

#[test]
fn canary_connection_reset_leaks_an_inflight_ticket() {
    let tracker = ConnectionTracker::new(1);
    let ticket = tracker
        .try_enter()
        .expect("fixture should acquire the only ticket");

    std::mem::forget(ticket);

    assert_eq!(tracker.active(), 1);
    assert!(
        tracker.try_enter().is_none(),
        "a leaked ticket must keep the guard-visible pool exhausted"
    );
}

fn listener(config: RedisListenerConfig) -> Arc<RedisRespServer> {
    Arc::new(
        RedisRespServer::new(
            Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap()),
            config,
        )
        .unwrap(),
    )
}

async fn exchange_tracked(
    server: &Arc<RedisRespServer>,
    tracker: &ConnectionTracker,
    input: &'static [u8],
) -> Vec<u8> {
    let (mut client, server_io) = tokio::io::duplex(4096);
    let serve = spawn_tracked(Arc::clone(server), tracker.clone(), server_io);
    client.write_all(input).await.unwrap();
    let mut output = Vec::new();
    client.read_to_end(&mut output).await.unwrap();
    join_connection(serve).await;
    output
}

fn spawn_tracked(
    server: Arc<RedisRespServer>,
    tracker: ConnectionTracker,
    stream: DuplexStream,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let _ticket = tracker
            .try_enter()
            .expect("test harness should not spawn beyond its configured limit");
        server.serve_connection(stream).await.unwrap();
    })
}

async fn join_connection(handle: JoinHandle<()>) {
    tokio::time::timeout(Duration::from_secs(1), handle)
        .await
        .expect("connection task should finish promptly")
        .expect("connection task should not panic");
}

async fn wait_for_active(tracker: &ConnectionTracker, expected: u64) {
    tokio::time::timeout(Duration::from_secs(1), async {
        while tracker.active() != expected {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("connection tracker should reach the expected count");
}

#[derive(Clone)]
struct ConnectionTracker {
    max: u64,
    active: Arc<AtomicU64>,
    high_water: Arc<AtomicU64>,
}

impl ConnectionTracker {
    fn new(max: u64) -> Self {
        Self {
            max,
            active: Arc::new(AtomicU64::new(0)),
            high_water: Arc::new(AtomicU64::new(0)),
        }
    }

    fn try_enter(&self) -> Option<ConnectionTicket> {
        let mut current = self.active.load(Ordering::SeqCst);
        loop {
            if current >= self.max {
                return None;
            }
            match self.active.compare_exchange(
                current,
                current + 1,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => {
                    self.high_water.fetch_max(current + 1, Ordering::SeqCst);
                    return Some(ConnectionTicket {
                        tracker: self.clone(),
                    });
                }
                Err(observed) => current = observed,
            }
        }
    }

    fn active(&self) -> u64 {
        self.active.load(Ordering::SeqCst)
    }

    fn high_water(&self) -> u64 {
        self.high_water.load(Ordering::SeqCst)
    }
}

struct ConnectionTicket {
    tracker: ConnectionTracker,
}

impl Drop for ConnectionTicket {
    fn drop(&mut self) {
        self.tracker.active.fetch_sub(1, Ordering::SeqCst);
    }
}
