#[path = "../../../tools/historical-source-cache-074/cache.rs"]
mod controls;
fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path))
        .unwrap()
        .replace("\r\n", "\n")
}
fn old(path: &str) -> String {
    let output = std::process::Command::new("git")
        .args(["--no-replace-objects", "show", &format!("11ad3be9:{path}")])
        .current_dir(root())
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout)
        .unwrap()
        .replace("\r\n", "\n")
}
fn same(expected: &str, actual: &str) -> bool {
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
#[test]
fn immutable_source_cache_changes_only_the_historical_loader_not_current_validation() {
    let path = "tools/authority-runtime-074/gate.rs";
    let before = old(path);
    let import =
        "#[path = \"../historical-source-cache-074/library.rs\"]\npub mod historical_cache;\n";
    let wrapper = "pub fn frozen(revision: &str, path: &str) -> String {\n    assert!(revision.len() == 40 && revision.bytes().all(|byte| byte.is_ascii_hexdigit()));\n    static CACHE: std::sync::OnceLock<historical_cache::Cache> = std::sync::OnceLock::new();\n    CACHE\n        .get_or_init(historical_cache::Cache::default)\n        .get(revision, path, || load_frozen(revision, path))\n}\nfn load_frozen(revision: &str, path: &str) -> String {";
    let signature = "pub fn frozen(revision: &str, path: &str) -> String {";
    assert_eq!(before.matches(signature).count(), 1);
    let expected = before
        .replacen(
            "use crate::storage_contract as storage;\n",
            &format!("use crate::storage_contract as storage;\n{import}"),
            1,
        )
        .replacen(signature, wrapper, 1);
    let actual = read(path);
    assert!(same(&expected, &actual));
    for (from, to) in [
        ("revision.len() == 40", "true"),
        (
            ".get(revision, path, || load_frozen(revision, path))",
            ".get(revision, path, || String::new())",
        ),
        (".arg(\"--no-replace-objects\")", ".arg(\"--help\")"),
        ("accepts(path, &raw)", "true"),
        ("return raw;", "return String::new();"),
    ] {
        assert!(actual.contains(from));
        assert!(!same(&expected, &actual.replacen(from, to, 1)));
    }
    let reader = read("tools/authority-runtime-074/io.rs");
    assert_eq!(reader, old("tools/authority-runtime-074/io.rs"));
    assert_eq!(reader.matches("std::fs::read_to_string(path)?").count(), 1);
    assert!(!reader.contains("Cache"));
}
#[test]
fn reviewed_inputs_delegate_only_their_exact_frozen_lookup() {
    let path = "tools/historical-input-view-074/contract.rs";
    let before = old(path);
    let start = before
        .find("    let output = std::process::Command::new(\"git\")")
        .unwrap();
    let end = start + before[start..].find("\n}\npub fn expected").unwrap();
    let expected = format!(
        "{}    assert_eq!(root(), crate::runtime::root());\n    crate::runtime::frozen(revision, path){}",
        &before[..start],
        &before[end..]
    );
    let actual = read(path);
    assert!(same(&expected, &actual));
    for (from, to) in [
        ("assert!(matches!(revision, REVIEWED | PREDECESSOR));", ""),
        ("assert!(PATHS.contains(&path));", ""),
        ("accepts(path, raw)", "true"),
        ("crate::runtime::frozen(revision, path)", "String::new()"),
    ] {
        assert!(actual.contains(from));
        assert!(!same(&expected, &actual.replacen(from, to, 1)));
    }
}
