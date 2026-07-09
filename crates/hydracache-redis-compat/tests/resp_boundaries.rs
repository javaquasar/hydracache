use hydracache_redis_compat::{
    decode_resp2_command, decode_resp2_command_with_limits, RedisCommand, RedisCompatError,
    RespDecodeLimits,
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

proptest! {
    #[test]
    fn resp_decoder_never_panics_on_arbitrary_bytes(input in proptest::collection::vec(any::<u8>(), 0..512)) {
        let _ = decode_resp2_command(&input);
    }
}
