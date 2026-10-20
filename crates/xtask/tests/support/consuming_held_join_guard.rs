//! Exactly three additive hooks; no exclusion for any old guard body or format.
pub(crate) const HOOKS: [(&str, &str); 3] = [
    (
        "tools/long-run-supervisor-074/src/diagnostic_fixed_placement.rs",
        "\n#[path = \"diagnostic_consuming_held_join.rs\"]\nmod held_join;\n",
    ),
    (
        "tools/long-run-supervisor-074/src/diagnostic_held_runtime_trust.rs",
        "\n#[path = \"diagnostic_held_late_trust.rs\"]\npub(crate) mod late_join;\n",
    ),
    (
        "tools/long-run-supervisor-074/src/diagnostic_held_role_files.rs",
        "\n#[path = \"diagnostic_held_role_provenance.rs\"]\nmod provenance;\n",
    ),
];
#[allow(dead_code)]
pub(crate) const SOURCES: [&str; 3] = [
    "tools/long-run-supervisor-074/src/diagnostic_consuming_held_join.rs",
    "tools/long-run-supervisor-074/src/diagnostic_held_late_trust.rs",
    "tools/long-run-supervisor-074/src/diagnostic_held_role_provenance.rs",
];
pub(crate) fn normalize(path: &str, raw: &str) -> String {
    let Some((_, exact)) = HOOKS.iter().find(|(file, _)| *file == path) else {
        return raw.into();
    };
    assert_eq!(
        raw.matches(exact).count(),
        1,
        "exact consuming held join hook: {path}"
    );
    raw.replacen(exact, "", 1)
}
#[allow(dead_code)]
pub(crate) fn previous_list(current: &str) -> String {
    for path in SOURCES {
        assert_eq!(
            current.lines().filter(|p| *p == path).count(),
            1,
            "exact consuming held join source: {path}"
        );
    }
    current
        .lines()
        .filter(|path| !SOURCES.contains(path))
        .map(|path| format!("{path}\n"))
        .collect()
}
