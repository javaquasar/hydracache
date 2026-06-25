use std::fs;
use std::path::Path;

use hydracache_client_protocol::ClientFrame;

#[test]
fn golden_client_v1_fixtures_round_trip() {
    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/client-v1");
    let mut fixture_count = 0;

    for entry in fs::read_dir(&fixture_dir).expect("client-v1 fixture directory exists") {
        let entry = entry.expect("fixture entry is readable");
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("hex") {
            continue;
        }
        fixture_count += 1;
        let text = fs::read_to_string(&path).expect("fixture is readable");
        let bytes = decode_hex(&text);
        let frame = ClientFrame::decode(&bytes, 1024).expect("fixture decodes");

        assert_eq!(frame.protocol_version(), 1);
        assert_eq!(
            frame.encode().expect("fixture re-encodes").as_ref(),
            bytes.as_slice(),
            "fixture {} must round-trip byte-for-byte",
            path.display()
        );
    }

    assert!(
        fixture_count > 0,
        "at least one client-v1 fixture is checked in"
    );
}

fn decode_hex(input: &str) -> Vec<u8> {
    let compact = input
        .chars()
        .filter(|ch| !ch.is_whitespace() && *ch != '#')
        .collect::<String>();
    assert_eq!(compact.len() % 2, 0, "hex fixture must have an even length");

    compact
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = from_hex(pair[0]);
            let low = from_hex(pair[1]);
            (high << 4) | low
        })
        .collect()
}

fn from_hex(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => 10 + byte - b'a',
        b'A'..=b'F' => 10 + byte - b'A',
        _ => panic!("invalid hex byte {}", byte as char),
    }
}
