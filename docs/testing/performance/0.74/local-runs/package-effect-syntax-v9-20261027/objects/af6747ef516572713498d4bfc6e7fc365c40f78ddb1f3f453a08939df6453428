//! Exact older document fixtures for source assertions only, not the live reader.
#[path = "io.rs"]
pub mod actual;
pub use actual::{read_to_string, selected};
use sha2::{Digest, Sha256};
use std::{io, path::Path};

pub const PATH: &str = "docs/COMPAT.md";
pub const PRODUCER: &str = "d3256fc08b8da2df424d6da5bce99c0e168939b0";
const HEADER: &str = "# HydraCache Compatibility Register\n\n";
const ADDITIONS: &str = include_str!("../source-successor-view-074/diagnostic-additions.txt");
const CURRENT: &str = "da3ea98f49625b4bf85a45f5b6b60fc70e4b58480b3249e1e44e0a2e5ec10089";
const PREVIOUS: &str = "8743a9ad50122f733571e8106a49d43ccc5e44c79e864470842e6d9b4e0c9780";
pub const READER: &str = "crates/xtask/tests/authority_reader_phase_074.rs";
pub const READER_PRODUCER: &str = "5ec4dd647195c286299842c9faa71d3c2fa58efd";
fn digest(raw: &str) -> String {
    Sha256::digest(raw.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
pub fn assertion_input(path: &str, raw: String) -> String {
    if path == READER {
        let mut raw = raw.replace("\r\n", "\n");
        assert_eq!(
            digest(&raw),
            "58a8f0c7fc36df765b970ebf2f91015b077210a6f45a04441241f6d93f123f7d",
            "exact reader at {READER_PRODUCER}"
        );
        for edge in [
            "#[path = \"../../../tools/authority-reader-phase-074/current_main.rs\"]\nmod current_main;\n",
            "    let raw = if path == main_phase::SOURCE {\n        current_main::project(raw)\n    } else {\n        raw\n    };\n",
        ] {
            assert_eq!(raw.matches(edge).count(), 1);
            raw = raw.replacen(edge, "", 1);
        }
        assert_eq!(
            digest(&raw),
            "04b3d1c95ec0a5ecfedf89a880e01c9a4a9cd843bdbf5118ce8fcb2f1da248f4",
            "complete preceding reader"
        );
        return raw;
    }
    if path != PATH {
        return raw;
    }
    let raw = raw.replace("\r\n", "\n");
    assert_eq!(
        digest(&raw),
        CURRENT,
        "fixed current document at {PRODUCER}"
    );
    let additions = ADDITIONS.replace("\r\n", "\n");
    assert_eq!(additions.len(), 2395);
    let prefix = format!("{HEADER}{additions}");
    let previous = format!("{HEADER}{}", raw.strip_prefix(&prefix).unwrap());
    assert_eq!(digest(&previous), PREVIOUS, "complete old document fixture");
    previous
}
fn current_fixture(raw: String) -> String {
    let raw = raw.replace("\r\n", "\n");
    assert_eq!(
        digest(&raw),
        PREVIOUS,
        "only the exact old positive fixture"
    );
    let additions = ADDITIONS.replace("\r\n", "\n");
    assert_eq!(additions.len(), 2395);
    let current = format!("{HEADER}{additions}{}", raw.strip_prefix(HEADER).unwrap());
    assert_eq!(digest(&current), CURRENT, "complete current fixture");
    current
}
pub fn project(root: &Path, path: &Path, raw: String) -> String {
    let raw = if selected(root, path) == Some(PATH) {
        current_fixture(raw)
    } else {
        raw
    };
    actual::project(root, path, raw)
}
pub fn read_with(
    root: &Path,
    path: &Path,
    observe: impl FnOnce(&Path) -> io::Result<String>,
) -> io::Result<String> {
    actual::read_with(root, path, |path| {
        observe(path).map(|raw| {
            if selected(root, path) == Some(PATH) {
                current_fixture(raw)
            } else {
                raw
            }
        })
    })
}
