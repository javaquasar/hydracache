#[path = "../../../tools/material-lock-fixture-074/contract.rs"]
mod contract;
fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn original() -> String {
    let output = std::process::Command::new("git")
        .args([
            "--no-replace-objects",
            "show",
            &format!("{}:{}", contract::BASE, contract::SOURCE),
        ])
        .current_dir(root())
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout)
        .unwrap()
        .replace("\r\n", "\n")
}
#[test]
fn synthetic_material_lock_current_source_is_exact_and_old_assertions_remain() {
    let source = std::fs::read_to_string(root().join(contract::SOURCE)).unwrap();
    let control = std::fs::read_to_string(root().join(contract::CONTROL)).unwrap();
    assert!(contract::helper_matches(&control));
    assert!(
        contract::accepts(&original(), &source),
        "synthetic fixture correction is absent or drifted"
    );
}
#[test]
fn synthetic_material_lock_delta_cannot_remove_old_checks_or_change_production_body() {
    let old = original();
    let expected = contract::candidate(&old);
    assert!(!contract::accepts(&old, &old));
    for (from, to) in [
        (contract::INPUT, ""),
        (contract::HOOK, ""),
        ("bundle.revalidate()?;", ""),
        (
            "verify_receipt(receipt, &state.identity)",
            "verify_receipt(receipt, &other.identity)",
        ),
    ] {
        assert!(expected.contains(from));
        assert!(!contract::accepts(&old, &expected.replacen(from, to, 1)));
    }
    let control = std::fs::read_to_string(root().join(contract::CONTROL)).unwrap();
    assert!(!contract::helper_matches(&format!("{control}\n// drift")));
    assert!(std::panic::catch_unwind(|| contract::candidate(&format!(
        "{old}\n// predecessor drift"
    )))
    .is_err());
}
