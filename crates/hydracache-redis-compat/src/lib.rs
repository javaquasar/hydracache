//! Redis RESP edge compatibility primitives.
//!
//! This crate owns the parser-neutral command model for the optional Redis
//! facade. The RESP listener executes through the same verified client-surface
//! dispatch seam as the stable HydraCache client API.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bytes::{Bytes, BytesMut};
use hydracache_client_protocol::{
    BatchPutEntry, ClientErrorEnvelope, ClientRequest, ClientRequestEnvelope, ClientResponse,
    ClientResponseEnvelope, CompareValueExpireMode, ConditionalPutCondition, Namespace,
    StructuredKey, TtlState,
};
use hydracache_client_transport_axum::{ClientIdentity, ClientSurfaceState};
use redis_protocol::resp2::{
    decode::decode_bytes as decode_resp2_bytes, encode::extend_encode as extend_encode_resp2,
    types::BytesFrame as Resp2BytesFrame,
};
use redis_protocol::resp3::{
    decode::complete::decode_bytes as decode_resp3_bytes,
    encode::complete::extend_encode as extend_encode_resp3,
    types::{BytesFrame as Resp3BytesFrame, FrameMap as Resp3FrameMap},
};
use subtle::ConstantTimeEq;
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::time;

/// RESP dialects claimed by the 0.63 edge surface.
pub const SUPPORTED_RESP_DIALECT: &str = "RESP2+RESP3";

/// Default namespace used when no explicit RESP namespace mapping exists.
pub const DEFAULT_REDIS_NAMESPACE: &str = "redis";

/// Default maximum RESP frame bytes accepted by the edge codec.
pub const DEFAULT_MAX_RESP_FRAME_BYTES: usize = 1024 * 1024;

/// Default maximum RESP array elements accepted before command translation.
pub const DEFAULT_MAX_RESP_ARRAY_ELEMENTS: usize = 1024;

/// Default maximum RESP bulk/simple string bytes accepted before command translation.
pub const DEFAULT_MAX_RESP_BULK_STRING_BYTES: usize = 16 * 1024 * 1024;

/// Default per-read buffer size for RESP sockets.
pub const DEFAULT_REDIS_READ_BUFFER_BYTES: usize = 8 * 1024;

/// Default idle timeout for one RESP connection.
pub const DEFAULT_REDIS_IDLE_TIMEOUT: Duration = Duration::from_secs(60);

const DEFAULT_REDIS_AUTH_USERNAME: &str = "default";
const REDIS_NOAUTH_MESSAGE: &str = "NOAUTH Authentication required.";
const REDIS_WRONGPASS_MESSAGE: &str =
    "WRONGPASS invalid username-password pair or user is disabled.";
const REDIS_SYNTAX_ERROR: &str = "syntax error";
const REDIS_INVALID_SET_EXPIRE_TIME: &str = "invalid expire time in 'set' command";
const REDIS_UNSUPPORTED_LUA_SCRIPT: &str = "unsupported Lua script";

const LOCK_RELEASE_SCRIPT_SIMPLE: &str =
    "if redis.call('get', KEYS[1]) == ARGV[1] then return redis.call('del', KEYS[1]) else return 0 end";
const LOCK_EXTEND_SCRIPT_SIMPLE: &str =
    "if redis.call('get', KEYS[1]) == ARGV[1] then return redis.call('pexpire', KEYS[1], ARGV[2]) else return 0 end";
const LOCK_RELEASE_SCRIPT_REDIS_PY: &str = r#"
        local token = redis.call('get', KEYS[1])
        if not token or token ~= ARGV[1] then
            return 0
        end
        redis.call('del', KEYS[1])
        return 1
    "#;
const LOCK_EXTEND_SCRIPT_REDIS_PY: &str = r#"
        local token = redis.call('get', KEYS[1])
        if not token or token ~= ARGV[1] then
            return 0
        end
        local expiration = redis.call('pttl', KEYS[1])
        if not expiration then
            expiration = 0
        end
        if expiration < 0 then
            return 0
        end

        local newttl = ARGV[2]
        if ARGV[3] == "0" then
            newttl = ARGV[2] + expiration
        end
        redis.call('pexpire', KEYS[1], newttl)
        return 1
    "#;
const LOCK_REACQUIRE_SCRIPT_REDIS_PY: &str = r#"
        local token = redis.call('get', KEYS[1])
        if not token or token ~= ARGV[1] then
            return 0
        end
        redis.call('pexpire', KEYS[1], ARGV[2])
        return 1
    "#;
const LOCK_ACQUIRE_SCRIPT_REDLOCK: &str = r#"
  -- Return 0 if an entry already exists.
  for i, key in ipairs(KEYS) do
    if redis.call("exists", key) == 1 then
      return 0
    end
  end

  -- Create an entry for each provided key.
  for i, key in ipairs(KEYS) do
    redis.call("set", key, ARGV[1], "PX", ARGV[2])
  end

  -- Return the number of entries added.
  return #KEYS
"#;
const LOCK_EXTEND_SCRIPT_REDLOCK: &str = r#"
  -- Return 0 if an entry exists with a *different* lock value.
  for i, key in ipairs(KEYS) do
    if redis.call("get", key) ~= ARGV[1] then
      return 0
    end
  end

  -- Update the entry for each provided key.
  for i, key in ipairs(KEYS) do
    redis.call("set", key, ARGV[1], "PX", ARGV[2])
  end

  -- Return the number of entries updated.
  return #KEYS
"#;
const LOCK_RELEASE_SCRIPT_REDLOCK: &str = r#"
  local count = 0
  for i, key in ipairs(KEYS) do
    -- Only remove entries for *this* lock value.
    if redis.call("get", key) == ARGV[1] then
      redis.pcall("del", key)
      count = count + 1
    end
  end

  -- Return the number of entries removed.
  return count
"#;

/// RESP wire dialect used by one connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RespDialect {
    /// RESP2 request decoding and response encoding.
    Resp2,
    /// RESP3 request decoding and response encoding.
    Resp3,
}

impl RespDialect {
    fn from_hello_version(version: u8) -> Option<Self> {
        match version {
            2 => Some(Self::Resp2),
            3 => Some(Self::Resp3),
            _ => None,
        }
    }
}

/// RESP decoder resource limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RespDecodeLimits {
    /// Maximum buffered frame bytes.
    pub max_frame_bytes: usize,
    /// Maximum array elements in any RESP array.
    pub max_array_elements: usize,
    /// Maximum bytes in any bulk/simple string.
    pub max_bulk_string_bytes: usize,
}

impl Default for RespDecodeLimits {
    fn default() -> Self {
        Self {
            max_frame_bytes: DEFAULT_MAX_RESP_FRAME_BYTES,
            max_array_elements: DEFAULT_MAX_RESP_ARRAY_ELEMENTS,
            max_bulk_string_bytes: DEFAULT_MAX_RESP_BULK_STRING_BYTES,
        }
    }
}

/// Listener-level configuration for the Redis RESP edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedisListenerConfig {
    /// Namespace used for Redis key/value commands.
    pub namespace: String,
    /// Verified client id used when dispatching through the HydraCache client surface.
    pub client_id: String,
    /// Tenant bound to the verified Redis edge identity.
    pub tenant: String,
    /// RESP decoder limits.
    pub decode_limits: RespDecodeLimits,
    /// Bytes read from the socket per read operation.
    pub read_buffer_bytes: usize,
    /// Idle timeout for an open RESP connection.
    pub idle_timeout: Duration,
    /// Optional Redis AUTH policy for this listener.
    pub auth: RedisAuthConfig,
}

impl Default for RedisListenerConfig {
    fn default() -> Self {
        Self {
            namespace: DEFAULT_REDIS_NAMESPACE.to_owned(),
            client_id: "redis-resp".to_owned(),
            tenant: DEFAULT_REDIS_NAMESPACE.to_owned(),
            decode_limits: RespDecodeLimits::default(),
            read_buffer_bytes: DEFAULT_REDIS_READ_BUFFER_BYTES,
            idle_timeout: DEFAULT_REDIS_IDLE_TIMEOUT,
            auth: RedisAuthConfig::default(),
        }
    }
}

impl RedisListenerConfig {
    fn validate(&self) -> Result<(), RedisServeError> {
        if self.read_buffer_bytes == 0 {
            return Err(RedisServeError::Config(
                "read_buffer_bytes must be non-zero",
            ));
        }
        if self.idle_timeout.is_zero() {
            return Err(RedisServeError::Config("idle_timeout must be non-zero"));
        }
        if self.decode_limits.max_frame_bytes == 0 {
            return Err(RedisServeError::Config(
                "decode_limits.max_frame_bytes must be non-zero",
            ));
        }
        if self.decode_limits.max_array_elements == 0 {
            return Err(RedisServeError::Config(
                "decode_limits.max_array_elements must be non-zero",
            ));
        }
        if self.decode_limits.max_bulk_string_bytes == 0 {
            return Err(RedisServeError::Config(
                "decode_limits.max_bulk_string_bytes must be non-zero",
            ));
        }
        RedisTranslationContext::new(self.namespace.clone(), "redis-resp-config")
            .map_err(|_| RedisServeError::Config("namespace must be valid"))?;
        self.auth.validate()?;
        Ok(())
    }
}

/// Redis AUTH policy for one RESP listener.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct RedisAuthConfig {
    /// Whether data/mutating commands require successful AUTH first.
    pub required: bool,
    /// Optional required ACL-style username. When absent, `default` is accepted for HELLO AUTH.
    pub username: Option<String>,
    /// Opaque password/token bytes represented as UTF-8 for config ergonomics.
    pub password: Option<String>,
}

impl std::fmt::Debug for RedisAuthConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RedisAuthConfig")
            .field("required", &self.required)
            .field("username", &self.username)
            .field("password", &self.password.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

impl RedisAuthConfig {
    /// Create a required-auth policy with an optional ACL-style username.
    pub fn required(password: impl Into<String>) -> Self {
        Self {
            required: true,
            username: None,
            password: Some(password.into()),
        }
    }

    fn validate(&self) -> Result<(), RedisServeError> {
        if self
            .username
            .as_deref()
            .is_some_and(|username| username.trim().is_empty())
        {
            return Err(RedisServeError::Config(
                "auth.username must not be empty when set",
            ));
        }
        if self.required {
            let Some(password) = self.password.as_deref() else {
                return Err(RedisServeError::Config(
                    "auth.password is required when auth.required=true",
                ));
            };
            if password.trim().is_empty() {
                return Err(RedisServeError::Config(
                    "auth.password must not be empty when auth.required=true",
                ));
            }
        }
        Ok(())
    }

    fn matches_attempt(&self, attempt: &RedisAuthAttempt) -> bool {
        if !self.required {
            return true;
        }
        let Some(password) = self.password.as_deref() else {
            return false;
        };
        let password_matches = hardened_bytes_eq(password.as_bytes(), attempt.password.as_slice());
        let username_matches = match self.username.as_deref() {
            Some(expected) => attempt
                .username
                .as_deref()
                .is_some_and(|actual| hardened_bytes_eq(expected.as_bytes(), actual)),
            None => attempt.username.as_deref().is_none_or(|actual| {
                actual.eq_ignore_ascii_case(DEFAULT_REDIS_AUTH_USERNAME.as_bytes())
            }),
        };

        password_matches & username_matches
    }
}

fn hardened_bytes_eq(expected: &[u8], actual: &[u8]) -> bool {
    let common_len = expected.len().min(actual.len());
    let prefix_matches: bool = expected[..common_len].ct_eq(&actual[..common_len]).into();
    let length_matches = expected.len() == actual.len();
    let mut trailing_diff = 0u8;

    for byte in expected[common_len..]
        .iter()
        .chain(actual[common_len..].iter())
    {
        trailing_diff |= *byte;
    }

    prefix_matches & length_matches & (trailing_diff == 0)
}

/// Snapshot of listener counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RedisListenerMetrics {
    /// Connections accepted by this in-process listener.
    pub accepted_connections: u64,
    /// RESP commands answered by this listener.
    pub commands: u64,
    /// Decode/translation/encode failures surfaced as RESP errors.
    pub errors: u64,
}

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
    Hello {
        /// Requested RESP protocol version.
        version: u8,
        /// Optional `AUTH username password` clause.
        auth: Option<RedisAuthAttempt>,
    },
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
    /// `INFO [section]`.
    Info { section: Option<Vec<u8>> },
    /// `SELECT index`.
    Select { db: Vec<u8> },
    /// `TYPE key`.
    Type { key: Vec<u8> },
    /// `GET key`.
    Get { key: Vec<u8> },
    /// `SET key value ...`, or legacy TTL aliases `SETEX`/`PSETEX` normalized to `SET EX/PX`.
    Set {
        /// Binary-safe Redis key.
        key: Vec<u8>,
        /// Opaque value bytes.
        value: Vec<u8>,
        /// Additional SET modifiers retained for W0/W2 semantic validation.
        options: Vec<Vec<u8>>,
    },
    /// `MSET key value [key value ...]`.
    Mset { entries: Vec<(Vec<u8>, Vec<u8>)> },
    /// `MGET key...`.
    Mget { keys: Vec<Vec<u8>> },
    /// `DEL key...`.
    Del { keys: Vec<Vec<u8>> },
    /// `EXISTS key...`.
    Exists { keys: Vec<Vec<u8>> },
    /// `EXPIRE key seconds` or `PEXPIRE key milliseconds`.
    Expire {
        key: Vec<u8>,
        ttl: Vec<u8>,
        unit: RedisTtlUnit,
    },
    /// `PERSIST key`.
    Persist { key: Vec<u8> },
    /// `TTL key` or `PTTL key`.
    Ttl { key: Vec<u8>, unit: RedisTtlUnit },
    /// `EVAL script numkeys key [arg ...]`.
    Eval {
        script: Vec<u8>,
        numkeys: Vec<u8>,
        keys_and_args: Vec<Vec<u8>>,
    },
    /// `EVALSHA sha numkeys key [arg ...]`.
    EvalSha {
        sha: Vec<u8>,
        numkeys: Vec<u8>,
        keys_and_args: Vec<Vec<u8>>,
    },
    /// `SCRIPT LOAD script`.
    ScriptLoad { script: Vec<u8> },
    /// `SCRIPT EXISTS sha [sha ...]`.
    ScriptExists { shas: Vec<Vec<u8>> },
    /// `HC.STATS`.
    HcStats,
    /// `HC.DIAGNOSTICS`.
    HcDiagnostics,
    /// `HC.INVALIDATE key`.
    HcInvalidate { key: Vec<u8> },
    /// `HC.NAMESPACE [namespace]`.
    HcNamespace { namespace: Option<Vec<u8>> },
    /// `HC.TAG key tag [tag ...]`.
    HcTag { key: Vec<u8>, tags: Vec<Vec<u8>> },
    /// `HC.SETTAGS key tag [tag ...]`.
    HcSetTags { key: Vec<u8>, tags: Vec<Vec<u8>> },
    /// `HC.INVALIDATE_TAG tag`.
    HcInvalidateTag { tag: Vec<u8> },
    /// `HC.*` command that is intentionally not enabled yet.
    HcCandidate { command: String, args: Vec<Vec<u8>> },
    /// Health/readiness command recognized but not yet claimed.
    HealthProbeCandidate { command: String, args: Vec<Vec<u8>> },
    /// Dangerous or administrative Redis command disabled by default.
    AdminDisabled { command: String, args: Vec<Vec<u8>> },
    /// Recognized command with invalid arity.
    WrongArity { command: String, args: Vec<Vec<u8>> },
    /// Command outside the accepted W1 parser subset.
    Unsupported { verb: String, args: Vec<Vec<u8>> },
}

/// Redis AUTH credentials parsed from `AUTH` or `HELLO ... AUTH ...`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedisAuthAttempt {
    /// Optional ACL-style username.
    pub username: Option<Vec<u8>>,
    /// Password or opaque token bytes.
    pub password: Vec<u8>,
}

/// Redis TTL response unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedisTtlUnit {
    /// Seconds, for `TTL`.
    Seconds,
    /// Milliseconds, for `PTTL`.
    Milliseconds,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RedisLockScriptKind {
    Acquire,
    Release,
    Extend(RedisLockExtendScript),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RedisLockExtendScript {
    Replace,
    RedisPy,
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
    /// RESP3 map response.
    Map(Vec<(RespValue, RespValue)>),
    /// Null bulk response.
    Null,
    /// RESP error response with a stable, redacted message.
    Error(String),
}

/// Codec and command-shape failures.
#[derive(Debug, Error)]
pub enum RedisCompatError {
    /// The underlying RESP codec rejected the bytes.
    #[error("RESP decode error: {0}")]
    Decode(String),
    /// The underlying RESP encoder rejected the response.
    #[error("RESP encode error: {0}")]
    Encode(String),
    /// Buffered RESP frame is too large.
    #[error("RESP frame too large: {actual} bytes exceeds {max} bytes")]
    FrameTooLarge {
        /// Actual buffered bytes.
        actual: usize,
        /// Configured maximum bytes.
        max: usize,
    },
    /// RESP array has too many elements.
    #[error("RESP array too large: {actual} elements exceeds {max} elements")]
    ArrayTooLarge {
        /// Actual array elements.
        actual: usize,
        /// Configured maximum elements.
        max: usize,
    },
    /// RESP string is too large.
    #[error("RESP bulk string too large: {actual} bytes exceeds {max} bytes")]
    BulkStringTooLarge {
        /// Actual string bytes.
        actual: usize,
        /// Configured maximum bytes.
        max: usize,
    },
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

/// Runtime errors from serving a RESP connection.
#[derive(Debug, Error)]
pub enum RedisServeError {
    /// Static listener config is invalid.
    #[error("redis listener config error: {0}")]
    Config(&'static str),
    /// The configured Redis edge identity could not be verified.
    #[error("redis listener identity error: {0}")]
    Identity(String),
    /// The socket failed while reading or writing.
    #[error("redis listener io error: {0}")]
    Io(#[from] std::io::Error),
    /// RESP response encoding failed.
    #[error("redis listener codec error: {0}")]
    Codec(#[from] RedisCompatError),
}

/// RESP connection executor backed by the HydraCache client surface.
#[derive(Debug)]
pub struct RedisRespServer {
    state: Arc<ClientSurfaceState>,
    identity: ClientIdentity,
    config: RedisListenerConfig,
    tag_index: Mutex<RedisTagIndex>,
    script_cache: Mutex<BTreeMap<String, RedisLockScriptKind>>,
    next_request_id: AtomicU64,
    accepted_connections: AtomicU64,
    commands: AtomicU64,
    errors: AtomicU64,
}

#[derive(Debug, Clone)]
struct RedisConnectionState {
    identity: ClientIdentity,
    authenticated: bool,
    dialect: RespDialect,
}

impl RedisConnectionState {
    fn new(identity: &ClientIdentity, auth: &RedisAuthConfig) -> Self {
        Self {
            identity: identity.clone(),
            authenticated: !auth.required,
            dialect: RespDialect::Resp2,
        }
    }

    fn trusted(identity: &ClientIdentity) -> Self {
        Self {
            identity: identity.clone(),
            authenticated: true,
            dialect: RespDialect::Resp2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuthSuccessResponse {
    Ok,
    Hello { dialect: RespDialect },
}

impl RedisRespServer {
    /// Create a Redis RESP executor around shared client surface state.
    pub fn new(
        state: Arc<ClientSurfaceState>,
        config: RedisListenerConfig,
    ) -> Result<Self, RedisServeError> {
        config.validate()?;
        let identity = ClientIdentity::new(config.client_id.clone(), config.tenant.clone())
            .map_err(|error| RedisServeError::Identity(error.to_string()))?;
        Ok(Self {
            state,
            identity,
            config,
            tag_index: Mutex::new(RedisTagIndex::default()),
            script_cache: Mutex::new(BTreeMap::new()),
            next_request_id: AtomicU64::new(1),
            accepted_connections: AtomicU64::new(0),
            commands: AtomicU64::new(0),
            errors: AtomicU64::new(0),
        })
    }

    /// Return the shared client surface state.
    pub fn state(&self) -> Arc<ClientSurfaceState> {
        Arc::clone(&self.state)
    }

    /// Return listener counters.
    pub fn metrics(&self) -> RedisListenerMetrics {
        RedisListenerMetrics {
            accepted_connections: self.accepted_connections.load(Ordering::SeqCst),
            commands: self.commands.load(Ordering::SeqCst),
            errors: self.errors.load(Ordering::SeqCst),
        }
    }

    /// Serve one RESP connection until EOF, QUIT, idle timeout, or malformed input.
    pub async fn serve_connection<S>(&self, mut stream: S) -> Result<(), RedisServeError>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        self.accepted_connections.fetch_add(1, Ordering::SeqCst);
        let mut connection = RedisConnectionState::new(&self.identity, &self.config.auth);
        let mut buffer = Vec::with_capacity(self.config.read_buffer_bytes);
        let mut read_chunk = vec![0; self.config.read_buffer_bytes];
        loop {
            let bytes_read =
                match time::timeout(self.config.idle_timeout, stream.read(&mut read_chunk)).await {
                    Ok(result) => result?,
                    Err(_) => return Ok(()),
                };
            if bytes_read == 0 {
                return Ok(());
            }
            buffer.extend_from_slice(&read_chunk[..bytes_read]);

            while !buffer.is_empty() {
                let decoded = decode_resp_command_with_limits(
                    &buffer,
                    connection.dialect,
                    self.config.decode_limits,
                );
                let (command, consumed) = match decoded {
                    Ok(Some(decoded)) => decoded,
                    Ok(None) => break,
                    Err(error) => {
                        self.write_error(&mut stream, connection.dialect, format!("ERR {error}"))
                            .await?;
                        return Ok(());
                    }
                };
                buffer.drain(..consumed);

                let should_close = matches!(command, RedisCommand::Quit);
                let response = self.execute_connection_command(command, &mut connection);
                self.write_response(&mut stream, connection.dialect, response)
                    .await?;
                self.commands.fetch_add(1, Ordering::SeqCst);
                if should_close {
                    return Ok(());
                }
            }
        }
    }

    /// Execute one parsed Redis command through the configured client-surface state.
    pub fn execute_command(&self, command: RedisCommand) -> RespValue {
        let mut connection = RedisConnectionState::trusted(&self.identity);
        self.execute_connection_command(command, &mut connection)
    }

    fn execute_connection_command(
        &self,
        command: RedisCommand,
        connection: &mut RedisConnectionState,
    ) -> RespValue {
        match command {
            RedisCommand::Auth { username, password } => self.apply_auth(
                RedisAuthAttempt { username, password },
                connection,
                AuthSuccessResponse::Ok,
            ),
            RedisCommand::Hello { version, auth } => {
                let Some(dialect) = RespDialect::from_hello_version(version) else {
                    self.errors.fetch_add(1, Ordering::SeqCst);
                    return RedisTranslationError::UnsupportedRespDialect { version }
                        .into_resp_value();
                };
                if let Some(attempt) = auth {
                    self.apply_auth(attempt, connection, AuthSuccessResponse::Hello { dialect })
                } else {
                    connection.dialect = dialect;
                    hello_response(dialect)
                }
            }
            command if self.requires_auth(&command) && !connection.authenticated => {
                self.errors.fetch_add(1, Ordering::SeqCst);
                RespValue::Error(REDIS_NOAUTH_MESSAGE.to_owned())
            }
            command => self.execute_authenticated_command(command, &connection.identity),
        }
    }

    fn execute_authenticated_command(
        &self,
        command: RedisCommand,
        identity: &ClientIdentity,
    ) -> RespValue {
        match command {
            RedisCommand::ScriptLoad { script } => self.script_load_response(script),
            RedisCommand::ScriptExists { shas } => self.script_exists_response(shas),
            command => {
                if matches!(&command, RedisCommand::Info { .. }) {
                    return info_response(self.metrics());
                }
                self.execute_translatable_authenticated_command(command, identity)
            }
        }
    }

    fn execute_translatable_authenticated_command(
        &self,
        command: RedisCommand,
        identity: &ClientIdentity,
    ) -> RespValue {
        if matches!(&command, RedisCommand::Info { .. }) {
            return info_response(self.metrics());
        }
        let cleanup = RedisTagCleanup::from_command(&command);
        let context = match self.translation_context() {
            Ok(context) => context,
            Err(error) => {
                self.errors.fetch_add(1, Ordering::SeqCst);
                return error.into_resp_value();
            }
        };
        let translated = match translate_redis_command(command, &context) {
            Ok(translated) => translated,
            Err(error) => {
                self.errors.fetch_add(1, Ordering::SeqCst);
                return error.into_resp_value();
            }
        };
        let response = self.execute_translated(translated, identity);
        cleanup.apply_if_success(&self.tag_index, &response);
        response
    }

    fn script_load_response(&self, script: Vec<u8>) -> RespValue {
        let Some(kind) = classify_lock_script(&script) else {
            self.errors.fetch_add(1, Ordering::SeqCst);
            return RespValue::Error(format!("ERR {REDIS_UNSUPPORTED_LUA_SCRIPT}"));
        };
        let sha = sha1_hex(&script);
        self.script_cache
            .lock()
            .expect("redis script cache mutex")
            .insert(sha.clone(), kind);
        RespValue::BulkString(sha.into_bytes())
    }

    fn script_exists_response(&self, shas: Vec<Vec<u8>>) -> RespValue {
        let cache = self.script_cache.lock().expect("redis script cache mutex");
        RespValue::Array(
            shas.into_iter()
                .map(|sha| {
                    let exists = std::str::from_utf8(&sha)
                        .ok()
                        .is_some_and(|sha| cache.contains_key(&sha.to_ascii_lowercase()));
                    RespValue::Integer(i64::from(exists))
                })
                .collect(),
        )
    }

    fn execute_translated(
        &self,
        translated: RedisTranslatedCommand,
        identity: &ClientIdentity,
    ) -> RespValue {
        match translated {
            RedisTranslatedCommand::Immediate(value) => value,
            RedisTranslatedCommand::Execute(plan) => self.execute_plan(plan, identity),
            RedisTranslatedCommand::Extension(extension) => {
                self.extension_response(extension, identity)
            }
        }
    }

    fn execute_plan(&self, plan: RedisExecutionPlan, identity: &ClientIdentity) -> RespValue {
        let mut responses = plan
            .initial_requests()
            .iter()
            .cloned()
            .map(|request| self.state.dispatch_verified_request(identity, request))
            .collect::<Vec<_>>();
        let followups = match plan.followup_requests(&responses) {
            Ok(followups) => followups,
            Err(error) => {
                self.errors.fetch_add(1, Ordering::SeqCst);
                return error.into_resp_value();
            }
        };
        responses.extend(
            followups
                .into_iter()
                .map(|request| self.state.dispatch_verified_request(identity, request)),
        );
        match plan.reduce(&responses) {
            Ok(value) => value,
            Err(error) => {
                self.errors.fetch_add(1, Ordering::SeqCst);
                error.into_resp_value()
            }
        }
    }

    fn extension_response(
        &self,
        extension: RedisExtensionRequest,
        identity: &ClientIdentity,
    ) -> RespValue {
        match self.extension_response_result(extension, identity) {
            Ok(value) => value,
            Err(error) => {
                self.errors.fetch_add(1, Ordering::SeqCst);
                error.into_resp_value()
            }
        }
    }

    fn extension_response_result(
        &self,
        extension: RedisExtensionRequest,
        identity: &ClientIdentity,
    ) -> Result<RespValue, RedisTranslationError> {
        match extension {
            RedisExtensionRequest::Stats => Ok(RespValue::Array(vec![
                bulk_static("surface"),
                bulk_static("redis"),
                bulk_static("tenant"),
                RespValue::BulkString(identity.tenant().as_bytes().to_vec()),
                bulk_static("dispatch_attempts"),
                counter_value(self.state.dispatch_attempts()),
                bulk_static("state_mutations"),
                counter_value(self.state.state_mutations()),
                bulk_static("active_subscriptions"),
                counter_value(self.state.active_subscriptions()),
                bulk_static("accepted_connections"),
                counter_value(self.metrics().accepted_connections),
                bulk_static("commands"),
                counter_value(self.metrics().commands),
                bulk_static("errors"),
                counter_value(self.metrics().errors),
            ])),
            RedisExtensionRequest::Diagnostics => Ok(RespValue::Array(vec![
                bulk_static("protocol"),
                bulk_static(SUPPORTED_RESP_DIALECT),
                bulk_static("namespace"),
                RespValue::BulkString(self.config.namespace.as_bytes().to_vec()),
                bulk_static("tenant"),
                RespValue::BulkString(identity.tenant().as_bytes().to_vec()),
                bulk_static("max_frame_bytes"),
                counter_value(self.config.decode_limits.max_frame_bytes as u64),
                bulk_static("max_array_elements"),
                counter_value(self.config.decode_limits.max_array_elements as u64),
                bulk_static("max_bulk_string_bytes"),
                counter_value(self.config.decode_limits.max_bulk_string_bytes as u64),
            ])),
            RedisExtensionRequest::Namespace { requested } => {
                self.namespace_extension_response(requested)
            }
            RedisExtensionRequest::Tag { key, tags } => {
                self.tag_extension_response(identity, key, tags, RedisTagMode::Add)
            }
            RedisExtensionRequest::SetTags { key, tags } => {
                self.tag_extension_response(identity, key, tags, RedisTagMode::Replace)
            }
            RedisExtensionRequest::InvalidateTag { tag } => {
                self.invalidate_tag_extension_response(identity, tag)
            }
        }
    }

    fn namespace_extension_response(
        &self,
        requested: Option<String>,
    ) -> Result<RespValue, RedisTranslationError> {
        match requested {
            None => Ok(RespValue::BulkString(
                self.config.namespace.as_bytes().to_vec(),
            )),
            Some(namespace) if namespace == self.config.namespace => {
                Ok(RespValue::SimpleString("OK"))
            }
            Some(_) => Err(RedisTranslationError::UnsupportedShape {
                detail: "HC.NAMESPACE can select only the configured listener namespace",
            }),
        }
    }

    fn tag_extension_response(
        &self,
        identity: &ClientIdentity,
        key: Vec<u8>,
        tags: Vec<String>,
        mode: RedisTagMode,
    ) -> Result<RespValue, RedisTranslationError> {
        let namespace = self.configured_namespace()?;
        if !self.live_key_exists(identity, &namespace, &key)? {
            self.tag_index
                .lock()
                .expect("redis tag index mutex")
                .remove_key(&key);
            return Ok(RespValue::Integer(0));
        }

        let changed = {
            let mut index = self.tag_index.lock().expect("redis tag index mutex");
            match mode {
                RedisTagMode::Add => index.add_tags(&key, &tags),
                RedisTagMode::Replace => index.set_tags(&key, &tags),
            }
        };
        Ok(RespValue::Integer(changed.min(i64::MAX as usize) as i64))
    }

    fn invalidate_tag_extension_response(
        &self,
        identity: &ClientIdentity,
        tag: String,
    ) -> Result<RespValue, RedisTranslationError> {
        let namespace = self.configured_namespace()?;
        let raw_keys = self
            .tag_index
            .lock()
            .expect("redis tag index mutex")
            .keys_for_tag(&tag);
        if raw_keys.is_empty() {
            return Ok(RespValue::Integer(0));
        }

        let structured_keys = raw_keys
            .iter()
            .map(|key| redis_key_to_structured_key(key))
            .collect::<Result<Vec<_>, _>>()?;
        let lookup = self.state.dispatch_verified_request(
            identity,
            ClientRequestEnvelope::new(
                self.next_internal_request_id("hc-invalidate-tag-lookup"),
                ClientRequest::BatchGet {
                    ns: namespace.clone(),
                    keys: structured_keys.clone(),
                },
            ),
        );
        if let Err(error) = &lookup.result {
            return Ok(client_error_to_resp(error));
        }
        let values = batch_values(&lookup, raw_keys.len())?;
        let mut invalidated = 0usize;

        for (index, value) in values.into_iter().enumerate() {
            if value.is_none() {
                self.tag_index
                    .lock()
                    .expect("redis tag index mutex")
                    .remove_key(&raw_keys[index]);
                continue;
            }

            let response = self.state.dispatch_verified_request(
                identity,
                ClientRequestEnvelope::new(
                    self.next_internal_request_id(format!("hc-invalidate-tag-{index}")),
                    ClientRequest::Invalidate {
                        ns: namespace.clone(),
                        key: structured_keys[index].clone(),
                    },
                ),
            );
            match &response.result {
                Ok(ClientResponse::Invalidated) => {
                    invalidated += 1;
                    self.tag_index
                        .lock()
                        .expect("redis tag index mutex")
                        .remove_key(&raw_keys[index]);
                }
                Err(error) => return Ok(client_error_to_resp(error)),
                Ok(other) => return unexpected_response(other),
            }
        }

        Ok(RespValue::Integer(invalidated.min(i64::MAX as usize) as i64))
    }

    fn live_key_exists(
        &self,
        identity: &ClientIdentity,
        namespace: &Namespace,
        key: &[u8],
    ) -> Result<bool, RedisTranslationError> {
        let response = self.state.dispatch_verified_request(
            identity,
            ClientRequestEnvelope::new(
                self.next_internal_request_id("hc-tag-lookup"),
                ClientRequest::Get {
                    ns: namespace.clone(),
                    key: redis_key_to_structured_key(key)?,
                },
            ),
        );
        match &response.result {
            Ok(ClientResponse::Value { value }) => Ok(value.is_some()),
            Err(error) => Err(client_error_to_unexpected(error)),
            Ok(other) => unexpected_response(other),
        }
    }

    fn configured_namespace(&self) -> Result<Namespace, RedisTranslationError> {
        Namespace::new(self.config.namespace.clone()).map_err(|error| {
            RedisTranslationError::Protocol {
                detail: error.to_string(),
            }
        })
    }

    fn next_internal_request_id(&self, suffix: impl std::fmt::Display) -> String {
        let request_id = self.next_request_id.fetch_add(1, Ordering::SeqCst);
        format!("redis-resp-{request_id}-{suffix}")
    }

    fn apply_auth(
        &self,
        attempt: RedisAuthAttempt,
        connection: &mut RedisConnectionState,
        success_response: AuthSuccessResponse,
    ) -> RespValue {
        if self.config.auth.matches_attempt(&attempt) {
            connection.authenticated = true;
            connection.identity = self.identity.clone();
            match success_response {
                AuthSuccessResponse::Ok => RespValue::SimpleString("OK"),
                AuthSuccessResponse::Hello { dialect } => {
                    connection.dialect = dialect;
                    hello_response(dialect)
                }
            }
        } else {
            self.errors.fetch_add(1, Ordering::SeqCst);
            RespValue::Error(REDIS_WRONGPASS_MESSAGE.to_owned())
        }
    }

    fn requires_auth(&self, command: &RedisCommand) -> bool {
        self.config.auth.required
            && matches!(
                command,
                RedisCommand::Get { .. }
                    | RedisCommand::Set { .. }
                    | RedisCommand::Mset { .. }
                    | RedisCommand::Mget { .. }
                    | RedisCommand::Del { .. }
                    | RedisCommand::Exists { .. }
                    | RedisCommand::Info { .. }
                    | RedisCommand::Select { .. }
                    | RedisCommand::Type { .. }
                    | RedisCommand::Expire { .. }
                    | RedisCommand::Persist { .. }
                    | RedisCommand::Ttl { .. }
                    | RedisCommand::Eval { .. }
                    | RedisCommand::EvalSha { .. }
                    | RedisCommand::ScriptLoad { .. }
                    | RedisCommand::ScriptExists { .. }
                    | RedisCommand::HcStats
                    | RedisCommand::HcDiagnostics
                    | RedisCommand::HcInvalidate { .. }
                    | RedisCommand::HcNamespace { .. }
                    | RedisCommand::HcTag { .. }
                    | RedisCommand::HcSetTags { .. }
                    | RedisCommand::HcInvalidateTag { .. }
            )
    }

    fn translation_context(&self) -> Result<RedisTranslationContext, RedisTranslationError> {
        let request_id = self.next_request_id.fetch_add(1, Ordering::SeqCst);
        Ok(RedisTranslationContext::new(
            self.config.namespace.clone(),
            format!("redis-resp-{request_id}"),
        )?
        .with_loaded_scripts(
            self.script_cache
                .lock()
                .expect("redis script cache mutex")
                .clone(),
        ))
    }

    async fn write_response<S>(
        &self,
        stream: &mut S,
        dialect: RespDialect,
        response: RespValue,
    ) -> Result<(), RedisServeError>
    where
        S: AsyncWrite + Unpin,
    {
        let encoded = encode_resp_value(response, dialect)?;
        stream.write_all(&encoded).await?;
        stream.flush().await?;
        Ok(())
    }

    async fn write_error<S>(
        &self,
        stream: &mut S,
        dialect: RespDialect,
        message: String,
    ) -> Result<(), RedisServeError>
    where
        S: AsyncWrite + Unpin,
    {
        self.errors.fetch_add(1, Ordering::SeqCst);
        self.write_response(stream, dialect, RespValue::Error(message))
            .await
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RedisTagMode {
    Add,
    Replace,
}

#[derive(Debug, Default)]
struct RedisTagIndex {
    key_tags: BTreeMap<Vec<u8>, BTreeSet<String>>,
    tag_keys: BTreeMap<String, BTreeSet<Vec<u8>>>,
}

impl RedisTagIndex {
    fn add_tags(&mut self, key: &[u8], tags: &[String]) -> usize {
        let key = key.to_vec();
        let mut added = 0usize;
        for tag in tags {
            let inserted = self
                .key_tags
                .entry(key.clone())
                .or_default()
                .insert(tag.clone());
            if inserted {
                self.tag_keys
                    .entry(tag.clone())
                    .or_default()
                    .insert(key.clone());
                added += 1;
            }
        }
        added
    }

    fn set_tags(&mut self, key: &[u8], tags: &[String]) -> usize {
        self.remove_key(key);
        let unique_tags = tags.iter().cloned().collect::<BTreeSet<_>>();
        for tag in &unique_tags {
            self.key_tags
                .entry(key.to_vec())
                .or_default()
                .insert(tag.clone());
            self.tag_keys
                .entry(tag.clone())
                .or_default()
                .insert(key.to_vec());
        }
        unique_tags.len()
    }

    fn keys_for_tag(&self, tag: &str) -> Vec<Vec<u8>> {
        self.tag_keys
            .get(tag)
            .map(|keys| keys.iter().cloned().collect())
            .unwrap_or_default()
    }

    fn remove_key(&mut self, key: &[u8]) {
        let Some(tags) = self.key_tags.remove(key) else {
            return;
        };
        for tag in tags {
            let remove_tag = if let Some(keys) = self.tag_keys.get_mut(&tag) {
                keys.remove(key);
                keys.is_empty()
            } else {
                false
            };
            if remove_tag {
                self.tag_keys.remove(&tag);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RedisTagCleanup {
    None,
    RemoveKeys(Vec<Vec<u8>>),
}

impl RedisTagCleanup {
    fn from_command(command: &RedisCommand) -> Self {
        match command {
            RedisCommand::Del { keys } => Self::RemoveKeys(keys.clone()),
            RedisCommand::HcInvalidate { key } => Self::RemoveKeys(vec![key.clone()]),
            _ => Self::None,
        }
    }

    fn apply_if_success(self, tag_index: &Mutex<RedisTagIndex>, response: &RespValue) {
        if matches!(response, RespValue::Error(_)) {
            return;
        }
        let Self::RemoveKeys(keys) = self else {
            return;
        };
        let mut index = tag_index.lock().expect("redis tag index mutex");
        for key in keys {
            index.remove_key(&key);
        }
    }
}

fn counter_value(value: u64) -> RespValue {
    RespValue::Integer(value.min(i64::MAX as u64) as i64)
}

/// Decode one RESP2 command frame.
pub fn decode_resp2_command(
    input: &[u8],
) -> Result<Option<(RedisCommand, usize)>, RedisCompatError> {
    decode_resp2_command_with_limits(input, RespDecodeLimits::default())
}

/// Decode one RESP3 command frame.
pub fn decode_resp3_command(
    input: &[u8],
) -> Result<Option<(RedisCommand, usize)>, RedisCompatError> {
    decode_resp3_command_with_limits(input, RespDecodeLimits::default())
}

/// Decode one RESP2 command frame with resource limits.
pub fn decode_resp2_command_with_limits(
    input: &[u8],
    limits: RespDecodeLimits,
) -> Result<Option<(RedisCommand, usize)>, RedisCompatError> {
    if input.len() > limits.max_frame_bytes {
        return Err(RedisCompatError::FrameTooLarge {
            actual: input.len(),
            max: limits.max_frame_bytes,
        });
    }
    let bytes = Bytes::copy_from_slice(input);
    let Some((frame, consumed)) =
        decode_resp2_bytes(&bytes).map_err(|error| RedisCompatError::Decode(error.to_string()))?
    else {
        return Ok(None);
    };
    enforce_resp2_frame_limits(&frame, limits)?;
    let command = command_from_resp2_frame(frame)?;
    Ok(Some((command, consumed)))
}

/// Decode one RESP3 command frame with resource limits.
pub fn decode_resp3_command_with_limits(
    input: &[u8],
    limits: RespDecodeLimits,
) -> Result<Option<(RedisCommand, usize)>, RedisCompatError> {
    if input.len() > limits.max_frame_bytes {
        return Err(RedisCompatError::FrameTooLarge {
            actual: input.len(),
            max: limits.max_frame_bytes,
        });
    }
    let bytes = Bytes::copy_from_slice(input);
    let Some((frame, consumed)) =
        decode_resp3_bytes(&bytes).map_err(|error| RedisCompatError::Decode(error.to_string()))?
    else {
        return Ok(None);
    };
    enforce_resp3_frame_limits(&frame, limits)?;
    let command = command_from_resp3_frame(frame)?;
    Ok(Some((command, consumed)))
}

fn decode_resp_command_with_limits(
    input: &[u8],
    dialect: RespDialect,
    limits: RespDecodeLimits,
) -> Result<Option<(RedisCommand, usize)>, RedisCompatError> {
    match dialect {
        RespDialect::Resp2 => decode_resp2_command_with_limits(input, limits),
        RespDialect::Resp3 => decode_resp3_command_with_limits(input, limits),
    }
}

/// Encode a RESP2 response value.
pub fn encode_resp2_value(value: RespValue) -> Result<Vec<u8>, RedisCompatError> {
    let frame = resp_value_to_resp2_frame(value);
    let mut output = BytesMut::new();
    extend_encode_resp2(&mut output, &frame, false)
        .map_err(|error| RedisCompatError::Encode(error.to_string()))?;
    Ok(output.to_vec())
}

/// Encode a RESP3 response value.
pub fn encode_resp3_value(value: RespValue) -> Result<Vec<u8>, RedisCompatError> {
    let frame = resp_value_to_resp3_frame(value);
    let mut output = BytesMut::new();
    extend_encode_resp3(&mut output, &frame, false)
        .map_err(|error| RedisCompatError::Encode(error.to_string()))?;
    Ok(output.to_vec())
}

fn encode_resp_value(value: RespValue, dialect: RespDialect) -> Result<Vec<u8>, RedisCompatError> {
    match dialect {
        RespDialect::Resp2 => encode_resp2_value(value),
        RespDialect::Resp3 => encode_resp3_value(value),
    }
}

fn resp_value_to_resp2_frame(value: RespValue) -> Resp2BytesFrame {
    match value {
        RespValue::SimpleString(value) => {
            Resp2BytesFrame::SimpleString(Bytes::from_static(value.as_bytes()))
        }
        RespValue::Integer(value) => Resp2BytesFrame::Integer(value),
        RespValue::BulkString(value) => Resp2BytesFrame::BulkString(Bytes::from(value)),
        RespValue::Array(values) => Resp2BytesFrame::Array(
            values
                .into_iter()
                .map(resp_value_to_resp2_frame)
                .collect::<Vec<_>>(),
        ),
        RespValue::Map(values) => Resp2BytesFrame::Array(
            values
                .into_iter()
                .flat_map(|(key, value)| {
                    [
                        resp_value_to_resp2_frame(key),
                        resp_value_to_resp2_frame(value),
                    ]
                })
                .collect::<Vec<_>>(),
        ),
        RespValue::Null => Resp2BytesFrame::Null,
        RespValue::Error(value) => Resp2BytesFrame::Error(value.into()),
    }
}

fn resp_value_to_resp3_frame(value: RespValue) -> Resp3BytesFrame {
    match value {
        RespValue::SimpleString(value) => Resp3BytesFrame::SimpleString {
            data: Bytes::from_static(value.as_bytes()),
            attributes: None,
        },
        RespValue::Integer(value) => Resp3BytesFrame::Number {
            data: value,
            attributes: None,
        },
        RespValue::BulkString(value) => Resp3BytesFrame::BlobString {
            data: Bytes::from(value),
            attributes: None,
        },
        RespValue::Array(values) => Resp3BytesFrame::Array {
            data: values.into_iter().map(resp_value_to_resp3_frame).collect(),
            attributes: None,
        },
        RespValue::Map(values) => {
            let mut data = Resp3FrameMap::default();
            for (key, value) in values {
                data.insert(
                    resp_value_to_resp3_frame(key),
                    resp_value_to_resp3_frame(value),
                );
            }
            Resp3BytesFrame::Map {
                data,
                attributes: None,
            }
        }
        RespValue::Null => Resp3BytesFrame::Null,
        RespValue::Error(value) => Resp3BytesFrame::SimpleError {
            data: value.into(),
            attributes: None,
        },
    }
}

fn enforce_resp2_frame_limits(
    frame: &Resp2BytesFrame,
    limits: RespDecodeLimits,
) -> Result<(), RedisCompatError> {
    match frame {
        Resp2BytesFrame::Array(items) => {
            if items.len() > limits.max_array_elements {
                return Err(RedisCompatError::ArrayTooLarge {
                    actual: items.len(),
                    max: limits.max_array_elements,
                });
            }
            for item in items {
                enforce_resp2_frame_limits(item, limits)?;
            }
        }
        Resp2BytesFrame::BulkString(bytes) | Resp2BytesFrame::SimpleString(bytes) => {
            if bytes.len() > limits.max_bulk_string_bytes {
                return Err(RedisCompatError::BulkStringTooLarge {
                    actual: bytes.len(),
                    max: limits.max_bulk_string_bytes,
                });
            }
        }
        Resp2BytesFrame::Error(error) => {
            if error.len() > limits.max_bulk_string_bytes {
                return Err(RedisCompatError::BulkStringTooLarge {
                    actual: error.len(),
                    max: limits.max_bulk_string_bytes,
                });
            }
        }
        Resp2BytesFrame::Integer(_) | Resp2BytesFrame::Null => {}
    }
    Ok(())
}

fn enforce_resp3_frame_limits(
    frame: &Resp3BytesFrame,
    limits: RespDecodeLimits,
) -> Result<(), RedisCompatError> {
    match frame {
        Resp3BytesFrame::Array { data, .. } | Resp3BytesFrame::Push { data, .. } => {
            if data.len() > limits.max_array_elements {
                return Err(RedisCompatError::ArrayTooLarge {
                    actual: data.len(),
                    max: limits.max_array_elements,
                });
            }
            for item in data {
                enforce_resp3_frame_limits(item, limits)?;
            }
        }
        Resp3BytesFrame::Map { data, .. } => {
            let actual = data.len().saturating_mul(2);
            if actual > limits.max_array_elements {
                return Err(RedisCompatError::ArrayTooLarge {
                    actual,
                    max: limits.max_array_elements,
                });
            }
            for (key, value) in data {
                enforce_resp3_frame_limits(key, limits)?;
                enforce_resp3_frame_limits(value, limits)?;
            }
        }
        Resp3BytesFrame::Set { data, .. } => {
            if data.len() > limits.max_array_elements {
                return Err(RedisCompatError::ArrayTooLarge {
                    actual: data.len(),
                    max: limits.max_array_elements,
                });
            }
            for item in data {
                enforce_resp3_frame_limits(item, limits)?;
            }
        }
        Resp3BytesFrame::BlobString { data, .. }
        | Resp3BytesFrame::SimpleString { data, .. }
        | Resp3BytesFrame::BlobError { data, .. }
        | Resp3BytesFrame::BigNumber { data, .. }
        | Resp3BytesFrame::VerbatimString { data, .. }
        | Resp3BytesFrame::ChunkedString(data) => {
            if data.len() > limits.max_bulk_string_bytes {
                return Err(RedisCompatError::BulkStringTooLarge {
                    actual: data.len(),
                    max: limits.max_bulk_string_bytes,
                });
            }
        }
        Resp3BytesFrame::SimpleError { data, .. } => {
            if data.len() > limits.max_bulk_string_bytes {
                return Err(RedisCompatError::BulkStringTooLarge {
                    actual: data.len(),
                    max: limits.max_bulk_string_bytes,
                });
            }
        }
        Resp3BytesFrame::Number { .. }
        | Resp3BytesFrame::Double { .. }
        | Resp3BytesFrame::Boolean { .. }
        | Resp3BytesFrame::Null
        | Resp3BytesFrame::Hello { .. } => {}
    }
    Ok(())
}

/// Per-command translation context for the RESP edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedisTranslationContext {
    namespace: Namespace,
    request_id: String,
    loaded_scripts: BTreeMap<String, RedisLockScriptKind>,
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
            loaded_scripts: BTreeMap::new(),
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

    fn with_loaded_scripts(
        mut self,
        loaded_scripts: BTreeMap<String, RedisLockScriptKind>,
    ) -> Self {
        self.loaded_scripts = loaded_scripts;
        self
    }

    fn resolve_script_sha(&self, sha: &[u8]) -> Option<RedisLockScriptKind> {
        let sha = std::str::from_utf8(sha).ok()?.to_ascii_lowercase();
        self.loaded_scripts
            .get(&sha)
            .copied()
            .or_else(|| known_lock_script_sha(&sha))
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
            loaded_scripts: BTreeMap::new(),
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
    /// Lua script SHA is unknown to the lock-script allowlist/cache.
    #[error("NOSCRIPT No matching script. Please use EVAL.")]
    NoScript,
    /// Redis database index is malformed or negative.
    #[error("ERR invalid DB index")]
    InvalidDatabaseIndex,
    /// HydraCache exposes one Redis-compatible logical database only.
    #[error("ERR multiple Redis databases are not supported; use SELECT 0")]
    MultipleDatabasesUnsupported,
    /// Command has the wrong number of arguments.
    #[error("ERR wrong number of arguments for '{command}' command")]
    WrongArity {
        /// Command name.
        command: String,
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
    /// Listener namespace inspection or same-namespace confirmation.
    Namespace { requested: Option<String> },
    /// Add tags to an existing key in the RESP listener's local tag index.
    Tag { key: Vec<u8>, tags: Vec<String> },
    /// Replace tags for an existing key in the RESP listener's local tag index.
    SetTags { key: Vec<u8>, tags: Vec<String> },
    /// Invalidate keys associated with a tag in the RESP listener's local tag index.
    InvalidateTag { tag: String },
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
    ConditionalStoredInteger,
    Mset,
    Mget { expected_items: usize },
    Del { expected_items: usize },
    Exists { expected_items: usize },
    Type,
    Expiry,
    Ttl { unit: RedisTtlUnit },
    Invalidate,
    CompareValueApplied,
}

impl RedisResponseReducer {
    fn reduce(
        &self,
        responses: &[ClientResponseEnvelope],
    ) -> Result<RespValue, RedisTranslationError> {
        match self {
            Self::Get => reduce_get(responses),
            Self::Set => reduce_set(responses),
            Self::ConditionalStoredInteger => reduce_conditional_stored_integer(responses),
            Self::Mset => reduce_mset(responses),
            Self::Mget { expected_items } => reduce_mget(responses, *expected_items),
            Self::Del { expected_items } => reduce_del(responses, *expected_items),
            Self::Exists { expected_items } => reduce_exists(responses, *expected_items),
            Self::Type => reduce_type(responses),
            Self::Expiry => reduce_expiry(responses),
            Self::Ttl { unit } => reduce_ttl(responses, *unit),
            Self::Invalidate => reduce_invalidate(responses),
            Self::CompareValueApplied => reduce_compare_value_applied(responses),
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
        RedisCommand::Hello { version, auth: _ } => {
            let Some(dialect) = RespDialect::from_hello_version(version) else {
                return Err(RedisTranslationError::UnsupportedRespDialect { version });
            };
            RedisTranslatedCommand::Immediate(hello_response(dialect))
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
        RedisCommand::Info { .. } => {
            RedisTranslatedCommand::Immediate(info_response(RedisListenerMetrics::default()))
        }
        RedisCommand::Select { db } => translate_select(db)?,
        RedisCommand::Type { key } => RedisTranslatedCommand::Execute(single_request_plan(
            context,
            "type",
            ClientRequest::Get {
                ns: context.namespace.clone(),
                key: redis_key_to_structured_key(&key)?,
            },
            RedisResponseReducer::Type,
        )),
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
            let options = parse_set_options(&options)?;
            let key = redis_key_to_structured_key(&key)?;
            let request = match options {
                RedisSetOptions::Upsert { ttl_ms } => ClientRequest::Put {
                    ns: context.namespace.clone(),
                    key,
                    value,
                    ttl_ms,
                    dimensions: Vec::new(),
                },
                RedisSetOptions::IfAbsent { ttl_ms } => ClientRequest::ConditionalPut {
                    ns: context.namespace.clone(),
                    key,
                    value,
                    ttl_ms: Some(ttl_ms),
                    condition: ConditionalPutCondition::IfAbsent,
                },
            };
            RedisTranslatedCommand::Execute(single_request_plan(
                context,
                "set",
                request,
                RedisResponseReducer::Set,
            ))
        }
        RedisCommand::Mset { entries } => {
            let entries = entries
                .into_iter()
                .map(|(key, value)| {
                    Ok(BatchPutEntry {
                        key: redis_key_to_structured_key(&key)?,
                        value,
                    })
                })
                .collect::<Result<Vec<_>, RedisTranslationError>>()?;
            RedisTranslatedCommand::Execute(single_request_plan(
                context,
                "mset",
                ClientRequest::BatchPut {
                    ns: context.namespace.clone(),
                    entries,
                },
                RedisResponseReducer::Mset,
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
        RedisCommand::Expire { key, ttl, unit } => {
            let ttl_ms = parse_expire_ttl_ms(unit, &ttl)?;
            RedisTranslatedCommand::Execute(single_request_plan(
                context,
                "expire",
                ClientRequest::Expire {
                    ns: context.namespace.clone(),
                    key: redis_key_to_structured_key(&key)?,
                    ttl_ms,
                },
                RedisResponseReducer::Expiry,
            ))
        }
        RedisCommand::Persist { key } => RedisTranslatedCommand::Execute(single_request_plan(
            context,
            "persist",
            ClientRequest::Persist {
                ns: context.namespace.clone(),
                key: redis_key_to_structured_key(&key)?,
            },
            RedisResponseReducer::Expiry,
        )),
        RedisCommand::Ttl { key, unit } => RedisTranslatedCommand::Execute(single_request_plan(
            context,
            "ttl",
            ClientRequest::GetTtl {
                ns: context.namespace.clone(),
                key: redis_key_to_structured_key(&key)?,
            },
            RedisResponseReducer::Ttl { unit },
        )),
        RedisCommand::Eval {
            script,
            numkeys,
            keys_and_args,
        } => {
            let Some(kind) = classify_lock_script(&script) else {
                return Err(RedisTranslationError::UnsupportedShape {
                    detail: REDIS_UNSUPPORTED_LUA_SCRIPT,
                });
            };
            RedisTranslatedCommand::Execute(translate_lock_script(
                context,
                kind,
                &numkeys,
                keys_and_args,
            )?)
        }
        RedisCommand::EvalSha {
            sha,
            numkeys,
            keys_and_args,
        } => {
            let Some(kind) = context.resolve_script_sha(&sha) else {
                return Err(RedisTranslationError::NoScript);
            };
            RedisTranslatedCommand::Execute(translate_lock_script(
                context,
                kind,
                &numkeys,
                keys_and_args,
            )?)
        }
        RedisCommand::ScriptLoad { .. } | RedisCommand::ScriptExists { .. } => {
            return Err(RedisTranslationError::UnsupportedShape {
                detail: "SCRIPT LOAD/EXISTS require listener script cache",
            });
        }
        RedisCommand::HcStats => RedisTranslatedCommand::Extension(RedisExtensionRequest::Stats),
        RedisCommand::HcDiagnostics => {
            RedisTranslatedCommand::Extension(RedisExtensionRequest::Diagnostics)
        }
        RedisCommand::HcNamespace { namespace } => {
            RedisTranslatedCommand::Extension(RedisExtensionRequest::Namespace {
                requested: namespace
                    .map(|namespace| parse_hc_label("HC.NAMESPACE", namespace))
                    .transpose()?,
            })
        }
        RedisCommand::HcTag { key, tags } => {
            RedisTranslatedCommand::Extension(RedisExtensionRequest::Tag {
                key,
                tags: parse_hc_tags("HC.TAG", tags)?,
            })
        }
        RedisCommand::HcSetTags { key, tags } => {
            RedisTranslatedCommand::Extension(RedisExtensionRequest::SetTags {
                key,
                tags: parse_hc_tags("HC.SETTAGS", tags)?,
            })
        }
        RedisCommand::HcInvalidateTag { tag } => {
            RedisTranslatedCommand::Extension(RedisExtensionRequest::InvalidateTag {
                tag: parse_hc_label("HC.INVALIDATE_TAG", tag)?,
            })
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
        RedisCommand::WrongArity { command, .. } => {
            return Err(RedisTranslationError::WrongArity { command });
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

fn translate_lock_script(
    context: &RedisTranslationContext,
    kind: RedisLockScriptKind,
    numkeys: &[u8],
    keys_and_args: Vec<Vec<u8>>,
) -> Result<RedisExecutionPlan, RedisTranslationError> {
    let numkeys = ascii_decimal_u64(numkeys)?;
    if numkeys != 1 {
        return Err(RedisTranslationError::UnsupportedShape {
            detail: REDIS_SYNTAX_ERROR,
        });
    }
    match kind {
        RedisLockScriptKind::Acquire => {
            if keys_and_args.len() != 3 {
                return Err(RedisTranslationError::UnsupportedShape {
                    detail: REDIS_SYNTAX_ERROR,
                });
            }
            let key = redis_key_to_structured_key(&keys_and_args[0])?;
            let value = keys_and_args[1].clone();
            let ttl_ms = parse_positive_ttl_ms(b"PX", &keys_and_args[2])?;
            Ok(single_request_plan(
                context,
                "eval-acquire",
                ClientRequest::ConditionalPut {
                    ns: context.namespace.clone(),
                    key,
                    value,
                    condition: ConditionalPutCondition::IfAbsent,
                    ttl_ms: Some(ttl_ms),
                },
                RedisResponseReducer::ConditionalStoredInteger,
            ))
        }
        RedisLockScriptKind::Release => {
            if keys_and_args.len() != 2 {
                return Err(RedisTranslationError::UnsupportedShape {
                    detail: REDIS_SYNTAX_ERROR,
                });
            }
            let key = redis_key_to_structured_key(&keys_and_args[0])?;
            let expected_value = keys_and_args[1].clone();
            Ok(single_request_plan(
                context,
                "eval-release",
                ClientRequest::CompareValueAndInvalidate {
                    ns: context.namespace.clone(),
                    key,
                    expected_value,
                },
                RedisResponseReducer::CompareValueApplied,
            ))
        }
        RedisLockScriptKind::Extend(script) => {
            let (expected_arity, mode) = match script {
                RedisLockExtendScript::Replace => (3, CompareValueExpireMode::Replace),
                RedisLockExtendScript::RedisPy => {
                    if keys_and_args.len() != 4 {
                        return Err(RedisTranslationError::UnsupportedShape {
                            detail: REDIS_SYNTAX_ERROR,
                        });
                    }
                    let mode = match keys_and_args[3].as_slice() {
                        b"0" => CompareValueExpireMode::AddToRemaining,
                        b"1" => CompareValueExpireMode::ReplaceIfExpiring,
                        _ => {
                            return Err(RedisTranslationError::UnsupportedShape {
                                detail: REDIS_SYNTAX_ERROR,
                            });
                        }
                    };
                    (4, mode)
                }
            };
            if keys_and_args.len() != expected_arity {
                return Err(RedisTranslationError::UnsupportedShape {
                    detail: REDIS_SYNTAX_ERROR,
                });
            }
            let key = redis_key_to_structured_key(&keys_and_args[0])?;
            let expected_value = keys_and_args[1].clone();
            let ttl_ms = parse_positive_ttl_ms(b"PX", &keys_and_args[2])?;
            Ok(single_request_plan(
                context,
                "eval-extend",
                ClientRequest::CompareValueAndExpire {
                    ns: context.namespace.clone(),
                    key,
                    expected_value,
                    ttl_ms,
                    mode,
                },
                RedisResponseReducer::CompareValueApplied,
            ))
        }
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
        Ok(ClientResponse::ConditionalStored { stored: true }) => Ok(RespValue::SimpleString("OK")),
        Ok(ClientResponse::ConditionalStored { stored: false }) => Ok(RespValue::Null),
        Err(error) => Ok(client_error_to_resp(error)),
        Ok(other) => unexpected_response(other),
    }
}

fn reduce_conditional_stored_integer(
    responses: &[ClientResponseEnvelope],
) -> Result<RespValue, RedisTranslationError> {
    let response = single_response(responses)?;
    match &response.result {
        Ok(ClientResponse::ConditionalStored { stored }) => {
            Ok(RespValue::Integer(i64::from(*stored)))
        }
        Err(error) => Ok(client_error_to_resp(error)),
        Ok(other) => unexpected_response(other),
    }
}

fn reduce_mset(responses: &[ClientResponseEnvelope]) -> Result<RespValue, RedisTranslationError> {
    let response = single_response(responses)?;
    match &response.result {
        Ok(ClientResponse::Batch { items }) => {
            if let Some(error) = items.iter().find_map(|item| item.result.as_ref().err()) {
                Ok(client_error_to_resp(error))
            } else {
                Ok(RespValue::SimpleString("OK"))
            }
        }
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

fn reduce_type(responses: &[ClientResponseEnvelope]) -> Result<RespValue, RedisTranslationError> {
    let response = single_response(responses)?;
    match &response.result {
        Ok(ClientResponse::Value { value }) => Ok(RespValue::SimpleString(if value.is_some() {
            "string"
        } else {
            "none"
        })),
        Err(error) => Ok(client_error_to_resp(error)),
        Ok(other) => unexpected_response(other),
    }
}

fn reduce_expiry(responses: &[ClientResponseEnvelope]) -> Result<RespValue, RedisTranslationError> {
    let response = single_response(responses)?;
    match &response.result {
        Ok(ClientResponse::Expiry { applied }) => Ok(RespValue::Integer(i64::from(*applied))),
        Err(error) => Ok(client_error_to_resp(error)),
        Ok(other) => unexpected_response(other),
    }
}

fn reduce_ttl(
    responses: &[ClientResponseEnvelope],
    unit: RedisTtlUnit,
) -> Result<RespValue, RedisTranslationError> {
    let response = single_response(responses)?;
    match &response.result {
        Ok(ClientResponse::Ttl { state }) => {
            Ok(RespValue::Integer(ttl_state_to_integer(*state, unit)))
        }
        Err(error) => Ok(client_error_to_resp(error)),
        Ok(other) => unexpected_response(other),
    }
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

fn reduce_compare_value_applied(
    responses: &[ClientResponseEnvelope],
) -> Result<RespValue, RedisTranslationError> {
    let response = single_response(responses)?;
    match &response.result {
        Ok(ClientResponse::CompareValueApplied { applied }) => {
            Ok(RespValue::Integer(i64::from(*applied)))
        }
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

fn ttl_state_to_integer(state: TtlState, unit: RedisTtlUnit) -> i64 {
    match state {
        TtlState::Missing => -2,
        TtlState::Persistent => -1,
        TtlState::ExpiresIn { ttl_ms } => match unit {
            RedisTtlUnit::Milliseconds => ttl_ms.min(i64::MAX as u64) as i64,
            RedisTtlUnit::Seconds => ttl_ms
                .saturating_add(999)
                .saturating_div(1_000)
                .min(i64::MAX as u64) as i64,
        },
    }
}

fn classify_lock_script(script: &[u8]) -> Option<RedisLockScriptKind> {
    let canonical = canonical_lua(script);
    if canonical == canonical_lua(LOCK_ACQUIRE_SCRIPT_REDLOCK.as_bytes()) {
        Some(RedisLockScriptKind::Acquire)
    } else if canonical == canonical_lua(LOCK_RELEASE_SCRIPT_SIMPLE.as_bytes())
        || canonical == canonical_lua(LOCK_RELEASE_SCRIPT_REDIS_PY.as_bytes())
        || canonical == canonical_lua(LOCK_RELEASE_SCRIPT_REDLOCK.as_bytes())
    {
        Some(RedisLockScriptKind::Release)
    } else if canonical == canonical_lua(LOCK_EXTEND_SCRIPT_SIMPLE.as_bytes())
        || canonical == canonical_lua(LOCK_REACQUIRE_SCRIPT_REDIS_PY.as_bytes())
        || canonical == canonical_lua(LOCK_EXTEND_SCRIPT_REDLOCK.as_bytes())
    {
        Some(RedisLockScriptKind::Extend(RedisLockExtendScript::Replace))
    } else if canonical == canonical_lua(LOCK_EXTEND_SCRIPT_REDIS_PY.as_bytes()) {
        Some(RedisLockScriptKind::Extend(RedisLockExtendScript::RedisPy))
    } else {
        None
    }
}

fn known_lock_script_sha(sha: &str) -> Option<RedisLockScriptKind> {
    let sha = sha.to_ascii_lowercase();
    if sha == sha1_hex(LOCK_ACQUIRE_SCRIPT_REDLOCK.as_bytes()) {
        Some(RedisLockScriptKind::Acquire)
    } else if sha == sha1_hex(LOCK_RELEASE_SCRIPT_SIMPLE.as_bytes())
        || sha == sha1_hex(LOCK_RELEASE_SCRIPT_REDIS_PY.as_bytes())
        || sha == sha1_hex(LOCK_RELEASE_SCRIPT_REDLOCK.as_bytes())
    {
        Some(RedisLockScriptKind::Release)
    } else if sha == sha1_hex(LOCK_EXTEND_SCRIPT_SIMPLE.as_bytes())
        || sha == sha1_hex(LOCK_REACQUIRE_SCRIPT_REDIS_PY.as_bytes())
        || sha == sha1_hex(LOCK_EXTEND_SCRIPT_REDLOCK.as_bytes())
    {
        Some(RedisLockScriptKind::Extend(RedisLockExtendScript::Replace))
    } else if sha == sha1_hex(LOCK_EXTEND_SCRIPT_REDIS_PY.as_bytes()) {
        Some(RedisLockScriptKind::Extend(RedisLockExtendScript::RedisPy))
    } else {
        None
    }
}

fn canonical_lua(script: &[u8]) -> String {
    String::from_utf8_lossy(script)
        .chars()
        .filter_map(|ch| {
            if ch.is_ascii_whitespace() || ch == ';' {
                None
            } else if ch == '"' {
                Some('\'')
            } else {
                Some(ch.to_ascii_lowercase())
            }
        })
        .collect()
}

fn sha1_hex(bytes: &[u8]) -> String {
    let mut h0 = 0x6745_2301u32;
    let mut h1 = 0xefcd_ab89u32;
    let mut h2 = 0x98ba_dcfeu32;
    let mut h3 = 0x1032_5476u32;
    let mut h4 = 0xc3d2_e1f0u32;

    let bit_len = (bytes.len() as u64).wrapping_mul(8);
    let mut message = bytes.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in message.chunks_exact(64) {
        let mut words = [0u32; 80];
        for (index, word) in words.iter_mut().take(16).enumerate() {
            let start = index * 4;
            *word = u32::from_be_bytes([
                chunk[start],
                chunk[start + 1],
                chunk[start + 2],
                chunk[start + 3],
            ]);
        }
        for index in 16..80 {
            words[index] =
                (words[index - 3] ^ words[index - 8] ^ words[index - 14] ^ words[index - 16])
                    .rotate_left(1);
        }

        let mut a = h0;
        let mut b = h1;
        let mut c = h2;
        let mut d = h3;
        let mut e = h4;
        for (index, word) in words.iter().enumerate() {
            let (f, k) = match index {
                0..=19 => ((b & c) | ((!b) & d), 0x5a82_7999),
                20..=39 => (b ^ c ^ d, 0x6ed9_eba1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8f1b_bcdc),
                _ => (b ^ c ^ d, 0xca62_c1d6),
            };
            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(*word);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }
        h0 = h0.wrapping_add(a);
        h1 = h1.wrapping_add(b);
        h2 = h2.wrapping_add(c);
        h3 = h3.wrapping_add(d);
        h4 = h4.wrapping_add(e);
    }

    format!("{h0:08x}{h1:08x}{h2:08x}{h3:08x}{h4:08x}")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RedisSetOptions {
    Upsert { ttl_ms: Option<u64> },
    IfAbsent { ttl_ms: u64 },
}

fn parse_set_options(options: &[Vec<u8>]) -> Result<RedisSetOptions, RedisTranslationError> {
    if options.is_empty() {
        return Ok(RedisSetOptions::Upsert { ttl_ms: None });
    }

    let mut ttl_ms = None;
    let mut if_absent = false;
    let mut index = 0;
    while index < options.len() {
        let option = &options[index];
        if option.eq_ignore_ascii_case(b"NX") {
            if if_absent {
                return Err(RedisTranslationError::UnsupportedShape {
                    detail: REDIS_SYNTAX_ERROR,
                });
            }
            if_absent = true;
            index += 1;
        } else if option.eq_ignore_ascii_case(b"EX") || option.eq_ignore_ascii_case(b"PX") {
            if ttl_ms.is_some() || index + 1 >= options.len() {
                return Err(RedisTranslationError::UnsupportedShape {
                    detail: REDIS_SYNTAX_ERROR,
                });
            }
            ttl_ms = Some(parse_positive_ttl_ms(option, &options[index + 1])?);
            index += 2;
        } else {
            return Err(RedisTranslationError::UnsupportedShape {
                detail: REDIS_SYNTAX_ERROR,
            });
        }
    }

    match (if_absent, ttl_ms) {
        (false, ttl_ms) => Ok(RedisSetOptions::Upsert { ttl_ms }),
        (true, Some(ttl_ms)) => Ok(RedisSetOptions::IfAbsent { ttl_ms }),
        (true, None) => Err(RedisTranslationError::UnsupportedShape {
            detail: REDIS_SYNTAX_ERROR,
        }),
    }
}

fn parse_positive_ttl_ms(unit: &[u8], value: &[u8]) -> Result<u64, RedisTranslationError> {
    let value = ascii_decimal_u64(value)?;
    if value == 0 {
        return Err(RedisTranslationError::UnsupportedShape {
            detail: REDIS_INVALID_SET_EXPIRE_TIME,
        });
    }
    ttl_unit_to_millis(unit, value)
}

fn parse_expire_ttl_ms(unit: RedisTtlUnit, value: &[u8]) -> Result<u64, RedisTranslationError> {
    let value = ascii_decimal_i64(value)?;
    if value <= 0 {
        return Ok(0);
    }
    match unit {
        RedisTtlUnit::Seconds => {
            (value as u64)
                .checked_mul(1_000)
                .ok_or(RedisTranslationError::UnsupportedShape {
                    detail: "TTL value is too large",
                })
        }
        RedisTtlUnit::Milliseconds => Ok(value as u64),
    }
}

fn ttl_unit_to_millis(unit: &[u8], value: u64) -> Result<u64, RedisTranslationError> {
    if unit.eq_ignore_ascii_case(b"EX") {
        value
            .checked_mul(1_000)
            .ok_or(RedisTranslationError::UnsupportedShape {
                detail: REDIS_INVALID_SET_EXPIRE_TIME,
            })
    } else if unit.eq_ignore_ascii_case(b"PX") {
        Ok(value)
    } else {
        Err(RedisTranslationError::UnsupportedShape {
            detail: REDIS_SYNTAX_ERROR,
        })
    }
}

fn ascii_decimal_u64(value: &[u8]) -> Result<u64, RedisTranslationError> {
    std::str::from_utf8(value)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or(RedisTranslationError::UnsupportedShape {
            detail: "TTL value must be an integer",
        })
}

fn ascii_decimal_i64(value: &[u8]) -> Result<i64, RedisTranslationError> {
    std::str::from_utf8(value)
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .ok_or(RedisTranslationError::UnsupportedShape {
            detail: "TTL value must be an integer",
        })
}

fn parse_hc_tags(
    command: &'static str,
    tags: Vec<Vec<u8>>,
) -> Result<Vec<String>, RedisTranslationError> {
    tags.into_iter()
        .map(|tag| parse_hc_label(command, tag))
        .collect()
}

fn parse_hc_label(command: &'static str, value: Vec<u8>) -> Result<String, RedisTranslationError> {
    let value = String::from_utf8(value).map_err(|_| RedisTranslationError::UnsupportedShape {
        detail: hc_label_error(command),
    })?;
    if value.is_empty() {
        return Err(RedisTranslationError::UnsupportedShape {
            detail: hc_label_error(command),
        });
    }
    Ok(value)
}

fn hc_label_error(command: &'static str) -> &'static str {
    match command {
        "HC.NAMESPACE" => "HC.NAMESPACE requires a non-empty UTF-8 namespace",
        "HC.TAG" | "HC.SETTAGS" | "HC.INVALIDATE_TAG" => "HC tags must be non-empty UTF-8 strings",
        _ => "HC labels must be non-empty UTF-8 strings",
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

fn hello_response(dialect: RespDialect) -> RespValue {
    match dialect {
        RespDialect::Resp2 => resp2_hello_response(),
        RespDialect::Resp3 => resp3_hello_response(),
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

fn resp3_hello_response() -> RespValue {
    RespValue::Map(vec![
        (
            RespValue::SimpleString("server"),
            RespValue::SimpleString("hydracache"),
        ),
        (
            RespValue::SimpleString("version"),
            RespValue::SimpleString(env!("CARGO_PKG_VERSION")),
        ),
        (RespValue::SimpleString("proto"), RespValue::Integer(3)),
        (RespValue::SimpleString("id"), RespValue::Integer(0)),
        (
            RespValue::SimpleString("mode"),
            RespValue::SimpleString("standalone"),
        ),
        (
            RespValue::SimpleString("role"),
            RespValue::SimpleString("master"),
        ),
        (
            RespValue::SimpleString("modules"),
            RespValue::Array(Vec::new()),
        ),
    ])
}

fn command_metadata_response() -> RespValue {
    RespValue::Array(vec![
        command_metadata("ping", -1, &["fast"], 0, 0, 0),
        command_metadata("echo", 2, &["fast"], 0, 0, 0),
        command_metadata("quit", 1, &["fast"], 0, 0, 0),
        command_metadata("hello", -2, &["fast"], 0, 0, 0),
        command_metadata("auth", -2, &["fast"], 0, 0, 0),
        command_metadata("client", -2, &["fast"], 0, 0, 0),
        command_metadata("command", 1, &["readonly", "fast"], 0, 0, 0),
        command_metadata("info", -1, &["readonly", "fast"], 0, 0, 0),
        command_metadata("select", 2, &["fast"], 0, 0, 0),
        command_metadata("type", 2, &["readonly", "fast"], 1, 1, 1),
        command_metadata("get", 2, &["readonly", "fast"], 1, 1, 1),
        command_metadata("set", -3, &["write"], 1, 1, 1),
        command_metadata("setex", 4, &["write"], 1, 1, 1),
        command_metadata("psetex", 4, &["write"], 1, 1, 1),
        command_metadata("mset", -3, &["write"], 1, -1, 2),
        command_metadata("mget", -2, &["readonly", "fast"], 1, -1, 1),
        command_metadata("del", -2, &["write"], 1, -1, 1),
        command_metadata("exists", -2, &["readonly", "fast"], 1, -1, 1),
        command_metadata("expire", 3, &["write", "fast"], 1, 1, 1),
        command_metadata("pexpire", 3, &["write", "fast"], 1, 1, 1),
        command_metadata("persist", 2, &["write", "fast"], 1, 1, 1),
        command_metadata("ttl", 2, &["readonly", "fast"], 1, 1, 1),
        command_metadata("pttl", 2, &["readonly", "fast"], 1, 1, 1),
        command_metadata("hc.stats", 1, &["readonly", "fast"], 0, 0, 0),
        command_metadata("hc.diagnostics", 1, &["readonly", "fast"], 0, 0, 0),
        command_metadata("hc.invalidate", 2, &["write", "fast"], 1, 1, 1),
        command_metadata("hc.namespace", -1, &["readonly", "fast"], 0, 0, 0),
        command_metadata("hc.tag", -3, &["write", "fast"], 1, 1, 1),
        command_metadata("hc.settags", -3, &["write", "fast"], 1, 1, 1),
        command_metadata("hc.invalidate_tag", 2, &["write"], 0, 0, 0),
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

fn info_response(metrics: RedisListenerMetrics) -> RespValue {
    RespValue::BulkString(
        format!(
            "# Server\r\nredis_mode:standalone\r\nredis_scope:node-local\r\nrole:master\r\nhydracache_version:{}\r\nhydracache_resp:{}\r\n\r\n# Stats\r\ntotal_connections_received:{}\r\ntotal_commands_processed:{}\r\nhydracache_resp_errors:{}\r\n",
            env!("CARGO_PKG_VERSION"),
            SUPPORTED_RESP_DIALECT,
            metrics.accepted_connections,
            metrics.commands,
            metrics.errors
        )
        .into_bytes(),
    )
}

fn translate_select(db: Vec<u8>) -> Result<RedisTranslatedCommand, RedisTranslationError> {
    let db = std::str::from_utf8(&db)
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .ok_or(RedisTranslationError::InvalidDatabaseIndex)?;
    if db < 0 {
        return Err(RedisTranslationError::InvalidDatabaseIndex);
    }
    if db != 0 {
        return Err(RedisTranslationError::MultipleDatabasesUnsupported);
    }
    Ok(RedisTranslatedCommand::Immediate(RespValue::SimpleString(
        "OK",
    )))
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

fn command_from_resp2_frame(frame: Resp2BytesFrame) -> Result<RedisCommand, RedisCompatError> {
    let Resp2BytesFrame::Array(frames) = frame else {
        return Err(RedisCompatError::NonArrayCommand);
    };
    let args = frames
        .into_iter()
        .map(resp2_frame_bytes)
        .collect::<Result<Vec<_>, _>>()?;
    command_from_args(args)
}

fn command_from_resp3_frame(frame: Resp3BytesFrame) -> Result<RedisCommand, RedisCompatError> {
    let Resp3BytesFrame::Array { data, .. } = frame else {
        return Err(RedisCompatError::NonArrayCommand);
    };
    let args = data
        .into_iter()
        .map(resp3_frame_bytes)
        .collect::<Result<Vec<_>, _>>()?;
    command_from_args(args)
}

fn command_from_args(mut args: Vec<Vec<u8>>) -> Result<RedisCommand, RedisCompatError> {
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
        "HELLO" => parse_hello_command(args),
        "AUTH" if args.len() == 1 => RedisCommand::Auth {
            username: None,
            password: args.remove(0),
        },
        "AUTH" if args.len() == 2 => RedisCommand::Auth {
            username: Some(args.remove(0)),
            password: args.remove(0),
        },
        "AUTH" => RedisCommand::WrongArity {
            command: "AUTH".to_owned(),
            args,
        },
        "CLIENT" => parse_client_command(args),
        "COMMAND" => RedisCommand::Command,
        "INFO" if args.len() <= 1 => RedisCommand::Info {
            section: args.into_iter().next(),
        },
        "INFO" => RedisCommand::WrongArity {
            command: "INFO".to_owned(),
            args,
        },
        "SELECT" if args.len() == 1 => RedisCommand::Select { db: args.remove(0) },
        "SELECT" => RedisCommand::WrongArity {
            command: "SELECT".to_owned(),
            args,
        },
        "TYPE" if args.len() == 1 => RedisCommand::Type {
            key: args.remove(0),
        },
        "TYPE" => RedisCommand::WrongArity {
            command: "TYPE".to_owned(),
            args,
        },
        "GET" if args.len() == 1 => RedisCommand::Get {
            key: args.remove(0),
        },
        "SET" if args.len() >= 2 => RedisCommand::Set {
            key: args.remove(0),
            value: args.remove(0),
            options: args,
        },
        "SET" => RedisCommand::WrongArity {
            command: "SET".to_owned(),
            args,
        },
        "SETEX" if args.len() == 3 => {
            let key = args.remove(0);
            let ttl = args.remove(0);
            let value = args.remove(0);
            RedisCommand::Set {
                key,
                value,
                options: vec![b"EX".to_vec(), ttl],
            }
        }
        "PSETEX" if args.len() == 3 => {
            let key = args.remove(0);
            let ttl = args.remove(0);
            let value = args.remove(0);
            RedisCommand::Set {
                key,
                value,
                options: vec![b"PX".to_vec(), ttl],
            }
        }
        "SETEX" | "PSETEX" => RedisCommand::WrongArity {
            command: normalized,
            args,
        },
        "MSET" if !args.is_empty() && args.len().is_multiple_of(2) => RedisCommand::Mset {
            entries: args
                .chunks_exact(2)
                .map(|pair| (pair[0].clone(), pair[1].clone()))
                .collect(),
        },
        "MSET" => RedisCommand::WrongArity {
            command: "MSET".to_owned(),
            args,
        },
        "MGET" if !args.is_empty() => RedisCommand::Mget { keys: args },
        "DEL" if !args.is_empty() => RedisCommand::Del { keys: args },
        "EXISTS" if !args.is_empty() => RedisCommand::Exists { keys: args },
        "EXPIRE" if args.len() == 2 => RedisCommand::Expire {
            key: args.remove(0),
            ttl: args.remove(0),
            unit: RedisTtlUnit::Seconds,
        },
        "PEXPIRE" if args.len() == 2 => RedisCommand::Expire {
            key: args.remove(0),
            ttl: args.remove(0),
            unit: RedisTtlUnit::Milliseconds,
        },
        "PERSIST" if args.len() == 1 => RedisCommand::Persist {
            key: args.remove(0),
        },
        "TTL" if args.len() == 1 => RedisCommand::Ttl {
            key: args.remove(0),
            unit: RedisTtlUnit::Seconds,
        },
        "PTTL" if args.len() == 1 => RedisCommand::Ttl {
            key: args.remove(0),
            unit: RedisTtlUnit::Milliseconds,
        },
        "EXPIRE" | "PEXPIRE" | "PERSIST" | "TTL" | "PTTL" => RedisCommand::WrongArity {
            command: normalized,
            args,
        },
        "EVAL" if args.len() >= 2 => RedisCommand::Eval {
            script: args.remove(0),
            numkeys: args.remove(0),
            keys_and_args: args,
        },
        "EVALSHA" if args.len() >= 2 => RedisCommand::EvalSha {
            sha: args.remove(0),
            numkeys: args.remove(0),
            keys_and_args: args,
        },
        "EVAL" | "EVALSHA" => RedisCommand::WrongArity {
            command: normalized,
            args,
        },
        "SCRIPT" => parse_script_command(args),
        "HC.STATS" if args.is_empty() => RedisCommand::HcStats,
        "HC.DIAGNOSTICS" if args.is_empty() => RedisCommand::HcDiagnostics,
        "HC.INVALIDATE" if args.len() == 1 => RedisCommand::HcInvalidate {
            key: args.remove(0),
        },
        "HC.NAMESPACE" if args.is_empty() => RedisCommand::HcNamespace { namespace: None },
        "HC.NAMESPACE" if args.len() == 1 => RedisCommand::HcNamespace {
            namespace: Some(args.remove(0)),
        },
        "HC.NAMESPACE" => RedisCommand::WrongArity {
            command: "HC.NAMESPACE".to_owned(),
            args,
        },
        "HC.TAG" if args.len() >= 2 => RedisCommand::HcTag {
            key: args.remove(0),
            tags: args,
        },
        "HC.TAG" => RedisCommand::WrongArity {
            command: "HC.TAG".to_owned(),
            args,
        },
        "HC.SETTAGS" if args.len() >= 2 => RedisCommand::HcSetTags {
            key: args.remove(0),
            tags: args,
        },
        "HC.SETTAGS" => RedisCommand::WrongArity {
            command: "HC.SETTAGS".to_owned(),
            args,
        },
        "HC.INVALIDATE_TAG" if args.len() == 1 => RedisCommand::HcInvalidateTag {
            tag: args.remove(0),
        },
        "HC.INVALIDATE_TAG" => RedisCommand::WrongArity {
            command: "HC.INVALIDATE_TAG".to_owned(),
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

fn parse_hello_command(mut args: Vec<Vec<u8>>) -> RedisCommand {
    let version = args
        .first()
        .and_then(|value| std::str::from_utf8(value).ok())
        .and_then(|value| value.parse::<u8>().ok())
        .unwrap_or(0);
    if !args.is_empty() {
        args.remove(0);
    }

    let mut auth = None;
    let mut index = 0;
    while index < args.len() {
        if args[index].eq_ignore_ascii_case(b"AUTH") {
            if index + 2 >= args.len() {
                return RedisCommand::WrongArity {
                    command: "HELLO".to_owned(),
                    args,
                };
            }
            auth = Some(RedisAuthAttempt {
                username: Some(args[index + 1].clone()),
                password: args[index + 2].clone(),
            });
            index += 3;
        } else if args[index].eq_ignore_ascii_case(b"SETNAME") {
            if index + 1 >= args.len() {
                return RedisCommand::WrongArity {
                    command: "HELLO".to_owned(),
                    args,
                };
            }
            index += 2;
        } else {
            return RedisCommand::Unsupported {
                verb: "HELLO".to_owned(),
                args,
            };
        }
    }

    RedisCommand::Hello { version, auth }
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

fn parse_script_command(mut args: Vec<Vec<u8>>) -> RedisCommand {
    let Some(subcommand) = args.first() else {
        return RedisCommand::Unsupported {
            verb: "SCRIPT".to_owned(),
            args,
        };
    };
    let subcommand = String::from_utf8_lossy(subcommand).to_ascii_uppercase();
    match subcommand.as_str() {
        "LOAD" if args.len() == 2 => RedisCommand::ScriptLoad {
            script: args.remove(1),
        },
        "EXISTS" if args.len() >= 2 => RedisCommand::ScriptExists {
            shas: args.into_iter().skip(1).collect(),
        },
        "LOAD" | "EXISTS" => RedisCommand::WrongArity {
            command: format!("SCRIPT {subcommand}"),
            args,
        },
        _ => RedisCommand::Unsupported {
            verb: format!("SCRIPT {subcommand}"),
            args,
        },
    }
}

fn resp2_frame_bytes(frame: Resp2BytesFrame) -> Result<Vec<u8>, RedisCompatError> {
    match frame {
        Resp2BytesFrame::BulkString(bytes) | Resp2BytesFrame::SimpleString(bytes) => {
            Ok(bytes.to_vec())
        }
        _ => Err(RedisCompatError::NonStringArgument),
    }
}

fn resp3_frame_bytes(frame: Resp3BytesFrame) -> Result<Vec<u8>, RedisCompatError> {
    match frame {
        Resp3BytesFrame::BlobString { data, .. }
        | Resp3BytesFrame::SimpleString { data, .. }
        | Resp3BytesFrame::VerbatimString { data, .. } => Ok(data.to_vec()),
        Resp3BytesFrame::Number { data, .. } => Ok(data.to_string().into_bytes()),
        _ => Err(RedisCompatError::NonStringArgument),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydracache_client_transport_axum::{
        ClientIdentity, ClientSurfaceLimits, ClientSurfaceState,
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[test]
    fn facade_advertises_resp2_and_resp3_for_this_release() {
        assert_eq!(SUPPORTED_RESP_DIALECT, "RESP2+RESP3");
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

        let resp3_null = encode_resp3_value(RespValue::Null).unwrap();
        assert_eq!(resp3_null, b"_\r\n");
    }

    #[test]
    fn resp3_command_frames_decode_to_same_parser_neutral_model() {
        let input = b"*2\r\n$3\r\nGET\r\n$3\r\nkey\r\n";
        let (command, consumed) = decode_resp3_command(input).unwrap().unwrap();
        assert_eq!(consumed, input.len());
        assert_eq!(
            command,
            RedisCommand::Get {
                key: b"key".to_vec()
            }
        );

        let (expire, _) = decode_resp3_command(b"*3\r\n$6\r\nEXPIRE\r\n$1\r\nk\r\n:30\r\n")
            .unwrap()
            .unwrap();
        assert_eq!(
            expire,
            RedisCommand::Expire {
                key: b"k".to_vec(),
                ttl: b"30".to_vec(),
                unit: RedisTtlUnit::Seconds
            }
        );
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
    fn command_parser_recognizes_mset_and_ttl_family() {
        let (mset, _) = decode_resp2_command(
            b"*5\r\n$4\r\nMSET\r\n$1\r\na\r\n$1\r\n1\r\n$1\r\nb\r\n$1\r\n2\r\n",
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            mset,
            RedisCommand::Mset {
                entries: vec![
                    (b"a".to_vec(), b"1".to_vec()),
                    (b"b".to_vec(), b"2".to_vec())
                ]
            }
        );

        let (expire, _) = decode_resp2_command(b"*3\r\n$6\r\nEXPIRE\r\n$1\r\nk\r\n$2\r\n30\r\n")
            .unwrap()
            .unwrap();
        assert_eq!(
            expire,
            RedisCommand::Expire {
                key: b"k".to_vec(),
                ttl: b"30".to_vec(),
                unit: RedisTtlUnit::Seconds
            }
        );

        let (pttl, _) = decode_resp2_command(b"*2\r\n$4\r\nPTTL\r\n$1\r\nk\r\n")
            .unwrap()
            .unwrap();
        assert_eq!(
            pttl,
            RedisCommand::Ttl {
                key: b"k".to_vec(),
                unit: RedisTtlUnit::Milliseconds
            }
        );

        let (setex, _) =
            decode_resp2_command(b"*4\r\n$5\r\nSETEX\r\n$1\r\nk\r\n$2\r\n30\r\n$1\r\nv\r\n")
                .unwrap()
                .unwrap();
        assert_eq!(
            setex,
            RedisCommand::Set {
                key: b"k".to_vec(),
                value: b"v".to_vec(),
                options: vec![b"EX".to_vec(), b"30".to_vec()]
            }
        );

        let (psetex, _) =
            decode_resp2_command(b"*4\r\n$6\r\nPSETEX\r\n$1\r\nk\r\n$3\r\n250\r\n$1\r\nv\r\n")
                .unwrap()
                .unwrap();
        assert_eq!(
            psetex,
            RedisCommand::Set {
                key: b"k".to_vec(),
                value: b"v".to_vec(),
                options: vec![b"PX".to_vec(), b"250".to_vec()]
            }
        );
    }

    #[test]
    fn command_parser_recognizes_auth_and_hello_auth() {
        let (auth, _) =
            decode_resp2_command(b"*3\r\n$4\r\nAUTH\r\n$7\r\ndefault\r\n$6\r\nsecret\r\n")
                .unwrap()
                .unwrap();
        assert_eq!(
            auth,
            RedisCommand::Auth {
                username: Some(b"default".to_vec()),
                password: b"secret".to_vec()
            }
        );

        let (hello_auth, _) = decode_resp2_command(
            b"*5\r\n$5\r\nHELLO\r\n$1\r\n2\r\n$4\r\nAUTH\r\n$7\r\ndefault\r\n$6\r\nsecret\r\n",
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            hello_auth,
            RedisCommand::Hello {
                version: 2,
                auth: Some(RedisAuthAttempt {
                    username: Some(b"default".to_vec()),
                    password: b"secret".to_vec()
                })
            }
        );
    }

    #[test]
    fn command_parser_reports_mset_wrong_arity() {
        let (command, _) =
            decode_resp2_command(b"*4\r\n$4\r\nMSET\r\n$1\r\na\r\n$1\r\n1\r\n$1\r\nb\r\n")
                .unwrap()
                .unwrap();
        assert_eq!(
            command,
            RedisCommand::WrongArity {
                command: "MSET".to_owned(),
                args: vec![b"a".to_vec(), b"1".to_vec(), b"b".to_vec()]
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
    fn malformed_resp_frames_fail_loudly_like_redis_protocol_suite() {
        let decode_error_frames: &[&[u8]] = &[
            b"*3\r\n$3\r\nSET\r\n$1\r\nx\r\nfooz\r\n",
            b"*3\r\n$3\r\nSET\r\n$1\r\nx\r\n$-10\r\n",
            b"*3\r\n$3\r\nSET\r\n$1\r\nx\r\n$blabla\r\n",
        ];
        for frame in decode_error_frames {
            assert!(
                matches!(
                    decode_resp2_command(frame),
                    Err(RedisCompatError::Decode(_))
                ),
                "frame should be rejected by RESP decoder: {frame:?}"
            );
        }

        assert!(matches!(
            decode_resp2_command(b":1\r\n"),
            Err(RedisCompatError::NonArrayCommand)
        ));
        assert!(matches!(
            decode_resp2_command(b"*0\r\n"),
            Err(RedisCompatError::EmptyCommand)
        ));
        assert!(matches!(
            decode_resp2_command(b"*2\r\n$3\r\nGET\r\n:1\r\n"),
            Err(RedisCompatError::NonStringArgument)
        ));
    }

    #[test]
    fn resp_decode_limits_reject_oversized_arrays_and_bulk_strings() {
        let limits = RespDecodeLimits {
            max_frame_bytes: 1024,
            max_array_elements: 2,
            max_bulk_string_bytes: 128,
        };
        assert!(matches!(
            decode_resp2_command_with_limits(b"*3\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n", limits),
            Err(RedisCompatError::ArrayTooLarge { actual: 3, max: 2 })
        ));

        let limits = RespDecodeLimits {
            max_frame_bytes: 1024,
            max_array_elements: 8,
            max_bulk_string_bytes: 1,
        };
        assert!(matches!(
            decode_resp2_command_with_limits(b"*2\r\n$3\r\nGET\r\n$2\r\nkk\r\n", limits),
            Err(RedisCompatError::BulkStringTooLarge { actual: 3, max: 1 })
        ));

        let limits = RespDecodeLimits {
            max_frame_bytes: 8,
            max_array_elements: 8,
            max_bulk_string_bytes: 128,
        };
        assert!(matches!(
            decode_resp2_command_with_limits(b"*1\r\n$4\r\nPING\r\n", limits),
            Err(RedisCompatError::FrameTooLarge { actual: 14, max: 8 })
        ));
    }

    #[test]
    fn sha1_hex_matches_known_answer_vectors() {
        assert_eq!(sha1_hex(b""), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        assert_eq!(sha1_hex(b"abc"), "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(
            sha1_hex(b"return 'loaded'"),
            "b534286061d4b9e4026607613b95c06c06015ae8"
        );
    }

    #[test]
    fn lock_script_sha_fingerprints_are_frozen_for_reviewed_client_versions() {
        let cases = [
            (
                "simple release",
                LOCK_RELEASE_SCRIPT_SIMPLE,
                "e9f69f2beb755be68b5e456ee2ce9aadfbc4ebf4",
                Some(RedisLockScriptKind::Release),
            ),
            (
                "simple extend",
                LOCK_EXTEND_SCRIPT_SIMPLE,
                "9136fcf51831e5cf49f109b6e9c97d5b675280d6",
                Some(RedisLockScriptKind::Extend(RedisLockExtendScript::Replace)),
            ),
            (
                "redis-py 5.2.1 release",
                LOCK_RELEASE_SCRIPT_REDIS_PY,
                "c3f8721cbb97f72bc19e972846bd7aaf91901658",
                Some(RedisLockScriptKind::Release),
            ),
            (
                "redis-py 5.2.1 extend",
                LOCK_EXTEND_SCRIPT_REDIS_PY,
                "a4e8783852e6b949f9ef3a97212805108459a890",
                Some(RedisLockScriptKind::Extend(RedisLockExtendScript::RedisPy)),
            ),
            (
                "redis-py 5.2.1 reacquire",
                LOCK_REACQUIRE_SCRIPT_REDIS_PY,
                "1cac51482acf5858da00f6d685d68f886cd6b6b2",
                Some(RedisLockScriptKind::Extend(RedisLockExtendScript::Replace)),
            ),
            (
                "redlock 5.0.0-beta.2 acquire",
                LOCK_ACQUIRE_SCRIPT_REDLOCK,
                "96da70f7716f27d278a5218544df37fd8b0a5e4c",
                Some(RedisLockScriptKind::Acquire),
            ),
            (
                "redlock 5.0.0-beta.2 extend",
                LOCK_EXTEND_SCRIPT_REDLOCK,
                "aed6f382e410db8ba7926d4e5e9aab410bf2a78a",
                Some(RedisLockScriptKind::Extend(RedisLockExtendScript::Replace)),
            ),
            (
                "redlock 5.0.0-beta.2 release",
                LOCK_RELEASE_SCRIPT_REDLOCK,
                "e4612211c9f8f51c257e26e056b0a654b3187242",
                Some(RedisLockScriptKind::Release),
            ),
        ];

        for (label, script, expected_sha, expected_kind) in cases {
            assert_eq!(sha1_hex(script.as_bytes()), expected_sha, "{label}");
            assert_eq!(
                classify_lock_script(script.as_bytes()),
                expected_kind,
                "{label}"
            );
            assert_eq!(
                known_lock_script_sha(expected_sha),
                expected_kind,
                "{label}"
            );
            assert_eq!(
                known_lock_script_sha(&expected_sha.to_ascii_uppercase()),
                expected_kind,
                "{label} uppercase"
            );
        }
    }

    #[test]
    fn hello2_and_hello3_are_supported_and_switch_dialect() {
        let context = RedisTranslationContext::default();
        let hello2 = translate_redis_command(
            RedisCommand::Hello {
                version: 2,
                auth: None,
            },
            &context,
        )
        .unwrap();
        let RedisTranslatedCommand::Immediate(RespValue::Array(fields)) = hello2 else {
            panic!("HELLO 2 should be immediate array");
        };
        assert!(fields.windows(2).any(|pair| {
            matches!(
                pair,
                [RespValue::BulkString(name), RespValue::Integer(2)] if name == b"proto"
            )
        }));

        let hello3 = translate_redis_command(
            RedisCommand::Hello {
                version: 3,
                auth: None,
            },
            &context,
        )
        .unwrap();
        let RedisTranslatedCommand::Immediate(RespValue::Map(fields)) = hello3 else {
            panic!("HELLO 3 should be immediate RESP3 map");
        };
        assert!(fields.contains(&(RespValue::SimpleString("proto"), RespValue::Integer(3))));
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
        assert!(names.contains(&"auth".to_owned()));
        assert!(names.contains(&"info".to_owned()));
        assert!(names.contains(&"select".to_owned()));
        assert!(names.contains(&"type".to_owned()));
        assert!(names.contains(&"mset".to_owned()));
        assert!(names.contains(&"mget".to_owned()));
        assert!(names.contains(&"del".to_owned()));
        assert!(names.contains(&"ttl".to_owned()));
        assert!(names.contains(&"pttl".to_owned()));
        assert!(names.contains(&"hc.namespace".to_owned()));
        assert!(names.contains(&"hc.tag".to_owned()));
        assert!(names.contains(&"hc.settags".to_owned()));
        assert!(names.contains(&"hc.invalidate_tag".to_owned()));
        assert!(!names.contains(&"hset".to_owned()));
        assert!(!names.contains(&"cluster".to_owned()));
    }

    #[test]
    fn info_returns_minimal_honest_facade_state() {
        let context = RedisTranslationContext::default();
        let (command, consumed) = decode_resp2_command(b"*1\r\n$4\r\nINFO\r\n")
            .unwrap()
            .unwrap();

        assert_eq!(consumed, b"*1\r\n$4\r\nINFO\r\n".len());
        assert_eq!(command, RedisCommand::Info { section: None });
        let RedisTranslatedCommand::Immediate(RespValue::BulkString(info)) =
            translate_redis_command(command, &context).unwrap()
        else {
            panic!("INFO should be an immediate bulk string");
        };
        let info = String::from_utf8(info).unwrap();
        assert!(info.contains("# Server\r\n"));
        assert!(info.contains("redis_mode:standalone\r\n"));
        assert!(info.contains("redis_scope:node-local\r\n"));
        assert!(info.contains("role:master\r\n"));
        assert!(info.contains("hydracache_version:"));
        assert!(info.contains("hydracache_resp:RESP2+RESP3\r\n"));
        assert!(info.contains("# Stats\r\n"));
        assert!(info.contains("total_connections_received:0\r\n"));
        assert!(info.contains("total_commands_processed:0\r\n"));
        assert!(!info.contains("used_memory"));
        assert!(!info.contains("cluster_enabled"));
        assert!(!info.contains("db0:"));
    }

    #[test]
    fn info_section_argument_does_not_fabricate_redis_keyspace_state() {
        let context = RedisTranslationContext::default();
        let (command, consumed) = decode_resp2_command(b"*2\r\n$4\r\nINFO\r\n$8\r\nkeyspace\r\n")
            .unwrap()
            .unwrap();

        assert_eq!(consumed, b"*2\r\n$4\r\nINFO\r\n$8\r\nkeyspace\r\n".len());
        assert_eq!(
            command,
            RedisCommand::Info {
                section: Some(b"keyspace".to_vec())
            }
        );
        let RedisTranslatedCommand::Immediate(RespValue::BulkString(info)) =
            translate_redis_command(command, &context).unwrap()
        else {
            panic!("INFO keyspace should be an immediate bulk string");
        };
        let info = String::from_utf8(info).unwrap();
        assert!(info.contains("redis_mode:standalone\r\n"));
        assert!(info.contains("redis_scope:node-local\r\n"));
        assert!(info.contains("hydracache_resp:RESP2+RESP3\r\n"));
        assert!(!info.contains("db0:"));
        assert!(!info.contains("keys="));
        assert!(!info.contains("expires="));
    }

    #[test]
    fn select_zero_is_supported_as_noop_for_single_database_contract() {
        let context = RedisTranslationContext::default();
        let (command, consumed) = decode_resp2_command(b"*2\r\n$6\r\nSELECT\r\n$1\r\n0\r\n")
            .unwrap()
            .unwrap();

        assert_eq!(consumed, b"*2\r\n$6\r\nSELECT\r\n$1\r\n0\r\n".len());
        assert_eq!(command, RedisCommand::Select { db: b"0".to_vec() });
        assert_eq!(
            translate_redis_command(command, &context).unwrap(),
            RedisTranslatedCommand::Immediate(RespValue::SimpleString("OK"))
        );
    }

    #[test]
    fn select_nonzero_and_invalid_db_fail_loud() {
        let context = RedisTranslationContext::default();

        let error = translate_redis_command(RedisCommand::Select { db: b"1".to_vec() }, &context)
            .unwrap_err();
        assert_eq!(error, RedisTranslationError::MultipleDatabasesUnsupported);
        assert_eq!(
            encode_resp2_value(error.into_resp_value()).unwrap(),
            b"-ERR multiple Redis databases are not supported; use SELECT 0\r\n"
        );

        let error = translate_redis_command(
            RedisCommand::Select {
                db: b"not-a-db".to_vec(),
            },
            &context,
        )
        .unwrap_err();
        assert_eq!(error, RedisTranslationError::InvalidDatabaseIndex);
        assert_eq!(
            encode_resp2_value(error.into_resp_value()).unwrap(),
            b"-ERR invalid DB index\r\n"
        );
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
                RedisCommand::Mset {
                    entries: vec![
                        (b"a".to_vec(), b"1".to_vec()),
                        (b"b".to_vec(), b"2".to_vec()),
                        (b"a".to_vec(), b"3".to_vec()),
                    ],
                },
            ),
            RespValue::SimpleString("OK")
        );
        assert_eq!(
            run_command(
                &state,
                &identity,
                RedisCommand::Mget {
                    keys: vec![b"a".to_vec(), b"b".to_vec()],
                },
            ),
            RespValue::Array(vec![
                RespValue::BulkString(b"3".to_vec()),
                RespValue::BulkString(b"2".to_vec())
            ])
        );
        assert_eq!(
            run_command(
                &state,
                &identity,
                RedisCommand::Del {
                    keys: vec![
                        b"k".to_vec(),
                        b"a".to_vec(),
                        b"b".to_vec(),
                        b"missing".to_vec()
                    ],
                },
            ),
            RespValue::Integer(3)
        );
        assert_eq!(
            run_command(&state, &identity, RedisCommand::Get { key: b"k".to_vec() },),
            RespValue::Null
        );
    }

    #[test]
    fn type_reports_string_or_none_through_client_surface() {
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
            run_command(&state, &identity, RedisCommand::Type { key: b"k".to_vec() },),
            RespValue::SimpleString("string")
        );
        assert_eq!(
            run_command(
                &state,
                &identity,
                RedisCommand::Type {
                    key: b"missing".to_vec(),
                },
            ),
            RespValue::SimpleString("none")
        );
    }

    #[test]
    fn mset_oversized_value_rejects_without_partial_mutation() {
        let state = ClientSurfaceState::new(ClientSurfaceLimits {
            max_value_bytes: 2,
            ..ClientSurfaceLimits::default()
        })
        .unwrap();
        let identity = ClientIdentity::new("redis-client", DEFAULT_REDIS_NAMESPACE).unwrap();

        let response = run_command(
            &state,
            &identity,
            RedisCommand::Mset {
                entries: vec![
                    (b"a".to_vec(), b"ok".to_vec()),
                    (b"b".to_vec(), b"too-large".to_vec()),
                ],
            },
        );
        let RespValue::Error(error) = response else {
            panic!("MSET with oversized value should return RESP error");
        };
        assert!(error.contains("too large"));

        assert_eq!(
            run_command(&state, &identity, RedisCommand::Get { key: b"a".to_vec() },),
            RespValue::Null
        );
        assert_eq!(state.state_mutations(), 0);
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
    fn setex_psetex_expire_pexpire_persist_and_ttl_pttl_match_redis_semantics() {
        let (state, identity) = surface();
        state.set_cache_time_for_tests(Some(1_000));

        assert_eq!(
            run_command(
                &state,
                &identity,
                RedisCommand::Set {
                    key: b"k".to_vec(),
                    value: b"v".to_vec(),
                    options: vec![b"EX".to_vec(), b"10".to_vec()],
                },
            ),
            RespValue::SimpleString("OK")
        );
        assert_eq!(
            run_command(
                &state,
                &identity,
                RedisCommand::Ttl {
                    key: b"k".to_vec(),
                    unit: RedisTtlUnit::Seconds,
                },
            ),
            RespValue::Integer(10)
        );

        assert_eq!(
            run_command(
                &state,
                &identity,
                RedisCommand::Expire {
                    key: b"k".to_vec(),
                    ttl: b"250".to_vec(),
                    unit: RedisTtlUnit::Milliseconds,
                },
            ),
            RespValue::Integer(1)
        );
        state.advance_cache_time_for_tests(50);
        assert_eq!(
            run_command(
                &state,
                &identity,
                RedisCommand::Ttl {
                    key: b"k".to_vec(),
                    unit: RedisTtlUnit::Milliseconds,
                },
            ),
            RespValue::Integer(200)
        );

        assert_eq!(
            run_command(
                &state,
                &identity,
                RedisCommand::Persist { key: b"k".to_vec() },
            ),
            RespValue::Integer(1)
        );
        assert_eq!(
            run_command(
                &state,
                &identity,
                RedisCommand::Ttl {
                    key: b"k".to_vec(),
                    unit: RedisTtlUnit::Seconds,
                },
            ),
            RespValue::Integer(-1)
        );
        assert_eq!(
            run_command(
                &state,
                &identity,
                RedisCommand::Ttl {
                    key: b"missing".to_vec(),
                    unit: RedisTtlUnit::Seconds,
                },
            ),
            RespValue::Integer(-2)
        );

        assert_eq!(
            run_command(
                &state,
                &identity,
                RedisCommand::Set {
                    key: b"setex".to_vec(),
                    value: b"v".to_vec(),
                    options: vec![b"EX".to_vec(), b"5".to_vec()],
                },
            ),
            RespValue::SimpleString("OK")
        );
        assert_eq!(
            run_command(
                &state,
                &identity,
                RedisCommand::Ttl {
                    key: b"setex".to_vec(),
                    unit: RedisTtlUnit::Seconds,
                },
            ),
            RespValue::Integer(5)
        );

        assert_eq!(
            run_command(
                &state,
                &identity,
                RedisCommand::Set {
                    key: b"psetex".to_vec(),
                    value: b"v".to_vec(),
                    options: vec![b"PX".to_vec(), b"250".to_vec()],
                },
            ),
            RespValue::SimpleString("OK")
        );
        assert_eq!(
            run_command(
                &state,
                &identity,
                RedisCommand::Ttl {
                    key: b"psetex".to_vec(),
                    unit: RedisTtlUnit::Milliseconds,
                },
            ),
            RespValue::Integer(250)
        );
    }

    #[test]
    fn set_write_conditional_options_follow_conformance_contract() {
        let context = RedisTranslationContext::default();
        let option_cases = [
            vec![b"NX".to_vec()],
            vec![b"XX".to_vec()],
            vec![b"GET".to_vec()],
            vec![b"KEEPTTL".to_vec()],
            vec![b"EXAT".to_vec(), b"2000".to_vec()],
            vec![b"PXAT".to_vec(), b"2000".to_vec()],
            vec![
                b"NX".to_vec(),
                b"NX".to_vec(),
                b"PX".to_vec(),
                b"5000".to_vec(),
            ],
            vec![
                b"PX".to_vec(),
                b"5000".to_vec(),
                b"EX".to_vec(),
                b"5".to_vec(),
            ],
        ];

        for options in option_cases {
            let error = translate_redis_command(
                RedisCommand::Set {
                    key: b"k".to_vec(),
                    value: b"v".to_vec(),
                    options,
                },
                &context,
            )
            .unwrap_err();
            assert_eq!(
                encode_resp2_value(error.into_resp_value()).unwrap(),
                b"-ERR syntax error\r\n"
            );
        }
    }

    #[test]
    fn set_nx_px_acquires_missing_key_and_contention_returns_null() {
        let server = listener();

        assert_eq!(
            server.execute_command(RedisCommand::Set {
                key: b"lock:k".to_vec(),
                value: b"token".to_vec(),
                options: vec![b"NX".to_vec(), b"PX".to_vec(), b"5000".to_vec()],
            }),
            RespValue::SimpleString("OK")
        );
        assert_eq!(
            server.execute_command(RedisCommand::Set {
                key: b"lock:k".to_vec(),
                value: b"contender".to_vec(),
                options: vec![b"PX".to_vec(), b"5000".to_vec(), b"NX".to_vec()],
            }),
            RespValue::Null
        );

        assert_eq!(
            server.execute_command(RedisCommand::Get {
                key: b"lock:k".to_vec()
            }),
            RespValue::BulkString(b"token".to_vec())
        );
    }

    #[test]
    fn set_nx_ex_ttl_uses_seconds_and_expires() {
        let server = listener();
        server.state().set_cache_time_for_tests(Some(1_000));

        assert_eq!(
            server.execute_command(RedisCommand::Set {
                key: b"lock:k".to_vec(),
                value: b"token".to_vec(),
                options: vec![b"NX".to_vec(), b"EX".to_vec(), b"1".to_vec()],
            }),
            RespValue::SimpleString("OK")
        );
        server.state().advance_cache_time_for_tests(1_001);
        assert_eq!(
            server.execute_command(RedisCommand::Set {
                key: b"lock:k".to_vec(),
                value: b"new-token".to_vec(),
                options: vec![b"NX".to_vec(), b"PX".to_vec(), b"5000".to_vec()],
            }),
            RespValue::SimpleString("OK")
        );
        assert_eq!(
            server.execute_command(RedisCommand::Get {
                key: b"lock:k".to_vec()
            }),
            RespValue::BulkString(b"new-token".to_vec())
        );
    }

    #[test]
    fn plain_set_removes_existing_ttl_like_redis() {
        let (state, identity) = surface();
        state.set_cache_time_for_tests(Some(1_000));

        assert_eq!(
            run_command(
                &state,
                &identity,
                RedisCommand::Set {
                    key: b"k".to_vec(),
                    value: b"volatile".to_vec(),
                    options: vec![b"EX".to_vec(), b"100".to_vec()],
                },
            ),
            RespValue::SimpleString("OK")
        );
        assert_eq!(
            run_command(
                &state,
                &identity,
                RedisCommand::Ttl {
                    key: b"k".to_vec(),
                    unit: RedisTtlUnit::Seconds,
                },
            ),
            RespValue::Integer(100)
        );

        assert_eq!(
            run_command(
                &state,
                &identity,
                RedisCommand::Set {
                    key: b"k".to_vec(),
                    value: b"persistent".to_vec(),
                    options: Vec::new(),
                },
            ),
            RespValue::SimpleString("OK")
        );
        assert_eq!(
            run_command(&state, &identity, RedisCommand::Get { key: b"k".to_vec() }),
            RespValue::BulkString(b"persistent".to_vec())
        );
        assert_eq!(
            run_command(
                &state,
                &identity,
                RedisCommand::Ttl {
                    key: b"k".to_vec(),
                    unit: RedisTtlUnit::Seconds,
                },
            ),
            RespValue::Integer(-1)
        );
    }

    #[test]
    fn eval_known_unlock_script_deletes_only_matching_token() {
        let server = listener();
        assert_eq!(
            server.execute_command(RedisCommand::Set {
                key: b"lock:k".to_vec(),
                value: b"owner".to_vec(),
                options: vec![b"NX".to_vec(), b"PX".to_vec(), b"5000".to_vec()],
            }),
            RespValue::SimpleString("OK")
        );

        assert_eq!(
            server.execute_command(RedisCommand::Eval {
                script: LOCK_RELEASE_SCRIPT_SIMPLE.as_bytes().to_vec(),
                numkeys: b"1".to_vec(),
                keys_and_args: vec![b"lock:k".to_vec(), b"wrong".to_vec()],
            }),
            RespValue::Integer(0)
        );
        assert_eq!(
            server.execute_command(RedisCommand::Get {
                key: b"lock:k".to_vec()
            }),
            RespValue::BulkString(b"owner".to_vec())
        );
        assert_eq!(
            server.execute_command(RedisCommand::Eval {
                script: LOCK_RELEASE_SCRIPT_SIMPLE.as_bytes().to_vec(),
                numkeys: b"1".to_vec(),
                keys_and_args: vec![b"lock:k".to_vec(), b"owner".to_vec()],
            }),
            RespValue::Integer(1)
        );
        assert_eq!(
            server.execute_command(RedisCommand::Get {
                key: b"lock:k".to_vec()
            }),
            RespValue::Null
        );
    }

    #[test]
    fn eval_redis_py_release_and_reacquire_scripts_are_exact_allowlisted() {
        let server = listener();
        server.state().set_cache_time_for_tests(Some(1_000));
        assert_eq!(
            server.execute_command(RedisCommand::Set {
                key: b"lock:k".to_vec(),
                value: b"owner".to_vec(),
                options: vec![b"NX".to_vec(), b"PX".to_vec(), b"100".to_vec()],
            }),
            RespValue::SimpleString("OK")
        );

        assert_eq!(
            server.execute_command(RedisCommand::Eval {
                script: LOCK_RELEASE_SCRIPT_REDIS_PY.as_bytes().to_vec(),
                numkeys: b"1".to_vec(),
                keys_and_args: vec![b"lock:k".to_vec(), b"wrong".to_vec()],
            }),
            RespValue::Integer(0)
        );
        assert_eq!(
            server.execute_command(RedisCommand::Eval {
                script: LOCK_REACQUIRE_SCRIPT_REDIS_PY.as_bytes().to_vec(),
                numkeys: b"1".to_vec(),
                keys_and_args: vec![b"lock:k".to_vec(), b"wrong".to_vec(), b"750".to_vec()],
            }),
            RespValue::Integer(0)
        );
        assert_eq!(
            server.execute_command(RedisCommand::Eval {
                script: LOCK_REACQUIRE_SCRIPT_REDIS_PY.as_bytes().to_vec(),
                numkeys: b"1".to_vec(),
                keys_and_args: vec![b"lock:k".to_vec(), b"owner".to_vec(), b"750".to_vec()],
            }),
            RespValue::Integer(1)
        );
        assert_eq!(
            server.execute_command(RedisCommand::Ttl {
                key: b"lock:k".to_vec(),
                unit: RedisTtlUnit::Milliseconds,
            }),
            RespValue::Integer(750)
        );

        assert_eq!(
            server.execute_command(RedisCommand::Eval {
                script: LOCK_RELEASE_SCRIPT_REDIS_PY.as_bytes().to_vec(),
                numkeys: b"1".to_vec(),
                keys_and_args: vec![b"lock:k".to_vec(), b"owner".to_vec()],
            }),
            RespValue::Integer(1)
        );
        assert_eq!(
            server.execute_command(RedisCommand::Get {
                key: b"lock:k".to_vec()
            }),
            RespValue::Null
        );
    }

    #[test]
    fn eval_redlock_single_resource_scripts_acquire_extend_and_release() {
        let server = listener();
        assert_eq!(
            server.execute_command(RedisCommand::Eval {
                script: LOCK_ACQUIRE_SCRIPT_REDLOCK.as_bytes().to_vec(),
                numkeys: b"1".to_vec(),
                keys_and_args: vec![b"lock:k".to_vec(), b"owner".to_vec(), b"5000".to_vec()],
            }),
            RespValue::Integer(1)
        );
        assert_eq!(
            server.execute_command(RedisCommand::Eval {
                script: LOCK_ACQUIRE_SCRIPT_REDLOCK.as_bytes().to_vec(),
                numkeys: b"1".to_vec(),
                keys_and_args: vec![b"lock:k".to_vec(), b"contender".to_vec(), b"5000".to_vec()],
            }),
            RespValue::Integer(0)
        );
        assert_eq!(
            server.execute_command(RedisCommand::Eval {
                script: LOCK_EXTEND_SCRIPT_REDLOCK.as_bytes().to_vec(),
                numkeys: b"1".to_vec(),
                keys_and_args: vec![b"lock:k".to_vec(), b"owner".to_vec(), b"6000".to_vec()],
            }),
            RespValue::Integer(1)
        );
        assert_eq!(
            server.execute_command(RedisCommand::Eval {
                script: LOCK_RELEASE_SCRIPT_REDLOCK.as_bytes().to_vec(),
                numkeys: b"1".to_vec(),
                keys_and_args: vec![b"lock:k".to_vec(), b"contender".to_vec()],
            }),
            RespValue::Integer(0)
        );
        assert_eq!(
            server.execute_command(RedisCommand::Eval {
                script: LOCK_RELEASE_SCRIPT_REDLOCK.as_bytes().to_vec(),
                numkeys: b"1".to_vec(),
                keys_and_args: vec![b"lock:k".to_vec(), b"owner".to_vec()],
            }),
            RespValue::Integer(1)
        );
        assert_eq!(
            server.execute_command(RedisCommand::Get {
                key: b"lock:k".to_vec()
            }),
            RespValue::Null
        );
    }

    #[test]
    fn script_load_exists_and_evalsha_are_allowlist_scoped() {
        let server = listener();
        server.state().set_cache_time_for_tests(Some(1_000));
        assert_eq!(
            server.execute_command(RedisCommand::Set {
                key: b"lock:k".to_vec(),
                value: b"owner".to_vec(),
                options: vec![b"NX".to_vec(), b"PX".to_vec(), b"100".to_vec()],
            }),
            RespValue::SimpleString("OK")
        );

        let RespValue::BulkString(sha) = server.execute_command(RedisCommand::ScriptLoad {
            script: LOCK_EXTEND_SCRIPT_SIMPLE.as_bytes().to_vec(),
        }) else {
            panic!("SCRIPT LOAD should return a SHA bulk string");
        };
        assert_eq!(sha, b"9136fcf51831e5cf49f109b6e9c97d5b675280d6");
        assert_eq!(
            server.execute_command(RedisCommand::ScriptLoad {
                script: LOCK_RELEASE_SCRIPT_REDIS_PY.as_bytes().to_vec(),
            }),
            RespValue::BulkString(b"c3f8721cbb97f72bc19e972846bd7aaf91901658".to_vec())
        );
        assert_eq!(
            server.execute_command(RedisCommand::ScriptLoad {
                script: LOCK_EXTEND_SCRIPT_REDIS_PY.as_bytes().to_vec(),
            }),
            RespValue::BulkString(b"a4e8783852e6b949f9ef3a97212805108459a890".to_vec())
        );
        assert_eq!(
            server.execute_command(RedisCommand::ScriptLoad {
                script: LOCK_REACQUIRE_SCRIPT_REDIS_PY.as_bytes().to_vec(),
            }),
            RespValue::BulkString(b"1cac51482acf5858da00f6d685d68f886cd6b6b2".to_vec())
        );
        let upper_sha = sha.iter().map(u8::to_ascii_uppercase).collect::<Vec<_>>();
        assert_eq!(
            server.execute_command(RedisCommand::ScriptExists {
                shas: vec![
                    sha.clone(),
                    sha.clone(),
                    upper_sha.clone(),
                    b"c3f8721cbb97f72bc19e972846bd7aaf91901658".to_vec(),
                    b"a4e8783852e6b949f9ef3a97212805108459a890".to_vec(),
                    b"1cac51482acf5858da00f6d685d68f886cd6b6b2".to_vec(),
                    b"unknown".to_vec()
                ],
            }),
            RespValue::Array(vec![
                RespValue::Integer(1),
                RespValue::Integer(1),
                RespValue::Integer(1),
                RespValue::Integer(1),
                RespValue::Integer(1),
                RespValue::Integer(1),
                RespValue::Integer(0)
            ])
        );
        assert_eq!(
            server.execute_command(RedisCommand::EvalSha {
                sha: sha.clone(),
                numkeys: b"1".to_vec(),
                keys_and_args: vec![b"lock:k".to_vec(), b"owner".to_vec(), b"1000".to_vec()],
            }),
            RespValue::Integer(1)
        );
        assert_eq!(
            server.execute_command(RedisCommand::EvalSha {
                sha: upper_sha,
                numkeys: b"1".to_vec(),
                keys_and_args: vec![b"lock:k".to_vec(), b"owner".to_vec(), b"750".to_vec()],
            }),
            RespValue::Integer(1)
        );

        let unsupported_script = b"return 'not a lock script'".to_vec();
        let unsupported_sha = sha1_hex(&unsupported_script).into_bytes();
        assert_eq!(
            server.execute_command(RedisCommand::ScriptLoad {
                script: unsupported_script,
            }),
            RespValue::Error("ERR unsupported Lua script".to_owned())
        );
        assert_eq!(
            server.execute_command(RedisCommand::ScriptExists {
                shas: vec![unsupported_sha],
            }),
            RespValue::Array(vec![RespValue::Integer(0)])
        );

        server.state().advance_cache_time_for_tests(150);
        assert_eq!(
            server.execute_command(RedisCommand::Get {
                key: b"lock:k".to_vec()
            }),
            RespValue::BulkString(b"owner".to_vec())
        );
    }

    #[test]
    fn eval_extend_script_maps_keys1_token_and_ttl_without_mutating_on_bad_args() {
        let server = listener();
        server.state().set_cache_time_for_tests(Some(1_000));
        assert_eq!(
            server.execute_command(RedisCommand::Set {
                key: b"lock:k".to_vec(),
                value: b"owner".to_vec(),
                options: vec![b"NX".to_vec(), b"PX".to_vec(), b"100".to_vec()],
            }),
            RespValue::SimpleString("OK")
        );

        assert_eq!(
            server.execute_command(RedisCommand::Eval {
                script: LOCK_EXTEND_SCRIPT_SIMPLE.as_bytes().to_vec(),
                numkeys: b"2".to_vec(),
                keys_and_args: vec![
                    b"lock:k".to_vec(),
                    b"other".to_vec(),
                    b"owner".to_vec(),
                    b"1000".to_vec(),
                ],
            }),
            RespValue::Error("ERR syntax error".to_owned())
        );
        assert_eq!(
            server.execute_command(RedisCommand::Eval {
                script: LOCK_EXTEND_SCRIPT_SIMPLE.as_bytes().to_vec(),
                numkeys: b"1".to_vec(),
                keys_and_args: vec![b"other".to_vec(), b"owner".to_vec(), b"1000".to_vec()],
            }),
            RespValue::Integer(0)
        );
        assert_eq!(
            server.execute_command(RedisCommand::Eval {
                script: LOCK_EXTEND_SCRIPT_SIMPLE.as_bytes().to_vec(),
                numkeys: b"1".to_vec(),
                keys_and_args: vec![b"lock:k".to_vec(), b"1000".to_vec(), b"owner".to_vec()],
            }),
            RespValue::Error("ERR TTL value must be an integer".to_owned())
        );
        assert_eq!(
            server.execute_command(RedisCommand::Eval {
                script: LOCK_EXTEND_SCRIPT_SIMPLE.as_bytes().to_vec(),
                numkeys: b"1".to_vec(),
                keys_and_args: vec![b"lock:k".to_vec(), b"owner".to_vec(), b"1000".to_vec()],
            }),
            RespValue::Integer(1)
        );

        server.state().advance_cache_time_for_tests(150);
        assert_eq!(
            server.execute_command(RedisCommand::Get {
                key: b"lock:k".to_vec()
            }),
            RespValue::BulkString(b"owner".to_vec())
        );
        assert_eq!(
            server.execute_command(RedisCommand::Get {
                key: b"other".to_vec()
            }),
            RespValue::Null
        );
    }

    #[test]
    fn eval_redis_py_extend_adds_to_remaining_ttl_and_rejects_persistent_keys() {
        let server = listener();
        server.state().set_cache_time_for_tests(Some(1_000));
        assert_eq!(
            server.execute_command(RedisCommand::Set {
                key: b"lock:k".to_vec(),
                value: b"owner".to_vec(),
                options: vec![b"NX".to_vec(), b"PX".to_vec(), b"100".to_vec()],
            }),
            RespValue::SimpleString("OK")
        );

        server.state().advance_cache_time_for_tests(25);
        assert_eq!(
            server.execute_command(RedisCommand::Eval {
                script: LOCK_EXTEND_SCRIPT_REDIS_PY.as_bytes().to_vec(),
                numkeys: b"1".to_vec(),
                keys_and_args: vec![
                    b"lock:k".to_vec(),
                    b"owner".to_vec(),
                    b"40".to_vec(),
                    b"0".to_vec(),
                ],
            }),
            RespValue::Integer(1)
        );
        assert_eq!(
            server.execute_command(RedisCommand::Ttl {
                key: b"lock:k".to_vec(),
                unit: RedisTtlUnit::Milliseconds,
            }),
            RespValue::Integer(115)
        );
        assert_eq!(
            server.execute_command(RedisCommand::Eval {
                script: LOCK_EXTEND_SCRIPT_REDIS_PY.as_bytes().to_vec(),
                numkeys: b"1".to_vec(),
                keys_and_args: vec![
                    b"lock:k".to_vec(),
                    b"owner".to_vec(),
                    b"40".to_vec(),
                    b"1".to_vec(),
                ],
            }),
            RespValue::Integer(1)
        );
        assert_eq!(
            server.execute_command(RedisCommand::Ttl {
                key: b"lock:k".to_vec(),
                unit: RedisTtlUnit::Milliseconds,
            }),
            RespValue::Integer(40)
        );

        assert_eq!(
            server.execute_command(RedisCommand::Set {
                key: b"lock:persistent".to_vec(),
                value: b"owner".to_vec(),
                options: Vec::new(),
            }),
            RespValue::SimpleString("OK")
        );
        assert_eq!(
            server.execute_command(RedisCommand::Eval {
                script: LOCK_EXTEND_SCRIPT_REDIS_PY.as_bytes().to_vec(),
                numkeys: b"1".to_vec(),
                keys_and_args: vec![
                    b"lock:persistent".to_vec(),
                    b"owner".to_vec(),
                    b"40".to_vec(),
                    b"0".to_vec(),
                ],
            }),
            RespValue::Integer(0)
        );
        assert_eq!(
            server.execute_command(RedisCommand::Eval {
                script: LOCK_EXTEND_SCRIPT_REDIS_PY.as_bytes().to_vec(),
                numkeys: b"1".to_vec(),
                keys_and_args: vec![
                    b"lock:persistent".to_vec(),
                    b"owner".to_vec(),
                    b"40".to_vec(),
                    b"1".to_vec(),
                ],
            }),
            RespValue::Integer(0)
        );
        assert_eq!(
            server.execute_command(RedisCommand::Ttl {
                key: b"lock:persistent".to_vec(),
                unit: RedisTtlUnit::Milliseconds,
            }),
            RespValue::Integer(-1)
        );
        assert_eq!(
            server.execute_command(RedisCommand::Eval {
                script: LOCK_EXTEND_SCRIPT_REDIS_PY.as_bytes().to_vec(),
                numkeys: b"1".to_vec(),
                keys_and_args: vec![
                    b"lock:k".to_vec(),
                    b"owner".to_vec(),
                    b"40".to_vec(),
                    b"maybe".to_vec(),
                ],
            }),
            RespValue::Error("ERR syntax error".to_owned())
        );
    }

    #[test]
    fn unknown_eval_script_fails_loud_before_mutation() {
        let server = listener();
        assert_eq!(
            server.execute_command(RedisCommand::Set {
                key: b"lock:k".to_vec(),
                value: b"owner".to_vec(),
                options: Vec::new(),
            }),
            RespValue::SimpleString("OK")
        );
        let response = server.execute_command(RedisCommand::Eval {
            script: b"return redis.call('set', KEYS[1], ARGV[1])".to_vec(),
            numkeys: b"1".to_vec(),
            keys_and_args: vec![b"lock:k".to_vec(), b"changed".to_vec()],
        });
        assert_eq!(
            response,
            RespValue::Error("ERR unsupported Lua script".to_owned())
        );
        assert_eq!(
            server.execute_command(RedisCommand::Get {
                key: b"lock:k".to_vec()
            }),
            RespValue::BulkString(b"owner".to_vec())
        );
    }

    #[test]
    fn expire_zero_or_negative_deletes_key_and_returns_one() {
        let (state, identity) = surface();
        state.set_cache_time_for_tests(Some(1_000));

        run_command(
            &state,
            &identity,
            RedisCommand::Set {
                key: b"zero".to_vec(),
                value: b"v".to_vec(),
                options: Vec::new(),
            },
        );
        assert_eq!(
            run_command(
                &state,
                &identity,
                RedisCommand::Expire {
                    key: b"zero".to_vec(),
                    ttl: b"0".to_vec(),
                    unit: RedisTtlUnit::Seconds,
                },
            ),
            RespValue::Integer(1)
        );
        assert_eq!(
            run_command(
                &state,
                &identity,
                RedisCommand::Get {
                    key: b"zero".to_vec()
                },
            ),
            RespValue::Null
        );

        run_command(
            &state,
            &identity,
            RedisCommand::Set {
                key: b"negative".to_vec(),
                value: b"v".to_vec(),
                options: Vec::new(),
            },
        );
        assert_eq!(
            run_command(
                &state,
                &identity,
                RedisCommand::Expire {
                    key: b"negative".to_vec(),
                    ttl: b"-1".to_vec(),
                    unit: RedisTtlUnit::Milliseconds,
                },
            ),
            RespValue::Integer(1)
        );
        assert_eq!(
            run_command(
                &state,
                &identity,
                RedisCommand::Get {
                    key: b"negative".to_vec()
                },
            ),
            RespValue::Null
        );
    }

    #[test]
    fn expired_by_nonpositive_expire_is_absent_for_get_mget_exists_ttl() {
        let (state, identity) = surface();
        state.set_cache_time_for_tests(Some(1_000));
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
                RedisCommand::Expire {
                    key: b"k".to_vec(),
                    ttl: b"0".to_vec(),
                    unit: RedisTtlUnit::Seconds,
                },
            ),
            RespValue::Integer(1)
        );
        assert_eq!(
            run_command(&state, &identity, RedisCommand::Get { key: b"k".to_vec() }),
            RespValue::Null
        );
        assert_eq!(
            run_command(
                &state,
                &identity,
                RedisCommand::Mget {
                    keys: vec![b"k".to_vec()]
                },
            ),
            RespValue::Array(vec![RespValue::Null])
        );
        assert_eq!(
            run_command(
                &state,
                &identity,
                RedisCommand::Exists {
                    keys: vec![b"k".to_vec()]
                },
            ),
            RespValue::Integer(0)
        );
        assert_eq!(
            run_command(
                &state,
                &identity,
                RedisCommand::Ttl {
                    key: b"k".to_vec(),
                    unit: RedisTtlUnit::Seconds,
                },
            ),
            RespValue::Integer(-2)
        );
    }

    #[test]
    fn expire_pexpire_and_persist_on_missing_key_return_zero() {
        let (state, identity) = surface();

        assert_eq!(
            run_command(
                &state,
                &identity,
                RedisCommand::Expire {
                    key: b"missing".to_vec(),
                    ttl: b"30".to_vec(),
                    unit: RedisTtlUnit::Seconds,
                },
            ),
            RespValue::Integer(0)
        );
        assert_eq!(
            run_command(
                &state,
                &identity,
                RedisCommand::Expire {
                    key: b"missing".to_vec(),
                    ttl: b"250".to_vec(),
                    unit: RedisTtlUnit::Milliseconds,
                },
            ),
            RespValue::Integer(0)
        );
        assert_eq!(
            run_command(
                &state,
                &identity,
                RedisCommand::Persist {
                    key: b"missing".to_vec()
                },
            ),
            RespValue::Integer(0)
        );
    }

    #[test]
    fn expire_options_are_unsupported_without_mutating_existing_ttl() {
        let server = listener();
        server.state().set_cache_time_for_tests(Some(1_000));
        assert_eq!(
            server.execute_command(RedisCommand::Set {
                key: b"k".to_vec(),
                value: b"v".to_vec(),
                options: vec![b"EX".to_vec(), b"100".to_vec()],
            }),
            RespValue::SimpleString("OK")
        );

        let option_frames: &[(&[u8], &[u8])] = &[
            (
                b"*4\r\n$6\r\nEXPIRE\r\n$1\r\nk\r\n$3\r\n200\r\n$2\r\nNX\r\n",
                b"NX",
            ),
            (
                b"*4\r\n$6\r\nEXPIRE\r\n$1\r\nk\r\n$3\r\n200\r\n$2\r\nXX\r\n",
                b"XX",
            ),
            (
                b"*4\r\n$6\r\nEXPIRE\r\n$1\r\nk\r\n$3\r\n200\r\n$2\r\nGT\r\n",
                b"GT",
            ),
            (
                b"*4\r\n$6\r\nEXPIRE\r\n$1\r\nk\r\n$3\r\n200\r\n$2\r\nLT\r\n",
                b"LT",
            ),
            (
                b"*4\r\n$6\r\nEXPIRE\r\n$1\r\nk\r\n$3\r\n200\r\n$2\r\nAB\r\n",
                b"AB",
            ),
        ];

        for (frame, option) in option_frames {
            let (command, consumed) = decode_resp2_command(frame).unwrap().unwrap();
            assert_eq!(consumed, frame.len());
            assert!(
                matches!(
                    &command,
                    RedisCommand::WrongArity { command, args }
                        if command == "EXPIRE"
                            && args.last().is_some_and(|arg| arg.eq_ignore_ascii_case(option))
                ),
                "EXPIRE option should stay outside the supported shape: {option:?}"
            );
            assert_eq!(
                server.execute_command(command),
                RespValue::Error("ERR wrong number of arguments for 'EXPIRE' command".to_owned())
            );
            assert_eq!(
                server.execute_command(RedisCommand::Ttl {
                    key: b"k".to_vec(),
                    unit: RedisTtlUnit::Seconds,
                }),
                RespValue::Integer(100)
            );
        }
    }

    #[test]
    fn rejected_set_and_expire_shapes_use_redis_error_class_or_documented_normalization() {
        let context = RedisTranslationContext::default();

        let set_option_error = translate_redis_command(
            RedisCommand::Set {
                key: b"k".to_vec(),
                value: b"v".to_vec(),
                options: vec![b"NX".to_vec()],
            },
            &context,
        )
        .unwrap_err();
        assert_eq!(
            encode_resp2_value(set_option_error.into_resp_value()).unwrap(),
            b"-ERR syntax error\r\n"
        );

        let set_zero_ttl_error = translate_redis_command(
            RedisCommand::Set {
                key: b"k".to_vec(),
                value: b"v".to_vec(),
                options: vec![b"EX".to_vec(), b"0".to_vec()],
            },
            &context,
        )
        .unwrap_err();
        assert_eq!(
            encode_resp2_value(set_zero_ttl_error.into_resp_value()).unwrap(),
            b"-ERR invalid expire time in 'set' command\r\n"
        );

        let expire_overflow_error = translate_redis_command(
            RedisCommand::Expire {
                key: b"k".to_vec(),
                ttl: b"9223372036854775807".to_vec(),
                unit: RedisTtlUnit::Seconds,
            },
            &context,
        )
        .unwrap_err();
        let encoded =
            String::from_utf8(encode_resp2_value(expire_overflow_error.into_resp_value()).unwrap())
                .unwrap();
        assert!(encoded.starts_with("-ERR "));
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
        assert_eq!(
            translate_redis_command(
                RedisCommand::HcNamespace {
                    namespace: Some(b"redis".to_vec())
                },
                &context
            )
            .unwrap(),
            RedisTranslatedCommand::Extension(RedisExtensionRequest::Namespace {
                requested: Some("redis".to_owned())
            })
        );
        assert_eq!(
            translate_redis_command(
                RedisCommand::HcTag {
                    key: b"k".to_vec(),
                    tags: vec![b"model".to_vec()]
                },
                &context
            )
            .unwrap(),
            RedisTranslatedCommand::Extension(RedisExtensionRequest::Tag {
                key: b"k".to_vec(),
                tags: vec!["model".to_owned()]
            })
        );
    }

    #[test]
    fn hc_namespace_is_listener_scoped_not_redis_multidb() {
        let server = listener();

        assert_eq!(
            server.execute_command(RedisCommand::HcNamespace { namespace: None }),
            RespValue::BulkString(DEFAULT_REDIS_NAMESPACE.as_bytes().to_vec())
        );
        assert_eq!(
            server.execute_command(RedisCommand::HcNamespace {
                namespace: Some(DEFAULT_REDIS_NAMESPACE.as_bytes().to_vec())
            }),
            RespValue::SimpleString("OK")
        );
        assert_eq!(
            server.execute_command(RedisCommand::HcNamespace {
                namespace: Some(b"other".to_vec())
            }),
            RespValue::Error(
                "ERR HC.NAMESPACE can select only the configured listener namespace".to_owned()
            )
        );
    }

    #[test]
    fn hc_tag_settags_and_invalidate_tag_use_edge_local_index_and_client_surface() {
        let server = listener();
        assert_eq!(
            server.execute_command(RedisCommand::Set {
                key: b"tagged:1".to_vec(),
                value: b"v1".to_vec(),
                options: Vec::new(),
            }),
            RespValue::SimpleString("OK")
        );
        assert_eq!(
            server.execute_command(RedisCommand::Set {
                key: b"tagged:2".to_vec(),
                value: b"v2".to_vec(),
                options: Vec::new(),
            }),
            RespValue::SimpleString("OK")
        );
        assert_eq!(
            server.execute_command(RedisCommand::Set {
                key: b"untagged".to_vec(),
                value: b"keep".to_vec(),
                options: Vec::new(),
            }),
            RespValue::SimpleString("OK")
        );

        assert_eq!(
            server.execute_command(RedisCommand::HcTag {
                key: b"tagged:1".to_vec(),
                tags: vec![b"model".to_vec(), b"shared".to_vec()],
            }),
            RespValue::Integer(2)
        );
        assert_eq!(
            server.execute_command(RedisCommand::HcTag {
                key: b"tagged:1".to_vec(),
                tags: vec![b"model".to_vec()],
            }),
            RespValue::Integer(0)
        );
        assert_eq!(
            server.execute_command(RedisCommand::HcSetTags {
                key: b"tagged:2".to_vec(),
                tags: vec![b"model".to_vec(), b"model".to_vec()],
            }),
            RespValue::Integer(1)
        );

        assert_eq!(
            server.execute_command(RedisCommand::HcInvalidateTag {
                tag: b"model".to_vec(),
            }),
            RespValue::Integer(2)
        );
        assert_eq!(
            server.execute_command(RedisCommand::Get {
                key: b"tagged:1".to_vec()
            }),
            RespValue::Null
        );
        assert_eq!(
            server.execute_command(RedisCommand::Get {
                key: b"tagged:2".to_vec()
            }),
            RespValue::Null
        );
        assert_eq!(
            server.execute_command(RedisCommand::Get {
                key: b"untagged".to_vec()
            }),
            RespValue::BulkString(b"keep".to_vec())
        );
        assert_eq!(
            server.execute_command(RedisCommand::HcInvalidateTag {
                tag: b"model".to_vec(),
            }),
            RespValue::Integer(0)
        );
        assert_eq!(server.state().state_mutations(), 5);
    }

    #[test]
    fn hc_tag_missing_key_does_not_create_metadata_or_mutate() {
        let server = listener();

        assert_eq!(
            server.execute_command(RedisCommand::HcTag {
                key: b"missing".to_vec(),
                tags: vec![b"model".to_vec()],
            }),
            RespValue::Integer(0)
        );
        assert_eq!(
            server.execute_command(RedisCommand::HcInvalidateTag {
                tag: b"model".to_vec(),
            }),
            RespValue::Integer(0)
        );
        assert_eq!(server.state().state_mutations(), 0);
    }

    #[test]
    fn hc_invalidate_tag_prunes_expired_keys_without_counting_them() {
        let state = Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap());
        state.set_cache_time_for_tests(Some(1_000));
        let server =
            RedisRespServer::new(Arc::clone(&state), RedisListenerConfig::default()).unwrap();

        assert_eq!(
            server.execute_command(RedisCommand::Set {
                key: b"expiring".to_vec(),
                value: b"v".to_vec(),
                options: vec![b"PX".to_vec(), b"10".to_vec()],
            }),
            RespValue::SimpleString("OK")
        );
        assert_eq!(
            server.execute_command(RedisCommand::HcTag {
                key: b"expiring".to_vec(),
                tags: vec![b"model".to_vec()],
            }),
            RespValue::Integer(1)
        );

        state.advance_cache_time_for_tests(20);
        assert_eq!(
            server.execute_command(RedisCommand::HcInvalidateTag {
                tag: b"model".to_vec(),
            }),
            RespValue::Integer(0)
        );
        assert_eq!(
            server.execute_command(RedisCommand::HcInvalidateTag {
                tag: b"model".to_vec(),
            }),
            RespValue::Integer(0)
        );
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

        let (tag, _) = decode_resp2_command(b"*3\r\n$6\r\nHC.TAG\r\n$1\r\nk\r\n$5\r\nmodel\r\n")
            .unwrap()
            .unwrap();
        assert_eq!(
            tag,
            RedisCommand::HcTag {
                key: b"k".to_vec(),
                tags: vec![b"model".to_vec()]
            }
        );

        let (invalidate_tag, _) =
            decode_resp2_command(b"*2\r\n$17\r\nHC.INVALIDATE_TAG\r\n$5\r\nmodel\r\n")
                .unwrap()
                .unwrap();
        assert_eq!(
            invalidate_tag,
            RedisCommand::HcInvalidateTag {
                tag: b"model".to_vec()
            }
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
    fn admin_commands_are_disabled_by_default_without_config_or_flush_mutation() {
        let state = Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap());
        let server =
            RedisRespServer::new(Arc::clone(&state), RedisListenerConfig::default()).unwrap();
        assert_eq!(
            server.execute_command(RedisCommand::Set {
                key: b"k".to_vec(),
                value: b"v".to_vec(),
                options: Vec::new(),
            }),
            RespValue::SimpleString("OK")
        );
        let dispatches_after_set = state.dispatch_attempts();
        let mutations_after_set = state.state_mutations();

        for (raw, expected, response) in [
            (
                b"*3\r\n$6\r\nCONFIG\r\n$3\r\nGET\r\n$1\r\n*\r\n".as_slice(),
                RedisCommand::AdminDisabled {
                    command: "CONFIG".to_owned(),
                    args: vec![b"GET".to_vec(), b"*".to_vec()],
                },
                RespValue::Error(
                    "NOPERM CONFIG is disabled by the HydraCache Redis facade".to_owned(),
                ),
            ),
            (
                b"*1\r\n$7\r\nFLUSHDB\r\n".as_slice(),
                RedisCommand::AdminDisabled {
                    command: "FLUSHDB".to_owned(),
                    args: Vec::new(),
                },
                RespValue::Error(
                    "NOPERM FLUSHDB is disabled by the HydraCache Redis facade".to_owned(),
                ),
            ),
            (
                b"*1\r\n$8\r\nFLUSHALL\r\n".as_slice(),
                RedisCommand::AdminDisabled {
                    command: "FLUSHALL".to_owned(),
                    args: Vec::new(),
                },
                RespValue::Error(
                    "NOPERM FLUSHALL is disabled by the HydraCache Redis facade".to_owned(),
                ),
            ),
        ] {
            let (command, consumed) = decode_resp2_command(raw).unwrap().unwrap();
            assert_eq!(consumed, raw.len());
            assert_eq!(command, expected);
            assert_eq!(server.execute_command(command), response);
            assert_eq!(state.dispatch_attempts(), dispatches_after_set);
            assert_eq!(state.state_mutations(), mutations_after_set);
        }

        assert_eq!(
            server.execute_command(RedisCommand::Get { key: b"k".to_vec() }),
            RespValue::BulkString(b"v".to_vec())
        );
        assert_eq!(state.state_mutations(), 1);
        assert_eq!(server.metrics().errors, 3);
    }

    #[test]
    fn cluster_and_moved_ask_are_never_emitted() {
        let context = RedisTranslationContext::default();
        let cluster_subcommands = [
            "INFO",
            "SLOTS",
            "NODES",
            "SHARDS",
            "KEYSLOT",
            "GETKEYSINSLOT",
        ];
        for subcommand in cluster_subcommands {
            let command = RedisCommand::Unsupported {
                verb: "CLUSTER".to_owned(),
                args: vec![subcommand.as_bytes().to_vec()],
            };
            let error = translate_redis_command(command, &context).unwrap_err();
            assert!(matches!(
                &error,
                RedisTranslationError::UnsupportedCommand { command } if command == "CLUSTER"
            ));
            let encoded =
                String::from_utf8(encode_resp2_value(error.into_resp_value()).unwrap()).unwrap();
            assert_eq!(encoded, "-ERR unsupported command CLUSTER\r\n");
            assert!(!encoded.contains("MOVED"));
            assert!(!encoded.contains("ASK"));
        }

        let non_cluster_errors = [
            translate_redis_command(
                RedisCommand::AdminDisabled {
                    command: "FLUSHDB".to_owned(),
                    args: Vec::new(),
                },
                &context,
            )
            .unwrap_err(),
            translate_redis_command(
                RedisCommand::Unsupported {
                    verb: "HSET".to_owned(),
                    args: vec![b"k".to_vec(), b"field".to_vec(), b"v".to_vec()],
                },
                &context,
            )
            .unwrap_err(),
        ];

        for error in non_cluster_errors {
            let encoded =
                String::from_utf8(encode_resp2_value(error.into_resp_value()).unwrap()).unwrap();
            assert!(!encoded.contains("MOVED"));
            assert!(!encoded.contains("ASK"));
        }
    }

    #[test]
    fn cluster_commands_decode_as_unsupported_standalone_contract() {
        for (input, expected_args) in [
            (
                b"*2\r\n$7\r\nCLUSTER\r\n$5\r\nSLOTS\r\n".as_slice(),
                vec![b"SLOTS".to_vec()],
            ),
            (
                b"*2\r\n$7\r\nCLUSTER\r\n$5\r\nNODES\r\n".as_slice(),
                vec![b"NODES".to_vec()],
            ),
            (
                b"*2\r\n$7\r\nCLUSTER\r\n$4\r\nINFO\r\n".as_slice(),
                vec![b"INFO".to_vec()],
            ),
            (
                b"*3\r\n$7\r\nCLUSTER\r\n$7\r\nKEYSLOT\r\n$3\r\nkey\r\n".as_slice(),
                vec![b"KEYSLOT".to_vec(), b"key".to_vec()],
            ),
            (
                b"*4\r\n$7\r\nCLUSTER\r\n$13\r\nGETKEYSINSLOT\r\n$1\r\n0\r\n$2\r\n10\r\n"
                    .as_slice(),
                vec![b"GETKEYSINSLOT".to_vec(), b"0".to_vec(), b"10".to_vec()],
            ),
        ] {
            let (command, consumed) = decode_resp2_command(input).unwrap().unwrap();
            assert_eq!(consumed, input.len());
            assert_eq!(
                command,
                RedisCommand::Unsupported {
                    verb: "CLUSTER".to_owned(),
                    args: expected_args
                }
            );
        }
    }

    #[test]
    fn info_role_dbsize_type_scan_and_config_follow_contract_classification() {
        let context = RedisTranslationContext::default();
        assert!(matches!(
            translate_redis_command(RedisCommand::Info { section: None }, &context),
            Ok(RedisTranslatedCommand::Immediate(RespValue::BulkString(_)))
        ));
        assert!(matches!(
            translate_redis_command(RedisCommand::Type { key: b"k".to_vec() }, &context),
            Ok(RedisTranslatedCommand::Execute(_))
        ));

        for command in ["ROLE", "DBSIZE", "SCAN", "CLIENT LIST", "CLIENT ID"] {
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

    #[tokio::test]
    async fn resp_listener_serves_pipelined_get_set_over_io() {
        let state = Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap());
        let server =
            RedisRespServer::new(Arc::clone(&state), RedisListenerConfig::default()).unwrap();
        let output = exchange(
            &server,
            b"*3\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n\
              *2\r\n$3\r\nGET\r\n$1\r\nk\r\n\
              *3\r\n$4\r\nMGET\r\n$1\r\nk\r\n$7\r\nmissing\r\n\
              *1\r\n$4\r\nQUIT\r\n",
        )
        .await;

        assert_eq!(output, b"+OK\r\n$1\r\nv\r\n*2\r\n$1\r\nv\r\n$-1\r\n+OK\r\n");
        assert_eq!(state.dispatch_attempts(), 3);
        assert_eq!(
            server.metrics(),
            RedisListenerMetrics {
                accepted_connections: 1,
                commands: 4,
                errors: 0,
            }
        );
    }

    #[tokio::test]
    async fn resp_listener_select_zero_ok_and_nonzero_keeps_default_database() {
        let state = Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap());
        let server =
            RedisRespServer::new(Arc::clone(&state), RedisListenerConfig::default()).unwrap();
        let output = exchange(
            &server,
            b"*2\r\n$6\r\nSELECT\r\n$1\r\n0\r\n\
              *3\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n\
              *2\r\n$6\r\nSELECT\r\n$1\r\n1\r\n\
              *2\r\n$3\r\nGET\r\n$1\r\nk\r\n\
              *1\r\n$4\r\nQUIT\r\n",
        )
        .await;

        assert_eq!(
            output,
            b"+OK\r\n+OK\r\n-ERR multiple Redis databases are not supported; use SELECT 0\r\n$1\r\nv\r\n+OK\r\n"
        );
        assert_eq!(state.dispatch_attempts(), 2);
        assert_eq!(
            server.metrics(),
            RedisListenerMetrics {
                accepted_connections: 1,
                commands: 5,
                errors: 1,
            }
        );
    }

    #[tokio::test]
    async fn resp_listener_info_probe_does_not_fabricate_keyspace_or_cluster_state() {
        let server = listener();
        let output = exchange(
            &server,
            b"*1\r\n$4\r\nPING\r\n\
              *1\r\n$4\r\nINFO\r\n\
              *1\r\n$4\r\nQUIT\r\n",
        )
        .await;
        let output = String::from_utf8(output).unwrap();

        assert!(output.starts_with("+PONG\r\n$"));
        assert!(output.contains("# Server\r\n"));
        assert!(output.contains("redis_mode:standalone\r\n"));
        assert!(output.contains("redis_scope:node-local\r\n"));
        assert!(output.contains("role:master\r\n"));
        assert!(output.contains("hydracache_resp:RESP2+RESP3\r\n"));
        assert!(output.contains("total_connections_received:1\r\n"));
        assert!(output.contains("total_commands_processed:1\r\n"));
        assert!(output.ends_with("+OK\r\n"));
        assert!(!output.contains("used_memory"));
        assert!(!output.contains("cluster_enabled"));
        assert!(!output.contains("db0:"));
    }

    #[tokio::test]
    async fn resp_listener_type_reports_string_and_none() {
        let server = listener();
        let output = exchange(
            &server,
            b"*3\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n\
              *2\r\n$4\r\nTYPE\r\n$1\r\nk\r\n\
              *2\r\n$4\r\nTYPE\r\n$7\r\nmissing\r\n\
              *1\r\n$4\r\nQUIT\r\n",
        )
        .await;

        assert_eq!(output, b"+OK\r\n+string\r\n+none\r\n+OK\r\n");
        assert_eq!(server.state().dispatch_attempts(), 3);
    }

    #[tokio::test]
    async fn resp_listener_admin_commands_are_disabled_before_mutation() {
        let server = listener();
        let output = exchange(
            &server,
            b"*3\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n\
              *3\r\n$6\r\nCONFIG\r\n$3\r\nGET\r\n$1\r\n*\r\n\
              *1\r\n$7\r\nFLUSHDB\r\n\
              *1\r\n$8\r\nFLUSHALL\r\n\
              *2\r\n$3\r\nGET\r\n$1\r\nk\r\n\
              *1\r\n$4\r\nQUIT\r\n",
        )
        .await;

        assert_eq!(
            output,
            b"+OK\r\n-NOPERM CONFIG is disabled by the HydraCache Redis facade\r\n-NOPERM FLUSHDB is disabled by the HydraCache Redis facade\r\n-NOPERM FLUSHALL is disabled by the HydraCache Redis facade\r\n$1\r\nv\r\n+OK\r\n"
        );
        assert_eq!(server.state().dispatch_attempts(), 2);
        assert_eq!(server.state().state_mutations(), 1);
        assert_eq!(
            server.metrics(),
            RedisListenerMetrics {
                accepted_connections: 1,
                commands: 6,
                errors: 3,
            }
        );
    }

    #[tokio::test]
    async fn resp3_commands_roundtrip_supported_cache_subset() {
        let state = Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap());
        let server =
            RedisRespServer::new(Arc::clone(&state), RedisListenerConfig::default()).unwrap();
        let output = exchange(
            &server,
            b"*2\r\n$5\r\nHELLO\r\n$1\r\n3\r\n\
              *3\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n\
              *2\r\n$3\r\nGET\r\n$1\r\nk\r\n\
              *5\r\n$4\r\nMSET\r\n$1\r\na\r\n$1\r\n1\r\n$1\r\nb\r\n$1\r\n2\r\n\
              *3\r\n$4\r\nMGET\r\n$1\r\nk\r\n$7\r\nmissing\r\n\
              *1\r\n$4\r\nQUIT\r\n",
        )
        .await;
        let output = String::from_utf8(output).unwrap();

        assert!(output.starts_with("%7\r\n"));
        assert!(output.contains("+proto\r\n:3\r\n"));
        assert!(output.ends_with("+OK\r\n$1\r\nv\r\n+OK\r\n*2\r\n$1\r\nv\r\n_\r\n+OK\r\n"));
        assert_eq!(state.dispatch_attempts(), 4);
    }

    #[tokio::test]
    async fn resp3_unsupported_aggregate_inputs_fail_before_mutation() {
        let state = Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap());
        let server =
            RedisRespServer::new(Arc::clone(&state), RedisListenerConfig::default()).unwrap();
        let output = exchange(
            &server,
            b"*2\r\n$5\r\nHELLO\r\n$1\r\n3\r\n\
              *3\r\n$3\r\nSET\r\n%1\r\n+a\r\n+b\r\n$1\r\nv\r\n",
        )
        .await;
        let output = String::from_utf8(output).unwrap();

        assert!(output.contains("+proto\r\n:3\r\n"));
        assert!(output.contains("-ERR Redis command argument must be a bulk or simple string\r\n"));
        assert_eq!(state.dispatch_attempts(), 0);
        assert_eq!(server.metrics().errors, 1);
    }

    #[tokio::test]
    async fn resp_listener_surfaces_errors_without_moved_or_ask() {
        let server = listener();
        let output = exchange(
            &server,
            b"*3\r\n$4\r\nHSET\r\n$1\r\nk\r\n$1\r\nv\r\n\
              *1\r\n$4\r\nPING\r\n\
              *1\r\n$4\r\nQUIT\r\n",
        )
        .await;
        let output = String::from_utf8(output).unwrap();

        assert!(output.contains("-ERR unsupported command HSET\r\n"));
        assert!(output.contains("+PONG\r\n"));
        assert!(output.contains("+OK\r\n"));
        assert!(!output.contains("MOVED"));
        assert!(!output.contains("ASK"));
        assert_eq!(server.metrics().errors, 1);
    }

    #[tokio::test]
    async fn cluster_mode_commands_fail_loud_over_resp_without_topology_or_redirects() {
        let server = listener();
        let output = exchange(
            &server,
            b"*2\r\n$7\r\nCLUSTER\r\n$5\r\nSLOTS\r\n\
              *2\r\n$7\r\nCLUSTER\r\n$5\r\nNODES\r\n\
              *2\r\n$7\r\nCLUSTER\r\n$4\r\nINFO\r\n\
              *2\r\n$5\r\nHELLO\r\n$1\r\n3\r\n\
              *2\r\n$7\r\nCLUSTER\r\n$5\r\nSLOTS\r\n\
              *1\r\n$4\r\nQUIT\r\n",
        )
        .await;
        let output = String::from_utf8(output).unwrap();

        assert_eq!(
            output
                .matches("-ERR unsupported command CLUSTER\r\n")
                .count(),
            4
        );
        assert!(output.contains("+proto\r\n:3\r\n"));
        assert!(output.ends_with("+OK\r\n"));
        assert!(!output.contains("MOVED"));
        assert!(!output.contains("ASK"));
        assert!(!output.contains("slot"));
        assert!(!output.contains("node"));
        assert_eq!(server.metrics().errors, 4);
    }

    #[tokio::test]
    async fn hc_stats_and_diagnostics_over_resp_are_bounded_and_redacted() {
        let server = listener();
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
        assert!(output.contains("state_mutations"));
        assert!(output.contains(SUPPORTED_RESP_DIALECT));
        assert!(output.contains(DEFAULT_REDIS_NAMESPACE));
        assert!(!output.contains("secret-key"));
        assert!(!output.contains("secret-value"));
        assert!(!output.contains("redis-resp-test"));
    }

    #[tokio::test]
    async fn auth_hello_auth_and_noauth_errors_match_contract() {
        let server = auth_listener(None);
        let output = exchange(
            &server,
            b"*2\r\n$3\r\nGET\r\n$1\r\nk\r\n\
              *2\r\n$4\r\nAUTH\r\n$5\r\nwrong\r\n\
              *2\r\n$4\r\nAUTH\r\n$6\r\nsecret\r\n\
              *3\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n\
              *2\r\n$3\r\nGET\r\n$1\r\nk\r\n\
              *1\r\n$4\r\nQUIT\r\n",
        )
        .await;
        let output = String::from_utf8(output).unwrap();

        assert!(output.starts_with("-NOAUTH Authentication required.\r\n"));
        assert!(
            output.contains("-WRONGPASS invalid username-password pair or user is disabled.\r\n")
        );
        assert!(output.contains("+OK\r\n+OK\r\n$1\r\nv\r\n+OK\r\n"));
        assert!(!output.contains("secret"));
        assert!(!output.contains("wrong"));
        assert_eq!(server.state().dispatch_attempts(), 2);
        assert_eq!(server.metrics().errors, 2);

        let hello = exchange(
            &auth_listener(None),
            b"*5\r\n$5\r\nHELLO\r\n$1\r\n2\r\n$4\r\nAUTH\r\n$7\r\ndefault\r\n$6\r\nsecret\r\n\
              *3\r\n$3\r\nSET\r\n$1\r\nh\r\n$1\r\nv\r\n\
              *2\r\n$3\r\nGET\r\n$1\r\nh\r\n\
              *1\r\n$4\r\nQUIT\r\n",
        )
        .await;
        let hello = String::from_utf8(hello).unwrap();
        assert!(hello.contains("hydracache"));
        assert!(hello.contains("$5\r\nproto\r\n:2\r\n"));
        assert!(hello.ends_with("+OK\r\n$1\r\nv\r\n+OK\r\n"));

        let hello3 = exchange(
            &auth_listener(None),
            b"*5\r\n$5\r\nHELLO\r\n$1\r\n3\r\n$4\r\nAUTH\r\n$7\r\ndefault\r\n$6\r\nsecret\r\n\
              *3\r\n$3\r\nSET\r\n$2\r\nh3\r\n$1\r\nv\r\n\
              *2\r\n$3\r\nGET\r\n$2\r\nh3\r\n\
              *1\r\n$4\r\nQUIT\r\n",
        )
        .await;
        let hello3 = String::from_utf8(hello3).unwrap();
        assert!(hello3.contains("+proto\r\n:3\r\n"));
        assert!(hello3.ends_with("+OK\r\n$1\r\nv\r\n+OK\r\n"));
    }

    #[test]
    fn redis_auth_uses_hardened_credential_comparison_contract() {
        let mut auth = RedisAuthConfig::required("secret-token");
        auth.username = Some("app-user".to_owned());

        assert!(auth.matches_attempt(&RedisAuthAttempt {
            username: Some(b"app-user".to_vec()),
            password: b"secret-token".to_vec(),
        }));
        assert!(!auth.matches_attempt(&RedisAuthAttempt {
            username: Some(b"app-user".to_vec()),
            password: b"secret-toke".to_vec(),
        }));
        assert!(!auth.matches_attempt(&RedisAuthAttempt {
            username: Some(b"app-user".to_vec()),
            password: b"secret-token-extra".to_vec(),
        }));
        assert!(!auth.matches_attempt(&RedisAuthAttempt {
            username: Some(b"other-user".to_vec()),
            password: b"secret-token".to_vec(),
        }));
        assert!(!auth.matches_attempt(&RedisAuthAttempt {
            username: Some(b"other-user".to_vec()),
            password: b"wrong-prefix".to_vec(),
        }));

        let default_user_auth = RedisAuthConfig::required("secret-token");
        assert!(default_user_auth.matches_attempt(&RedisAuthAttempt {
            username: None,
            password: b"secret-token".to_vec(),
        }));
        assert!(default_user_auth.matches_attempt(&RedisAuthAttempt {
            username: Some(b"DEFAULT".to_vec()),
            password: b"secret-token".to_vec(),
        }));
        assert!(!default_user_auth.matches_attempt(&RedisAuthAttempt {
            username: Some(b"other-user".to_vec()),
            password: b"secret-token".to_vec(),
        }));

        assert!(hardened_bytes_eq(b"secret-token", b"secret-token"));
        assert!(!hardened_bytes_eq(b"secret-token", b"secret-toke"));
        assert!(!hardened_bytes_eq(b"secret-token", b"secret-token\0"));
    }

    #[tokio::test]
    async fn redis_auth_required_listener_rejects_data_commands_before_auth() {
        let server = auth_listener(None);
        let output = exchange(
            &server,
            b"*2\r\n$3\r\nGET\r\n$1\r\nk\r\n\
              *1\r\n$8\r\nHC.STATS\r\n\
              *1\r\n$4\r\nPING\r\n\
              *1\r\n$4\r\nQUIT\r\n",
        )
        .await;
        let output = String::from_utf8(output).unwrap();

        assert_eq!(output.matches("NOAUTH Authentication required.").count(), 2);
        assert!(output.contains("+PONG\r\n"));
        assert!(output.ends_with("+OK\r\n"));
        assert_eq!(server.state().dispatch_attempts(), 0);
    }

    #[tokio::test]
    async fn redis_auth_success_binds_connection_local_client_identity() {
        let server = auth_listener(None);
        let first = exchange(
            &server,
            b"*2\r\n$4\r\nAUTH\r\n$6\r\nsecret\r\n\
              *3\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n\
              *1\r\n$4\r\nQUIT\r\n",
        )
        .await;
        assert_eq!(first, b"+OK\r\n+OK\r\n+OK\r\n");
        assert_eq!(server.state().dispatch_attempts(), 1);

        let second = exchange(
            &server,
            b"*2\r\n$3\r\nGET\r\n$1\r\nk\r\n\
              *1\r\n$4\r\nQUIT\r\n",
        )
        .await;
        assert_eq!(second, b"-NOAUTH Authentication required.\r\n+OK\r\n");
        assert_eq!(
            server.state().dispatch_attempts(),
            1,
            "unauthenticated second connection must not reach client surface"
        );
    }

    #[tokio::test]
    async fn redis_auth_redacts_credentials_from_errors_logs_and_metrics() {
        let server = auth_listener(Some("app-user"));
        let output = exchange(
            &server,
            b"*3\r\n$4\r\nAUTH\r\n$8\r\napp-user\r\n$6\r\nsecret\r\n\
              *1\r\n$14\r\nHC.DIAGNOSTICS\r\n\
              *1\r\n$4\r\nQUIT\r\n",
        )
        .await;
        let output = String::from_utf8(output).unwrap();

        assert!(output.starts_with("+OK\r\n"));
        assert!(!output.contains("secret"));
        assert!(!output.contains("app-user"));
        assert!(!format!("{:?}", server.config.auth).contains("secret"));
        assert!(!format!("{:?}", server.metrics()).contains("secret"));
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

    fn listener() -> RedisRespServer {
        let config = RedisListenerConfig {
            client_id: "redis-resp-test".to_owned(),
            ..RedisListenerConfig::default()
        };
        RedisRespServer::new(
            Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap()),
            config,
        )
        .unwrap()
    }

    fn auth_listener(username: Option<&str>) -> RedisRespServer {
        let mut auth = RedisAuthConfig::required("secret");
        auth.username = username.map(ToOwned::to_owned);
        let config = RedisListenerConfig {
            client_id: "redis-resp-test".to_owned(),
            auth,
            ..RedisListenerConfig::default()
        };
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
}
