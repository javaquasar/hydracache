#[test]
fn cacheable_macro_compile_tests() {
    let tests = trybuild::TestCases::new();
    tests.pass("tests/cacheable/pass_cacheable.rs");
    tests.pass("tests/cacheable/pass_cacheable_attribute.rs");
    tests.pass("tests/cacheable/pass_cacheable_attribute_key_tags.rs");
    tests.pass("tests/cacheable/pass_cacheable_infallible.rs");
    tests.pass("tests/cacheable/pass_cacheable_tags.rs");
    tests.compile_fail("tests/cacheable/fail_attribute_conflicting_key.rs");
    tests.compile_fail("tests/cacheable/fail_attribute_conflicting_ttl.rs");
    tests.compile_fail("tests/cacheable/fail_attribute_duplicate_key_segments.rs");
    tests.compile_fail("tests/cacheable/fail_attribute_empty_key_segments.rs");
    tests.compile_fail("tests/cacheable/fail_attribute_empty_tag_segment_group.rs");
    tests.compile_fail("tests/cacheable/fail_attribute_empty_tag_segments.rs");
    tests.compile_fail("tests/cacheable/fail_attribute_flat_tag_segments.rs");
    tests.compile_fail("tests/cacheable/fail_attribute_missing_cache.rs");
    tests.compile_fail("tests/cacheable/fail_attribute_missing_key.rs");
    tests.compile_fail("tests/cacheable/fail_attribute_non_result.rs");
    tests.compile_fail("tests/cacheable/fail_attribute_unknown_option.rs");
    tests.compile_fail("tests/cacheable/fail_conflicting_ttl.rs");
    tests.compile_fail("tests/cacheable/fail_missing_cache.rs");
    tests.compile_fail("tests/cacheable/fail_missing_key.rs");
    tests.compile_fail("tests/cacheable/fail_missing_load.rs");
}
