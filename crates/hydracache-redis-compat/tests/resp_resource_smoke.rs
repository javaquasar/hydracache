use std::sync::Arc;
use std::time::Duration;

use hydracache_client_transport_axum::{ClientSurfaceLimits, ClientSurfaceState};
use hydracache_redis_compat::{
    RedisListenerConfig, RedisListenerMetrics, RedisRespServer, RespDecodeLimits,
    DEFAULT_REDIS_NAMESPACE,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const RESOURCE_SMOKE_ENV: &str = "HYDRACACHE_RUN_REDIS_COMPAT_RESOURCE_SMOKE";

#[test]
fn resource_smoke_gate_manifest_and_docs_are_wired() {
    let gates = include_str!("../../../docs/GATES.md");
    let testing = include_str!("../../../docs/TESTING.md");

    assert!(gates.contains(RESOURCE_SMOKE_ENV));
    assert!(gates.contains("--test resp_resource_smoke"));
    assert!(testing.contains("--test resp_resource_smoke"));
    assert!(gates.contains("slowloris/oversized frame behavior"));
    assert!(gates.contains("no key/value leakage in logs or metrics"));
}

#[test]
fn resource_smoke_heavy_gate_is_executable_and_env_gated() {
    let source = include_str!("resp_resource_smoke.rs");
    let gates = include_str!("../../../docs/GATES.md");
    let testing = include_str!("../../../docs/TESTING.md");

    for test_name in [
        "resp_resource_smoke_bounds_pipelined_connection_and_redacts_extension_output",
        "slowloris_and_oversized_frames_fail_loud_without_mutation",
    ] {
        assert!(source.contains(&format!("async fn {test_name}")));
        assert!(source.contains("#[ignore"));
    }
    assert!(source.contains(RESOURCE_SMOKE_ENV));
    assert!(gates.contains("--test resp_resource_smoke"));
    assert!(testing.contains(RESOURCE_SMOKE_ENV));
}

#[tokio::test]
#[ignore = "requires HYDRACACHE_RUN_REDIS_COMPAT_RESOURCE_SMOKE=1; resource/hostile-input smoke"]
async fn resp_resource_smoke_bounds_pipelined_connection_and_redacts_extension_output() {
    if !env_gate_enabled(RESOURCE_SMOKE_ENV) {
        eprintln!("skipping RESP resource smoke; set {RESOURCE_SMOKE_ENV}=1 to run it");
        return;
    }

    let server = listener(RedisListenerConfig::default());
    let output = exchange(
        &server,
        b"*3\r\n$3\r\nSET\r\n$10\r\nsecret-key\r\n$12\r\nsecret-value\r\n\
          *1\r\n$8\r\nHC.STATS\r\n\
          *1\r\n$14\r\nHC.DIAGNOSTICS\r\n\
          *1\r\n$4\r\nQUIT\r\n",
    )
    .await;
    let output = String::from_utf8(output).unwrap();

    assert!(output.contains("dispatch_attempts"));
    assert!(output.contains("accepted_connections"));
    assert!(output.contains(DEFAULT_REDIS_NAMESPACE));
    assert!(!output.contains("secret-key"));
    assert!(!output.contains("secret-value"));
    assert_eq!(server.metrics().errors, 0);
}

#[tokio::test]
#[ignore = "requires HYDRACACHE_RUN_REDIS_COMPAT_RESOURCE_SMOKE=1; slowloris/oversized frame smoke"]
async fn slowloris_and_oversized_frames_fail_loud_without_mutation() {
    if !env_gate_enabled(RESOURCE_SMOKE_ENV) {
        eprintln!("skipping RESP resource smoke; set {RESOURCE_SMOKE_ENV}=1 to run it");
        return;
    }

    let slowloris = listener(RedisListenerConfig {
        idle_timeout: Duration::from_millis(5),
        ..RedisListenerConfig::default()
    });
    let (_slow_client, slow_server_io) = tokio::io::duplex(64);
    tokio::time::timeout(
        Duration::from_secs(1),
        slowloris.serve_connection(slow_server_io),
    )
    .await
    .unwrap()
    .unwrap();

    let oversized = listener(RedisListenerConfig {
        decode_limits: RespDecodeLimits {
            max_frame_bytes: 8,
            ..RespDecodeLimits::default()
        },
        ..RedisListenerConfig::default()
    });
    let output = exchange(&oversized, b"*1\r\n$4\r\nPING\r\n").await;
    let output = String::from_utf8(output).unwrap();

    assert!(output.contains("ERR RESP frame too large"));
    assert_eq!(oversized.state().state_mutations(), 0);
    assert_eq!(
        oversized.metrics(),
        RedisListenerMetrics {
            accepted_connections: 1,
            commands: 0,
            errors: 1,
        }
    );
}

fn env_gate_enabled(name: &str) -> bool {
    std::env::var(name).is_ok_and(|value| value == "1" || value.eq_ignore_ascii_case("true"))
}

fn listener(config: RedisListenerConfig) -> RedisRespServer {
    RedisRespServer::new(
        Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap()),
        config,
    )
    .unwrap()
}

async fn exchange(server: &RedisRespServer, input: &'static [u8]) -> Vec<u8> {
    let (mut client, server_io) = tokio::io::duplex(4096);
    let serve = async {
        server.serve_connection(server_io).await.unwrap();
    };
    let client = async {
        client.write_all(input).await.unwrap();
        let mut output = Vec::new();
        client.read_to_end(&mut output).await.unwrap();
        output
    };
    let (_, output) = tokio::join!(serve, client);
    output
}
