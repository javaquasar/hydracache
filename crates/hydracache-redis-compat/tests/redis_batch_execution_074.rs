use std::sync::Arc;

use hydracache::{
    ConsumerIsolation, ConsumerIsolationConfig, NamespaceQuota, Tenant, TenantRoster,
};
use hydracache_client_transport_axum::{
    ClientIdentity, ClientSurfaceLimits, ClientSurfaceMutationKind, ClientSurfaceState,
};
use hydracache_redis_compat::{
    RedisCommand, RedisListenerConfig, RedisRespServer, RespValue, DEFAULT_REDIS_NAMESPACE,
};

fn listener(limits: ClientSurfaceLimits) -> RedisRespServer {
    RedisRespServer::new(
        Arc::new(ClientSurfaceState::new(limits).unwrap()),
        RedisListenerConfig::default(),
    )
    .unwrap()
}

fn isolated_listener(max_bytes: u64, max_entries: u64) -> RedisRespServer {
    let roster = TenantRoster::new(vec![Tenant::new(DEFAULT_REDIS_NAMESPACE)
        .unwrap()
        .allow_client("redis-resp")
        .namespace(
            DEFAULT_REDIS_NAMESPACE,
            NamespaceQuota::new(max_bytes, max_entries),
        )])
    .unwrap();
    let isolation = ConsumerIsolation::new(roster, ConsumerIsolationConfig::default());
    let state =
        ClientSurfaceState::with_isolation(ClientSurfaceLimits::default(), isolation).unwrap();
    RedisRespServer::new(Arc::new(state), RedisListenerConfig::default()).unwrap()
}

fn set(server: &RedisRespServer, key: &[u8], value: &[u8], options: Vec<Vec<u8>>) -> RespValue {
    server.execute_command(RedisCommand::Set {
        key: key.to_vec(),
        value: value.to_vec(),
        options,
    })
}

fn get(server: &RedisRespServer, key: &[u8]) -> RespValue {
    server.execute_command(RedisCommand::Get { key: key.to_vec() })
}

#[test]
fn del_deduplicates_keys_counts_only_live_removals_and_accepts_binary_keys() {
    let server = listener(ClientSurfaceLimits::default());
    let binary = b"\0\xffbinary\r\n";
    assert_eq!(
        set(&server, b"a", b"one", Vec::new()),
        RespValue::SimpleString("OK")
    );
    assert_eq!(
        set(&server, binary, b"two", Vec::new()),
        RespValue::SimpleString("OK")
    );

    assert_eq!(
        server.execute_command(RedisCommand::Del {
            keys: vec![
                b"a".to_vec(),
                b"missing".to_vec(),
                b"a".to_vec(),
                binary.to_vec(),
            ],
        }),
        RespValue::Integer(2)
    );
    assert_eq!(get(&server, b"a"), RespValue::Null);
    assert_eq!(get(&server, binary), RespValue::Null);
}

#[test]
fn del_does_not_count_expired_entries() {
    let server = listener(ClientSurfaceLimits::default());
    server.state().set_cache_time_for_tests(Some(1_000));
    assert_eq!(
        set(
            &server,
            b"expired",
            b"value",
            vec![b"PX".to_vec(), b"10".to_vec()],
        ),
        RespValue::SimpleString("OK")
    );
    server.state().advance_cache_time_for_tests(11);

    assert_eq!(
        server.execute_command(RedisCommand::Del {
            keys: vec![b"expired".to_vec()],
        }),
        RespValue::Integer(0)
    );
    assert_eq!(
        server
            .state()
            .retained_state_for_diagnostics()
            .store_entries,
        0
    );
}

#[test]
fn rejected_oversized_del_is_atomic() {
    let server = listener(ClientSurfaceLimits {
        max_batch_entries: 2,
        ..ClientSurfaceLimits::default()
    });
    for key in [b"a".as_slice(), b"b", b"c"] {
        assert_eq!(
            set(&server, key, b"v", Vec::new()),
            RespValue::SimpleString("OK")
        );
    }

    let response = server.execute_command(RedisCommand::Del {
        keys: vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()],
    });
    assert!(
        matches!(&response, RespValue::Error(message) if message.contains("request too large")),
        "unexpected oversized DEL response: {response:?}"
    );
    for key in [b"a".as_slice(), b"b", b"c"] {
        assert_eq!(get(&server, key), RespValue::BulkString(b"v".to_vec()));
    }
}

#[test]
fn del_releases_tenant_quota_for_the_whole_batch() {
    let server = isolated_listener(2, 2);
    assert_eq!(
        set(&server, b"a", b"1", Vec::new()),
        RespValue::SimpleString("OK")
    );
    assert_eq!(
        set(&server, b"b", b"2", Vec::new()),
        RespValue::SimpleString("OK")
    );
    assert_eq!(
        server.execute_command(RedisCommand::Del {
            keys: vec![b"a".to_vec(), b"b".to_vec()],
        }),
        RespValue::Integer(2)
    );
    assert_eq!(
        set(&server, b"c", b"34", Vec::new()),
        RespValue::SimpleString("OK")
    );
}

#[tokio::test]
async fn del_emits_one_invalidation_per_removed_key_in_input_order() {
    let server = listener(ClientSurfaceLimits::default());
    let identity = ClientIdentity::new("redis-resp", DEFAULT_REDIS_NAMESPACE).unwrap();
    let mut events = server.state().subscribe_mutations(&identity).unwrap();
    for key in [b"c".as_slice(), b"a", b"b"] {
        assert_eq!(
            set(&server, key, b"v", Vec::new()),
            RespValue::SimpleString("OK")
        );
        let event = events.recv().await.unwrap();
        assert_eq!(event.kind(), ClientSurfaceMutationKind::Stored);
    }

    assert_eq!(
        server.execute_command(RedisCommand::Del {
            keys: vec![
                b"c".to_vec(),
                b"missing".to_vec(),
                b"a".to_vec(),
                b"b".to_vec()
            ],
        }),
        RespValue::Integer(3)
    );
    let mut observed = Vec::new();
    for _ in 0..3 {
        let event = events.recv().await.unwrap();
        assert_eq!(event.kind(), ClientSurfaceMutationKind::Invalidated);
        observed.push(event.key().unwrap().stable_key());
    }
    assert_eq!(
        observed,
        [
            "redis-binary-v1-63",
            "redis-binary-v1-61",
            "redis-binary-v1-62"
        ]
    );
}
