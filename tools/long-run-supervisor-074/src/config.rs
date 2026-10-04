use crate::service::ServicePolicy;
use ed25519_dalek::VerifyingKey;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub const PRODUCTION_SOCKET: &str = "/run/hydracache-perf/supervisor-v1.sock";
pub const PRODUCTION_CAMPAIGN_ROOT: &str = "/var/lib/hydracache-performance/campaigns";
pub const MAX_CONFIG_BYTES: usize = 16_384;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerConfig {
    pub schema_version: u32,
    pub socket_path: PathBuf,
    pub campaign_root: PathBuf,
    pub socket_mode: u32,
    pub expected_repository_id: u64,
    pub allowed_actor_ids: Vec<u64>,
    pub allowed_client_uids: Vec<u32>,
    pub required_client_gid: u32,
    pub verification_key_hex: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("supervisor configuration is empty, oversized, malformed, or has unknown fields")]
    Document,
    #[error("supervisor configuration violates a frozen invariant")]
    Invariant,
}

impl ServerConfig {
    pub fn parse(bytes: &[u8], production_paths: bool) -> Result<Self, ConfigError> {
        if bytes.is_empty() || bytes.len() > MAX_CONFIG_BYTES {
            return Err(ConfigError::Document);
        }
        let config: Self = toml::from_slice(bytes).map_err(|_| ConfigError::Document)?;
        config.validate(production_paths)?;
        Ok(config)
    }

    pub fn policy(&self) -> Result<ServicePolicy, ConfigError> {
        let key = decode_key(&self.verification_key_hex).ok_or(ConfigError::Invariant)?;
        Ok(ServicePolicy {
            expected_repository_id: self.expected_repository_id,
            allowed_actor_ids: self.allowed_actor_ids.clone(),
            allowed_client_uids: self.allowed_client_uids.clone(),
            required_client_gid: self.required_client_gid,
            verifying_key: VerifyingKey::from_bytes(&key).map_err(|_| ConfigError::Invariant)?,
        })
    }

    fn validate(&self, production_paths: bool) -> Result<(), ConfigError> {
        if self.schema_version != 1
            || self.socket_mode != 0o660
            || self.expected_repository_id == 0
            || self.required_client_gid == 0
            || self.allowed_actor_ids.is_empty()
            || self.allowed_client_uids.is_empty()
            || has_duplicate(&self.allowed_actor_ids)
            || has_duplicate(&self.allowed_client_uids)
            || self.allowed_actor_ids.contains(&0)
            || self.allowed_client_uids.contains(&0)
            || decode_key(&self.verification_key_hex).is_none()
            || !absolute_without_parent(&self.socket_path)
            || !absolute_without_parent(&self.campaign_root)
        {
            return Err(ConfigError::Invariant);
        }
        if production_paths
            && (self.socket_path != Path::new(PRODUCTION_SOCKET)
                || self.campaign_root != Path::new(PRODUCTION_CAMPAIGN_ROOT))
        {
            return Err(ConfigError::Invariant);
        }
        Ok(())
    }
}

fn has_duplicate<T: Ord + Copy>(values: &[T]) -> bool {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    sorted.windows(2).any(|pair| pair[0] == pair[1])
}

fn absolute_without_parent(path: &Path) -> bool {
    path.is_absolute()
        && path.components().all(|component| {
            !matches!(
                component,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        })
}

fn decode_key(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64 {
        return None;
    }
    let mut decoded = [0_u8; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        decoded[index] = (nibble(pair[0])? << 4) | nibble(pair[1])?;
    }
    Some(decoded)
}

fn nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}
