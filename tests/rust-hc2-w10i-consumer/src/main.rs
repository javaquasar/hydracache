//! Built outside the workspace against the unpacked HC/2 SDK candidate.

use hydracache_client_hc2::{
    BatchItemState, BatchOperation, Capability, ClientConfig, GrpcMtlsAdapter, GrpcMtlsConfig,
    Hc2Client, RequestOptions,
};
use std::io::Read;
use std::path::Path;
use std::time::Duration;

mod workload;

const PACKAGE_SHA256: &str = env!("W10I_RUST_ARTIFACT_SHA256");
const MAX_ITEMS: usize = 14;
const MAX_MANIFEST_BYTES: usize = 128 * 1024;

enum Expected {
    Mutation(bool),
    Value(Option<Vec<u8>>),
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let [action, root, endpoint, rows @ ..] = args.as_slice() else {
        panic!("expected action, credentials directory, endpoint and key/value pairs");
    };
    if matches!(action.as_str(), "load" | "watch") {
        let root = Path::new(root);
        let client = connect(root, endpoint, true).await;
        if action == "load" {
            workload::load(&client, root, rows).await;
        } else {
            workload::watch(&client, root, rows).await;
        }
        return;
    }
    assert!(
        matches!(action.as_str(), "put" | "check" | "check-file" | "mixed"),
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
            .map(|pair| (decode_key(&pair[0]), Some(pair[1].as_str())))
            .collect()
    };
    let (operations, checks): (Vec<_>, Vec<_>) = if action == "mixed" {
        mixed_batch(&entries)
    } else {
        entries
            .into_iter()
            .map(|(key, value)| {
                if action == "put" {
                    (
                        BatchOperation::Put {
                            key: key.into(),
                            value: value.unwrap().as_bytes().to_vec().into(),
                            ttl: None,
                        },
                        Expected::Mutation(true),
                    )
                } else {
                    (
                        BatchOperation::Get { key: key.into() },
                        Expected::Value(value.map(|value| value.as_bytes().to_vec())),
                    )
                }
            })
            .unzip()
    };
    let client = connect(root, endpoint, false).await;
    let options = RequestOptions {
        timeout: Duration::from_secs(4),
        idempotency_key: Default::default(),
    };
    let outcomes = client
        .batch_outcomes(operations, Some(options))
        .await
        .expect("execute Rust cohort batch");
    let count = checks.len();
    assert_eq!(outcomes.len(), count);
    for (index, (outcome, expected)) in outcomes.into_iter().zip(checks).enumerate() {
        assert_eq!(outcome.item_id as usize, index + 1);
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
        match expected {
            Expected::Mutation(applied) => assert!(
                result
                    .mutation
                    .is_some_and(|mutation| mutation.applied == applied),
                "wrong mutation outcome for Rust cohort item {index}"
            ),
            Expected::Value(expected) => assert!(
                result.mutation.is_none()
                    && result.value.as_ref().map(|value| value.value.as_ref())
                        == expected.as_deref(),
                "wrong value for Rust cohort item {index}"
            ),
        }
    }
    println!(
        "W10i Rust package sha256={PACKAGE_SHA256} action={action} confirmed={}",
        count
    );
}

async fn connect(root: &Path, endpoint: &str, subscribe: bool) -> Hc2Client {
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
    if subscribe {
        config.capabilities.push(Capability::Subscriptions);
    }
    Hc2Client::connect(&adapter, config)
        .await
        .expect("connect packaged Rust client")
}

fn mixed_batch(entries: &[(Vec<u8>, Option<&str>)]) -> (Vec<BatchOperation>, Vec<Expected>) {
    let [(update, Some(original)), (remove, Some(_))] = entries else {
        panic!("mixed batch requires one update and one removal key");
    };
    let replacement = format!("updated:{original}").into_bytes();
    let compare = |expected: &[u8]| BatchOperation::CompareAndSet {
        key: update.clone().into(),
        expected: expected.to_vec().into(),
        replacement: replacement.clone().into(),
        ttl: None,
    };
    let operations = vec![
        compare(b"intentionally-stale"),
        compare(original.as_bytes()),
        BatchOperation::Get {
            key: update.clone().into(),
        },
        BatchOperation::Delete {
            key: remove.clone().into(),
        },
        BatchOperation::Get {
            key: remove.clone().into(),
        },
    ];
    let checks = vec![
        Expected::Mutation(false),
        Expected::Mutation(true),
        Expected::Value(Some(replacement)),
        Expected::Mutation(true),
        Expected::Value(None),
    ];
    (operations, checks)
}

fn decode_manifest(input: &str) -> Vec<(Vec<u8>, Option<&str>)> {
    assert!(
        input.len() <= MAX_MANIFEST_BYTES,
        "readback manifest too large"
    );
    let rows: Vec<_> = input
        .lines()
        .map(|line| {
            let (key, expected) = line
                .split_once('\t')
                .expect("readback row lacks its expectation");
            let value = if expected == "absent" {
                None
            } else {
                Some(
                    expected
                        .strip_prefix("present\t")
                        .expect("invalid readback expectation"),
                )
            };
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
        let manifest = format!("4800\tpresent\tfirst\n42\tpresent\t{value}\n");
        assert_eq!(
            super::decode_manifest(&manifest),
            [
                (vec![0x48, 0], Some("first")),
                (vec![0x42], Some(value.as_str()))
            ]
        );
    }

    #[test]
    #[should_panic(expected = "readback row lacks its expectation")]
    fn rejects_a_manifest_row_without_a_value() {
        super::decode_manifest("4800\n");
    }

    #[test]
    fn distinguishes_a_tombstone_from_an_empty_value() {
        assert_eq!(
            super::decode_manifest("41\tabsent\n42\tpresent\t\n"),
            [(vec![0x41], None), (vec![0x42], Some(""))]
        );
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
