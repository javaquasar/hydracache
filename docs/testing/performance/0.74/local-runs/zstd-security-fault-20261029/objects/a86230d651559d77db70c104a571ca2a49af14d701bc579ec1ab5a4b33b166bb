//! Independent upstream borrowed-context observations; no wire/MTU acceptance.
use serde_json::json;
use std::io::{Read, Write};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn snapshot() {
    let mut cctx = zstd::zstd_safe::CCtx::create();
    zstd::stream::write::Encoder::with_context(Vec::new(), &mut cctx)
        .write_all(b"first job")
        .unwrap();
    let mut encoder = zstd::stream::write::Encoder::with_context(Vec::new(), &mut cctx);
    encoder.write_all(b"second job").unwrap();
    let frame = encoder.finish().unwrap();
    let output = zstd::decode_all(frame.as_slice()).unwrap();
    let encoder = json!({"output":hex(&output),"isolated":output==b"second job"});

    let first = zstd::encode_all(&b"first job"[..], 1).unwrap();
    let second = zstd::encode_all(&b"second job"[..], 1).unwrap();
    let mut dctx = zstd::zstd_safe::DCtx::create();
    zstd::stream::read::Decoder::with_context(first.as_slice(), &mut dctx)
        .read_exact(&mut [0u8; 5])
        .unwrap();
    let mut output = Vec::new();
    zstd::stream::read::Decoder::with_context(second.as_slice(), &mut dctx)
        .read_to_end(&mut output)
        .unwrap();
    let decoder = json!({"output":hex(&output),"isolated":output==b"second job"});
    println!(
        "HC074_ZSTD_CONTEXT:{}",
        json!({"encoder":encoder,"decoder":decoder})
    );
}
