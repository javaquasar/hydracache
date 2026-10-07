//! Canonical baseline fixtures, committed before the consuming helper exists.
use std::sync::Arc;

use hydracache_client_protocol::{
    ClientErrorCode, ClientErrorEnvelope, ClientResponse, ClientResponseEnvelope,
};
use hydracache_client_transport_axum::{ClientIdentity, ClientSurfaceLimits, ClientSurfaceState};
use hydracache_redis_compat::{
    encode_resp2_value, encode_resp3_value, translate_redis_command, RedisCommand,
    RedisListenerConfig, RedisRespServer, RedisTranslatedCommand, RedisTranslationContext,
    RespValue, DEFAULT_REDIS_NAMESPACE,
};

fn fixture() -> (Arc<ClientSurfaceState>, RedisRespServer) {
    let state = Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap());
    state.set_cache_time_for_tests(Some(1000));
    let server = RedisRespServer::new(Arc::clone(&state), RedisListenerConfig::default()).unwrap();
    (state, server)
}
fn borrowed_oracle(state: &ClientSurfaceState, command: RedisCommand) -> RespValue {
    let identity = ClientIdentity::new("redis-resp", DEFAULT_REDIS_NAMESPACE).unwrap();
    let context = RedisTranslationContext::new(DEFAULT_REDIS_NAMESPACE, "borrowed-oracle").unwrap();
    match translate_redis_command(command, &context).unwrap() {
        RedisTranslatedCommand::Execute(plan) => {
            let mut responses = plan
                .initial_requests()
                .iter()
                .cloned()
                .map(|request| state.dispatch_verified_request(&identity, request))
                .collect::<Vec<_>>();
            let followups = plan.followup_requests(&responses).unwrap();
            responses.extend(
                followups
                    .into_iter()
                    .map(|request| state.dispatch_verified_request(&identity, request)),
            );
            plan.reduce(&responses).unwrap()
        }
        RedisTranslatedCommand::Immediate(value) => value,
        _ => panic!("fixture has no extension commands"),
    }
}
fn set(key: &[u8], value: &[u8]) -> RedisCommand {
    RedisCommand::Set {
        key: key.to_vec(),
        value: value.to_vec(),
        options: Vec::new(),
    }
}
fn get(key: &[u8]) -> RedisCommand {
    RedisCommand::Get { key: key.to_vec() }
}

#[test]
fn public_borrowed_get_preserves_original_owner_values_and_error_details() {
    let RedisTranslatedCommand::Execute(plan) = translate_redis_command(
        get(b"binary\0\xff"),
        &RedisTranslationContext::new(DEFAULT_REDIS_NAMESPACE, "borrowed").unwrap(),
    )
    .unwrap() else {
        panic!()
    };
    for size in [0, 64, 4096, 1048576] {
        let responses = vec![ClientResponseEnvelope::ok(
            "response",
            ClientResponse::Value {
                value: Some(vec![0xff; size]),
            },
        )];
        let original = responses.clone();
        let value = plan.reduce(&responses).unwrap();
        assert_eq!(responses, original);
        let RespValue::BulkString(value) = value else {
            panic!()
        };
        let Ok(ClientResponse::Value {
            value: Some(source),
        }) = &responses[0].result
        else {
            panic!()
        };
        assert_eq!(value, *source);
        if size != 0 {
            assert_ne!(value.as_ptr(), source.as_ptr());
        }
    }
    assert_eq!(
        plan.reduce(&[ClientResponseEnvelope::ok(
            "miss",
            ClientResponse::Value { value: None }
        )])
        .unwrap(),
        RespValue::Null
    );
    for count in [0, 2, 3] {
        let responses = vec![ClientResponseEnvelope::ok("wrong", ClientResponse::Stored); count];
        let error = plan.reduce(&responses).unwrap_err();
        assert!(error
            .to_string()
            .contains(&format!("expected one response, got {count}")));
    }
    assert!(plan
        .reduce(&[ClientResponseEnvelope::ok("wrong", ClientResponse::Stored)])
        .unwrap_err()
        .to_string()
        .contains("unexpected response kind: stored"));
    let response = ClientResponseEnvelope::error(
        "denied",
        ClientErrorEnvelope::new(ClientErrorCode::Unauthorized, false, "denied"),
    );
    assert!(matches!(
        plan.reduce(&[response]).unwrap(),
        RespValue::Error(_)
    ));
}

#[test]
fn native_get_response_capacity_matches_payload_len_for_admitted_sizes() {
    for size in [0, 1, 64, 4096, 1048576] {
        let (state, server) = fixture();
        assert_eq!(
            server.execute_command(set(b"key", &vec![0xff; size])),
            RespValue::SimpleString("OK")
        );
        let context =
            RedisTranslationContext::new(DEFAULT_REDIS_NAMESPACE, "native-capacity").unwrap();
        let RedisTranslatedCommand::Execute(plan) =
            translate_redis_command(get(b"key"), &context).unwrap()
        else {
            panic!()
        };
        let identity = ClientIdentity::new("redis-resp", DEFAULT_REDIS_NAMESPACE).unwrap();
        let response =
            state.dispatch_verified_request(&identity, plan.initial_requests()[0].clone());
        let Ok(ClientResponse::Value { value: Some(value) }) = response.result else {
            panic!()
        };
        assert_eq!(value.len(), size);
        assert_eq!(value.capacity(), value.len());
    }
}

#[test]
fn execution_matches_borrowed_oracle_for_binary_empty_missing_and_large_get() {
    for size in [0, 1, 64, 4096, 1048576] {
        let (actual_state, server) = fixture();
        let (oracle, _) = fixture();
        let key = b"binary\0\xffkey";
        let value = (0..size).map(|i| (i % 256) as u8).collect::<Vec<_>>();
        for command in [set(key, &value), get(key), get(b"missing")] {
            let expected = borrowed_oracle(&oracle, command.clone());
            let actual = server.execute_command(command);
            assert_eq!(actual, expected);
            assert_eq!(
                encode_resp2_value(actual.clone()).unwrap(),
                encode_resp2_value(expected.clone()).unwrap()
            );
            assert_eq!(
                encode_resp3_value(actual).unwrap(),
                encode_resp3_value(expected).unwrap()
            );
        }
        assert_eq!(actual_state.dispatch_attempts(), oracle.dispatch_attempts());
        assert_eq!(actual_state.state_mutations(), oracle.state_mutations());
        assert_eq!(
            actual_state.retained_state_for_diagnostics().store_entries,
            1
        );
        assert_eq!(oracle.retained_state_for_diagnostics().store_entries, 1);
    }
}

#[test]
fn retained_get_response_survives_replacement_expiry_and_removal() {
    let (state, server) = fixture();
    let value = vec![0xff; 4096];
    assert_eq!(
        server.execute_command(set(b"key", &value)),
        RespValue::SimpleString("OK")
    );
    let response = server.execute_command(get(b"key"));
    let with_expiry = RedisCommand::Set {
        key: b"key".to_vec(),
        value: b"replacement".to_vec(),
        options: vec![b"PX".to_vec(), b"10".to_vec()],
    };
    assert_eq!(
        server.execute_command(with_expiry),
        RespValue::SimpleString("OK")
    );
    state.advance_cache_time_for_tests(10);
    assert_eq!(server.execute_command(get(b"key")), RespValue::Null);
    assert_eq!(
        server.execute_command(RedisCommand::Del {
            keys: vec![b"key".to_vec()]
        }),
        RespValue::Integer(0)
    );
    assert_eq!(response, RespValue::BulkString(value));
    assert_eq!(state.retained_state_for_diagnostics().store_entries, 0);
}

#[test]
fn multi_key_duplicate_order_and_counts_match_canonical_borrowed_oracle() {
    let (actual_state, server) = fixture();
    let (oracle, _) = fixture();
    let commands = [
        RedisCommand::Mset {
            entries: vec![
                (b"a".to_vec(), b"first".to_vec()),
                (b"b".to_vec(), vec![0xff; 4096]),
                (b"a".to_vec(), b"last".to_vec()),
            ],
        },
        get(b"a"),
        RedisCommand::Mget {
            keys: vec![
                b"b".to_vec(),
                b"missing".to_vec(),
                b"b".to_vec(),
                b"a".to_vec(),
            ],
        },
        RedisCommand::Exists {
            keys: vec![b"a".to_vec(), b"a".to_vec(), b"missing".to_vec()],
        },
        RedisCommand::Del {
            keys: vec![b"a".to_vec(), b"a".to_vec(), b"missing".to_vec()],
        },
        get(b"a"),
        get(b"b"),
    ];
    for command in commands {
        let expected = borrowed_oracle(&oracle, command.clone());
        assert_eq!(server.execute_command(command), expected);
    }
    assert_eq!(actual_state.dispatch_attempts(), oracle.dispatch_attempts());
    assert_eq!(actual_state.state_mutations(), oracle.state_mutations());
    assert_eq!(
        actual_state.retained_state_for_diagnostics().store_entries,
        1
    );
}
