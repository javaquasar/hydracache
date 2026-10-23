//! Exact current installation gate plus a verified predecessor view for old phases.
use crate::storage_contract as storage;
#[path = "performance.rs"]
pub mod performance;
#[path = "../status-field-gate-074/contract.rs"]
pub mod status_fields;
use std::path::{Path, PathBuf};

pub const BASE: &str = storage::SOURCE;
pub const ARTIFACT: &str = "f193fabae86c9bd7a2e778a9463a43984834deb5";
pub const HISTORY: &str = "fe5ebda429d9ef1f3a9ccd059600d264936cc283";
pub const FIXTURE: &str = storage::FIXTURE;
pub const OBSERVER: &str = storage::OBSERVER;
pub const TESTS: &str = "tools/long-run-supervisor-074/src/diagnostic_authority_fixture_tests.rs";
pub const SOURCE_TEST: &str = "crates/xtask/tests/authority_storage_source_074.rs";
pub const HISTORY_TEST: &str = "crates/xtask/tests/authority_rust_history_074.rs";
pub const COMPAT: [&str; 5] = [
    "authority_breakdown_controls.rs",
    "authority_leaf_name_controls.rs",
    "authority_leaf_name_alloc.rs",
    "authority_open_name_controls.rs",
    "authority_open_name_alloc.rs",
];
const IMPORT: &str = "#[path = \"../../../tools/authority-runtime-074/gate.rs\"]\nmod runtime;\n";
pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
pub fn frozen(revision: &str, path: &str) -> String {
    let output = std::process::Command::new("git")
        .arg("--no-replace-objects")
        .args(["show", &format!("{revision}:{path}")])
        .current_dir(root())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "missing exact runtime gate predecessor {path}"
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .replace("\r\n", "\n")
}
fn patch(source: &str, old: &str, new: &str, count: usize) -> String {
    assert_eq!(
        source.matches(old).count(),
        count,
        "exact runtime compatibility edit {old}"
    );
    source.replace(old, new)
}
pub fn legacy_adapter(path: &str, original: &str) -> String {
    let body = if path == SOURCE_TEST {
        "    std::fs::read_to_string(root().join(path))\n        .unwrap()\n        .replace('\\r', \"\")"
    } else {
        assert_eq!(path, HISTORY_TEST);
        "    std::fs::read_to_string(history::root().join(path))\n        .unwrap()\n        .replace(\"\\r\\n\", \"\\n\")"
    };
    let new = format!("    runtime::view(path, {{\n{body}\n    }})");
    let mut output = patch(original, body, &new, 1);
    let at = output.find("\nuse ").unwrap() + 1;
    let imports = if path == SOURCE_TEST {
        format!("{IMPORT}use contract as storage_contract;\n")
    } else {
        format!("{IMPORT}#[path = \"../../../tools/authority-storage-source-074/contract.rs\"]\nmod storage_contract;\n")
    };
    output.insert_str(at, &imports);
    output
}
pub fn expected(path: &str) -> Option<String> {
    if path == status_fields::SOURCE {
        return Some(status_fields::candidate(&frozen(BASE, path)));
    }
    if path == status_fields::LEGACY_TEST {
        return Some(status_fields::legacy_test(&frozen(BASE, path)));
    }
    if path == performance::MAIN {
        return Some(format!("{}\n{}", performance::IMPORTS, frozen(BASE, path)));
    }
    if let Some((_, read)) = performance::READERS.iter().find(|(file, _)| *file == path) {
        return Some(performance::reader(&frozen(BASE, path), read));
    }
    if path == FIXTURE {
        return Some(storage::candidate(&frozen(BASE, path)));
    }
    if path == OBSERVER {
        return Some(storage::observer(&frozen(BASE, path)));
    }
    if path == SOURCE_TEST {
        return Some(legacy_adapter(path, &frozen(ARTIFACT, path)));
    }
    if path == HISTORY_TEST {
        return Some(legacy_adapter(path, &frozen(HISTORY, path)));
    }
    if path == TESTS {
        return Some(format!(
            "{}\n{}",
            frozen(BASE, path).trim_end(),
            include_str!("filesystem.rs").replace("\r\n", "\n")
        ));
    }
    let name = path.strip_prefix("tools/long-run-supervisor-074/tests/support/")?;
    if !COMPAT.contains(&name) {
        return None;
    }
    let original = frozen(BASE, path);
    Some(match name {
        "authority_breakdown_controls.rs" => patch(
            &original,
            "for entry in &self.registry.entries",
            "for entry in &mut self.registry.entries",
            2,
        ),
        "authority_leaf_name_controls.rs" => {
            let text = patch(
                &original,
                "        raw,\n        name,",
                "        raw,\n        name,\n        scratch: None,",
                1,
            );
            let text = patch(&text, "fn check(&self,", "fn check(&mut self,", 1);
            let text = patch(
                &text,
                "let value = LeafFixture::build",
                "let mut value = LeafFixture::build",
                1,
            );
            patch(
                &text,
                "let value = LeafFixture::new(case)",
                "let mut value = LeafFixture::new(case)",
                1,
            )
        }
        "authority_leaf_name_alloc.rs" => {
            let text = patch(
                &original,
                "let fixture = LeafFixture::new(case)",
                "let mut fixture = LeafFixture::new(case)",
                1,
            );
            patch(
                &text,
                "let run = |mode, iterations|",
                "let mut run = |mode, iterations|",
                1,
            )
        }
        "authority_open_name_controls.rs" => {
            let text = patch(&original, "fn leaf(&self,", "fn leaf(&mut self,", 1);
            patch(&text, ".as_ref()", ".as_mut()", 1)
        }
        "authority_open_name_alloc.rs" => patch(
            &original,
            "let fixture = Probe::new(case)",
            "let mut fixture = Probe::new(case)",
            1,
        ),
        _ => unreachable!(),
    })
}
pub fn accepts(path: &str, raw: &str) -> bool {
    if path == status_fields::SOURCE || path == status_fields::LEGACY_TEST {
        let helper = std::fs::read_to_string(root().join(status_fields::HELPER)).unwrap();
        if !status_fields::helper_is_pinned(&helper) {
            return false;
        }
    }
    expected(path).is_some_and(|expected| storage::accepts(&expected, raw))
}
// This is not permissive normalization: require the whole approved current file
// before returning an old phase's predecessor. Unknown paths retain current bytes.
pub fn view(path: &str, raw: String) -> String {
    if expected(path).is_none() {
        return raw;
    }
    assert!(
        accepts(path, &raw),
        "current runtime file refused before historical view: {path}"
    );
    frozen(
        if path == SOURCE_TEST {
            ARTIFACT
        } else if path == HISTORY_TEST {
            HISTORY
        } else {
            BASE
        },
        path,
    )
}
