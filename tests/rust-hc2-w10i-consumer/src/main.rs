//! Built outside the workspace against the unpacked HC/2 SDK candidate.

use hydracache_client_hc2::{
    BatchItemState, BatchOperation, Capability, ClientConfig, GrpcMtlsAdapter, GrpcMtlsConfig,
    Hc2Client, RequestOptions,
};
use std::io::Read;
use std::path::Path;
use std::time::Duration;

const PACKAGE_SHA256: &str = env!("W10I_RUST_ARTIFACT_SHA256");
const MAX_ITEMS: usize = 14;
const MAX_MANIFEST_BYTES: usize = 128 * 1024;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let [action, root, endpoint, rows @ ..] = args.as_slice() else {
        panic!("expected action, credentials directory, endpoint and key/value pairs");
    };
    assert!(
        matches!(action.as_str(), "put" | "check" | "check-file"),
        "unknown cohort action"
    );
    let root = Path::new(root);
    let mut manifest = String::new();
    let entries = if action == "check-file" {
        let [file] = rows else {
            panic!("check-file requires one manifest");
        };
        std::fs::File::open(root.join(file))
            .expect("open Rust readback manifest")
            .take((MAX_MANIFEST_BYTES + 1) as u64)
            .read_to_string(&mut manifest)
            .expect("read Rust readback manifest");
        decode_manifest(&manifest)
    } else {
        assert!(
            rows.len().is_multiple_of(2) && (2..=MAX_ITEMS * 2).contains(&rows.len()),
            "invalid cohort size"
        );
        rows.chunks_exact(2)
            .map(|pair| (decode_key(&pair[0]), pair[1].as_str()))
            .collect()
    };
    let count = entries.len();
    let read = |name| std::fs::read(root.join(name)).expect("read fixture credential");
    let adapter = GrpcMtlsAdapter::new(
        GrpcMtlsConfig::new(
            format!("https://{endpoint}"),
            "localhost",
            read("clients.pem"),
            read("java-client.pem"),
            read("java-client.key"),
        )
        .expect("configure packaged Rust transport"),
    );
    let mut config = ClientConfig::new("w10i-packaged-rust", "w10i");
    config.capabilities = vec![Capability::Data, Capability::Batch];
    let client = Hc2Client::connect(&adapter, config)
        .await
        .expect("connect packaged Rust client");
    let operations = entries
        .iter()
        .map(|(key, value)| {
            if action == "put" {
                BatchOperation::Put {
                    key: key.clone().into(),
                    value: value.as_bytes().to_vec().into(),
                    ttl: None,
                }
            } else {
                BatchOperation::Get {
                    key: key.clone().into(),
                }
            }
        })
        .collect();
    let options = RequestOptions {
        timeout: Duration::from_secs(4),
        idempotency_key: Default::default(),
    };
    let outcomes = client
        .batch_outcomes(operations, Some(options))
        .await
        .expect("execute Rust cohort batch");
    assert_eq!(outcomes.len(), entries.len());
    for (index, (outcome, (_, expected))) in outcomes.into_iter().zip(entries).enumerate() {
        let receipt = outcome.receipt.expect("distributed Rust batch receipt");
        assert_eq!(
            receipt.state,
            BatchItemState::Confirmed,
            "unconfirmed Rust cohort item {index}"
        );
        assert!(receipt
            .applied
            .is_some_and(|applied| applied.version > 0 && applied.epoch > 0));
        let result = outcome.result.expect("confirmed Rust batch outcome");
        if action == "put" {
            assert!(result.mutation.is_some_and(|mutation| mutation.applied));
        } else {
            let expected = expected.as_bytes();
            assert!(
                result
                    .value
                    .is_some_and(|value| value.value.as_ref() == expected),
                "wrong value for Rust cohort item {index}"
            );
        }
    }
    println!(
        "W10i Rust package sha256={PACKAGE_SHA256} action={action} confirmed={}",
        count
    );
}

fn decode_manifest(input: &str) -> Vec<(Vec<u8>, &str)> {
    assert!(
        input.len() <= MAX_MANIFEST_BYTES,
        "readback manifest too large"
    );
    let rows: Vec<_> = input
        .lines()
        .map(|line| {
            let (key, value) = line.split_once('\t').expect("readback row lacks its value");
            (decode_key(key), value)
        })
        .collect();
    assert!((1..=MAX_ITEMS).contains(&rows.len()), "invalid cohort size");
    rows
}

fn decode_key(key: &str) -> Vec<u8> {
    assert!(key.is_ascii() && key.len().is_multiple_of(2) && (2..=256).contains(&key.len()));
    (0..key.len())
        .step_by(2)
        .map(|index| {
            u8::from_str_radix(&key[index..index + 2], 16).expect("invalid hexadecimal cohort key")
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn decodes_a_manifest_with_a_full_snapshot_probe() {
        let value = "Z".repeat(96 * 1024);
        let manifest = format!("4800\tfirst\n42\t{value}\n");
        assert_eq!(
            super::decode_manifest(&manifest),
            [(vec![0x48, 0], "first"), (vec![0x42], value.as_str())]
        );
    }

    #[test]
    #[should_panic(expected = "readback row lacks its value")]
    fn rejects_a_manifest_row_without_a_value() {
        super::decode_manifest("4800\n");
    }

    #[test]
    fn decodes_the_physical_key_with_embedded_zero() {
        assert_eq!(super::decode_key("484302010000"), b"HC\x02\x01\x00\x00");
    }

    #[test]
    #[should_panic(expected = "invalid hexadecimal cohort key")]
    fn rejects_a_non_hexadecimal_pair() {
        super::decode_key("44xx");
    }
}
