//! Regression for both inline format_args call sites and unchanged quoting.
#[test]
fn argument_and_path_format_roundtrip() {
    let rule = zbus::MatchRule::builder()
        .arg(0, "it's")
        .arg(1, "a,b")
        .arg_path(2, "/org/test")
        .build()
        .unwrap();
    let text = rule.to_string();
    assert_eq!(text, "arg0='it'\\''s',arg1='a,b',arg2path='/org/test'");
    let parsed = zbus::MatchRule::try_from(text.as_str()).unwrap();
    assert_eq!(parsed, rule);
}
