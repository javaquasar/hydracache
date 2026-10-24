//! Exact synthetic fixture correction; no change to production verification.
use sha2::{Digest, Sha256};
pub const SOURCE: &str = "tools/long-run-supervisor-074/src/diagnostic_artifacts_linux.rs";
pub const BASE: &str = "25faa7bb762aa229631e6f6cfc25cae33e16ff43";
pub const CONTROL: &str = "tools/material-lock-fixture-074/controls.rs";
pub const INPUT: &str = "include_bytes!(\"../../../docs/testing/performance/0.74/local-runs/material-lock-frozen-input-20261024/objects/be46eaf8e97e507bda5e3f2b671d0d622e6a639ce1d4613769a4693723d62b19\")";
pub const HOOK: &str = "    include!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/../material-lock-fixture-074/controls.rs\"));\n\n";
fn digest(raw: &str) -> String {
    Sha256::digest(raw.replace("\r\n", "\n").as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
pub fn helper_matches(raw: &str) -> bool {
    digest(raw) == "f6ed1e6539ed69ec2445c1971ca6fafa4cb9126643f2c93da9b45b2b3c9da25f"
}
pub fn candidate(original: &str) -> String {
    assert_eq!(
        digest(original),
        "ede23782d4ff6a5ae052ca542998c3ebd8968af2ad678f67b179af4fdb05d828"
    );
    let original = original.replace("\r\n", "\n");
    let line = "include_bytes!(\"../../../Cargo.lock\")";
    assert_eq!(original.matches(line).count(), 1);
    let candidate = original.replacen(line, INPUT, 1);
    let test =
        "    #[test]\n    fn synthetic_material_origin_and_exact_execution_state_cannot_drift() {";
    assert_eq!(candidate.matches(test).count(), 1);
    candidate.replacen(test, &format!("{HOOK}{test}"), 1)
}
pub fn accepts(original: &str, actual: &str) -> bool {
    let expected = candidate(original);
    syn::parse_file(actual).is_ok()
        && expected
            .parse::<proc_macro2::TokenStream>()
            .unwrap()
            .to_string()
            == actual
                .parse::<proc_macro2::TokenStream>()
                .map(|tokens| tokens.to_string())
                .unwrap_or_default()
}
