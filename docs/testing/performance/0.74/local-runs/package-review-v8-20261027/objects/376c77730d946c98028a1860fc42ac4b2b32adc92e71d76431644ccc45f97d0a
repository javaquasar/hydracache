//! Exact approved-roots predecessor for source tests, never runtime trust admission.
use serde::Deserialize;
use sha2::{Digest, Sha256};

pub const PATH: &str = "Cargo.lock";
pub const PREVIOUS: &str = "ba9c5535311ffd0e6759506898077dbc6a56830a";
pub const PRODUCER: &str = "63cb4d7b9bca1c901b2f7420f86effd3309a6882";
const RULES: &[u8] = include_bytes!("rules.json");
const RULES_SHA: &str = "d7ec7bf6240b74130719ed8be061f744c783710ecb5486c1de2ad86492544ba5";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Rules {
    schema: String,
    path: String,
    previous_revision: String,
    producer_revision: String,
    current_sha256: String,
    previous_sha256: String,
    patches: Vec<Patch>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Patch {
    offset: usize,
    current: String,
    previous: String,
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn rules() -> &'static Rules {
    static VALUE: std::sync::OnceLock<Rules> = std::sync::OnceLock::new();
    VALUE.get_or_init(|| {
        assert_eq!(
            digest(RULES),
            RULES_SHA,
            "exact compiled roots inverse edits"
        );
        let value: Rules = serde_json::from_slice(RULES).unwrap();
        assert_eq!(value.schema, "hc074-approved-roots-lock-view-v1");
        assert_eq!(value.path, PATH);
        assert_eq!(value.previous_revision, PREVIOUS);
        assert_eq!(value.producer_revision, PRODUCER);
        assert_eq!(value.patches.len(), 6);
        value
    })
}

pub fn project(path: &str, raw: String) -> String {
    if path != PATH {
        return raw;
    }
    let row = rules();
    let mut raw = raw.replace("\r\n", "\n");
    assert_eq!(
        digest(raw.as_bytes()),
        row.current_sha256,
        "unreviewed roots lock input"
    );
    let mut limit = raw.len();
    for patch in row.patches.iter().rev() {
        let end = patch.offset.checked_add(patch.current.len()).unwrap();
        assert!(end <= limit, "ordered roots inverse edits");
        assert_eq!(
            raw.get(patch.offset..end),
            Some(patch.current.as_str()),
            "exact roots inverse edit"
        );
        raw.replace_range(patch.offset..end, &patch.previous);
        limit = patch.offset;
    }
    assert_eq!(
        digest(raw.as_bytes()),
        row.previous_sha256,
        "complete pre-roots lock"
    );
    raw
}
