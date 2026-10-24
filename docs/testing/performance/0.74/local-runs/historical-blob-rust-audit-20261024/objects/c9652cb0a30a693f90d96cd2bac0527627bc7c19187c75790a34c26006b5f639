//! Independent pinned-byte controls; not a production transport or source view.
#[path = "../../../tools/historical-blob-batch-074/decoder.rs"]
mod decoder;
use decoder::{Expected, Kind, Record, Refusal};
use sha2::{Digest, Sha256};

const OID: &str = "1111111111111111111111111111111111111111";
const OTHER: &str = "2222222222222222222222222222222222222222";

fn expected(kind: Kind, bytes: &[u8]) -> Expected {
    Expected::Object {
        oid: OID.to_owned(),
        kind,
        bytes: bytes.len(),
        sha256: Sha256::digest(bytes).into(),
    }
}
fn stream(kind: &str, bytes: &[u8]) -> Vec<u8> {
    let mut value = format!("{OID} {kind} {}\n", bytes.len()).into_bytes();
    value.extend_from_slice(bytes);
    value.push(b'\n');
    value
}

#[test]
fn every_kind_empty_binary_and_large_bytes_match_the_independent_reference() {
    let bodies = [
        Vec::new(),
        (0..=255).collect(),
        b"a\0\r\nb\n".repeat(65_536),
    ];
    for (kind, name) in [
        (Kind::Blob, "blob"),
        (Kind::Tree, "tree"),
        (Kind::Commit, "commit"),
        (Kind::Tag, "tag"),
    ] {
        for body in &bodies {
            let raw = stream(name, body);
            let pins = [expected(kind, body)];
            assert_eq!(
                decoder::decode(&raw, &pins),
                Ok(vec![Record::Object { kind, bytes: body }])
            );
        }
    }
}

#[test]
fn missing_and_duplicate_records_keep_original_order() {
    let spec = format!("{OID}:folder/space λ");
    let mut raw = stream("blob", b"first");
    raw.extend_from_slice(format!("{spec} missing\n").as_bytes());
    raw.extend_from_slice(&stream("blob", b"first"));
    let reference = [
        expected(Kind::Blob, b"first"),
        Expected::Missing { spec },
        expected(Kind::Blob, b"first"),
    ];
    assert_eq!(
        decoder::decode(&raw, &reference),
        Ok(vec![
            Record::Object {
                kind: Kind::Blob,
                bytes: b"first"
            },
            Record::Missing,
            Record::Object {
                kind: Kind::Blob,
                bytes: b"first"
            },
        ])
    );
}

#[test]
fn modified_bytes_cannot_create_their_own_pin_or_publish_a_valid_prefix() {
    let good = stream("blob", b"fixed");
    let pins = [
        expected(Kind::Blob, b"fixed"),
        expected(Kind::Blob, b"fixed"),
    ];
    let mut changed = good.clone();
    changed.extend_from_slice(&stream("blob", b"drift"));
    assert_eq!(decoder::decode(&changed, &pins), Err(Refusal::Content));
    let wrong_pin = [expected(Kind::Blob, b"drift")];
    assert_eq!(decoder::decode(&good, &wrong_pin), Err(Refusal::Content));
}

#[test]
fn every_truncated_prefix_and_extra_suffix_refuse() {
    let raw = stream("blob", b"a\0\r\ncontent");
    let pins = [expected(Kind::Blob, b"a\0\r\ncontent")];
    for end in 0..raw.len() {
        assert!(decoder::decode(&raw[..end], &pins).is_err(), "prefix {end}");
    }
    assert!(decoder::decode(&[raw.clone(), b"extra".to_vec()].concat(), &pins).is_err());
    let mut wrong_terminator = raw;
    *wrong_terminator.last_mut().unwrap() = b'X';
    assert_eq!(
        decoder::decode(&wrong_terminator, &pins),
        Err(Refusal::Truncated)
    );
}

#[test]
fn wrong_oid_type_count_and_noncanonical_headers_refuse() {
    let pins = [expected(Kind::Blob, b"fixed")];
    for header in [
        format!("{OTHER} blob 5"),
        format!("{OID} tree 5"),
        format!("{OID} blob 05"),
        format!("{OID} blob -5"),
        format!("{OID} blob 4"),
        format!("{OID} blob 99999999999999999999999999"),
        format!("{OID} blob 5 "),
        format!("{OID}\tblob 5"),
        format!("{OID} blob 5\r"),
    ] {
        assert_eq!(
            decoder::decode(format!("{header}\nfixed\n").as_bytes(), &pins),
            Err(Refusal::Header)
        );
    }
}

#[test]
fn invalid_reference_or_missing_identity_never_accepts_the_stream() {
    for id in [
        "HEAD",
        "abc",
        "F111111111111111111111111111111111111111",
        "",
    ] {
        let mut pin = expected(Kind::Blob, b"fixed");
        if let Expected::Object { oid, .. } = &mut pin {
            *oid = id.to_owned();
        }
        assert_eq!(decoder::decode(&[], &[pin]), Err(Refusal::Reference));
    }
    for path in [
        "", "./a", "/a", "a/../b", "a//b", "a\0b", "a\nb", "a\rb", "a\\b",
    ] {
        let pin = Expected::Missing {
            spec: format!("{OID}:{path}"),
        };
        assert_eq!(decoder::decode(&[], &[pin]), Err(Refusal::Reference));
    }
    let pin = Expected::Missing {
        spec: format!("{OID}:absent"),
    };
    for raw in [
        format!("{OTHER}:absent missing\n"),
        format!("{OID}:absent ambiguous\n"),
    ] {
        assert_eq!(
            decoder::decode(raw.as_bytes(), std::slice::from_ref(&pin)),
            Err(Refusal::Header)
        );
    }
}

#[test]
fn size_record_count_and_header_budgets_refuse() {
    assert_eq!(
        decoder::decode(&vec![0; decoder::MAX_STREAM + 1], &[]),
        Err(Refusal::Budget)
    );
    let pins = vec![expected(Kind::Blob, b""); decoder::MAX_RECORDS + 1];
    assert_eq!(decoder::decode(&[], &pins), Err(Refusal::Budget));
    let pin = Expected::Object {
        oid: OID.into(),
        kind: Kind::Blob,
        bytes: decoder::MAX_STREAM + 1,
        sha256: [0; 32],
    };
    assert_eq!(decoder::decode(&[], &[pin]), Err(Refusal::Reference));
    assert_eq!(
        decoder::decode(&vec![b'x'; 5000], &[expected(Kind::Blob, b"")]),
        Err(Refusal::Header)
    );
    let pin = Expected::Missing {
        spec: format!("{OID}:{}", "x".repeat(4097)),
    };
    assert_eq!(decoder::decode(&[], &[pin]), Err(Refusal::Reference));
    assert_eq!(decoder::decode(&[], &[]), Ok(vec![]));
    assert_eq!(decoder::decode(b"x", &[]), Err(Refusal::Extra));
}

#[test]
fn independent_threads_and_observations_cannot_share_stale_records() {
    std::thread::scope(|scope| {
        let threads: Vec<_> = (0..8)
            .map(|n| {
                scope.spawn(move || {
                    let body = format!("independent observation {n}");
                    let raw = stream("blob", body.as_bytes());
                    let pins = [expected(Kind::Blob, body.as_bytes())];
                    assert!(decoder::decode(&raw, &pins).is_ok());
                    assert_eq!(
                        decoder::decode(&raw, &[expected(Kind::Blob, b"stale")]),
                        Err(Refusal::Header)
                    );
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
    });
}
