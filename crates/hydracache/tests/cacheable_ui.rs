#[test]
fn cacheable_macro_compile_tests() {
    let tests = trybuild::TestCases::new();
    tests.pass("tests/cacheable/pass_cacheable.rs");
    tests.compile_fail("tests/cacheable/fail_conflicting_ttl.rs");
    tests.compile_fail("tests/cacheable/fail_missing_cache.rs");
    tests.compile_fail("tests/cacheable/fail_missing_key.rs");
    tests.compile_fail("tests/cacheable/fail_missing_load.rs");
}
