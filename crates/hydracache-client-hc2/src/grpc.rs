use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures_util::StreamExt;
use tokio::sync::{mpsc, watch};
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::{Certificate, Channel, ClientTlsConfig, Endpoint, Identity};

use crate::adapter::{AdapterConnection, ConnectionControl, TransportAdapter};
use crate::wire::client_plane_alpha_client::ClientPlaneAlphaClient;
use crate::{ClientConfig, ClientError, ErrorCode, RetryAdvice, TransportKind};

/// Mandatory mTLS material for the gRPC HC/2 adapter.
#[derive(Clone)]
pub struct GrpcMtlsConfig {
    endpoint: String,
    server_name: String,
    ca_pem: Vec<u8>,
    client_certificate_pem: Vec<u8>,
    client_private_key_pem: Vec<u8>,
}

impl std::fmt::Debug for GrpcMtlsConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("GrpcMtlsConfig")
            .field("endpoint", &self.endpoint)
            .field("server_name", &self.server_name)
            .field("ca_pem", &"[redacted]")
            .field("client_certificate_pem", &"[redacted]")
            .field("client_private_key_pem", &"[redacted]")
            .finish()
    }
}

impl GrpcMtlsConfig {
    /// Build fail-closed gRPC+mTLS material. No plaintext constructor exists.
    pub fn new(
        endpoint: impl Into<String>,
        server_name: impl Into<String>,
        ca_pem: impl Into<Vec<u8>>,
        client_certificate_pem: impl Into<Vec<u8>>,
        client_private_key_pem: impl Into<Vec<u8>>,
    ) -> Result<Self, ClientError> {
        let value = Self {
            endpoint: endpoint.into(),
            server_name: server_name.into(),
            ca_pem: ca_pem.into(),
            client_certificate_pem: client_certificate_pem.into(),
            client_private_key_pem: client_private_key_pem.into(),
        };
        let authority = value.endpoint.strip_prefix("https://");
        if authority.is_none_or(|authority| authority.is_empty() || authority.contains('/'))
            || value.endpoint.contains('@')
            || value.endpoint.contains('?')
            || value.endpoint.contains('#')
            || value.server_name.trim().is_empty()
            || value.server_name.chars().any(char::is_control)
            || value.ca_pem.is_empty()
            || value.client_certificate_pem.is_empty()
            || value.client_private_key_pem.is_empty()
        {
            return Err(ClientError::new(
                ErrorCode::InvalidRequest,
                RetryAdvice::Never,
                "invalid or incomplete gRPC mTLS configuration",
            ));
        }
        Ok(value)
    }
}

/// Generated gRPC bidirectional adapter over mandatory mTLS.
#[derive(Debug, Clone)]
pub struct GrpcMtlsAdapter {
    config: GrpcMtlsConfig,
}

impl GrpcMtlsAdapter {
    /// Create the adapter from validated mTLS material.
    pub const fn new(config: GrpcMtlsConfig) -> Self {
        Self { config }
    }
}

#[derive(Debug)]
struct GrpcControl(watch::Sender<bool>);

impl ConnectionControl for GrpcControl {
    fn close(&self) {
        let _ = self.0.send(true);
    }
}

#[async_trait]
impl TransportAdapter for GrpcMtlsAdapter {
    fn kind(&self) -> TransportKind {
        TransportKind::GrpcBidirectional
    }

    async fn connect(&self, config: &ClientConfig) -> Result<AdapterConnection, ClientError> {
        let tls = ClientTlsConfig::new()
            .ca_certificate(Certificate::from_pem(&self.config.ca_pem))
            .identity(Identity::from_pem(
                &self.config.client_certificate_pem,
                &self.config.client_private_key_pem,
            ))
            .domain_name(self.config.server_name.clone());
        let endpoint = Endpoint::from_shared(self.config.endpoint.clone())
            .map_err(|_| transport_error("invalid gRPC endpoint"))?
            .connect_timeout(config.connect_timeout)
            .timeout(Duration::from_secs(300))
            .tls_config(tls)
            .map_err(|_| transport_error("invalid gRPC TLS policy"))?;
        let channel: Channel = endpoint
            .connect()
            .await
            .map_err(|_| transport_error("gRPC mTLS connection failed"))?;
        let mut client = ClientPlaneAlphaClient::new(channel)
            .max_decoding_message_size(config.limits.max_inbound_message_bytes)
            .max_encoding_message_size(config.limits.max_inbound_message_bytes);
        let (outbound, outbound_rx) = mpsc::channel(config.limits.max_outbound_frames);
        let (close_tx, mut close_rx) = watch::channel(false);
        let request_stream = ReceiverStream::new(outbound_rx).take_until(async move {
            while close_rx.changed().await.is_ok() {
                if *close_rx.borrow() {
                    break;
                }
            }
        });
        let mut response = client
            .open(request_stream)
            .await
            .map_err(|_| transport_error("gRPC HC/2 stream open failed"))?
            .into_inner();
        let (inbound_tx, inbound) = mpsc::channel(config.limits.max_outbound_frames);
        tokio::spawn(async move {
            loop {
                match response.message().await {
                    Ok(Some(envelope)) => {
                        if inbound_tx.send(Ok(envelope)).await.is_err() {
                            break;
                        }
                    }
                    Ok(None) => {
                        let _ = inbound_tx
                            .send(Err(transport_error("gRPC HC/2 stream closed")))
                            .await;
                        break;
                    }
                    Err(_) => {
                        let _ = inbound_tx
                            .send(Err(transport_error("gRPC HC/2 receive failed")))
                            .await;
                        break;
                    }
                }
            }
        });
        Ok(AdapterConnection::new(
            outbound,
            inbound,
            Arc::new(GrpcControl(close_tx)),
        ))
    }
}

const fn transport_error(detail: &'static str) -> ClientError {
    ClientError::new(
        ErrorCode::Unavailable,
        RetryAdvice::ReconnectIdempotent,
        detail,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid(endpoint: &str) -> Result<GrpcMtlsConfig, ClientError> {
        GrpcMtlsConfig::new(endpoint, "localhost", b"ca", b"certificate", b"private-key")
    }

    #[test]
    fn mtls_configuration_rejects_plaintext_paths_credentials_and_missing_material() {
        assert!(valid("https://localhost:9443").is_ok());
        for endpoint in [
            "http://localhost:9443",
            "https://localhost:9443/path",
            "https://user@localhost:9443",
            "https://localhost:9443?query",
            "https://localhost:9443#fragment",
        ] {
            assert!(
                valid(endpoint).is_err(),
                "accepted unsafe endpoint {endpoint}"
            );
        }
        assert!(GrpcMtlsConfig::new("https://localhost:9443", "localhost", [], [], []).is_err());
    }

    #[test]
    fn debug_output_never_contains_credentials() {
        let config = GrpcMtlsConfig::new(
            "https://localhost:9443",
            "localhost",
            b"secret-ca",
            b"secret-cert",
            b"secret-key",
        )
        .unwrap();
        let debug = format!("{config:?}");
        assert!(!debug.contains("secret"));
        assert_eq!(debug.matches("[redacted]").count(), 3);
    }
}
