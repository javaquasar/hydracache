use hydracache::{
    CacheOptions, HydraCache, MemoryInstrumentationMode, MemorySnapshotRequest,
    RetainedByteEstimate, RetainedByteEstimateError,
};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::Duration;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}

fn corpus() -> Value {
    serde_json::from_slice(
        &std::fs::read(root().join("docs/testing/memory/0.71/retained-byte-estimator-v1.json"))
            .expect("golden corpus"),
    )
    .expect("valid golden corpus")
}

#[test]
fn golden_object_weight_corpus_matches_versioned_estimator() {
    if usize::BITS != 64 {
        return;
    }
    let corpus = corpus();
    assert_eq!(corpus["policy_effect"], "reporting-only");
    assert_eq!(corpus["legacy_moka_weigher"], "encoded-value-bytes");
    let schema: Value = serde_json::from_slice(
        &std::fs::read(
            root().join("docs/testing/memory/0.71/retained-byte-estimate-v1.schema.json"),
        )
        .expect("estimate schema"),
    )
    .expect("valid estimate schema");
    let validator = jsonschema::validator_for(&schema).expect("compiled estimate schema");

    for case in corpus["cases"].as_array().expect("cases") {
        let key = "k".repeat(case["key_length"].as_u64().expect("key length") as usize);
        let tags = case["tag_lengths"]
            .as_array()
            .expect("tag lengths")
            .iter()
            .map(|length| "t".repeat(length.as_u64().expect("tag length") as usize))
            .collect::<Vec<_>>();
        let estimate = RetainedByteEstimate::for_entry(
            &key,
            case["value_bytes"].as_u64().expect("value bytes") as usize,
            &tags,
            case["expires"].as_bool().expect("expires"),
        )
        .expect("representable estimate");
        assert_eq!(
            estimate.total_bytes,
            case["expected_total_bytes"].as_u64().expect("total"),
            "{}",
            case["id"]
        );
        validator
            .validate(&serde_json::to_value(estimate).expect("serialized estimate"))
            .unwrap_or_else(|error| panic!("{}: {error}", case["id"]));
    }
}

#[test]
fn moka_adapter_rejects_unrepresentable_retained_weight() {
    let estimate = RetainedByteEstimate {
        schema_version: "hydracache-retained-byte-estimate-v1",
        encoded_key_bytes: 0,
        value_bytes: 0,
        cache_entry_bytes: 0,
        primary_index_bytes: 0,
        tag_vector_bytes: 0,
        tag_string_bytes: 0,
        tag_index_bytes: 0,
        expiry_bytes: 0,
        total_bytes: u64::from(u32::MAX) + 1,
    };
    assert_eq!(
        estimate.try_moka_weight(),
        Err(RetainedByteEstimateError::MokaWeightUnrepresentable {
            total_bytes: u64::from(u32::MAX) + 1
        })
    );
}

#[tokio::test]
async fn insert_replace_delete_and_flush_reconcile_retained_totals() {
    let cache = HydraCache::local()
        .memory_instrumentation_mode(MemoryInstrumentationMode::Production)
        .max_capacity(1024 * 1024)
        .build();
    cache
        .put(
            "accounted",
            vec![7_u8; 64],
            CacheOptions::new()
                .tags(["one", "two"])
                .ttl(Duration::from_secs(60)),
        )
        .await
        .expect("insert");
    cache.diagnostics().await;
    let inserted = exact_snapshot(&cache).await;
    let inserted_estimate = RetainedByteEstimate::for_entry(
        "accounted",
        inserted.logical_value_bytes as usize,
        &["one".to_owned(), "two".to_owned()],
        true,
    )
    .expect("insert estimate");
    assert_eq!(
        inserted.estimated_retained_bytes,
        inserted_estimate.total_bytes
    );
    let reconciliation = cache
        .reconcile_memory_footprint()
        .await
        .expect("insert reconciliation");
    assert_eq!(
        reconciliation.counter_estimated_retained_bytes,
        reconciliation.exact_estimated_retained_bytes
    );

    cache
        .put(
            "accounted",
            vec![9_u8; 512],
            CacheOptions::new().tag("replacement"),
        )
        .await
        .expect("replace");
    cache.diagnostics().await;
    let replaced = exact_snapshot(&cache).await;
    assert_eq!(replaced.live_entries, 1);
    assert!(replaced.estimated_retained_bytes > inserted.estimated_retained_bytes);
    cache
        .reconcile_memory_footprint()
        .await
        .expect("replacement reconciliation");

    cache.remove("accounted").await.expect("remove");
    cache.diagnostics().await;
    let removed = exact_snapshot(&cache).await;
    assert_eq!(removed.live_entries, 0);
    assert_eq!(removed.estimated_retained_bytes, 0);

    cache
        .put("flush", vec![1_u8; 32], CacheOptions::new())
        .await
        .expect("refill");
    cache.flush().await.expect("flush");
    cache.diagnostics().await;
    let flushed = exact_snapshot(&cache).await;
    assert_eq!(flushed.live_entries, 0);
    assert_eq!(flushed.estimated_retained_bytes, 0);
}

#[test]
fn w2a_does_not_reinterpret_legacy_capacity() {
    let builder = std::fs::read_to_string(root().join("crates/hydracache/src/builder.rs"))
        .expect("builder source");
    assert!(builder.contains("entry.value.len().min(max_entry_bytes).max(1) as u32"));
    assert!(!builder.contains("try_moka_weight()"));
}

async fn exact_snapshot(cache: &HydraCache) -> hydracache::MemoryFootprintSnapshot {
    let barrier = cache.memory_snapshot_barrier().expect("barrier");
    cache
        .memory_footprint_snapshot(MemorySnapshotRequest::Exact {
            acknowledged_epoch: barrier.epoch,
        })
        .await
        .expect("exact snapshot")
}
