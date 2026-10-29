//! Narrow successor for the complete audit; unchanged old rules remain authoritative.
pub mod additions;
pub mod alarm_addition;
pub mod contract;
pub mod current;
pub fn known_addition(path: &str) -> bool {
    crate::current_inputs::known_addition(path)
        || additions::known(path)
        || alarm_addition::known(path)
}
pub fn require_addition(path: &str) {
    if crate::current_inputs::known_addition(path) {
        crate::current_inputs::require_addition(path);
    } else if alarm_addition::known(path) {
        alarm_addition::require(path);
    } else {
        additions::require(path);
    }
}
pub fn read(path: &str) -> String {
    if !contract::known(path) {
        return crate::current_inputs::read(path);
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
    current::view(path, raw)
}
