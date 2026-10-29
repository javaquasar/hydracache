//! Fixed approved Tokio successor; no discovery, IO, Git, clock or live authority.
use super::{digest, Entry, SCOPE};
use std::collections::BTreeMap;

const RAW: &[u8] = include_bytes!("../../../../tools/inventory-545-view-074/rules.json");
const SHA256: &str = "1f41899edf172c4ae0a29f17a5fd8a1cee0fd8bf1047c5da5c4ea94b1dca962b";
const CHANGED: &str = "tools/long-run-supervisor-074/src/diagnostic_manager_channel.rs";
const ADDED: &str = "tools/long-run-supervisor-074/src/diagnostic_manager_alarm_tests.rs";

fn description() -> &'static serde_json::Value {
    static VALUE: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();
    VALUE.get_or_init(|| {
        assert_eq!(digest(RAW), SHA256);
        let row: serde_json::Value = serde_json::from_slice(RAW).unwrap();
        assert_eq!(row["schema"], "hc074-inventory545-phase-v1");
        assert_eq!(
            row["previous_revision"],
            "958b3135a15c00b9f6aad41522cef9d1f6608d3a"
        );
        assert_eq!(row["revision"], "2a4a771d48ed6ad54288fd15ea7c75b56f4bd80d");
        assert_eq!(row["previous_count"], 544);
        assert_eq!(row["current_count"], 545);
        assert!(row["removed"].as_array().unwrap().is_empty());
        assert_eq!(row["changed"].as_object().unwrap().len(), 1);
        assert_eq!(row["added"].as_object().unwrap().len(), 1);
        assert!(row["changed"].get(CHANGED).is_some());
        assert!(row["added"].get(ADDED).is_some());
        row
    })
}

fn entry(path: &str, row: &serde_json::Value) -> Entry {
    assert!(path.starts_with(&format!("{SCOPE}/")));
    assert!(path.ends_with(".rs"));
    assert_eq!(row["mode"], "100644");
    let blob = row["blob"].as_str().unwrap();
    let canonical = row["canonical_sha256"].as_str().unwrap();
    assert_eq!(blob.len(), 40);
    assert_eq!(canonical.len(), 64);
    assert!(blob
        .bytes()
        .chain(canonical.bytes())
        .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()));
    Entry {
        mode: "100644".into(),
        blob: blob.into(),
        digest: canonical.into(),
    }
}

pub(crate) fn predecessor_entries(current: &BTreeMap<String, Entry>) -> BTreeMap<String, Entry> {
    assert_eq!(current.len(), 545);
    let row = description();
    let mut old = current.clone();
    assert_eq!(old.remove(ADDED), Some(entry(ADDED, &row["added"][ADDED])));
    assert_eq!(
        old[CHANGED],
        entry(CHANGED, &row["changed"][CHANGED]["current"])
    );
    old.insert(
        CHANGED.into(),
        entry(CHANGED, &row["changed"][CHANGED]["previous"]),
    );
    assert_eq!(old.len(), 544);
    old
}

pub(super) fn extend(previous: BTreeMap<String, Entry>) -> BTreeMap<String, Entry> {
    assert_eq!(previous.len(), 544);
    let row = description();
    assert_eq!(
        previous[CHANGED],
        entry(CHANGED, &row["changed"][CHANGED]["previous"])
    );
    assert!(!previous.contains_key(ADDED));
    let mut current = previous.clone();
    current.insert(
        CHANGED.into(),
        entry(CHANGED, &row["changed"][CHANGED]["current"]),
    );
    assert!(current
        .insert(ADDED.into(), entry(ADDED, &row["added"][ADDED]))
        .is_none());
    assert_eq!(predecessor_entries(&current), previous);
    current
}

pub(super) fn predecessor_names(current: &str) -> String {
    assert_eq!(current.lines().count(), 545);
    assert_eq!(current.lines().filter(|name| *name == ADDED).count(), 1);
    let old: String = current
        .lines()
        .filter(|name| *name != ADDED)
        .map(|name| format!("{name}\n"))
        .collect();
    assert_eq!(old.lines().count(), 544);
    old
}
