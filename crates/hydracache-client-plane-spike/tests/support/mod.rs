#![allow(dead_code)]

use std::sync::Arc;

use rcgen::{BasicConstraints, CertificateParams, CertifiedIssuer, IsCa, KeyPair};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::server::WebPkiClientVerifier;
use rustls::{ClientConfig, RootCertStore, ServerConfig};
use tokio_rustls::{TlsAcceptor, TlsConnector};

pub struct TestPki {
    pub ca_pem: String,
    pub server_cert_pem: String,
    pub server_key_pem: String,
    pub client_cert_pem: String,
    pub client_key_pem: String,
    ca_der: CertificateDer<'static>,
    server_cert_der: CertificateDer<'static>,
    server_key_der: PrivatePkcs8KeyDer<'static>,
    client_cert_der: CertificateDer<'static>,
    client_key_der: PrivatePkcs8KeyDer<'static>,
}

impl TestPki {
    pub fn generate() -> Self {
        // The workspace enables more than one rustls provider transitively.
        // Select one explicitly before constructing configs in each test binary.
        let _ = rustls::crypto::ring::default_provider().install_default();
        let mut ca_params = CertificateParams::new(Vec::<String>::new()).unwrap();
        ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        let ca = CertifiedIssuer::self_signed(ca_params, KeyPair::generate().unwrap()).unwrap();

        let server_key = KeyPair::generate().unwrap();
        let server_cert = CertificateParams::new(vec!["localhost".to_owned()])
            .unwrap()
            .signed_by(&server_key, &ca)
            .unwrap();
        let client_key = KeyPair::generate().unwrap();
        let client_cert = CertificateParams::new(vec!["hc2-client".to_owned()])
            .unwrap()
            .signed_by(&client_key, &ca)
            .unwrap();

        Self {
            ca_pem: ca.pem(),
            server_cert_pem: server_cert.pem(),
            server_key_pem: server_key.serialize_pem(),
            client_cert_pem: client_cert.pem(),
            client_key_pem: client_key.serialize_pem(),
            ca_der: ca.der().clone(),
            server_cert_der: server_cert.der().clone(),
            server_key_der: PrivatePkcs8KeyDer::from(server_key.serialize_der()),
            client_cert_der: client_cert.der().clone(),
            client_key_der: PrivatePkcs8KeyDer::from(client_key.serialize_der()),
        }
    }

    pub fn server_acceptor(&self) -> TlsAcceptor {
        let mut client_roots = RootCertStore::empty();
        client_roots.add(self.ca_der.clone()).unwrap();
        let verifier = WebPkiClientVerifier::builder(Arc::new(client_roots))
            .build()
            .unwrap();
        let config = ServerConfig::builder()
            .with_client_cert_verifier(verifier)
            .with_single_cert(
                vec![self.server_cert_der.clone()],
                PrivateKeyDer::Pkcs8(self.server_key_der.clone_key()),
            )
            .unwrap();
        TlsAcceptor::from(Arc::new(config))
    }

    pub fn client_connector(&self) -> TlsConnector {
        let mut server_roots = RootCertStore::empty();
        server_roots.add(self.ca_der.clone()).unwrap();
        let config = ClientConfig::builder()
            .with_root_certificates(server_roots)
            .with_client_auth_cert(
                vec![self.client_cert_der.clone()],
                PrivateKeyDer::Pkcs8(self.client_key_der.clone_key()),
            )
            .unwrap();
        TlsConnector::from(Arc::new(config))
    }

    pub fn client_connector_without_identity(&self) -> TlsConnector {
        let mut server_roots = RootCertStore::empty();
        server_roots.add(self.ca_der.clone()).unwrap();
        let config = ClientConfig::builder()
            .with_root_certificates(server_roots)
            .with_no_client_auth();
        TlsConnector::from(Arc::new(config))
    }
}
