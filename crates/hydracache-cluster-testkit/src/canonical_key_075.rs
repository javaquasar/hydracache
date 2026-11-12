//! Provisional cross-language canonical-key framing.
//!
//! This is a reference codec only. It intentionally defines neither the production partition hash
//! nor a wire/durable format identity.

const DOMAIN: &[u8; 8] = b"HCRK075\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReferenceKeyBounds {
    pub max_tenant_bytes: usize,
    pub max_namespace_bytes: usize,
    pub max_key_bytes: usize,
    pub max_frame_bytes: usize,
}

impl Default for ReferenceKeyBounds {
    fn default() -> Self {
        Self {
            max_tenant_bytes: 256,
            max_namespace_bytes: 256,
            max_key_bytes: 1024 * 1024,
            max_frame_bytes: 1024 * 1024 + 1024,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceKeyError {
    InvalidBound(&'static str),
    EmptyTenant,
    EmptyNamespace,
    ZeroGeneration,
    FieldTooLarge {
        field: &'static str,
        limit: usize,
        actual: usize,
    },
    FrameTooLarge {
        limit: usize,
        actual: usize,
    },
    MalformedFrame,
    InvalidUtf8(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceCanonicalKey {
    pub tenant: String,
    pub namespace: String,
    pub namespace_generation: u64,
    pub key: Vec<u8>,
}

impl ReferenceCanonicalKey {
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

    pub fn encode(&self, bounds: ReferenceKeyBounds) -> Result<Vec<u8>, ReferenceKeyError> {
        validate_bounds(bounds)?;
        if self.tenant.is_empty() {
            return Err(ReferenceKeyError::EmptyTenant);
        }
        if self.namespace.is_empty() {
            return Err(ReferenceKeyError::EmptyNamespace);
        }
        if self.namespace_generation == 0 {
            return Err(ReferenceKeyError::ZeroGeneration);
        }
        check_field("tenant", self.tenant.len(), bounds.max_tenant_bytes)?;
        check_field(
            "namespace",
            self.namespace.len(),
            bounds.max_namespace_bytes,
        )?;
        check_field("key", self.key.len(), bounds.max_key_bytes)?;
        let size = DOMAIN
            .len()
            .checked_add(4 + self.tenant.len())
            .and_then(|size| size.checked_add(4 + self.namespace.len()))
            .and_then(|size| size.checked_add(8))
            .and_then(|size| size.checked_add(4 + self.key.len()))
            .ok_or(ReferenceKeyError::FrameTooLarge {
                limit: bounds.max_frame_bytes,
                actual: usize::MAX,
            })?;
        if size > bounds.max_frame_bytes {
            return Err(ReferenceKeyError::FrameTooLarge {
                limit: bounds.max_frame_bytes,
                actual: size,
            });
        }
        let mut encoded = Vec::with_capacity(size);
        encoded.extend_from_slice(DOMAIN);
        push_bytes(&mut encoded, self.tenant.as_bytes())?;
        push_bytes(&mut encoded, self.namespace.as_bytes())?;
        encoded.extend_from_slice(&self.namespace_generation.to_be_bytes());
        push_bytes(&mut encoded, &self.key)?;
        Ok(encoded)
    }

    pub fn decode(frame: &[u8], bounds: ReferenceKeyBounds) -> Result<Self, ReferenceKeyError> {
        validate_bounds(bounds)?;
        if frame.len() > bounds.max_frame_bytes {
            return Err(ReferenceKeyError::FrameTooLarge {
                limit: bounds.max_frame_bytes,
                actual: frame.len(),
            });
        }
        let mut cursor = Cursor::new(frame);
        if cursor.take(DOMAIN.len())? != DOMAIN {
            return Err(ReferenceKeyError::MalformedFrame);
        }
        let tenant = cursor.take_bounded_string("tenant", bounds.max_tenant_bytes)?;
        let namespace = cursor.take_bounded_string("namespace", bounds.max_namespace_bytes)?;
        let generation = u64::from_be_bytes(
            cursor
                .take(8)?
                .try_into()
                .map_err(|_| ReferenceKeyError::MalformedFrame)?,
        );
        let key = cursor.take_bounded("key", bounds.max_key_bytes)?.to_vec();
        if !cursor.is_empty() {
            return Err(ReferenceKeyError::MalformedFrame);
        }
        let decoded = Self::new(tenant, namespace, generation, key);
        decoded.encode(bounds)?;
        Ok(decoded)
    }
}

fn push_bytes(target: &mut Vec<u8>, value: &[u8]) -> Result<(), ReferenceKeyError> {
    let length = u32::try_from(value.len()).map_err(|_| ReferenceKeyError::MalformedFrame)?;
    target.extend_from_slice(&length.to_be_bytes());
    target.extend_from_slice(value);
    Ok(())
}

fn validate_bounds(bounds: ReferenceKeyBounds) -> Result<(), ReferenceKeyError> {
    for (name, value) in [
        ("tenant", bounds.max_tenant_bytes),
        ("namespace", bounds.max_namespace_bytes),
        ("key", bounds.max_key_bytes),
        ("frame", bounds.max_frame_bytes),
    ] {
        if value == 0 {
            return Err(ReferenceKeyError::InvalidBound(name));
        }
    }
    Ok(())
}

fn check_field(field: &'static str, actual: usize, limit: usize) -> Result<(), ReferenceKeyError> {
    if actual > limit {
        Err(ReferenceKeyError::FieldTooLarge {
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

    fn take(&mut self, length: usize) -> Result<&'a [u8], ReferenceKeyError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or(ReferenceKeyError::MalformedFrame)?;
        let value = self
            .frame
            .get(self.offset..end)
            .ok_or(ReferenceKeyError::MalformedFrame)?;
        self.offset = end;
        Ok(value)
    }

    fn take_bounded(
        &mut self,
        field: &'static str,
        limit: usize,
    ) -> Result<&'a [u8], ReferenceKeyError> {
        let length = u32::from_be_bytes(
            self.take(4)?
                .try_into()
                .map_err(|_| ReferenceKeyError::MalformedFrame)?,
        ) as usize;
        check_field(field, length, limit)?;
        self.take(length)
    }

    fn take_bounded_string(
        &mut self,
        field: &'static str,
        limit: usize,
    ) -> Result<String, ReferenceKeyError> {
        String::from_utf8(self.take_bounded(field, limit)?.to_vec())
            .map_err(|_| ReferenceKeyError::InvalidUtf8(field))
    }

    fn is_empty(&self) -> bool {
        self.offset == self.frame.len()
    }
}
