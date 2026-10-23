#[allow(dead_code)]
#[path = "../../../tools/authority-runtime-074/gate.rs"]
mod runtime;
#[path = "../../../tools/authority-storage-source-074/contract.rs"]
mod storage_contract;

fn read(path: &str) -> String {
    std::fs::read_to_string(runtime::root().join(path))
        .unwrap()
        .replace("\r\n", "\n")
}
#[test]
fn status_field_scan_exact_current_source_precedes_any_historical_view() {
    let source = runtime::status_fields::SOURCE;
    let actual = read(source);
    let baseline = runtime::frozen(runtime::BASE, source);
    assert!(runtime::status_fields::helper_is_pinned(&read(
        runtime::status_fields::HELPER
    )));
    assert!(runtime::accepts(source, &actual));
    assert!(!runtime::accepts(source, &baseline));
    assert_eq!(runtime::view(source, actual), baseline);
}
#[test]
fn status_field_scan_refuses_removed_guards_reads_order_and_helper_drift() {
    let source = runtime::status_fields::SOURCE;
    let actual = read(source);
    for (before, after) in [
        (".take(33)", ".take(32)"),
        ("if duplicate {", "if false {"),
        ("if values.len() > 32", "if values.len() > 33"),
        (
            "scalar(projection_fields::Name::Pid)",
            "scalar(projection_fields::Name::Tgid)",
        ),
        (
            "groups_field(projection_fields::Name::Groups)",
            "groups_field(projection_fields::Name::Gid)",
        ),
        (
            "projection(scratch.read(file)?, &self.credentials)?",
            "self.projection",
        ),
        ("origin.revalidate()?;", ""),
        ("after.realtime_ns < before.realtime_ns", "false"),
        ("60_000_000_000", "61_000_000_000"),
    ] {
        assert!(actual.contains(before), "non-vacuous mutation: {before}");
        for (at, _) in actual.match_indices(before) {
            let mut changed = actual.clone();
            changed.replace_range(at..at + before.len(), after);
            assert!(!runtime::accepts(source, &changed));
            assert!(std::panic::catch_unwind(|| runtime::view(source, changed)).is_err());
        }
    }
    let helper = read(runtime::status_fields::HELPER);
    for before in [
        "text.lines()",
        "result.duplicate[slot] = true",
        "Name::Pid",
        "Name::Groups",
    ] {
        assert!(helper.contains(before));
        assert!(!runtime::status_fields::helper_is_pinned(
            &helper.replacen(before, "changed", 1)
        ));
    }
}
