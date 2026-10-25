//! One exact synthetic lock-fixture delta; all production provider bytes remain.
pub(crate) use super::integrated::contract;

const REVIEWED: &str = "c91068bfa8b297bb075b63457da235aaa9c6232e";

fn frozen(revision: &str) -> String {
    assert!(revision == REVIEWED || revision == contract::BASE);
    let output = std::process::Command::new("git")
        .arg("--no-replace-objects")
        .args(["show", &format!("{revision}:{}", contract::SOURCE)])
        .current_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "missing exact material fixture source"
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .replace("\r\n", "\n")
}

pub(crate) fn original() -> String {
    frozen(contract::BASE)
}

pub(crate) fn accepts(original: &str, raw: &str) -> bool {
    raw.replace("\r\n", "\n") == frozen(REVIEWED) && contract::accepts(original, raw)
}

pub(crate) fn view(name: &str, raw: &str) -> String {
    if name != "diagnostic_artifacts_linux.rs" {
        return raw.to_owned();
    }
    let helper = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(contract::CONTROL),
    )
    .unwrap();
    assert!(
        contract::helper_matches(&helper),
        "unreviewed fixture control"
    );
    let original = original();
    assert!(accepts(&original, raw), "unreviewed provider body");
    original
}
