//! Verified test-only source census; not host provenance or execution admission.
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[path = "artifact_inventory_phase_544.rs"]
mod phase_544;

pub(crate) const SCOPE: &str = "tools/long-run-supervisor-074/src";
const LEDGER: &[u8] = include_bytes!("../../../../docs/testing/performance/0.74/local-runs/role-census-inventory-20261024/objects/cfd5e7f1c153f5214dd71da196dfdbac2d1c05806fa14865605e9ce0776bc9c0");
const DIGEST: &str = "cfd5e7f1c153f5214dd71da196dfdbac2d1c05806fa14865605e9ce0776bc9c0";
pub(crate) struct Entry {
    pub(crate) mode: String,
    pub(crate) blob: String,
    pub(crate) digest: String,
}
struct Inventory {
    entries: BTreeMap<String, Entry>,
    current: String,
    phase530: String,
    previous: String,
}
fn digest(raw: &[u8]) -> String {
    Sha256::digest(raw)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn ledger() -> &'static Inventory {
    static VALUE: std::sync::OnceLock<Inventory> = std::sync::OnceLock::new();
    VALUE.get_or_init(|| {
        assert_eq!(digest(LEDGER), DIGEST);
        let value: serde_json::Value = serde_json::from_slice(LEDGER).unwrap();
        assert_eq!(value["schema"], "hc074-artifact-inventory-observation-v1");
        assert_eq!(
            value["pinned_current_source"],
            "c91068bfa8b297bb075b63457da235aaa9c6232e"
        );
        let entries: BTreeMap<_, _> = value["current"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(name, entry)| {
                assert!(name.starts_with(&format!("{SCOPE}/")));
                assert!(!name.split('/').any(|part| part == ".."));
                assert_eq!(entry["mode"], "100644");
                (
                    name.clone(),
                    Entry {
                        mode: entry["mode"].as_str().unwrap().to_owned(),
                        blob: entry["blob"].as_str().unwrap().to_owned(),
                        digest: entry["canonical_sha256"].as_str().unwrap().to_owned(),
                    },
                )
            })
            .collect();
        assert_eq!(entries.len(), 530);
        let phase = &value["phases"]["446"];
        assert_eq!(
            phase["revision"],
            "6c6e47ee5dd3504d0be064b28af5a3e3ac087c9e"
        );
        assert_eq!(phase["added_count"], 84);
        assert_eq!(phase["changed_old_count"], 31);
        assert!(phase["removed"].as_array().unwrap().is_empty());
        let names = |previous: bool| {
            entries
                .keys()
                .filter(|name| !previous || phase["added"].get(*name).is_none())
                .map(|name| format!("{name}\n"))
                .collect::<String>()
        };
        let phase530 = names(false);
        let previous = names(true);
        assert_eq!(previous.lines().count(), 446);
        let entries = phase_544::extend(entries);
        let current: String = entries.keys().map(|name| format!("{name}\n")).collect();
        assert_eq!(phase_544::predecessor_names(&current), phase530);
        Inventory {
            entries,
            current,
            phase530,
            previous,
        }
    })
}
pub(crate) fn entries() -> &'static BTreeMap<String, Entry> {
    &ledger().entries
}
pub(crate) fn current() -> &'static str {
    &ledger().current
}
pub(crate) fn project_with(
    names: &str,
    observe: impl FnMut(&str) -> Result<String, &'static str>,
) -> Result<String, &'static str> {
    let phase530 = project_phase530_with(names, observe)?;
    if phase530 != ledger().phase530 {
        return Err("unreviewed predecessor inventory");
    }
    Ok(ledger().previous.clone())
}
pub(crate) fn project_phase530_with(
    names: &str,
    mut observe: impl FnMut(&str) -> Result<String, &'static str>,
) -> Result<String, &'static str> {
    if names != current() {
        return Err("unknown, missing, duplicate or reordered source inventory");
    }
    for (path, expected) in entries() {
        if observe(path)? != expected.digest {
            return Err("current source bytes drift");
        }
    }
    Ok(phase_544::predecessor_names(names))
}
pub(crate) fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn git_index() -> Result<String, &'static str> {
    let output = std::process::Command::new("git")
        .arg("--no-replace-objects")
        .args(["ls-files", "--stage", "--", SCOPE])
        .current_dir(root())
        .output()
        .map_err(|_| "index query failed")?;
    if !output.status.success() {
        return Err("index query refused");
    }
    String::from_utf8(output.stdout).map_err(|_| "non-UTF8 index")
}
pub(crate) fn expected_index() -> String {
    entries()
        .iter()
        .map(|(name, entry)| format!("{} {} 0\t{name}\n", entry.mode, entry.blob))
        .collect()
}
pub(crate) fn index_matches(raw: &str) -> bool {
    raw == expected_index()
}
fn reparse(metadata: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        let _ = metadata;
        false
    }
}
fn filesystem_paths(directory: &Path, paths: &mut Vec<String>) -> Result<(), &'static str> {
    for item in std::fs::read_dir(directory).map_err(|_| "source directory unreadable")? {
        let path = item.map_err(|_| "source entry unreadable")?.path();
        let metadata =
            std::fs::symlink_metadata(&path).map_err(|_| "source metadata unreadable")?;
        if reparse(&metadata) || metadata.file_type().is_symlink() {
            return Err("source alias refused");
        }
        if metadata.is_dir() {
            filesystem_paths(&path, paths)?;
        } else if metadata.is_file() {
            paths.push(
                path.strip_prefix(root())
                    .map_err(|_| "foreign source")?
                    .to_str()
                    .ok_or("non-UTF8 source")?
                    .replace('\\', "/"),
            );
        } else {
            return Err("nonregular source");
        }
    }
    Ok(())
}
pub(crate) fn project(names: &str) -> Result<String, &'static str> {
    if names != current() {
        return Err("source list drift");
    }
    if !index_matches(&git_index()?) {
        return Err("source index drift");
    }
    let mut paths = Vec::new();
    filesystem_paths(&root().join(SCOPE), &mut paths)?;
    paths.sort();
    if paths
        .iter()
        .map(|name| format!("{name}\n"))
        .collect::<String>()
        != names
    {
        return Err("source filesystem inventory drift");
    }
    project_with(names, |path| {
        let raw = std::fs::read(root().join(path)).map_err(|_| "source unreadable")?;
        let canonical = String::from_utf8(raw)
            .map_err(|_| "non-UTF8 source body")?
            .replace("\r\n", "\n");
        Ok(digest(canonical.as_bytes()))
    })
}
