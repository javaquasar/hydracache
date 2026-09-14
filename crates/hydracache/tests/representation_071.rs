use std::path::{Path, PathBuf};

use hydracache::{HydraCache, RetainedByteEstimate};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}

fn assert_public_traits<T: Send + Sync + Unpin>() {}

#[test]
fn public_cache_and_estimator_preserve_required_auto_traits() {
    assert_public_traits::<HydraCache>();
    assert_public_traits::<RetainedByteEstimate>();
}

#[test]
fn estimator_distinguishes_key_tag_ttl_and_payload_shapes() {
    let base = RetainedByteEstimate::for_entry("k", 64, &[], false).expect("base");
    let large_key =
        RetainedByteEstimate::for_entry(&"k".repeat(128), 64, &[], false).expect("large key");
    let tagged =
        RetainedByteEstimate::for_entry("k", 64, &["alpha".to_owned(), "beta".to_owned()], false)
            .expect("tagged");
    let expiring = RetainedByteEstimate::for_entry("k", 64, &[], true).expect("expiring");
    let payload = RetainedByteEstimate::for_entry("k", 4_096, &[], false).expect("payload");

    assert!(large_key.total_bytes > base.total_bytes);
    assert!(tagged.total_bytes > base.total_bytes);
    assert!(expiring.total_bytes > base.total_bytes);
    assert!(payload.total_bytes > base.total_bytes);
}

#[test]
fn representation_change_remains_explicitly_deferred_without_d2_evidence() {
    let policy =
        std::fs::read_to_string(root().join("docs/testing/memory/0.71/release-policy.toml"))
            .expect("release policy");
    let row = policy
        .split("[[optional_work]]")
        .find(|row| row.contains("id = \"W5-W7\""))
        .expect("W5-W7 disposition");
    assert!(row.contains("disposition = \"deferred\""));
    assert!(row.contains("bounded and correct"));
}
