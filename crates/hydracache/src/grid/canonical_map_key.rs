//! Bounded binary map-key framing for the D2 value-plane work.
//!
//! This provisional framing is shared with the cross-language conformance tests.
//! Its production wire and durable identities are assigned separately.

use std::mem::size_of;

const DOMAIN: &[u8; 8] = b"HCRK075\0";
const HEADER_BYTES: usize = DOMAIN.len() + 3 * size_of::<u32>() + size_of::<u64>();

/// Byte limits for each component and the complete frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanonicalMapKeyBounds {
    /// Maximum tenant name length in UTF-8 bytes.
    pub max_tenant_bytes: usize,
    /// Maximum namespace name length in UTF-8 bytes.
    pub max_namespace_bytes: usize,
    /// Maximum opaque key length in bytes.
    pub max_key_bytes: usize,
    /// Maximum complete frame length, including its header.
    pub max_frame_bytes: usize,
}

impl Default for CanonicalMapKeyBounds {
    fn default() -> Self {
        Self {
            max_tenant_bytes: 256,
            max_namespace_bytes: 256,
            max_key_bytes: 1024 * 1024,
            max_frame_bytes: 1024 * 1024 + 1024,
        }
    }
}

/// Failure to encode or decode a provisional binary map key.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CanonicalMapKeyError {
    #[error("the {0} limit must be positive")]
    InvalidBound(&'static str),
    #[error("tenant is empty")]
    EmptyTenant,
    #[error("namespace is empty")]
    EmptyNamespace,
    #[error("namespace generation must be positive")]
    ZeroGeneration,
    #[error("{field} has {actual} bytes, exceeding its {limit}-byte limit")]
    FieldTooLarge {
        field: &'static str,
        limit: usize,
        actual: usize,
    },
    #[error("frame has {actual} bytes, exceeding its {limit}-byte limit")]
    FrameTooLarge { limit: usize, actual: usize },
    #[error("malformed canonical map key")]
    MalformedFrame,
    #[error("{0} is not valid UTF-8")]
    InvalidUtf8(&'static str),
}

/// The tenant, namespace incarnation and opaque key bytes that identify a map entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalMapKey {
    /// Tenant identity, preserved byte-for-byte as UTF-8.
    pub tenant: String,
    /// Namespace name, preserved byte-for-byte as UTF-8.
    pub namespace: String,
    /// Generation of this namespace incarnation.
    pub namespace_generation: u64,
    /// Opaque key bytes, including an empty key where the caller permits it.
    pub key: Vec<u8>,
}

impl CanonicalMapKey {
    /// Collect the components. Encoding checks identity and byte limits.
    pub fn new(
        tenant: impl Into<String>,
        namespace: impl Into<String>,
        namespace_generation: u64,
        key: Vec<u8>,
    ) -> Self {
        Self {
            tenant: tenant.into(),
            namespace: namespace.into(),
            namespace_generation,
            key,
        }
    }

    /// Encode the bounded components with big-endian lengths and generation.
    pub fn encode(&self, bounds: CanonicalMapKeyBounds) -> Result<Vec<u8>, CanonicalMapKeyError> {
        validate_bounds(bounds)?;
        validate_identity(&self.tenant, &self.namespace, self.namespace_generation)?;
        check_field("tenant", self.tenant.len(), bounds.max_tenant_bytes)?;
        check_field(
            "namespace",
            self.namespace.len(),
            bounds.max_namespace_bytes,
        )?;
        check_field("key", self.key.len(), bounds.max_key_bytes)?;
        let size = HEADER_BYTES
            .checked_add(self.tenant.len())
            .and_then(|size| size.checked_add(self.namespace.len()))
            .and_then(|size| size.checked_add(self.key.len()))
            .ok_or(CanonicalMapKeyError::FrameTooLarge {
                limit: bounds.max_frame_bytes,
                actual: usize::MAX,
            })?;
        if size > bounds.max_frame_bytes {
            return Err(CanonicalMapKeyError::FrameTooLarge {
                limit: bounds.max_frame_bytes,
                actual: size,
            });
        }
        let tenant_length = frame_length(self.tenant.len())?;
        let namespace_length = frame_length(self.namespace.len())?;
        let key_length = frame_length(self.key.len())?;
        let mut encoded = Vec::with_capacity(size);
        encoded.extend_from_slice(DOMAIN);
        push_bytes(&mut encoded, tenant_length, self.tenant.as_bytes());
        push_bytes(&mut encoded, namespace_length, self.namespace.as_bytes());
        encoded.extend_from_slice(&self.namespace_generation.to_be_bytes());
        push_bytes(&mut encoded, key_length, &self.key);
        Ok(encoded)
    }

    /// Validate the complete frame before copying its components into an owned key.
    pub fn decode(
        frame: &[u8],
        bounds: CanonicalMapKeyBounds,
    ) -> Result<Self, CanonicalMapKeyError> {
        validate_bounds(bounds)?;
        if frame.len() > bounds.max_frame_bytes {
            return Err(CanonicalMapKeyError::FrameTooLarge {
                limit: bounds.max_frame_bytes,
                actual: frame.len(),
            });
        }
        let mut cursor = Cursor::new(frame);
        if cursor.take(DOMAIN.len())? != DOMAIN {
            return Err(CanonicalMapKeyError::MalformedFrame);
        }
        let tenant = cursor.take_bounded_string("tenant", bounds.max_tenant_bytes)?;
        let namespace = cursor.take_bounded_string("namespace", bounds.max_namespace_bytes)?;
        let generation = u64::from_be_bytes(
            cursor
                .take(8)?
                .try_into()
                .map_err(|_| CanonicalMapKeyError::MalformedFrame)?,
        );
        let key = cursor.take_bounded("key", bounds.max_key_bytes)?;
        if !cursor.is_empty() {
            return Err(CanonicalMapKeyError::MalformedFrame);
        }
        validate_identity(tenant, namespace, generation)?;
        Ok(Self::new(tenant, namespace, generation, key.to_vec()))
    }
}

fn frame_length(length: usize) -> Result<u32, CanonicalMapKeyError> {
    u32::try_from(length).map_err(|_| CanonicalMapKeyError::MalformedFrame)
}

fn push_bytes(target: &mut Vec<u8>, length: u32, value: &[u8]) {
    target.extend_from_slice(&length.to_be_bytes());
    target.extend_from_slice(value);
}

fn validate_identity(
    tenant: &str,
    namespace: &str,
    generation: u64,
) -> Result<(), CanonicalMapKeyError> {
    if tenant.is_empty() {
        return Err(CanonicalMapKeyError::EmptyTenant);
    }
    if namespace.is_empty() {
        return Err(CanonicalMapKeyError::EmptyNamespace);
    }
    if generation == 0 {
        return Err(CanonicalMapKeyError::ZeroGeneration);
    }
    Ok(())
}

fn validate_bounds(bounds: CanonicalMapKeyBounds) -> Result<(), CanonicalMapKeyError> {
    for (name, value) in [
        ("tenant", bounds.max_tenant_bytes),
        ("namespace", bounds.max_namespace_bytes),
        ("key", bounds.max_key_bytes),
        ("frame", bounds.max_frame_bytes),
    ] {
        if value == 0 {
            return Err(CanonicalMapKeyError::InvalidBound(name));
        }
    }
    Ok(())
}

fn check_field(
    field: &'static str,
    actual: usize,
    limit: usize,
) -> Result<(), CanonicalMapKeyError> {
    if actual > limit {
        Err(CanonicalMapKeyError::FieldTooLarge {
            field,
            limit,
            actual,
        })
    } else {
        Ok(())
    }
}

struct Cursor<'a> {
    frame: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn new(frame: &'a [u8]) -> Self {
        Self { frame, offset: 0 }
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], CanonicalMapKeyError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or(CanonicalMapKeyError::MalformedFrame)?;
        let value = self
            .frame
            .get(self.offset..end)
            .ok_or(CanonicalMapKeyError::MalformedFrame)?;
        self.offset = end;
        Ok(value)
    }

    fn take_bounded(
        &mut self,
        field: &'static str,
        limit: usize,
    ) -> Result<&'a [u8], CanonicalMapKeyError> {
        let length = u32::from_be_bytes(
            self.take(4)?
                .try_into()
                .map_err(|_| CanonicalMapKeyError::MalformedFrame)?,
        ) as usize;
        check_field(field, length, limit)?;
        self.take(length)
    }

    fn take_bounded_string(
        &mut self,
        field: &'static str,
        limit: usize,
    ) -> Result<&'a str, CanonicalMapKeyError> {
        std::str::from_utf8(self.take_bounded(field, limit)?)
            .map_err(|_| CanonicalMapKeyError::InvalidUtf8(field))
    }

    fn is_empty(&self) -> bool {
        self.offset == self.frame.len()
    }
}
