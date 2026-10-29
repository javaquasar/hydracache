//! Test-only observer inside serialize; original gate remains unmodified.
use super::{CompressedStreamWriter, deserialize_stream};
use serde_json::{Value, json};

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

fn observe(id: String, threshold: u16, payload: Vec<String>) -> Value {
    let mut writer = CompressedStreamWriter::with_block_threshold(threshold);
    let mut bounds = Vec::new();
    for item in &payload {
        bounds.push(writer.serialized_len_upperbound_after(item.as_str()));
        writer.append(item.as_str());
    }
    let bytes = writer.finish();
    let mut cursor = bytes.as_slice();
    let decoded: Vec<String> = deserialize_stream(&mut cursor).unwrap();
    assert_eq!(decoded, payload);
    assert!(cursor.is_empty());
    let bound_held = bounds.last().is_none_or(|bound| bytes.len() <= *bound);
    json!({"id":id,"threshold":threshold,"payload":payload,"wire":hex(&bytes),
           "bounds":bounds,"bound_held":bound_held,"actual":bytes.len(),"remaining":cursor.len()})
}

fn cases() -> Vec<Value> {
    let mut rows = Vec::new();
    for threshold in [16, 200, 1000, u16::MAX] {
        for count in [0, 1, 100] {
            for kind in 0..3 {
                rows.push(observe(
                    format!("{threshold}/{count}/{kind}"),
                    threshold,
                    payload(kind, count),
                ));
            }
        }
    }
    rows.push(observe(
        "max-string".to_owned(),
        1000,
        vec!["X".repeat(65533)],
    ));
    rows
}

#[test]
fn minimal_bound_contract() {
    let row = observe("one-byte".to_owned(), 1, vec!["!".to_owned()]);
    println!("HC074_MIN_BOUND:{row}");
    assert!(
        row["bound_held"].as_bool().unwrap(),
        "original upperbound contract"
    );
}

#[test]
fn observe_streams() {
    println!(
        "HC074_STREAM_BOUNDS:{}",
        json!({"seed":0x0741028_u64,"cases":cases()})
    );
}

#[test]
fn all_stream_bounds_hold() {
    for row in cases() {
        assert!(
            row["bound_held"].as_bool().unwrap(),
            "upperbound for {}",
            row["id"]
        );
    }
}
