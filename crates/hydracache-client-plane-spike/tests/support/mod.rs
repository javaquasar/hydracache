#![allow(dead_code)]

use std::sync::Arc;

use rcgen::{
    date_time_ymd, BasicConstraints, CertificateParams, CertifiedIssuer, ExtendedKeyUsagePurpose,
    IsCa, KeyPair,
};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeafValidity {
    Valid,
    Expired,
    NotYetValid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeafPurpose {
    Correct,
    Wrong,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PkiProfile {
    pub server_name: String,
    pub server_validity: LeafValidity,
    pub client_validity: LeafValidity,
    pub server_purpose: LeafPurpose,
    pub client_purpose: LeafPurpose,
}

impl Default for PkiProfile {
    fn default() -> Self {
        Self {
            server_name: "localhost".to_owned(),
            server_validity: LeafValidity::Valid,
            client_validity: LeafValidity::Valid,
            server_purpose: LeafPurpose::Correct,
            client_purpose: LeafPurpose::Correct,
        }
    }
}

impl TestPki {
    pub fn generate() -> Self {
        Self::generate_with(PkiProfile::default())
    }

    pub fn generate_with(profile: PkiProfile) -> Self {
        // The workspace enables more than one rustls provider transitively.
        // Select one explicitly before constructing configs in each test binary.
        let _ = rustls::crypto::ring::default_provider().install_default();
        let mut ca_params = CertificateParams::new(Vec::<String>::new()).unwrap();
        ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        let ca = CertifiedIssuer::self_signed(ca_params, KeyPair::generate().unwrap()).unwrap();

        let server_key = KeyPair::generate().unwrap();
        let mut server_params = CertificateParams::new(vec![profile.server_name]).unwrap();
        apply_validity(&mut server_params, profile.server_validity);
        server_params.extended_key_usages = vec![match profile.server_purpose {
            LeafPurpose::Correct => ExtendedKeyUsagePurpose::ServerAuth,
            LeafPurpose::Wrong => ExtendedKeyUsagePurpose::ClientAuth,
        }];
        let server_cert = server_params.signed_by(&server_key, &ca).unwrap();
        let client_key = KeyPair::generate().unwrap();
        let mut client_params = CertificateParams::new(vec!["hc2-client".to_owned()]).unwrap();
        apply_validity(&mut client_params, profile.client_validity);
        client_params.extended_key_usages = vec![match profile.client_purpose {
            LeafPurpose::Correct => ExtendedKeyUsagePurpose::ClientAuth,
            LeafPurpose::Wrong => ExtendedKeyUsagePurpose::ServerAuth,
        }];
        let client_cert = client_params.signed_by(&client_key, &ca).unwrap();

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
        self.server_acceptor_trusting(&[self])
    }

    pub fn server_acceptor_trusting(&self, trusted_client_roots: &[&Self]) -> TlsAcceptor {
        self.server_acceptor_trusting_with_versions(trusted_client_roots, rustls::DEFAULT_VERSIONS)
    }

    pub fn server_acceptor_tls13_only(&self) -> TlsAcceptor {
        self.server_acceptor_trusting_with_versions(&[self], &[&rustls::version::TLS13])
    }

    pub fn server_acceptor_aes256_only(&self) -> TlsAcceptor {
        let mut client_root_store = RootCertStore::empty();
        client_root_store.add(self.ca_der.clone()).unwrap();
        let verifier = WebPkiClientVerifier::builder(Arc::new(client_root_store))
            .build()
            .unwrap();
        let mut provider = rustls::crypto::ring::default_provider();
        provider
            .cipher_suites
            .retain(|suite| suite.suite() == rustls::CipherSuite::TLS13_AES_256_GCM_SHA384);
        let config = ServerConfig::builder_with_provider(Arc::new(provider))
            .with_protocol_versions(&[&rustls::version::TLS13])
            .unwrap()
            .with_client_cert_verifier(verifier)
            .with_single_cert(
                vec![self.server_cert_der.clone()],
                PrivateKeyDer::Pkcs8(self.server_key_der.clone_key()),
            )
            .unwrap();
        TlsAcceptor::from(Arc::new(config))
    }

    fn server_acceptor_trusting_with_versions(
        &self,
        trusted_client_roots: &[&Self],
        versions: &[&'static rustls::SupportedProtocolVersion],
    ) -> TlsAcceptor {
        let mut client_root_store = RootCertStore::empty();
        for root in trusted_client_roots {
            client_root_store.add(root.ca_der.clone()).unwrap();
        }
        let verifier = WebPkiClientVerifier::builder(Arc::new(client_root_store))
            .build()
            .unwrap();
        let config = ServerConfig::builder_with_protocol_versions(versions)
            .with_client_cert_verifier(verifier)
            .with_single_cert(
                vec![self.server_cert_der.clone()],
                PrivateKeyDer::Pkcs8(self.server_key_der.clone_key()),
            )
            .unwrap();
        TlsAcceptor::from(Arc::new(config))
    }

    pub fn client_connector(&self) -> TlsConnector {
        self.client_connector_with_identity_and_roots(self, &[self])
    }

    pub fn client_connector_with_identity(&self, identity: &Self) -> TlsConnector {
        self.client_connector_with_identity_and_roots(identity, &[self])
    }

    pub fn client_connector_with_identity_and_roots(
        &self,
        identity: &Self,
        trusted_server_roots: &[&Self],
    ) -> TlsConnector {
        self.client_connector_with_identity_roots_and_versions(
            identity,
            trusted_server_roots,
            rustls::DEFAULT_VERSIONS,
        )
    }

    pub fn client_connector_tls12_only(&self) -> TlsConnector {
        self.client_connector_with_identity_roots_and_versions(
            self,
            &[self],
            &[&rustls::version::TLS12],
        )
    }

    pub fn client_connector_aes128_only(&self) -> TlsConnector {
        let mut server_root_store = RootCertStore::empty();
        server_root_store.add(self.ca_der.clone()).unwrap();
        let mut provider = rustls::crypto::ring::default_provider();
        provider
            .cipher_suites
            .retain(|suite| suite.suite() == rustls::CipherSuite::TLS13_AES_128_GCM_SHA256);
        let config = ClientConfig::builder_with_provider(Arc::new(provider))
            .with_protocol_versions(&[&rustls::version::TLS13])
            .unwrap()
            .with_root_certificates(server_root_store)
            .with_client_auth_cert(
                vec![self.client_cert_der.clone()],
                PrivateKeyDer::Pkcs8(self.client_key_der.clone_key()),
            )
            .unwrap();
        TlsConnector::from(Arc::new(config))
    }

    fn client_connector_with_identity_roots_and_versions(
        &self,
        identity: &Self,
        trusted_server_roots: &[&Self],
        versions: &[&'static rustls::SupportedProtocolVersion],
    ) -> TlsConnector {
        let mut server_root_store = RootCertStore::empty();
        for root in trusted_server_roots {
            server_root_store.add(root.ca_der.clone()).unwrap();
        }
        let config = ClientConfig::builder_with_protocol_versions(versions)
            .with_root_certificates(server_root_store)
            .with_client_auth_cert(
                vec![identity.client_cert_der.clone()],
                PrivateKeyDer::Pkcs8(identity.client_key_der.clone_key()),
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

fn apply_validity(params: &mut CertificateParams, validity: LeafValidity) {
    match validity {
        LeafValidity::Valid => {}
        LeafValidity::Expired => {
            params.not_before = date_time_ymd(2000, 1, 1);
            params.not_after = date_time_ymd(2001, 1, 1);
        }
        LeafValidity::NotYetValid => {
            params.not_before = date_time_ymd(4090, 1, 1);
            params.not_after = date_time_ymd(4091, 1, 1);
        }
    }
}
