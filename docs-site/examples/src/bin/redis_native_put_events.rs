use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt as _;
use hydracache::{CacheOptions, HydraCache};
use hydracache_client_transport_axum::{ClientSurfaceLimits, ClientSurfaceState};
use hydracache_redis_compat::{RedisKeyspaceEventConfig, RedisListenerConfig, RedisRespServer};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ANCHOR: redis-native-put-events
    let cache = HydraCache::local().build();
    let resp = Arc::new(
        RedisRespServer::new(
            Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default())?),
            RedisListenerConfig {
                keyspace_events: RedisKeyspaceEventConfig {
                    enabled: true,
                    ..RedisKeyspaceEventConfig::default()
                },
                ..RedisListenerConfig::default()
            },
        )?
        .with_native_cache_events(cache.clone()),
    );
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let accepting = tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let resp = Arc::clone(&resp);
            tokio::spawn(async move {
                let _ = resp.serve_connection(stream).await;
            });
        }
    });

    // This is a normal Redis client using a normal keyspace subscription.
    let client = redis::Client::open(format!("redis://{address}/"))?;
    let mut pubsub = client.get_async_pubsub().await?;
    pubsub.subscribe("__keyevent@0__:set").await?;

    // The backend writes through the native HydraCache API. The `redis`
    // typed namespace is the explicit, non-leaking bridge to Redis key bytes.
    cache
        .typed::<String>("redis")
        .put("user:42", "Ada".to_owned(), CacheOptions::new())
        .await?;

    let message = tokio::time::timeout(Duration::from_secs(2), pubsub.on_message().next())
        .await?
        .ok_or("Redis subscription closed before the event")?;
    assert_eq!(message.get_channel_name(), "__keyevent@0__:set");
    assert_eq!(message.get_payload::<String>()?, "user:42");

    drop(pubsub);
    accepting.abort();
    // ANCHOR_END: redis-native-put-events
    Ok(())
}
