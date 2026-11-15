//! Public root-catalog regression, not a certificate-chain exploit reproduction.
//! Public emSign G3 SPKI from the published webpki-roots 1.0.8 catalog.
use rustls::{pki_types::TrustAnchor, RootCertStore};

const DISALLOWED_SPKI: &[u8] = b"0\x10\x06\x07*\x86H\xce=\x02\x01\x06\x05+\x81\x04\x00\"\x03b\x00\x04#\xa5\x0c\xb8-\x12\xf5(\xf3\xb1\xb2\xdd\xe2\x02\x12\x80\x9e9_IM\x9f\xc9%4Yt\xec\xbb\x06\x1c\xe7\xc0r\xaf\xe8\xae/\xe1AT\x87\x14\xa8J\xb2\xe8|\x82\xe6[j\xb5\xdc\xb3u\xce\x8b\x06\xd0\x86#\xbfF\xd5\x8e\x0f?\x04\xf4\xd7\x1c\x92~\xf6\xa5c\xc2\xf5_\x8e.O\xa1\x18\x19\x02+2\n\x82d}\x16\x93\xd1";

fn bad_anchor(root: &TrustAnchor<'_>) -> bool {
    root.subject_public_key_info.as_ref() == DISALLOWED_SPKI
}

#[test]
fn bundled_anchor_catalog_does_not_trust_emsign_g3() {
    assert!(!webpki_roots::TLS_SERVER_ROOTS.is_empty());
    assert_eq!(DISALLOWED_SPKI.len(), 118);
    assert!(!webpki_roots::TLS_SERVER_ROOTS.iter().any(bad_anchor));
}

#[test]
fn bundled_der_catalog_does_not_contain_emsign_g3_public_key() {
    assert!(!webpki_root_certs::TLS_SERVER_ROOT_CERTS.is_empty());
    assert!(
        !webpki_root_certs::TLS_SERVER_ROOT_CERTS.iter().any(|cert| {
            cert.as_ref()
                .windows(DISALLOWED_SPKI.len())
                .any(|key| key == DISALLOWED_SPKI)
        })
    );
}

#[test]
fn compatibility_shim_reexports_the_same_parent_without_its_own_trust_store() {
    assert_eq!(
        webpki_roots_legacy::TLS_SERVER_ROOTS,
        webpki_roots::TLS_SERVER_ROOTS
    );
}

#[test]
fn complete_der_and_anchor_catalogs_load_into_the_normal_verifier_store() {
    let mut from_der = RootCertStore::empty();
    for cert in webpki_root_certs::TLS_SERVER_ROOT_CERTS {
        from_der.add(cert.clone()).unwrap();
    }
    let from_anchors = RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    };
    assert!(!from_der.is_empty());
    assert!(catalogs_match_registered_policy(
        &from_der.roots,
        &from_anchors.roots
    ));
}

fn catalogs_match_registered_policy(der: &[TrustAnchor<'_>], anchors: &[TrustAnchor<'_>]) -> bool {
    if der.is_empty() || der.len() != anchors.len() {
        return false;
    }
    let order = |a: &TrustAnchor<'_>, b: &TrustAnchor<'_>| {
        (
            a.subject.as_ref(),
            a.subject_public_key_info.as_ref(),
            a.name_constraints.as_ref().map(|v| v.as_ref()),
        )
            .cmp(&(
                b.subject.as_ref(),
                b.subject_public_key_info.as_ref(),
                b.name_constraints.as_ref().map(|v| v.as_ref()),
            ))
    };
    let mut actual = der.to_vec();
    let mut expected = anchors.to_vec();
    actual.sort_by(order);
    expected.sort_by(order);
    let mut registered_difference = 0;
    for (actual, expected) in actual.iter().zip(&expected) {
        if actual.subject != expected.subject
            || actual.subject_public_key_info != expected.subject_public_key_info
        {
            return false;
        }
        if actual.name_constraints != expected.name_constraints {
            let subject = b"TUBITAK Kamu SM SSL Kok Sertifikasi - Surum 1";
            if !actual
                .subject
                .as_ref()
                .windows(subject.len())
                .any(|v| v == subject)
                || actual.name_constraints.is_some()
                || expected.name_constraints.as_ref().map(|v| v.as_ref())
                    != Some(b"\xa0\x07\x30\x05\x82\x03.tr".as_slice())
            {
                return false;
            }
            registered_difference += 1;
        }
    }
    registered_difference == 1
}

#[test]
fn catalog_oracle_refuses_unregistered_policy_and_key_mutations() {
    let mut store = RootCertStore::empty();
    for cert in webpki_root_certs::TLS_SERVER_ROOT_CERTS {
        store.add(cert.clone()).unwrap();
    }
    let mut altered = webpki_roots::TLS_SERVER_ROOTS.to_vec();
    assert!(catalogs_match_registered_policy(&store.roots, &altered));
    let constrained = altered
        .iter()
        .position(|p| p.name_constraints.is_some())
        .unwrap();
    altered[constrained].name_constraints = None;
    assert!(!catalogs_match_registered_policy(&store.roots, &altered));
    altered = webpki_roots::TLS_SERVER_ROOTS.to_vec();
    altered[0].subject_public_key_info = altered[1].subject_public_key_info.clone();
    assert!(!catalogs_match_registered_policy(&store.roots, &altered));
    assert!(!catalogs_match_registered_policy(
        &store.roots[..store.roots.len() - 1],
        webpki_roots::TLS_SERVER_ROOTS
    ));
}
