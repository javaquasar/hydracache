//! One already committed diagnostic-document insertion; no schema or live change.
use sha2::{Digest, Sha256};

pub const PATH: &str = "docs/COMPAT.md";
const HEADER: &str = "# HydraCache Compatibility Register\n\n";
const ADDITIONS: &str = include_str!("diagnostic-additions.txt");

fn digest(raw: &str) -> String {
    Sha256::digest(raw.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub fn project(path: &str, raw: String) -> String {
    if path != PATH {
        return raw;
    }
    let mut raw = raw.replace("\r\n", "\n");
    assert_eq!(
        digest(&raw),
        "da3ea98f49625b4bf85a45f5b6b60fc70e4b58480b3249e1e44e0a2e5ec10089",
        "unreviewed diagnostic document successor"
    );
    let additions = ADDITIONS.replace("\r\n", "\n");
    assert_eq!(additions.len(), 2395);
    assert!(raw.starts_with(&format!("{HEADER}{additions}")));
    raw.replace_range(HEADER.len()..HEADER.len() + additions.len(), "");
    assert_eq!(
        digest(&raw),
        "8743a9ad50122f733571e8106a49d43ccc5e44c79e864470842e6d9b4e0c9780",
        "complete diagnostic document predecessor"
    );
    raw
}
