//! Identical private test overlay for both checksum-reviewed upstream fixtures.
use crate::serialize::{CompressedStreamWriter, deserialize_stream};
use serde_json::{Value, json};
use std::io::{Read, Write};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn payload(kind: usize, count: usize) -> Vec<String> {
    let mut seed = 0x0741028_u64;
    (0..count)
        .map(|i| match kind {
            0 => "same text ".repeat(i % 13 + 1),
            1 => (0..i % 151 + 1)
                .map(|_| {
                    seed ^= seed << 13;
                    seed ^= seed >> 7;
                    seed ^= seed << 17;
                    char::from(32 + (seed % 95) as u8)
                })
                .collect(),
            _ => "λ😀Ключ\n\0".repeat(i % 11 + 1),
        })
        .collect()
}

fn stream(id: String, threshold: u16, payload: Vec<String>) -> Value {
    let mut writer = CompressedStreamWriter::with_block_threshold(threshold);
    let mut upperbounds = Vec::new();
    for item in &payload {
        upperbounds.push(writer.serialized_len_upperbound_after(item.as_str()));
        writer.append(item.as_str());
    }
    let encoded = writer.finish();
    if let Some(bound) = upperbounds.last() {
        assert!(encoded.len() <= *bound);
    }
    let mut cursor = encoded.as_slice();
    let decoded: Vec<String> = deserialize_stream(&mut cursor).unwrap();
    assert_eq!(decoded, payload);
    assert!(cursor.is_empty());
    json!({"id":id,"threshold":threshold,"payload":payload,"wire":hex(&encoded),
           "upperbounds":upperbounds,"remaining":cursor.len()})
}

#[test]
fn snapshot() {
    let mut cases = Vec::new();
    for threshold in [16, 200, 1000, u16::MAX] {
        for count in [0, 1, 100] {
            for kind in 0..3 {
                cases.push(stream(
                    format!("{threshold}/{count}/{kind}"),
                    threshold,
                    payload(kind, count),
                ));
            }
        }
    }
    cases.push(stream(
        "max-string".to_owned(),
        1000,
        vec!["X".repeat(65533)],
    ));
    let malformed: [&[u8]; 7] = [
        &[],
        &[3],
        &[1],
        &[1, 3],
        &[1, 1, 0, 0x28],
        &[1, 4, 0, 0, 0, 0, 0, 0],
        &[2, 3, 0, 1, 0, 255, 0],
    ];
    let faults: Vec<Value> = malformed
        .iter()
        .enumerate()
        .map(|(i, bytes)| {
            let mut cursor = *bytes;
            let result = deserialize_stream::<String>(&mut cursor);
            assert!(result.is_err(), "malformed stream {i}");
            json!({"id":i,"input":hex(bytes),"accepted":result.is_ok(),
               "message":result.unwrap_err().to_string(),"remaining":cursor.len()})
        })
        .collect();

    // The upstream 0.14.3 regression examples, observed on both libraries.
    let mut cctx = zstd::zstd_safe::CCtx::create();
    zstd::stream::write::Encoder::with_context(Vec::new(), &mut cctx)
        .write_all(b"first job")
        .unwrap();
    let mut encoder = zstd::stream::write::Encoder::with_context(Vec::new(), &mut cctx);
    encoder.write_all(b"second job").unwrap();
    let frame = encoder.finish().unwrap();
    let output = zstd::decode_all(frame.as_slice()).unwrap();
    let encode = json!({"output":hex(&output),"isolated":output==b"second job"});

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
    let decode = json!({"output":hex(&output),"isolated":output==b"second job"});
    println!(
        "HC074_ZSTD_SNAPSHOT:{}",
        json!({"seed":0x0741028_u64,"cases":cases,
        "faults":faults,"encoder":encode,"decoder":decode,"native_version":zstd::zstd_safe::version_string()})
    );
}
