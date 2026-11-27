//! Built outside the workspace against the unpacked HC/2 SDK candidate.

use hydracache_client_hc2::{
    BatchItemState, BatchOperation, Capability, ClientConfig, GrpcMtlsAdapter, GrpcMtlsConfig,
    Hc2Client, RequestOptions,
};
use std::path::Path;
use std::time::Duration;

const PACKAGE_SHA256: &str = env!("W10I_RUST_ARTIFACT_SHA256");
const MAX_ITEMS: usize = 13;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let [action, root, endpoint, rows @ ..] = args.as_slice() else {
        panic!("expected action, credentials directory, endpoint and key/value pairs");
    };
    assert!(
        matches!(action.as_str(), "put" | "check"),
        "unknown cohort action"
    );
    assert!(
        rows.len().is_multiple_of(2) && (2..=MAX_ITEMS * 2).contains(&rows.len()),
        "invalid cohort size"
    );
    let root = Path::new(root);
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
    let entries: Vec<_> = rows
        .chunks_exact(2)
        .map(|pair| (decode_key(&pair[0]), &pair[1]))
        .collect();
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
        rows.len() / 2
    );
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
    fn decodes_the_physical_key_with_embedded_zero() {
        assert_eq!(super::decode_key("484302010000"), b"HC\x02\x01\x00\x00");
    }

    #[test]
    #[should_panic(expected = "invalid hexadecimal cohort key")]
    fn rejects_a_non_hexadecimal_pair() {
        super::decode_key("44xx");
    }
}
