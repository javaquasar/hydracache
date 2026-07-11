use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use hydracache_redis_compat::RedisRespServer;
use rustls::pki_types::{
    pem::{Error as PemError, PemObject},
    CertificateDer, PrivateKeyDer,
};
use thiserror::Error;
use tokio::net::TcpListener;
use tokio::sync::watch;
use tokio_rustls::TlsAcceptor;

use crate::admin_http::SharedServerRuntime;
use crate::config::TlsConfig;

/// TCP accept-loop failures for the optional Redis RESP listener.
#[derive(Debug, Error)]
pub enum RedisTcpError {
    /// Accepting a TCP connection failed.
    #[error("redis tcp accept error: {0}")]
    Accept(#[from] std::io::Error),
}

/// Redis RESP TLS acceptor backed by the server TLS certificate/key material.
#[derive(Clone)]
pub struct RedisTlsAcceptor {
    inner: TlsAcceptor,
}

impl RedisTlsAcceptor {
    /// Build a Redis TLS acceptor from the shared server TLS config.
    pub fn from_tls_config(config: &TlsConfig) -> Result<Self, RedisTlsError> {
        let cert_path = config
            .cert_path
            .as_deref()
            .ok_or(RedisTlsError::MissingCertPath)?;
        let key_path = config
            .key_path
            .as_deref()
            .ok_or(RedisTlsError::MissingKeyPath)?;
        Self::from_pem_files(cert_path, key_path)
    }

    /// Build a Redis TLS acceptor from PEM files.
    pub fn from_pem_files(cert_path: &Path, key_path: &Path) -> Result<Self, RedisTlsError> {
        install_default_rustls_provider();
        let certs = read_certs(cert_path)?;
        let key = read_private_key(key_path)?;
        let config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(|source| RedisTlsError::InvalidTlsConfig {
                cert_path: cert_path.to_path_buf(),
                key_path: key_path.to_path_buf(),
                source: Box::new(source),
            })?;
        Ok(Self {
            inner: TlsAcceptor::from(Arc::new(config)),
        })
    }

    async fn accept(
        &self,
        stream: tokio::net::TcpStream,
    ) -> Result<tokio_rustls::server::TlsStream<tokio::net::TcpStream>, std::io::Error> {
        self.inner.accept(stream).await
    }
}

/// Redis TLS startup failures.
#[derive(Debug, Error)]
pub enum RedisTlsError {
    /// TLS is enabled without a certificate path.
    #[error("redis rediss requires tls.cert_path")]
    MissingCertPath,
    /// TLS is enabled without a private key path.
    #[error("redis rediss requires tls.key_path")]
    MissingKeyPath,
    /// Certificate file could not be read.
    #[error("failed to read redis TLS certificate {path}: {source}")]
    CertRead {
        /// Certificate path.
        path: PathBuf,
        /// IO source.
        source: std::io::Error,
    },
    /// Certificate file did not contain any certificate.
    #[error("redis TLS certificate {path} does not contain any certificate")]
    EmptyCert { path: PathBuf },
    /// Certificate file could not be parsed.
    #[error("failed to parse redis TLS certificate {path}: {source}")]
    CertParse {
        /// Certificate path.
        path: PathBuf,
        /// Parse source.
        source: PemError,
    },
    /// Private key file could not be read.
    #[error("failed to read redis TLS private key {path}: {source}")]
    KeyRead {
        /// Private key path.
        path: PathBuf,
        /// IO source.
        source: std::io::Error,
    },
    /// Private key file did not contain a key.
    #[error("redis TLS private key {path} does not contain a private key")]
    EmptyKey { path: PathBuf },
    /// Private key file could not be parsed.
    #[error("failed to parse redis TLS private key {path}: {source}")]
    KeyParse {
        /// Private key path.
        path: PathBuf,
        /// Parse source.
        source: PemError,
    },
    /// rustls rejected the certificate/key pair.
    #[error("invalid redis TLS certificate/key pair {cert_path} / {key_path}: {source}")]
    InvalidTlsConfig {
        /// Certificate path.
        cert_path: PathBuf,
        /// Private key path.
        key_path: PathBuf,
        /// rustls source.
        source: Box<rustls::Error>,
    },
}

/// Serve the optional Redis RESP listener until shutdown is requested.
pub async fn serve_redis_listener(
    listener: TcpListener,
    server: Arc<RedisRespServer>,
    runtime: SharedServerRuntime,
    tls: Option<RedisTlsAcceptor>,
    mut shutdown: watch::Receiver<bool>,
) -> Result<(), RedisTcpError> {
    loop {
        if *shutdown.borrow() {
            return Ok(());
        }
        tokio::select! {
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    return Ok(());
                }
            }
            accepted = listener.accept() => {
                let (stream, _) = accepted?;
                if !runtime
                    .lock()
                    .expect("server runtime mutex")
                    .begin_redis_connection()
                {
                    continue;
                }
                let guard = RedisConnectionGuard::new(Arc::clone(&runtime));
                let server = Arc::clone(&server);
                let tls = tls.clone();
                tokio::spawn(async move {
                    let _guard = guard;
                    match tls {
                        Some(tls) => {
                            if let Ok(stream) = tls.accept(stream).await {
                                let _ = server.serve_connection(stream).await;
                            }
                        }
                        None => {
                            let _ = server.serve_connection(stream).await;
                        }
                    }
                });
            }
        }
    }
}

fn read_certs(path: &Path) -> Result<Vec<CertificateDer<'static>>, RedisTlsError> {
    let pem = fs::read(path).map_err(|source| RedisTlsError::CertRead {
        path: path.to_path_buf(),
        source,
    })?;
    let certs = CertificateDer::pem_slice_iter(&pem)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| RedisTlsError::CertParse {
            path: path.to_path_buf(),
            source,
        })?;
    if certs.is_empty() {
        return Err(RedisTlsError::EmptyCert {
            path: path.to_path_buf(),
        });
    }
    Ok(certs)
}

fn read_private_key(path: &Path) -> Result<PrivateKeyDer<'static>, RedisTlsError> {
    let pem = fs::read(path).map_err(|source| RedisTlsError::KeyRead {
        path: path.to_path_buf(),
        source,
    })?;
    match PrivateKeyDer::from_pem_slice(&pem) {
        Ok(key) => Ok(key),
        Err(PemError::NoItemsFound) => Err(RedisTlsError::EmptyKey {
            path: path.to_path_buf(),
        }),
        Err(source) => Err(RedisTlsError::KeyParse {
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn install_default_rustls_provider() {
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    }
}

struct RedisConnectionGuard {
    runtime: SharedServerRuntime,
}

impl RedisConnectionGuard {
    fn new(runtime: SharedServerRuntime) -> Self {
        Self { runtime }
    }
}

impl Drop for RedisConnectionGuard {
    fn drop(&mut self) {
        self.runtime
            .lock()
            .expect("server runtime mutex")
            .finish_redis_connection();
    }
}
