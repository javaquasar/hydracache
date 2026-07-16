use hydracache_redis_compat::{
    decode_resp2_command, decode_resp2_command_with_limits, decode_resp3_command,
    decode_resp3_command_with_limits, encode_resp2_value, encode_resp3_value, RedisCommand,
    RedisCompatError, RespDecodeLimits, RespValue,
};
use proptest::prelude::*;

fn resp_fixture(text: &'static str) -> Vec<u8> {
    text.lines()
        .flat_map(|line| {
            line.as_bytes()
                .iter()
                .copied()
                .chain(b"\r\n".iter().copied())
        })
        .collect()
}

#[test]
fn golden_resp_fixtures_decode_to_expected() {
    let cases = [
        (
            resp_fixture(include_str!("fixtures/commands/get.resp")),
            RedisCommand::Get {
                key: b"key".to_vec(),
            },
        ),
        (
            resp_fixture(include_str!("fixtures/commands/set.resp")),
            RedisCommand::Set {
                key: b"key".to_vec(),
                value: b"value".to_vec(),
                options: Vec::new(),
            },
        ),
        (
            resp_fixture(include_str!("fixtures/commands/mget.resp")),
            RedisCommand::Mget {
                keys: vec![b"a".to_vec(), b"b".to_vec()],
            },
        ),
    ];

    for (bytes, expected) in cases {
        let (command, consumed) = decode_resp2_command(&bytes).unwrap().unwrap();
        assert_eq!(consumed, bytes.len());
        assert_eq!(command, expected);
    }
}

#[test]
fn partial_resp_frames_decode_like_complete_frames() {
    let bytes = resp_fixture(include_str!("fixtures/commands/set.resp"));
    assert!(decode_resp2_command(&bytes[..bytes.len() - 1])
        .unwrap()
        .is_none());

    let (command, consumed) = decode_resp2_command(&bytes).unwrap().unwrap();
    assert_eq!(consumed, bytes.len());
    assert_eq!(
        command,
        RedisCommand::Set {
            key: b"key".to_vec(),
            value: b"value".to_vec(),
            options: Vec::new(),
        }
    );
}

#[test]
fn multiple_resp_frames_in_one_read_are_all_processed() {
    let mut bytes = resp_fixture(include_str!("fixtures/commands/set.resp"));
    bytes.extend(resp_fixture(include_str!("fixtures/commands/get.resp")));

    let mut commands = Vec::new();
    let mut offset = 0;
    while offset < bytes.len() {
        let (command, consumed) = decode_resp2_command(&bytes[offset..]).unwrap().unwrap();
        commands.push(command);
        offset += consumed;
    }

    assert_eq!(commands.len(), 2);
    assert!(matches!(commands[0], RedisCommand::Set { .. }));
    assert!(matches!(commands[1], RedisCommand::Get { .. }));
}

#[test]
fn oversized_bulk_and_array_frames_are_rejected_before_allocation_spike() {
    let bytes = resp_fixture(include_str!("fixtures/commands/set.resp"));
    let frame_error = decode_resp2_command_with_limits(
        &bytes,
        RespDecodeLimits {
            max_frame_bytes: 8,
            ..RespDecodeLimits::default()
        },
    )
    .unwrap_err();
    assert!(matches!(
        frame_error,
        RedisCompatError::FrameTooLarge { .. }
    ));

    let array_error = decode_resp2_command_with_limits(
        &bytes,
        RespDecodeLimits {
            max_array_elements: 2,
            ..RespDecodeLimits::default()
        },
    )
    .unwrap_err();
    assert!(matches!(
        array_error,
        RedisCompatError::ArrayTooLarge { .. }
    ));

    let bulk_error = decode_resp2_command_with_limits(
        &bytes,
        RespDecodeLimits {
            max_bulk_string_bytes: 4,
            ..RespDecodeLimits::default()
        },
    )
    .unwrap_err();
    assert!(matches!(
        bulk_error,
        RedisCompatError::BulkStringTooLarge { .. }
    ));
}

#[test]
fn resp3_null_uses_underscore_encoding_and_resp2_uses_dash_one() {
    assert_eq!(encode_resp2_value(RespValue::Null).unwrap(), b"$-1\r\n");
    assert_eq!(encode_resp3_value(RespValue::Null).unwrap(), b"_\r\n");
}

#[test]
fn resp3_integer_array_binary_bulk_and_error_frames_match_golden_bytes() {
    assert_eq!(
        encode_resp3_value(RespValue::Integer(-2)).unwrap(),
        b":-2\r\n"
    );
    assert_eq!(
        encode_resp3_value(RespValue::BulkString(vec![0, b'\r', b'\n', 0xff])).unwrap(),
        b"$4\r\n\0\r\n\xff\r\n"
    );
    assert_eq!(
        encode_resp3_value(RespValue::Array(vec![
            RespValue::BulkString(b"v".to_vec()),
            RespValue::Null,
            RespValue::Integer(1),
        ]))
        .unwrap(),
        b"*3\r\n$1\r\nv\r\n_\r\n:1\r\n"
    );
    assert_eq!(
        encode_resp3_value(RespValue::Error("ERR syntax error".to_owned())).unwrap(),
        b"-ERR syntax error\r\n"
    );
}

#[test]
fn resp3_partial_pipeline_consumed_and_limit_boundaries_match_resp2() {
    let first = b"*3\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n";
    let second = b"*2\r\n$3\r\nGET\r\n$1\r\nk\r\n";
    assert!(decode_resp3_command(&first[..first.len() - 1])
        .unwrap()
        .is_none());

    let mut pipeline = first.to_vec();
    pipeline.extend_from_slice(second);
    let (set, consumed) = decode_resp3_command(&pipeline).unwrap().unwrap();
    assert!(matches!(set, RedisCommand::Set { .. }));
    assert_eq!(consumed, first.len());
    let (get, consumed) = decode_resp3_command(&pipeline[consumed..])
        .unwrap()
        .unwrap();
    assert!(matches!(get, RedisCommand::Get { .. }));
    assert_eq!(consumed, second.len());

    let limits = RespDecodeLimits {
        max_frame_bytes: 1024,
        max_array_elements: 2,
        max_bulk_string_bytes: 128,
    };
    assert!(matches!(
        decode_resp3_command_with_limits(first, limits),
        Err(RedisCompatError::ArrayTooLarge { actual: 3, max: 2 })
    ));
    let limits = RespDecodeLimits {
        max_frame_bytes: 1024,
        max_array_elements: 8,
        max_bulk_string_bytes: 2,
    };
    assert!(matches!(
        decode_resp3_command_with_limits(second, limits),
        Err(RedisCompatError::BulkStringTooLarge { actual: 3, max: 2 })
    ));
}

#[test]
fn resp3_top_level_nested_and_attributed_aggregates_fail_loud() {
    let attributed = [
        b"|1\r\n+trace\r\n+x\r\n*2\r\n+GET\r\n+k\r\n".as_slice(),
        b"*2\r\n+GET\r\n|1\r\n+trace\r\n+x\r\n+k\r\n".as_slice(),
    ];
    for input in attributed {
        assert!(matches!(
            decode_resp3_command(input),
            Err(RedisCompatError::UnsupportedResp3Attributes)
        ));
    }

    for input in [
        b"%1\r\n+GET\r\n+k\r\n".as_slice(),
        b"~2\r\n+GET\r\n+k\r\n".as_slice(),
        b">2\r\n+GET\r\n+k\r\n".as_slice(),
    ] {
        assert!(matches!(
            decode_resp3_command(input),
            Err(RedisCompatError::NonArrayCommand)
        ));
    }

    for input in [
        b"*2\r\n+GET\r\n%1\r\n+a\r\n+b\r\n".as_slice(),
        b"*2\r\n+GET\r\n~1\r\n+k\r\n".as_slice(),
        b"*2\r\n+GET\r\n>1\r\n+k\r\n".as_slice(),
        b"*2\r\n+GET\r\n*1\r\n+k\r\n".as_slice(),
    ] {
        assert!(matches!(
            decode_resp3_command(input),
            Err(RedisCompatError::NonStringArgument)
        ));
    }
}

proptest! {
    #[test]
    fn resp_decoder_never_panics_on_arbitrary_bytes(input in proptest::collection::vec(any::<u8>(), 0..512)) {
        let _ = decode_resp2_command(&input);
    }

    #[test]
    fn resp3_decoder_never_panics_on_arbitrary_bytes(input in proptest::collection::vec(any::<u8>(), 0..512)) {
        let _ = decode_resp3_command(&input);
    }
}
