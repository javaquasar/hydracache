//! Qualified source assertion view for the one changed read-tail handoff only.
use sha2::{Digest, Sha256};
pub const PATH: &str = "tools/history-status-readers-074/inputs.rs";
fn digest(raw: &str) -> String {
    Sha256::digest(raw.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
pub fn project(path: &str, raw: String) -> String {
    if path != PATH {
        return raw;
    }
    let raw = raw.replace("\r\n", "\n");
    assert_eq!(
        digest(&raw),
        "f0b1e70f06838dbf336f4efe4805e7ba6da2b31a0f9b3e134bf525bf16901353",
        "exact reviewed current history facade"
    );
    let edge = "    current::view(path, raw)\n}";
    assert_eq!(raw.matches(edge).count(), 1);
    let previous = raw.replacen(
        edge,
        "    crate::runtime::view(path, contract::project(path, raw))\n}",
        1,
    );
    let tail = previous
        .split_once("pub(crate) fn read(path: &str) -> String")
        .unwrap()
        .1;
    assert_eq!(
        digest(tail),
        "38676083ac2939fab1c83d46436a2ac8c450c7c63dc3ed8d912f672c839c5089",
        "complete original read tail after the one qualified handoff"
    );
    previous
}
