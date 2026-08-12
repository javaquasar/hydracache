use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt as _;
use hydracache_client_protocol::{ClientRequest, ClientRequestEnvelope, Namespace, StructuredKey};
use hydracache_client_transport_axum::{ClientIdentity, ClientSurfaceLimits, ClientSurfaceState};
use hydracache_redis_compat::{
    RedisAuthConfig, RedisKeyspaceEventConfig, RedisListenerConfig, RedisRespServer,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio::time::timeout;

#[tokio::test]
async fn redis_rs_pattern_listener_receives_keyspace_and_keyevent_messages() {
    let (server, address, accept) = spawn_event_listener(RedisAuthConfig::default()).await;
    let client = redis::Client::open(format!("redis://{address}/")).unwrap();
    let mut pubsub = client.get_async_pubsub().await.unwrap();
    pubsub.psubscribe("__key*__:*").await.unwrap();

    let mut writer = client.get_multiplexed_async_connection().await.unwrap();
    let response: String = redis::cmd("SET")
        .arg("user:42")
        .arg("value")
        .query_async(&mut writer)
        .await
        .unwrap();
    assert_eq!(response, "OK");

    let mut messages = pubsub.on_message();
    let first = timeout(Duration::from_secs(2), messages.next())
        .await
        .expect("keyspace event timeout")
        .expect("keyspace event stream closed");
    let second = timeout(Duration::from_secs(2), messages.next())
        .await
        .expect("keyevent timeout")
        .expect("keyevent stream closed");

    assert_eq!(first.get_pattern::<String>().unwrap(), "__key*__:*");
    assert_eq!(first.get_channel_name(), "__keyspace@0__:user:42");
    assert_eq!(first.get_payload::<String>().unwrap(), "set");
    assert_eq!(second.get_pattern::<String>().unwrap(), "__key*__:*");
    assert_eq!(second.get_channel_name(), "__keyevent@0__:set");
    assert_eq!(second.get_payload::<String>().unwrap(), "user:42");
    assert_eq!(server.metrics().event_messages, 2);

    drop(messages);
    drop(pubsub);
    drop(writer);
    accept.abort();
}

#[tokio::test]
async fn redis_rs_listener_covers_supported_mutation_event_names() {
    let (_server, address, accept) = spawn_event_listener(RedisAuthConfig::default()).await;
    let client = redis::Client::open(format!("redis://{address}/")).unwrap();
    let mut pubsub = client.get_async_pubsub().await.unwrap();
    pubsub.psubscribe("__keyevent@0__:*").await.unwrap();
    let mut writer = client.get_multiplexed_async_connection().await.unwrap();
    let mut messages = pubsub.on_message();

    let _: String = redis::cmd("SET")
        .arg("lifecycle")
        .arg("value")
        .query_async(&mut writer)
        .await
        .unwrap();
    assert_keyevent(&mut messages, "set", "lifecycle").await;

    let applied: i64 = redis::cmd("EXPIRE")
        .arg("lifecycle")
        .arg(60)
        .query_async(&mut writer)
        .await
        .unwrap();
    assert_eq!(applied, 1);
    assert_keyevent(&mut messages, "expire", "lifecycle").await;

    let applied: i64 = redis::cmd("PERSIST")
        .arg("lifecycle")
        .query_async(&mut writer)
        .await
        .unwrap();
    assert_eq!(applied, 1);
    assert_keyevent(&mut messages, "persist", "lifecycle").await;

    let removed: i64 = redis::cmd("DEL")
        .arg("lifecycle")
        .query_async(&mut writer)
        .await
        .unwrap();
    assert_eq!(removed, 1);
    assert_keyevent(&mut messages, "del", "lifecycle").await;

    drop(messages);
    drop(pubsub);
    drop(writer);
    accept.abort();
}

#[tokio::test]
async fn redis_listener_observes_hc2_style_mutation_on_shared_verified_surface() {
    let (server, address, accept) = spawn_event_listener(RedisAuthConfig::default()).await;
    let client = redis::Client::open(format!("redis://{address}/")).unwrap();
    let mut pubsub = client.get_async_pubsub().await.unwrap();
    pubsub.subscribe("__keyevent@0__:set").await.unwrap();

    let identity = ClientIdentity::new("hc2-event-test", "redis").unwrap();
    let response = server.state().dispatch_verified_request(
        &identity,
        ClientRequestEnvelope::new(
            "hc2-put",
            ClientRequest::Put {
                ns: Namespace::new("redis").unwrap(),
                key: StructuredKey::new(vec!["redis-binary-v1-686332".to_owned()]).unwrap(),
                value: b"shared".to_vec(),
                ttl_ms: None,
                dimensions: Vec::new(),
            },
        ),
    );
    assert!(response.result.is_ok());

    let message = timeout(Duration::from_secs(2), pubsub.on_message().next())
        .await
        .expect("shared-surface event timeout")
        .expect("shared-surface event stream closed");
    assert_eq!(message.get_channel_name(), "__keyevent@0__:set");
    assert_eq!(message.get_payload::<String>().unwrap(), "hc2");

    drop(pubsub);
    accept.abort();
}

#[tokio::test]
async fn event_listener_is_disabled_by_default_and_auth_precedes_subscription() {
    let disabled = RedisRespServer::new(
        Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap()),
        RedisListenerConfig::default(),
    )
    .unwrap();
    let disabled_output = exchange(
        &disabled,
        b"*2\r\n$9\r\nSUBSCRIBE\r\n$18\r\n__keyevent@0__:set\r\n*1\r\n$4\r\nQUIT\r\n",
    )
    .await;
    assert_eq!(
        disabled_output,
        b"-ERR keyspace event listener is disabled\r\n+OK\r\n"
    );

    let auth = RedisAuthConfig::required("event-secret");
    let protected = event_server(auth);
    let protected_output = exchange(
        &protected,
        b"*2\r\n$9\r\nSUBSCRIBE\r\n$18\r\n__keyevent@0__:set\r\n*1\r\n$4\r\nQUIT\r\n",
    )
    .await;
    assert_eq!(
        protected_output,
        b"-NOAUTH Authentication required.\r\n+OK\r\n"
    );
}

#[tokio::test]
async fn unsubscribe_acknowledges_and_leaves_resp2_subscribed_mode() {
    let server = event_server(RedisAuthConfig::default());
    let output = exchange(
        &server,
        b"*2\r\n$9\r\nSUBSCRIBE\r\n$18\r\n__keyevent@0__:set\r\n\
          *2\r\n$11\r\nUNSUBSCRIBE\r\n$18\r\n__keyevent@0__:set\r\n\
          *1\r\n$4\r\nPING\r\n\
          *1\r\n$4\r\nQUIT\r\n",
    )
    .await;
    assert_eq!(
        output,
        b"*3\r\n$9\r\nsubscribe\r\n$18\r\n__keyevent@0__:set\r\n:1\r\n\
          *3\r\n$11\r\nunsubscribe\r\n$18\r\n__keyevent@0__:set\r\n:0\r\n\
          +PONG\r\n+OK\r\n"
    );
    assert_eq!(server.metrics().active_event_subscribers, 0);
}

#[tokio::test]
async fn pattern_unsubscribe_acknowledges_and_leaves_resp2_subscribed_mode() {
    let server = event_server(RedisAuthConfig::default());
    let output = exchange(
        &server,
        b"*2\r\n$10\r\nPSUBSCRIBE\r\n$10\r\n__key*__:*\r\n\
          *2\r\n$12\r\nPUNSUBSCRIBE\r\n$10\r\n__key*__:*\r\n\
          *1\r\n$4\r\nPING\r\n\
          *1\r\n$4\r\nQUIT\r\n",
    )
    .await;
    assert_eq!(
        output,
        b"*3\r\n$10\r\npsubscribe\r\n$10\r\n__key*__:*\r\n:1\r\n\
          *3\r\n$12\r\npunsubscribe\r\n$10\r\n__key*__:*\r\n:0\r\n\
          +PONG\r\n+OK\r\n"
    );
    assert_eq!(server.metrics().active_event_subscribers, 0);
}

#[tokio::test]
async fn subscription_limit_is_atomic_and_counts_unique_channels() {
    let server = RedisRespServer::new(
        Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap()),
        RedisListenerConfig {
            keyspace_events: RedisKeyspaceEventConfig {
                enabled: true,
                max_subscriptions_per_connection: 1,
                max_subscription_bytes_per_connection: 64 * 1024,
            },
            ..RedisListenerConfig::default()
        },
    )
    .unwrap();

    let rejected = exchange(
        &server,
        b"*3\r\n$9\r\nSUBSCRIBE\r\n$1\r\na\r\n$1\r\nb\r\n*1\r\n$4\r\nQUIT\r\n",
    )
    .await;
    assert_eq!(
        rejected,
        b"-ERR maximum event subscriptions exceeded\r\n+OK\r\n"
    );

    let duplicate = exchange(
        &server,
        b"*3\r\n$9\r\nSUBSCRIBE\r\n$1\r\na\r\n$1\r\na\r\n\
          *1\r\n$11\r\nUNSUBSCRIBE\r\n*1\r\n$4\r\nQUIT\r\n",
    )
    .await;
    assert_eq!(
        duplicate,
        b"*3\r\n$9\r\nsubscribe\r\n$1\r\na\r\n:1\r\n\
          *3\r\n$9\r\nsubscribe\r\n$1\r\na\r\n:1\r\n\
          *3\r\n$11\r\nunsubscribe\r\n$1\r\na\r\n:0\r\n+OK\r\n"
    );
}

#[tokio::test]
async fn subscription_byte_limit_rejects_admission_atomically() {
    let server = RedisRespServer::new(
        Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap()),
        RedisListenerConfig {
            keyspace_events: RedisKeyspaceEventConfig {
                enabled: true,
                max_subscriptions_per_connection: 8,
                max_subscription_bytes_per_connection: 3,
            },
            ..RedisListenerConfig::default()
        },
    )
    .unwrap();

    let output = exchange(
        &server,
        b"*3\r\n$9\r\nSUBSCRIBE\r\n$2\r\naa\r\n$2\r\nbb\r\n*1\r\n$4\r\nQUIT\r\n",
    )
    .await;
    assert_eq!(
        output,
        b"-ERR maximum event subscriptions exceeded\r\n+OK\r\n"
    );
    assert_eq!(server.metrics().active_event_subscribers, 0);
}

#[tokio::test]
async fn resp2_subscribed_mode_rejects_ordinary_commands() {
    let server = event_server(RedisAuthConfig::default());
    let output = exchange(
        &server,
        b"*2\r\n$9\r\nSUBSCRIBE\r\n$1\r\nx\r\n\
          *2\r\n$3\r\nGET\r\n$1\r\nx\r\n\
          *2\r\n$11\r\nUNSUBSCRIBE\r\n$1\r\nx\r\n\
          *1\r\n$4\r\nQUIT\r\n",
    )
    .await;
    assert_eq!(
        output,
        b"*3\r\n$9\r\nsubscribe\r\n$1\r\nx\r\n:1\r\n\
          -ERR only (P)SUBSCRIBE / (P)UNSUBSCRIBE / PING / QUIT are allowed in RESP2 subscribed mode\r\n\
          *3\r\n$11\r\nunsubscribe\r\n$1\r\nx\r\n:0\r\n+OK\r\n"
    );
}

#[tokio::test]
async fn resp3_push_mode_keeps_ordinary_commands_available() {
    let server = Arc::new(event_server(RedisAuthConfig::default()));
    let (mut client, server_io) = tokio::io::duplex(4096);
    let serving = Arc::clone(&server);
    let serve = tokio::spawn(async move {
        serving.serve_connection(server_io).await.unwrap();
    });

    client
        .write_all(
            b"*2\r\n$5\r\nHELLO\r\n$1\r\n3\r\n\
              *2\r\n$9\r\nSUBSCRIBE\r\n$18\r\n__keyevent@0__:set\r\n",
        )
        .await
        .unwrap();
    let subscribed = read_until(
        &mut client,
        b">3\r\n$9\r\nsubscribe\r\n$18\r\n__keyevent@0__:set\r\n:1\r\n",
    )
    .await;
    assert!(subscribed.starts_with(b"%7\r\n"));

    client
        .write_all(b"*3\r\n$3\r\nSET\r\n$4\r\nuser\r\n$5\r\nvalue\r\n")
        .await
        .unwrap();
    let message = read_until(
        &mut client,
        b">3\r\n$7\r\nmessage\r\n$18\r\n__keyevent@0__:set\r\n$4\r\nuser\r\n",
    )
    .await;
    assert!(message.starts_with(b"+OK\r\n"));

    client.write_all(b"*1\r\n$4\r\nQUIT\r\n").await.unwrap();
    assert_eq!(read_until(&mut client, b"+OK\r\n").await, b"+OK\r\n");
    serve.await.unwrap();
}

async fn spawn_event_listener(
    auth: RedisAuthConfig,
) -> (Arc<RedisRespServer>, std::net::SocketAddr, JoinHandle<()>) {
    let server = Arc::new(event_server(auth));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let serving = Arc::clone(&server);
    let accept = tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let server = Arc::clone(&serving);
            tokio::spawn(async move {
                server.serve_connection(stream).await.unwrap();
            });
        }
    });
    (server, address, accept)
}

fn event_server(auth: RedisAuthConfig) -> RedisRespServer {
    RedisRespServer::new(
        Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap()),
        RedisListenerConfig {
            auth,
            keyspace_events: RedisKeyspaceEventConfig {
                enabled: true,
                max_subscriptions_per_connection: 8,
                max_subscription_bytes_per_connection: 64 * 1024,
            },
            ..RedisListenerConfig::default()
        },
    )
    .unwrap()
}

async fn exchange(server: &RedisRespServer, input: &'static [u8]) -> Vec<u8> {
    let (mut client, server_io) = tokio::io::duplex(4096);
    let serve = async {
        server.serve_connection(server_io).await.unwrap();
    };
    let client_io = async move {
        client.write_all(input).await.unwrap();
        client.shutdown().await.unwrap();
        let mut output = Vec::new();
        client.read_to_end(&mut output).await.unwrap();
        output
    };
    let (_, output) = tokio::join!(serve, client_io);
    output
}

async fn read_until(stream: &mut (impl tokio::io::AsyncRead + Unpin), marker: &[u8]) -> Vec<u8> {
    timeout(Duration::from_secs(2), async {
        let mut output = Vec::new();
        let mut chunk = [0_u8; 512];
        while !output.windows(marker.len()).any(|window| window == marker) {
            let read = stream.read(&mut chunk).await.unwrap();
            assert_ne!(read, 0, "RESP stream closed before the expected marker");
            output.extend_from_slice(&chunk[..read]);
        }
        output
    })
    .await
    .expect("RESP marker timeout")
}

async fn assert_keyevent(
    messages: &mut (impl futures_util::Stream<Item = redis::Msg> + Unpin),
    event: &str,
    key: &str,
) {
    let message = timeout(Duration::from_secs(2), messages.next())
        .await
        .expect("keyevent timeout")
        .expect("keyevent stream closed");
    assert_eq!(
        message.get_channel_name(),
        format!("__keyevent@0__:{event}")
    );
    assert_eq!(message.get_payload::<String>().unwrap(), key);
}
