use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt as _;
use hydracache::{CacheOptions, HydraCache};
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
async fn native_bridge_exact_subscription_is_namespace_fenced_and_metadata_only() {
    let cache = HydraCache::local().build();
    let server = event_server_with_native(RedisAuthConfig::default(), "backend", cache.clone());
    let (server, address, accept) = spawn_server(server).await;
    let client = redis::Client::open(format!("redis://{address}/")).unwrap();
    let mut pubsub = client.get_async_pubsub().await.unwrap();
    pubsub.subscribe("__keyevent@0__:set").await.unwrap();
    pubsub.subscribe("__keyevent@0__:del").await.unwrap();
    let mut messages = pubsub.on_message();

    cache
        .put(
            "backend-extra:user:42",
            "prefix-collision".to_owned(),
            CacheOptions::new(),
        )
        .await
        .unwrap();
    cache
        .put(
            "redis:user:42",
            "wrong-namespace".to_owned(),
            CacheOptions::new(),
        )
        .await
        .unwrap();
    assert!(
        timeout(Duration::from_millis(100), messages.next())
            .await
            .is_err(),
        "prefix collisions and foreign namespaces must remain invisible"
    );

    cache
        .typed::<String>("backend")
        .put(
            "user:42:тест",
            "native-only".to_owned(),
            CacheOptions::new(),
        )
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(2), messages.next())
        .await
        .expect("native exact-subscription event timeout")
        .expect("native exact-subscription stream closed");
    assert_eq!(event.get_channel_name(), "__keyevent@0__:set");
    assert_eq!(event.get_payload::<String>().unwrap(), "user:42:тест");

    drop(messages);
    let mut reader = client.get_multiplexed_async_connection().await.unwrap();
    let value: Option<Vec<u8>> = redis::cmd("GET")
        .arg("user:42:тест")
        .query_async(&mut reader)
        .await
        .unwrap();
    assert_eq!(
        value, None,
        "native bridge must not copy values into the RESP store"
    );

    let mut messages = pubsub.on_message();
    cache
        .typed::<String>("backend")
        .remove("user:42:тест")
        .await
        .unwrap();
    assert!(
        timeout(Duration::from_millis(100), messages.next())
            .await
            .is_err(),
        "native remove must not fabricate a Redis del event"
    );
    assert_eq!(server.metrics().event_messages, 1);

    drop(messages);
    drop(pubsub);
    drop(reader);
    accept.abort();
}

#[tokio::test]
async fn native_bridge_resp3_uses_push_frames() {
    let cache = HydraCache::local().build();
    let server = Arc::new(event_server_with_native(
        RedisAuthConfig::default(),
        "redis",
        cache.clone(),
    ));
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

    cache
        .typed::<String>("redis")
        .put("native-resp3", "value".to_owned(), CacheOptions::new())
        .await
        .unwrap();
    let message = read_until(
        &mut client,
        b">3\r\n$7\r\nmessage\r\n$18\r\n__keyevent@0__:set\r\n$12\r\nnative-resp3\r\n",
    )
    .await;
    assert_eq!(
        message,
        b">3\r\n$7\r\nmessage\r\n$18\r\n__keyevent@0__:set\r\n$12\r\nnative-resp3\r\n"
    );
    assert_eq!(server.metrics().event_messages, 1);

    client.write_all(b"*1\r\n$4\r\nQUIT\r\n").await.unwrap();
    assert_eq!(read_until(&mut client, b"+OK\r\n").await, b"+OK\r\n");
    serve.await.unwrap();
}

#[tokio::test]
async fn native_bridge_unsubscribe_drops_receiver_and_resubscribe_does_not_replay() {
    let cache = HydraCache::local().build();
    let server = Arc::new(event_server_with_native(
        RedisAuthConfig::default(),
        "redis",
        cache.clone(),
    ));
    let (mut client, server_io) = tokio::io::duplex(4096);
    let serving = Arc::clone(&server);
    let serve = tokio::spawn(async move {
        serving.serve_connection(server_io).await.unwrap();
    });

    let subscribe = b"*2\r\n$9\r\nSUBSCRIBE\r\n$18\r\n__keyevent@0__:set\r\n";
    client.write_all(subscribe).await.unwrap();
    read_until(
        &mut client,
        b"*3\r\n$9\r\nsubscribe\r\n$18\r\n__keyevent@0__:set\r\n:1\r\n",
    )
    .await;

    cache
        .typed::<String>("redis")
        .put("before", "value".to_owned(), CacheOptions::new())
        .await
        .unwrap();
    read_until(
        &mut client,
        b"*3\r\n$7\r\nmessage\r\n$18\r\n__keyevent@0__:set\r\n$6\r\nbefore\r\n",
    )
    .await;

    client
        .write_all(b"*1\r\n$11\r\nUNSUBSCRIBE\r\n")
        .await
        .unwrap();
    read_until(
        &mut client,
        b"*3\r\n$11\r\nunsubscribe\r\n$18\r\n__keyevent@0__:set\r\n:0\r\n",
    )
    .await;
    timeout(Duration::from_secs(1), async {
        while server.metrics().active_event_subscribers != 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("unsubscribe should release the connection subscriber guard");

    let published_before = cache.stats().events_published;
    cache
        .typed::<String>("redis")
        .put(
            "while-unsubscribed",
            "value".to_owned(),
            CacheOptions::new(),
        )
        .await
        .unwrap();
    assert_eq!(
        cache.stats().events_published,
        published_before,
        "zero subscriptions must release the native event receiver"
    );

    client.write_all(subscribe).await.unwrap();
    read_until(
        &mut client,
        b"*3\r\n$9\r\nsubscribe\r\n$18\r\n__keyevent@0__:set\r\n:1\r\n",
    )
    .await;
    cache
        .typed::<String>("redis")
        .put("after", "value".to_owned(), CacheOptions::new())
        .await
        .unwrap();
    let output = read_until(
        &mut client,
        b"*3\r\n$7\r\nmessage\r\n$18\r\n__keyevent@0__:set\r\n$5\r\nafter\r\n",
    )
    .await;
    assert!(!String::from_utf8_lossy(&output).contains("while-unsubscribed"));

    client.write_all(b"*1\r\n$4\r\nQUIT\r\n").await.unwrap();
    read_until(&mut client, b"+OK\r\n").await;
    serve.await.unwrap();
    assert_eq!(server.metrics().active_event_subscribers, 0);
}

#[tokio::test]
async fn native_bridge_lag_disconnects_without_blocking_writers_and_can_recover() {
    let cache = HydraCache::local().event_buffer_capacity(1).build();
    let server = Arc::new(event_server_with_native(
        RedisAuthConfig::default(),
        "redis",
        cache.clone(),
    ));
    let (mut client, server_io) = tokio::io::duplex(128);
    let serving = Arc::clone(&server);
    let serve = tokio::spawn(async move {
        serving.serve_connection(server_io).await.unwrap();
    });

    client
        .write_all(b"*2\r\n$9\r\nSUBSCRIBE\r\n$18\r\n__keyevent@0__:set\r\n")
        .await
        .unwrap();
    read_until(
        &mut client,
        b"*3\r\n$9\r\nsubscribe\r\n$18\r\n__keyevent@0__:set\r\n:1\r\n",
    )
    .await;

    cache
        .put(
            format!("redis:{}", "x".repeat(512)).as_str(),
            0_u64,
            CacheOptions::new(),
        )
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(10)).await;
    timeout(Duration::from_secs(1), async {
        for index in 0..32_u64 {
            cache
                .put(
                    format!("redis:tail:{index}").as_str(),
                    index,
                    CacheOptions::new(),
                )
                .await
                .unwrap();
        }
    })
    .await
    .expect("native writers must not wait for a slow Redis socket");

    let mut output = Vec::new();
    timeout(Duration::from_secs(2), client.read_to_end(&mut output))
        .await
        .expect("lagged connection should close")
        .unwrap();
    assert!(String::from_utf8_lossy(&output)
        .contains("ERR native cache event subscriber lagged; reconnect and resubscribe"));
    serve.await.unwrap();
    assert_eq!(server.metrics().active_event_subscribers, 0);
    assert_eq!(server.metrics().lagged_event_subscribers, 1);
    assert!(cache.stats().event_subscriber_lagged > 0);

    let (mut recovered, recovered_io) = tokio::io::duplex(4096);
    let serving = Arc::clone(&server);
    let recovered_serve = tokio::spawn(async move {
        serving.serve_connection(recovered_io).await.unwrap();
    });
    recovered
        .write_all(b"*2\r\n$9\r\nSUBSCRIBE\r\n$18\r\n__keyevent@0__:set\r\n")
        .await
        .unwrap();
    read_until(
        &mut recovered,
        b"*3\r\n$9\r\nsubscribe\r\n$18\r\n__keyevent@0__:set\r\n:1\r\n",
    )
    .await;
    cache
        .typed::<String>("redis")
        .put("recovered", "value".to_owned(), CacheOptions::new())
        .await
        .unwrap();
    read_until(
        &mut recovered,
        b"*3\r\n$7\r\nmessage\r\n$18\r\n__keyevent@0__:set\r\n$9\r\nrecovered\r\n",
    )
    .await;
    recovered.write_all(b"*1\r\n$4\r\nQUIT\r\n").await.unwrap();
    read_until(&mut recovered, b"+OK\r\n").await;
    recovered_serve.await.unwrap();
}

#[tokio::test]
async fn native_and_verified_surface_events_are_delivered_once_without_global_order_claim() {
    let cache = HydraCache::local().build();
    let server = event_server_with_native(RedisAuthConfig::default(), "redis", cache.clone());
    let (server, address, accept) = spawn_server(server).await;
    let client = redis::Client::open(format!("redis://{address}/")).unwrap();
    let mut pubsub = client.get_async_pubsub().await.unwrap();
    pubsub.subscribe("__keyevent@0__:set").await.unwrap();
    let identity = ClientIdentity::new("parallel-hc2", "redis").unwrap();

    let native = cache.typed::<String>("redis");
    let native_put = native.put("native", "value".to_owned(), CacheOptions::new());
    let verified_put = async {
        server.state().dispatch_verified_request(
            &identity,
            ClientRequestEnvelope::new(
                "parallel-hc2-put",
                ClientRequest::Put {
                    ns: Namespace::new("redis").unwrap(),
                    key: StructuredKey::new(vec!["redis-binary-v1-686332".to_owned()]).unwrap(),
                    value: b"shared".to_vec(),
                    ttl_ms: None,
                    dimensions: Vec::new(),
                },
            ),
        )
    };
    let (native_result, verified_response) = tokio::join!(native_put, verified_put);
    native_result.unwrap();
    assert!(verified_response.result.is_ok());

    let mut messages = pubsub.on_message();
    let mut payloads = BTreeSet::new();
    for _ in 0..2 {
        let message = timeout(Duration::from_secs(2), messages.next())
            .await
            .expect("parallel source event timeout")
            .expect("parallel source stream closed");
        assert_eq!(message.get_channel_name(), "__keyevent@0__:set");
        assert!(payloads.insert(message.get_payload::<String>().unwrap()));
    }
    assert_eq!(
        payloads,
        BTreeSet::from(["hc2".to_owned(), "native".to_owned()])
    );
    assert!(
        timeout(Duration::from_millis(100), messages.next())
            .await
            .is_err(),
        "one mutation on each independent source must not duplicate delivery"
    );
    assert_eq!(server.metrics().event_messages, 2);

    drop(messages);
    drop(pubsub);
    accept.abort();
}

#[tokio::test]
async fn authenticated_redis_client_receives_native_event_only_after_auth() {
    let cache = HydraCache::local().build();
    let server = event_server_with_native(
        RedisAuthConfig::required("event-secret"),
        "redis",
        cache.clone(),
    );
    let (_server, address, accept) = spawn_server(server).await;
    let client = redis::Client::open(format!("redis://:event-secret@{address}/")).unwrap();
    let mut pubsub = client.get_async_pubsub().await.unwrap();
    pubsub.subscribe("__keyevent@0__:set").await.unwrap();

    cache
        .typed::<String>("redis")
        .put("authenticated", "value".to_owned(), CacheOptions::new())
        .await
        .unwrap();
    let event = timeout(Duration::from_secs(2), pubsub.on_message().next())
        .await
        .expect("authenticated native event timeout")
        .expect("authenticated native event stream closed");
    assert_eq!(event.get_channel_name(), "__keyevent@0__:set");
    assert_eq!(event.get_payload::<String>().unwrap(), "authenticated");

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
    spawn_server(event_server(auth)).await
}

async fn spawn_server(
    server: RedisRespServer,
) -> (Arc<RedisRespServer>, std::net::SocketAddr, JoinHandle<()>) {
    let server = Arc::new(server);
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

fn event_server_with_native(
    auth: RedisAuthConfig,
    namespace: &str,
    cache: HydraCache,
) -> RedisRespServer {
    RedisRespServer::new(
        Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap()),
        RedisListenerConfig {
            auth,
            namespace: namespace.to_owned(),
            keyspace_events: RedisKeyspaceEventConfig {
                enabled: true,
                max_subscriptions_per_connection: 8,
                max_subscription_bytes_per_connection: 64 * 1024,
            },
            ..RedisListenerConfig::default()
        },
    )
    .unwrap()
    .with_native_cache_events(cache)
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
