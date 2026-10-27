//! One independently pinned Tokio test addition; every invocation observes afresh.
use super::contract::digest;
use std::path::Path;

pub const PATH: &str = "tools/long-run-supervisor-074/src/diagnostic_manager_alarm_tests.rs";
pub const PRODUCER: &str = "2a4a771d48ed6ad54288fd15ea7c75b56f4bd80d";

fn row() -> &'static serde_json::Value {
    static VALUE: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();
    let value = VALUE.get_or_init(|| {
        let raw = include_bytes!("../inventory-545-view-074/rules.json");
        assert_eq!(
            digest(raw),
            "1f41899edf172c4ae0a29f17a5fd8a1cee0fd8bf1047c5da5c4ea94b1dca962b"
        );
        let value: serde_json::Value = serde_json::from_slice(raw).unwrap();
        assert_eq!(value["schema"], "hc074-inventory545-phase-v1");
        assert_eq!(value["revision"], PRODUCER);
        assert_eq!(value["previous_count"], 544);
        assert_eq!(value["current_count"], 545);
        assert_eq!(value["added"].as_object().unwrap().len(), 1);
        assert_eq!(value["changed"].as_object().unwrap().len(), 1);
        value
    });
    &value["added"][PATH]
}

pub fn known(path: &str) -> bool {
    path == PATH
}

pub fn accepts(path: &str, raw: &str) -> bool {
    if !known(path) {
        return false;
    }
    let raw = raw.replace("\r\n", "\n");
    row()["mode"] == "100644"
        && row()["canonical_sha256"].as_str() == Some(digest(raw.as_bytes()).as_str())
        && raw == crate::runtime::frozen(PRODUCER, path)
}

pub fn index_matches(path: &str, raw: &str) -> bool {
    known(path)
        && raw
            == format!(
                "{} {} 0\t{path}\n",
                row()["mode"].as_str().unwrap(),
                row()["blob"].as_str().unwrap()
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
        "exact new alarm source index"
    );
    assert!(
        accepts(path, &std::fs::read_to_string(file).unwrap()),
        "exact new alarm source body"
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
