//! Exact malformed corpus and upstream context-reset controls, no bound repair.
use crate::serialize::deserialize_stream;
use serde_json::json;
use std::io::{Read, Write};

#[test]
fn malformed_streams_keep_decisions_and_cursor() {
    let malformed: [&[u8]; 7] = [
        &[],
        &[3],
        &[1],
        &[1, 3],
        &[1, 1, 0, 0x28],
        &[1, 4, 0, 0, 0, 0, 0, 0],
        &[2, 3, 0, 1, 0, 255, 0],
    ];
    let mut rows = Vec::new();
    for (id, bytes) in malformed.into_iter().enumerate() {
        let mut cursor = bytes;
        let result = deserialize_stream::<String>(&mut cursor);
        assert!(result.is_err());
        rows.push(json!({"id":id,"input":bytes,"accepted":false,"remaining":cursor.len(),"message":result.unwrap_err().to_string()}));
    }
    println!("HC074_SECURITY_MALFORMED:{}", json!({"rows":rows}));
}

#[test]
fn reused_contexts_start_only_the_second_job() {
    let mut cctx = zstd::zstd_safe::CCtx::create();
    zstd::stream::write::Encoder::with_context(Vec::new(), &mut cctx)
        .write_all(b"first job")
        .unwrap();
    let mut encoder = zstd::stream::write::Encoder::with_context(Vec::new(), &mut cctx);
    encoder.write_all(b"second job").unwrap();
    let encoded = encoder.finish().unwrap();
    assert_eq!(zstd::decode_all(encoded.as_slice()).unwrap(), b"second job");
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
    assert_eq!(output, b"second job");
}
