//! Declared source compatibility only; no runtime enrollment or execution permit.
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
#[path = "../../../../tools/authority-runtime-074/cost_reader.rs"]
pub(crate) mod cost_reader;
#[path = "../../../../tools/authority-runtime-074/history_reader.rs"]
pub(crate) mod history_reader;

pub(crate) const SOURCE: &str = "33c54b0cda8c330b115009cdf092ad002077cedf";
pub(crate) const REVIEWED: &str = "b571bef2bcd2ceb3ed044c9e31bce4f670cfafdd";
const RAW: &str = include_str!("current_tree_input_rules_074.json");
fn digest(raw: &str) -> String {
    Sha256::digest(raw.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn rules() -> &'static Value {
    static RULES: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    RULES.get_or_init(|| {
        let raw = RAW.replace("\r\n", "\n");
        assert_eq!(
            digest(&raw),
            "85ab712a7b5e11d5430749108df9a013887eb027f8d9ce8683b4676f1b5f83fa"
        );
        let value: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(value["schema"], "hc074-current-tree-test-input-v1");
        assert_eq!(value["source"], SOURCE);
        assert_eq!(value["reviewed"], REVIEWED);
        assert_eq!(value["purpose"], "test-source-compatibility-only");
        assert_eq!(value["execution_admitted"], false);
        assert_eq!(value["patches"].as_object().unwrap().len(), 9);
        assert_eq!(value["additions"].as_object().unwrap().len(), 81);
        value
    })
}
pub(crate) fn patches() -> &'static Map<String, Value> {
    rules()["patches"].as_object().unwrap()
}
pub(crate) fn additions() -> &'static Map<String, Value> {
    rules()["additions"].as_object().unwrap()
}
fn expected_patch(path: &str) -> Option<String> {
    let rule = patches().get(path)?;
    let mut raw = crate::runtime::frozen(SOURCE, path);
    assert_eq!(digest(&raw), rule["source_sha256"].as_str().unwrap());
    for edit in rule["edits"].as_array().unwrap() {
        let before = edit["before"].as_str().unwrap();
        let after = edit["after"].as_str().unwrap();
        assert!(!before.is_empty() && before != after);
        assert_eq!(
            raw.matches(before).count(),
            1,
            "exact current input patch {path}"
        );
        raw = raw.replacen(before, after, 1);
    }
    assert_eq!(digest(&raw), rule["current_sha256"].as_str().unwrap());
    assert_eq!(raw, crate::runtime::frozen(REVIEWED, path));
    Some(raw)
}
pub(crate) fn accepts_patch(path: &str, raw: &str) -> bool {
    expected_patch(path).is_some_and(|expected| raw.replace("\r\n", "\n") == expected)
}
pub(crate) fn read(path: &str) -> String {
    if cost_reader::FILES.contains(&path) {
        let raw = std::fs::read_to_string(crate::runtime::root().join(path)).unwrap();
        return cost_reader::previous(path, &raw);
    }
    if path == crate::runtime::HISTORY_TEST {
        let raw = std::fs::read_to_string(crate::runtime::root().join(path)).unwrap();
        return crate::runtime::view(path, history_reader::previous(&raw));
    }
    if patches().contains_key(path) {
        let raw = std::fs::read_to_string(crate::runtime::root().join(path)).unwrap();
        assert!(
            accepts_patch(path, &raw),
            "unreviewed historical input {path}"
        );
        return crate::runtime::frozen(SOURCE, path);
    }
    crate::runtime_io::read_to_string(crate::runtime::root().join(path)).unwrap()
}
pub(crate) fn known_addition(path: &str) -> bool {
    additions().contains_key(path)
}
pub(crate) fn accepts_addition(path: &str, raw: &str) -> bool {
    let Some(rule) = additions().get(path) else {
        return false;
    };
    let raw = raw.replace("\r\n", "\n");
    rule["mode"] == "100644"
        && rule["bytes"].as_u64() == Some(raw.len() as u64)
        && rule["sha256"].as_str() == Some(digest(&raw).as_str())
        && raw == crate::runtime::frozen(REVIEWED, path)
}
pub(crate) fn addition_index_matches(path: &str, raw: &str) -> bool {
    let Some(rule) = additions().get(path) else {
        return false;
    };
    raw == format!(
        "{} {} 0\t{path}\n",
        rule["mode"].as_str().unwrap(),
        rule["blob"].as_str().unwrap()
    )
}
pub(crate) fn require_addition(path: &str) {
    assert!(known_addition(path));
    let file = crate::runtime::root().join(path);
    let metadata = std::fs::symlink_metadata(&file).unwrap();
    assert!(metadata.is_file() && !metadata.file_type().is_symlink());
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        assert_eq!(metadata.file_attributes() & 0x400, 0);
    }
    let output = std::process::Command::new("git")
        .arg("--no-replace-objects")
        .args(["ls-files", "--stage", "--", path])
        .current_dir(crate::runtime::root())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(addition_index_matches(
        path,
        &String::from_utf8(output.stdout).unwrap()
    ));
    assert!(
        accepts_addition(path, &std::fs::read_to_string(file).unwrap()),
        "unreviewed new source {path}"
    );
}
