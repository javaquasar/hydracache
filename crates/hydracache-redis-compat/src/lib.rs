//! Redis RESP edge compatibility primitives.
//!
//! This crate owns the parser-neutral command model for the optional Redis
//! facade. The production listener is wired later; W1 keeps the RESP codec
//! dependency contained here so the core and stable HydraCache client protocol
//! remain untouched.

use bytes::{Bytes, BytesMut};
use hydracache_client_protocol::{
    ClientErrorEnvelope, ClientRequest, ClientRequestEnvelope, ClientResponse,
    ClientResponseEnvelope, Namespace, StructuredKey,
};
use redis_protocol::resp2::decode::decode_bytes;
use redis_protocol::resp2::encode::extend_encode;
use redis_protocol::resp2::types::BytesFrame;
use thiserror::Error;

/// RESP dialect claimed by the 0.63 edge surface.
pub const SUPPORTED_RESP_DIALECT: &str = "RESP2";

/// Default namespace used when no explicit RESP namespace mapping exists.
pub const DEFAULT_REDIS_NAMESPACE: &str = "redis";

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
    /// `HC.STATS`.
    HcStats,
    /// `HC.DIAGNOSTICS`.
    HcDiagnostics,
    /// `HC.INVALIDATE key`.
    HcInvalidate { key: Vec<u8> },
    /// `HC.*` command that is intentionally not enabled yet.
    HcCandidate { command: String, args: Vec<Vec<u8>> },
    /// Health/readiness command recognized but not yet claimed.
    HealthProbeCandidate { command: String, args: Vec<Vec<u8>> },
    /// Dangerous or administrative Redis command disabled by default.
    AdminDisabled { command: String, args: Vec<Vec<u8>> },
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
    /// Array response.
    Array(Vec<RespValue>),
    /// Null bulk response.
    Null,
    /// RESP error response with a stable, redacted message.
    Error(String),
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
    let frame = resp_value_to_frame(value);
    let mut output = BytesMut::new();
    extend_encode(&mut output, &frame, false)
        .map_err(|error| RedisCompatError::Encode(error.to_string()))?;
    Ok(output.to_vec())
}

fn resp_value_to_frame(value: RespValue) -> BytesFrame {
    match value {
        RespValue::SimpleString(value) => {
            BytesFrame::SimpleString(Bytes::from_static(value.as_bytes()))
        }
        RespValue::Integer(value) => BytesFrame::Integer(value),
        RespValue::BulkString(value) => BytesFrame::BulkString(Bytes::from(value)),
        RespValue::Array(values) => BytesFrame::Array(
            values
                .into_iter()
                .map(resp_value_to_frame)
                .collect::<Vec<_>>(),
        ),
        RespValue::Null => BytesFrame::Null,
        RespValue::Error(value) => BytesFrame::Error(value.into()),
    }
}

/// Per-command translation context for the RESP edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedisTranslationContext {
    namespace: Namespace,
    request_id: String,
}

impl RedisTranslationContext {
    /// Create a translation context for one Redis command.
    pub fn new(
        namespace: impl Into<String>,
        request_id: impl Into<String>,
    ) -> Result<Self, RedisTranslationError> {
        let namespace =
            Namespace::new(namespace).map_err(|error| RedisTranslationError::Protocol {
                detail: error.to_string(),
            })?;
        let request_id = request_id.into();
        if request_id.trim().is_empty() {
            return Err(RedisTranslationError::Protocol {
                detail: "request_id must not be empty".to_owned(),
            });
        }
        Ok(Self {
            namespace,
            request_id,
        })
    }

    /// Namespace applied to translated HydraCache client requests.
    pub fn namespace(&self) -> &Namespace {
        &self.namespace
    }

    /// Stable request id prefix for generated HydraCache envelopes.
    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    fn request_id_for(&self, suffix: impl std::fmt::Display) -> String {
        format!("{}-{suffix}", self.request_id)
    }
}

impl Default for RedisTranslationContext {
    fn default() -> Self {
        Self {
            namespace: Namespace::new(DEFAULT_REDIS_NAMESPACE)
                .expect("default Redis namespace is valid"),
            request_id: "redis-resp".to_owned(),
        }
    }
}

/// Redis command translation failures before HydraCache dispatch.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RedisTranslationError {
    /// Command is outside the release compatibility contract.
    #[error("ERR unsupported command {command}")]
    UnsupportedCommand {
        /// Unsupported command name.
        command: String,
    },
    /// RESP dialect is not supported by this release.
    #[error("NOPROTO unsupported RESP protocol version {version}")]
    UnsupportedRespDialect {
        /// Requested RESP protocol version.
        version: u8,
    },
    /// Command shape is not yet safely translatable.
    #[error("ERR {detail}")]
    UnsupportedShape {
        /// Stable redacted detail.
        detail: &'static str,
    },
    /// Command is recognized but remains candidate until native support lands.
    #[error("ERR {command} is candidate and not enabled in this release")]
    CandidateCommand {
        /// Candidate command name.
        command: String,
    },
    /// Command is intentionally disabled unless a future admin gate enables it.
    #[error("NOPERM {command} is disabled by the HydraCache Redis facade")]
    AdminDisabled {
        /// Disabled command name.
        command: String,
    },
    /// HydraCache protocol mapping failed before dispatch.
    #[error("ERR protocol mapping failed: {detail}")]
    Protocol {
        /// Stable redacted detail.
        detail: String,
    },
    /// HydraCache returned a response shape the reducer did not expect.
    #[error("ERR unexpected HydraCache response: {detail}")]
    UnexpectedClientResponse {
        /// Stable redacted detail.
        detail: String,
    },
}

impl RedisTranslationError {
    /// Convert this failure into a RESP error value.
    pub fn into_resp_value(self) -> RespValue {
        RespValue::Error(self.to_string())
    }
}

/// Translation result for one Redis command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedisTranslatedCommand {
    /// Command can be answered without HydraCache state mutation/read.
    Immediate(RespValue),
    /// Command must execute through the HydraCache client surface.
    Execute(RedisExecutionPlan),
    /// HydraCache-only extension that the host listener must satisfy from tenant-scoped data.
    Extension(RedisExtensionRequest),
}

/// HydraCache-only extension requests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedisExtensionRequest {
    /// Tenant-scoped bounded stats snapshot.
    Stats,
    /// Tenant-scoped bounded diagnostics snapshot.
    Diagnostics,
}

/// HydraCache client-surface execution plan for one Redis command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedisExecutionPlan {
    initial_requests: Vec<ClientRequestEnvelope>,
    followup: RedisFollowup,
    reducer: RedisResponseReducer,
}

impl RedisExecutionPlan {
    /// Initial requests to dispatch through `ClientSurfaceState`.
    pub fn initial_requests(&self) -> &[ClientRequestEnvelope] {
        &self.initial_requests
    }

    /// Build follow-up requests after initial responses have been dispatched.
    pub fn followup_requests(
        &self,
        responses: &[ClientResponseEnvelope],
    ) -> Result<Vec<ClientRequestEnvelope>, RedisTranslationError> {
        match &self.followup {
            RedisFollowup::None => Ok(Vec::new()),
            RedisFollowup::InvalidateExisting {
                namespace,
                keys,
                request_id,
            } => {
                let Some(initial_response) = responses.first() else {
                    return Err(RedisTranslationError::UnexpectedClientResponse {
                        detail: "missing initial DEL lookup response".to_owned(),
                    });
                };
                if initial_response.result.is_err() {
                    return Ok(Vec::new());
                }
                let values = batch_values(initial_response, keys.len())?;
                Ok(values
                    .into_iter()
                    .zip(keys.iter())
                    .enumerate()
                    .filter_map(|(index, (value, key))| {
                        value.map(|_| {
                            ClientRequestEnvelope::new(
                                format!("{request_id}-invalidate-{index}"),
                                ClientRequest::Invalidate {
                                    ns: namespace.clone(),
                                    key: key.clone(),
                                },
                            )
                        })
                    })
                    .collect())
            }
        }
    }

    /// Reduce HydraCache responses into the final RESP value.
    pub fn reduce(
        &self,
        responses: &[ClientResponseEnvelope],
    ) -> Result<RespValue, RedisTranslationError> {
        self.reducer.reduce(responses)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RedisFollowup {
    None,
    InvalidateExisting {
        namespace: Namespace,
        keys: Vec<StructuredKey>,
        request_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RedisResponseReducer {
    Get,
    Set,
    Mget { expected_items: usize },
    Del { expected_items: usize },
    Exists { expected_items: usize },
    Invalidate,
}

impl RedisResponseReducer {
    fn reduce(
        &self,
        responses: &[ClientResponseEnvelope],
    ) -> Result<RespValue, RedisTranslationError> {
        match self {
            Self::Get => reduce_get(responses),
            Self::Set => reduce_set(responses),
            Self::Mget { expected_items } => reduce_mget(responses, *expected_items),
            Self::Del { expected_items } => reduce_del(responses, *expected_items),
            Self::Exists { expected_items } => reduce_exists(responses, *expected_items),
            Self::Invalidate => reduce_invalidate(responses),
        }
    }
}

/// Translate one parsed Redis command into an immediate response or HydraCache execution plan.
pub fn translate_redis_command(
    command: RedisCommand,
    context: &RedisTranslationContext,
) -> Result<RedisTranslatedCommand, RedisTranslationError> {
    Ok(match command {
        RedisCommand::Ping { message } => RedisTranslatedCommand::Immediate(match message {
            Some(message) => RespValue::BulkString(message),
            None => RespValue::SimpleString("PONG"),
        }),
        RedisCommand::Echo { message } => {
            RedisTranslatedCommand::Immediate(RespValue::BulkString(message))
        }
        RedisCommand::Quit => RedisTranslatedCommand::Immediate(RespValue::SimpleString("OK")),
        RedisCommand::Hello { version: 2 } => {
            RedisTranslatedCommand::Immediate(resp2_hello_response())
        }
        RedisCommand::Hello { version } => {
            return Err(RedisTranslationError::UnsupportedRespDialect { version });
        }
        RedisCommand::Auth { .. } => {
            return Err(RedisTranslationError::UnsupportedCommand {
                command: "AUTH".to_owned(),
            });
        }
        RedisCommand::ClientSetName { .. } | RedisCommand::ClientSetInfo { .. } => {
            RedisTranslatedCommand::Immediate(RespValue::SimpleString("OK"))
        }
        RedisCommand::Command => RedisTranslatedCommand::Immediate(command_metadata_response()),
        RedisCommand::Get { key } => RedisTranslatedCommand::Execute(single_request_plan(
            context,
            "get",
            ClientRequest::Get {
                ns: context.namespace.clone(),
                key: redis_key_to_structured_key(&key)?,
            },
            RedisResponseReducer::Get,
        )),
        RedisCommand::Set {
            key,
            value,
            options,
        } => {
            if !options.is_empty() {
                return Err(RedisTranslationError::UnsupportedShape {
                    detail: "SET options are candidate until TTL and option semantics are gated",
                });
            }
            RedisTranslatedCommand::Execute(single_request_plan(
                context,
                "set",
                ClientRequest::Put {
                    ns: context.namespace.clone(),
                    key: redis_key_to_structured_key(&key)?,
                    value,
                    ttl_ms: None,
                    dimensions: Vec::new(),
                },
                RedisResponseReducer::Set,
            ))
        }
        RedisCommand::Mget { keys } => {
            let structured_keys = redis_keys_to_structured_keys(keys)?;
            let expected_items = structured_keys.len();
            RedisTranslatedCommand::Execute(single_request_plan(
                context,
                "mget",
                ClientRequest::BatchGet {
                    ns: context.namespace.clone(),
                    keys: structured_keys,
                },
                RedisResponseReducer::Mget { expected_items },
            ))
        }
        RedisCommand::Del { keys } => {
            let structured_keys = dedupe_structured_keys(redis_keys_to_structured_keys(keys)?);
            let expected_items = structured_keys.len();
            RedisTranslatedCommand::Execute(RedisExecutionPlan {
                initial_requests: vec![ClientRequestEnvelope::new(
                    context.request_id_for("del-lookup"),
                    ClientRequest::BatchGet {
                        ns: context.namespace.clone(),
                        keys: structured_keys.clone(),
                    },
                )],
                followup: RedisFollowup::InvalidateExisting {
                    namespace: context.namespace.clone(),
                    keys: structured_keys,
                    request_id: context.request_id_for("del"),
                },
                reducer: RedisResponseReducer::Del { expected_items },
            })
        }
        RedisCommand::Exists { keys } => {
            let structured_keys = redis_keys_to_structured_keys(keys)?;
            let expected_items = structured_keys.len();
            RedisTranslatedCommand::Execute(single_request_plan(
                context,
                "exists",
                ClientRequest::BatchGet {
                    ns: context.namespace.clone(),
                    keys: structured_keys,
                },
                RedisResponseReducer::Exists { expected_items },
            ))
        }
        RedisCommand::HcStats => RedisTranslatedCommand::Extension(RedisExtensionRequest::Stats),
        RedisCommand::HcDiagnostics => {
            RedisTranslatedCommand::Extension(RedisExtensionRequest::Diagnostics)
        }
        RedisCommand::HcInvalidate { key } => RedisTranslatedCommand::Execute(single_request_plan(
            context,
            "hc-invalidate",
            ClientRequest::Invalidate {
                ns: context.namespace.clone(),
                key: redis_key_to_structured_key(&key)?,
            },
            RedisResponseReducer::Invalidate,
        )),
        RedisCommand::HcCandidate { command, .. } => {
            return Err(RedisTranslationError::CandidateCommand { command });
        }
        RedisCommand::HealthProbeCandidate { command, .. } => {
            return Err(RedisTranslationError::CandidateCommand { command });
        }
        RedisCommand::AdminDisabled { command, .. } => {
            return Err(RedisTranslationError::AdminDisabled { command });
        }
        RedisCommand::Unsupported { verb, .. } => {
            return Err(RedisTranslationError::UnsupportedCommand { command: verb });
        }
    })
}

fn single_request_plan(
    context: &RedisTranslationContext,
    suffix: &'static str,
    request: ClientRequest,
    reducer: RedisResponseReducer,
) -> RedisExecutionPlan {
    RedisExecutionPlan {
        initial_requests: vec![ClientRequestEnvelope::new(
            context.request_id_for(suffix),
            request,
        )],
        followup: RedisFollowup::None,
        reducer,
    }
}

fn reduce_get(responses: &[ClientResponseEnvelope]) -> Result<RespValue, RedisTranslationError> {
    let response = single_response(responses)?;
    match &response.result {
        Ok(ClientResponse::Value { value }) => Ok(optional_bulk(value.clone())),
        Err(error) => Ok(client_error_to_resp(error)),
        Ok(other) => unexpected_response(other),
    }
}

fn reduce_set(responses: &[ClientResponseEnvelope]) -> Result<RespValue, RedisTranslationError> {
    let response = single_response(responses)?;
    match &response.result {
        Ok(ClientResponse::Stored) => Ok(RespValue::SimpleString("OK")),
        Err(error) => Ok(client_error_to_resp(error)),
        Ok(other) => unexpected_response(other),
    }
}

fn reduce_mget(
    responses: &[ClientResponseEnvelope],
    expected_items: usize,
) -> Result<RespValue, RedisTranslationError> {
    let response = single_response(responses)?;
    if let Err(error) = &response.result {
        return Ok(client_error_to_resp(error));
    }
    let values = batch_values(response, expected_items)?;
    Ok(RespValue::Array(
        values.into_iter().map(optional_bulk).collect(),
    ))
}

fn reduce_del(
    responses: &[ClientResponseEnvelope],
    expected_items: usize,
) -> Result<RespValue, RedisTranslationError> {
    let Some(initial_response) = responses.first() else {
        return Err(RedisTranslationError::UnexpectedClientResponse {
            detail: "missing DEL lookup response".to_owned(),
        });
    };
    if let Err(error) = &initial_response.result {
        return Ok(client_error_to_resp(error));
    }
    let values = batch_values(initial_response, expected_items)?;
    let deleted = values.iter().filter(|value| value.is_some()).count();
    if responses.len() != deleted + 1 {
        return Err(RedisTranslationError::UnexpectedClientResponse {
            detail: "DEL follow-up response count mismatch".to_owned(),
        });
    }
    for response in &responses[1..] {
        match &response.result {
            Ok(ClientResponse::Invalidated) => {}
            Err(error) => return Ok(client_error_to_resp(error)),
            Ok(other) => return unexpected_response(other),
        }
    }
    Ok(RespValue::Integer(deleted as i64))
}

fn reduce_exists(
    responses: &[ClientResponseEnvelope],
    expected_items: usize,
) -> Result<RespValue, RedisTranslationError> {
    let response = single_response(responses)?;
    if let Err(error) = &response.result {
        return Ok(client_error_to_resp(error));
    }
    let values = batch_values(response, expected_items)?;
    Ok(RespValue::Integer(
        values.iter().filter(|value| value.is_some()).count() as i64,
    ))
}

fn reduce_invalidate(
    responses: &[ClientResponseEnvelope],
) -> Result<RespValue, RedisTranslationError> {
    let response = single_response(responses)?;
    match &response.result {
        Ok(ClientResponse::Invalidated) => Ok(RespValue::SimpleString("OK")),
        Err(error) => Ok(client_error_to_resp(error)),
        Ok(other) => unexpected_response(other),
    }
}

fn single_response(
    responses: &[ClientResponseEnvelope],
) -> Result<&ClientResponseEnvelope, RedisTranslationError> {
    match responses {
        [response] => Ok(response),
        _ => Err(RedisTranslationError::UnexpectedClientResponse {
            detail: format!("expected one response, got {}", responses.len()),
        }),
    }
}

fn batch_values(
    response: &ClientResponseEnvelope,
    expected_items: usize,
) -> Result<Vec<Option<Vec<u8>>>, RedisTranslationError> {
    let Ok(ClientResponse::Batch { items }) = &response.result else {
        return Err(RedisTranslationError::UnexpectedClientResponse {
            detail: "expected batch response".to_owned(),
        });
    };
    if items.len() != expected_items {
        return Err(RedisTranslationError::UnexpectedClientResponse {
            detail: format!("expected {expected_items} batch items, got {}", items.len()),
        });
    }
    let mut values = vec![None; expected_items];
    for item in items {
        if item.index >= expected_items {
            return Err(RedisTranslationError::UnexpectedClientResponse {
                detail: "batch item index out of range".to_owned(),
            });
        }
        let value = match &item.result {
            Ok(value) => value.clone(),
            Err(error) => return Err(client_error_to_unexpected(error)),
        };
        values[item.index] = Some(value);
    }
    values
        .into_iter()
        .map(|value| {
            value.ok_or_else(|| RedisTranslationError::UnexpectedClientResponse {
                detail: "missing batch item index".to_owned(),
            })
        })
        .collect()
}

fn optional_bulk(value: Option<Vec<u8>>) -> RespValue {
    match value {
        Some(value) => RespValue::BulkString(value),
        None => RespValue::Null,
    }
}

fn unexpected_response<T>(response: &ClientResponse) -> Result<T, RedisTranslationError> {
    Err(RedisTranslationError::UnexpectedClientResponse {
        detail: format!("{response:?}"),
    })
}

fn client_error_to_resp(error: &ClientErrorEnvelope) -> RespValue {
    RespValue::Error(format!("ERR HydraCache client error: {}", error.message))
}

fn client_error_to_unexpected(error: &ClientErrorEnvelope) -> RedisTranslationError {
    RedisTranslationError::UnexpectedClientResponse {
        detail: format!("client error in batch response: {}", error.message),
    }
}

fn resp2_hello_response() -> RespValue {
    RespValue::Array(vec![
        bulk_static("server"),
        bulk_static("hydracache"),
        bulk_static("version"),
        bulk_static(env!("CARGO_PKG_VERSION")),
        bulk_static("proto"),
        RespValue::Integer(2),
        bulk_static("id"),
        RespValue::Integer(0),
        bulk_static("mode"),
        bulk_static("standalone"),
        bulk_static("role"),
        bulk_static("master"),
        bulk_static("modules"),
        RespValue::Array(Vec::new()),
    ])
}

fn command_metadata_response() -> RespValue {
    RespValue::Array(vec![
        command_metadata("ping", -1, &["fast"], 0, 0, 0),
        command_metadata("echo", 2, &["fast"], 0, 0, 0),
        command_metadata("quit", 1, &["fast"], 0, 0, 0),
        command_metadata("hello", -2, &["fast"], 0, 0, 0),
        command_metadata("client", -2, &["fast"], 0, 0, 0),
        command_metadata("command", 1, &["readonly", "fast"], 0, 0, 0),
        command_metadata("get", 2, &["readonly", "fast"], 1, 1, 1),
        command_metadata("set", -3, &["write"], 1, 1, 1),
        command_metadata("mget", -2, &["readonly", "fast"], 1, -1, 1),
        command_metadata("del", -2, &["write"], 1, -1, 1),
        command_metadata("exists", -2, &["readonly", "fast"], 1, -1, 1),
    ])
}

fn command_metadata(
    name: &'static str,
    arity: i64,
    flags: &[&'static str],
    first_key: i64,
    last_key: i64,
    key_step: i64,
) -> RespValue {
    RespValue::Array(vec![
        bulk_static(name),
        RespValue::Integer(arity),
        RespValue::Array(flags.iter().copied().map(bulk_static).collect()),
        RespValue::Integer(first_key),
        RespValue::Integer(last_key),
        RespValue::Integer(key_step),
    ])
}

fn bulk_static(value: &'static str) -> RespValue {
    RespValue::BulkString(value.as_bytes().to_vec())
}

fn redis_keys_to_structured_keys(
    keys: Vec<Vec<u8>>,
) -> Result<Vec<StructuredKey>, RedisTranslationError> {
    keys.iter()
        .map(|key| redis_key_to_structured_key(key))
        .collect()
}

fn redis_key_to_structured_key(key: &[u8]) -> Result<StructuredKey, RedisTranslationError> {
    let encoded = if key.is_empty() {
        "redis-binary-v1-empty".to_owned()
    } else {
        let mut encoded = String::with_capacity("redis-binary-v1-".len() + key.len() * 2);
        encoded.push_str("redis-binary-v1-");
        for byte in key {
            use std::fmt::Write as _;
            write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
        }
        encoded
    };
    StructuredKey::new(vec![encoded]).map_err(|error| RedisTranslationError::Protocol {
        detail: error.to_string(),
    })
}

fn dedupe_structured_keys(keys: Vec<StructuredKey>) -> Vec<StructuredKey> {
    let mut unique = Vec::new();
    for key in keys {
        if !unique.contains(&key) {
            unique.push(key);
        }
    }
    unique
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
        "HC.STATS" if args.is_empty() => RedisCommand::HcStats,
        "HC.DIAGNOSTICS" if args.is_empty() => RedisCommand::HcDiagnostics,
        "HC.INVALIDATE" if args.len() == 1 => RedisCommand::HcInvalidate {
            key: args.remove(0),
        },
        "HC.NAMESPACE" | "HC.TAG" | "HC.SETTAGS" | "HC.INVALIDATE_TAG" => {
            RedisCommand::HcCandidate {
                command: normalized,
                args,
            }
        }
        "INFO" => RedisCommand::HealthProbeCandidate {
            command: normalized,
            args,
        },
        "CONFIG" | "FLUSHDB" | "FLUSHALL" => RedisCommand::AdminDisabled {
            command: normalized,
            args,
        },
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
    use hydracache_client_transport_axum::{
        ClientIdentity, ClientSurfaceLimits, ClientSurfaceState,
    };

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

        let error = encode_resp2_value(RespValue::Error("ERR unsupported command HSET".to_owned()))
            .unwrap();
        assert_eq!(error, b"-ERR unsupported command HSET\r\n");
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

    #[test]
    fn hello2_is_supported_and_hello3_behavior_matches_contract() {
        let context = RedisTranslationContext::default();
        let hello2 = translate_redis_command(RedisCommand::Hello { version: 2 }, &context).unwrap();
        let RedisTranslatedCommand::Immediate(RespValue::Array(fields)) = hello2 else {
            panic!("HELLO 2 should be immediate array");
        };
        assert!(fields.windows(2).any(|pair| {
            matches!(
                pair,
                [RespValue::BulkString(name), RespValue::Integer(2)] if name == b"proto"
            )
        }));

        let hello3 = translate_redis_command(RedisCommand::Hello { version: 3 }, &context);
        assert!(matches!(
            hello3,
            Err(RedisTranslationError::UnsupportedRespDialect { version: 3 })
        ));
    }

    #[test]
    fn command_reply_advertises_only_supported_subset() {
        let context = RedisTranslationContext::default();
        let command = translate_redis_command(RedisCommand::Command, &context).unwrap();
        let RedisTranslatedCommand::Immediate(value) = command else {
            panic!("COMMAND should be immediate");
        };
        let names = command_names(value);
        assert!(names.contains(&"get".to_owned()));
        assert!(names.contains(&"set".to_owned()));
        assert!(names.contains(&"mget".to_owned()));
        assert!(names.contains(&"del".to_owned()));
        assert!(!names.contains(&"hset".to_owned()));
        assert!(!names.contains(&"cluster".to_owned()));
    }

    #[test]
    fn get_set_del_mget_mset_roundtrip_through_client_surface() {
        let (state, identity) = surface();

        assert_eq!(
            run_command(
                &state,
                &identity,
                RedisCommand::Set {
                    key: b"k".to_vec(),
                    value: b"v".to_vec(),
                    options: Vec::new(),
                },
            ),
            RespValue::SimpleString("OK")
        );
        assert_eq!(
            run_command(&state, &identity, RedisCommand::Get { key: b"k".to_vec() },),
            RespValue::BulkString(b"v".to_vec())
        );
        assert_eq!(
            run_command(
                &state,
                &identity,
                RedisCommand::Mget {
                    keys: vec![b"k".to_vec(), b"missing".to_vec()],
                },
            ),
            RespValue::Array(vec![RespValue::BulkString(b"v".to_vec()), RespValue::Null])
        );
        assert_eq!(
            run_command(
                &state,
                &identity,
                RedisCommand::Del {
                    keys: vec![b"k".to_vec(), b"missing".to_vec()],
                },
            ),
            RespValue::Integer(1)
        );
        assert_eq!(
            run_command(&state, &identity, RedisCommand::Get { key: b"k".to_vec() },),
            RespValue::Null
        );

        let context = RedisTranslationContext::default();
        let mset = translate_redis_command(
            RedisCommand::Unsupported {
                verb: "MSET".to_owned(),
                args: Vec::new(),
            },
            &context,
        );
        assert!(matches!(
            mset,
            Err(RedisTranslationError::UnsupportedCommand { command }) if command == "MSET"
        ));
    }

    #[test]
    fn del_and_exists_return_redis_integer_counts() {
        let (state, identity) = surface();
        run_command(
            &state,
            &identity,
            RedisCommand::Set {
                key: b"k".to_vec(),
                value: b"v".to_vec(),
                options: Vec::new(),
            },
        );

        assert_eq!(
            run_command(
                &state,
                &identity,
                RedisCommand::Exists {
                    keys: vec![b"k".to_vec(), b"k".to_vec(), b"missing".to_vec()],
                },
            ),
            RespValue::Integer(2)
        );
        assert_eq!(
            run_command(
                &state,
                &identity,
                RedisCommand::Del {
                    keys: vec![b"k".to_vec(), b"k".to_vec(), b"missing".to_vec()],
                },
            ),
            RespValue::Integer(1)
        );
    }

    #[test]
    fn mget_preserves_order_and_represents_misses_as_nil_bulk() {
        let (state, identity) = surface();
        run_command(
            &state,
            &identity,
            RedisCommand::Set {
                key: b"a".to_vec(),
                value: b"1".to_vec(),
                options: Vec::new(),
            },
        );
        run_command(
            &state,
            &identity,
            RedisCommand::Set {
                key: b"b".to_vec(),
                value: b"2".to_vec(),
                options: Vec::new(),
            },
        );

        assert_eq!(
            run_command(
                &state,
                &identity,
                RedisCommand::Mget {
                    keys: vec![b"b".to_vec(), b"missing".to_vec(), b"a".to_vec()],
                },
            ),
            RespValue::Array(vec![
                RespValue::BulkString(b"2".to_vec()),
                RespValue::Null,
                RespValue::BulkString(b"1".to_vec()),
            ])
        );
    }

    #[test]
    fn ttl_commands_are_candidate_until_client_surface_exposes_ttl_metadata() {
        let context = RedisTranslationContext::default();
        let translated = translate_redis_command(
            RedisCommand::Set {
                key: b"k".to_vec(),
                value: b"v".to_vec(),
                options: vec![b"EX".to_vec(), b"10".to_vec()],
            },
            &context,
        );
        assert!(matches!(
            translated,
            Err(RedisTranslationError::UnsupportedShape { detail }) if detail.contains("TTL")
        ));
    }

    #[test]
    fn redis_binary_keys_map_to_structured_keys_without_utf8_assumptions() {
        let context = RedisTranslationContext::default();
        let translated =
            translate_redis_command(RedisCommand::Get { key: vec![0, 255] }, &context).unwrap();
        let RedisTranslatedCommand::Execute(plan) = translated else {
            panic!("GET should require client surface execution");
        };
        let [request] = plan.initial_requests() else {
            panic!("GET should produce one request");
        };
        let ClientRequest::Get { key, .. } = &request.request else {
            panic!("GET should translate to ClientRequest::Get");
        };
        assert_eq!(key.segments(), &["redis-binary-v1-00ff".to_owned()]);
    }

    #[test]
    fn hc_invalidate_key_goes_through_client_surface_limits_and_audit() {
        let (state, identity) = surface();
        run_command(
            &state,
            &identity,
            RedisCommand::Set {
                key: b"k".to_vec(),
                value: b"v".to_vec(),
                options: Vec::new(),
            },
        );

        assert_eq!(
            run_command(
                &state,
                &identity,
                RedisCommand::HcInvalidate { key: b"k".to_vec() },
            ),
            RespValue::SimpleString("OK")
        );
        assert_eq!(
            run_command(&state, &identity, RedisCommand::Get { key: b"k".to_vec() },),
            RespValue::Null
        );
        assert_eq!(state.state_mutations(), 2);
    }

    #[test]
    fn hc_stats_and_diagnostics_are_tenant_scoped_extension_requests() {
        let context = RedisTranslationContext::default();
        assert_eq!(
            translate_redis_command(RedisCommand::HcStats, &context).unwrap(),
            RedisTranslatedCommand::Extension(RedisExtensionRequest::Stats)
        );
        assert_eq!(
            translate_redis_command(RedisCommand::HcDiagnostics, &context).unwrap(),
            RedisTranslatedCommand::Extension(RedisExtensionRequest::Diagnostics)
        );
    }

    #[test]
    fn hc_tag_commands_are_unsupported_until_native_metadata_path_exists() {
        let context = RedisTranslationContext::default();
        let translated = translate_redis_command(
            RedisCommand::HcCandidate {
                command: "HC.TAG".to_owned(),
                args: vec![b"k".to_vec(), b"tag".to_vec()],
            },
            &context,
        );
        assert!(matches!(
            translated,
            Err(RedisTranslationError::CandidateCommand { command }) if command == "HC.TAG"
        ));
    }

    #[test]
    fn hc_invalidate_tag_does_not_scan_and_loop_over_visible_keys() {
        let context = RedisTranslationContext::default();
        let translated = translate_redis_command(
            RedisCommand::HcCandidate {
                command: "HC.INVALIDATE_TAG".to_owned(),
                args: vec![b"tag".to_vec()],
            },
            &context,
        );
        assert!(matches!(
            translated,
            Err(RedisTranslationError::CandidateCommand { command })
                if command == "HC.INVALIDATE_TAG"
        ));
    }

    #[test]
    fn hc_commands_decode_to_explicit_extension_shapes() {
        let (stats, _) = decode_resp2_command(b"*1\r\n$8\r\nHC.STATS\r\n")
            .unwrap()
            .unwrap();
        assert_eq!(stats, RedisCommand::HcStats);

        let (invalidate, _) = decode_resp2_command(b"*2\r\n$13\r\nHC.INVALIDATE\r\n$1\r\nk\r\n")
            .unwrap()
            .unwrap();
        assert_eq!(
            invalidate,
            RedisCommand::HcInvalidate { key: b"k".to_vec() }
        );
    }

    #[test]
    fn unsupported_commands_fail_loud_with_stable_error() {
        let context = RedisTranslationContext::default();
        let error = translate_redis_command(
            RedisCommand::Unsupported {
                verb: "HSET".to_owned(),
                args: vec![b"k".to_vec(), b"field".to_vec(), b"value".to_vec()],
            },
            &context,
        )
        .unwrap_err();
        assert!(matches!(
            &error,
            RedisTranslationError::UnsupportedCommand { command } if command == "HSET"
        ));
        assert_eq!(
            encode_resp2_value(error.into_resp_value()).unwrap(),
            b"-ERR unsupported command HSET\r\n"
        );
    }

    #[test]
    fn flushall_is_admin_disabled_by_default() {
        let context = RedisTranslationContext::default();
        let (command, _) = decode_resp2_command(b"*1\r\n$8\r\nFLUSHALL\r\n")
            .unwrap()
            .unwrap();
        assert_eq!(
            command,
            RedisCommand::AdminDisabled {
                command: "FLUSHALL".to_owned(),
                args: Vec::new(),
            }
        );

        let error = translate_redis_command(command, &context).unwrap_err();
        assert!(matches!(
            &error,
            RedisTranslationError::AdminDisabled { command } if command == "FLUSHALL"
        ));
        assert_eq!(
            encode_resp2_value(error.into_resp_value()).unwrap(),
            b"-NOPERM FLUSHALL is disabled by the HydraCache Redis facade\r\n"
        );
    }

    #[test]
    fn cluster_and_moved_ask_are_never_emitted() {
        let context = RedisTranslationContext::default();
        let errors = [
            translate_redis_command(
                RedisCommand::Unsupported {
                    verb: "CLUSTER".to_owned(),
                    args: vec![b"INFO".to_vec()],
                },
                &context,
            )
            .unwrap_err(),
            translate_redis_command(
                RedisCommand::AdminDisabled {
                    command: "FLUSHDB".to_owned(),
                    args: Vec::new(),
                },
                &context,
            )
            .unwrap_err(),
            translate_redis_command(RedisCommand::Hello { version: 3 }, &context).unwrap_err(),
        ];

        for error in errors {
            let encoded =
                String::from_utf8(encode_resp2_value(error.into_resp_value()).unwrap()).unwrap();
            assert!(!encoded.contains("MOVED"));
            assert!(!encoded.contains("ASK"));
        }
    }

    #[test]
    fn info_role_dbsize_type_scan_and_config_follow_contract_classification() {
        let context = RedisTranslationContext::default();
        assert!(matches!(
            translate_redis_command(
                RedisCommand::HealthProbeCandidate {
                    command: "INFO".to_owned(),
                    args: Vec::new(),
                },
                &context,
            ),
            Err(RedisTranslationError::CandidateCommand { command }) if command == "INFO"
        ));

        for command in ["ROLE", "DBSIZE", "TYPE", "SCAN", "CLIENT LIST", "CLIENT ID"] {
            assert!(matches!(
                translate_redis_command(
                    RedisCommand::Unsupported {
                        verb: command.to_owned(),
                        args: Vec::new(),
                    },
                    &context,
                ),
                Err(RedisTranslationError::UnsupportedCommand { command: rejected })
                    if rejected == command
            ));
        }

        assert!(matches!(
            translate_redis_command(
                RedisCommand::AdminDisabled {
                    command: "CONFIG".to_owned(),
                    args: vec![b"GET".to_vec(), b"*".to_vec()],
                },
                &context,
            ),
            Err(RedisTranslationError::AdminDisabled { command }) if command == "CONFIG"
        ));
    }

    fn surface() -> (ClientSurfaceState, ClientIdentity) {
        (
            ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap(),
            ClientIdentity::new("redis-client", DEFAULT_REDIS_NAMESPACE).unwrap(),
        )
    }

    fn run_command(
        state: &ClientSurfaceState,
        identity: &ClientIdentity,
        command: RedisCommand,
    ) -> RespValue {
        let context = RedisTranslationContext::default();
        let translated = translate_redis_command(command, &context).unwrap();
        match translated {
            RedisTranslatedCommand::Immediate(value) => value,
            RedisTranslatedCommand::Execute(plan) => {
                let mut responses = plan
                    .initial_requests()
                    .iter()
                    .cloned()
                    .map(|request| state.dispatch_verified_request(identity, request))
                    .collect::<Vec<_>>();
                let followups = plan.followup_requests(&responses).unwrap();
                responses.extend(
                    followups
                        .into_iter()
                        .map(|request| state.dispatch_verified_request(identity, request)),
                );
                plan.reduce(&responses).unwrap()
            }
            RedisTranslatedCommand::Extension(extension) => {
                panic!("test helper cannot execute extension request {extension:?}")
            }
        }
    }

    fn command_names(value: RespValue) -> Vec<String> {
        let RespValue::Array(commands) = value else {
            panic!("COMMAND should return an array");
        };
        commands
            .into_iter()
            .map(|command| {
                let RespValue::Array(fields) = command else {
                    panic!("COMMAND row should be an array");
                };
                let Some(RespValue::BulkString(name)) = fields.first() else {
                    panic!("COMMAND row should start with name");
                };
                String::from_utf8(name.clone()).unwrap()
            })
            .collect()
    }
}
