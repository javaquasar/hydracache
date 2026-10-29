//! Fourteen independently inventoried additions; no discovery or permissive fallback.
use super::contract::digest;
use std::path::Path;
pub const REVISION: &str = "958b3135a15c00b9f6aad41522cef9d1f6608d3a";
pub fn rows() -> &'static serde_json::Map<String, serde_json::Value> {
    static VALUE: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();
    let value = VALUE.get_or_init(|| {
        let raw = include_bytes!("../../docs/testing/performance/0.74/local-runs/inventory544-provenance-20261026/objects/fec2195d38bd7784e9c584704b90d5851f5a5c228a900513aa11cf91b42fad8f");
        assert_eq!(digest(raw), "fec2195d38bd7784e9c584704b90d5851f5a5c228a900513aa11cf91b42fad8f");
        let value: serde_json::Value = serde_json::from_slice(raw).unwrap();
        assert_eq!(value["schema"], "hc074-inventory544-phase-v1");
        assert_eq!(value["revision"], REVISION);
        assert_eq!(value["previous_revision"], "c91068bfa8b297bb075b63457da235aaa9c6232e");
        assert_eq!(value["added"].as_object().unwrap().len(), 14);
        value
    });
    value["added"].as_object().unwrap()
}
pub fn known(path: &str) -> bool {
    rows().contains_key(path)
}
pub fn accepts(path: &str, raw: &str) -> bool {
    let Some(row) = rows().get(path) else {
        return false;
    };
    let raw = raw.replace("\r\n", "\n");
    row["mode"] == "100644"
        && row["canonical_sha256"].as_str() == Some(digest(raw.as_bytes()).as_str())
        && raw == crate::runtime::frozen(REVISION, path)
}
pub fn index_matches(path: &str, index: &str) -> bool {
    let Some(row) = rows().get(path) else {
        return false;
    };
    index
        == format!(
            "{} {} 0\t{path}\n",
            row["mode"].as_str().unwrap(),
            row["blob"].as_str().unwrap()
        )
}
pub fn require_with(root: &Path, path: &str, observe_index: impl FnOnce() -> String) {
    assert!(known(path));
    let file = root.join(path);
    let metadata = std::fs::symlink_metadata(&file).unwrap();
    assert!(metadata.is_file() && !metadata.file_type().is_symlink());
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        assert_eq!(metadata.file_attributes() & 0x400, 0);
    }
    assert!(
        index_matches(path, &observe_index()),
        "unreviewed new source index: {path}"
    );
    assert!(
        accepts(path, &std::fs::read_to_string(file).unwrap()),
        "unreviewed new source body: {path}"
    );
}
pub fn require(path: &str) {
    require_with(&crate::runtime::root(), path, || {
        let output = std::process::Command::new("git")
            .arg("--no-replace-objects")
            .args(["ls-files", "--stage", "--", path])
            .current_dir(crate::runtime::root())
            .output()
            .unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap()
    });
}
