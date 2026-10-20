//! Exact additive module only; no old source, schema or live check is excluded.
pub(crate) const FILE: &str = "tools/long-run-supervisor-074/src/diagnostic_held_build.rs";
pub(crate) const SOURCE: &str = "tools/long-run-supervisor-074/src/diagnostic_held_role_files.rs";
pub(crate) const HOOK: &str = "\n#[cfg(target_os = \"linux\")]\n#[path = \"diagnostic_held_role_files.rs\"]\npub mod role_files;\n";
pub(crate) fn normalize(path: &str, raw: &str) -> String {
    if path != FILE {
        return raw.into();
    }
    assert_eq!(raw.matches(HOOK).count(), 1, "exact role-file module hook");
    raw.replacen(HOOK, "", 1)
}
pub(crate) fn previous_list(current: &str) -> String {
    assert_eq!(
        current.lines().filter(|p| *p == SOURCE).count(),
        1,
        "exact role-file source"
    );
    current
        .lines()
        .filter(|p| *p != SOURCE)
        .map(|p| format!("{p}\n"))
        .collect()
}
