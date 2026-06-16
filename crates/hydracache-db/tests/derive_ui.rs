#[test]
fn proc_macro_compile_tests() {
    let tests = trybuild::TestCases::new();
    tests.pass("tests/derive/pass_entity.rs");
    tests.pass("tests/derive/pass_field_id.rs");
    tests.pass("tests/derive/pass_no_collection.rs");
    tests.compile_fail("tests/derive/fail_conflicting_id_metadata.rs");
    tests.compile_fail("tests/derive/fail_duplicate_field_id.rs");
    tests.compile_fail("tests/derive/fail_field_id_value.rs");
    tests.compile_fail("tests/derive/fail_missing_entity.rs");
    tests.compile_fail("tests/derive/fail_missing_id.rs");
    tests.compile_fail("tests/derive/fail_unknown_field_option.rs");
    tests.compile_fail("tests/derive/fail_unknown_option.rs");
    tests.pass("tests/policy/pass_entity_policy.rs");
    tests.pass("tests/policy/pass_key_policy.rs");
    tests.pass("tests/policy/pass_preset_refresh_policy.rs");
    tests.pass("tests/policy/pass_segment_policy.rs");
    tests.compile_fail("tests/policy/fail_conflicting_key_sources.rs");
    tests.compile_fail("tests/policy/fail_empty_key_segments.rs");
    tests.compile_fail("tests/policy/fail_empty_tag_segment_group.rs");
    tests.compile_fail("tests/policy/fail_entity_missing_id.rs");
    tests.compile_fail("tests/policy/fail_flat_tag_segments.rs");
    tests.compile_fail("tests/policy/fail_missing_key_source.rs");
    tests.compile_fail("tests/policy/fail_preset_with_ttl.rs");
    tests.compile_fail("tests/policy/fail_unknown_preset.rs");
}
