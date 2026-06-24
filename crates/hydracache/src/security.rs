use std::error::Error;
use std::fmt;

/// Current at-rest sealed artifact format.
pub const AT_REST_ARTIFACT_FORMAT_VERSION: u16 = 1;

/// Operator-supplied key material.
#[derive(Clone, PartialEq, Eq)]
pub struct KeyMaterial {
    key_id: String,
    secret: Vec<u8>,
}

impl KeyMaterial {
    /// Create key material after validating the id and secret are explicit.
    pub fn new(
        key_id: impl Into<String>,
        secret: impl Into<Vec<u8>>,
    ) -> Result<Self, SecurityError> {
        let key_id = key_id.into();
        let secret = secret.into();
        if key_id.trim().is_empty() {
            return Err(SecurityError::InvalidKeyMaterial("empty key id"));
        }
        if secret.is_empty() {
            return Err(SecurityError::InvalidKeyMaterial("empty key secret"));
        }
        Ok(Self { key_id, secret })
    }

    /// Stable key id persisted next to a sealed artifact.
    pub fn key_id(&self) -> &str {
        &self.key_id
    }
}

impl fmt::Debug for KeyMaterial {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("KeyMaterial")
            .field("key_id", &self.key_id)
            .field("secret_len", &self.secret.len())
            .finish_non_exhaustive()
    }
}

/// Provider responsible for current and previous at-rest keys.
pub trait AtRestKeyProvider: Send + Sync {
    /// Return the key used to seal new artifacts.
    fn current_key(&self) -> Result<KeyMaterial, SecurityError>;

    /// Return a key by id for opening an existing artifact.
    fn key(&self, key_id: &str) -> Result<KeyMaterial, SecurityError>;
}

/// Static key provider useful for tests and operator-wired deployments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticAtRestKeyProvider {
    current: KeyMaterial,
    previous: Vec<KeyMaterial>,
}

impl StaticAtRestKeyProvider {
    /// Create a provider with one current key.
    pub fn new(
        key_id: impl Into<String>,
        secret: impl Into<Vec<u8>>,
    ) -> Result<Self, SecurityError> {
        Ok(Self {
            current: KeyMaterial::new(key_id, secret)?,
            previous: Vec::new(),
        })
    }

    /// Add one previous key accepted for reads during a rotation window.
    pub fn with_previous_key(
        mut self,
        key_id: impl Into<String>,
        secret: impl Into<Vec<u8>>,
    ) -> Result<Self, SecurityError> {
        self.previous.push(KeyMaterial::new(key_id, secret)?);
        Ok(self)
    }
}

impl AtRestKeyProvider for StaticAtRestKeyProvider {
    fn current_key(&self) -> Result<KeyMaterial, SecurityError> {
        Ok(self.current.clone())
    }

    fn key(&self, key_id: &str) -> Result<KeyMaterial, SecurityError> {
        if self.current.key_id == key_id {
            return Ok(self.current.clone());
        }
        self.previous
            .iter()
            .find(|key| key.key_id == key_id)
            .cloned()
            .ok_or_else(|| SecurityError::UnknownKey(key_id.to_owned()))
    }
}

/// Persistable sealed at-rest artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedArtifact {
    /// Artifact format version.
    pub format_version: u16,
    /// Logical artifact type, such as `snapshot` or `pitr-log`.
    pub artifact_kind: String,
    /// Key id used to seal the ciphertext.
    pub key_id: String,
    /// Deterministic nonce used by the sealing boundary.
    pub nonce: u64,
    /// Checksum of the plaintext after opening.
    pub plaintext_checksum: u64,
    /// Sealed bytes that may be persisted.
    pub ciphertext: Vec<u8>,
}

/// At-rest artifact sealer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtRestSealer<P> {
    provider: P,
}

impl<P> AtRestSealer<P>
where
    P: AtRestKeyProvider,
{
    /// Create a sealer from an operator-owned provider.
    pub fn new(provider: P) -> Self {
        Self { provider }
    }

    /// Seal plaintext bytes into a persistable artifact.
    pub fn seal(
        &self,
        artifact_kind: impl Into<String>,
        plaintext: &[u8],
    ) -> Result<SealedArtifact, SecurityError> {
        let artifact_kind = artifact_kind.into();
        if artifact_kind.trim().is_empty() {
            return Err(SecurityError::InvalidArtifactKind);
        }
        let key = self.provider.current_key()?;
        let nonce = checksum_parts(&[artifact_kind.as_bytes(), key.key_id.as_bytes(), plaintext]);
        let ciphertext = apply_stream(plaintext, &key.secret, nonce);
        Ok(SealedArtifact {
            format_version: AT_REST_ARTIFACT_FORMAT_VERSION,
            artifact_kind,
            key_id: key.key_id,
            nonce,
            plaintext_checksum: checksum_parts(&[plaintext]),
            ciphertext,
        })
    }

    /// Open a sealed artifact and validate its checksum.
    pub fn open(&self, artifact: &SealedArtifact) -> Result<Vec<u8>, SecurityError> {
        if artifact.format_version != AT_REST_ARTIFACT_FORMAT_VERSION {
            return Err(SecurityError::UnsupportedArtifactFormat(
                artifact.format_version,
            ));
        }
        if artifact.artifact_kind.trim().is_empty() {
            return Err(SecurityError::InvalidArtifactKind);
        }
        let key = self.provider.key(&artifact.key_id)?;
        let plaintext = apply_stream(&artifact.ciphertext, &key.secret, artifact.nonce);
        if checksum_parts(&[&plaintext]) != artifact.plaintext_checksum {
            return Err(SecurityError::UndecryptableArtifact);
        }
        Ok(plaintext)
    }
}

/// Certificate material tracked during a rotation window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertificateBundle {
    /// Stable certificate id or fingerprint.
    pub cert_id: String,
    /// Subject used for diagnostics.
    pub subject: String,
    /// Expiration time in Unix epoch seconds.
    pub not_after_epoch_secs: u64,
}

impl CertificateBundle {
    /// Create certificate metadata.
    pub fn new(
        cert_id: impl Into<String>,
        subject: impl Into<String>,
        not_after_epoch_secs: u64,
    ) -> Result<Self, SecurityError> {
        let cert_id = cert_id.into();
        if cert_id.trim().is_empty() {
            return Err(SecurityError::InvalidCertificate("empty certificate id"));
        }
        Ok(Self {
            cert_id,
            subject: subject.into(),
            not_after_epoch_secs,
        })
    }

    fn is_valid_at(&self, now_epoch_secs: u64) -> bool {
        self.not_after_epoch_secs > now_epoch_secs
    }
}

/// Certificate rotation window that accepts current and explicitly retained previous certs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertificateRotationWindow {
    current: CertificateBundle,
    previous: Vec<CertificateBundle>,
}

impl CertificateRotationWindow {
    /// Create a rotation window with one current certificate.
    pub fn new(current: CertificateBundle) -> Self {
        Self {
            current,
            previous: Vec::new(),
        }
    }

    /// Add a previous certificate accepted during rollout.
    pub fn with_previous(mut self, previous: CertificateBundle) -> Self {
        self.previous.push(previous);
        self
    }

    /// Promote a new current certificate while retaining the old one.
    pub fn promote(mut self, current: CertificateBundle) -> Self {
        self.previous.push(self.current);
        self.current = current;
        self
    }

    /// Return whether the certificate id is accepted at the supplied time.
    pub fn accepts(&self, cert_id: &str, now_epoch_secs: u64) -> bool {
        self.current.cert_id == cert_id && self.current.is_valid_at(now_epoch_secs)
            || self
                .previous
                .iter()
                .any(|cert| cert.cert_id == cert_id && cert.is_valid_at(now_epoch_secs))
    }

    /// Current certificate id.
    pub fn current_id(&self) -> &str {
        &self.current.cert_id
    }
}

/// Security lifecycle errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecurityError {
    /// Key material is incomplete.
    InvalidKeyMaterial(&'static str),
    /// The artifact kind must be explicit.
    InvalidArtifactKind,
    /// The artifact was sealed with an unknown key id.
    UnknownKey(String),
    /// The artifact format is unsupported.
    UnsupportedArtifactFormat(u16),
    /// Ciphertext could not be opened and verified.
    UndecryptableArtifact,
    /// Certificate metadata is incomplete.
    InvalidCertificate(&'static str),
}

impl fmt::Display for SecurityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidKeyMaterial(reason) => {
                write!(formatter, "invalid at-rest key material: {reason}")
            }
            Self::InvalidArtifactKind => formatter.write_str("artifact kind must be non-empty"),
            Self::UnknownKey(key_id) => write!(formatter, "unknown at-rest key id: {key_id}"),
            Self::UnsupportedArtifactFormat(version) => {
                write!(formatter, "unsupported at-rest artifact format: {version}")
            }
            Self::UndecryptableArtifact => {
                formatter.write_str("sealed artifact could not be opened and verified")
            }
            Self::InvalidCertificate(reason) => write!(formatter, "invalid certificate: {reason}"),
        }
    }
}

impl Error for SecurityError {}

fn apply_stream(input: &[u8], secret: &[u8], nonce: u64) -> Vec<u8> {
    let nonce = nonce.to_le_bytes();
    input
        .iter()
        .enumerate()
        .map(|(index, byte)| {
            let secret_byte = secret[index % secret.len()];
            let nonce_byte = nonce[index % nonce.len()];
            byte ^ secret_byte ^ nonce_byte ^ (index as u8).wrapping_mul(31)
        })
        .collect()
}

fn checksum_parts(parts: &[&[u8]]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for part in parts {
        for byte in *part {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        hash ^= 0xff;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}
