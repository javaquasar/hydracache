//! Exact source-test security predecessors, never a runtime admission API.
use serde::Deserialize;
use sha2::{Digest, Sha256};

pub const PATHS: [&str; 4] = [
    "Cargo.toml",
    "Cargo.lock",
    "tools/long-run-supervisor-074/src/diagnostic_manager_channel.rs",
    "crates/xtask/tests/performance_contract_074.rs",
];
pub const BASE: &str = "fa9e708868b22999d60c9ccd97b694662910a095";
const PRODUCER: &str = "2a4a771d48ed6ad54288fd15ea7c75b56f4bd80d";
const RULES: &[u8] = include_bytes!("rules.json");
const RULES_SHA: &str = "279a9a8c88039fd2224618c0f406d0d4c48eb3135c76560c1981b615c3dd9bc2";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Rules {
    schema: String,
    rows: Vec<Row>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Row {
    path: String,
    previous_revision: String,
    producer_revision: Option<String>,
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

fn digest(raw: &[u8]) -> String {
    Sha256::digest(raw)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn rules() -> &'static Rules {
    // Only immutable compiled rule data is cached; never a current observation.
    static VALUE: std::sync::OnceLock<Rules> = std::sync::OnceLock::new();
    VALUE.get_or_init(|| {
        assert_eq!(digest(RULES), RULES_SHA, "pinned security inverse edits");
        let value: Rules = serde_json::from_slice(RULES).unwrap();
        assert_eq!(value.schema, "hc074-security-input-view-v1");
        assert!(value.rows.iter().map(|row| row.path.as_str()).eq(PATHS));
        for (index, row) in value.rows.iter().enumerate() {
            assert_eq!(
                row.previous_revision,
                if index == 3 {
                    "33c54b0cda8c330b115009cdf092ad002077cedf"
                } else {
                    BASE
                }
            );
            assert_eq!(
                row.producer_revision.as_deref(),
                (index < 3).then_some(PRODUCER)
            );
            assert_eq!(row.patches.len(), [2, 6, 3, 6][index]);
        }
        value
    })
}

pub fn project(path: &str, raw: String) -> String {
    if !PATHS.contains(&path) {
        return raw;
    }
    let row = rules().rows.iter().find(|row| row.path == path).unwrap();
    let mut raw = raw.replace("\r\n", "\n");
    assert_eq!(
        digest(raw.as_bytes()),
        row.current_sha256,
        "unreviewed current security input: {path}"
    );
    let mut limit = raw.len();
    for patch in row.patches.iter().rev() {
        let end = patch.offset.checked_add(patch.current.len()).unwrap();
        assert!(end <= limit, "ordered nonoverlapping security edits");
        assert_eq!(
            raw.get(patch.offset..end),
            Some(patch.current.as_str()),
            "exact security inverse edit"
        );
        raw.replace_range(patch.offset..end, &patch.previous);
        limit = patch.offset;
    }
    assert_eq!(
        digest(raw.as_bytes()),
        row.previous_sha256,
        "complete security predecessor: {path}"
    );
    raw
}
