//! Signed, canonical, replay-safe offline discovery for HC/2.

use std::collections::BTreeMap;
use std::fmt;

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use thiserror::Error;

use super::*;

const DOMAIN: &[u8] = b"HYDRACACHE-HC2-DISCOVERY\0";
const FORMAT_VERSION: u16 = 1;
const ALGORITHM_ED25519: u16 = 1;
const SIGNATURE_BYTES: usize = 64;
const PUBLIC_KEY_BYTES: usize = 32;
const MAX_ARTIFACT_BYTES: usize = 1024 * 1024;
const MAX_KEY_ID_BYTES: usize = 64;
const MAX_TRUSTED_KEYS: usize = 8;
const MAX_LIFETIME_SECS: u64 = 7 * 24 * 60 * 60;
const MAX_CLOCK_SKEW_SECS: u64 = 5 * 60;

/// Stable, bounded identifier for one offline-discovery signing key.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DiscoveryKeyId(String);

impl DiscoveryKeyId {
    pub fn parse(value: impl Into<String>) -> Result<Self, SignedDiscoveryError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > MAX_KEY_ID_BYTES
            || !value.is_ascii()
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(SignedDiscoveryError::InvalidKeyId);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Public trust-root entry. It contains no signing material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedDiscoveryKey {
    key_id: DiscoveryKeyId,
    verifying_key: VerifyingKey,
}

impl TrustedDiscoveryKey {
    pub fn from_bytes(
        key_id: impl Into<String>,
        public_key: [u8; PUBLIC_KEY_BYTES],
    ) -> Result<Self, SignedDiscoveryError> {
        Ok(Self {
            key_id: DiscoveryKeyId::parse(key_id)?,
            verifying_key: VerifyingKey::from_bytes(&public_key)
                .map_err(|_| SignedDiscoveryError::InvalidPublicKey)?,
        })
    }

    pub fn key_id(&self) -> &DiscoveryKeyId {
        &self.key_id
    }

    pub fn public_key_bytes(&self) -> [u8; PUBLIC_KEY_BYTES] {
        self.verifying_key.to_bytes()
    }
}

/// Offline signing key. Debug output deliberately exposes only its key ID.
pub struct DiscoverySigningKey {
    key_id: DiscoveryKeyId,
    signing_key: SigningKey,
}

impl fmt::Debug for DiscoverySigningKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DiscoverySigningKey")
            .field("key_id", &self.key_id)
            .finish_non_exhaustive()
    }
}

impl DiscoverySigningKey {
    /// Load a deterministic 32-byte Ed25519 seed from an external secret store.
    pub fn from_seed(
        key_id: impl Into<String>,
        seed: [u8; 32],
    ) -> Result<Self, SignedDiscoveryError> {
        Ok(Self {
            key_id: DiscoveryKeyId::parse(key_id)?,
            signing_key: SigningKey::from_bytes(&seed),
        })
    }

    pub fn trusted_key(&self) -> TrustedDiscoveryKey {
        TrustedDiscoveryKey {
            key_id: self.key_id.clone(),
            verifying_key: self.signing_key.verifying_key(),
        }
    }

    pub fn sign(
        &self,
        mut advertisement: DiscoveryAdvertisement,
        issued_at_unix_secs: u64,
        expires_at_unix_secs: u64,
    ) -> Result<SignedDiscoveryArtifact, SignedDiscoveryError> {
        advertisement.validate_payload()?;
        normalize_advertisement(&mut advertisement);
        validate_window(issued_at_unix_secs, expires_at_unix_secs)?;
        let mut artifact = SignedDiscoveryArtifact {
            key_id: self.key_id.clone(),
            issued_at_unix_secs,
            expires_at_unix_secs,
            advertisement,
            signature: [0; SIGNATURE_BYTES],
        };
        let canonical = artifact.canonical_message()?;
        artifact.signature = self.signing_key.sign(&canonical).to_bytes();
        Ok(artifact)
    }
}

/// Bounded trust store supporting an explicit old/new rotation overlap.
#[derive(Debug, Clone, Default)]
pub struct DiscoveryTrustStore {
    keys: BTreeMap<DiscoveryKeyId, VerifyingKey>,
}

impl DiscoveryTrustStore {
    pub fn new<I>(keys: I) -> Result<Self, SignedDiscoveryError>
    where
        I: IntoIterator<Item = TrustedDiscoveryKey>,
    {
        let mut store = Self::default();
        for key in keys {
            store.add_key(key)?;
        }
        Ok(store)
    }

    pub fn add_key(&mut self, key: TrustedDiscoveryKey) -> Result<(), SignedDiscoveryError> {
        if self.keys.contains_key(&key.key_id) {
            return Err(SignedDiscoveryError::DuplicateKeyId);
        }
        if self.keys.len() >= MAX_TRUSTED_KEYS {
            return Err(SignedDiscoveryError::TrustStoreLimit);
        }
        self.keys.insert(key.key_id, key.verifying_key);
        Ok(())
    }

    pub fn remove_key(&mut self, key_id: &DiscoveryKeyId) -> bool {
        self.keys.remove(key_id).is_some()
    }

    pub fn contains(&self, key_id: &DiscoveryKeyId) -> bool {
        self.keys.contains_key(key_id)
    }
}

/// Verification limits and the expected cluster identity for offline discovery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedDiscoveryVerifier {
    expected_cluster_id: String,
    max_lifetime_secs: u64,
    max_clock_skew_secs: u64,
}

impl SignedDiscoveryVerifier {
    pub fn new(
        expected_cluster_id: impl Into<String>,
        max_lifetime_secs: u64,
        max_clock_skew_secs: u64,
    ) -> Result<Self, SignedDiscoveryError> {
        let expected_cluster_id = expected_cluster_id.into();
        if !valid_cluster_id(&expected_cluster_id)
            || max_lifetime_secs == 0
            || max_lifetime_secs > MAX_LIFETIME_SECS
            || max_clock_skew_secs > MAX_CLOCK_SKEW_SECS
        {
            return Err(SignedDiscoveryError::InvalidVerifierPolicy);
        }
        Ok(Self {
            expected_cluster_id,
            max_lifetime_secs,
            max_clock_skew_secs,
        })
    }

    pub fn verify(
        &self,
        artifact: SignedDiscoveryArtifact,
        trust_store: &DiscoveryTrustStore,
        now_unix_secs: u64,
    ) -> Result<VerifiedSignedAdvertisement, SignedDiscoveryError> {
        artifact.advertisement.validate_payload()?;
        let verifying_key = trust_store
            .keys
            .get(&artifact.key_id)
            .ok_or(SignedDiscoveryError::UnknownKey)?;
        let signature = Signature::from_bytes(&artifact.signature);
        verifying_key
            .verify(&artifact.canonical_message()?, &signature)
            .map_err(|_| SignedDiscoveryError::InvalidSignature)?;
        if artifact.advertisement.cluster_id != self.expected_cluster_id {
            return Err(SignedDiscoveryError::UnexpectedCluster);
        }
        validate_window(artifact.issued_at_unix_secs, artifact.expires_at_unix_secs)?;
        if artifact.expires_at_unix_secs - artifact.issued_at_unix_secs > self.max_lifetime_secs {
            return Err(SignedDiscoveryError::LifetimeExceeded);
        }
        if now_unix_secs.saturating_add(self.max_clock_skew_secs) < artifact.issued_at_unix_secs {
            return Err(SignedDiscoveryError::NotYetValid);
        }
        if now_unix_secs
            > artifact
                .expires_at_unix_secs
                .saturating_add(self.max_clock_skew_secs)
        {
            return Err(SignedDiscoveryError::Expired);
        }
        Ok(VerifiedSignedAdvertisement {
            advertisement: AuthenticatedAdvertisement {
                inner: artifact.advertisement,
            },
            key_id: artifact.key_id,
            issued_at_unix_secs: artifact.issued_at_unix_secs,
        })
    }
}

/// Signed offline document. Decode alone does not authenticate the payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedDiscoveryArtifact {
    key_id: DiscoveryKeyId,
    issued_at_unix_secs: u64,
    expires_at_unix_secs: u64,
    advertisement: DiscoveryAdvertisement,
    signature: [u8; SIGNATURE_BYTES],
}

impl SignedDiscoveryArtifact {
    pub fn key_id(&self) -> &DiscoveryKeyId {
        &self.key_id
    }

    pub fn issued_at_unix_secs(&self) -> u64 {
        self.issued_at_unix_secs
    }

    pub fn expires_at_unix_secs(&self) -> u64 {
        self.expires_at_unix_secs
    }

    pub fn canonical_message(&self) -> Result<Vec<u8>, SignedDiscoveryError> {
        canonical_message(
            &self.key_id,
            self.issued_at_unix_secs,
            self.expires_at_unix_secs,
            &self.advertisement,
        )
    }

    pub fn signature_bytes(&self) -> [u8; SIGNATURE_BYTES] {
        self.signature
    }

    pub fn encode(&self) -> Result<Vec<u8>, SignedDiscoveryError> {
        let mut encoded = self.canonical_message()?;
        put_u16(&mut encoded, SIGNATURE_BYTES as u16);
        encoded.extend_from_slice(&self.signature);
        if encoded.len() > MAX_ARTIFACT_BYTES {
            return Err(SignedDiscoveryError::ArtifactTooLarge);
        }
        Ok(encoded)
    }

    pub fn decode(encoded: &[u8]) -> Result<Self, SignedDiscoveryError> {
        if encoded.len() > MAX_ARTIFACT_BYTES {
            return Err(SignedDiscoveryError::ArtifactTooLarge);
        }
        let mut reader = Reader::new(encoded);
        reader.expect(DOMAIN)?;
        if reader.u16()? != FORMAT_VERSION {
            return Err(SignedDiscoveryError::UnknownFormat);
        }
        if reader.u16()? != ALGORITHM_ED25519 {
            return Err(SignedDiscoveryError::UnknownAlgorithm);
        }
        let key_id = DiscoveryKeyId::parse(reader.string(MAX_KEY_ID_BYTES)?)?;
        let issued_at_unix_secs = reader.u64()?;
        let expires_at_unix_secs = reader.u64()?;
        let cluster_id = reader.string(MAX_CLUSTER_ID_BYTES)?;
        let epoch = reader.u64()?;
        let generation = reader.u16()?;
        let node_count = usize::from(reader.u16()?);
        if node_count == 0 || node_count > MAX_DISCOVERY_NODES {
            return Err(SignedDiscoveryError::InvalidPayload(
                TransportPolicyError::NoEnabledTransport,
            ));
        }
        let mut nodes = Vec::with_capacity(node_count);
        for _ in 0..node_count {
            let node_id = reader.string(MAX_NODE_ID_BYTES)?;
            let node_epoch = reader.u64()?;
            let endpoint_count = usize::from(reader.u8()?);
            if endpoint_count == 0 || endpoint_count > MAX_NODE_ENDPOINTS {
                return Err(SignedDiscoveryError::InvalidPayload(
                    TransportPolicyError::InvalidNodeEndpointCount,
                ));
            }
            let mut endpoints = Vec::with_capacity(endpoint_count);
            for _ in 0..endpoint_count {
                let candidate = decode_candidate(reader.u8()?)?;
                let maturity = decode_maturity(reader.u8()?)?;
                let flags = reader.u8()?;
                if flags & !0b11 != 0 {
                    return Err(SignedDiscoveryError::MalformedArtifact);
                }
                let uri = reader.string(512)?;
                let server_name = reader.string(253)?;
                endpoints.push(TransportEndpoint {
                    candidate,
                    authority: TransportAuthority::parse(
                        candidate,
                        &uri,
                        &server_name,
                        EndpointOrigin::AuthenticatedDiscovery,
                    )?,
                    maturity,
                    require_mtls: flags & 0b01 != 0,
                    ready: flags & 0b10 != 0,
                });
            }
            nodes.push(
                ServerTransportPolicy::validate(generation, endpoints)?
                    .advertise_node(node_id, node_epoch)?,
            );
        }
        let advertisement = DiscoveryAdvertisement::assemble(cluster_id, epoch, generation, nodes)?;
        let signed_message_end = reader.position();
        if usize::from(reader.u16()?) != SIGNATURE_BYTES {
            return Err(SignedDiscoveryError::MalformedArtifact);
        }
        let signature: [u8; SIGNATURE_BYTES] = reader
            .take(SIGNATURE_BYTES)?
            .try_into()
            .map_err(|_| SignedDiscoveryError::MalformedArtifact)?;
        if !reader.is_empty() {
            return Err(SignedDiscoveryError::MalformedArtifact);
        }
        let artifact = Self {
            key_id,
            issued_at_unix_secs,
            expires_at_unix_secs,
            advertisement,
            signature,
        };
        if artifact.canonical_message()?.as_slice() != &encoded[..signed_message_end] {
            return Err(SignedDiscoveryError::NonCanonicalArtifact);
        }
        Ok(artifact)
    }
}

/// Opaque result of signature, cluster, time, and payload verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedSignedAdvertisement {
    advertisement: AuthenticatedAdvertisement,
    key_id: DiscoveryKeyId,
    issued_at_unix_secs: u64,
}

impl VerifiedSignedAdvertisement {
    pub fn key_id(&self) -> &DiscoveryKeyId {
        &self.key_id
    }
}

/// Replay gate for signed offline discovery, composed with the H07 epoch gate.
#[derive(Debug, Default)]
pub struct OfflineDiscoveryState {
    discovery: DiscoveryState,
    highest_issued_at_unix_secs: u64,
    pending_replacement: Option<String>,
}

impl OfflineDiscoveryState {
    pub fn accept(
        &mut self,
        verified: VerifiedSignedAdvertisement,
    ) -> Result<DiscoveryUpdate, SignedDiscoveryError> {
        if verified.issued_at_unix_secs <= self.highest_issued_at_unix_secs {
            return Err(SignedDiscoveryError::Replay);
        }
        let update = if let Some(replacement) = &self.pending_replacement {
            if verified.advertisement.cluster_id() != replacement {
                return Err(SignedDiscoveryError::UnexpectedRecoveryCluster);
            }
            let mut replacement_state = DiscoveryState::default();
            let update = replacement_state.accept(verified.advertisement)?;
            self.discovery = replacement_state;
            self.pending_replacement = None;
            update
        } else {
            self.discovery.accept(verified.advertisement)?
        };
        self.highest_issued_at_unix_secs = verified.issued_at_unix_secs;
        Ok(update)
    }

    pub fn accepted(&self) -> Option<&AuthenticatedAdvertisement> {
        self.discovery.accepted()
    }

    /// Begin an atomic, fail-closed cluster replacement. The accepted cluster
    /// remains active until a valid signed document for `replacement_cluster`
    /// passes every gate.
    pub fn begin_cluster_replacement(
        &mut self,
        current_cluster: &str,
        replacement_cluster: impl Into<String>,
    ) -> Result<(), SignedDiscoveryError> {
        let accepted = self
            .discovery
            .accepted()
            .ok_or(SignedDiscoveryError::NoAcceptedCluster)?;
        if accepted.cluster_id() != current_cluster {
            return Err(SignedDiscoveryError::RecoveryCurrentClusterMismatch);
        }
        let replacement_cluster = replacement_cluster.into();
        if !valid_cluster_id(&replacement_cluster) || replacement_cluster == current_cluster {
            return Err(SignedDiscoveryError::InvalidRecoveryTarget);
        }
        self.pending_replacement = Some(replacement_cluster);
        Ok(())
    }

    pub fn cancel_cluster_replacement(&mut self) {
        self.pending_replacement = None;
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SignedDiscoveryError {
    #[error("discovery signing key id is invalid")]
    InvalidKeyId,
    #[error("discovery public key is invalid")]
    InvalidPublicKey,
    #[error("discovery trust store contains a duplicate key id")]
    DuplicateKeyId,
    #[error("discovery trust store exceeds its key bound")]
    TrustStoreLimit,
    #[error("signed discovery verifier policy is invalid")]
    InvalidVerifierPolicy,
    #[error("signed discovery validity window is invalid")]
    InvalidValidityWindow,
    #[error("signed discovery lifetime exceeds policy")]
    LifetimeExceeded,
    #[error("signed discovery is not yet valid")]
    NotYetValid,
    #[error("signed discovery is expired")]
    Expired,
    #[error("signed discovery key id is not trusted")]
    UnknownKey,
    #[error("signed discovery signature is invalid")]
    InvalidSignature,
    #[error("signed discovery belongs to an unexpected cluster")]
    UnexpectedCluster,
    #[error("signed discovery artifact exceeds its byte bound")]
    ArtifactTooLarge,
    #[error("signed discovery artifact is truncated or malformed")]
    MalformedArtifact,
    #[error("signed discovery format version is unsupported")]
    UnknownFormat,
    #[error("signed discovery algorithm is unsupported")]
    UnknownAlgorithm,
    #[error("signed discovery bytes are not canonical")]
    NonCanonicalArtifact,
    #[error("signed discovery artifact was replayed")]
    Replay,
    #[error("cluster replacement has no currently accepted cluster")]
    NoAcceptedCluster,
    #[error("cluster replacement current-cluster assertion failed")]
    RecoveryCurrentClusterMismatch,
    #[error("cluster replacement target is invalid")]
    InvalidRecoveryTarget,
    #[error("signed recovery document is for the wrong replacement cluster")]
    UnexpectedRecoveryCluster,
    #[error("signed discovery payload is invalid: {0}")]
    InvalidPayload(#[from] TransportPolicyError),
    #[error("signed discovery endpoint is invalid: {0}")]
    InvalidEndpoint(#[from] EndpointParseError),
}

fn validate_window(issued: u64, expires: u64) -> Result<(), SignedDiscoveryError> {
    if issued == 0 || expires <= issued {
        return Err(SignedDiscoveryError::InvalidValidityWindow);
    }
    Ok(())
}

fn valid_cluster_id(cluster_id: &str) -> bool {
    !cluster_id.trim().is_empty() && cluster_id.len() <= MAX_CLUSTER_ID_BYTES
}

fn normalize_advertisement(advertisement: &mut DiscoveryAdvertisement) {
    advertisement
        .nodes
        .sort_by(|left, right| left.node_id.cmp(&right.node_id));
    for node in &mut advertisement.nodes {
        node.endpoints
            .sort_by_key(|endpoint| candidate_code(endpoint.candidate));
    }
}

fn canonical_message(
    key_id: &DiscoveryKeyId,
    issued_at_unix_secs: u64,
    expires_at_unix_secs: u64,
    advertisement: &DiscoveryAdvertisement,
) -> Result<Vec<u8>, SignedDiscoveryError> {
    advertisement.validate_payload()?;
    let mut encoded = Vec::with_capacity(1024);
    encoded.extend_from_slice(DOMAIN);
    put_u16(&mut encoded, FORMAT_VERSION);
    put_u16(&mut encoded, ALGORITHM_ED25519);
    put_string(&mut encoded, key_id.as_str())?;
    put_u64(&mut encoded, issued_at_unix_secs);
    put_u64(&mut encoded, expires_at_unix_secs);
    put_string(&mut encoded, &advertisement.cluster_id)?;
    put_u64(&mut encoded, advertisement.epoch);
    put_u16(&mut encoded, advertisement.generation);

    let mut nodes = advertisement.nodes.iter().collect::<Vec<_>>();
    nodes.sort_by(|left, right| left.node_id.cmp(&right.node_id));
    put_u16(
        &mut encoded,
        u16::try_from(nodes.len()).map_err(|_| SignedDiscoveryError::ArtifactTooLarge)?,
    );
    for node in nodes {
        put_string(&mut encoded, node.node_id.as_str())?;
        put_u64(&mut encoded, node.epoch);
        let mut endpoints = node.endpoints.iter().collect::<Vec<_>>();
        endpoints.sort_by_key(|endpoint| candidate_code(endpoint.candidate));
        encoded.push(
            u8::try_from(endpoints.len()).map_err(|_| SignedDiscoveryError::ArtifactTooLarge)?,
        );
        for endpoint in endpoints {
            encoded.push(candidate_code(endpoint.candidate));
            encoded.push(maturity_code(endpoint.maturity));
            encoded.push(u8::from(endpoint.require_mtls) | (u8::from(endpoint.ready) << 1));
            put_string(&mut encoded, &endpoint.authority.canonical_uri())?;
            put_string(&mut encoded, endpoint.authority.server_name())?;
        }
    }
    if encoded.len() + 2 + SIGNATURE_BYTES > MAX_ARTIFACT_BYTES {
        return Err(SignedDiscoveryError::ArtifactTooLarge);
    }
    Ok(encoded)
}

fn candidate_code(candidate: TransportCandidate) -> u8 {
    match candidate {
        TransportCandidate::GrpcBidirectional => 1,
        TransportCandidate::Http2Bidirectional => 2,
        TransportCandidate::DedicatedTcpTls => 3,
    }
}

fn decode_candidate(value: u8) -> Result<TransportCandidate, SignedDiscoveryError> {
    match value {
        1 => Ok(TransportCandidate::GrpcBidirectional),
        2 => Ok(TransportCandidate::Http2Bidirectional),
        3 => Ok(TransportCandidate::DedicatedTcpTls),
        _ => Err(SignedDiscoveryError::MalformedArtifact),
    }
}

fn maturity_code(maturity: AdapterMaturity) -> u8 {
    match maturity {
        AdapterMaturity::Experimental => 1,
        AdapterMaturity::Preview => 2,
        AdapterMaturity::Stable => 3,
    }
}

fn decode_maturity(value: u8) -> Result<AdapterMaturity, SignedDiscoveryError> {
    match value {
        1 => Ok(AdapterMaturity::Experimental),
        2 => Ok(AdapterMaturity::Preview),
        3 => Ok(AdapterMaturity::Stable),
        _ => Err(SignedDiscoveryError::MalformedArtifact),
    }
}

fn put_u16(output: &mut Vec<u8>, value: u16) {
    output.extend_from_slice(&value.to_be_bytes());
}

fn put_u64(output: &mut Vec<u8>, value: u64) {
    output.extend_from_slice(&value.to_be_bytes());
}

fn put_string(output: &mut Vec<u8>, value: &str) -> Result<(), SignedDiscoveryError> {
    let length = u16::try_from(value.len()).map_err(|_| SignedDiscoveryError::ArtifactTooLarge)?;
    put_u16(output, length);
    output.extend_from_slice(value.as_bytes());
    Ok(())
}

struct Reader<'a> {
    input: &'a [u8],
    position: usize,
}

impl<'a> Reader<'a> {
    fn new(input: &'a [u8]) -> Self {
        Self { input, position: 0 }
    }

    fn position(&self) -> usize {
        self.position
    }

    fn is_empty(&self) -> bool {
        self.position == self.input.len()
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], SignedDiscoveryError> {
        let end = self
            .position
            .checked_add(length)
            .ok_or(SignedDiscoveryError::MalformedArtifact)?;
        let bytes = self
            .input
            .get(self.position..end)
            .ok_or(SignedDiscoveryError::MalformedArtifact)?;
        self.position = end;
        Ok(bytes)
    }

    fn expect(&mut self, expected: &[u8]) -> Result<(), SignedDiscoveryError> {
        if self.take(expected.len())? != expected {
            return Err(SignedDiscoveryError::UnknownFormat);
        }
        Ok(())
    }

    fn u8(&mut self) -> Result<u8, SignedDiscoveryError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, SignedDiscoveryError> {
        Ok(u16::from_be_bytes(
            self.take(2)?
                .try_into()
                .map_err(|_| SignedDiscoveryError::MalformedArtifact)?,
        ))
    }

    fn u64(&mut self) -> Result<u64, SignedDiscoveryError> {
        Ok(u64::from_be_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| SignedDiscoveryError::MalformedArtifact)?,
        ))
    }

    fn string(&mut self, max_length: usize) -> Result<String, SignedDiscoveryError> {
        let length = usize::from(self.u16()?);
        if length == 0 || length > max_length {
            return Err(SignedDiscoveryError::MalformedArtifact);
        }
        let value = std::str::from_utf8(self.take(length)?)
            .map_err(|_| SignedDiscoveryError::MalformedArtifact)?;
        if !value.is_ascii() {
            return Err(SignedDiscoveryError::MalformedArtifact);
        }
        Ok(value.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ISSUED: u64 = 1_800_000_000;
    const EXPIRES: u64 = ISSUED + 3_600;

    fn endpoint(candidate: TransportCandidate, host: &str, port: u16) -> TransportEndpoint {
        let scheme = match candidate {
            TransportCandidate::GrpcBidirectional => "hc2+grpc",
            TransportCandidate::Http2Bidirectional => "hc2+h2",
            TransportCandidate::DedicatedTcpTls => "hc2+tls",
        };
        TransportEndpoint::parse(
            candidate,
            &format!("{scheme}://{host}:{port}"),
            host,
            candidate.proven_maturity(),
            EndpointOrigin::AuthenticatedDiscovery,
        )
        .unwrap()
    }

    fn node(node_id: &str, epoch: u64, endpoints: Vec<TransportEndpoint>) -> NodeAdvertisement {
        ServerTransportPolicy::validate(HC2_GENERATION, endpoints)
            .unwrap()
            .advertise_node(node_id, epoch)
            .unwrap()
    }

    fn advertisement(
        cluster_id: &str,
        epoch: u64,
        node_epoch: u64,
        port: u16,
    ) -> DiscoveryAdvertisement {
        DiscoveryAdvertisement::assemble(
            cluster_id,
            epoch,
            HC2_GENERATION,
            vec![node(
                "node-a",
                node_epoch,
                vec![endpoint(
                    TransportCandidate::GrpcBidirectional,
                    "node-a.example.com",
                    port,
                )],
            )],
        )
        .unwrap()
    }

    fn signer(key_id: &str, seed_byte: u8) -> DiscoverySigningKey {
        DiscoverySigningKey::from_seed(key_id, [seed_byte; 32]).unwrap()
    }

    fn verifier(cluster_id: &str) -> SignedDiscoveryVerifier {
        SignedDiscoveryVerifier::new(cluster_id, 3_600, 0).unwrap()
    }

    fn trust(signer: &DiscoverySigningKey) -> DiscoveryTrustStore {
        DiscoveryTrustStore::new([signer.trusted_key()]).unwrap()
    }

    fn to_hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    #[test]
    fn artifact_round_trip_signature_and_tamper_matrix_are_fail_closed() {
        let signer = signer("root-2026-a", 7);
        let trust = trust(&signer);
        let verifier = verifier("cluster-a");
        let artifact = signer
            .sign(advertisement("cluster-a", 42, 7, 7443), ISSUED, EXPIRES)
            .unwrap();
        let encoded = artifact.encode().unwrap();
        let decoded = SignedDiscoveryArtifact::decode(&encoded).unwrap();
        assert_eq!(decoded, artifact);
        let verified = verifier.verify(decoded, &trust, ISSUED + 1).unwrap();
        assert_eq!(verified.key_id().as_str(), "root-2026-a");

        let mut payload_tamper = encoded.clone();
        let port = payload_tamper
            .windows(4)
            .position(|window| window == b"7443")
            .expect("canonical endpoint port");
        payload_tamper[port + 3] = b'4';
        assert_eq!(
            verifier.verify(
                SignedDiscoveryArtifact::decode(&payload_tamper).unwrap(),
                &trust,
                ISSUED + 1,
            ),
            Err(SignedDiscoveryError::InvalidSignature)
        );

        let mut signature_tamper = encoded;
        *signature_tamper.last_mut().unwrap() ^= 1;
        assert_eq!(
            verifier.verify(
                SignedDiscoveryArtifact::decode(&signature_tamper).unwrap(),
                &trust,
                ISSUED + 1,
            ),
            Err(SignedDiscoveryError::InvalidSignature)
        );
    }

    #[test]
    fn key_time_cluster_and_artifact_bounds_are_enforced() {
        let signer = signer("root-2026-a", 7);
        let artifact = signer
            .sign(advertisement("cluster-a", 42, 7, 7443), ISSUED, EXPIRES)
            .unwrap();
        let empty_trust = DiscoveryTrustStore::default();
        assert_eq!(
            verifier("cluster-a").verify(artifact.clone(), &empty_trust, ISSUED),
            Err(SignedDiscoveryError::UnknownKey)
        );
        let trust = trust(&signer);
        assert_eq!(
            verifier("cluster-b").verify(artifact.clone(), &trust, ISSUED),
            Err(SignedDiscoveryError::UnexpectedCluster)
        );
        assert_eq!(
            verifier("cluster-a").verify(artifact.clone(), &trust, ISSUED - 1),
            Err(SignedDiscoveryError::NotYetValid)
        );
        assert_eq!(
            verifier("cluster-a").verify(artifact.clone(), &trust, EXPIRES + 1),
            Err(SignedDiscoveryError::Expired)
        );
        let short_lifetime = SignedDiscoveryVerifier::new("cluster-a", 60, 0).unwrap();
        assert_eq!(
            short_lifetime.verify(artifact.clone(), &trust, ISSUED),
            Err(SignedDiscoveryError::LifetimeExceeded)
        );

        let encoded = artifact.encode().unwrap();
        assert_eq!(
            SignedDiscoveryArtifact::decode(&encoded[..encoded.len() - 1]),
            Err(SignedDiscoveryError::MalformedArtifact)
        );
        let mut trailing = encoded;
        trailing.push(0);
        assert_eq!(
            SignedDiscoveryArtifact::decode(&trailing),
            Err(SignedDiscoveryError::MalformedArtifact)
        );
        assert_eq!(
            SignedDiscoveryArtifact::decode(&vec![0; MAX_ARTIFACT_BYTES + 1]),
            Err(SignedDiscoveryError::ArtifactTooLarge)
        );
        assert_eq!(
            SignedDiscoveryVerifier::new(" ", 60, 0),
            Err(SignedDiscoveryError::InvalidVerifierPolicy)
        );
    }

    #[test]
    fn canonicalization_is_independent_of_node_and_endpoint_input_order() {
        let signer = signer("root-2026-a", 7);
        let node_a = node(
            "node-a",
            4,
            vec![
                endpoint(
                    TransportCandidate::DedicatedTcpTls,
                    "node-a.example.com",
                    7445,
                ),
                endpoint(
                    TransportCandidate::GrpcBidirectional,
                    "node-a.example.com",
                    7443,
                ),
            ],
        );
        let node_b = node(
            "node-b",
            9,
            vec![endpoint(
                TransportCandidate::Http2Bidirectional,
                "node-b.example.com",
                7444,
            )],
        );
        let left = DiscoveryAdvertisement::assemble(
            "cluster-a",
            42,
            HC2_GENERATION,
            vec![node_b.clone(), node_a.clone()],
        )
        .unwrap();
        let right =
            DiscoveryAdvertisement::assemble("cluster-a", 42, HC2_GENERATION, vec![node_a, node_b])
                .unwrap();
        let left = signer.sign(left, ISSUED, EXPIRES).unwrap();
        let right = signer.sign(right, ISSUED, EXPIRES).unwrap();
        assert_eq!(
            left.canonical_message().unwrap(),
            right.canonical_message().unwrap()
        );
        assert_eq!(left.signature_bytes(), right.signature_bytes());
        assert_eq!(left.encode().unwrap(), right.encode().unwrap());
        assert_eq!(left, right);
        assert_eq!(
            SignedDiscoveryArtifact::decode(&left.encode().unwrap()),
            Ok(left)
        );
    }

    #[test]
    fn decoder_rejects_a_validly_shaped_but_noncanonical_node_order() {
        let signer = signer("root-2026-a", 7);
        let artifact = signer
            .sign(
                DiscoveryAdvertisement::assemble(
                    "cluster-a",
                    42,
                    HC2_GENERATION,
                    vec![
                        node(
                            "node-a",
                            7,
                            vec![endpoint(
                                TransportCandidate::GrpcBidirectional,
                                "node-a.example.com",
                                7443,
                            )],
                        ),
                        node(
                            "node-b",
                            8,
                            vec![endpoint(
                                TransportCandidate::GrpcBidirectional,
                                "node-b.example.com",
                                7443,
                            )],
                        ),
                    ],
                )
                .unwrap(),
                ISSUED,
                EXPIRES,
            )
            .unwrap();
        let mut encoded = artifact.encode().unwrap();
        let node_a = encoded
            .windows(6)
            .position(|window| window == b"node-a")
            .unwrap()
            - 2;
        let node_b = encoded
            .windows(6)
            .position(|window| window == b"node-b")
            .unwrap()
            - 2;
        let record_length = node_b - node_a;
        assert_eq!(
            &encoded[node_b + record_length..node_b + record_length + 2],
            &[0, 64]
        );
        let first = encoded[node_a..node_b].to_vec();
        let second = encoded[node_b..node_b + record_length].to_vec();
        encoded[node_a..node_b].copy_from_slice(&second);
        encoded[node_b..node_b + record_length].copy_from_slice(&first);
        assert_eq!(
            SignedDiscoveryArtifact::decode(&encoded),
            Err(SignedDiscoveryError::NonCanonicalArtifact)
        );
    }

    #[test]
    fn trust_store_refuses_duplicate_and_ninth_keys_without_mutation() {
        let first = signer("root-0", 1);
        let mut trust = DiscoveryTrustStore::new([first.trusted_key()]).unwrap();
        assert_eq!(
            trust.add_key(first.trusted_key()),
            Err(SignedDiscoveryError::DuplicateKeyId)
        );
        for index in 1_u8..8 {
            trust
                .add_key(signer(&format!("root-{index}"), index + 1).trusted_key())
                .unwrap();
        }
        let retained = DiscoveryKeyId::parse("root-7").unwrap();
        assert!(trust.contains(&retained));
        assert_eq!(
            trust.add_key(signer("root-8", 9).trusted_key()),
            Err(SignedDiscoveryError::TrustStoreLimit)
        );
        assert!(trust.contains(&retained));
        assert!(!trust.contains(&DiscoveryKeyId::parse("root-8").unwrap()));
    }

    #[test]
    fn replay_equivocation_and_epoch_rollback_do_not_mutate_accepted_state() {
        let signer = signer("root-2026-a", 7);
        let trust = trust(&signer);
        let verifier = verifier("cluster-a");
        let first = signer
            .sign(advertisement("cluster-a", 42, 7, 7443), ISSUED, EXPIRES)
            .unwrap();
        let mut state = OfflineDiscoveryState::default();
        assert_eq!(
            state.accept(verifier.verify(first.clone(), &trust, ISSUED).unwrap()),
            Ok(DiscoveryUpdate::Advanced)
        );
        assert_eq!(
            state.accept(verifier.verify(first, &trust, ISSUED).unwrap()),
            Err(SignedDiscoveryError::Replay)
        );

        let equivocation = signer
            .sign(advertisement("cluster-a", 42, 8, 7444), ISSUED + 1, EXPIRES)
            .unwrap();
        assert_eq!(
            state.accept(verifier.verify(equivocation, &trust, ISSUED + 1).unwrap()),
            Err(SignedDiscoveryError::InvalidPayload(
                TransportPolicyError::DiscoveryEquivocation
            ))
        );
        assert_eq!(state.accepted().unwrap().epoch(), 42);

        let rollback = signer
            .sign(advertisement("cluster-a", 41, 8, 7444), ISSUED + 2, EXPIRES)
            .unwrap();
        assert_eq!(
            state.accept(verifier.verify(rollback, &trust, ISSUED + 2).unwrap()),
            Err(SignedDiscoveryError::InvalidPayload(
                TransportPolicyError::DiscoveryRollback
            ))
        );
        assert_eq!(state.accepted().unwrap().epoch(), 42);

        let refresh = signer
            .sign(advertisement("cluster-a", 42, 7, 7443), ISSUED + 3, EXPIRES)
            .unwrap();
        assert_eq!(
            state.accept(verifier.verify(refresh, &trust, ISSUED + 3).unwrap()),
            Ok(DiscoveryUpdate::Unchanged)
        );
    }

    #[test]
    fn trust_rotation_overlap_accepts_new_key_then_removed_key_fails() {
        let old = signer("root-2026-a", 7);
        let current = signer("root-2026-b", 8);
        let mut trust = trust(&old);
        trust.add_key(current.trusted_key()).unwrap();
        let verifier = verifier("cluster-a");

        let old_artifact = old
            .sign(advertisement("cluster-a", 42, 7, 7443), ISSUED, EXPIRES)
            .unwrap();
        let current_artifact = current
            .sign(advertisement("cluster-a", 43, 8, 7444), ISSUED + 1, EXPIRES)
            .unwrap();
        assert!(verifier
            .verify(old_artifact.clone(), &trust, ISSUED)
            .is_ok());
        assert!(verifier
            .verify(current_artifact, &trust, ISSUED + 1)
            .is_ok());
        assert!(trust.remove_key(old.trusted_key().key_id()));
        assert_eq!(
            verifier.verify(old_artifact, &trust, ISSUED),
            Err(SignedDiscoveryError::UnknownKey)
        );
    }

    #[test]
    fn cluster_replacement_is_explicit_atomic_and_replay_safe() {
        let signer = signer("root-2026-a", 7);
        let trust = trust(&signer);
        let first = signer
            .sign(advertisement("cluster-a", 42, 7, 7443), ISSUED, EXPIRES)
            .unwrap();
        let mut state = OfflineDiscoveryState::default();
        state
            .accept(verifier("cluster-a").verify(first, &trust, ISSUED).unwrap())
            .unwrap();

        assert_eq!(
            state.begin_cluster_replacement("wrong", "cluster-b"),
            Err(SignedDiscoveryError::RecoveryCurrentClusterMismatch)
        );
        assert_eq!(
            state.begin_cluster_replacement("cluster-a", " "),
            Err(SignedDiscoveryError::InvalidRecoveryTarget)
        );
        state
            .begin_cluster_replacement("cluster-a", "cluster-b")
            .unwrap();
        let wrong = signer
            .sign(advertisement("cluster-c", 1, 1, 8443), ISSUED + 1, EXPIRES)
            .unwrap();
        assert_eq!(
            state.accept(
                verifier("cluster-c")
                    .verify(wrong, &trust, ISSUED + 1)
                    .unwrap()
            ),
            Err(SignedDiscoveryError::UnexpectedRecoveryCluster)
        );
        assert_eq!(state.accepted().unwrap().cluster_id(), "cluster-a");

        let replacement = signer
            .sign(advertisement("cluster-b", 1, 1, 8443), ISSUED + 2, EXPIRES)
            .unwrap();
        assert_eq!(
            state.accept(
                verifier("cluster-b")
                    .verify(replacement, &trust, ISSUED + 2)
                    .unwrap()
            ),
            Ok(DiscoveryUpdate::Advanced)
        );
        assert_eq!(state.accepted().unwrap().cluster_id(), "cluster-b");
        assert_eq!(state.accepted().unwrap().epoch(), 1);
    }

    #[test]
    fn cross_language_ed25519_canonical_vector_is_stable() {
        let signer = signer("root-2026-a", 7);
        let artifact = signer
            .sign(advertisement("cluster-a", 42, 7, 7443), ISSUED, EXPIRES)
            .unwrap();
        assert_eq!(
            to_hex(&artifact.canonical_message().unwrap()),
            "485944524143414348452d4843322d444953434f564552590000010001000b726f6f742d323032362d61000000006b49d200000000006b49e0100009636c75737465722d61000000000000002a0005000100066e6f64652d6100000000000000070101020300226863322b677270633a2f2f6e6f64652d612e6578616d706c652e636f6d3a3734343300126e6f64652d612e6578616d706c652e636f6d"
        );
        assert_eq!(
            to_hex(&signer.trusted_key().public_key_bytes()),
            "ea4a6c63e29c520abef5507b132ec5f9954776aebebe7b92421eea691446d22c"
        );
        assert_eq!(
            to_hex(&artifact.signature_bytes()),
            "2bcce116ff87d015d8f36564a85044df33adecca281c9d6a38355fc28d5edac9ca8b2a9314c2d3ff1244dff1c30b648af279959bfa72ff4705a9dd3276f5780c"
        );
    }
}
