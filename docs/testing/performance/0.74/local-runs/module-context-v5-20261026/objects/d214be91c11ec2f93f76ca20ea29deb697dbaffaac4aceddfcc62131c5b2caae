//! Exact reviewed current source text; never compiled predecessor or runtime admission.
use sha2::{Digest, Sha256};
pub fn digest(raw: &[u8]) -> String {
    Sha256::digest(raw)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
pub fn rules() -> &'static serde_json::Value {
    static VALUE: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();
    VALUE.get_or_init(|| {
        let raw = include_bytes!("rules.json");
        assert_eq!(
            digest(raw),
            "4ef20240b953381c2e4dbfe7892b0a9205dbdefba3012e89a113030372d0d389"
        );
        let value: serde_json::Value = serde_json::from_slice(raw).unwrap();
        assert_eq!(value["schema"], "hc074-current-frontier-v1");
        assert_eq!(value["source"], "33c54b0cda8c330b115009cdf092ad002077cedf");
        assert_eq!(
            value["baseline"],
            "bf352a1dfbe3040af9ce058a5d42efe55202db62"
        );
        assert_eq!(value["rows"].as_array().unwrap().len(), 12);
        value
    })
}
pub fn known(path: &str) -> bool {
    rules()["rows"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["path"] == path)
}
pub fn project(path: &str, raw: String) -> String {
    let Some(row) = rules()["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["path"] == path)
    else {
        return raw;
    };
    let mut raw = raw.replace("\r\n", "\n");
    assert_eq!(
        digest(raw.as_bytes()),
        row["current_sha256"].as_str().unwrap()
    );
    let mut limit = raw.len();
    for edit in row["patches"].as_array().unwrap().iter().rev() {
        let at = usize::try_from(edit["offset"].as_u64().unwrap()).unwrap();
        let current = edit["current"].as_str().unwrap();
        let end = at.checked_add(current.len()).unwrap();
        assert!(end <= limit);
        assert_eq!(raw.get(at..end), Some(current));
        raw.replace_range(at..end, edit["previous"].as_str().unwrap());
        limit = at;
    }
    assert_eq!(
        digest(raw.as_bytes()),
        row["previous_sha256"].as_str().unwrap()
    );
    raw
}
