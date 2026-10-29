//! Exact caller edits only; no live or runtime source approval.
use sha2::{Digest, Sha256};
fn digest(raw: &[u8]) -> String {
    Sha256::digest(raw)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn rows() -> &'static serde_json::Value {
    static DATA: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        let raw = include_bytes!("rules.json");
        assert_eq!(
            digest(raw),
            "f780c166ff6e708eb2b09013075e414426bad8ae443f537a5ad0a08dcebfb3ce"
        );
        let data: serde_json::Value = serde_json::from_slice(raw).unwrap();
        assert_eq!(data["schema"], "hc074-bootstrap-doc-reader-v1");
        assert_eq!(data["baseline"], "181c077abdfbae5a8c4dfe67cee6f1c95519d529");
        assert_eq!(data["rows"].as_array().unwrap().len(), 11);
        data
    })
}
pub fn adapt(path: &str, old: &str) -> Option<String> {
    let row = rows()["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["path"] == path)?;
    let mut raw = old.replace("\r\n", "\n");
    assert_eq!(
        digest(raw.as_bytes()),
        row["before_sha256"].as_str().unwrap(),
        "fixed caller predecessor: {path}"
    );
    for edit in row["edits"].as_array().unwrap() {
        let before = edit["before"].as_str().unwrap();
        assert!(!before.is_empty());
        assert_eq!(
            raw.matches(before).count() as u64,
            edit["count"].as_u64().unwrap()
        );
        raw = raw.replace(before, edit["after"].as_str().unwrap());
    }
    raw.push_str(row["append"].as_str().unwrap());
    assert_eq!(
        digest(raw.as_bytes()),
        row["after_sha256"].as_str().unwrap(),
        "whole caller successor"
    );
    Some(raw)
}
