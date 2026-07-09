//! Redis RESP edge compatibility primitives.
//!
//! This crate owns the parser-neutral command model for the optional Redis
//! facade. The production listener is wired later; W1 keeps the RESP codec
//! dependency contained here so the core and stable HydraCache client protocol
//! remain untouched.

use bytes::{Bytes, BytesMut};
use redis_protocol::resp2::decode::decode_bytes;
use redis_protocol::resp2::encode::extend_encode;
use redis_protocol::resp2::types::BytesFrame;
use thiserror::Error;

/// RESP dialect claimed by the 0.63 edge surface.
pub const SUPPORTED_RESP_DIALECT: &str = "RESP2";

/// Parser-neutral Redis command subset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedisCommand {
    /// `PING [message]`.
    Ping { message: Option<Vec<u8>> },
    /// `ECHO message`.
    Echo { message: Vec<u8> },
    /// `QUIT`.
    Quit,
    /// `HELLO version ...`.
    Hello { version: u8 },
    /// `AUTH [username] password`.
    Auth {
        /// Optional ACL-style username.
        username: Option<Vec<u8>>,
        /// Password or opaque token bytes.
        password: Vec<u8>,
    },
    /// `CLIENT SETNAME name`.
    ClientSetName { name: Vec<u8> },
    /// `CLIENT SETINFO ...`.
    ClientSetInfo { args: Vec<Vec<u8>> },
    /// `COMMAND`.
    Command,
    /// `GET key`.
    Get { key: Vec<u8> },
    /// `SET key value ...`.
    Set {
        /// Binary-safe Redis key.
        key: Vec<u8>,
        /// Opaque value bytes.
        value: Vec<u8>,
        /// Additional SET modifiers retained for W0/W2 semantic validation.
        options: Vec<Vec<u8>>,
    },
    /// `MGET key...`.
    Mget { keys: Vec<Vec<u8>> },
    /// `DEL key...`.
    Del { keys: Vec<Vec<u8>> },
    /// `EXISTS key...`.
    Exists { keys: Vec<Vec<u8>> },
    /// Command outside the accepted W1 parser subset.
    Unsupported { verb: String, args: Vec<Vec<u8>> },
}

/// RESP value helpers used by the future listener.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RespValue {
    /// Simple string response.
    SimpleString(&'static str),
    /// Integer response.
    Integer(i64),
    /// Bulk string response.
    BulkString(Vec<u8>),
    /// Null bulk response.
    Null,
}

/// Codec and command-shape failures.
#[derive(Debug, Error)]
pub enum RedisCompatError {
    /// The underlying RESP codec rejected the bytes.
    #[error("RESP2 decode error: {0}")]
    Decode(String),
    /// The underlying RESP encoder rejected the response.
    #[error("RESP2 encode error: {0}")]
    Encode(String),
    /// Redis commands must be RESP arrays.
    #[error("Redis command must be a RESP array")]
    NonArrayCommand,
    /// Redis command arrays must contain at least the verb.
    #[error("Redis command array is empty")]
    EmptyCommand,
    /// Command arguments must be binary-safe strings.
    #[error("Redis command argument must be a bulk or simple string")]
    NonStringArgument,
    /// Command verb is not valid UTF-8.
    #[error("Redis command verb must be UTF-8")]
    NonUtf8Verb,
}

/// Decode one RESP2 command frame.
pub fn decode_resp2_command(
    input: &[u8],
) -> Result<Option<(RedisCommand, usize)>, RedisCompatError> {
    let bytes = Bytes::copy_from_slice(input);
    let Some((frame, consumed)) =
        decode_bytes(&bytes).map_err(|error| RedisCompatError::Decode(error.to_string()))?
    else {
        return Ok(None);
    };
    let command = command_from_frame(frame)?;
    Ok(Some((command, consumed)))
}

/// Encode a RESP2 response value.
pub fn encode_resp2_value(value: RespValue) -> Result<Vec<u8>, RedisCompatError> {
    let frame = match value {
        RespValue::SimpleString(value) => {
            BytesFrame::SimpleString(Bytes::from_static(value.as_bytes()))
        }
        RespValue::Integer(value) => BytesFrame::Integer(value),
        RespValue::BulkString(value) => BytesFrame::BulkString(Bytes::from(value)),
        RespValue::Null => BytesFrame::Null,
    };
    let mut output = BytesMut::new();
    extend_encode(&mut output, &frame, false)
        .map_err(|error| RedisCompatError::Encode(error.to_string()))?;
    Ok(output.to_vec())
}

fn command_from_frame(frame: BytesFrame) -> Result<RedisCommand, RedisCompatError> {
    let BytesFrame::Array(frames) = frame else {
        return Err(RedisCompatError::NonArrayCommand);
    };
    let mut args = frames
        .into_iter()
        .map(frame_bytes)
        .collect::<Result<Vec<_>, _>>()?;
    if args.is_empty() {
        return Err(RedisCompatError::EmptyCommand);
    }
    let verb = args.remove(0);
    let verb = std::str::from_utf8(&verb).map_err(|_| RedisCompatError::NonUtf8Verb)?;
    let normalized = verb.to_ascii_uppercase();

    Ok(match normalized.as_str() {
        "PING" if args.len() <= 1 => RedisCommand::Ping {
            message: args.into_iter().next(),
        },
        "ECHO" if args.len() == 1 => RedisCommand::Echo {
            message: args.remove(0),
        },
        "QUIT" => RedisCommand::Quit,
        "HELLO" => {
            let version = args
                .first()
                .and_then(|value| std::str::from_utf8(value).ok())
                .and_then(|value| value.parse::<u8>().ok())
                .unwrap_or(0);
            RedisCommand::Hello { version }
        }
        "AUTH" if args.len() == 1 => RedisCommand::Auth {
            username: None,
            password: args.remove(0),
        },
        "AUTH" if args.len() == 2 => RedisCommand::Auth {
            username: Some(args.remove(0)),
            password: args.remove(0),
        },
        "CLIENT" => parse_client_command(args),
        "COMMAND" => RedisCommand::Command,
        "GET" if args.len() == 1 => RedisCommand::Get {
            key: args.remove(0),
        },
        "SET" if args.len() >= 2 => RedisCommand::Set {
            key: args.remove(0),
            value: args.remove(0),
            options: args,
        },
        "MGET" if !args.is_empty() => RedisCommand::Mget { keys: args },
        "DEL" if !args.is_empty() => RedisCommand::Del { keys: args },
        "EXISTS" if !args.is_empty() => RedisCommand::Exists { keys: args },
        _ => RedisCommand::Unsupported {
            verb: normalized,
            args,
        },
    })
}

fn parse_client_command(mut args: Vec<Vec<u8>>) -> RedisCommand {
    let Some(subcommand) = args.first() else {
        return RedisCommand::Unsupported {
            verb: "CLIENT".to_owned(),
            args,
        };
    };
    let subcommand = String::from_utf8_lossy(subcommand).to_ascii_uppercase();
    match subcommand.as_str() {
        "SETNAME" if args.len() == 2 => RedisCommand::ClientSetName {
            name: args.remove(1),
        },
        "SETINFO" => RedisCommand::ClientSetInfo {
            args: args.into_iter().skip(1).collect(),
        },
        _ => RedisCommand::Unsupported {
            verb: format!("CLIENT {subcommand}"),
            args,
        },
    }
}

fn frame_bytes(frame: BytesFrame) -> Result<Vec<u8>, RedisCompatError> {
    match frame {
        BytesFrame::BulkString(bytes) | BytesFrame::SimpleString(bytes) => Ok(bytes.to_vec()),
        _ => Err(RedisCompatError::NonStringArgument),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn facade_advertises_resp2_only_for_this_release() {
        assert_eq!(SUPPORTED_RESP_DIALECT, "RESP2");
    }

    #[test]
    fn resp_frame_roundtrip_matches_redis_protocol() {
        let input = b"*2\r\n$3\r\nGET\r\n$3\r\nkey\r\n";
        let (command, consumed) = decode_resp2_command(input).unwrap().unwrap();
        assert_eq!(consumed, input.len());
        assert_eq!(
            command,
            RedisCommand::Get {
                key: b"key".to_vec()
            }
        );

        let encoded = encode_resp2_value(RespValue::BulkString(b"value".to_vec())).unwrap();
        assert_eq!(encoded, b"$5\r\nvalue\r\n");
    }

    #[test]
    fn command_parser_keeps_set_options_parser_neutral() {
        let input = b"*5\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n$2\r\nEX\r\n$2\r\n10\r\n";
        let (command, _) = decode_resp2_command(input).unwrap().unwrap();
        assert_eq!(
            command,
            RedisCommand::Set {
                key: b"k".to_vec(),
                value: b"v".to_vec(),
                options: vec![b"EX".to_vec(), b"10".to_vec()]
            }
        );
    }

    #[test]
    fn unsupported_command_is_loudly_classified_without_semantic_mapping() {
        let input = b"*3\r\n$4\r\nHSET\r\n$1\r\nk\r\n$1\r\nv\r\n";
        let (command, _) = decode_resp2_command(input).unwrap().unwrap();
        assert_eq!(
            command,
            RedisCommand::Unsupported {
                verb: "HSET".to_owned(),
                args: vec![b"k".to_vec(), b"v".to_vec()]
            }
        );
    }

    #[test]
    fn partial_frame_waits_for_more_bytes() {
        assert!(decode_resp2_command(b"*2\r\n$3\r\nGET\r\n")
            .unwrap()
            .is_none());
    }
}
