//! Private bounded RS256 profile. Configured keys are not independently enrolled GitHub keys.
use super::OidcError;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{de::MapAccess, Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;

pub(super) const SCHEMA: &str = "diagnostic-oidc-trust-input-074-v1";
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
    pub(super) fn decode(raw: &[u8]) -> Result<Self, OidcError> {
        if raw.is_empty() || raw.len() > 2048 {
            return Err(OidcError::Invalid);
        }
        let p: Self = serde_json::from_slice(raw).map_err(|_| OidcError::Invalid)?;
        let mut canonical = crate::canonical_json(&p).map_err(|_| OidcError::Invalid)?;
        canonical.push(b'\n');
        let n = segment(&p.key.n, 684)?;
        let e = segment(&p.key.e, 4)?;
        if canonical != raw
            || p.schema_version != SCHEMA
            || p.repository_id != crate::diagnostic_builder::REPOSITORY_ID
            || p.issuer != ISSUER
            || p.workflow_ref != WORKFLOW
            || p.environment != ENVIRONMENT
            || p.workflow_sha.len() != 40
            || !p
                .workflow_sha
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || !bounded_text(&p.subject, 256)
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
        Ok(p)
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
    profile: &Profile,
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
    if h.alg != "RS256" || h.typ != "JWT" || h.kid != profile.key.kid {
        return Err(OidcError::Invalid);
    }
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
