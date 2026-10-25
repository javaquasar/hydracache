//! Exact successor of the immutable 530-source census, not a discovery policy.
use super::{digest, Entry, SCOPE};
use std::collections::BTreeMap;

const RAW: &[u8] = include_bytes!("../../../../docs/testing/performance/0.74/local-runs/inventory544-provenance-20261026/objects/fec2195d38bd7784e9c584704b90d5851f5a5c228a900513aa11cf91b42fad8f");
const SHA256: &str = "fec2195d38bd7784e9c584704b90d5851f5a5c228a900513aa11cf91b42fad8f";

fn description() -> &'static serde_json::Value {
    static VALUE: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();
    VALUE.get_or_init(|| {
        assert_eq!(digest(RAW), SHA256);
        let value: serde_json::Value = serde_json::from_slice(RAW).unwrap();
        assert_eq!(value["schema"], "hc074-inventory544-phase-v1");
        assert_eq!(
            value["previous_revision"],
            "c91068bfa8b297bb075b63457da235aaa9c6232e"
        );
        assert_eq!(
            value["revision"],
            "958b3135a15c00b9f6aad41522cef9d1f6608d3a"
        );
        assert_eq!(
            value["previous_ledger_sha256"],
            "cfd5e7f1c153f5214dd71da196dfdbac2d1c05806fa14865605e9ce0776bc9c0"
        );
        assert_eq!(value["previous_count"], 530);
        assert_eq!(value["current_count"], 544);
        assert_eq!(value["added"].as_object().unwrap().len(), 14);
        assert_eq!(value["changed"].as_object().unwrap().len(), 3);
        assert!(value["removed"].as_array().unwrap().is_empty());
        value
    })
}

fn entry(name: &str, value: &serde_json::Value) -> Entry {
    assert!(name.starts_with(&format!("{SCOPE}/")) && name.ends_with(".rs"));
    assert!(!name.split('/').any(|part| matches!(part, "" | "." | "..")));
    assert_eq!(value["mode"], "100644");
    let blob = value["blob"].as_str().unwrap();
    let canonical = value["canonical_sha256"].as_str().unwrap();
    assert_eq!(blob.len(), 40);
    assert_eq!(canonical.len(), 64);
    assert!(blob
        .bytes()
        .chain(canonical.bytes())
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)));
    Entry {
        mode: "100644".into(),
        blob: blob.into(),
        digest: canonical.into(),
    }
}

pub(super) fn extend(previous: BTreeMap<String, Entry>) -> BTreeMap<String, Entry> {
    assert_eq!(previous.len(), 530);
    let value = description();
    let mut current = previous;
    let changed = value["changed"].as_object().unwrap();
    let expected = [
        "diagnostic_authority_fixed.rs",
        "diagnostic_authority_fixture.rs",
        "diagnostic_kernel_origin.rs",
    ]
    .map(|name| format!("{SCOPE}/{name}"));
    assert!(expected.iter().all(|name| changed.contains_key(name)));
    for (name, change) in changed {
        let old = entry(name, &change["previous"]);
        let actual = &current[name];
        assert_eq!(
            (&actual.mode, &actual.blob, &actual.digest),
            (&old.mode, &old.blob, &old.digest)
        );
        let new = entry(name, &change["current"]);
        assert_ne!(new.blob, old.blob);
        assert_ne!(new.digest, old.digest);
        current.insert(name.clone(), new);
    }
    for (name, value) in value["added"].as_object().unwrap() {
        assert!(current.insert(name.clone(), entry(name, value)).is_none());
    }
    assert_eq!(current.len(), 544);
    current
}

pub(super) fn predecessor_names(current: &str) -> String {
    let added = description()["added"].as_object().unwrap();
    current
        .lines()
        .filter(|name| !added.contains_key(*name))
        .map(|name| format!("{name}\n"))
        .collect()
}
