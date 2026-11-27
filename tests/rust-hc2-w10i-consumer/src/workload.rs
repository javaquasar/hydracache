use hydracache_client_hc2::{
    BatchItemOutcome, BatchItemState, BatchOperation, ErrorCode, GapReason, Hc2Client,
    RequestOptions, SubscriptionEvent,
};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::{decode_key, PACKAGE_SHA256};

pub(crate) async fn load(client: &Hc2Client, root: &Path, args: &[String]) {
    let [origin] = args else {
        panic!("load requires a shared time origin");
    };
    let origin: u128 = origin.parse().unwrap();
    let elapsed = || {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
            - origin
    };
    let plan = std::fs::read_to_string(root.join("rust-load.plan")).unwrap();
    let mut ledger = BufWriter::new(std::fs::File::create(root.join("rust-load.tsv")).unwrap());
    let mut rounds = 0;
    for (seq, pair) in plan.lines().enumerate() {
        if root.join("packaged-load.stop").exists() {
            break;
        }
        let (a, b) = pair.split_once('\t').expect("load key pair");
        let update = decode_key(a);
        let remove = decode_key(b);
        let compare = |expected: &[u8]| BatchOperation::CompareAndSet {
            key: update.clone().into(),
            expected: expected.to_vec().into(),
            replacement: b"updated".to_vec().into(),
            ttl: None,
        };
        let put = |key: &[u8]| BatchOperation::Put {
            key: key.to_vec().into(),
            value: b"seed".to_vec().into(),
            ttl: None,
        };
        let batch = vec![
            put(&update),
            compare(b"stale"),
            compare(b"seed"),
            BatchOperation::Get { key: update.into() },
            put(&remove),
            BatchOperation::Delete {
                key: remove.clone().into(),
            },
            BatchOperation::Get { key: remove.into() },
        ];
        writeln!(ledger, "submit\t{seq}\t{}\t{a}\t{b}", elapsed()).unwrap();
        ledger.flush().unwrap();
        let outcome = match client.batch_outcomes(batch, Some(options())).await {
            Ok(rows) => {
                assert_eq!(rows.len(), 7);
                rows.into_iter()
                    .enumerate()
                    .map(|(index, row)| encode(row, index))
                    .collect::<Vec<_>>()
                    .join("\t")
            }
            Err(error)
                if matches!(
                    error.code(),
                    ErrorCode::Unavailable | ErrorCode::DeadlineExceeded
                ) =>
            {
                "u\tu\tu\tu\tu\tu\tu".into()
            }
            Err(error) => panic!("unexpected Rust load failure: {error}"),
        };
        writeln!(ledger, "outcome\t{seq}\t{}\t{outcome}", elapsed()).unwrap();
        ledger.flush().unwrap();
        rounds += 1;
        if seq == 0 {
            std::fs::File::create(root.join("rust-load.ready")).unwrap();
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    assert!(
        root.join("packaged-load.stop").exists(),
        "Rust load exhausted its key plan"
    );
    ledger.get_ref().sync_all().unwrap();
    println!("W10i Rust package sha256={PACKAGE_SHA256} action=load rounds={rounds}");
}

fn encode(row: BatchItemOutcome, index: usize) -> String {
    assert_eq!(row.item_id as usize, index + 1);
    let Some(receipt) = row.receipt else {
        assert!(
            row.result.is_err_and(|error| matches!(
                error.code(),
                ErrorCode::Unavailable | ErrorCode::DeadlineExceeded
            )),
            "missing Rust load receipt"
        );
        return "u".into();
    };
    match receipt.state {
        BatchItemState::NotApplied => {
            assert!(row.result.is_err());
            "n".into()
        }
        BatchItemState::OutcomeUnknown => {
            assert!(row.result.is_err());
            "u".into()
        }
        BatchItemState::Confirmed => {
            let applied = receipt.applied.expect("confirmed Rust load item version");
            let result = row.result.expect("confirmed Rust load item result");
            let state = if matches!(index, 3 | 6) {
                assert!(result.mutation.is_none());
                match result.value.as_ref().map(|v| v.value.as_ref()) {
                    None => '0',
                    Some(b"seed") => 's',
                    Some(b"updated") => 'v',
                    _ => panic!("unexpected Rust load readback"),
                }
            } else if result.mutation.expect("Rust load mutation").applied {
                'a'
            } else {
                'r'
            };
            format!(
                "{state}:{}:{}:{}",
                receipt.partition_id, applied.epoch, applied.version
            )
        }
    }
}

pub(crate) async fn watch(client: &Hc2Client, root: &Path, args: &[String]) {
    let [label, key, watermark, previous, next, mode] = args else {
        panic!("invalid Rust subscriber arguments");
    };
    let key = decode_key(key);
    let watermark: u64 = watermark.parse().unwrap();
    let mut stream = client
        .subscribe_with_values(key.clone().into(), watermark, true)
        .await
        .unwrap();
    if watermark > 0 {
        let gap = tokio::time::timeout(Duration::from_secs(10), stream.next_delivery())
            .await
            .unwrap()
            .unwrap();
        assert!(
            matches!(gap.event, SubscriptionEvent::Gap { subscription_id, after_watermark }
                if subscription_id == stream.id() && after_watermark == watermark)
        );
        let detail = gap.gap.unwrap();
        assert_eq!(detail.reason, GapReason::ResumeUnbound);
        assert!(detail.partitions.is_empty());
    }
    let current = client
        .get(key.clone().into(), Some(options()))
        .await
        .unwrap();
    assert_eq!(
        current.as_ref().map(|entry| entry.value.as_ref()),
        (previous != "-").then_some(previous.as_bytes())
    );
    std::fs::File::create(root.join(format!("rust-{label}.ready"))).unwrap();
    let event = tokio::time::timeout(Duration::from_secs(10), stream.next())
        .await
        .unwrap()
        .unwrap();
    let SubscriptionEvent::Event(event) = event else {
        panic!("missing fresh Rust subscription event: {event:?}");
    };
    assert!(
        !event.removed
            && event.key.as_ref() == key.as_slice()
            && event.value.as_ref() == next.as_bytes()
            && event.watermark > watermark
    );
    let tmp = root.join(format!("rust-{label}.tmp"));
    std::fs::write(&tmp, event.watermark.to_string()).unwrap();
    std::fs::rename(tmp, root.join(format!("rust-{label}.seen"))).unwrap();
    if mode == "hold" {
        tokio::time::timeout(Duration::from_secs(55), async {
            while !root.join(format!("watch-{label}.release")).exists() {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("Rust subscription was not released after the fault");
    }
    stream.close();
    println!("W10i Rust package sha256={PACKAGE_SHA256} action=watch stage={label} resumed={watermark} seen={}", event.watermark);
}

fn options() -> RequestOptions {
    RequestOptions {
        timeout: Duration::from_secs(2),
        idempotency_key: Default::default(),
    }
}
