//! Fixed public-SPKI distrust regression. No compromised private key is used.
//! Public inputs: rustls/webpki 8ecdcc76f56b68fda67a6adf7976f5eed5006a4c,
//! src/data/distrusted/*.spki.der (ISC upstream). Provenance is retained in evidence.
use rcgen::{
    BasicConstraints, CertificateParams, CertifiedIssuer, IsCa, KeyPair, PublicKeyData,
    SignatureAlgorithm, PKCS_ECDSA_P384_SHA384,
};
use rustls::{
    client::{danger::ServerCertVerifier, WebPkiServerVerifier},
    pki_types::{CertificateDer, ServerName, UnixTime},
    RootCertStore,
};
use std::{sync::Arc, time::Duration};

const PUBLIC_SPKIS: &[(&str, &str)] = &[
    ("EMDVTLSCAG3B.spki.der", "301006072a8648ce3d020106052b8104002203620004e79ed9c04a9773c4735e51111da85ac2bd5e64ab374b80e12cbc2038ec1e4f0e3b812fc9859e1e67afb04e2f8bc97f7485ba65aa911ee8a4e45b62d5cc03d56be678b3c38cb4dc937ca2460e238453c892935d29d44f91e15606b63ccedab468"),
    ("EMDVTLSCAG3C.spki.der", "301006072a8648ce3d020106052b81040022036200043a7a7862c8c44d324ddd69033a6782c0efeefe4055e68a123fa0371ba3bd9194514ee172a433e3630f02bd188037d9e5149f9b77db94a281df987b2fea888b74bdfe8fda4c1099ee6ca19566fe3ef0e5f6b09a556ccc8ba50561df1b2a441d01"),
    ("EMEVTLSCAG3B.spki.der", "301006072a8648ce3d020106052b810400220362000425dba85a64d397da0b1052e9ea546e487befea9c9d9d68a0c5eadcfeef75a69bb5d6fc01f42bfbca6eb40bcd64c0767e36ffd05d97a813c5aa0c6c9530e79cf5f0866468f13806420aa5c851840b1e252540e790e4b654c3852ed64af1e8657d"),
    ("EMEVTLSCAG3C.spki.der", "301006072a8648ce3d020106052b81040022036200042d56e829a6fb82c64a4697d960af951e9067ae91f926d34306da48a8aaeb7869b6a67e6d27426e0442aa03b36e1988dff636514ee1cf07f5ba4b3ff2a06645ae7d31d5866ad31aa3d246eb0b3177ee99cb7566c516ce74808a668f91f0c47fca"),
    ("EMOVCSCAG3B.spki.der", "301006072a8648ce3d020106052b8104002203620004aabb0fb0666acfcf01fad52bfcfd4c2bf2cb3c3497de73950e5819b61fe695098741c8c491439b34a0431caffe5bf9b4c1c8e8ae0001f9f4dc8ad82c59f3639e96cd6c3d4db0cefec26c3d62517b37761372743f8e8add77b10064f2abe5e050"),
    ("EMOVCSCAG3C.spki.der", "301006072a8648ce3d020106052b8104002203620004b9f0e0634702a69ca6aadd443cfd62c41b9ba4b20924697d7974e7691a126fb96afb3e34c2fd3bc5f483061f265dc2a58436347323510436b28f04f3862363067980e2503e02532d6546235a7165a259a1ef496f741b5fdee7787208e4b8faef"),
    ("EMOVTLSCAG3B.spki.der", "301006072a8648ce3d020106052b810400220362000473897aa0eb3cfdca8b12402ffaf0abc4e1212baf8dee778ac9ee2d6db83fac280c5e05638ae7a2acd71c322a8e361cdeb9f69c4302b9dd8d73d1cc6caab2cc0aee4e216ce979dd518a4af4b626e5684cdd658037ed0c854e4cc12f4cf8646d0d"),
    ("EMOVTLSCAG3C.spki.der", "301006072a8648ce3d020106052b8104002203620004138251cd52ac9298c1c8aad977321deb97e709bd0b4ca0aca55dc8ad51dcfc9d1589a1597e3a5120e1efd631c63e1835cacae29869a62e1631e8a28181ab56616dc45d918abc09f3ab0e63cf792aa4dced7387be37bba569549f1c02b270ed67"),
];

fn decode(raw: &str) -> Vec<u8> {
    assert_eq!(raw.len() % 2, 0);
    raw.as_bytes()
        .chunks_exact(2)
        .map(|word| u8::from_str_radix(std::str::from_utf8(word).unwrap(), 16).unwrap())
        .collect()
}
struct PublicPoint(Vec<u8>);
impl PublicPoint {
    fn from_contents(hex: &str) -> Self {
        let raw = decode(hex);
        // Fixed P-384 algorithm and BIT STRING header; the 97-byte point follows.
        let prefix = decode("301006072a8648ce3d020106052b81040022036200");
        assert_eq!(raw.len(), 118);
        assert!(raw.starts_with(&prefix));
        let point = raw[prefix.len()..].to_vec();
        assert_eq!(point.len(), 97);
        assert_eq!(point[0], 4);
        Self(point)
    }
}
impl PublicKeyData for PublicPoint {
    fn der_bytes(&self) -> &[u8] {
        &self.0
    }
    fn algorithm(&self) -> &'static SignatureAlgorithm {
        &PKCS_ECDSA_P384_SHA384
    }
}
fn issuer() -> CertifiedIssuer<'static, KeyPair> {
    let mut params = CertificateParams::new(Vec::<String>::new()).unwrap();
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    CertifiedIssuer::self_signed(params, KeyPair::generate().unwrap()).unwrap()
}
fn leaf(key: &impl PublicKeyData, ca: &CertifiedIssuer<'_, KeyPair>) -> CertificateDer<'static> {
    let mut params = CertificateParams::new(vec!["localhost".to_owned()]).unwrap();
    params.not_before = rcgen::date_time_ymd(2020, 1, 1);
    params.not_after = rcgen::date_time_ymd(2030, 1, 1);
    params.signed_by(key, ca).unwrap().der().clone()
}
fn verifier(ca: &CertifiedIssuer<'_, KeyPair>) -> Arc<WebPkiServerVerifier> {
    let mut roots = RootCertStore::empty();
    roots.add(ca.der().clone()).unwrap();
    WebPkiServerVerifier::builder_with_provider(
        Arc::new(roots),
        Arc::new(rustls::crypto::aws_lc_rs::default_provider()),
    )
    .build()
    .unwrap()
}
fn time(seconds: u64) -> UnixTime {
    UnixTime::since_unix_epoch(Duration::from_secs(seconds))
}
#[test]
fn all_eight_distrusted_public_keys_are_rejected_even_under_a_trusted_test_ca() {
    let ca = issuer();
    let verifier = verifier(&ca);
    let mut accepted = 0;
    let mut distrusted = 0;
    for (name, contents) in PUBLIC_SPKIS {
        let cert = leaf(&PublicPoint::from_contents(contents), &ca);
        match verifier.verify_server_cert(
            &cert,
            &[],
            &ServerName::try_from("localhost").unwrap(),
            &[],
            time(1_780_000_000),
        ) {
            Ok(_) => accepted += 1,
            Err(error) => {
                assert!(
                    format!("{error:?}").contains("CertDistrusted"),
                    "unexpected public fixture rejection for {name}: {error:?}"
                );
                distrusted += 1;
            }
        }
    }
    assert_eq!(
        accepted, 0,
        "distrusted public keys accepted by certificate verification"
    );
    assert_eq!(distrusted, 8);
}
#[test]
fn ordinary_chain_still_checks_name_time_and_issuer() {
    let ca = issuer();
    let other = issuer();
    let verifier = verifier(&ca);
    let key = KeyPair::generate().unwrap();
    let cert = leaf(&key, &ca);
    let good = ServerName::try_from("localhost").unwrap();
    assert!(verifier
        .verify_server_cert(&cert, &[], &good, &[], time(1_780_000_000))
        .is_ok());
    assert!(verifier
        .verify_server_cert(
            &cert,
            &[],
            &ServerName::try_from("foreign.invalid").unwrap(),
            &[],
            time(1_780_000_000)
        )
        .is_err());
    assert!(verifier
        .verify_server_cert(&cert, &[], &good, &[], time(0))
        .is_err());
    assert!(verifier
        .verify_server_cert(&cert, &[], &good, &[], time(2_000_000_000))
        .is_err());
    assert!(verifier
        .verify_server_cert(&leaf(&key, &other), &[], &good, &[], time(1_780_000_000))
        .is_err());
}
