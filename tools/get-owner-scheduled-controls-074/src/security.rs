//! Ephemeral transport material only. No application-security equivalence claim.
use rustls::{
    pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer},
    RootCertStore,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio_rustls::TlsConnector;

pub use crate::native::MtlsFixture;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TransportReceipt {
    pub root_certificate_sha256: String,
    pub server_certificate_sha256: String,
    pub client_certificate_sha256: String,
    pub server_name: &'static str,
    pub required_client_certificate: bool,
    pub cross_surface_numeric_comparison_allowed: bool,
}
fn certificate_hash(pem: &str) -> String {
    // Generated fixture has exactly one certificate; hash DER, not PEM wrapping.
    let cert = CertificateDer::from_pem_slice(pem.as_bytes()).expect("generated certificate");
    format!("{:x}", Sha256::digest(cert.as_ref()))
}
impl MtlsFixture {
    pub fn new() -> Result<Self, String> {
        crate::native::pki()
    }
    pub fn receipt(&self) -> TransportReceipt {
        TransportReceipt {
            root_certificate_sha256: certificate_hash(&self.ca),
            server_certificate_sha256: certificate_hash(&self.server_cert),
            client_certificate_sha256: certificate_hash(&self.client_cert),
            server_name: "localhost",
            required_client_certificate: true,
            cross_surface_numeric_comparison_allowed: false,
        }
    }
    pub(crate) fn connector(&self) -> Result<TlsConnector, String> {
        let mut roots = RootCertStore::empty();
        roots
            .add(CertificateDer::from_pem_slice(self.ca.as_bytes()).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        let config = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_client_auth_cert(
                vec![CertificateDer::from_pem_slice(self.client_cert.as_bytes())
                    .map_err(|e| e.to_string())?],
                PrivateKeyDer::from_pem_slice(self.client_key.as_bytes())
                    .map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
        Ok(TlsConnector::from(Arc::new(config)))
    }
    pub(crate) fn files(&self) -> Result<tempfile::TempDir, String> {
        let files = tempfile::tempdir().map_err(|e| e.to_string())?;
        for (name, contents) in [
            ("server.pem", &self.server_cert),
            ("server.key", &self.server_key),
            ("clients.pem", &self.ca),
        ] {
            std::fs::write(files.path().join(name), contents).map_err(|e| e.to_string())?;
        }
        Ok(files)
    }
}
/// Only transport material + exact dataset match. Never an admission receipt.
pub fn validate_transport_material_match(
    a: &TransportReceipt,
    b: &TransportReceipt,
    dataset_a: &str,
    dataset_b: &str,
) -> Result<(), String> {
    if a != b
        || !a.required_client_certificate
        || a.server_name != "localhost"
        || a.cross_surface_numeric_comparison_allowed
        || dataset_a != dataset_b
        || dataset_a.is_empty()
    {
        return Err(
            "transport material or dataset mismatch; numeric comparison remains forbidden"
                .to_owned(),
        );
    }
    for digest in [
        &a.root_certificate_sha256,
        &a.server_certificate_sha256,
        &a.client_certificate_sha256,
    ] {
        if digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("invalid transport certificate digest".to_owned());
        }
    }
    Ok(())
}
