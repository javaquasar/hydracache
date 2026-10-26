//! Retired test-source text only: this never compiles or re-enables the old selector.
use sha2::{Digest, Sha256};
pub const SOURCE: &str = "crates/xtask/tests/performance_contract_074.rs";
const RULES: &[u8] = include_bytes!("main-rules.json");
fn digest(raw: &[u8]) -> String {
    Sha256::digest(raw)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn rule() -> &'static serde_json::Value {
    static RULE: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();
    RULE.get_or_init(|| {
        assert_eq!(
            digest(RULES),
            "9373e36370e3893316a983fb159debba29afbbf4095c86f8f71d4384c80cc4f5"
        );
        let value: serde_json::Value = serde_json::from_slice(RULES).unwrap();
        assert_eq!(value["schema"], "hc074-source-successor-view-v1");
        assert_eq!(value["rows"].as_array().unwrap().len(), 1);
        assert_eq!(value["rows"][0]["path"], SOURCE);
        assert_eq!(
            value["rows"][0]["revision"],
            "a67a5299ab335424e9b11330dc06bd749ea9b859"
        );
        assert_eq!(
            value["rows"][0]["previous_revision"],
            "37db65f68e67ed7c0ee0f1cc84fed1c30bf9822a"
        );
        value
    })
}
pub fn project(raw: String) -> String {
    let row = &rule()["rows"][0];
    let mut raw = raw.replace("\r\n", "\n");
    assert_eq!(
        digest(raw.as_bytes()),
        row["current_sha256"].as_str().unwrap(),
        "current terminal test body"
    );
    let mut limit = raw.len();
    for edit in row["patches"].as_array().unwrap().iter().rev() {
        let at = usize::try_from(edit["offset"].as_u64().unwrap()).unwrap();
        let exact = edit["current"].as_str().unwrap();
        let end = at.checked_add(exact.len()).unwrap();
        assert!(end <= limit);
        assert_eq!(raw.get(at..end), Some(exact));
        raw.replace_range(at..end, edit["previous"].as_str().unwrap());
        limit = at;
    }
    assert_eq!(
        digest(raw.as_bytes()),
        row["previous_sha256"].as_str().unwrap(),
        "whole pre-terminal test body"
    );
    raw
}
