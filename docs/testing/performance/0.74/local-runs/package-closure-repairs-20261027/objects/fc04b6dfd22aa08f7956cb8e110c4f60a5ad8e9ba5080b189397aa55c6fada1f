//! Deterministic fixture interpreter selection; no probing, fallback or PATH edit.
pub(crate) const fn program_for(windows: bool) -> &'static str {
    if windows {
        "python"
    } else {
        "python3"
    }
}

pub(crate) const fn program() -> &'static str {
    program_for(cfg!(windows))
}

#[test]
fn fixture_python_selection_is_explicit_for_both_platforms() {
    assert_eq!(program_for(true), "python");
    assert_eq!(program_for(false), "python3");
    assert_eq!(program(), program_for(cfg!(windows)));
}
