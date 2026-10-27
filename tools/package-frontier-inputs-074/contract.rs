//! Exact source-only joins after the original reader's complete physical checks.
//! No IO, Git, historical execution, live observation cache or execution authority.
use sha2::{Digest, Sha256};

pub const ROWS: [(&str, &str, &str, &str); 4] = [
    (
        "crates/xtask/tests/memory_compat_071.rs",
        "c14fa94916dbb5412e4d489b2234910b799e12a2",
        "7b0cd9d09b83e971fcfb71db8b9d83eb58043ff7f3561759b8f0cd23b66675e9",
        "e307c64b25827fefca627afc72ad54ae279258bcbcec1dc67018239a89384847",
    ),
    (
        "crates/xtask/tests/memory_profile_071.rs",
        "c14fa94916dbb5412e4d489b2234910b799e12a2",
        "ad3933474362336f192f8f55960efe316cb373b0ae72b6e16ca1a8369657d022",
        "6a8a0d68c61202e71833eae8d806a5ecc795b2593e2ef26c34fe0f117b9a2549",
    ),
    (
        "crates/xtask/tests/support/fresh_status_guard.rs",
        "7f4d185820e0c14a9fba4bb482f98e65df5172ce",
        "591042e3e621dba39c4cac01beb4f5d6eea6a6e9c9dd646245b045112246c705",
        "3b98eaed3badcb3cda57930e45cae7fcaf2e2bad56a0f6729547050346b4949d",
    ),
    (
        "crates/xtask/tests/support/pid_path_guard.rs",
        "7f4d185820e0c14a9fba4bb482f98e65df5172ce",
        "26b52510d206b94e4f358a9ca4febf66b65d44ee3690ce14f71b14d5a2865b30",
        "350c4602ad92893bf07a29ee3d054e08456e0f852bd61ac53aee8d1014a70497",
    ),
];

fn digest(raw: &str) -> String {
    Sha256::digest(raw.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn edit(raw: String, current: &str, previous: &str) -> String {
    assert_eq!(raw.matches(current).count(), 1, "one exact source join");
    raw.replacen(current, previous, 1)
}
pub fn known(path: &str) -> bool {
    ROWS.iter().any(|row| row.0 == path)
}
pub fn project(path: &str, raw: String) -> String {
    let Some((_, _, current_sha, previous_sha)) = ROWS.iter().find(|row| row.0 == path) else {
        return raw;
    };
    let mut raw = raw.replace("\r\n", "\n");
    assert_eq!(digest(&raw), *current_sha, "whole current input: {path}");
    if path == ROWS[0].0 || path == ROWS[1].0 {
        raw = edit(
            raw,
            "#[path = \"support/fixture_python_074.rs\"]\nmod fixture_python;\n\n",
            "",
        );
        raw = edit(
            raw,
            "Command::new(fixture_python::program())",
            "Command::new(\"python\")",
        );
    } else if path == ROWS[2].0 {
        raw = edit(
            raw,
            "pid_path::normalize_legacy(path, raw)",
            "pid_path::normalize(path, raw)",
        );
    } else {
        raw = edit(
            raw,
            "#[allow(dead_code)]\n#[path = \"pid_stage_join.rs\"]\npub(crate) mod stage_join;\n",
            "",
        );
        raw = edit(raw, "#[allow(dead_code)]\npub(crate) fn normalize(path: &str, raw: &str) -> String {\n    let raw = custody_cost::normalize(path, raw);\n    finish(path, stage_join::project(path, raw))\n}\n// Fresh-status and its successors already enter the named earlier origin stage.\n#[allow(dead_code)]\npub(crate) fn normalize_legacy(path: &str, raw: &str) -> String {\n    finish(path, custody_cost::normalize(path, raw))\n}\nfn finish(path: &str, mut raw: String) -> String {\n", "pub(crate) fn normalize(path: &str, raw: &str) -> String {\n    let mut raw = custody_cost::normalize(path, raw);\n");
    }
    assert_eq!(
        digest(&raw),
        *previous_sha,
        "whole named predecessor: {path}"
    );
    raw
}
