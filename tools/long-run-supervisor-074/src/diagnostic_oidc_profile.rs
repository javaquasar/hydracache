//! Private bounded RS256 profile. Configured keys are not independently enrolled GitHub keys.
use super::OidcError;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{de::MapAccess, Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;

pub(super) const SCHEMA: &str = "diagnostic-oidc-trust-input-074-v1";
const ROSTER_SCHEMA: &str = "diagnostic-oidc-trust-input-074-v2";
pub(super) const MAX_ROSTER_BYTES: usize = 8192;
pub(super) const MAX_ROSTER_KEYS: usize = 8;
pub(super) const ISSUER: &str = "https://token.actions.githubusercontent.com";
pub(super) const WORKFLOW: &str = "javaquasar/hydracache/.github/workflows/diagnostic-controller-074.yml@refs/heads/feat/0.74-resp-native-throughput";
pub(super) const ENVIRONMENT: &str = "performance-diagnostic-controller-074";
pub(super) const MAX_TOKEN: usize = 16384;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Key {
    pub kid: String,
    pub kty: String,
    pub alg: String,
    #[serde(rename = "use")]
    pub usage: String,
    pub n: String,
    pub e: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Profile {
    pub schema_version: String,
    pub repository_id: u64,
    pub issuer: String,
    pub workflow_ref: String,
    pub workflow_sha: String,
    pub environment: String,
    pub subject: String,
    pub snapshot_not_before: u64,
    pub snapshot_expires_at: u64,
    pub key: Key,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RosterProfile {
    schema_version: String,
    repository_id: u64,
    issuer: String,
    workflow_ref: String,
    workflow_sha: String,
    environment: String,
    subject: String,
    snapshot_not_before: u64,
    snapshot_expires_at: u64,
    keys: Vec<Key>,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub(super) enum TrustProfile {
    V1(Profile),
    V2(RosterProfile),
}

pub(super) struct SelectedProfile<'a> {
    repository_id: u64,
    issuer: &'a str,
    workflow_ref: &'a str,
    workflow_sha: &'a str,
    environment: &'a str,
    subject: &'a str,
    snapshot_not_before: u64,
    snapshot_expires_at: u64,
    key: &'a Key,
}

pub(super) trait KeyPolicy {
    fn select(&self, kid: &str) -> Result<SelectedProfile<'_>, OidcError>;
}
fn bounded_text(s: &str, max: usize) -> bool {
    !s.is_empty() && s.len() <= max && !s.chars().any(char::is_control)
}
fn segment(s: &str, max: usize) -> Result<Vec<u8>, OidcError> {
    if s.is_empty() || s.len() > max {
        return Err(OidcError::Invalid);
    }
    let raw = URL_SAFE_NO_PAD.decode(s).map_err(|_| OidcError::Invalid)?;
    if URL_SAFE_NO_PAD.encode(&raw) != s {
        return Err(OidcError::Invalid);
    }
    Ok(raw)
}
impl Profile {
    fn selected(&self) -> SelectedProfile<'_> {
        SelectedProfile {
            repository_id: self.repository_id,
            issuer: &self.issuer,
            workflow_ref: &self.workflow_ref,
            workflow_sha: &self.workflow_sha,
            environment: &self.environment,
            subject: &self.subject,
            snapshot_not_before: self.snapshot_not_before,
            snapshot_expires_at: self.snapshot_expires_at,
            key: &self.key,
        }
    }
    #[cfg(test)]
    pub(super) fn decode(raw: &[u8]) -> Result<Self, OidcError> {
        if raw.is_empty() || raw.len() > 2048 {
            return Err(OidcError::Invalid);
        }
        let p: Self = serde_json::from_slice(raw).map_err(|_| OidcError::Invalid)?;
        let mut canonical = crate::canonical_json(&p).map_err(|_| OidcError::Invalid)?;
        canonical.push(b'\n');
        if canonical != raw || p.schema_version != SCHEMA {
            return Err(OidcError::Invalid);
        }
        p.selected().validate()?;
        Ok(p)
    }
}
impl SelectedProfile<'_> {
    fn validate(&self) -> Result<(), OidcError> {
        let p = self;
        let n = segment(&p.key.n, 684)?;
        let e = segment(&p.key.e, 4)?;
        if p.repository_id != crate::diagnostic_builder::REPOSITORY_ID
            || p.issuer != ISSUER
            || p.workflow_ref != WORKFLOW
            || p.environment != ENVIRONMENT
            || p.workflow_sha.len() != 40
            || !p
                .workflow_sha
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || !bounded_text(p.subject, 256)
            || p.snapshot_not_before == 0
            || p.snapshot_expires_at <= p.snapshot_not_before
            || !bounded_text(&p.key.kid, 128)
            || p.key.kty != "RSA"
            || p.key.alg != "RS256"
            || p.key.usage != "sig"
            || !(256..=512).contains(&n.len())
            || n[0] & 0x80 == 0
            || n.last().is_none_or(|b| b & 1 == 0)
            || e != [1, 0, 1]
        {
            return Err(OidcError::Invalid);
        }
        Ok(())
    }
}
impl KeyPolicy for Profile {
    fn select(&self, kid: &str) -> Result<SelectedProfile<'_>, OidcError> {
        if kid != self.key.kid {
            return Err(OidcError::Invalid);
        }
        Ok(self.selected())
    }
}
impl RosterProfile {
    fn selected<'a>(&'a self, key: &'a Key) -> SelectedProfile<'a> {
        // The selected key always belongs to this retained configured roster.
        SelectedProfile {
            repository_id: self.repository_id,
            issuer: &self.issuer,
            workflow_ref: &self.workflow_ref,
            workflow_sha: &self.workflow_sha,
            environment: &self.environment,
            subject: &self.subject,
            snapshot_not_before: self.snapshot_not_before,
            snapshot_expires_at: self.snapshot_expires_at,
            key,
        }
    }
}
impl TrustProfile {
    pub(super) fn decode(raw: &[u8]) -> Result<Self, OidcError> {
        if raw.is_empty() || raw.len() > MAX_ROSTER_BYTES {
            return Err(OidcError::Invalid);
        }
        let profile: Self = serde_json::from_slice(raw).map_err(|_| OidcError::Invalid)?;
        let mut canonical = crate::canonical_json(&profile).map_err(|_| OidcError::Invalid)?;
        canonical.push(b'\n');
        if canonical != raw {
            return Err(OidcError::Invalid);
        }
        match &profile {
            Self::V1(p) => {
                if raw.len() > 2048 || p.schema_version != SCHEMA {
                    return Err(OidcError::Invalid);
                }
                p.selected().validate()?;
            }
            Self::V2(p) => {
                if p.schema_version != ROSTER_SCHEMA
                    || !(1..=MAX_ROSTER_KEYS).contains(&p.keys.len())
                    || p.keys.windows(2).any(|pair| pair[0].kid >= pair[1].kid)
                {
                    return Err(OidcError::Invalid);
                }
                let mut material = std::collections::BTreeSet::new();
                for key in &p.keys {
                    p.selected(key).validate()?;
                    if !material.insert((&key.n, &key.e)) {
                        return Err(OidcError::Invalid);
                    }
                }
            }
        }
        Ok(profile)
    }
}
impl KeyPolicy for TrustProfile {
    fn select(&self, kid: &str) -> Result<SelectedProfile<'_>, OidcError> {
        match self {
            Self::V1(p) => p.select(kid),
            Self::V2(p) => {
                let index = p
                    .keys
                    .binary_search_by(|key| key.kid.as_str().cmp(kid))
                    .map_err(|_| OidcError::Invalid)?;
                Ok(p.selected(&p.keys[index]))
            }
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    alg: String,
    typ: String,
    kid: String,
}

// Reject duplicate names even in ignored fields and escaped equivalent names.
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum Scalar {
    Text(String),
    Number(u64),
    Boolean(bool),
}
#[derive(Serialize)]
struct Claims(BTreeMap<String, Scalar>);
impl<'de> Deserialize<'de> for Claims {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Claims;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("bounded unique scalar claims")
            }
            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Claims, M::Error> {
                let mut out = BTreeMap::new();
                while let Some(key) = map.next_key::<String>()? {
                    if out.len() >= 64 || !bounded_text(&key, 64) || out.contains_key(&key) {
                        return Err(serde::de::Error::custom("invalid claims"));
                    }
                    let value: Scalar = map.next_value()?;
                    if matches!(&value, Scalar::Text(s) if s.len() > 2048 || s.chars().any(char::is_control))
                    {
                        return Err(serde::de::Error::custom("invalid claims"));
                    }
                    out.insert(key, value);
                }
                Ok(Claims(out))
            }
        }
        deserializer.deserialize_map(Visitor)
    }
}
impl Claims {
    fn text(&self, k: &str) -> Result<&str, OidcError> {
        match self.0.get(k) {
            Some(Scalar::Text(s)) => Ok(s),
            _ => Err(OidcError::Invalid),
        }
    }
    fn number(&self, k: &str) -> Result<u64, OidcError> {
        match self.0.get(k) {
            Some(Scalar::Number(n)) => Ok(*n),
            _ => Err(OidcError::Invalid),
        }
    }
    fn decimal(&self, k: &str) -> Result<u64, OidcError> {
        let s = self.text(k)?;
        let n: u64 = s.parse().map_err(|_| OidcError::Invalid)?;
        if n == 0 || n.to_string() != s {
            return Err(OidcError::Invalid);
        }
        Ok(n)
    }
}
pub(super) fn verify(
    profile: &impl KeyPolicy,
    token: &[u8],
    request: &crate::diagnostic_ipc::DiagnosticRequest,
    request_hash: &str,
    now: u64,
) -> Result<String, OidcError> {
    if token.is_empty() || token.len() > MAX_TOKEN {
        return Err(OidcError::Invalid);
    }
    let token = std::str::from_utf8(token).map_err(|_| OidcError::Invalid)?;
    let parts: Vec<_> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(OidcError::Invalid);
    }
    let h: Header =
        serde_json::from_slice(&segment(parts[0], 1024)?).map_err(|_| OidcError::Invalid)?;
    if h.alg != "RS256" || h.typ != "JWT" {
        return Err(OidcError::Invalid);
    }
    let profile = profile.select(&h.kid)?;
    let payload = segment(parts[1], 12288)?;
    let signature = segment(parts[2], 684)?;
    let n = segment(&profile.key.n, 684)?;
    let e = segment(&profile.key.e, 4)?;
    if signature.len() != n.len() {
        return Err(OidcError::Invalid);
    }
    let input_len = parts[0].len() + 1 + parts[1].len();
    ring::signature::RsaPublicKeyComponents { n: &n, e: &e }
        .verify(
            &ring::signature::RSA_PKCS1_2048_8192_SHA256,
            &token.as_bytes()[..input_len],
            &signature,
        )
        .map_err(|_| OidcError::Signature)?;
    let c: Claims = serde_json::from_slice(&payload).map_err(|_| OidcError::Invalid)?;
    let iat = c.number("iat")?;
    let nbf = c.number("nbf")?;
    let exp = c.number("exp")?;
    let digest =
        crate::sha256_hex(&crate::canonical_json(request).map_err(|_| OidcError::Invalid)?);
    if digest != request_hash
        || !crate::is_hash(request_hash)
        || c.text("iss")? != ISSUER
        || c.text("aud")? != format!("urn:hydracache:diagnostic:074:request:{request_hash}")
        || c.text("sub")? != profile.subject
        || c.text("repository")? != "javaquasar/hydracache"
        || c.decimal("repository_id")? != request.controller.repository_id
        || request.controller.repository_id != profile.repository_id
        || c.decimal("actor_id")? != request.controller.actor_id
        || c.decimal("run_id")? != request.controller.run_id
        || c.decimal("run_attempt")? != u64::from(request.controller.run_attempt)
        || c.text("workflow_ref")? != profile.workflow_ref
        || c.text("workflow_sha")? != profile.workflow_sha
        || c.text("environment")? != profile.environment
        || c.text("event_name")? != "workflow_dispatch"
        || c.text("runner_environment")? != "github-hosted"
        || c.0.contains_key("job_workflow_ref")
        || c.0.contains_key("job_workflow_sha")
        || !bounded_text(c.text("jti")?, 128)
        || nbf == 0
        || nbf > iat
        || iat > now
        || exp <= now
        || exp <= iat
        || exp - iat > 600
        || now < profile.snapshot_not_before
        || now >= profile.snapshot_expires_at
        || now < request.issued_at_unix_seconds
        || now >= request.expires_at_unix_seconds
    {
        return Err(OidcError::Invalid);
    }
    Ok(crate::sha256_hex(token.as_bytes()))
}
