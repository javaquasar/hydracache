use std::env;
use std::fs;
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Server role selected at startup.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerRole {
    /// Embedded-compatible single-process cache.
    #[default]
    Local,
    /// Cluster member that owns partitions and durable state.
    Member,
    /// Client/near-cache process that connects to members.
    Client,
}

/// TLS startup policy.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Whether TLS is enabled for externally reachable listeners.
    pub enabled: bool,
    /// Operator-supplied certificate path.
    pub cert_path: Option<PathBuf>,
    /// Operator-supplied private-key path.
    pub key_path: Option<PathBuf>,
    /// Operator-supplied CA bundle path.
    pub ca_path: Option<PathBuf>,
    /// Explicit acknowledgement for local/staging insecure deployments.
    pub acknowledge_insecure: bool,
}

impl TlsConfig {
    /// Return whether all configured TLS paths are present.
    pub fn has_complete_material(&self) -> bool {
        !self.enabled
            || (self.cert_path.is_some() && self.key_path.is_some() && self.ca_path.is_some())
    }
}

/// Backup startup policy.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupConfig {
    /// Whether background backup/PITR services are enabled.
    pub enabled: bool,
    /// Local/object-store destination URI.
    pub location: Option<String>,
}

/// Standalone daemon configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    /// Server role.
    pub role: ServerRole,
    /// Public actuator/data listen address.
    pub listen_addr: SocketAddr,
    /// Internal cluster listen address.
    pub cluster_addr: SocketAddr,
    /// Seed members used by member/client roles.
    pub seeds: Vec<String>,
    /// Durable state directory for member mode.
    pub storage_dir: Option<PathBuf>,
    /// Graceful shutdown drain timeout.
    pub drain_timeout_ms: u64,
    /// TLS policy.
    pub tls: TlsConfig,
    /// Backup policy.
    pub backup: BackupConfig,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            role: ServerRole::Local,
            listen_addr: "127.0.0.1:8080"
                .parse()
                .expect("default listen address is valid"),
            cluster_addr: "127.0.0.1:7000"
                .parse()
                .expect("default cluster address is valid"),
            seeds: Vec::new(),
            storage_dir: None,
            drain_timeout_ms: 30_000,
            tls: TlsConfig::default(),
            backup: BackupConfig::default(),
        }
    }
}

impl ServerConfig {
    /// Load config from a TOML file.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ServerConfigError> {
        let path = path.as_ref();
        let text = fs::read_to_string(path).map_err(|source| ServerConfigError::ConfigRead {
            path: path.to_path_buf(),
            source,
        })?;
        Self::from_toml_str(&text)
    }

    /// Load config from TOML text.
    pub fn from_toml_str(text: &str) -> Result<Self, ServerConfigError> {
        let config = toml::from_str::<Self>(text).map_err(ServerConfigError::ConfigParse)?;
        config.validate()?;
        Ok(config)
    }

    /// Load config from selected environment variables.
    pub fn from_env() -> Result<Self, ServerConfigError> {
        let mut config = Self::default();
        if let Ok(role) = env::var("HYDRACACHE_ROLE") {
            config.role = parse_role(&role)?;
        }
        if let Ok(listen) = env::var("HYDRACACHE_LISTEN_ADDR") {
            config.listen_addr = listen
                .parse()
                .map_err(|_| ServerConfigError::InvalidAddress(listen))?;
        }
        if let Ok(cluster) = env::var("HYDRACACHE_CLUSTER_ADDR") {
            config.cluster_addr = cluster
                .parse()
                .map_err(|_| ServerConfigError::InvalidAddress(cluster))?;
        }
        if let Ok(storage_dir) = env::var("HYDRACACHE_STORAGE_DIR") {
            config.storage_dir = Some(PathBuf::from(storage_dir));
        }
        if let Ok(seeds) = env::var("HYDRACACHE_SEEDS") {
            config.seeds = seeds
                .split(',')
                .map(str::trim)
                .filter(|seed| !seed.is_empty())
                .map(ToOwned::to_owned)
                .collect();
        }
        if env::var("HYDRACACHE_TLS_ACK_INSECURE").as_deref() == Ok("true") {
            config.tls.acknowledge_insecure = true;
        }
        if env::var("HYDRACACHE_TLS_ENABLED").as_deref() == Ok("true") {
            config.tls.enabled = true;
        }
        if let Ok(path) = env::var("HYDRACACHE_TLS_CERT_PATH") {
            config.tls.cert_path = Some(PathBuf::from(path));
        }
        if let Ok(path) = env::var("HYDRACACHE_TLS_KEY_PATH") {
            config.tls.key_path = Some(PathBuf::from(path));
        }
        if let Ok(path) = env::var("HYDRACACHE_TLS_CA_PATH") {
            config.tls.ca_path = Some(PathBuf::from(path));
        }
        if env::var("HYDRACACHE_BACKUP_ENABLED").as_deref() == Ok("true") {
            config.backup.enabled = true;
        }
        if let Ok(location) = env::var("HYDRACACHE_BACKUP_LOCATION") {
            config.backup.location = Some(location);
        }
        config.validate()?;
        Ok(config)
    }

    /// Validate startup invariants.
    pub fn validate(&self) -> Result<(), ServerConfigError> {
        if self.drain_timeout_ms == 0 {
            return Err(ServerConfigError::DrainTimeoutZero);
        }
        if matches!(self.role, ServerRole::Member) && self.storage_dir.is_none() {
            return Err(ServerConfigError::MissingStorageDir);
        }
        if matches!(self.role, ServerRole::Member | ServerRole::Client) && self.seeds.is_empty() {
            return Err(ServerConfigError::MissingSeeds);
        }
        if self.backup.enabled
            && self
                .backup
                .location
                .as_deref()
                .unwrap_or("")
                .trim()
                .is_empty()
        {
            return Err(ServerConfigError::MissingBackupLocation);
        }
        if !self.tls.has_complete_material() {
            return Err(ServerConfigError::IncompleteTlsMaterial);
        }
        if self.exposes_non_loopback() && !self.tls.enabled && !self.tls.acknowledge_insecure {
            return Err(ServerConfigError::NonLoopbackWithoutTls);
        }
        Ok(())
    }

    /// Return the configured drain timeout.
    pub fn drain_timeout(&self) -> Duration {
        Duration::from_millis(self.drain_timeout_ms)
    }

    /// Return whether any listener is externally reachable.
    pub fn exposes_non_loopback(&self) -> bool {
        !is_loopback(self.listen_addr.ip()) || !is_loopback(self.cluster_addr.ip())
    }
}

/// Fail-loud configuration errors.
#[derive(Debug, Error)]
pub enum ServerConfigError {
    /// Config file could not be read.
    #[error("failed to read config {path}: {source}")]
    ConfigRead {
        /// Config path.
        path: PathBuf,
        /// Source IO error.
        source: std::io::Error,
    },
    /// Config file could not be parsed.
    #[error("failed to parse config: {0}")]
    ConfigParse(toml::de::Error),
    /// Role value is unknown.
    #[error("invalid server role: {0}")]
    InvalidRole(String),
    /// Address value is invalid.
    #[error("invalid listen address: {0}")]
    InvalidAddress(String),
    /// Drain timeout cannot be zero.
    #[error("drain_timeout_ms must be greater than zero")]
    DrainTimeoutZero,
    /// Member mode requires durable state.
    #[error("member role requires storage_dir")]
    MissingStorageDir,
    /// Member/client mode requires seeds.
    #[error("member/client role requires at least one seed")]
    MissingSeeds,
    /// Backup enabled without a destination.
    #[error("backup.enabled requires backup.location")]
    MissingBackupLocation,
    /// TLS enabled without full material paths.
    #[error("tls.enabled requires cert_path, key_path, and ca_path")]
    IncompleteTlsMaterial,
    /// External listener without TLS and without explicit acknowledgement.
    #[error("non-loopback listeners require TLS or acknowledge_insecure=true")]
    NonLoopbackWithoutTls,
}

fn parse_role(value: &str) -> Result<ServerRole, ServerConfigError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "local" => Ok(ServerRole::Local),
        "member" => Ok(ServerRole::Member),
        "client" => Ok(ServerRole::Client),
        _ => Err(ServerConfigError::InvalidRole(value.to_owned())),
    }
}

fn is_loopback(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => ip.is_loopback(),
        IpAddr::V6(ip) => ip.is_loopback(),
    }
}
