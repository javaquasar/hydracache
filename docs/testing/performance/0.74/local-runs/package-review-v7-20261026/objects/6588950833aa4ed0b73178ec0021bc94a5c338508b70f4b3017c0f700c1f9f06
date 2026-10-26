//! Reuse the admitted current frontier without modifying the original 9/81 gate.
#[path = "../current-frontier-074/additions.rs"]
pub mod additions;
#[path = "../current-frontier-074/contract.rs"]
pub mod contract;
#[path = "../remaining-readers-074/inputs.rs"]
pub mod previous;

pub(crate) fn known_addition(path: &str) -> bool {
    previous::known_addition(path) || additions::known(path)
}
pub(crate) fn require_addition(path: &str) {
    if previous::known_addition(path) {
        previous::require_addition(path);
    } else {
        additions::require(path);
    }
}
pub(crate) fn read(path: &str) -> String {
    if !contract::known(path) {
        return previous::read(path);
    }
    let root = crate::runtime::root();
    let file = root.join(path);
    // Preserve the old fallback's observation, canonical-root and path checks.
    let raw = std::fs::read_to_string(&file).unwrap();
    let root = root.canonicalize().unwrap();
    let absolute = file.canonicalize().unwrap();
    let relative = absolute
        .strip_prefix(root)
        .unwrap()
        .to_str()
        .unwrap()
        .replace('\\', "/");
    assert_eq!(relative, path, "fixed frontier input path");
    assert!(!crate::runtime_io::owned_case_view::PATHS.contains(&path));
    assert!(!crate::runtime_io::reviewed_inputs::PATHS.contains(&path));
    // MAIN first becomes the exact pre-terminal body, then the unchanged runtime
    // gate checks its complete expected body before releasing SOURCE.
    crate::runtime::view(path, contract::project(path, raw))
}
