//! Exact test-only additions; never runtime admission or production source masking.
use sha2::{Digest, Sha256};
use std::path::Path;
#[path = "../material-lock-fixture-074/contract.rs"]
mod material_lock;
pub const BASE: &str = "25faa7bb762aa229631e6f6cfc25cae33e16ff43";
pub const PATHS: [&str; 6] = [
    "tools/authority-runtime-074/io.rs",
    "tools/long-run-supervisor-074/tests/held_launcher.rs",
    "tools/long-run-supervisor-074/src/diagnostic_validation_trace.rs",
    "tools/long-run-supervisor-074/src/diagnostic_cgroup_root_transport_tests.rs",
    "tools/long-run-supervisor-074/src/diagnostic_held_unit_material_kernel_tests.rs",
    material_lock::SOURCE,
];
const BASE_HASHES: [&str; 6] = [
    "1b4e852c40018bf6023d9e8e965e5debebc65b4689d556e2760b41c102952701",
    "70ca8141f1664f6e59664bbf02bf0695ddb48f69109d5e5551883c60e2048f81",
    "a3a5fd83f5f1c79945a8282adaac1b895f5f89d63a952365c538c15858bc1be2",
    "33732600eb4fa3c8e436a68caa3a24d2741c4daac92d7e6965b1282873b45554",
    "b603700b10f2284777c89108e973a1720c9896b0668cf2b17c40f3c2becd7b1e",
    "ede23782d4ff6a5ae052ca542998c3ebd8968af2ad678f67b179af4fdb05d828",
];
pub const HELPERS: [(&str, &str); 2] = [
    (
        "tools/owned-case-074/library.rs",
        "85f5a8476c9d69aa17566d4239ecd277b03a6f06a65689357e96caf3d0af47ef",
    ),
    (
        "tools/owned-case-074/controls.rs",
        "047f2c59eb745c986645485ebbbec27958ccd6b2ca1974966e04359a420071e3",
    ),
];
pub const CONTROLS: &str =
    "\n#[path = \"../../owned-case-074/controls.rs\"]\nmod owned_case_controls;\n";
const IMPORT: &str =
    "#[path = \"../owned-case-074/contract.rs\"]\npub(crate) mod owned_case_view;\n";
const BRANCH: &str = "    if owned_case_view::PATHS.contains(&relative.as_str()) {\n        owned_case_view::require_helpers(&super::runtime::root());\n        let original = super::runtime::frozen(owned_case_view::BASE, &relative);\n        assert!(owned_case_view::accepts(&relative, &original, &raw), \"unreviewed owned test scope\");\n        raw = original;\n    }\n";
fn digest(raw: &str) -> String {
    Sha256::digest(raw.replace("\r\n", "\n").as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
pub fn helper_matches(path: &str, raw: &str) -> bool {
    HELPERS
        .iter()
        .find(|(file, _)| *file == path)
        .is_some_and(|(_, expected)| digest(raw) == *expected)
}
pub fn require_helpers(root: &Path) {
    for (path, _) in HELPERS {
        assert!(
            helper_matches(path, &std::fs::read_to_string(root.join(path)).unwrap()),
            "unreviewed test helper {path}"
        );
    }
    assert!(
        material_lock::helper_matches(
            &std::fs::read_to_string(root.join(material_lock::CONTROL)).unwrap()
        ),
        "unreviewed synthetic material control"
    );
}
fn replace(raw: &str, before: &str, after: &str) -> String {
    assert_eq!(raw.matches(before).count(), 1, "one exact owned-case edit");
    raw.replacen(before, after, 1)
}
pub fn candidate(path: &str, original: &str) -> Option<String> {
    let index = PATHS.iter().position(|file| *file == path)?;
    assert_eq!(
        digest(original),
        BASE_HASHES[index],
        "exact owned-case predecessor"
    );
    let raw = original.replace("\r\n", "\n");
    Some(if path == PATHS[0] {
        let declaration = "#[path = \"../historical-input-view-074/contract.rs\"]\npub(crate) mod reviewed_inputs;\n";
        let raw = replace(&raw, declaration, &format!("{IMPORT}{declaration}"));
        let raw = replace(
            &raw,
            "    let raw = std::fs::read_to_string(path)?",
            "    let mut raw = std::fs::read_to_string(path)?",
        );
        replace(
            &raw,
            "    if reviewed_inputs::PATHS.contains",
            &format!("{BRANCH}    if reviewed_inputs::PATHS.contains"),
        )
    } else if path == PATHS[1] {
        format!("{raw}{CONTROLS}")
    } else if path == PATHS[2] {
        format!("{raw}\n#[cfg(target_os = \"linux\")]\n#[path = \"../../owned-case-074/library.rs\"]\npub(crate) mod owned_case;\n")
    } else if path == material_lock::SOURCE {
        assert_eq!(BASE, material_lock::BASE, "same exact fixture predecessor");
        material_lock::candidate(&raw)
    } else {
        let name = if path == PATHS[3] {
            "cgroup_root_transport_every_bound_byte_refuses_and_closes_rights"
        } else {
            "held_material_kernel_drop_closes_all_six_resources_and_partial_creation_is_raii"
        };
        let signature = format!("fn {name}() {{");
        replace(&raw, &signature, &format!("{signature}\n    if crate::diagnostic_ipc::validation_trace::owned_case::isolate(module_path!(), \"{name}\",) {{ return; }}"))
    })
}
pub fn accepts(path: &str, original: &str, actual: &str) -> bool {
    candidate(path, original).is_some_and(|expected| {
        syn::parse_file(actual).is_ok()
            && expected
                .parse::<proc_macro2::TokenStream>()
                .unwrap()
                .to_string()
                == actual
                    .parse::<proc_macro2::TokenStream>()
                    .map(|tokens| tokens.to_string())
                    .unwrap_or_default()
    })
}
