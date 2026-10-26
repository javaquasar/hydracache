//! Fixed whole-caller transitions only; no permission learned from observations.
use sha2::{Digest, Sha256};
pub const BASE: &str = "3e7e0aa343ed51f99651761f9bb7b0b10efd359a";
pub const FRESH: &str = "crates/xtask/tests/fresh_kernel_cost_074.rs";
pub fn digest(raw: &str) -> String {
    Sha256::digest(raw.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
pub fn rules() -> &'static serde_json::Value {
    static DATA: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        let raw = include_str!("rules.json");
        assert_eq!(
            digest(raw),
            "b8803a5f12460fa2278ffef033dc0738c4dc925a65a661f701ea09ae9bb6883e"
        );
        let value: serde_json::Value = serde_json::from_str(raw).unwrap();
        assert_eq!(value["schema"], "hc074-remaining-readers-v1");
        assert_eq!(value["baseline"], BASE);
        assert_eq!(value["rows"].as_array().unwrap().len(), 7);
        assert_eq!(value["rows"][0]["path"], FRESH);
        value
    })
}
pub fn current(path: &str, old: &str) -> Option<String> {
    let row = rules()["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["path"] == path)?;
    let mut current = old.replace("\r\n", "\n");
    assert_eq!(digest(&current), row["before_sha256"].as_str().unwrap());
    for edit in row["edits"].as_array().unwrap() {
        let before = edit["before"].as_str().unwrap();
        assert!(!before.is_empty());
        assert_eq!(
            current.matches(before).count() as u64,
            edit["count"].as_u64().unwrap()
        );
        current = current.replace(before, edit["after"].as_str().unwrap());
    }
    current.push_str(row["append"].as_str().unwrap());
    assert_eq!(digest(&current), row["after_sha256"].as_str().unwrap());
    Some(current)
}
pub fn previous(path: &str, raw: &str) -> Option<String> {
    if !rules()["rows"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["path"] == path)
    {
        return None;
    }
    let old = crate::runtime::frozen(BASE, path);
    (raw.replace("\r\n", "\n") == current(path, &old).unwrap()).then_some(old)
}
