//! Stable external client protocol primitives.
//!
//! Release 0.49 starts the external-consumer surface by reserving a small,
//! deterministic frame contract and golden fixtures. W1 expands the payload
//! schema; W0 keeps the compatibility substrate intentionally narrow.

use bytes::Bytes;
use thiserror::Error;

/// First supported external client protocol version.
pub const PROTOCOL_VERSION: u16 = 1;

/// Bytes used by the unsigned length prefix.
pub const LENGTH_PREFIX_BYTES: usize = 4;

/// Bytes used by the protocol-version field inside the frame body.
pub const VERSION_BYTES: usize = 2;

/// Smallest complete frame: length prefix plus version.
pub const MIN_FRAME_BYTES: usize = LENGTH_PREFIX_BYTES + VERSION_BYTES;

/// A length-prefixed external client frame.
///
/// The wire shape is:
///
/// ```text
/// u32 body_len_be | u16 protocol_version_be | payload bytes
/// ```
///
/// `body_len` includes the version field and the payload. Unknown future
/// protocol versions are rejected loud, matching RULES R-3/R-4.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientFrame {
    protocol_version: u16,
    payload: Bytes,
}

impl ClientFrame {
    /// Build a v1 frame.
    pub fn new(payload: impl Into<Bytes>) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            payload: payload.into(),
        }
    }

    /// Build a frame with an explicit protocol version for compatibility tests.
    pub fn with_version(protocol_version: u16, payload: impl Into<Bytes>) -> Self {
        Self {
            protocol_version,
            payload: payload.into(),
        }
    }

    /// Return the frame protocol version.
    pub fn protocol_version(&self) -> u16 {
        self.protocol_version
    }

    /// Return the opaque payload bytes.
    pub fn payload(&self) -> &Bytes {
        &self.payload
    }

    /// Encode the frame with a big-endian length prefix.
    pub fn encode(&self) -> Result<Bytes, ClientProtocolError> {
        let body_len = VERSION_BYTES.checked_add(self.payload.len()).ok_or(
            ClientProtocolError::FrameTooLarge {
                actual: usize::MAX,
                max: u32::MAX as usize,
            },
        )?;
        if body_len > u32::MAX as usize {
            return Err(ClientProtocolError::FrameTooLarge {
                actual: body_len,
                max: u32::MAX as usize,
            });
        }

        let mut out = Vec::with_capacity(LENGTH_PREFIX_BYTES + body_len);
        out.extend_from_slice(&(body_len as u32).to_be_bytes());
        out.extend_from_slice(&self.protocol_version.to_be_bytes());
        out.extend_from_slice(&self.payload);
        Ok(Bytes::from(out))
    }

    /// Decode and validate a frame.
    pub fn decode(bytes: &[u8], max_frame_bytes: usize) -> Result<Self, ClientProtocolError> {
        if bytes.len() > max_frame_bytes {
            return Err(ClientProtocolError::FrameTooLarge {
                actual: bytes.len(),
                max: max_frame_bytes,
            });
        }
        if bytes.len() < MIN_FRAME_BYTES {
            return Err(ClientProtocolError::TruncatedFrame {
                actual: bytes.len(),
                needed: MIN_FRAME_BYTES,
            });
        }

        let body_len = u32::from_be_bytes(
            bytes[0..LENGTH_PREFIX_BYTES]
                .try_into()
                .expect("slice length is checked"),
        ) as usize;
        if body_len < VERSION_BYTES {
            return Err(ClientProtocolError::TruncatedFrame {
                actual: body_len,
                needed: VERSION_BYTES,
            });
        }

        let expected = LENGTH_PREFIX_BYTES + body_len;
        if expected != bytes.len() {
            return Err(ClientProtocolError::LengthMismatch {
                declared: body_len,
                actual: bytes.len().saturating_sub(LENGTH_PREFIX_BYTES),
            });
        }

        let version_start = LENGTH_PREFIX_BYTES;
        let version_end = version_start + VERSION_BYTES;
        let protocol_version = u16::from_be_bytes(
            bytes[version_start..version_end]
                .try_into()
                .expect("slice length is checked"),
        );
        if protocol_version > PROTOCOL_VERSION {
            return Err(ClientProtocolError::UnsupportedVersion {
                version: protocol_version,
                supported_max: PROTOCOL_VERSION,
            });
        }

        Ok(Self {
            protocol_version,
            payload: Bytes::copy_from_slice(&bytes[version_end..]),
        })
    }
}

/// External client protocol decode/encode errors.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ClientProtocolError {
    /// Frame exceeds the configured limit.
    #[error("client frame is {actual} bytes, exceeding max_frame_bytes={max}")]
    FrameTooLarge {
        /// Observed frame length.
        actual: usize,
        /// Configured limit.
        max: usize,
    },
    /// Not enough bytes were supplied to parse a complete frame.
    #[error("truncated client frame: {actual} bytes available, {needed} needed")]
    TruncatedFrame {
        /// Observed frame length.
        actual: usize,
        /// Required frame length.
        needed: usize,
    },
    /// The length prefix and supplied bytes disagree.
    #[error(
        "client frame length mismatch: declared body {declared} bytes, actual body {actual} bytes"
    )]
    LengthMismatch {
        /// Body length from the prefix.
        declared: usize,
        /// Body length present after the prefix.
        actual: usize,
    },
    /// The frame is from a future protocol version.
    #[error("unsupported client protocol version {version}; supported max is {supported_max}")]
    UnsupportedVersion {
        /// Version from the frame.
        version: u16,
        /// Highest version this reader supports.
        supported_max: u16,
    },
}
